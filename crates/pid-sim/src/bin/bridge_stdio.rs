use anyhow::{bail, Context, Result};
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use serde_json::json;
use std::collections::BTreeMap;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.len() != 2 {
        bail!(
            "usage: {} <run-log.jsonl>",
            args.first()
                .and_then(|value| value.to_str())
                .unwrap_or("pid-sim-bridge-stdio")
        );
    }

    let path = PathBuf::from(&args[1]);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut writer = RunLogWriter::create(&path)?;
    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "pid-sim-bridge-stdio".to_string());
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: "bridge-stdio-run".to_string(),
        timestamp_ns: 0,
        config_hash: pid_runlog::canonical_json_hash(
            &json!({"bridge": "stdio", "sim": "deterministic_object"}),
        )?,
        metadata,
    })?;

    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: "pid-sim-bridge-stdio".to_string(),
        session_id: Some("bridge-stdio".to_string()),
    };
    let sim = pid_sim::demo_sim();
    writer.append(&sim.snapshot_event())?;
    let mut session = pid_sim::SimBridgeSession::new(writer, sim);

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    let handled = pid_sim::dispatch_rpc_lines(
        BufReader::new(stdin.lock()),
        &mut output,
        &mut session,
        actor,
    )?;
    output.flush().context("failed to flush stdout")?;

    session.record_event(&RunLogEvent::RunEnded {
        run_id: "bridge-stdio-run".to_string(),
        timestamp_ns: session.timestamp_ns(),
        status: RunStatus::Succeeded,
        message: Some(format!("processed {handled} request(s)")),
    })?;
    session.flush()?;
    eprintln!("wrote {}", path.display());
    Ok(())
}
