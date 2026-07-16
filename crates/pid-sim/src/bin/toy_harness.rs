use anyhow::{bail, Context, Result};
use pid_sim::toy_harness::{
    ensure_distinct_toy_output_paths, run_toy_harness, write_toy_harness_runlog,
    write_toy_harness_summary, ToyHarnessConfig,
};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut config = ToyHarnessConfig::default();
    let mut summary_path = PathBuf::from("outputs/toy_vla_summary.json");
    let mut runlog_path = PathBuf::from("outputs/toy_vla_runlog.jsonl");
    let args = std::env::args().collect::<Vec<_>>();
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--episodes" if i + 1 < args.len() => {
                config.episodes = args[i + 1]
                    .parse()
                    .context("--episodes requires a positive integer")?;
                i += 2;
            }
            "--seed" if i + 1 < args.len() => {
                config.seed = args[i + 1]
                    .parse()
                    .context("--seed requires an unsigned integer")?;
                i += 2;
            }
            "--summary-json" if i + 1 < args.len() => {
                summary_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--runlog" if i + 1 < args.len() => {
                runlog_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            other => bail!("unknown or incomplete argument: {other}"),
        }
    }

    ensure_distinct_toy_output_paths(&runlog_path, &summary_path)?;
    let report = run_toy_harness(config)?;
    write_toy_harness_summary(&summary_path, &report)?;
    write_toy_harness_runlog(&runlog_path, Some(&summary_path), &report)?;
    println!("run_id={}", report.run_id);
    println!("episodes={}", report.samples.len());
    println!("failures={}", report.failures());
    println!("success_rate={:.6}", report.baselines.success_rate);
    println!(
        "baseline_majority_accuracy={:.6}",
        report.baselines.majority_accuracy
    );
    println!(
        "baseline_action_error_accuracy={:.6}",
        report.baselines.action_error_accuracy
    );
    println!("pid_synergy={:.6}", report.pid.synergy);
    println!("wrote_summary={}", summary_path.display());
    println!("wrote_runlog={}", runlog_path.display());
    Ok(())
}

fn print_usage() {
    println!(
        "Usage: pid-toy-harness [--episodes N] [--seed N] [--summary-json PATH] [--runlog PATH]"
    );
}
