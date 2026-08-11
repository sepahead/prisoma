use anyhow::{bail, Context, Result};
use pid_sim::offline_harness::{
    offline_vlda_axis_provenance_failure_messages,
    offline_vlda_heldout_class_coverage_failure_message,
    offline_vlda_heldout_class_coverage_status,
    offline_vlda_heldout_episode_disjoint_failure_message,
    offline_vlda_heldout_episode_disjoint_status, offline_vlda_heldout_split_failure_message,
    offline_vlda_heldout_split_status, offline_vlda_split_scientific_eligibility_failure_message,
    offline_vlda_success_label_failure_message, offline_vlda_success_label_status,
    offline_vlda_train_split_pid_status, read_offline_vlda_dataset_with_hash_and_limits,
    read_offline_vlda_resource_limits,
    run_offline_vlda_invocation_borrowed_with_options_and_limits, write_offline_pid_uncertainty,
    write_offline_vlda_runlog_with_options_and_uncertainty, write_offline_vlda_summary,
    OfflineVldaHarnessOptions, OfflineVldaResourceLimits, OfflineVldaRunlogArtifacts,
    OfflineVldaRunlogOptions, OfflineVldaUncertaintyConfig, PermutationScheme, PidMode,
    PlsComponentSelection,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
struct Args {
    input: PathBuf,
    resource_limits_json: Option<PathBuf>,
    summary_json: PathBuf,
    runlog: PathBuf,
    require_success_labels: bool,
    require_heldout_split: bool,
    require_heldout_class_coverage: bool,
    require_heldout_episode_disjoint: bool,
    require_axis_provenance_honest: bool,
    pid_mode: PidMode,
    discrete_bins: usize,
    pls: PlsComponentSelection,
    bootstrap: usize,
    permutation: usize,
    uncertainty_block_size: usize,
    uncertainty_alpha: f64,
    permutation_scheme_circular: bool,
    uncertainty_json: Option<PathBuf>,
}

const DEFAULT_RESOURCE_LIMITS_JSON_EXAMPLE: &str = r#"{
  "max_input_bytes": 67108864,
  "max_samples": 1024,
  "max_total_axis_scalars": 1048576,
  "max_total_metadata_entries": 65536,
  "max_total_metadata_json_nodes": 65536,
  "max_total_metadata_utf8_bytes": 8388608,
  "max_metadata_json_depth": 64,
  "max_pairwise_distance_evaluations": 50000000,
  "max_distance_coordinate_evaluations": 100000000,
  "max_dense_solver_operations": 100000000
}"#;

fn main() -> Result<()> {
    let args = parse_args(std::env::args().skip(1))?;
    let uncertainty_path = (args.bootstrap > 0 || args.permutation > 0).then(|| {
        args.uncertainty_json
            .clone()
            .unwrap_or_else(|| args.summary_json.with_extension("uncertainty.json"))
    });
    ensure_distinct_paths(&args, uncertainty_path.as_deref())?;
    let resource_limits = match &args.resource_limits_json {
        Some(path) => read_offline_vlda_resource_limits(path)?,
        None => OfflineVldaResourceLimits::default(),
    };
    let input_uri = args
        .input
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("--input path must be valid UTF-8 for run-log provenance"))?
        .to_string();
    let (dataset, input_sha256) =
        read_offline_vlda_dataset_with_hash_and_limits(&args.input, &resource_limits)?;
    let harness_options = OfflineVldaHarnessOptions {
        pid_mode: args.pid_mode,
        discrete_bins: args.discrete_bins,
        pls: args.pls,
    };
    let uncertainty_config = OfflineVldaUncertaintyConfig {
        n_boot: args.bootstrap,
        n_perm: args.permutation,
        block_size: args.uncertainty_block_size,
        alpha: args.uncertainty_alpha,
        permutation_scheme: if args.permutation_scheme_circular {
            // min_shift = the block size: the same dependence length the user
            // already sizes for the moving-block bootstrap.
            PermutationScheme::CircularShift {
                min_shift: args.uncertainty_block_size,
            }
        } else {
            PermutationScheme::FullShuffle
        },
        ..Default::default()
    };
    let invocation = run_offline_vlda_invocation_borrowed_with_options_and_limits(
        &dataset,
        Some(input_uri),
        Some(input_sha256),
        &harness_options,
        &uncertainty_config,
        &resource_limits,
    )?;
    let report = invocation.report;
    let uncertainty_output = if let Some(uncertainty) = invocation.uncertainty {
        let path = uncertainty_path
            .clone()
            .context("enabled uncertainty output path was not resolved")?;
        Some((path, uncertainty))
    } else {
        None
    };
    write_offline_vlda_summary(&args.summary_json, &report)?;
    if let Some((path, uncertainty)) = &uncertainty_output {
        write_offline_pid_uncertainty(path, uncertainty)?;
    }
    write_offline_vlda_runlog_with_options_and_uncertainty(
        &args.runlog,
        OfflineVldaRunlogArtifacts {
            summary_path: Some(&args.summary_json),
            input_path: Some(&args.input),
            uncertainty_path: uncertainty_output.as_ref().map(|(path, _)| path.as_path()),
            uncertainty: uncertainty_output
                .as_ref()
                .map(|(_, uncertainty)| uncertainty),
        },
        &dataset,
        &report,
        OfflineVldaRunlogOptions {
            require_success_labels: args.require_success_labels,
            require_heldout_split: args.require_heldout_split,
            require_heldout_class_coverage: args.require_heldout_class_coverage,
            require_heldout_episode_disjoint: args.require_heldout_episode_disjoint,
            require_axis_provenance_honest: args.require_axis_provenance_honest,
        },
    )?;

    if let Some((path, uncertainty)) = &uncertainty_output {
        println!(
            "pid_uncertainty={} mode={} stability_interpretation={} n_boot={} n_perm={} perm_scheme={} subsample_len={} pairs={}",
            path.display(),
            uncertainty.mode,
            uncertainty.stability_interpretation,
            uncertainty.n_boot,
            uncertainty.n_perm,
            uncertainty.permutation_scheme,
            uncertainty.subsample_len,
            uncertainty.pairs.len(),
        );
    }
    println!(
        "offline_vlda_summary={} runlog={} samples={} config_hash={} geometry_diagnostic_status={} success_label_status={} heldout_split_status={} train_split_pid_status={} heldout_class_coverage_status={} heldout_episode_disjoint_status={}",
        args.summary_json.display(),
        args.runlog.display(),
        report.dims.samples,
        report.config_hash,
        report.geometry.diagnostics.status,
        offline_vlda_success_label_status(&report),
        offline_vlda_heldout_split_status(&report),
        offline_vlda_train_split_pid_status(&report),
        offline_vlda_heldout_class_coverage_status(&report),
        offline_vlda_heldout_episode_disjoint_status(&report)
    );
    let mut failures = Vec::new();
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
    if args.require_heldout_split
        || args.require_heldout_class_coverage
        || args.require_heldout_episode_disjoint
    {
        if let Some(message) = offline_vlda_split_scientific_eligibility_failure_message(&dataset) {
            failures.push(message);
        }
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
    let mut resource_limits_json = None;
    let mut summary_json = PathBuf::from("outputs/offline_vlda_summary.json");
    let mut runlog = PathBuf::from("outputs/offline_vlda_runlog.jsonl");
    let mut require_success_labels = false;
    let mut require_heldout_split = false;
    let mut require_heldout_class_coverage = false;
    let mut require_heldout_episode_disjoint = false;
    let mut require_axis_provenance_honest = false;
    let mut pid_mode = PidMode::Disabled;
    let mut discrete_bins: usize = 10;
    let mut pls = PlsComponentSelection::Fixed(2);
    let mut bootstrap: usize = 0;
    let mut permutation: usize = 0;
    let mut uncertainty_block_size: usize = 1;
    let mut uncertainty_alpha: f64 = 0.05;
    let mut permutation_scheme_circular = false;
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
            "--resource-limits-json" => {
                resource_limits_json = Some(PathBuf::from(
                    iter.next()
                        .context("--resource-limits-json requires a path")?,
                ));
            }
            "--summary-json" => {
                summary_json =
                    PathBuf::from(iter.next().context("--summary-json requires a path")?);
            }
            "--runlog" => {
                runlog = PathBuf::from(iter.next().context("--runlog requires a path")?);
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
                let mode_str = iter.next().context(
                    "--pid-mode requires 'none', 'continuous', 'discrete', or 'discrete-pls'",
                )?;
                pid_mode = match mode_str.as_str() {
                    "none" => PidMode::Disabled,
                    "continuous" => PidMode::Continuous,
                    "discrete" => PidMode::Discrete,
                    "discrete-pls" => PidMode::DiscretePls,
                    other => bail!(
                        "--pid-mode must be 'none', 'continuous', 'discrete', or 'discrete-pls', got '{other}'"
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
                let raw = iter
                    .next()
                    .context("--pls-components requires a number, 'cv', or 'cv:MAX'")?;
                pls = if raw == "cv" {
                    PlsComponentSelection::CvQ2 { max_components: 8 }
                } else if let Some(max) = raw.strip_prefix("cv:") {
                    let max = max
                        .parse::<usize>()
                        .with_context(|| format!("--pls-components: invalid cv max '{raw}'"))?;
                    if max < 1 {
                        bail!("--pls-components cv:MAX must have MAX >= 1");
                    }
                    PlsComponentSelection::CvQ2 {
                        max_components: max,
                    }
                } else {
                    let k = raw
                        .parse::<usize>()
                        .with_context(|| format!("--pls-components: invalid number '{raw}'"))?;
                    if k < 1 {
                        bail!("--pls-components must be >= 1");
                    }
                    PlsComponentSelection::Fixed(k)
                };
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
            "--permutation-scheme" => {
                let raw = iter
                    .next()
                    .context("--permutation-scheme requires full-shuffle|circular-shift")?;
                permutation_scheme_circular = match raw.as_str() {
                    "full-shuffle" => false,
                    "circular-shift" => true,
                    other => bail!(
                        "--permutation-scheme: expected full-shuffle|circular-shift, got '{other}'"
                    ),
                };
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
    if uncertainty_json.is_some() && bootstrap == 0 && permutation == 0 {
        bail!("--uncertainty-json requires --bootstrap N or --permutation N with N > 0");
    }
    Ok(Args {
        input,
        resource_limits_json,
        summary_json,
        runlog,
        require_success_labels,
        require_heldout_split,
        require_heldout_class_coverage,
        require_heldout_episode_disjoint,
        require_axis_provenance_honest,
        pid_mode,
        discrete_bins,
        pls,
        bootstrap,
        permutation,
        uncertainty_block_size,
        uncertainty_alpha,
        permutation_scheme_circular,
        uncertainty_json,
    })
}

fn comparable_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if absolute.exists() {
        return absolute
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", absolute.display()));
    }
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    if let (Some(parent), Some(name)) = (normalized.parent(), normalized.file_name()) {
        if let Ok(parent) = parent.canonicalize() {
            return Ok(parent.join(name));
        }
    }
    Ok(normalized)
}

fn paths_alias(left: &Path, right: &Path) -> Result<bool> {
    if left.exists() && right.exists() {
        return same_file::is_same_file(left, right).with_context(|| {
            format!(
                "failed to compare path identities {} and {}",
                left.display(),
                right.display()
            )
        });
    }
    Ok(comparable_path(left)? == comparable_path(right)?)
}

fn ensure_distinct_paths(args: &Args, uncertainty_path: Option<&Path>) -> Result<()> {
    let mut paths = vec![
        ("input", args.input.as_path(), false),
        ("summary", args.summary_json.as_path(), true),
        ("runlog", args.runlog.as_path(), true),
    ];
    if let Some(path) = args.resource_limits_json.as_deref() {
        paths.push(("resource limits", path, false));
    }
    if let Some(path) = uncertainty_path {
        paths.push(("uncertainty", path, true));
    }

    for (name, path, output) in &paths {
        if *output
            && std::fs::symlink_metadata(path)
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            bail!("{name} output must not be a symlink: {}", path.display());
        }
    }
    for (index, (left_name, left, _)) in paths.iter().enumerate() {
        for (right_name, right, _) in &paths[index + 1..] {
            if paths_alias(left, right)? {
                bail!(
                    "{left_name} and {right_name} paths must be distinct: {}",
                    left.display()
                );
            }
        }
    }
    Ok(())
}

fn print_usage() {
    println!(
        "Usage: pid-offline-harness --input PATH [--resource-limits-json PATH] [--summary-json PATH] [--runlog PATH] [--require-success-labels] [--require-heldout-split] [--require-heldout-class-coverage] [--require-heldout-episode-disjoint] [--require-axis-provenance-honest] [--pid-mode none|continuous|discrete|discrete-pls] [--discrete-bins N] [--pls-components N|cv|cv:MAX] [--bootstrap N] [--permutation N] [--permutation-scheme full-shuffle|circular-shift] [--uncertainty-block-size N] [--uncertainty-alpha F] [--uncertainty-json PATH]\n\
         \n\
         Converts captured (V,L,D,A) embedding JSON into canonical summary and run-log artifacts.\n\
         Geometry output is descriptive. Its warnings never establish or block estimator validity.\n\
         \n\
         --resource-limits-json PATH\n\
                                 Use a reviewed, strict JSON override for the typed resource\n\
                                 limits. Defaults cap input at 64 MiB, samples at 1,024, pairwise\n\
                                 work at 50,000,000, coordinate work at 100,000,000, and dense-\n\
                                 solver work at 100,000,000. The CLI\n\
                                 checked-adds main and optional uncertainty projections before\n\
                                 analysis. Applied values and usage are bound into report and\n\
                                 run-log configuration. Larger SAFE or\n\
                                 NCP artifacts need a reviewed complete override.\n\
         --pid-mode none         Skip all MI/PID estimates; run labels, geometry, and non-PID\n\
                                 prediction baselines only (default estimator-request firebreak).\n\
         --pid-mode continuous   Opt in to KSG kNN-based MI and continuous I^sx PID.\n\
         --pid-mode discrete     Use equal-width quantization + counting-based discrete PID\n\
                                 (I_min-style redundancy, not discrete i^sx; results carry\n\
                                 saturation diagnostics — see grandplan §7.6).\n\
         --pid-mode discrete-pls PLS-project V/L/D toward A, then discrete PID on the\n\
                                 projections (fit is in-sample for the all-samples screen;\n\
                                 train-only for the train-split screen).\n\
         --discrete-bins N       Number of bins for discrete modes (default: 10, min: 2).\n\
         --pls-components X      PLS components for discrete-pls: a fixed count N
                                 (default: 2), or 'cv' / 'cv:MAX' for per-source LOO-CV
                                 Q² selection (default MAX: 8). This is the preregistered
                                 grandplan §6.2 fitted preprocessing method. In discrete-pls
                                 mode, the summary also carries a shuffled-target permutation
                                 control. Read the real atoms relative to that
                                 selection-inflation floor. Treat in-sample discrete-pls
                                 output as screening-only.\n\
         --bootstrap N           Number of m-out-of-n subsample resamples. The emitted raw\n\
                                 percentiles are stability envelopes at m, not calibrated\n\
                                 n-sample confidence intervals. Continuous mode requires N >= 2.\n\
         --uncertainty-block-size N\n\
                                 Predeclared dependence length (default: 1). Bootstrap use\n\
                                 requires N <= floor(samples/2). Circular shift also requires\n\
                                 samples >= 2*N+1.\n\
         --uncertainty-alpha F   Two-sided tail mass for those raw percentiles (default: 0.05);\n\
                                 it is not a confidence-interval significance claim.\n\
         --permutation-scheme    Null for --permutation p-values (default: full-shuffle).\n\
                                 full-shuffle assumes exchangeable (i.i.d.) rows and is\n\
                                 anti-conservative on autocorrelated per-step captures;\n\
                                 circular-shift preserves each source's own serial\n\
                                 dependence (rotation null; min_shift = the\n\
                                 --uncertainty-block-size dependence length)."
    );
    println!(
        "\nComplete default --resource-limits-json document:\n{DEFAULT_RESOURCE_LIMITS_JSON_EXAMPLE}"
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
            "--resource-limits-json".to_string(),
            "limits.json".to_string(),
            "--summary-json".to_string(),
            "summary.json".to_string(),
            "--runlog".to_string(),
            "runlog.jsonl".to_string(),
            "--require-success-labels".to_string(),
            "--require-heldout-split".to_string(),
            "--require-heldout-class-coverage".to_string(),
            "--require-heldout-episode-disjoint".to_string(),
        ])
        .unwrap();
        assert_eq!(args.input, PathBuf::from("fixture.json"));
        assert_eq!(
            args.resource_limits_json,
            Some(PathBuf::from("limits.json"))
        );
        assert_eq!(args.summary_json, PathBuf::from("summary.json"));
        assert_eq!(args.runlog, PathBuf::from("runlog.jsonl"));
        assert!(args.require_success_labels);
        assert!(args.require_heldout_split);
        assert!(args.require_heldout_class_coverage);
        assert!(args.require_heldout_episode_disjoint);
        assert_eq!(args.pid_mode, PidMode::Disabled);
        assert_eq!(args.discrete_bins, 10);
        assert_eq!(args.pls, PlsComponentSelection::Fixed(2));
    }

    fn write_limits_fixture(bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("limits.json");
        std::fs::write(&path, bytes).unwrap();
        (dir, path)
    }

    #[test]
    fn read_resource_limits_accepts_complete_positive_typed_file() {
        let expected = OfflineVldaResourceLimits::default();
        let (_dir, path) = write_limits_fixture(&serde_json::to_vec(&expected).unwrap());

        let observed = read_offline_vlda_resource_limits(&path).unwrap();

        assert_eq!(observed, expected);
    }

    #[test]
    fn help_resource_limits_example_matches_typed_defaults() {
        let observed: OfflineVldaResourceLimits =
            serde_json::from_str(DEFAULT_RESOURCE_LIMITS_JSON_EXAMPLE).unwrap();

        assert_eq!(observed, OfflineVldaResourceLimits::default());
    }

    #[test]
    fn read_resource_limits_accepts_reviewed_safe_size_override() {
        let expected = OfflineVldaResourceLimits {
            max_samples: 250_000,
            max_total_axis_scalars: 10_000_000,
            max_pairwise_distance_evaluations: 100_000_000_000,
            max_distance_coordinate_evaluations: 1_000_000_000_000,
            max_dense_solver_operations: 1_000_000_000_000,
            ..OfflineVldaResourceLimits::default()
        };
        let (_dir, path) = write_limits_fixture(&serde_json::to_vec(&expected).unwrap());

        let observed = read_offline_vlda_resource_limits(&path).unwrap();

        assert_eq!(observed, expected);
    }

    #[test]
    fn read_resource_limits_rejects_missing_required_field() {
        let mut value = serde_json::to_value(OfflineVldaResourceLimits::default()).unwrap();
        value.as_object_mut().unwrap().remove("max_samples");
        let (_dir, path) = write_limits_fixture(&serde_json::to_vec(&value).unwrap());

        let error = read_offline_vlda_resource_limits(&path).unwrap_err();

        assert!(error.to_string().contains("failed to parse"));
    }

    #[test]
    fn read_resource_limits_rejects_unknown_field() {
        let mut value = serde_json::to_value(OfflineVldaResourceLimits::default()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unreviewed_limit".to_string(), serde_json::json!(1));
        let (_dir, path) = write_limits_fixture(&serde_json::to_vec(&value).unwrap());

        let error = read_offline_vlda_resource_limits(&path).unwrap_err();

        assert!(error.to_string().contains("failed to parse"));
    }

    #[test]
    fn read_resource_limits_rejects_zero_field() {
        let limits = OfflineVldaResourceLimits {
            max_samples: 0,
            ..OfflineVldaResourceLimits::default()
        };
        let (_dir, path) = write_limits_fixture(&serde_json::to_vec(&limits).unwrap());

        let error = read_offline_vlda_resource_limits(&path).unwrap_err();

        assert!(error.to_string().contains("max_samples"));
        assert!(error.to_string().contains("observed 0"));
    }

    #[test]
    fn read_resource_limits_rejects_malformed_json() {
        let (_dir, path) = write_limits_fixture(b"{");

        let error = read_offline_vlda_resource_limits(&path).unwrap_err();

        assert!(error.to_string().contains("not strict JSON"));
    }

    #[test]
    fn read_resource_limits_rejects_duplicate_fields() {
        let encoded = serde_json::to_string(&OfflineVldaResourceLimits::default()).unwrap();
        let duplicate = encoded.replacen(
            "\"max_samples\":1024",
            "\"max_samples\":1024,\"max_samples\":1",
            1,
        );
        assert_ne!(duplicate, encoded);
        let (_dir, path) = write_limits_fixture(duplicate.as_bytes());

        let error = read_offline_vlda_resource_limits(&path).unwrap_err();

        assert!(error.to_string().contains("not strict JSON"));
    }

    #[test]
    fn read_resource_limits_rejects_file_byte_limit_before_json_parse() {
        let oversized = vec![b' '; 64 * 1_024 + 1];
        let (_dir, path) = write_limits_fixture(&oversized);

        let error = read_offline_vlda_resource_limits(&path).unwrap_err();

        assert!(error.to_string().contains("exceeds the 65536-byte limit"));
        assert!(!error.to_string().contains("failed to parse"));
    }

    #[test]
    fn read_resource_limits_rejects_integer_type_overflow() {
        let encoded = serde_json::to_string(&OfflineVldaResourceLimits::default()).unwrap();
        let extreme = encoded.replacen(
            "\"max_samples\":1024",
            "\"max_samples\":184467440737095516160",
            1,
        );
        assert_ne!(extreme, encoded);
        let (_dir, path) = write_limits_fixture(extreme.as_bytes());

        let error = read_offline_vlda_resource_limits(&path).unwrap_err();

        assert!(error.to_string().contains("failed to parse"));
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
    fn parse_args_accepts_pid_disabled_mode() {
        let args = parse_args([
            "--input".to_string(),
            "fixture.json".to_string(),
            "--pid-mode".to_string(),
            "none".to_string(),
        ])
        .unwrap();
        assert_eq!(args.pid_mode, PidMode::Disabled);
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
        assert_eq!(args.pls, PlsComponentSelection::Fixed(3));
    }

    #[test]
    fn parse_args_accepts_cv_component_selection() {
        let args = parse_args([
            "--input".to_string(),
            "fixture.json".to_string(),
            "--pid-mode".to_string(),
            "discrete-pls".to_string(),
            "--pls-components".to_string(),
            "cv".to_string(),
        ])
        .unwrap();
        assert_eq!(args.pls, PlsComponentSelection::CvQ2 { max_components: 8 });
        let args = parse_args([
            "--input".to_string(),
            "fixture.json".to_string(),
            "--pls-components".to_string(),
            "cv:4".to_string(),
        ])
        .unwrap();
        assert_eq!(args.pls, PlsComponentSelection::CvQ2 { max_components: 4 });
        assert!(parse_args([
            "--input".to_string(),
            "fixture.json".to_string(),
            "--pls-components".to_string(),
            "cv:0".to_string(),
        ])
        .is_err());
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

    #[test]
    fn parse_args_rejects_unused_uncertainty_output() {
        let error = parse_args([
            "--input".to_string(),
            "fixture.json".to_string(),
            "--uncertainty-json".to_string(),
            "uncertainty.json".to_string(),
        ])
        .unwrap_err();

        assert!(error.to_string().contains("requires --bootstrap"));
    }

    #[test]
    fn output_paths_must_be_distinct_before_any_write() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.json");
        let output = directory.path().join("output.json");
        std::fs::write(&input, b"{}").unwrap();
        let args = parse_args([
            "--input".to_string(),
            input.display().to_string(),
            "--summary-json".to_string(),
            output.display().to_string(),
            "--runlog".to_string(),
            output.display().to_string(),
        ])
        .unwrap();

        let error = ensure_distinct_paths(&args, None).unwrap_err();

        assert!(error.to_string().contains("summary and runlog"));
        assert!(!output.exists());
    }

    #[cfg(unix)]
    #[test]
    fn existing_hardlink_alias_cannot_overwrite_input() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.json");
        let summary = directory.path().join("summary.json");
        let runlog = directory.path().join("runlog.jsonl");
        std::fs::write(&input, b"protected input").unwrap();
        std::fs::hard_link(&input, &summary).unwrap();
        let args = parse_args([
            "--input".to_string(),
            input.display().to_string(),
            "--summary-json".to_string(),
            summary.display().to_string(),
            "--runlog".to_string(),
            runlog.display().to_string(),
        ])
        .unwrap();

        let error = ensure_distinct_paths(&args, None).unwrap_err();

        assert!(error.to_string().contains("input and summary"));
        assert_eq!(std::fs::read(&input).unwrap(), b"protected input");
    }

    #[test]
    fn existing_distinct_outputs_allow_reproducible_refresh() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("input.json");
        let summary = directory.path().join("summary.json");
        let runlog = directory.path().join("runlog.jsonl");
        std::fs::write(&input, b"{}").unwrap();
        std::fs::write(&summary, b"old summary").unwrap();
        std::fs::write(&runlog, b"old run log").unwrap();
        let args = parse_args([
            "--input".to_string(),
            input.display().to_string(),
            "--summary-json".to_string(),
            summary.display().to_string(),
            "--runlog".to_string(),
            runlog.display().to_string(),
        ])
        .unwrap();

        ensure_distinct_paths(&args, None).unwrap();
    }
}
