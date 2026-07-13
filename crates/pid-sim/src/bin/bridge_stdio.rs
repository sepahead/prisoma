use anyhow::{bail, Context, Result};
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let (safe_mode, path_arg) = match args.as_slice() {
        // `--safe-mode` with the path omitted must NOT match this arm — it
        // would silently run with safe mode OFF and a run log named
        // `--safe-mode`.
        [_, path] if path.to_str() != Some("--safe-mode") => (false, path),
        [_, flag, path] if flag.to_str() == Some("--safe-mode") => (true, path),
        _ => {
            bail!(
                "usage: {} [--safe-mode] <run-log.jsonl>",
                args.first()
                    .and_then(|value| value.to_str())
                    .unwrap_or("pid-sim-bridge-stdio")
            );
        }
    };

    let (path, artifact_root) = pid_sim::canonical_new_artifact_path(PathBuf::from(path_arg))?;

    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("failed to create new run log {}", path.display()))?;
    let mut writer = RunLogWriter::new(pid_sim::FsyncFileWriter::new(file));
    let config = pid_sim::deterministic_sim_config(
        "pid-sim-bridge-stdio",
        Some("stdio_jsonl"),
        None,
        None,
        Some(safe_mode),
    );
    let config_hash = pid_runlog::canonical_json_hash(&config)?;
    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "pid-sim-bridge-stdio".to_string());
    metadata.insert("safe_mode".to_string(), safe_mode.to_string());
    metadata.insert(
        "artifact_root".to_string(),
        artifact_root.display().to_string(),
    );
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: "bridge-stdio-run".to_string(),
        timestamp_ns: 0,
        config_hash: config_hash.clone(),
        metadata,
    })?;
    writer.append(&RunLogEvent::ConfigLogged {
        timestamp_ns: 0,
        config_hash,
        config,
    })?;

    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: "pid-sim-bridge-stdio".to_string(),
        session_id: Some("bridge-stdio".to_string()),
    };
    let sim = pid_sim::demo_sim();
    writer.append(&sim.snapshot_event())?;
    let mut session = pid_sim::SimBridgeSession::with_safe_mode_and_run_id(
        writer,
        sim,
        safe_mode,
        "bridge-stdio-run",
    );
    session.set_run_log_path(&path);
    session.set_artifact_root(&artifact_root)?;
    // Detect buffered provenance-storage failures before accepting control
    // input or emitting any response.
    session.flush()?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    let handled = pid_sim::dispatch_rpc_lines(
        BufReader::new(stdin.lock()),
        &mut output,
        &mut session,
        actor,
    )
    .and_then(|handled| {
        output.flush().context("failed to flush stdout")?;
        Ok(handled)
    });

    // When provenance storage remains writable, always seal accepted-client
    // transport errors as Failed. A provenance-storage error itself may leave
    // a partial/unreadable log and cannot be repaired by this transport.
    let (status, message) = match &handled {
        Ok(count) => (
            RunStatus::Succeeded,
            format!("processed {count} request(s)"),
        ),
        Err(err) => (RunStatus::Failed, format!("transport error: {err:#}")),
    };
    session.finish_run(status, Some(message))?;
    session.flush()?;
    eprintln!("wrote {}", path.display());
    handled.map(|_| ())
}
