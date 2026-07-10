//! §14.8.3 power gate runner: simulate the H1–H4 primary endpoints at their
//! preregistered minimum effect sizes and report power per candidate capture
//! size. Writes a JSON artifact and a markdown report.
//!
//! Usage: pid-sim-power-gate [--quick] [--out report.json] [--md report.md]

use pid_sim::power::{power_gate_markdown, run_power_gate, PowerGateConfig};

fn main() {
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
    let report = run_power_gate(&cfg);
    let md = power_gate_markdown(&report);
    if let Some(path) = flag("--out") {
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&report).expect("serialize"),
        )
        .expect("write json");
        eprintln!("[power-gate] wrote {path}");
    }
    if let Some(path) = flag("--md") {
        std::fs::write(&path, &md).expect("write md");
        eprintln!("[power-gate] wrote {path}");
    }
    println!("{md}");
    if !report.passed {
        std::process::exit(1);
    }
}
