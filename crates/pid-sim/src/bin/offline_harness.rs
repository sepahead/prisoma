use anyhow::{bail, Context, Result};
use pid_sim::offline_harness::{
    offline_vlda_geometry_gate_failure_message, offline_vlda_heldout_split_failure_message,
    offline_vlda_heldout_split_status, offline_vlda_success_label_failure_message,
    offline_vlda_success_label_status, read_offline_vlda_dataset, run_offline_vlda_harness,
    write_offline_vlda_runlog_with_options, write_offline_vlda_summary, OfflineVldaRunlogOptions,
};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Args {
    input: PathBuf,
    summary_json: PathBuf,
    runlog: PathBuf,
    require_geometry_pass: bool,
    require_success_labels: bool,
    require_heldout_split: bool,
}

fn main() -> Result<()> {
    let args = parse_args(std::env::args().skip(1))?;
    let input_sha256 = pid_runlog::sha256_file(&args.input)
        .with_context(|| format!("failed to hash {}", args.input.display()))?;
    let dataset = read_offline_vlda_dataset(&args.input)?;
    let report = run_offline_vlda_harness(
        dataset.clone(),
        Some(args.input.display().to_string()),
        Some(input_sha256),
    )?;
    write_offline_vlda_summary(&args.summary_json, &report)?;
    write_offline_vlda_runlog_with_options(
        &args.runlog,
        Some(&args.summary_json),
        Some(&args.input),
        &dataset,
        &report,
        OfflineVldaRunlogOptions {
            require_geometry_pass: args.require_geometry_pass,
            require_success_labels: args.require_success_labels,
            require_heldout_split: args.require_heldout_split,
        },
    )?;
    println!(
        "offline_vlda_summary={} runlog={} samples={} config_hash={} geometry_gate_status={} success_label_status={} heldout_split_status={}",
        args.summary_json.display(),
        args.runlog.display(),
        report.dims.samples,
        report.config_hash,
        report.geometry.gates.status,
        offline_vlda_success_label_status(&report),
        offline_vlda_heldout_split_status(&report)
    );
    let mut failures = Vec::new();
    if args.require_geometry_pass && report.geometry.gates.status != "pass" {
        failures.push(offline_vlda_geometry_gate_failure_message(&report));
    }
    if args.require_success_labels && report.metrics.success_rate.is_none() {
        failures.push(offline_vlda_success_label_failure_message(
            &dataset, &report,
        ));
    }
    if args.require_heldout_split && report.metrics.heldout_majority_success_accuracy.is_none() {
        failures.push(offline_vlda_heldout_split_failure_message(
            &dataset, &report,
        ));
    }
    if !failures.is_empty() {
        bail!("{}", failures.join("; "));
    }
    Ok(())
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args> {
    let mut input = None;
    let mut summary_json = PathBuf::from("outputs/offline_vlda_summary.json");
    let mut runlog = PathBuf::from("outputs/offline_vlda_runlog.jsonl");
    let mut require_geometry_pass = false;
    let mut require_success_labels = false;
    let mut require_heldout_split = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "--input" => {
                input = Some(PathBuf::from(
                    iter.next().context("--input requires a path")?,
                ));
            }
            "--summary-json" => {
                summary_json =
                    PathBuf::from(iter.next().context("--summary-json requires a path")?);
            }
            "--runlog" => {
                runlog = PathBuf::from(iter.next().context("--runlog requires a path")?);
            }
            "--require-geometry-pass" => {
                require_geometry_pass = true;
            }
            "--require-success-labels" => {
                require_success_labels = true;
            }
            "--require-heldout-split" => {
                require_heldout_split = true;
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    let input = input.context("--input is required")?;
    Ok(Args {
        input,
        summary_json,
        runlog,
        require_geometry_pass,
        require_success_labels,
        require_heldout_split,
    })
}

fn print_usage() {
    println!(
        "Usage: pid-offline-harness --input PATH [--summary-json PATH] [--runlog PATH] [--require-geometry-pass] [--require-success-labels] [--require-heldout-split]\n\
         \n\
         Converts captured (V,L,D,A) embedding JSON into canonical summary and run-log artifacts."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_accepts_paths() {
        let args = parse_args([
            "--input".to_string(),
            "fixture.json".to_string(),
            "--summary-json".to_string(),
            "summary.json".to_string(),
            "--runlog".to_string(),
            "runlog.jsonl".to_string(),
            "--require-geometry-pass".to_string(),
            "--require-success-labels".to_string(),
            "--require-heldout-split".to_string(),
        ])
        .unwrap();
        assert_eq!(args.input, PathBuf::from("fixture.json"));
        assert_eq!(args.summary_json, PathBuf::from("summary.json"));
        assert_eq!(args.runlog, PathBuf::from("runlog.jsonl"));
        assert!(args.require_geometry_pass);
        assert!(args.require_success_labels);
        assert!(args.require_heldout_split);
    }
}
