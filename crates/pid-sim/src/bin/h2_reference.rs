use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, Context, Result};
use pid_runlog::{RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use pid_sim::h2_reference::{
    run_h2_reference, H2AlarmResult, H2AnalysisPlan, H2Dataset, H2EventOntology, H2FeatureContract,
    H2FoldOutcome, H2ModelKind, H2ReferenceInput, H2ReferenceReport, H2SplitManifest,
    H2_REFERENCE_SCHEMA_VERSION,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const COMPONENT: &str = "pid-h2-reference";
const MAX_DATASET_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct Args {
    dataset: PathBuf,
    analysis_plan: PathBuf,
    event_ontology: PathBuf,
    feature_contract: PathBuf,
    split_manifest: PathBuf,
    summary_json: PathBuf,
    runlog: PathBuf,
}

#[derive(Debug)]
struct ExactSnapshot {
    bytes: Option<Vec<u8>>,
    sha256: Option<String>,
    byte_len: u64,
}

struct TemporaryPathGuard {
    path: PathBuf,
    installed: bool,
}

impl TemporaryPathGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            installed: false,
        }
    }
}

impl Drop for TemporaryPathGuard {
    fn drop(&mut self) {
        if !self.installed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
struct CliIssue {
    code: String,
    field: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct H2Summary {
    schema_version: u32,
    run_id: String,
    dataset_uri: String,
    dataset_sha256: Option<String>,
    analysis_plan_sha256: Option<String>,
    event_ontology_sha256: Option<String>,
    feature_contract_sha256: Option<String>,
    split_manifest_sha256: Option<String>,
    config_hash: String,
    parsed: bool,
    passed: bool,
    synthetic_fixture_only: bool,
    establishes_h2_evidence: bool,
    prospective_capture: bool,
    external_validation: bool,
    comparator_frontier_complete: bool,
    pid_dependency: String,
    report: Option<H2ReferenceReport>,
    fatal_issues: Vec<CliIssue>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct H2Verdict<'a> {
    schema_version: u32,
    passed: bool,
    synthetic_fixture_only: bool,
    establishes_h2_evidence: bool,
    prospective_capture: bool,
    external_validation: bool,
    comparator_frontier_complete: bool,
    pid_dependency: &'static str,
    dataset_sha256: Option<&'a str>,
    summary_sha256: &'a str,
    issue_count: usize,
    reason_codes: Vec<String>,
    denominators: Option<&'a pid_sim::h2_reference::H2Denominators>,
}

#[derive(Debug)]
struct Snapshots {
    dataset: ExactSnapshot,
    analysis_plan: ExactSnapshot,
    event_ontology: ExactSnapshot,
    feature_contract: ExactSnapshot,
    split_manifest: ExactSnapshot,
}

fn main() -> Result<()> {
    let Some(args) = parse_args()? else {
        print_usage();
        return Ok(());
    };
    ensure_parent(&args.summary_json)?;
    ensure_parent(&args.runlog)?;
    ensure_distinct_paths(&args)?;

    let snapshots = Snapshots {
        dataset: read_exact_snapshot(&args.dataset, MAX_DATASET_BYTES)?,
        analysis_plan: read_exact_snapshot(&args.analysis_plan, MAX_ARTIFACT_BYTES)?,
        event_ontology: read_exact_snapshot(&args.event_ontology, MAX_ARTIFACT_BYTES)?,
        feature_contract: read_exact_snapshot(&args.feature_contract, MAX_ARTIFACT_BYTES)?,
        split_manifest: read_exact_snapshot(&args.split_manifest, MAX_ARTIFACT_BYTES)?,
    };
    let dataset_sha256 = snapshots.dataset.sha256.clone();
    let run_id = dataset_sha256.as_ref().map_or_else(
        || "h2-reference-resource-limited".to_string(),
        |sha256| format!("h2-reference-{}", &sha256[..16]),
    );
    let (input, mut fatal_issues) = parse_and_bind(&args, &snapshots);
    let parsed = input.is_some();
    let report = input.as_ref().map(run_h2_reference);
    let config = compact_config(input.as_ref(), &snapshots);
    let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
    let passed =
        report.as_ref().is_some_and(H2ReferenceReport::is_valid) && fatal_issues.is_empty();
    if report.is_none() && fatal_issues.is_empty() {
        fatal_issues.push(CliIssue {
            code: "contract_parse_failed".to_string(),
            field: "input".to_string(),
            message: "H2 input artifacts could not be assembled".to_string(),
        });
    }
    let summary = H2Summary {
        schema_version: H2_REFERENCE_SCHEMA_VERSION,
        run_id: run_id.clone(),
        dataset_uri: args.dataset.display().to_string(),
        dataset_sha256: dataset_sha256.clone(),
        analysis_plan_sha256: snapshots.analysis_plan.sha256.clone(),
        event_ontology_sha256: snapshots.event_ontology.sha256.clone(),
        feature_contract_sha256: snapshots.feature_contract.sha256.clone(),
        split_manifest_sha256: snapshots.split_manifest.sha256.clone(),
        config_hash: config_hash.clone(),
        parsed,
        passed,
        synthetic_fixture_only: true,
        establishes_h2_evidence: false,
        prospective_capture: false,
        external_validation: false,
        comparator_frontier_complete: false,
        pid_dependency: "none".to_string(),
        report,
        fatal_issues,
    };
    let summary_bytes = serde_json::to_vec_pretty(&summary)?;
    let summary_sha256 = sha256_bytes(&summary_bytes);
    write_atomic_bytes(&args.summary_json, &summary_bytes)?;
    write_runlog(
        &args,
        &run_id,
        &config_hash,
        config,
        &summary,
        input.as_ref(),
        &summary_sha256,
    )?;

    println!("run_id={run_id}");
    println!("parsed={}", summary.parsed);
    println!("passed={}", summary.passed);
    println!("synthetic_fixture_only=true");
    println!("establishes_h2_evidence=false");
    println!("prospective_capture=false");
    println!("wrote_summary={}", args.summary_json.display());
    println!("wrote_runlog={}", args.runlog.display());
    if !summary.passed {
        bail!("H2 synthetic reference failed closed; inspect the summary and run log");
    }
    Ok(())
}

fn parse_and_bind(args: &Args, snapshots: &Snapshots) -> (Option<H2ReferenceInput>, Vec<CliIssue>) {
    let mut issues = Vec::new();
    let dataset = parse_snapshot::<H2Dataset>(
        &snapshots.dataset,
        "dataset",
        MAX_DATASET_BYTES,
        &mut issues,
    );
    let plan = parse_snapshot::<H2AnalysisPlan>(
        &snapshots.analysis_plan,
        "analysis_plan",
        MAX_ARTIFACT_BYTES,
        &mut issues,
    );
    let ontology = parse_snapshot::<H2EventOntology>(
        &snapshots.event_ontology,
        "event_ontology",
        MAX_ARTIFACT_BYTES,
        &mut issues,
    );
    let feature_contract = parse_snapshot::<H2FeatureContract>(
        &snapshots.feature_contract,
        "feature_contract",
        MAX_ARTIFACT_BYTES,
        &mut issues,
    );
    let split_manifest = parse_snapshot::<H2SplitManifest>(
        &snapshots.split_manifest,
        "split_manifest",
        MAX_ARTIFACT_BYTES,
        &mut issues,
    );
    if let Some(dataset) = &dataset {
        for (field, binding, path, snapshot) in [
            (
                "bindings.analysis_plan",
                &dataset.bindings.analysis_plan,
                &args.analysis_plan,
                &snapshots.analysis_plan,
            ),
            (
                "bindings.event_ontology",
                &dataset.bindings.event_ontology,
                &args.event_ontology,
                &snapshots.event_ontology,
            ),
            (
                "bindings.feature_contract",
                &dataset.bindings.feature_contract,
                &args.feature_contract,
                &snapshots.feature_contract,
            ),
            (
                "bindings.split_manifest",
                &dataset.bindings.split_manifest,
                &args.split_manifest,
                &snapshots.split_manifest,
            ),
        ] {
            if binding.artifact_uri != path.to_string_lossy() {
                push_issue(
                    &mut issues,
                    "artifact_uri_mismatch",
                    field,
                    format!(
                        "binding names {:?}, CLI supplied {:?}",
                        binding.artifact_uri,
                        path.display()
                    ),
                );
            }
            match snapshot.sha256.as_deref() {
                Some(actual) if actual == binding.sha256 => {}
                Some(actual) => push_issue(
                    &mut issues,
                    "artifact_hash_mismatch",
                    field,
                    format!("expected {}, captured {actual}", binding.sha256),
                ),
                None => push_issue(
                    &mut issues,
                    "artifact_resource_limit_exceeded",
                    field,
                    "artifact was not retained for exact-byte verification",
                ),
            }
        }
    }
    let input = match (dataset, plan, ontology, feature_contract, split_manifest) {
        (
            Some(dataset),
            Some(plan),
            Some(ontology),
            Some(feature_contract),
            Some(split_manifest),
        ) if issues.is_empty() => Some(H2ReferenceInput {
            dataset,
            plan,
            ontology,
            feature_contract,
            split_manifest,
        }),
        _ => None,
    };
    (input, issues)
}

fn parse_snapshot<T>(
    snapshot: &ExactSnapshot,
    field: &str,
    limit: u64,
    issues: &mut Vec<CliIssue>,
) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    let Some(bytes) = snapshot.bytes.as_deref() else {
        push_issue(
            issues,
            "artifact_resource_limit_exceeded",
            field,
            format!(
                "artifact is {} bytes; the limit is {limit}",
                snapshot.byte_len
            ),
        );
        return None;
    };
    match serde_json::from_slice(bytes) {
        Ok(value) => Some(value),
        Err(error) => {
            push_issue(issues, "contract_parse_failed", field, error.to_string());
            None
        }
    }
}

fn compact_config(input: Option<&H2ReferenceInput>, snapshots: &Snapshots) -> Value {
    let (episodes, landmarks, target_event_code, horizon_ns) = input.map_or_else(
        || (None, None, None, None),
        |input| {
            (
                Some(input.dataset.episodes.len()),
                input
                    .dataset
                    .episodes
                    .iter()
                    .try_fold(0_usize, |total, episode| {
                        total.checked_add(episode.landmarks.len())
                    }),
                Some(input.plan.estimand.target_event_code.as_str()),
                Some(input.plan.estimand.horizon_ns),
            )
        },
    );
    json!({
        "component": COMPONENT,
        "h2_reference_schema_version": H2_REFERENCE_SCHEMA_VERSION,
        "scope": "deterministic_synthetic_finite_landmark_benchmark_not_h2_evidence",
        "dataset_sha256": snapshots.dataset.sha256,
        "analysis_plan_sha256": snapshots.analysis_plan.sha256,
        "event_ontology_sha256": snapshots.event_ontology.sha256,
        "feature_contract_sha256": snapshots.feature_contract.sha256,
        "split_manifest_sha256": snapshots.split_manifest.sha256,
        "input_receipt": {
            "episode_count": episodes,
            "landmark_count": landmarks,
            "target_event_code": target_event_code,
            "horizon_ns": horizon_ns,
        },
        "configuration_storage": "compact_content_addressed_receipt",
        "pid_dependency": "none",
    })
}

fn write_runlog(
    args: &Args,
    run_id: &str,
    config_hash: &str,
    config: Value,
    summary: &H2Summary,
    input: Option<&H2ReferenceInput>,
    summary_sha256: &str,
) -> Result<()> {
    let metadata = [
        ("component".to_string(), COMPONENT.to_string()),
        ("claim".to_string(), "H2".to_string()),
        (
            "scope".to_string(),
            "deterministic_synthetic_fixture_not_h2_evidence".to_string(),
        ),
        ("pid_dependency".to_string(), "none".to_string()),
        (
            "timestamp_semantics".to_string(),
            "deterministic_event_index_not_capture_clock".to_string(),
        ),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    let temporary = temporary_path(&args.runlog, "runlog");
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("failed to create {}", temporary.display()))?;
    let mut guard = TemporaryPathGuard::new(temporary.clone());
    let mut writer = RunLogWriter::new(BufWriter::new(file));
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: run_id.to_string(),
        timestamp_ns: 0,
        config_hash: config_hash.to_string(),
        metadata: metadata.clone(),
    })?;
    writer.append(&RunLogEvent::ConfigLogged {
        timestamp_ns: 1,
        config_hash: config_hash.to_string(),
        config,
    })?;
    let mut timestamp_ns = 2_u64;
    for (name, kind, path, hash) in [
        (
            "h2_dataset",
            "h2_landmark_dataset_json",
            &args.dataset,
            summary.dataset_sha256.as_deref(),
        ),
        (
            "h2_analysis_plan",
            "h2_analysis_plan_json",
            &args.analysis_plan,
            summary.analysis_plan_sha256.as_deref(),
        ),
        (
            "h2_event_ontology",
            "h2_event_ontology_json",
            &args.event_ontology,
            summary.event_ontology_sha256.as_deref(),
        ),
        (
            "h2_feature_contract",
            "h2_feature_contract_json",
            &args.feature_contract,
            summary.feature_contract_sha256.as_deref(),
        ),
        (
            "h2_split_manifest",
            "h2_split_manifest_json",
            &args.split_manifest,
            summary.split_manifest_sha256.as_deref(),
        ),
        (
            "h2_summary",
            "h2_reference_summary_json",
            &args.summary_json,
            Some(summary_sha256),
        ),
    ] {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns,
            name: name.to_string(),
            kind: kind.to_string(),
            uri: path.display().to_string(),
            sha256: hash.map(str::to_string),
            metadata: metadata.clone(),
        })?;
        timestamp_ns += 1;
    }
    let mut step = 0_u64;
    if summary.passed {
        let input = input.context("passing H2 summary must retain its parsed input")?;
        let report = summary
            .report
            .as_ref()
            .context("passing H2 summary must include its report")?;
        for episode in &input.dataset.episodes {
            for landmark in &episode.landmarks {
                writer.append(&RunLogEvent::FrameObserved {
                    step,
                    timestamp_ns,
                    observation_hash: Some(pid_runlog::canonical_json_hash_v2(landmark)?),
                    metadata: [
                        ("episode_id".to_string(), episode.episode_id.clone()),
                        ("landmark_id".to_string(), landmark.landmark_id.clone()),
                        (
                            "source_timestamp_ns".to_string(),
                            landmark.time_ns.to_string(),
                        ),
                        ("scientific_evidence".to_string(), "false".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                })?;
                step += 1;
                timestamp_ns += 1;
            }
            for (name, event) in [
                (
                    "h2_reference.terminal_event",
                    episode.terminal_event.as_ref(),
                ),
                (
                    "h2_reference.censoring_event",
                    episode.censoring_event.as_ref(),
                ),
            ] {
                if let Some(event) = event {
                    writer.append(&RunLogEvent::LabelObserved {
                        step,
                        timestamp_ns,
                        name: name.to_string(),
                        value: serde_json::to_value(event)?,
                        metadata: [
                            ("episode_id".to_string(), episode.episode_id.clone()),
                            ("scientific_evidence".to_string(), "false".to_string()),
                        ]
                        .into_iter()
                        .collect(),
                    })?;
                    step += 1;
                    timestamp_ns += 1;
                }
            }
        }
        for outcome in &report.fold_outcomes {
            let H2FoldOutcome::Produced { score } = outcome else {
                continue;
            };
            for prediction in &score.predictions {
                writer.append(&RunLogEvent::LabelObserved {
                    step,
                    timestamp_ns,
                    name: "h2_reference.landmark_prediction".to_string(),
                    value: serde_json::to_value(prediction)?,
                    metadata: [
                        ("episode_id".to_string(), prediction.episode_id.clone()),
                        ("landmark_id".to_string(), prediction.landmark_id.clone()),
                        ("outer_fold".to_string(), prediction.outer_fold.clone()),
                        ("scientific_evidence".to_string(), "false".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                })?;
                step += 1;
                timestamp_ns += 1;
            }
            for (name, value) in [
                (
                    "h2_reference.fold_baseline_ipcw_brier",
                    score.baseline_brier,
                ),
                (
                    "h2_reference.fold_diagnostic_ipcw_brier",
                    score.diagnostic_brier,
                ),
                (
                    "h2_reference.fold_ipcw_brier_improvement",
                    score.brier_improvement,
                ),
            ] {
                writer.append(&RunLogEvent::EvaluationMetric {
                    step,
                    timestamp_ns,
                    name: name.to_string(),
                    value,
                    metadata: [
                        ("outer_fold".to_string(), score.outer_fold.clone()),
                        (
                            "eligible_landmarks".to_string(),
                            score.eligible_landmarks.to_string(),
                        ),
                        ("scientific_evidence".to_string(), "false".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                })?;
                step += 1;
                timestamp_ns += 1;
            }
        }
        if let Some(aggregate) = &report.aggregate_score {
            for (name, value) in [
                ("h2_reference.baseline_ipcw_brier", aggregate.baseline_brier),
                (
                    "h2_reference.diagnostic_ipcw_brier",
                    aggregate.diagnostic_brier,
                ),
                (
                    "h2_reference.ipcw_brier_improvement",
                    aggregate.brier_improvement,
                ),
            ] {
                writer.append(&RunLogEvent::EvaluationMetric {
                    step,
                    timestamp_ns,
                    name: name.to_string(),
                    value,
                    metadata: [
                        (
                            "eligible_landmarks".to_string(),
                            aggregate.eligible_landmarks.to_string(),
                        ),
                        (
                            "precision".to_string(),
                            "not_applicable_deterministic_synthetic".to_string(),
                        ),
                        ("scientific_evidence".to_string(), "false".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                })?;
                step += 1;
                timestamp_ns += 1;
            }
        }
        for model in [H2ModelKind::Baseline, H2ModelKind::Diagnostic] {
            let alarm_result = &report.alarm_results[&model];
            let alarm_metadata = [
                (
                    "model".to_string(),
                    format!("{model:?}").to_ascii_lowercase(),
                ),
                ("control_plane_event".to_string(), "false".to_string()),
                ("scientific_evidence".to_string(), "false".to_string()),
            ]
            .into_iter()
            .collect::<BTreeMap<_, _>>();
            let accounting = match alarm_result {
                H2AlarmResult::Produced { summary } => json!({
                    "status": "produced",
                    "model": summary.model,
                    "threshold": summary.threshold,
                    "alarms_emitted": summary.alarms_emitted,
                    "alarms_executed": summary.alarms_executed,
                    "alarms_matched": summary.alarms_matched,
                    "alarms_unmatched": summary.alarms_unmatched,
                    "alarms_late": summary.alarms_late,
                    "refractory_suppressed": summary.refractory_suppressed,
                    "capacity_rejected": summary.capacity_rejected,
                    "target_events": summary.target_events,
                    "detected_events": summary.detected_events,
                    "undetected_events": summary.undetected_events,
                    "lead_time_record_count": summary.lead_times.len(),
                    "detection_curve": summary.detection_curve,
                    "assumed_payoff_utility_per_evaluable_episode":
                        summary.assumed_payoff_utility_per_evaluable_episode,
                }),
                H2AlarmResult::Abstained { .. } => serde_json::to_value(alarm_result)?,
            };
            writer.append(&RunLogEvent::LabelObserved {
                step,
                timestamp_ns,
                name: "h2_reference.alarm_accounting".to_string(),
                value: accounting,
                metadata: alarm_metadata.clone(),
            })?;
            step += 1;
            timestamp_ns += 1;
            if let H2AlarmResult::Produced { summary } = alarm_result {
                for alarm in &summary.alarms {
                    writer.append(&RunLogEvent::LabelObserved {
                        step,
                        timestamp_ns,
                        name: "h2_reference.alarm_record".to_string(),
                        value: serde_json::to_value(alarm)?,
                        metadata: alarm_metadata.clone(),
                    })?;
                    step += 1;
                    timestamp_ns += 1;
                }
                for lead_time in &summary.lead_times {
                    writer.append(&RunLogEvent::LabelObserved {
                        step,
                        timestamp_ns,
                        name: "h2_reference.lead_time_record".to_string(),
                        value: serde_json::to_value(lead_time)?,
                        metadata: alarm_metadata.clone(),
                    })?;
                    step += 1;
                    timestamp_ns += 1;
                }
                writer.append(&RunLogEvent::EvaluationMetric {
                    step,
                    timestamp_ns,
                    name: format!(
                        "h2_reference.{}_assumed_payoff_utility_per_episode",
                        format!("{model:?}").to_ascii_lowercase()
                    ),
                    value: summary.assumed_payoff_utility_per_evaluable_episode,
                    metadata: [
                        (
                            "utility_kind".to_string(),
                            "declared_warning_payoff_scenario".to_string(),
                        ),
                        (
                            "measured_intervention_benefit".to_string(),
                            "false".to_string(),
                        ),
                        ("scientific_evidence".to_string(), "false".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                })?;
                step += 1;
                timestamp_ns += 1;
            }
        }
    }
    let reason_codes = reason_codes(summary)?;
    writer.append(&RunLogEvent::LabelObserved {
        step,
        timestamp_ns,
        name: "h2_reference.verdict".to_string(),
        value: serde_json::to_value(H2Verdict {
            schema_version: H2_REFERENCE_SCHEMA_VERSION,
            passed: summary.passed,
            synthetic_fixture_only: true,
            establishes_h2_evidence: false,
            prospective_capture: false,
            external_validation: false,
            comparator_frontier_complete: false,
            pid_dependency: "none",
            dataset_sha256: summary.dataset_sha256.as_deref(),
            summary_sha256,
            issue_count: issue_count(summary),
            reason_codes: reason_codes.clone(),
            denominators: summary.report.as_ref().map(|report| &report.denominators),
        })?,
        metadata: metadata.clone(),
    })?;
    timestamp_ns += 1;
    if !summary.passed {
        writer.append(&RunLogEvent::ErrorLogged {
            step: Some(step),
            timestamp_ns,
            message: format!("h2_reference_failed:{}", reason_codes.join(",")),
            recoverable: false,
        })?;
        timestamp_ns += 1;
    }
    writer.append(&RunLogEvent::RunEnded {
        run_id: run_id.to_string(),
        timestamp_ns,
        status: if summary.passed {
            RunStatus::Succeeded
        } else {
            RunStatus::Failed
        },
        message: Some(if summary.passed {
            "synthetic H2 reference arithmetic completed; no H2 evidence was produced".to_string()
        } else {
            "synthetic H2 reference failed closed; no H2 evidence was produced".to_string()
        }),
    })?;
    writer.flush_durable()?;
    drop(writer);

    let events = pid_runlog::read_events(BufReader::new(fs::File::open(&temporary)?))?;
    let replay = pid_runlog::summarize_events(&events)?;
    if replay.validation_errors != 0
        || replay.run_id.as_deref() != Some(run_id)
        || replay.config_hash.as_deref() != Some(config_hash)
        || replay.pid_metric_events != 0
        || replay.actions != 0
        || replay.interventions != 0
    {
        bail!("generated H2 run log failed validation or emitted a prohibited event");
    }
    fs::rename(&temporary, &args.runlog)
        .with_context(|| format!("failed to install {}", args.runlog.display()))?;
    guard.installed = true;
    sync_parent(&args.runlog)
}

fn reason_codes(summary: &H2Summary) -> Result<Vec<String>> {
    let mut values = summary
        .report
        .iter()
        .flat_map(|report| report.issues.iter())
        .map(|problem| {
            serde_json::to_value(problem.code).and_then(|value| {
                value.as_str().map(str::to_string).ok_or_else(|| {
                    serde_json::Error::io(std::io::Error::other("reason code was not a string"))
                })
            })
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    values.extend(
        summary
            .fatal_issues
            .iter()
            .map(|problem| problem.code.clone()),
    );
    values.sort();
    values.dedup();
    Ok(values)
}

fn issue_count(summary: &H2Summary) -> usize {
    summary
        .report
        .as_ref()
        .map_or(0, |report| report.issues.len())
        + summary.fatal_issues.len()
}

fn parse_args() -> Result<Option<Args>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        return Ok(None);
    }
    let mut dataset = None;
    let mut analysis_plan = None;
    let mut event_ontology = None;
    let mut feature_contract = None;
    let mut split_manifest = None;
    let mut summary_json = None;
    let mut runlog = None;
    let mut index = 0;
    while index < arguments.len() {
        let target = match arguments[index].as_str() {
            "--dataset" => &mut dataset,
            "--analysis-plan" => &mut analysis_plan,
            "--event-ontology" => &mut event_ontology,
            "--feature-contract" => &mut feature_contract,
            "--split-manifest" => &mut split_manifest,
            "--summary-json" => &mut summary_json,
            "--runlog" => &mut runlog,
            other => bail!("unknown argument: {other}"),
        };
        let value = arguments
            .get(index + 1)
            .with_context(|| format!("{} requires a path", arguments[index]))?;
        if target.replace(PathBuf::from(value)).is_some() {
            bail!("{} may be supplied only once", arguments[index]);
        }
        index += 2;
    }
    Ok(Some(Args {
        dataset: dataset.context("--dataset is required")?,
        analysis_plan: analysis_plan.context("--analysis-plan is required")?,
        event_ontology: event_ontology.context("--event-ontology is required")?,
        feature_contract: feature_contract.context("--feature-contract is required")?,
        split_manifest: split_manifest.context("--split-manifest is required")?,
        summary_json: summary_json.context("--summary-json is required")?,
        runlog: runlog.context("--runlog is required")?,
    }))
}

fn read_exact_snapshot(path: &Path, maximum: u64) -> Result<ExactSnapshot> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open input {}", path.display()))?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        bail!("input is not a regular file: {}", path.display());
    }
    if metadata.len() > maximum {
        return Ok(ExactSnapshot {
            bytes: None,
            sha256: None,
            byte_len: metadata.len(),
        });
    }
    let mut bytes = Vec::new();
    let mut hasher = Sha256::new();
    let mut byte_len = 0_u64;
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        byte_len = byte_len
            .checked_add(count as u64)
            .context("input byte count overflowed u64")?;
        if byte_len > maximum {
            return Ok(ExactSnapshot {
                bytes: None,
                sha256: None,
                byte_len,
            });
        }
        hasher.update(&chunk[..count]);
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok(ExactSnapshot {
        bytes: Some(bytes),
        sha256: Some(format!("{:x}", hasher.finalize())),
        byte_len,
    })
}

fn ensure_distinct_paths(args: &Args) -> Result<()> {
    let inputs = [
        &args.dataset,
        &args.analysis_plan,
        &args.event_ontology,
        &args.feature_contract,
        &args.split_manifest,
    ];
    let outputs = [&args.summary_json, &args.runlog];
    if comparable_path(&args.summary_json)? == comparable_path(&args.runlog)? {
        bail!("summary and run-log outputs must be distinct");
    }
    for input in inputs {
        for output in outputs {
            if comparable_path(input)? == comparable_path(output)? {
                bail!("output path aliases input path {}", input.display());
            }
        }
    }
    Ok(())
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
    let parent = absolute
        .parent()
        .context("output path must have an existing parent")?
        .canonicalize()
        .with_context(|| format!("failed to resolve parent of {}", absolute.display()))?;
    let name = absolute
        .file_name()
        .context("output path must name a file")?;
    Ok(parent.join(name))
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(())
}

fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = temporary_path(path, "atomic");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("failed to create {}", temporary.display()))?;
    let mut guard = TemporaryPathGuard::new(temporary.clone());
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, path)
        .with_context(|| format!("failed to install {}", path.display()))?;
    guard.installed = true;
    sync_parent(path)
}

fn temporary_path(path: &Path, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .map_or_else(|| "output".into(), |name| name.to_string_lossy());
    path.with_file_name(format!(
        ".{name}.{}.{}.h2-reference-{suffix}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn sync_parent(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn push_issue(
    issues: &mut Vec<CliIssue>,
    code: impl Into<String>,
    field: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(CliIssue {
        code: code.into(),
        field: field.into(),
        message: message.into(),
    });
}

fn print_usage() {
    println!(
        "Usage: pid-h2-reference --dataset PATH --analysis-plan PATH \\\n+         --event-ontology PATH --feature-contract PATH --split-manifest PATH \\\n+         --summary-json PATH --runlog PATH\n\
         Runs a deterministic synthetic finite-benchmark reference only; it produces no H2 evidence."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_paths_must_not_alias_an_input_or_each_other() {
        let root = std::env::temp_dir().join(format!(
            "prisoma-h2-reference-paths-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("sub")).expect("create test directory");
        let dataset = root.join("dataset.json");
        let summary = root.join("summary.json");
        let base = Args {
            dataset: dataset.clone(),
            analysis_plan: root.join("analysis-plan.json"),
            event_ontology: root.join("event-ontology.json"),
            feature_contract: root.join("feature-contract.json"),
            split_manifest: root.join("split-manifest.json"),
            summary_json: summary.clone(),
            runlog: summary,
        };
        assert!(ensure_distinct_paths(&base).is_err());
        let aliases_input = Args {
            runlog: dataset,
            summary_json: root.join("other-summary.json"),
            ..base
        };
        assert!(ensure_distinct_paths(&aliases_input).is_err());
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn compact_config_never_embeds_the_dataset() {
        let snapshot = || ExactSnapshot {
            bytes: None,
            sha256: Some("a".repeat(64)),
            byte_len: MAX_DATASET_BYTES,
        };
        let snapshots = Snapshots {
            dataset: snapshot(),
            analysis_plan: snapshot(),
            event_ontology: snapshot(),
            feature_contract: snapshot(),
            split_manifest: snapshot(),
        };
        let config = compact_config(None, &snapshots);
        let encoded = serde_json::to_vec(&config).expect("encode compact config");
        assert!(encoded.len() < 64 * 1024);
        assert!(config.get("dataset").is_none());
    }
}
