use anyhow::{Context, Result};
use pid_bridge::BridgeMethod;
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("outputs/demo_bridge_runlog.jsonl"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut writer = RunLogWriter::create(&path)?;
    let config = pid_sim::deterministic_sim_config(
        "pid-sim-bridge-demo",
        Some("local"),
        Some(0.1),
        Some(5),
        Some(false),
    );
    let config_hash = pid_runlog::canonical_json_hash(&config)?;
    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "pid-sim-bridge-demo".to_string());
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: "bridge-demo-run".to_string(),
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
        actor_id: "pid-sim-bridge-demo".to_string(),
        session_id: Some("bridge-demo".to_string()),
    };
    let sim = pid_sim::demo_sim();
    writer.append(&sim.snapshot_event())?;
    let mut session = pid_sim::SimBridgeSession::new(writer, sim);
    for idx in 0..5 {
        let request = pid_sim::bridge_request(
            format!("req-step-{idx}"),
            BridgeMethod::SimStep,
            actor.clone(),
            Some(idx),
            idx * 100_000_000,
            json!({ "dt": 0.1 }),
        );
        session.dispatch(&request)?;
    }
    session.record_event(&RunLogEvent::RunEnded {
        run_id: "bridge-demo-run".to_string(),
        timestamp_ns: 500_000_000,
        status: RunStatus::Succeeded,
        message: Some("bridge demo complete".to_string()),
    })?;
    session.flush()?;
    println!("wrote {}", path.display());
    Ok(())
}
