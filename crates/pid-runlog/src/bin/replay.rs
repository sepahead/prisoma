use anyhow::{bail, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let program = args
        .first()
        .cloned()
        .and_then(|s| s.into_string().ok())
        .unwrap_or_else(|| "pid-runlog-replay".to_string());

    if args.len() == 4 && args.get(1).and_then(|s| s.to_str()) == Some("--compare") {
        let left = pid_runlog::replay_state_from_path(PathBuf::from(args[2].clone()))?;
        let right = pid_runlog::replay_state_from_path(PathBuf::from(args[3].clone()))?;
        let left_hash = pid_runlog::canonical_json_hash(&left)?;
        let right_hash = pid_runlog::canonical_json_hash(&right)?;
        println!("left_trace_hash={left_hash}");
        println!("right_trace_hash={right_hash}");
        println!("match={}", left_hash == right_hash);
        return Ok(());
    }

    if args.len() != 2 {
        bail!("usage: {program} <run-log.jsonl>\n       {program} --compare <left.jsonl> <right.jsonl>");
    }

    let path = PathBuf::from(args[1].clone());
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
    println!("embeddings={}", state.embeddings.len());
    println!("bridge_records={}", state.bridge_records.len());
    println!("sim_snapshots={}", state.sim_snapshots);
    println!("artifacts={}", state.artifacts.len());
    println!("errors={}", state.errors.len());
    println!("flow_gt_records={}", state.flow_gt_records);

    Ok(())
}
