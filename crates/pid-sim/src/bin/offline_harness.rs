use anyhow::{bail, Context, Result};
use pid_sim::offline_harness::{
    compute_offline_pid_uncertainty, offline_vlda_axis_provenance_failure_messages,
    offline_vlda_geometry_gate_failure_message,
    offline_vlda_heldout_class_coverage_failure_message,
    offline_vlda_heldout_class_coverage_status,
    offline_vlda_heldout_episode_disjoint_failure_message,
    offline_vlda_heldout_episode_disjoint_status, offline_vlda_heldout_split_failure_message,
    offline_vlda_heldout_split_status, offline_vlda_success_label_failure_message,
    offline_vlda_success_label_status, offline_vlda_train_split_pid_status,
    read_offline_vlda_dataset, run_offline_vlda_harness_with_options,
    write_offline_pid_uncertainty, write_offline_vlda_runlog_with_options,
    write_offline_vlda_summary, OfflineVldaHarnessOptions, OfflineVldaRunlogOptions,
    OfflineVldaUncertaintyConfig, PidMode,
};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
struct Args {
    input: PathBuf,
    summary_json: PathBuf,
    runlog: PathBuf,
    require_geometry_pass: bool,
    require_success_labels: bool,
    require_heldout_split: bool,
    require_heldout_class_coverage: bool,
    require_heldout_episode_disjoint: bool,
    require_axis_provenance_honest: bool,
    pid_mode: PidMode,
    discrete_bins: usize,
    pls_components: usize,
    bootstrap: usize,
    permutation: usize,
    uncertainty_block_size: usize,
    uncertainty_alpha: f64,
    uncertainty_json: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = parse_args(std::env::args().skip(1))?;
    let input_sha256 = pid_runlog::sha256_file(&args.input)
        .with_context(|| format!("failed to hash {}", args.input.display()))?;
    let dataset = read_offline_vlda_dataset(&args.input)?;
    let harness_options = OfflineVldaHarnessOptions {
        pid_mode: args.pid_mode,
        discrete_bins: args.discrete_bins,
        pls_components: args.pls_components,
    };
    let report = run_offline_vlda_harness_with_options(
        dataset.clone(),
        Some(args.input.display().to_string()),
        Some(input_sha256),
        &harness_options,
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
            require_heldout_class_coverage: args.require_heldout_class_coverage,
            require_heldout_episode_disjoint: args.require_heldout_episode_disjoint,
            require_axis_provenance_honest: args.require_axis_provenance_honest,
        },
    )?;

    // Opt-in PID-screen uncertainty: written to a dedicated file so the canonical
    // run-log / summary metric counts are never perturbed.
    let uncertainty_config = OfflineVldaUncertaintyConfig {
        n_boot: args.bootstrap,
        n_perm: args.permutation,
        block_size: args.uncertainty_block_size,
        alpha: args.uncertainty_alpha,
        ..Default::default()
    };
    if uncertainty_config.enabled() {
        let uncertainty =
            compute_offline_pid_uncertainty(&dataset, args.pid_mode, &uncertainty_config)?;
        let path = args
            .uncertainty_json
            .clone()
            .unwrap_or_else(|| args.summary_json.with_extension("uncertainty.json"));
        write_offline_pid_uncertainty(&path, &uncertainty)?;
        println!(
            "pid_uncertainty={} mode={} n_boot={} n_perm={} subsample_len={} pairs={}",
            path.display(),
            uncertainty.mode,
            uncertainty.n_boot,
            uncertainty.n_perm,
            uncertainty.subsample_len,
            uncertainty.pairs.len(),
        );
    }
    println!(
        "offline_vlda_summary={} runlog={} samples={} config_hash={} geometry_gate_status={} success_label_status={} heldout_split_status={} train_split_pid_status={} heldout_class_coverage_status={} heldout_episode_disjoint_status={}",
        args.summary_json.display(),
        args.runlog.display(),
        report.dims.samples,
        report.config_hash,
        report.geometry.gates.status,
        offline_vlda_success_label_status(&report),
        offline_vlda_heldout_split_status(&report),
        offline_vlda_train_split_pid_status(&report),
        offline_vlda_heldout_class_coverage_status(&report),
        offline_vlda_heldout_episode_disjoint_status(&report)
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
    if args.require_heldout_class_coverage
        && offline_vlda_heldout_class_coverage_status(&report) != "pass"
    {
        failures.push(offline_vlda_heldout_class_coverage_failure_message(&report));
    }
    if args.require_heldout_episode_disjoint
        && offline_vlda_heldout_episode_disjoint_status(&report) != "pass"
    {
        failures.push(offline_vlda_heldout_episode_disjoint_failure_message(
            &report,
        ));
    }
    if args.require_axis_provenance_honest {
        failures.extend(offline_vlda_axis_provenance_failure_messages(
            &report.axis_provenance,
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
    let mut require_heldout_class_coverage = false;
    let mut require_heldout_episode_disjoint = false;
    let mut require_axis_provenance_honest = false;
    let mut pid_mode = PidMode::Continuous;
    let mut discrete_bins: usize = 10;
    let mut pls_components: usize = 2;
    let mut bootstrap: usize = 0;
    let mut permutation: usize = 0;
    let mut uncertainty_block_size: usize = 1;
    let mut uncertainty_alpha: f64 = 0.05;
    let mut uncertainty_json: Option<PathBuf> = None;
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
            "--require-heldout-class-coverage" => {
                require_heldout_class_coverage = true;
            }
            "--require-heldout-episode-disjoint" => {
                require_heldout_episode_disjoint = true;
            }
            "--require-axis-provenance-honest" => {
                require_axis_provenance_honest = true;
            }
            "--pid-mode" => {
                let mode_str = iter
                    .next()
                    .context("--pid-mode requires 'continuous', 'discrete', or 'discrete-pls'")?;
                pid_mode = match mode_str.as_str() {
                    "continuous" => PidMode::Continuous,
                    "discrete" => PidMode::Discrete,
                    "discrete-pls" => PidMode::DiscretePls,
                    other => bail!(
                        "--pid-mode must be 'continuous', 'discrete', or 'discrete-pls', got '{other}'"
                    ),
                };
            }
            "--discrete-bins" => {
                let bins_str = iter.next().context("--discrete-bins requires a number")?;
                discrete_bins = bins_str
                    .parse::<usize>()
                    .with_context(|| format!("--discrete-bins: invalid number '{bins_str}'"))?;
                if discrete_bins < 2 {
                    bail!("--discrete-bins must be >= 2");
                }
            }
            "--pls-components" => {
                let components_str = iter.next().context("--pls-components requires a number")?;
                pls_components = components_str.parse::<usize>().with_context(|| {
                    format!("--pls-components: invalid number '{components_str}'")
                })?;
                if pls_components < 1 {
                    bail!("--pls-components must be >= 1");
                }
            }
            "--bootstrap" => {
                let raw = iter.next().context("--bootstrap requires a number")?;
                bootstrap = raw
                    .parse::<usize>()
                    .with_context(|| format!("--bootstrap: invalid number '{raw}'"))?;
            }
            "--permutation" => {
                let raw = iter.next().context("--permutation requires a number")?;
                permutation = raw
                    .parse::<usize>()
                    .with_context(|| format!("--permutation: invalid number '{raw}'"))?;
            }
            "--uncertainty-block-size" => {
                let raw = iter
                    .next()
                    .context("--uncertainty-block-size requires a number")?;
                uncertainty_block_size = raw
                    .parse::<usize>()
                    .with_context(|| format!("--uncertainty-block-size: invalid number '{raw}'"))?;
                if uncertainty_block_size < 1 {
                    bail!("--uncertainty-block-size must be >= 1");
                }
            }
            "--uncertainty-alpha" => {
                let raw = iter
                    .next()
                    .context("--uncertainty-alpha requires a float in (0,1)")?;
                uncertainty_alpha = raw
                    .parse::<f64>()
                    .with_context(|| format!("--uncertainty-alpha: invalid float '{raw}'"))?;
                if !(uncertainty_alpha > 0.0 && uncertainty_alpha < 1.0) {
                    bail!("--uncertainty-alpha must be in (0,1)");
                }
            }
            "--uncertainty-json" => {
                uncertainty_json = Some(PathBuf::from(
                    iter.next().context("--uncertainty-json requires a path")?,
                ));
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
        require_heldout_class_coverage,
        require_heldout_episode_disjoint,
        require_axis_provenance_honest,
        pid_mode,
        discrete_bins,
        pls_components,
        bootstrap,
        permutation,
        uncertainty_block_size,
        uncertainty_alpha,
        uncertainty_json,
    })
}

fn print_usage() {
    println!(
        "Usage: pid-offline-harness --input PATH [--summary-json PATH] [--runlog PATH] [--require-geometry-pass] [--require-success-labels] [--require-heldout-split] [--require-heldout-class-coverage] [--require-heldout-episode-disjoint] [--require-axis-provenance-honest] [--pid-mode continuous|discrete|discrete-pls] [--discrete-bins N] [--pls-components N] [--bootstrap N] [--permutation N] [--uncertainty-block-size N] [--uncertainty-alpha F] [--uncertainty-json PATH]\n\
         \n\
         Converts captured (V,L,D,A) embedding JSON into canonical summary and run-log artifacts.\n\
         \n\
         --pid-mode continuous   Use KSG kNN-based MI and continuous I^sx PID (default).\n\
         --pid-mode discrete     Use equal-width quantization + counting-based discrete PID\n\
                                 (I_min-style redundancy, not discrete i^sx; results carry\n\
                                 saturation diagnostics — see grandplan §8.1.6).\n\
         --pid-mode discrete-pls PLS-project V/L/D toward A, then discrete PID on the\n\
                                 projections (fit is in-sample for the all-samples screen;\n\
                                 train-only for the train-split screen).\n\
         --discrete-bins N       Number of bins for discrete modes (default: 10, min: 2).\n\
         --pls-components N      PLS components for discrete-pls (default: 2, min: 1)."
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
            "--require-heldout-class-coverage".to_string(),
            "--require-heldout-episode-disjoint".to_string(),
        ])
        .unwrap();
        assert_eq!(args.input, PathBuf::from("fixture.json"));
        assert_eq!(args.summary_json, PathBuf::from("summary.json"));
        assert_eq!(args.runlog, PathBuf::from("runlog.jsonl"));
        assert!(args.require_geometry_pass);
        assert!(args.require_success_labels);
        assert!(args.require_heldout_split);
        assert!(args.require_heldout_class_coverage);
        assert!(args.require_heldout_episode_disjoint);
        assert_eq!(args.pid_mode, PidMode::Continuous);
        assert_eq!(args.discrete_bins, 10);
        assert_eq!(args.pls_components, 2);
    }

    #[test]
    fn parse_args_accepts_discrete_pid_mode() {
        let args = parse_args([
            "--input".to_string(),
            "fixture.json".to_string(),
            "--pid-mode".to_string(),
            "discrete".to_string(),
            "--discrete-bins".to_string(),
            "20".to_string(),
        ])
        .unwrap();
        assert_eq!(args.pid_mode, PidMode::Discrete);
        assert_eq!(args.discrete_bins, 20);
    }

    #[test]
    fn parse_args_accepts_discrete_pls_pid_mode() {
        let args = parse_args([
            "--input".to_string(),
            "fixture.json".to_string(),
            "--pid-mode".to_string(),
            "discrete-pls".to_string(),
            "--pls-components".to_string(),
            "3".to_string(),
        ])
        .unwrap();
        assert_eq!(args.pid_mode, PidMode::DiscretePls);
        assert_eq!(args.pls_components, 3);
    }

    #[test]
    fn parse_args_rejects_unknown_pid_mode() {
        assert!(parse_args([
            "--input".to_string(),
            "fixture.json".to_string(),
            "--pid-mode".to_string(),
            "quantum".to_string(),
        ])
        .is_err());
    }
}
