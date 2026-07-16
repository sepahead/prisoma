//! Runner for the retired v10.7 endpoint-sensitivity calculation.
//!
//! Its outputs are historical and nonpromotable: they do not evaluate the current EC1/H1–H4
//! registry, establish scientific success, or provide capture requirements.
//!
//! Usage: pid-sim-legacy-sensitivity [--quick] [--out report.json] [--md report.md]

use std::error::Error;

use pid_sim::legacy_sensitivity::{
    legacy_sensitivity_markdown, run_legacy_sensitivity_calculation, PowerGateConfig,
};

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
    let report = run_legacy_sensitivity_calculation(&cfg)?;
    let md = legacy_sensitivity_markdown(&report);
    if let Some(path) = flag("--out") {
        std::fs::write(&path, serde_json::to_string_pretty(&report)?)?;
        eprintln!("[legacy-sensitivity] wrote {path}");
    }
    if let Some(path) = flag("--md") {
        std::fs::write(&path, &md)?;
        eprintln!("[legacy-sensitivity] wrote {path}");
    }
    println!("{md}");
    Ok(())
}
