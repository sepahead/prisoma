use anyhow::{bail, Context, Result};
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use std::collections::BTreeMap;
use std::io::{BufReader, BufWriter, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let program = args
        .first()
        .and_then(|value| value.to_str())
        .unwrap_or("pid-sim-bridge-tcp");
    let mut safe_mode = false;
    let mut bind_addr = "127.0.0.1:38472".to_string();
    let mut path_arg = None;
    let mut idx = 1;
    while idx < args.len() {
        match args[idx].to_str() {
            Some("--safe-mode") => {
                safe_mode = true;
                idx += 1;
            }
            Some("--bind") => {
                idx += 1;
                let Some(value) = args.get(idx).and_then(|value| value.to_str()) else {
                    bail!("--bind requires an address");
                };
                bind_addr = value.to_string();
                idx += 1;
            }
            Some("-h" | "--help") => {
                eprintln!("usage: {program} [--safe-mode] [--bind ADDR] <run-log.jsonl>");
                return Ok(());
            }
            Some(_) if path_arg.is_none() => {
                path_arg = args.get(idx).cloned();
                idx += 1;
            }
            _ => bail!("usage: {program} [--safe-mode] [--bind ADDR] <run-log.jsonl>"),
        }
    }
    let Some(path_arg) = path_arg else {
        bail!("usage: {program} [--safe-mode] [--bind ADDR] <run-log.jsonl>");
    };
    let bind_addr: SocketAddr = bind_addr
        .parse()
        .with_context(|| format!("invalid bind address {bind_addr}"))?;

    let path = PathBuf::from(path_arg);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut writer = RunLogWriter::create(&path)?;
    let config = pid_sim::deterministic_sim_config(
        "pid-sim-bridge-tcp",
        Some("tcp_jsonl"),
        None,
        None,
        Some(safe_mode),
    );
    let config_hash = pid_runlog::canonical_json_hash(&config)?;
    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "pid-sim-bridge-tcp".to_string());
    metadata.insert("safe_mode".to_string(), safe_mode.to_string());
    metadata.insert("bind_addr".to_string(), bind_addr.to_string());
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: "bridge-tcp-run".to_string(),
        timestamp_ns: 0,
        config_hash: config_hash.clone(),
        metadata,
    })?;
    writer.append(&RunLogEvent::ConfigLogged {
        timestamp_ns: 0,
        config_hash,
        config,
    })?;

    let sim = pid_sim::demo_sim();
    writer.append(&sim.snapshot_event())?;
    let mut session = pid_sim::SimBridgeSession::with_safe_mode(writer, sim, safe_mode);
    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: "pid-sim-bridge-tcp".to_string(),
        session_id: Some("bridge-tcp".to_string()),
    };

    let listener =
        TcpListener::bind(bind_addr).with_context(|| format!("failed to bind {bind_addr}"))?;
    let local_addr = listener
        .local_addr()
        .context("failed to read local address")?;
    eprintln!("listening {local_addr}");
    let (stream, peer_addr) = listener.accept().context("failed to accept TCP client")?;
    eprintln!("accepted {peer_addr}");

    let reader = BufReader::new(stream.try_clone().context("failed to clone TCP stream")?);
    let mut output = BufWriter::new(stream);
    let handled = pid_sim::dispatch_rpc_lines(reader, &mut output, &mut session, actor)?;
    output.flush().context("failed to flush TCP responses")?;

    session.record_event(&RunLogEvent::RunEnded {
        run_id: "bridge-tcp-run".to_string(),
        timestamp_ns: session.timestamp_ns(),
        status: RunStatus::Succeeded,
        message: Some(format!("processed {handled} request(s) from {peer_addr}")),
    })?;
    session.flush()?;
    eprintln!("wrote {}", path.display());
    Ok(())
}
