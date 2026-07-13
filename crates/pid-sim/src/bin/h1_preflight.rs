use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, Context, Result};
use pid_runlog::{RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use pid_sim::h1_preflight::{
    validate_h1_preflight, H1ArtifactRef, H1ClockDomainKind, H1DesignBlindingOrderEntry,
    H1MissingValuePolicy, H1NoninterferenceTolerances, H1OutputAxisScale, H1OutputMetric,
    H1PidAbstentionPolicy, H1PreflightInput, H1PreflightReport, H1PreflightValidationScope,
    H1PrimaryProtocol, H1SplitManifestEntry, H1TargetPopulation, H1_PREFLIGHT_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const COMPONENT: &str = "pid-h1-preflight";
const MAX_INPUT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
const MAX_OPAQUE_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const SOFTWARE_PREFLIGHT_SCOPE: &str = "software_preflight_only";
const STRUCTURAL_PREFLIGHT_INTERPRETATION: &str = "structural_preflight_only";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct Args {
    input: PathBuf,
    artifact_root: Option<PathBuf>,
    summary_json: PathBuf,
    runlog: PathBuf,
}

struct TemporaryPathGuard {
    path: PathBuf,
    installed: bool,
}

struct InputSnapshot {
    bytes: Option<Vec<u8>>,
    sha256: String,
    size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
struct VerifiedArtifactRecord {
    roles: Vec<String>,
    uri: String,
    sha256: String,
    byte_len: u64,
}

struct ArtifactVerification {
    issues: Vec<H1PreflightCliIssue>,
    verified: Vec<VerifiedArtifactRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ArtifactKind {
    Opaque,
    StrictJson,
    SourceRun,
}

struct ArtifactGroup {
    expected_sha256: String,
    roles: BTreeSet<String>,
    kind: ArtifactKind,
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

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct H1PreflightCliIssue {
    code: String,
    field: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct H1PreflightSummary {
    schema_version: u32,
    run_id: String,
    input_uri: String,
    input_sha256: String,
    config_hash: String,
    parsed: bool,
    passed: bool,
    establishes_h1_evidence: bool,
    evidence_bundle_hash: String,
    verified_artifacts: Vec<VerifiedArtifactRecord>,
    report: Option<H1PreflightReport>,
    fatal_issues: Vec<H1PreflightCliIssue>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct H1PreflightVerdict<'a> {
    schema_version: u32,
    primary_protocol: &'a str,
    passed: bool,
    establishes_h1_evidence: bool,
    denominators: Option<&'a pid_sim::h1_preflight::H1PreflightDenominators>,
    issue_count: usize,
    reason_codes: Vec<String>,
    input_sha256: &'a str,
    summary_sha256: &'a str,
    evidence_bundle_hash: &'a str,
    verified_artifact_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1SplitManifestDocument {
    schema_version: u32,
    entries: Vec<H1SplitManifestEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1AnalysisPlanDocument {
    schema_version: u32,
    scope: String,
    primary_protocol: H1PrimaryProtocol,
    validation_scope: H1PreflightValidationScope,
    source_run_id: String,
    target_population_id: String,
    clock_epoch_id: String,
    policy_id: String,
    policy_spec_artifact: H1ArtifactRef,
    policy_spec_sha256: String,
    instrumentation_id: String,
    instrumentation_spec_artifact: H1ArtifactRef,
    instrumentation_spec_sha256: String,
    execution_context: String,
    baseline_state_boundary: String,
    application_boundary: String,
    reset_boundary: String,
    protocol_clone_boundary: String,
    treatment_site: String,
    control_version: String,
    treatment_version: String,
    treatment_dose: f64,
    treatment_dose_unit: String,
    missing_value_policy: H1MissingValuePolicy,
    pid_abstention_policy: H1PidAbstentionPolicy,
    tolerances: H1NoninterferenceTolerances,
    minimum_repeats: usize,
    permitted_interpretation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1PolicySpecDocument {
    schema_version: u32,
    scope: String,
    policy_id: String,
    semantic_sha256: String,
    policy: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1InstrumentationSpecDocument {
    schema_version: u32,
    scope: String,
    instrumentation_id: String,
    semantic_sha256: String,
    spec: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1TargetPopulationDocument {
    schema_version: u32,
    population_id: String,
    target_population: H1TargetPopulation,
    scope: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1ClockContractDocument {
    schema_version: u32,
    domain_id: String,
    kind: H1ClockDomainKind,
    epoch: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1DesignManifestDocument {
    schema_version: u32,
    entries: Vec<H1DesignBlindingOrderEntry>,
    scope: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1OutputMetricDocument {
    schema_version: u32,
    metric: H1OutputMetric,
    axes: Vec<H1OutputAxisScale>,
    scope: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum H1ReceiptSide {
    Uninstrumented,
    Instrumented,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1ReceiptEntry {
    case_id: String,
    repeat_id: String,
    paired_starting_state_sha256: String,
    sides: Vec<H1ReceiptSide>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1InputReceiptEntry {
    case_id: String,
    repeat_id: String,
    blinded_fixture_id: String,
    paired_starting_state_sha256: String,
    sides: Vec<H1ReceiptSide>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1ResetReceiptDocument {
    schema_version: u32,
    scope: String,
    status: String,
    reset_boundary: String,
    entries: Vec<H1ReceiptEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1RngReceiptDocument {
    schema_version: u32,
    scope: String,
    status: String,
    coupling: String,
    entries: Vec<H1ReceiptEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct H1InputReceiptDocument {
    schema_version: u32,
    scope: String,
    status: String,
    entries: Vec<H1InputReceiptEntry>,
}

fn main() -> Result<()> {
    let Some(args) = parse_args()? else {
        print_usage();
        return Ok(());
    };
    ensure_parent(&args.summary_json)?;
    ensure_parent(&args.runlog)?;
    ensure_distinct_paths(&args)?;

    let input_snapshot = read_input_snapshot(&args.input)?;
    let input_sha256 = input_snapshot.sha256;
    let artifact_root = canonical_artifact_root(&args)?;
    let (parsed, load_issue) = if let Some(input_bytes) = input_snapshot.bytes {
        match serde_json::from_slice::<H1PreflightInput>(&input_bytes) {
            Ok(input) => (Some(input), None),
            Err(error) => (
                None,
                Some(H1PreflightCliIssue {
                    code: "contract_parse_failed".to_string(),
                    field: "input".to_string(),
                    message: error.to_string(),
                }),
            ),
        }
    } else {
        (
            None,
            Some(H1PreflightCliIssue {
                code: "input_resource_limit_exceeded".to_string(),
                field: "input".to_string(),
                message: format!(
                    "input is {} bytes; the limit is {MAX_INPUT_BYTES}",
                    input_snapshot.size
                ),
            }),
        )
    };

    let (run_id, protocol, config, report, fatal_issues, verified_artifacts, evidence_bundle_hash) =
        match parsed {
            Some(input) => {
                let run_id = preflight_run_id(&input_sha256);
                let protocol = Some(input.declaration.primary_protocol);
                let verification = verify_input_artifacts(&input, &artifact_root, &args)?;
                let evidence_bundle_hash =
                    pid_runlog::canonical_json_hash_v2(&verification.verified)?;
                let config = json!({
                    "component": COMPONENT,
                    "preflight_schema_version": H1_PREFLIGHT_SCHEMA_VERSION,
                    "input_sha256": input_sha256,
                    "evidence_bundle_hash": evidence_bundle_hash,
                    "verified_artifacts": verification.verified,
                    "artifact_resolution": "relative_to_cli_artifact_root",
                    "timestamp_semantics": "deterministic_event_index_not_capture_clock",
                    "evidence_scope": "software_preflight_only",
                    "declaration": input.declaration,
                });
                let report = validate_h1_preflight(&input);
                (
                    run_id,
                    protocol,
                    config,
                    Some(report),
                    verification.issues,
                    verification.verified,
                    evidence_bundle_hash,
                )
            }
            None => {
                let run_id = preflight_run_id(&input_sha256);
                let verified_artifacts = Vec::<VerifiedArtifactRecord>::new();
                let evidence_bundle_hash = pid_runlog::canonical_json_hash_v2(&verified_artifacts)?;
                let config = json!({
                    "component": COMPONENT,
                    "preflight_schema_version": H1_PREFLIGHT_SCHEMA_VERSION,
                    "input_sha256": input_sha256,
                    "evidence_bundle_hash": evidence_bundle_hash,
                    "verified_artifacts": verified_artifacts,
                    "artifact_resolution": "not_evaluated_for_unparsed_input",
                    "timestamp_semantics": "deterministic_event_index_not_capture_clock",
                    "evidence_scope": "software_preflight_only",
                    "parsed": false,
                });
                (
                    run_id,
                    None,
                    config,
                    None,
                    vec![load_issue.context("unparsed input must have a load issue")?],
                    verified_artifacts,
                    evidence_bundle_hash,
                )
            }
        };

    let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
    let passed =
        report.as_ref().is_some_and(H1PreflightReport::is_valid) && fatal_issues.is_empty();
    let summary = H1PreflightSummary {
        schema_version: H1_PREFLIGHT_SCHEMA_VERSION,
        run_id: run_id.clone(),
        input_uri: args.input.display().to_string(),
        input_sha256: input_sha256.clone(),
        config_hash: config_hash.clone(),
        parsed: report.is_some(),
        passed,
        establishes_h1_evidence: false,
        evidence_bundle_hash,
        verified_artifacts,
        report,
        fatal_issues,
    };

    let summary_bytes = serde_json::to_vec_pretty(&summary)?;
    let summary_sha256 = sha256_bytes(&summary_bytes);
    write_atomic_bytes(&args.summary_json, &summary_bytes)?;
    write_runlog(
        &args,
        &run_id,
        protocol,
        &input_sha256,
        &summary_sha256,
        &config_hash,
        config,
        &summary,
    )?;

    println!("run_id={run_id}");
    println!("parsed={}", summary.parsed);
    println!("passed={}", summary.passed);
    println!("establishes_h1_evidence=false");
    println!("wrote_summary={}", args.summary_json.display());
    println!("wrote_runlog={}", args.runlog.display());

    if !summary.passed {
        bail!("H1 common preflight failed closed; inspect the summary and valid failed run log");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_runlog(
    args: &Args,
    run_id: &str,
    protocol: Option<H1PrimaryProtocol>,
    input_sha256: &str,
    summary_sha256: &str,
    config_hash: &str,
    config: Value,
    summary: &H1PreflightSummary,
) -> Result<()> {
    let protocol_name = protocol.map(protocol_label).unwrap_or("unparsed");
    let common_metadata = [
        ("component".to_string(), COMPONENT.to_string()),
        ("claim".to_string(), "H1".to_string()),
        ("primary_protocol".to_string(), protocol_name.to_string()),
        (
            "scope".to_string(),
            "fixture_validated_common_preflight_not_h1_evidence".to_string(),
        ),
        (
            "timestamp_semantics".to_string(),
            "deterministic_event_index_not_capture_clock".to_string(),
        ),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();

    let temporary_runlog = temporary_runlog_path(&args.runlog);
    if comparable_path(&temporary_runlog)? == comparable_path(&args.input)?
        || comparable_path(&temporary_runlog)? == comparable_path(&args.summary_json)?
    {
        bail!("temporary run-log path collides with an input or summary path");
    }
    let temporary_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_runlog)
        .with_context(|| {
            format!(
                "failed to create temporary run log {}",
                temporary_runlog.display()
            )
        })?;
    let mut temporary_guard = TemporaryPathGuard::new(temporary_runlog.clone());
    let mut writer = RunLogWriter::new(BufWriter::new(temporary_file));
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: run_id.to_string(),
        timestamp_ns: 0,
        config_hash: config_hash.to_string(),
        metadata: common_metadata.clone(),
    })?;
    writer.append(&RunLogEvent::ConfigLogged {
        timestamp_ns: 1,
        config_hash: config_hash.to_string(),
        config,
    })?;
    writer.append(&RunLogEvent::ArtifactLogged {
        timestamp_ns: 2,
        name: "h1_preflight_input".to_string(),
        kind: "h1_preflight_input_json".to_string(),
        uri: args.input.display().to_string(),
        sha256: Some(input_sha256.to_string()),
        metadata: common_metadata.clone(),
    })?;
    let mut timestamp_ns = 3_u64;
    for artifact in &summary.verified_artifacts {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns,
            name: format!("h1_verified_{}", timestamp_ns - 3),
            kind: "h1_preflight_verified_input_artifact".to_string(),
            uri: artifact.uri.clone(),
            sha256: Some(artifact.sha256.clone()),
            metadata: [
                ("byte_len".to_string(), artifact.byte_len.to_string()),
                ("roles".to_string(), artifact.roles.join(",")),
            ]
            .into_iter()
            .collect(),
        })?;
        timestamp_ns += 1;
    }
    writer.append(&RunLogEvent::ArtifactLogged {
        timestamp_ns,
        name: "h1_preflight_summary".to_string(),
        kind: "h1_preflight_summary_json".to_string(),
        uri: args.summary_json.display().to_string(),
        sha256: Some(summary_sha256.to_string()),
        metadata: common_metadata.clone(),
    })?;
    timestamp_ns += 1;
    let reason_codes = summary_reason_codes(summary)?;
    let verdict = H1PreflightVerdict {
        schema_version: H1_PREFLIGHT_SCHEMA_VERSION,
        primary_protocol: protocol_name,
        passed: summary.passed,
        establishes_h1_evidence: false,
        denominators: summary.report.as_ref().map(|report| &report.denominators),
        issue_count: summary_issue_count(summary),
        reason_codes: reason_codes.clone(),
        input_sha256,
        summary_sha256,
        evidence_bundle_hash: &summary.evidence_bundle_hash,
        verified_artifact_count: summary.verified_artifacts.len(),
    };
    writer.append(&RunLogEvent::LabelObserved {
        step: 0,
        timestamp_ns,
        name: "h1_preflight.verdict".to_string(),
        value: serde_json::to_value(verdict)?,
        metadata: common_metadata,
    })?;
    timestamp_ns += 1;

    if !summary.passed {
        writer.append(&RunLogEvent::ErrorLogged {
            step: None,
            timestamp_ns,
            message: format!(
                "h1_preflight_failed:{}",
                if reason_codes.is_empty() {
                    "unspecified".to_string()
                } else {
                    reason_codes.join(",")
                }
            ),
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
            "H1 common preflight contract passed; no H1 scientific evidence was produced"
                .to_string()
        } else {
            "H1 common preflight failed closed; no H1 scientific evidence was produced".to_string()
        }),
    })?;
    writer.flush_durable()?;
    drop(writer);
    fs::rename(&temporary_runlog, &args.runlog).with_context(|| {
        format!(
            "failed to atomically install run log {}",
            args.runlog.display()
        )
    })?;
    temporary_guard.installed = true;
    sync_parent(&args.runlog)?;
    Ok(())
}

fn summary_reason_codes(summary: &H1PreflightSummary) -> Result<Vec<String>> {
    let mut codes = summary
        .report
        .iter()
        .flat_map(|report| report.issues.iter())
        .map(|issue| reason_code(issue.code))
        .collect::<Result<Vec<_>>>()?;
    codes.extend(summary.fatal_issues.iter().map(|issue| issue.code.clone()));
    codes.sort();
    codes.dedup();
    Ok(codes)
}

fn summary_issue_count(summary: &H1PreflightSummary) -> usize {
    summary
        .report
        .as_ref()
        .map_or(0, |report| report.issues.len())
        + summary.fatal_issues.len()
}

fn reason_code(code: pid_sim::h1_preflight::H1PreflightReasonCode) -> Result<String> {
    let encoded = serde_json::to_value(code)?;
    encoded
        .as_str()
        .map(str::to_string)
        .context("H1 preflight reason code did not serialize as a string")
}

fn protocol_label(protocol: H1PrimaryProtocol) -> &'static str {
    match protocol {
        H1PrimaryProtocol::ProtocolA => "protocol_a",
        H1PrimaryProtocol::ProtocolB => "protocol_b",
    }
}

fn preflight_run_id(input_sha256: &str) -> String {
    format!("h1-preflight-{}", &input_sha256[..16])
}

fn parse_args() -> Result<Option<Args>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(None);
    }

    let mut input = None;
    let mut artifact_root = None;
    let mut summary_json = None;
    let mut runlog = None;
    let mut index = 0;
    while index < args.len() {
        let target = match args[index].as_str() {
            "--input" => &mut input,
            "--artifact-root" => &mut artifact_root,
            "--summary-json" => &mut summary_json,
            "--runlog" => &mut runlog,
            other => bail!("unknown argument: {other}"),
        };
        let value = args
            .get(index + 1)
            .with_context(|| format!("{} requires a path", args[index]))?;
        if target.replace(PathBuf::from(value)).is_some() {
            bail!("{} may be supplied only once", args[index]);
        }
        index += 2;
    }

    Ok(Some(Args {
        input: input.context("--input is required")?,
        artifact_root,
        summary_json: summary_json.context("--summary-json is required")?,
        runlog: runlog.context("--runlog is required")?,
    }))
}

fn ensure_distinct_paths(args: &Args) -> Result<()> {
    for (left_name, left, right_name, right) in [
        ("input", &args.input, "summary", &args.summary_json),
        ("input", &args.input, "runlog", &args.runlog),
        ("summary", &args.summary_json, "runlog", &args.runlog),
    ] {
        if comparable_path(left)? == comparable_path(right)? {
            bail!("{left_name} and {right_name} paths must be distinct");
        }
    }
    Ok(())
}

fn canonical_artifact_root(args: &Args) -> Result<PathBuf> {
    let root = args.artifact_root.clone().unwrap_or_else(|| {
        args.input
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    });
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve artifact root {}", root.display()))?;
    if !root.is_dir() {
        bail!("artifact root must be a directory: {}", root.display());
    }
    Ok(root)
}

fn read_input_snapshot(path: &Path) -> Result<InputSnapshot> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to open H1 preflight input {}", path.display()))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("failed to stat H1 preflight input {}", path.display()))?;
    if !metadata.is_file() {
        bail!("H1 preflight input must be a regular file");
    }

    let initial_capacity = usize::try_from(metadata.len().min(MAX_INPUT_BYTES)).unwrap_or(0);
    let mut bytes = Some(Vec::with_capacity(initial_capacity));
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut chunk)
            .with_context(|| format!("failed to read H1 preflight input {}", path.display()))?;
        if count == 0 {
            break;
        }
        size = size
            .checked_add(count as u64)
            .context("H1 preflight input byte count overflowed u64")?;
        hasher.update(&chunk[..count]);
        if size <= MAX_INPUT_BYTES {
            if let Some(buffer) = &mut bytes {
                buffer.extend_from_slice(&chunk[..count]);
            }
        } else {
            bytes = None;
        }
    }

    Ok(InputSnapshot {
        bytes,
        sha256: format!("{:x}", hasher.finalize()),
        size,
    })
}

fn verify_input_artifacts(
    input: &H1PreflightInput,
    artifact_root: &Path,
    args: &Args,
) -> Result<ArtifactVerification> {
    let declaration = &input.declaration;
    let mut groups = BTreeMap::<String, ArtifactGroup>::new();
    let mut issues = Vec::new();
    for (role, artifact, kind) in [
        (
            "declaration.source_run",
            &declaration.source_run,
            ArtifactKind::SourceRun,
        ),
        (
            "declaration.analysis_plan",
            &declaration.analysis_plan,
            ArtifactKind::StrictJson,
        ),
        (
            "declaration.policy_spec_artifact",
            &declaration.policy_spec_artifact,
            ArtifactKind::StrictJson,
        ),
        (
            "declaration.instrumentation_spec_artifact",
            &declaration.instrumentation_spec_artifact,
            ArtifactKind::StrictJson,
        ),
        (
            "declaration.split_manifest.artifact",
            &declaration.split_manifest.artifact,
            ArtifactKind::StrictJson,
        ),
        (
            "declaration.target_population_manifest",
            &declaration.target_population_manifest,
            ArtifactKind::StrictJson,
        ),
        (
            "declaration.design_blinding_order_manifest.artifact",
            &declaration.design_blinding_order_manifest.artifact,
            ArtifactKind::StrictJson,
        ),
        (
            "declaration.output_metric_contract.artifact",
            &declaration.output_metric_contract.artifact,
            ArtifactKind::StrictJson,
        ),
        (
            "declaration.clock.contract",
            &declaration.clock.contract,
            ArtifactKind::StrictJson,
        ),
    ] {
        add_artifact_group(&mut groups, role, artifact, kind, &mut issues);
    }
    for case in &input.cases {
        add_artifact_group(
            &mut groups,
            "case.baseline_snapshot",
            &case.baseline_snapshot.artifact,
            ArtifactKind::Opaque,
            &mut issues,
        );
        add_artifact_group(
            &mut groups,
            "case.moderator",
            &case.moderator.artifact,
            ArtifactKind::Opaque,
            &mut issues,
        );
        for repeat in &case.repeats {
            for (role, artifact) in [
                (
                    "repeat.paired_starting_state.artifact",
                    &repeat.paired_starting_state.artifact,
                ),
                (
                    "repeat.uninstrumented.memory_state",
                    &repeat.uninstrumented.memory_state,
                ),
                (
                    "repeat.uninstrumented.cache_state",
                    &repeat.uninstrumented.cache_state,
                ),
                (
                    "repeat.instrumented.memory_state",
                    &repeat.instrumented.memory_state,
                ),
                (
                    "repeat.instrumented.cache_state",
                    &repeat.instrumented.cache_state,
                ),
            ] {
                add_artifact_group(
                    &mut groups,
                    role,
                    artifact,
                    ArtifactKind::Opaque,
                    &mut issues,
                );
            }
            for (role, artifact) in [
                (
                    "repeat.paired_starting_state.reset_receipt",
                    &repeat.paired_starting_state.reset_receipt,
                ),
                (
                    "repeat.paired_starting_state.rng_coupling_receipt",
                    &repeat.paired_starting_state.rng_coupling_receipt,
                ),
                (
                    "repeat.paired_starting_state.input_coupling_receipt",
                    &repeat.paired_starting_state.input_coupling_receipt,
                ),
            ] {
                add_artifact_group(
                    &mut groups,
                    role,
                    artifact,
                    ArtifactKind::StrictJson,
                    &mut issues,
                );
            }
        }
    }

    let output_paths = [
        comparable_path(&args.summary_json)?,
        comparable_path(&args.runlog)?,
    ];
    let mut documents = BTreeMap::<String, Vec<u8>>::new();
    let mut verified = Vec::new();
    for (uri, group) in groups {
        let reference = H1ArtifactRef {
            artifact_uri: uri.clone(),
            sha256: group.expected_sha256.clone(),
        };
        match resolve_artifact(artifact_root, &reference) {
            Ok(path) => {
                if output_paths.iter().any(|output| output == &path) {
                    bail!(
                        "artifact {uri} aliases an output path; refusing to overwrite {}",
                        path.display()
                    );
                }
                let captured = match group.kind {
                    ArtifactKind::Opaque => hash_exact_file(&path, MAX_OPAQUE_ARTIFACT_BYTES)
                        .map(|(sha256, byte_len)| (sha256, byte_len, None, None)),
                    ArtifactKind::StrictJson => read_exact_file(&path, MAX_MANIFEST_BYTES)
                        .map(|(bytes, sha256, byte_len)| (sha256, byte_len, Some(bytes), None)),
                    ArtifactKind::SourceRun => snapshot_source_run(&path, &args.runlog)
                        .map(|(sha256, byte_len, summary)| (sha256, byte_len, None, Some(summary))),
                };
                match captured {
                    Ok((actual, byte_len, bytes, source_summary))
                        if actual == group.expected_sha256 =>
                    {
                        let roles = group.roles.into_iter().collect::<Vec<_>>();
                        if let Some(bytes) = bytes {
                            documents.insert(uri.clone(), bytes);
                        }
                        if let Some(summary) = source_summary {
                            if summary.validation_errors > 0 {
                                issues.push(H1PreflightCliIssue {
                                    code: "source_run_invalid".to_string(),
                                    field: "declaration.source_run".to_string(),
                                    message: format!(
                                        "source run has {} validation errors",
                                        summary.validation_errors
                                    ),
                                });
                            } else if summary.run_id.as_deref()
                                != Some(declaration.source_run_id.as_str())
                            {
                                issues.push(H1PreflightCliIssue {
                                    code: "source_run_id_mismatch".to_string(),
                                    field: "declaration.source_run_id".to_string(),
                                    message: format!(
                                        "source run records {:?}, declaration names {:?}",
                                        summary.run_id, declaration.source_run_id
                                    ),
                                });
                            }
                        }
                        verified.push(VerifiedArtifactRecord {
                            roles,
                            uri,
                            sha256: actual,
                            byte_len,
                        });
                    }
                    Ok((actual, _, _, _)) => issues.push(H1PreflightCliIssue {
                        code: "artifact_hash_mismatch".to_string(),
                        field: group.roles.into_iter().collect::<Vec<_>>().join(","),
                        message: format!(
                            "artifact {uri} has SHA-256 {actual}, expected {}",
                            group.expected_sha256
                        ),
                    }),
                    Err(error) => issues.push(H1PreflightCliIssue {
                        code: "artifact_snapshot_failed".to_string(),
                        field: group.roles.into_iter().collect::<Vec<_>>().join(","),
                        message: error.to_string(),
                    }),
                }
            }
            Err(error) => issues.push(H1PreflightCliIssue {
                code: "artifact_path_invalid".to_string(),
                field: group.roles.into_iter().collect::<Vec<_>>().join(","),
                message: error.to_string(),
            }),
        }
    }
    validate_semantic_documents(input, &documents, &mut issues);
    Ok(ArtifactVerification { issues, verified })
}

fn resolve_artifact(root: &Path, artifact: &H1ArtifactRef) -> Result<PathBuf> {
    let relative = Path::new(&artifact.artifact_uri);
    if relative.is_absolute()
        || relative.components().any(|component| {
            !matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        bail!("artifact URI must be a relative path without parent traversal");
    }
    let resolved = root.join(relative).canonicalize().with_context(|| {
        format!(
            "failed to resolve artifact {} below root {}",
            artifact.artifact_uri,
            root.display()
        )
    })?;
    if !resolved.starts_with(root) {
        bail!("artifact path escapes the declared artifact root");
    }
    if !resolved.is_file() {
        bail!(
            "artifact path is not a regular file: {}",
            resolved.display()
        );
    }
    Ok(resolved)
}

fn add_artifact_group(
    groups: &mut BTreeMap<String, ArtifactGroup>,
    role: &str,
    artifact: &H1ArtifactRef,
    kind: ArtifactKind,
    issues: &mut Vec<H1PreflightCliIssue>,
) {
    if let Some(group) = groups.get_mut(&artifact.artifact_uri) {
        group.roles.insert(role.to_string());
        if group.kind != kind {
            issues.push(H1PreflightCliIssue {
                code: "artifact_role_kind_conflict".to_string(),
                field: role.to_string(),
                message: format!(
                    "artifact URI {} is declared for incompatible {:?} and {:?} roles",
                    artifact.artifact_uri, group.kind, kind
                ),
            });
        }
        group.kind = group.kind.max(kind);
        if group.expected_sha256 != artifact.sha256 {
            issues.push(H1PreflightCliIssue {
                code: "artifact_identity_conflict".to_string(),
                field: role.to_string(),
                message: format!(
                    "artifact URI {} is declared with hashes {} and {}",
                    artifact.artifact_uri, group.expected_sha256, artifact.sha256
                ),
            });
        }
    } else {
        groups.insert(
            artifact.artifact_uri.clone(),
            ArtifactGroup {
                expected_sha256: artifact.sha256.clone(),
                roles: [role.to_string()].into_iter().collect(),
                kind,
            },
        );
    }
}

fn read_exact_file(path: &Path, max_bytes: u64) -> Result<(Vec<u8>, String, u64)> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to open artifact {}", path.display()))?;
    if !file.metadata()?.is_file() {
        bail!("artifact is not a regular file: {}", path.display());
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
            .context("artifact byte count overflowed u64")?;
        if byte_len > max_bytes {
            bail!(
                "artifact {} exceeds the {}-byte snapshot limit",
                path.display(),
                max_bytes
            );
        }
        hasher.update(&chunk[..count]);
        bytes
            .try_reserve(count)
            .context("failed to reserve artifact snapshot buffer")?;
        bytes.extend_from_slice(&chunk[..count]);
    }
    Ok((bytes, format!("{:x}", hasher.finalize()), byte_len))
}

fn hash_exact_file(path: &Path, max_bytes: u64) -> Result<(String, u64)> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to open artifact {}", path.display()))?;
    if !file.metadata()?.is_file() {
        bail!("artifact is not a regular file: {}", path.display());
    }
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
            .context("artifact byte count overflowed u64")?;
        if byte_len > max_bytes {
            bail!(
                "artifact {} exceeds the {}-byte hashing limit",
                path.display(),
                max_bytes
            );
        }
        hasher.update(&chunk[..count]);
    }
    Ok((format!("{:x}", hasher.finalize()), byte_len))
}

fn snapshot_source_run(
    source_path: &Path,
    runlog_output_path: &Path,
) -> Result<(String, u64, pid_runlog::RunLogSummary)> {
    let mut source = fs::File::open(source_path)
        .with_context(|| format!("failed to open source run {}", source_path.display()))?;
    if !source.metadata()?.is_file() {
        bail!("source run is not a regular file");
    }
    let snapshot_path = temporary_source_snapshot_path(runlog_output_path);
    let snapshot_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&snapshot_path)
        .with_context(|| {
            format!(
                "failed to create source-run snapshot {}",
                snapshot_path.display()
            )
        })?;
    let _guard = TemporaryPathGuard::new(snapshot_path);
    let inspection_file = snapshot_file.try_clone()?;
    let mut writer = BufWriter::new(snapshot_file);
    let mut hasher = Sha256::new();
    let mut byte_len = 0_u64;
    let max_bytes = pid_runlog::RunLogLimits::default().max_file_bytes;
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        let count = source.read(&mut chunk)?;
        if count == 0 {
            break;
        }
        byte_len = byte_len
            .checked_add(count as u64)
            .context("source-run byte count overflowed u64")?;
        if byte_len > max_bytes {
            bail!("source run exceeds the {max_bytes}-byte validation limit");
        }
        hasher.update(&chunk[..count]);
        writer.write_all(&chunk[..count])?;
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);

    let mut inspection_file = inspection_file;
    inspection_file.seek(SeekFrom::Start(0))?;
    let events = pid_runlog::read_events(BufReader::new(inspection_file))?;
    let summary = pid_runlog::summarize_events(&events)?;
    Ok((format!("{:x}", hasher.finalize()), byte_len, summary))
}

fn validate_semantic_documents(
    input: &H1PreflightInput,
    documents: &BTreeMap<String, Vec<u8>>,
    issues: &mut Vec<H1PreflightCliIssue>,
) {
    let declaration = &input.declaration;
    validate_one_document::<H1SplitManifestDocument>(
        documents,
        &declaration.split_manifest.artifact,
        "split_manifest",
        "split_manifest_parse_failed",
        issues,
        |document| {
            document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                && document.entries == declaration.split_manifest.entries
        },
    );
    validate_one_document::<H1AnalysisPlanDocument>(
        documents,
        &declaration.analysis_plan,
        "analysis_plan",
        "analysis_plan_parse_failed",
        issues,
        |document| {
            document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                && document.scope == SOFTWARE_PREFLIGHT_SCOPE
                && document.primary_protocol == declaration.primary_protocol
                && document.validation_scope == declaration.validation_scope
                && document.source_run_id == declaration.source_run_id
                && document.target_population_id == declaration.target_population_id
                && document.clock_epoch_id == declaration.clock.epoch_id
                && document.policy_id == declaration.policy_id
                && document.policy_spec_artifact == declaration.policy_spec_artifact
                && document.policy_spec_sha256 == declaration.policy_spec_sha256
                && document.instrumentation_id == declaration.instrumentation_id
                && document.instrumentation_spec_artifact
                    == declaration.instrumentation_spec_artifact
                && document.instrumentation_spec_sha256 == declaration.instrumentation_spec_sha256
                && document.execution_context == declaration.execution_context
                && document.baseline_state_boundary == declaration.baseline_state_boundary
                && document.application_boundary == declaration.application_boundary
                && document.reset_boundary == declaration.reset_boundary
                && document.protocol_clone_boundary == declaration.protocol_clone_boundary
                && document.treatment_site == declaration.treatment_site
                && document.control_version == declaration.control_version
                && document.treatment_version == declaration.treatment_version
                && document.treatment_dose == declaration.treatment_dose
                && document.treatment_dose_unit == declaration.treatment_dose_unit
                && document.missing_value_policy == declaration.missing_value_policy
                && document.pid_abstention_policy == declaration.pid_abstention_policy
                && document.tolerances == declaration.tolerances
                && document.minimum_repeats == declaration.minimum_repeats
                && document.permitted_interpretation == STRUCTURAL_PREFLIGHT_INTERPRETATION
        },
    );
    validate_one_document::<H1PolicySpecDocument>(
        documents,
        &declaration.policy_spec_artifact,
        "policy_spec",
        "policy_spec_parse_failed",
        issues,
        |document| {
            document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                && document.scope == SOFTWARE_PREFLIGHT_SCOPE
                && document.policy_id == declaration.policy_id
                && document.semantic_sha256 == declaration.policy_spec_sha256
                && pid_runlog::canonical_json_hash_v2(&document.policy)
                    .as_ref()
                    .ok()
                    .map(String::as_str)
                    == Some(declaration.policy_spec_sha256.as_str())
        },
    );
    validate_one_document::<H1InstrumentationSpecDocument>(
        documents,
        &declaration.instrumentation_spec_artifact,
        "instrumentation_spec",
        "instrumentation_spec_parse_failed",
        issues,
        |document| {
            document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                && document.scope == SOFTWARE_PREFLIGHT_SCOPE
                && document.instrumentation_id == declaration.instrumentation_id
                && document.semantic_sha256 == declaration.instrumentation_spec_sha256
                && document
                    .spec
                    .get("execution_context")
                    .and_then(Value::as_str)
                    == Some(declaration.execution_context.as_str())
                && pid_runlog::canonical_json_hash_v2(&document.spec)
                    .as_ref()
                    .ok()
                    .map(String::as_str)
                    == Some(declaration.instrumentation_spec_sha256.as_str())
        },
    );
    validate_one_document::<H1TargetPopulationDocument>(
        documents,
        &declaration.target_population_manifest,
        "target_population",
        "target_population_parse_failed",
        issues,
        |document| {
            document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                && document.population_id == declaration.target_population_id
                && document.target_population == declaration.target_population
                && document.scope == SOFTWARE_PREFLIGHT_SCOPE
        },
    );
    validate_one_document::<H1ClockContractDocument>(
        documents,
        &declaration.clock.contract,
        "clock_contract",
        "clock_contract_parse_failed",
        issues,
        |document| {
            document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                && document.domain_id == declaration.clock.domain_id
                && document.kind == declaration.clock.kind
                && document.epoch == declaration.clock.epoch_id
                && document.scope == SOFTWARE_PREFLIGHT_SCOPE
        },
    );
    validate_one_document::<H1DesignManifestDocument>(
        documents,
        &declaration.design_blinding_order_manifest.artifact,
        "design_manifest",
        "design_manifest_parse_failed",
        issues,
        |document| {
            document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                && document.entries == declaration.design_blinding_order_manifest.entries
                && document.scope == SOFTWARE_PREFLIGHT_SCOPE
        },
    );
    validate_one_document::<H1OutputMetricDocument>(
        documents,
        &declaration.output_metric_contract.artifact,
        "output_metric",
        "output_metric_parse_failed",
        issues,
        |document| {
            document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                && document.metric == declaration.output_metric_contract.metric
                && document.axes == declaration.output_metric_contract.axes
                && document.scope == SOFTWARE_PREFLIGHT_SCOPE
        },
    );

    let mut reset_receipts = BTreeMap::<String, (&H1ArtifactRef, Vec<H1ReceiptEntry>)>::new();
    let mut rng_receipts = BTreeMap::<String, (&H1ArtifactRef, Vec<H1ReceiptEntry>)>::new();
    let mut input_receipts = BTreeMap::<String, (&H1ArtifactRef, Vec<H1InputReceiptEntry>)>::new();
    for case in &input.cases {
        for repeat in &case.repeats {
            let sides = vec![H1ReceiptSide::Uninstrumented, H1ReceiptSide::Instrumented];
            let receipt_entry = || H1ReceiptEntry {
                case_id: case.case_id.clone(),
                repeat_id: repeat.repeat_id.clone(),
                paired_starting_state_sha256: repeat.paired_starting_state.artifact.sha256.clone(),
                sides: sides.clone(),
            };
            reset_receipts
                .entry(
                    repeat
                        .paired_starting_state
                        .reset_receipt
                        .artifact_uri
                        .clone(),
                )
                .or_insert_with(|| (&repeat.paired_starting_state.reset_receipt, Vec::new()))
                .1
                .push(receipt_entry());
            rng_receipts
                .entry(
                    repeat
                        .paired_starting_state
                        .rng_coupling_receipt
                        .artifact_uri
                        .clone(),
                )
                .or_insert_with(|| {
                    (
                        &repeat.paired_starting_state.rng_coupling_receipt,
                        Vec::new(),
                    )
                })
                .1
                .push(receipt_entry());
            input_receipts
                .entry(
                    repeat
                        .paired_starting_state
                        .input_coupling_receipt
                        .artifact_uri
                        .clone(),
                )
                .or_insert_with(|| {
                    (
                        &repeat.paired_starting_state.input_coupling_receipt,
                        Vec::new(),
                    )
                })
                .1
                .push(H1InputReceiptEntry {
                    case_id: case.case_id.clone(),
                    repeat_id: repeat.repeat_id.clone(),
                    blinded_fixture_id: repeat.blinded_fixture_id.clone(),
                    paired_starting_state_sha256: repeat
                        .paired_starting_state
                        .artifact
                        .sha256
                        .clone(),
                    sides,
                });
        }
    }
    for (_, (artifact, entries)) in reset_receipts {
        validate_one_document::<H1ResetReceiptDocument>(
            documents,
            artifact,
            "reset_receipt",
            "reset_receipt_parse_failed",
            issues,
            |document| {
                document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                    && document.scope == SOFTWARE_PREFLIGHT_SCOPE
                    && document.status == "applied"
                    && document.reset_boundary == declaration.reset_boundary
                    && document.entries == entries
            },
        );
    }
    for (_, (artifact, entries)) in rng_receipts {
        validate_one_document::<H1RngReceiptDocument>(
            documents,
            artifact,
            "rng_coupling_receipt",
            "rng_coupling_receipt_parse_failed",
            issues,
            |document| {
                document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                    && document.scope == SOFTWARE_PREFLIGHT_SCOPE
                    && document.status == "applied"
                    && document.coupling == "identical_counter_stream"
                    && document.entries == entries
            },
        );
    }
    for (_, (artifact, entries)) in input_receipts {
        validate_one_document::<H1InputReceiptDocument>(
            documents,
            artifact,
            "input_coupling_receipt",
            "input_coupling_receipt_parse_failed",
            issues,
            |document| {
                document.schema_version == H1_PREFLIGHT_SCHEMA_VERSION
                    && document.scope == SOFTWARE_PREFLIGHT_SCOPE
                    && document.status == "applied"
                    && document.entries == entries
            },
        );
    }
}

fn validate_one_document<T: for<'de> Deserialize<'de>>(
    documents: &BTreeMap<String, Vec<u8>>,
    artifact: &H1ArtifactRef,
    field: &str,
    parse_code: &str,
    issues: &mut Vec<H1PreflightCliIssue>,
    matches: impl FnOnce(&T) -> bool,
) {
    let Some(bytes) = documents.get(&artifact.artifact_uri) else {
        issues.push(H1PreflightCliIssue {
            code: format!("{field}_snapshot_missing"),
            field: format!("declaration.{field}"),
            message: "verified semantic document bytes are unavailable".to_string(),
        });
        return;
    };
    match serde_json::from_slice::<T>(bytes) {
        Ok(document) if matches(&document) => {}
        Ok(_) => issues.push(H1PreflightCliIssue {
            code: format!("{field}_content_mismatch"),
            field: format!("declaration.{field}"),
            message: "artifact contents do not exactly match the typed declaration".to_string(),
        }),
        Err(error) => issues.push(H1PreflightCliIssue {
            code: parse_code.to_string(),
            field: format!("declaration.{field}"),
            message: error.to_string(),
        }),
    }
}

fn comparable_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path
            .canonicalize()
            .with_context(|| format!("failed to resolve path {}", path.display()));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let normalized = normalize_lexically(&absolute);
    if let (Some(parent), Some(file_name)) = (normalized.parent(), normalized.file_name()) {
        if let Ok(canonical_parent) = parent.canonicalize() {
            return Ok(canonical_parent.join(file_name));
        }
    }
    Ok(normalized)
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }
    Ok(())
}

fn temporary_runlog_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map_or_else(|| "runlog".into(), |name| name.to_string_lossy());
    path.with_file_name(format!(
        ".{file_name}.{}.{}.h1-preflight.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn temporary_source_snapshot_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map_or_else(|| "runlog".into(), |name| name.to_string_lossy());
    path.with_file_name(format!(
        ".{file_name}.{}.{}.source-snapshot.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn temporary_atomic_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map_or_else(|| "output".into(), |name| name.to_string_lossy());
    path.with_file_name(format!(
        ".{file_name}.{}.{}.atomic.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = temporary_atomic_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("failed to create temporary output {}", temporary.display()))?;
    let mut guard = TemporaryPathGuard::new(temporary.clone());
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, path)
        .with_context(|| format!("failed to atomically install output {}", path.display()))?;
    guard.installed = true;
    sync_parent(path)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn sync_parent(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::File::open(parent)
        .with_context(|| format!("failed to open output directory {}", parent.display()))?
        .sync_all()
        .with_context(|| format!("failed to fsync output directory {}", parent.display()))?;
    Ok(())
}

fn print_usage() {
    println!(
        "Usage: pid-h1-preflight --input PATH [--artifact-root PATH] --summary-json PATH --runlog PATH\n\
         Validates common H1 capture/noninterference semantics only; a pass is not H1 evidence."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
    }

    fn valid_input() -> H1PreflightInput {
        let bytes = fs::read(fixture_root().join("h1_preflight_valid.json")).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn args() -> Args {
        let suffix = std::process::id();
        Args {
            input: fixture_root().join("h1_preflight_valid.json"),
            artifact_root: Some(fixture_root()),
            summary_json: std::env::temp_dir()
                .join(format!("prisoma-h1-preflight-test-{suffix}-summary.json")),
            runlog: std::env::temp_dir()
                .join(format!("prisoma-h1-preflight-test-{suffix}-runlog.jsonl")),
        }
    }

    #[test]
    fn checked_fixture_artifacts_and_source_run_are_valid() {
        let verification =
            verify_input_artifacts(&valid_input(), &fixture_root(), &args()).unwrap();
        assert!(
            verification.issues.is_empty(),
            "artifact issues found: {:?}",
            verification
                .issues
                .iter()
                .map(|issue| (&issue.code, &issue.message))
                .collect::<Vec<_>>()
        );
        assert!(!verification.verified.is_empty());
    }

    #[test]
    fn artifact_hash_mismatch_fails_closed() {
        let mut input = valid_input();
        input.declaration.source_run.sha256 = "0".repeat(64);
        let verification = verify_input_artifacts(&input, &fixture_root(), &args()).unwrap();
        assert!(verification
            .issues
            .iter()
            .any(|issue| issue.code == "artifact_hash_mismatch"));
    }

    #[test]
    fn split_artifact_must_match_the_typed_input_copy() {
        let mut input = valid_input();
        input.declaration.split_manifest.entries[0].outer_fold = "fold-2".to_string();
        let verification = verify_input_artifacts(&input, &fixture_root(), &args()).unwrap();
        assert!(verification
            .issues
            .iter()
            .any(|issue| issue.code == "split_manifest_content_mismatch"));
    }

    #[test]
    fn source_run_id_must_match_the_verified_run_log() {
        let mut input = valid_input();
        input.declaration.source_run_id = "different-source-run".to_string();
        let verification = verify_input_artifacts(&input, &fixture_root(), &args()).unwrap();
        assert!(verification
            .issues
            .iter()
            .any(|issue| issue.code == "source_run_id_mismatch"));
    }

    #[test]
    fn artifact_resolution_rejects_parent_traversal() {
        let artifact = H1ArtifactRef {
            artifact_uri: "../outside".to_string(),
            sha256: "0".repeat(64),
        };
        assert!(resolve_artifact(&fixture_root(), &artifact).is_err());
    }

    #[test]
    fn output_paths_cannot_alias_the_input() {
        let mut args = args();
        args.summary_json = args.input.clone();
        assert!(ensure_distinct_paths(&args).is_err());
    }

    fn verification_codes(input: &H1PreflightInput) -> BTreeSet<String> {
        verify_input_artifacts(input, &fixture_root(), &args())
            .unwrap()
            .issues
            .into_iter()
            .map(|issue| issue.code)
            .collect()
    }

    #[test]
    fn semantic_manifests_must_match_the_inline_contract() {
        let mut analysis = valid_input();
        analysis.declaration.treatment_version = "different-treatment".to_string();
        assert!(verification_codes(&analysis).contains("analysis_plan_content_mismatch"));

        let mut population_id = valid_input();
        population_id.declaration.target_population_id = "different-population".to_string();
        let codes = verification_codes(&population_id);
        assert!(codes.contains("analysis_plan_content_mismatch"));
        assert!(codes.contains("target_population_content_mismatch"));

        let mut target = valid_input();
        target.declaration.target_population = H1TargetPopulation::TransportPopulation;
        assert!(verification_codes(&target).contains("target_population_content_mismatch"));

        let mut clock = valid_input();
        clock.declaration.clock.kind = H1ClockDomainKind::SimulatorMonotonic;
        assert!(verification_codes(&clock).contains("clock_contract_content_mismatch"));

        let mut clock_epoch = valid_input();
        clock_epoch.declaration.clock.epoch_id = "different-epoch".to_string();
        let codes = verification_codes(&clock_epoch);
        assert!(codes.contains("analysis_plan_content_mismatch"));
        assert!(codes.contains("clock_contract_content_mismatch"));

        let mut metric = valid_input();
        metric.declaration.output_metric_contract.axes[0].scale = 2.0;
        assert!(verification_codes(&metric).contains("output_metric_content_mismatch"));

        let mut design = valid_input();
        design.declaration.design_blinding_order_manifest.entries[0].blinded = false;
        assert!(verification_codes(&design).contains("design_manifest_content_mismatch"));
    }

    #[test]
    fn incompatible_artifact_roles_fail_closed_without_semantic_bytes() {
        let mut analysis_alias = valid_input();
        analysis_alias.declaration.analysis_plan = analysis_alias.declaration.source_run.clone();
        let codes = verification_codes(&analysis_alias);
        assert!(codes.contains("artifact_role_kind_conflict"));
        assert!(codes.contains("analysis_plan_snapshot_missing"));

        let mut receipt_alias = valid_input();
        receipt_alias.cases[0].repeats[0]
            .paired_starting_state
            .reset_receipt = receipt_alias.declaration.source_run.clone();
        let codes = verification_codes(&receipt_alias);
        assert!(codes.contains("artifact_role_kind_conflict"));
        assert!(codes.contains("reset_receipt_snapshot_missing"));
    }

    #[test]
    fn typed_receipts_are_bound_to_repeat_state_fixture_and_both_sides() {
        let mut starting_state = valid_input();
        starting_state.cases[0].repeats[0]
            .paired_starting_state
            .artifact
            .sha256 = "0".repeat(64);
        let codes = verification_codes(&starting_state);
        assert!(codes.contains("reset_receipt_content_mismatch"));
        assert!(codes.contains("rng_coupling_receipt_content_mismatch"));
        assert!(codes.contains("input_coupling_receipt_content_mismatch"));

        let mut fixture = valid_input();
        fixture.cases[0].repeats[0].blinded_fixture_id = "different-blind-id".to_string();
        assert!(verification_codes(&fixture).contains("input_coupling_receipt_content_mismatch"));
    }

    #[test]
    fn software_only_scope_and_interpretation_are_enforced() {
        let input = valid_input();
        let artifact = &input.declaration.analysis_plan;
        let bytes = fs::read(fixture_root().join(&artifact.artifact_uri)).unwrap();
        let mut document: Value = serde_json::from_slice(&bytes).unwrap();
        document["scope"] = Value::String("scientific_evidence".to_string());
        document["permitted_interpretation"] = Value::String("h1_confirmed".to_string());
        let mut documents = BTreeMap::new();
        documents.insert(
            artifact.artifact_uri.clone(),
            serde_json::to_vec(&document).unwrap(),
        );
        let mut issues = Vec::new();
        validate_semantic_documents(&input, &documents, &mut issues);
        assert!(issues
            .iter()
            .any(|issue| issue.code == "analysis_plan_content_mismatch"));
    }

    #[test]
    fn input_snapshot_hashes_the_exact_retained_bytes() {
        let path = std::env::temp_dir().join(format!(
            "prisoma-h1-input-snapshot-{}-{}.json",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let bytes = br#"{"declaration":"fixture"}"#;
        fs::write(&path, bytes).unwrap();
        let snapshot = read_input_snapshot(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!(snapshot.bytes.as_deref(), Some(bytes.as_slice()));
        assert_eq!(snapshot.sha256, sha256_bytes(bytes));
        assert_eq!(snapshot.size, bytes.len() as u64);
    }
}
