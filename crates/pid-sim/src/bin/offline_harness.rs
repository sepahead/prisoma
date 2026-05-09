use anyhow::{bail, Context, Result};
use pid_sim::offline_harness::{
    read_offline_vlda_dataset, run_offline_vlda_harness, write_offline_vlda_runlog,
    write_offline_vlda_summary,
};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Args {
    input: PathBuf,
    summary_json: PathBuf,
    runlog: PathBuf,
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
    write_offline_vlda_runlog(
        &args.runlog,
        Some(&args.summary_json),
        Some(&args.input),
        &dataset,
        &report,
    )?;
    println!(
        "offline_vlda_summary={} runlog={} samples={} config_hash={}",
        args.summary_json.display(),
        args.runlog.display(),
        report.dims.samples,
        report.config_hash
    );
    Ok(())
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args> {
    let mut input = None;
    let mut summary_json = PathBuf::from("outputs/offline_vlda_summary.json");
    let mut runlog = PathBuf::from("outputs/offline_vlda_runlog.jsonl");
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
            other => bail!("unknown argument: {other}"),
        }
    }
    let input = input.context("--input is required")?;
    Ok(Args {
        input,
        summary_json,
        runlog,
    })
}

fn print_usage() {
    println!(
        "Usage: pid-offline-harness --input PATH [--summary-json PATH] [--runlog PATH]\n\
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
        ])
        .unwrap();
        assert_eq!(args.input, PathBuf::from("fixture.json"));
        assert_eq!(args.summary_json, PathBuf::from("summary.json"));
        assert_eq!(args.runlog, PathBuf::from("runlog.jsonl"));
    }
}
