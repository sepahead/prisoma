use anyhow::{bail, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        bail!("usage: {} <run-log.jsonl> [--tolerance eps]", args[0]);
    }
    let path = PathBuf::from(&args[1]);
    let mut tolerance = 1e-9;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--tolerance" => {
                let Some(value) = args.get(i + 1) else {
                    bail!("--tolerance requires a value");
                };
                tolerance = value.parse()?;
                i += 2;
            }
            other => bail!("unknown argument: {other}"),
        }
    }

    let events = pid_runlog::read_events_from_path(&path)?;
    let validation = pid_runlog::validate_events(&events);
    let flow = pid_sim::verify_flow_gt(&events, tolerance);
    println!("runlog_valid={}", validation.is_valid());
    println!("runlog_errors={}", validation.errors);
    println!("runlog_warnings={}", validation.warnings);
    println!("flow_valid={}", flow.is_valid());
    println!("flow_checked={}", flow.checked_flows);
    println!("flow_issues={}", flow.issues.len());
    for issue in &flow.issues {
        println!("flow_issue={issue}");
    }
    if !validation.is_valid() || !flow.is_valid() {
        std::process::exit(1);
    }
    Ok(())
}
