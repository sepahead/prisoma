use anyhow::{bail, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args_os();
    let program = args
        .next()
        .and_then(|s| s.into_string().ok())
        .unwrap_or_else(|| "pid-runlog-replay".to_string());
    let Some(path) = args.next().map(PathBuf::from) else {
        bail!("usage: {program} <run-log.jsonl>");
    };
    if args.next().is_some() {
        bail!("usage: {program} <run-log.jsonl>");
    }

    let events = pid_runlog::read_events_from_path(&path)?;
    let state = pid_runlog::replay_events(&events);
    let trace_hash = pid_runlog::canonical_json_hash(&state)?;

    println!("events={}", state.events_seen);
    println!("trace_hash={trace_hash}");
    if let Some(run_id) = &state.run_id {
        println!("run_id={run_id}");
    }
    if let Some(step) = state.last_step {
        println!("last_step={step}");
    }
    println!("actions={}", state.actions.len());
    println!("interventions={}", state.interventions.len());
    println!("objects={}", state.object_poses.len());
    println!("pid_metrics={}", state.pid_metrics.len());
    println!("geometry_metrics={}", state.geometry_metrics.len());
    println!("artifacts={}", state.artifacts.len());
    println!("errors={}", state.errors.len());
    println!("flow_gt_records={}", state.flow_gt_records);

    Ok(())
}
