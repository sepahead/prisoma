use anyhow::{Context, Result};
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("outputs/demo_runlog.jsonl"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut writer = RunLogWriter::create(&path)?;
    let config = pid_sim::deterministic_sim_config("pid-sim-demo", None, Some(0.1), Some(5), None);
    let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "pid-sim-demo".to_string());
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: "demo-run".to_string(),
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
        actor_id: "pid-sim-demo".to_string(),
        session_id: Some("demo".to_string()),
    };
    let mut sim = pid_sim::demo_sim();
    writer.append(&sim.snapshot_event())?;
    for _ in 0..5 {
        let payload = json!({ "dt": 0.1 });
        writer.append(&RunLogEvent::ActionApplied {
            step: sim.step(),
            timestamp_ns: sim.timestamp_ns(),
            actor: actor.clone(),
            action_type: "sim.step".to_string(),
            payload_hash: pid_runlog::canonical_json_hash_v2(&payload)?,
            payload,
        })?;
        let step = sim.step_fixed(0.1)?;
        writer.append(&sim.snapshot_event())?;
        for event in sim.pose_events() {
            writer.append(&event)?;
        }
        for event in step.flow_events() {
            writer.append(&event)?;
        }
        for event in step.flow_pred_events() {
            writer.append(&event)?;
        }
    }
    writer.append(&RunLogEvent::RunEnded {
        run_id: "demo-run".to_string(),
        timestamp_ns: sim.timestamp_ns(),
        status: RunStatus::Succeeded,
        message: Some("demo complete".to_string()),
    })?;
    writer.flush()?;
    println!("wrote {}", path.display());
    Ok(())
}
