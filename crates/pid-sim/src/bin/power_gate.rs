//! §6.8 power gate runner: simulate idealized H1–H4 endpoint sensitivities
//! at their preregistered minimum effects. Evaluated task/case counts are grid
//! points, not capture requirements or guarantees. Writes JSON and markdown.
//!
//! Usage: pid-sim-power-gate [--quick] [--out report.json] [--md report.md]

use std::error::Error;

use pid_sim::power::{power_gate_markdown, run_power_gate, PowerGateConfig};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1).cloned())
    };
    let mut cfg = PowerGateConfig::default();
    if args.iter().any(|a| a == "--quick") {
        cfg.replicates = 100;
        cfg.n_boot = 200;
    }
    let report = run_power_gate(&cfg)?;
    let md = power_gate_markdown(&report);
    if let Some(path) = flag("--out") {
        std::fs::write(&path, serde_json::to_string_pretty(&report)?)?;
        eprintln!("[power-gate] wrote {path}");
    }
    if let Some(path) = flag("--md") {
        std::fs::write(&path, &md)?;
        eprintln!("[power-gate] wrote {path}");
    }
    println!("{md}");
    if !report.idealized_sensitivity_gate_passed {
        std::process::exit(1);
    }
    Ok(())
}
