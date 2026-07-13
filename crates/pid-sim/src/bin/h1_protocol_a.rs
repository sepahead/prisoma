use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, Context, Result};
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use pid_sim::h1_preflight::{
    H1PreflightDeclaration, H1PreflightInput, H1PreflightReport, H1PreflightValidationScope,
    H1PrimaryProtocol, H1TargetPopulation, H1_PREFLIGHT_SCHEMA_VERSION,
};
use pid_sim::h1_protocol_a::{
    run_h1_protocol_a, H1ProtocolACaseOutcome, H1ProtocolAInput, H1ProtocolAReport,
    H1ProtocolATreatmentOrder, H1ProtocolATreatmentPair, H1_PROTOCOL_A_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const COMPONENT: &str = "pid-h1-protocol-a";
const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PREFLIGHT_BYTES: u64 = 64 * 1024 * 1024;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct Args {
    input: PathBuf,
    preflight_input: PathBuf,
    preflight_summary: PathBuf,
    preflight_runlog: PathBuf,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreflightSummary {
    schema_version: u32,
    run_id: String,
    input_uri: String,
    input_sha256: String,
    config_hash: String,
    parsed: bool,
    passed: bool,
    establishes_h1_evidence: bool,
    evidence_bundle_hash: String,
    verified_artifacts: Vec<Value>,
    report: Option<H1PreflightReport>,
    fatal_issues: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreflightVerdict {
    schema_version: u32,
    primary_protocol: String,
    passed: bool,
    establishes_h1_evidence: bool,
    denominators: Option<Value>,
    issue_count: usize,
    reason_codes: Vec<String>,
    input_sha256: String,
    summary_sha256: String,
    evidence_bundle_hash: String,
    verified_artifact_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ProtocolASummary {
    schema_version: u32,
    run_id: String,
    input_uri: String,
    input_sha256: Option<String>,
    config_hash: String,
    parsed: bool,
    passed: bool,
    synthetic_fixture_only: bool,
    establishes_h1_evidence: bool,
    preflight_input_sha256: Option<String>,
    preflight_summary_sha256: Option<String>,
    preflight_runlog_sha256: Option<String>,
    report: Option<H1ProtocolAReport>,
    fatal_issues: Vec<CliIssue>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ProtocolAVerdict<'a> {
    schema_version: u32,
    passed: bool,
    synthetic_fixture_only: bool,
    establishes_h1_evidence: bool,
    input_sha256: Option<&'a str>,
    summary_sha256: &'a str,
    issue_count: usize,
    reason_codes: Vec<String>,
    denominators: Option<&'a pid_sim::h1_protocol_a::H1ProtocolADenominators>,
}

fn main() -> Result<()> {
    let Some(args) = parse_args()? else {
        print_usage();
        return Ok(());
    };
    ensure_parent(&args.summary_json)?;
    ensure_parent(&args.runlog)?;
    ensure_distinct_paths(&args)?;

    let input_snapshot = read_exact_snapshot(&args.input, MAX_INPUT_BYTES)?;
    let input_sha256 = input_snapshot.sha256.clone();
    let (input, parse_issue) = match input_snapshot.bytes.as_deref() {
        Some(bytes) => match serde_json::from_slice::<H1ProtocolAInput>(bytes) {
            Ok(input) => (Some(input), None),
            Err(error) => (
                None,
                Some(CliIssue {
                    code: "contract_parse_failed".to_string(),
                    field: "input".to_string(),
                    message: error.to_string(),
                }),
            ),
        },
        None => (
            None,
            Some(CliIssue {
                code: "input_resource_limit_exceeded".to_string(),
                field: "input".to_string(),
                message: format!(
                    "input is {} bytes; the limit is {MAX_INPUT_BYTES}",
                    input_snapshot.byte_len
                ),
            }),
        ),
    };

    let run_id = input_sha256.as_ref().map_or_else(
        || "h1-protocol-a-resource-limited".to_string(),
        |sha256| format!("h1-protocol-a-{}", &sha256[..16]),
    );
    let parsed = input.is_some();
    let mut fatal_issues = parse_issue.into_iter().collect::<Vec<_>>();
    let mut preflight_hashes = (None, None, None);
    let mut treatment_plan = None;
    let mut report = None;
    let config = if let Some(input) = input {
        let verification = verify_preflight_binding(&input, &args)?;
        let preflight_eligible = verification.issues.is_empty();
        fatal_issues.extend(verification.issues);
        preflight_hashes = (
            verification.input_sha256.clone(),
            verification.summary_sha256.clone(),
            verification.runlog_sha256.clone(),
        );
        treatment_plan = Some(input.plan.treatment.clone());
        let protocol_report = preflight_eligible.then(|| run_h1_protocol_a(&input));
        let config = compact_protocol_a_config(
            &input,
            input_sha256.as_deref(),
            verification.input_sha256.as_deref(),
            verification.summary_sha256.as_deref(),
            verification.runlog_sha256.as_deref(),
        )?;
        report = protocol_report;
        config
    } else {
        json!({
            "component": COMPONENT,
            "protocol_a_schema_version": H1_PROTOCOL_A_SCHEMA_VERSION,
            "scope": "unparsed_input_no_protocol_execution",
            "input_sha256": input_sha256,
        })
    };
    let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
    let passed =
        report.as_ref().is_some_and(H1ProtocolAReport::is_valid) && fatal_issues.is_empty();
    let (preflight_input_sha256, preflight_summary_sha256, preflight_runlog_sha256) =
        preflight_hashes;
    let summary = ProtocolASummary {
        schema_version: H1_PROTOCOL_A_SCHEMA_VERSION,
        run_id: run_id.clone(),
        input_uri: args.input.display().to_string(),
        input_sha256: input_sha256.clone(),
        config_hash: config_hash.clone(),
        parsed,
        passed,
        synthetic_fixture_only: true,
        establishes_h1_evidence: false,
        preflight_input_sha256,
        preflight_summary_sha256,
        preflight_runlog_sha256,
        report,
        fatal_issues,
    };
    let summary_bytes = serde_json::to_vec_pretty(&summary)?;
    let summary_sha256 = sha256_bytes(&summary_bytes);
    write_atomic_bytes(&args.summary_json, &summary_bytes)?;
    write_runlog(
        &args,
        &run_id,
        input_sha256.as_deref(),
        &summary_sha256,
        &config_hash,
        config,
        &summary,
        treatment_plan.as_ref(),
    )?;

    println!("run_id={run_id}");
    println!("parsed={}", summary.parsed);
    println!("passed={}", summary.passed);
    println!("synthetic_fixture_only=true");
    println!("establishes_h1_evidence=false");
    println!("wrote_summary={}", args.summary_json.display());
    println!("wrote_runlog={}", args.runlog.display());
    if !summary.passed {
        bail!("H1 Protocol-A software reference failed closed; inspect the summary and run log");
    }
    Ok(())
}

fn compact_protocol_a_config(
    input: &H1ProtocolAInput,
    input_sha256: Option<&str>,
    preflight_input_sha256: Option<&str>,
    preflight_summary_sha256: Option<&str>,
    preflight_runlog_sha256: Option<&str>,
) -> Result<Value> {
    let audit_count = input.cases.iter().try_fold(0_usize, |total, case| {
        total
            .checked_add(case.audits.len())
            .context("Protocol-A audit count overflowed usize")
    })?;
    Ok(json!({
        "component": COMPONENT,
        "protocol_a_schema_version": H1_PROTOCOL_A_SCHEMA_VERSION,
        "scope": "deterministic_finite_benchmark_software_reference_only",
        "input_sha256": input_sha256,
        "preflight_input_sha256": preflight_input_sha256,
        "preflight_summary_sha256": preflight_summary_sha256,
        "preflight_runlog_sha256": preflight_runlog_sha256,
        "input_receipt": {
            "target_population_id": input.plan.target_population_id,
            "policy_id": input.plan.policy_id,
            "policy_spec_sha256": input.plan.policy_spec_sha256,
            "instrumentation_id": input.plan.instrumentation_id,
            "instrumentation_spec_sha256": input.plan.instrumentation_spec_sha256,
            "plan_sha256": pid_runlog::canonical_json_hash_v2(&input.plan)?,
            "policy_sha256": pid_runlog::canonical_json_hash_v2(&input.policy)?,
            "case_count": input.cases.len(),
            "audit_count": audit_count,
        },
        "configuration_storage": "compact_content_addressed_receipt",
    }))
}

struct PreflightVerification {
    input_sha256: Option<String>,
    summary_sha256: Option<String>,
    runlog_sha256: Option<String>,
    issues: Vec<CliIssue>,
}

fn verify_preflight_binding(
    input: &H1ProtocolAInput,
    args: &Args,
) -> Result<PreflightVerification> {
    let preflight_input = read_exact_snapshot(&args.preflight_input, MAX_PREFLIGHT_BYTES)?;
    let preflight_summary = read_exact_snapshot(&args.preflight_summary, MAX_PREFLIGHT_BYTES)?;
    let preflight_runlog = read_exact_snapshot(&args.preflight_runlog, MAX_PREFLIGHT_BYTES)?;
    let mut issues = Vec::new();
    for (field, expected_uri, expected_sha256, actual_path, actual_sha256, actual_bytes) in [
        (
            "preflight.input",
            input.preflight.input.artifact_uri.as_str(),
            input.preflight.input.sha256.as_str(),
            &args.preflight_input,
            preflight_input.sha256.as_deref(),
            preflight_input.byte_len,
        ),
        (
            "preflight.summary",
            input.preflight.summary.artifact_uri.as_str(),
            input.preflight.summary.sha256.as_str(),
            &args.preflight_summary,
            preflight_summary.sha256.as_deref(),
            preflight_summary.byte_len,
        ),
        (
            "preflight.runlog",
            input.preflight.runlog.artifact_uri.as_str(),
            input.preflight.runlog.sha256.as_str(),
            &args.preflight_runlog,
            preflight_runlog.sha256.as_deref(),
            preflight_runlog.byte_len,
        ),
    ] {
        if expected_uri != actual_path.to_string_lossy() {
            push_cli_issue(
                &mut issues,
                "preflight_artifact_uri_mismatch",
                field,
                format!(
                    "binding names {expected_uri:?}, CLI supplied {:?}",
                    actual_path.display()
                ),
            );
        }
        match actual_sha256 {
            Some(actual_sha256) if expected_sha256 == actual_sha256 => {}
            Some(actual_sha256) => push_cli_issue(
                &mut issues,
                "preflight_artifact_hash_mismatch",
                field,
                format!("expected {expected_sha256}, captured {actual_sha256}"),
            ),
            None => push_cli_issue(
                &mut issues,
                "preflight_resource_limit_exceeded",
                field,
                format!("artifact is {actual_bytes} bytes; the limit is {MAX_PREFLIGHT_BYTES}"),
            ),
        }
    }
    let preflight_document = preflight_input.bytes.as_deref().and_then(|bytes| {
        match serde_json::from_slice::<H1PreflightInput>(bytes) {
            Ok(document) => Some(document),
            Err(error) => {
                push_cli_issue(
                    &mut issues,
                    "preflight_input_parse_failed",
                    "preflight.input",
                    error.to_string(),
                );
                None
            }
        }
    });
    let summary =
        preflight_summary.bytes.as_deref().and_then(|bytes| {
            match serde_json::from_slice::<PreflightSummary>(bytes) {
                Ok(summary) => Some(summary),
                Err(error) => {
                    push_cli_issue(
                        &mut issues,
                        "preflight_summary_parse_failed",
                        "preflight.summary",
                        error.to_string(),
                    );
                    None
                }
            }
        });
    let runlog_events = preflight_runlog.bytes.as_deref().and_then(|bytes| {
        match pid_runlog::read_events(BufReader::new(Cursor::new(bytes))) {
            Ok(events) => Some(events),
            Err(error) => {
                push_cli_issue(
                    &mut issues,
                    "preflight_runlog_parse_failed",
                    "preflight.runlog",
                    error.to_string(),
                );
                None
            }
        }
    });
    if let (Some(preflight_document), Some(summary), Some(events), Some(summary_sha256)) = (
        &preflight_document,
        &summary,
        &runlog_events,
        preflight_summary.sha256.as_deref(),
    ) {
        validate_preflight_semantics(
            input,
            preflight_document,
            summary,
            events,
            summary_sha256,
            &mut issues,
        );
    }
    Ok(PreflightVerification {
        input_sha256: preflight_input.sha256,
        summary_sha256: preflight_summary.sha256,
        runlog_sha256: preflight_runlog.sha256,
        issues,
    })
}

fn validate_preflight_semantics(
    input: &H1ProtocolAInput,
    preflight_input: &H1PreflightInput,
    summary: &PreflightSummary,
    events: &[RunLogEvent],
    summary_sha256: &str,
    issues: &mut Vec<CliIssue>,
) {
    let summary_report_valid = summary.report.as_ref().is_some_and(|report| {
        report.primary_protocol == H1PrimaryProtocol::ProtocolA
            && report.passed
            && report.issues.is_empty()
    });
    let evidence_bundle_hash = pid_runlog::canonical_json_hash_v2(&summary.verified_artifacts);
    if summary.schema_version != H1_PREFLIGHT_SCHEMA_VERSION
        || !summary.parsed
        || !summary.passed
        || summary.establishes_h1_evidence
        || !summary_report_valid
        || !summary.fatal_issues.is_empty()
        || summary.run_id != input.preflight.run_id
        || summary.input_uri != input.preflight.input.artifact_uri
        || summary.input_sha256 != input.preflight.input.sha256
        || summary.evidence_bundle_hash != input.preflight.evidence_bundle_hash
        || evidence_bundle_hash.as_ref().ok().map(String::as_str)
            != Some(summary.evidence_bundle_hash.as_str())
    {
        push_cli_issue(
            issues,
            "preflight_summary_not_eligible",
            "preflight.summary",
            "summary does not prove a clean Protocol-A common-preflight pass",
        );
    }
    let runlog_summary = match pid_runlog::summarize_events(events) {
        Ok(value) => value,
        Err(error) => {
            push_cli_issue(
                issues,
                "preflight_runlog_invalid",
                "preflight.runlog",
                error.to_string(),
            );
            return;
        }
    };
    if runlog_summary.validation_errors > 0
        || runlog_summary.run_id.as_deref() != Some(summary.run_id.as_str())
        || runlog_summary.config_hash.as_deref() != Some(summary.config_hash.as_str())
        || runlog_summary.status != Some(RunStatus::Succeeded)
        || runlog_summary.pid_metric_events != 0
    {
        push_cli_issue(
            issues,
            "preflight_runlog_invalid",
            "preflight.runlog",
            "run log failed validation or names a different run",
        );
    }
    let verdict_values = events
        .iter()
        .filter_map(|event| {
            if let RunLogEvent::LabelObserved { name, value, .. } = event {
                if name == "h1_preflight.verdict" {
                    return Some(value);
                }
            }
            None
        })
        .collect::<Vec<_>>();
    let verdict = (verdict_values.len() == 1)
        .then(|| serde_json::from_value::<PreflightVerdict>(verdict_values[0].clone()).ok())
        .flatten();
    if verdict.as_ref().is_none_or(|verdict| {
        verdict.schema_version != H1_PREFLIGHT_SCHEMA_VERSION
            || verdict.primary_protocol != "protocol_a"
            || !verdict.passed
            || verdict.establishes_h1_evidence
            || verdict.denominators.is_none()
            || verdict.issue_count != 0
            || !verdict.reason_codes.is_empty()
            || verdict.input_sha256 != summary.input_sha256
            || verdict.summary_sha256 != summary_sha256
            || verdict.evidence_bundle_hash != summary.evidence_bundle_hash
            || verdict.verified_artifact_count != summary.verified_artifacts.len()
    }) {
        push_cli_issue(
            issues,
            "preflight_verdict_mismatch",
            "preflight.runlog",
            "run-log verdict does not exactly bind the supplied passing summary",
        );
    }
    let matching_run_started = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RunLogEvent::RunStarted {
                    run_id,
                    config_hash,
                    ..
                } if run_id == &summary.run_id && config_hash == &summary.config_hash
            )
        })
        .count();
    if matching_run_started != 1 {
        push_cli_issue(
            issues,
            "preflight_run_started_mismatch",
            "preflight.runlog",
            "run_started does not bind the supplied summary run and config hash",
        );
    }
    let configs = events
        .iter()
        .filter_map(|event| {
            if let RunLogEvent::ConfigLogged {
                config_hash,
                config,
                ..
            } = event
            {
                return Some((config_hash, config));
            }
            None
        })
        .collect::<Vec<_>>();
    let config_matches = configs.len() == 1
        && configs[0].0 == &summary.config_hash
        && pid_runlog::canonical_json_hash_v2(configs[0].1)
            .as_ref()
            .ok()
            .map(String::as_str)
            == Some(summary.config_hash.as_str());
    let declaration = (configs.len() == 1)
        .then(|| configs[0].1.get("declaration"))
        .flatten()
        .cloned()
        .and_then(|value| serde_json::from_value::<H1PreflightDeclaration>(value).ok());
    if !config_matches
        || declaration.as_ref() != Some(&preflight_input.declaration)
        || !declaration
            .as_ref()
            .is_some_and(|declaration| declaration_matches_plan(declaration, input))
    {
        push_cli_issue(
            issues,
            "preflight_declaration_mismatch",
            "preflight.runlog",
            "Protocol-A plan does not match the exact preflight policy, scope, boundaries, treatment, clock, and output contract",
        );
    }
}

fn declaration_matches_plan(
    declaration: &H1PreflightDeclaration,
    input: &H1ProtocolAInput,
) -> bool {
    declaration.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
        && declaration.primary_protocol == H1PrimaryProtocol::ProtocolA
        && declaration.validation_scope
            == H1PreflightValidationScope::RepresentativeMechanismFixture
        && declaration.target_population == H1TargetPopulation::FiniteBenchmark
        && declaration.target_population_id == input.plan.target_population_id
        && declaration.policy_id == input.plan.policy_id
        && declaration.policy_spec_sha256 == input.plan.policy_spec_sha256
        && declaration.instrumentation_id == input.plan.instrumentation_id
        && declaration.instrumentation_spec_sha256 == input.plan.instrumentation_spec_sha256
        && declaration.execution_context == input.plan.execution_context
        && declaration.clock.domain_id == input.plan.clock_domain_id
        && declaration.protocol_clone_boundary == input.plan.clone_boundary
        && declaration.application_boundary == input.plan.application_boundary
        && declaration.reset_boundary == input.plan.reset_boundary
        && declaration.treatment_site == input.plan.treatment.treatment_site
        && declaration.control_version == input.plan.treatment.control_version
        && declaration.treatment_version == input.plan.treatment.treated_version
        && declaration.treatment_dose == input.plan.treatment.dose
        && declaration.treatment_dose_unit == input.plan.treatment.dose_unit
        && declaration.output_metric_contract == input.plan.output_metric_contract
}

#[expect(
    clippy::too_many_arguments,
    reason = "the durable run-log boundary binds each independently hashed artifact explicitly"
)]
fn write_runlog(
    args: &Args,
    run_id: &str,
    input_sha256: Option<&str>,
    summary_sha256: &str,
    config_hash: &str,
    config: Value,
    summary: &ProtocolASummary,
    treatment: Option<&H1ProtocolATreatmentPair>,
) -> Result<()> {
    let metadata = [
        ("component".to_string(), COMPONENT.to_string()),
        ("claim".to_string(), "H1".to_string()),
        ("primary_protocol".to_string(), "protocol_a".to_string()),
        (
            "scope".to_string(),
            "deterministic_synthetic_fixture_not_h1_evidence".to_string(),
        ),
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
            "h1_protocol_a_input",
            "h1_protocol_a_input_json",
            &args.input,
            input_sha256,
        ),
        (
            "h1_common_preflight_input",
            "h1_preflight_input_json",
            &args.preflight_input,
            summary.preflight_input_sha256.as_deref(),
        ),
        (
            "h1_common_preflight_summary",
            "h1_preflight_summary_json",
            &args.preflight_summary,
            summary.preflight_summary_sha256.as_deref(),
        ),
        (
            "h1_common_preflight_runlog",
            "h1_preflight_runlog_jsonl",
            &args.preflight_runlog,
            summary.preflight_runlog_sha256.as_deref(),
        ),
        (
            "h1_protocol_a_summary",
            "h1_protocol_a_summary_json",
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
    let actor = Actor {
        actor_type: ActorType::System,
        actor_id: COMPONENT.to_string(),
        session_id: Some(run_id.to_string()),
    };
    let mut step = 0_u64;
    if summary.passed {
        let report = summary
            .report
            .as_ref()
            .context("a passing Protocol-A summary must include a report")?;
        for outcome in &report.case_outcomes {
            let H1ProtocolACaseOutcome::Produced { result } = outcome else {
                continue;
            };
            for receipt in &result.receipts {
                let control = (
                    "control",
                    treatment.map_or("unparsed-control", |value| value.control_version.as_str()),
                    0.0,
                    &receipt.control_output_sha256,
                    &receipt.control_treatment_receipt_sha256,
                );
                let treated = (
                    "treated",
                    treatment.map_or("unparsed-treated", |value| value.treated_version.as_str()),
                    treatment.map_or(0.0, |value| value.dose),
                    &receipt.treated_output_sha256,
                    &receipt.treated_treatment_receipt_sha256,
                );
                let ordered = match receipt.treatment_order {
                    H1ProtocolATreatmentOrder::ControlFirst => [control, treated],
                    H1ProtocolATreatmentOrder::TreatedFirst => [treated, control],
                };
                for (condition, version, dose, output_sha256, treatment_receipt_sha256) in ordered {
                    let payload = json!({
                        "case_id": result.case_id,
                        "audit_id": receipt.audit_id,
                        "treatment_order": receipt.treatment_order,
                        "condition": condition,
                        "treatment_version": version,
                        "treatment_site": treatment.map(|value| value.treatment_site.as_str()),
                        "state_axis": treatment.map(|value| value.state_axis),
                        "dose": dose,
                        "dose_unit": treatment.map(|value| value.dose_unit.as_str()),
                        "clone_state_sha256": receipt.clone_state_sha256,
                        "policy_input_sha256": receipt.policy_input_sha256,
                        "output_sha256": output_sha256,
                        "treatment_receipt_sha256": treatment_receipt_sha256,
                        "observed_rng_draws": 0,
                    });
                    let payload_hash = pid_runlog::canonical_json_hash_v2(&payload)?;
                    writer.append(&RunLogEvent::InterventionApplied {
                        step,
                        timestamp_ns,
                        actor: actor.clone(),
                        intervention_type: "h1_protocol_a_reference_treatment_version".to_string(),
                        payload_hash,
                        payload,
                    })?;
                    timestamp_ns += 1;
                    step += 1;
                }
            }
            for (name, value) in [
                ("h1_protocol_a.response", Some(result.response)),
                (
                    "h1_protocol_a.baseline_prediction",
                    result.baseline_prediction,
                ),
                (
                    "h1_protocol_a.diagnostic_prediction",
                    result.diagnostic_prediction,
                ),
            ] {
                if let Some(value) = value {
                    writer.append(&RunLogEvent::EvaluationMetric {
                        step,
                        timestamp_ns,
                        name: name.to_string(),
                        value,
                        metadata: [
                            ("case_id".to_string(), result.case_id.clone()),
                            ("outer_fold".to_string(), result.outer_fold.clone()),
                            ("scientific_evidence".to_string(), "false".to_string()),
                        ]
                        .into_iter()
                        .collect(),
                    })?;
                    timestamp_ns += 1;
                    step += 1;
                }
            }
        }
        if let Some(aggregate) = &report.aggregate_score {
            for (name, value) in [
                ("h1_protocol_a.baseline_mse", aggregate.baseline_mse),
                ("h1_protocol_a.diagnostic_mse", aggregate.diagnostic_mse),
                ("h1_protocol_a.mse_improvement", aggregate.mse_improvement),
            ] {
                writer.append(&RunLogEvent::EvaluationMetric {
                    step,
                    timestamp_ns,
                    name: name.to_string(),
                    value,
                    metadata: metadata.clone(),
                })?;
                timestamp_ns += 1;
                step += 1;
            }
        }
    }
    let reason_codes = reason_codes(summary)?;
    writer.append(&RunLogEvent::LabelObserved {
        step,
        timestamp_ns,
        name: "h1_protocol_a.verdict".to_string(),
        value: serde_json::to_value(ProtocolAVerdict {
            schema_version: H1_PROTOCOL_A_SCHEMA_VERSION,
            passed: summary.passed,
            synthetic_fixture_only: true,
            establishes_h1_evidence: false,
            input_sha256,
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
            message: format!("h1_protocol_a_failed:{}", reason_codes.join(",")),
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
            "deterministic Protocol-A software reference passed; no H1 evidence was produced"
                .to_string()
        } else {
            "deterministic Protocol-A software reference failed closed; no H1 evidence was produced"
                .to_string()
        }),
    })?;
    writer.flush_durable()?;
    drop(writer);
    fs::rename(&temporary, &args.runlog)
        .with_context(|| format!("failed to install {}", args.runlog.display()))?;
    guard.installed = true;
    sync_parent(&args.runlog)?;
    Ok(())
}

fn reason_codes(summary: &ProtocolASummary) -> Result<Vec<String>> {
    let mut values = summary
        .report
        .iter()
        .flat_map(|report| report.issues.iter())
        .map(|issue| {
            serde_json::to_value(issue.code).and_then(|value| {
                value.as_str().map(str::to_string).ok_or_else(|| {
                    serde_json::Error::io(std::io::Error::other("reason code was not a string"))
                })
            })
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    values.extend(summary.fatal_issues.iter().map(|issue| issue.code.clone()));
    values.sort();
    values.dedup();
    Ok(values)
}

fn issue_count(summary: &ProtocolASummary) -> usize {
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
    let mut input = None;
    let mut preflight_input = None;
    let mut preflight_summary = None;
    let mut preflight_runlog = None;
    let mut summary_json = None;
    let mut runlog = None;
    let mut index = 0;
    while index < arguments.len() {
        let target = match arguments[index].as_str() {
            "--input" => &mut input,
            "--preflight-input" => &mut preflight_input,
            "--preflight-summary" => &mut preflight_summary,
            "--preflight-runlog" => &mut preflight_runlog,
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
        input: input.context("--input is required")?,
        preflight_input: preflight_input.context("--preflight-input is required")?,
        preflight_summary: preflight_summary.context("--preflight-summary is required")?,
        preflight_runlog: preflight_runlog.context("--preflight-runlog is required")?,
        summary_json: summary_json.context("--summary-json is required")?,
        runlog: runlog.context("--runlog is required")?,
    }))
}

fn read_exact_snapshot(path: &Path, maximum: u64) -> Result<ExactSnapshot> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
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
        &args.input,
        &args.preflight_input,
        &args.preflight_summary,
        &args.preflight_runlog,
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
        ".{name}.{}.{}.h1-protocol-a-{suffix}.tmp",
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

fn push_cli_issue(
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
        "Usage: pid-h1-protocol-a --input PATH --preflight-input PATH \\\n         --preflight-summary PATH --preflight-runlog PATH --summary-json PATH --runlog PATH\n\
         Runs a deterministic finite-benchmark software reference only; it produces no H1 evidence."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_is_a_bounded_content_addressed_receipt() {
        let input = serde_json::from_slice::<H1ProtocolAInput>(include_bytes!(
            "../../fixtures/h1_protocol_a_valid.json"
        ))
        .expect("parse checked Protocol-A fixture");
        let hash = "a".repeat(64);
        let config =
            compact_protocol_a_config(&input, Some(&hash), Some(&hash), Some(&hash), Some(&hash))
                .expect("build compact config");
        let encoded = serde_json::to_vec(&config).expect("encode compact config");

        assert!(encoded.len() < 64 * 1024);
        assert_eq!(
            config.get("configuration_storage").and_then(Value::as_str),
            Some("compact_content_addressed_receipt")
        );
        assert!(config.get("input").is_none());
        assert_eq!(
            config
                .pointer("/input_receipt/case_count")
                .and_then(Value::as_u64),
            Some(input.cases.len() as u64)
        );
    }

    #[test]
    fn output_paths_must_not_alias_each_other_or_an_input() {
        let root = std::env::temp_dir().join(format!(
            "prisoma-h1-protocol-a-paths-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("sub")).expect("create test directory");
        let input = root.join("input.json");
        let summary = root.join("summary.json");
        let base = Args {
            input: input.clone(),
            preflight_input: root.join("preflight-input.json"),
            preflight_summary: root.join("preflight-summary.json"),
            preflight_runlog: root.join("preflight-runlog.jsonl"),
            summary_json: summary.clone(),
            runlog: summary,
        };
        assert!(ensure_distinct_paths(&base).is_err());

        let aliases_input = Args {
            runlog: input,
            summary_json: root.join("other-summary.json"),
            ..base
        };
        assert!(ensure_distinct_paths(&aliases_input).is_err());

        let lexical_alias = Args {
            input: root.join("input.json"),
            preflight_input: root.join("preflight-input.json"),
            preflight_summary: root.join("preflight-summary.json"),
            preflight_runlog: root.join("preflight-runlog.jsonl"),
            summary_json: root.join("same.json"),
            runlog: root.join("sub/../same.json"),
        };
        assert!(ensure_distinct_paths(&lexical_alias).is_err());
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[cfg(unix)]
    #[test]
    fn output_paths_detect_symlinked_parent_aliases() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "prisoma-h1-protocol-a-symlink-paths-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let real = root.join("real");
        fs::create_dir_all(&real).expect("create real directory");
        let alias = root.join("alias");
        symlink(&real, &alias).expect("create directory symlink");
        let args = Args {
            input: real.join("input.json"),
            preflight_input: real.join("preflight-input.json"),
            preflight_summary: real.join("preflight-summary.json"),
            preflight_runlog: real.join("preflight-runlog.jsonl"),
            summary_json: real.join("same.json"),
            runlog: alias.join("same.json"),
        };
        assert!(ensure_distinct_paths(&args).is_err());
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[cfg(unix)]
    #[test]
    fn output_paths_resolve_symlink_before_parent_components() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "prisoma-h1-protocol-a-symlink-parent-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let source = root.join("a");
        let target = root.join("b/c");
        fs::create_dir_all(&source).expect("create source directory");
        fs::create_dir_all(&target).expect("create target directory");
        symlink(&target, source.join("link")).expect("create directory symlink");
        let args = Args {
            input: source.join("input.json"),
            preflight_input: source.join("preflight-input.json"),
            preflight_summary: source.join("preflight-summary.json"),
            preflight_runlog: source.join("preflight-runlog.jsonl"),
            summary_json: root.join("b/same.json"),
            runlog: source.join("link/../same.json"),
        };
        assert!(ensure_distinct_paths(&args).is_err());
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn exact_snapshot_hashes_retained_bytes() {
        let path = std::env::temp_dir().join(format!(
            "prisoma-h1-protocol-a-snapshot-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let bytes = b"protocol-a";
        fs::write(&path, bytes).expect("write test input");
        let snapshot = read_exact_snapshot(&path, 1024).expect("read test input");
        fs::remove_file(path).expect("remove test input");
        assert_eq!(snapshot.bytes.as_deref(), Some(bytes.as_slice()));
        assert_eq!(
            snapshot.sha256.as_deref(),
            Some(sha256_bytes(bytes).as_str())
        );
    }

    #[test]
    fn oversized_snapshot_is_rejected_without_a_partial_hash() {
        let path = std::env::temp_dir().join(format!(
            "prisoma-h1-protocol-a-oversized-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let file = fs::File::create(&path).expect("create test input");
        file.set_len(1025).expect("size test input");
        drop(file);
        let snapshot = read_exact_snapshot(&path, 1024).expect("reject oversized input");
        fs::remove_file(path).expect("remove test input");
        assert!(snapshot.bytes.is_none());
        assert!(snapshot.sha256.is_none());
        assert_eq!(snapshot.byte_len, 1025);
    }

    #[test]
    fn oversized_preflight_artifact_is_a_fatal_binding_issue() {
        let root = std::env::temp_dir().join(format!(
            "prisoma-h1-protocol-a-preflight-limit-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("create test directory");
        let preflight_input = root.join("preflight-input.json");
        let preflight_summary = root.join("preflight-summary.json");
        let preflight_runlog = root.join("preflight-runlog.jsonl");
        fs::write(&preflight_input, b"{}").expect("write preflight input");
        let file = fs::File::create(&preflight_summary).expect("create preflight summary");
        file.set_len(MAX_PREFLIGHT_BYTES + 1)
            .expect("size preflight summary");
        drop(file);
        fs::write(&preflight_runlog, b"{}\n").expect("write preflight runlog");
        let input = serde_json::from_str::<H1ProtocolAInput>(include_str!(
            "../../fixtures/h1_protocol_a_valid.json"
        ))
        .expect("parse checked fixture");
        let args = Args {
            input: root.join("protocol-a.json"),
            preflight_input,
            preflight_summary,
            preflight_runlog,
            summary_json: root.join("summary.json"),
            runlog: root.join("runlog.jsonl"),
        };
        let verification = verify_preflight_binding(&input, &args).expect("verify binding");
        fs::remove_dir_all(root).expect("remove test directory");
        assert!(verification.issues.iter().any(|issue| {
            issue.code == "preflight_resource_limit_exceeded" && issue.field == "preflight.summary"
        }));
    }

    #[test]
    fn preflight_execution_context_must_match_the_protocol_plan() {
        let mut protocol = serde_json::from_str::<H1ProtocolAInput>(include_str!(
            "../../fixtures/h1_protocol_a_valid.json"
        ))
        .expect("parse checked Protocol-A fixture");
        let preflight = serde_json::from_str::<H1PreflightInput>(include_str!(
            "../../fixtures/h1_preflight_valid.json"
        ))
        .expect("parse checked preflight fixture");
        assert!(declaration_matches_plan(&preflight.declaration, &protocol));
        protocol.plan.execution_context = "different-process".to_string();
        assert!(!declaration_matches_plan(&preflight.declaration, &protocol));
    }
}
