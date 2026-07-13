//! Fail-closed common preflight for the two H1 intervention-response protocols.
//!
//! This module validates capture semantics and diagnostic-instrumentation
//! noninterference.  A passing report establishes only that the supplied records
//! satisfy this software contract; it is not evidence that either H1 protocol
//! succeeds scientifically.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// The only preflight schema accepted by this implementation.
pub const H1_PREFLIGHT_SCHEMA_VERSION: u32 = 1;

/// Canonical identifiers are deliberately ASCII and bounded so visually or
/// bytewise different spellings cannot evade duplicate/fold checks.
pub const H1_MAX_IDENTIFIER_BYTES: usize = 256;
/// Artifact references are locators, not unbounded metadata containers.
pub const H1_MAX_ARTIFACT_URI_BYTES: usize = 4096;

/// The primary H1 protocol. Protocols A and B are deliberately not blendable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1PrimaryProtocol {
    ProtocolA,
    ProtocolB,
}

/// Population to which the frozen analysis intends to generalize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1TargetPopulation {
    FiniteBenchmark,
    TaskFamilySuperpopulation,
    TransportPopulation,
}

/// Frozen distance used to compare instrumented and uninstrumented policy output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1OutputMetric {
    L2,
    LInf,
}

/// One named output axis and the positive physical scale used to normalize it.
/// Distances use `(left - right) / scale`, yielding a dimensionless contrast.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1OutputAxisScale {
    pub axis_name: String,
    pub scale: f64,
    pub unit: String,
}

/// Frozen output metric, axis order, physical scales, and provenance artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1OutputMetricContract {
    pub artifact: H1ArtifactRef,
    pub metric: H1OutputMetric,
    pub axes: Vec<H1OutputAxisScale>,
}

/// Order in which a paired noninterference repeat was evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1EvaluationOrder {
    UninstrumentedFirst,
    InstrumentedFirst,
}

/// Where the proposed moderator was captured relative to treatment.
///
/// Keeping prohibited stages representable lets the validator return a stable
/// scientific reason code instead of turning a readable but invalid capture into
/// a JSON parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1ModeratorLineageStage {
    UntreatedBaseline,
    TreatedForwardPass,
    TreatmentEngagement,
    DownstreamController,
    FutureFrame,
    Unknown,
}

/// A content-addressed artifact reference.
///
/// The pure validator checks locator and digest syntax. The I/O boundary must
/// hash the referenced bytes and compare them with `sha256` before treating the
/// artifact as verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ArtifactRef {
    pub artifact_uri: String,
    pub sha256: String,
}

/// Clock families whose numeric timestamps have an order-preserving meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1ClockDomainKind {
    Monotonic,
    SimulatorMonotonic,
}

/// One explicit clock domain shared by every timestamp in an input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ClockDomainContract {
    pub domain_id: String,
    pub epoch_id: String,
    pub kind: H1ClockDomainKind,
    pub contract: H1ArtifactRef,
}

/// Missing moderator/evaluation data are never silently imputed by this gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1MissingValuePolicy {
    FailRun,
    AbstainCase,
}

/// PID is optional for H1, but its missing/abstained behavior must be frozen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1PidAbstentionPolicy {
    NotApplicable,
    RecordAndExclude,
    FailRun,
}

/// One immutable mapping in the outer split manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1SplitManifestEntry {
    pub case_id: String,
    pub task_family_id: String,
    pub interference_cluster_id: String,
    pub outer_fold: String,
}

/// Typed copy of the split manifest plus the artifact whose bytes the CLI must
/// verify. The validator requires exact one-to-one agreement with `cases`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1SplitManifest {
    pub artifact: H1ArtifactRef,
    pub entries: Vec<H1SplitManifestEntry>,
}

/// One predeclared blinded fixture and evaluation order for a paired repeat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1DesignBlindingOrderEntry {
    pub case_id: String,
    pub repeat_id: String,
    pub blinded_fixture_id: String,
    pub blinded: bool,
    pub evaluation_order: H1EvaluationOrder,
}

/// Typed copy of the design/blinding/order manifest plus its provenance artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1DesignBlindingOrderManifest {
    pub artifact: H1ArtifactRef,
    pub entries: Vec<H1DesignBlindingOrderEntry>,
}

/// Finite resource limits for validation after bounded input parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PreflightLimits {
    pub max_cases: usize,
    pub max_split_entries: usize,
    pub max_repeats_per_case: usize,
    pub max_total_repeats: usize,
    pub max_output_dimension: usize,
}

impl Default for H1PreflightLimits {
    fn default() -> Self {
        Self {
            max_cases: 100_000,
            max_split_entries: 100_000,
            max_repeats_per_case: 10_000,
            max_total_repeats: 1_000_000,
            max_output_dimension: 65_536,
        }
    }
}

/// Frozen thresholds for the common instrumentation-noninterference gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1NoninterferenceTolerances {
    pub output_distance_max: f64,
    pub latency_absolute_delta_ns_max: u64,
    pub latency_relative_slowdown_max: f64,
    pub controller_timing_delta_ns_max: u64,
}

/// Frozen, analysis-level declaration shared by every case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PreflightDeclaration {
    pub schema_version: u32,
    pub primary_protocol: H1PrimaryProtocol,
    pub source_run_id: String,
    pub source_run: H1ArtifactRef,
    pub analysis_plan: H1ArtifactRef,
    pub split_manifest: H1SplitManifest,
    pub target_population: H1TargetPopulation,
    pub target_population_id: String,
    pub target_population_manifest: H1ArtifactRef,
    pub design_blinding_order_manifest: H1DesignBlindingOrderManifest,
    pub output_metric_contract: H1OutputMetricContract,
    pub clock: H1ClockDomainContract,
    pub baseline_state_boundary: String,
    pub application_boundary: String,
    pub reset_boundary: String,
    pub treatment_site: String,
    pub treatment_version: String,
    pub treatment_dose: f64,
    pub treatment_dose_unit: String,
    pub output_metric: H1OutputMetric,
    pub missing_value_policy: H1MissingValuePolicy,
    pub pid_abstention_policy: H1PidAbstentionPolicy,
    pub tolerances: H1NoninterferenceTolerances,
    /// Required repeats per case. Both evaluation orders must be represented,
    /// so values below two are invalid declarations.
    pub minimum_repeats: usize,
}

/// Immutable, content-addressed artifact captured at a declared time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1TimedArtifact {
    pub artifact: H1ArtifactRef,
    pub captured_timestamp_ns: u64,
}

/// Candidate pre-treatment moderator and its snapshot lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ModeratorArtifact {
    pub artifact: H1ArtifactRef,
    pub lineage_stage: H1ModeratorLineageStage,
    pub source_snapshot_sha256: String,
    pub captured_timestamp_ns: u64,
}

/// Per-side claims binding an evaluation to the repeat's verified paired start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PairedStartReceiptHashes {
    pub starting_state_sha256: String,
    pub reset_receipt_sha256: String,
    pub rng_coupling_receipt_sha256: String,
    pub input_coupling_receipt_sha256: String,
}

/// Repeat-level shared start and reset/RNG/input coupling receipts.
///
/// This is distinct from [`H1PreflightCase::baseline_snapshot`]. The latter is
/// the observational untreated baseline used for moderator lineage; this record
/// is the verified starting state for one noninterference pair. Neither is the
/// later policy-clone state required by Protocol A itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PairedStartingState {
    pub artifact: H1ArtifactRef,
    pub source_baseline_snapshot_sha256: String,
    pub reset_receipt: H1ArtifactRef,
    pub rng_coupling_receipt: H1ArtifactRef,
    pub input_coupling_receipt: H1ArtifactRef,
}

/// One side of an instrumented/uninstrumented comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PolicyEvaluation {
    pub output: Vec<f64>,
    pub clock_domain_id: String,
    pub evaluation_start_timestamp_ns: u64,
    pub latency_ns: u64,
    pub controller_timestamp_ns: u64,
    pub paired_start_receipts: H1PairedStartReceiptHashes,
    pub memory_state: H1ArtifactRef,
    pub cache_state: H1ArtifactRef,
}

/// A blinded paired repeat from the same immutable untreated state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PairedNoninterferenceRepeat {
    pub repeat_id: String,
    pub blinded_fixture_id: String,
    pub blinded: bool,
    pub evaluation_order: H1EvaluationOrder,
    pub paired_starting_state: H1PairedStartingState,
    pub uninstrumented: H1PolicyEvaluation,
    pub instrumented: H1PolicyEvaluation,
    /// Producer-reported distance. The validator recomputes this from the
    /// output vectors and the declaration's metric before applying the gate.
    pub output_distance: f64,
}

/// One pre-treatment case and all of its paired instrumentation repeats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PreflightCase {
    pub case_id: String,
    pub task_family_id: String,
    pub interference_cluster_id: String,
    pub outer_fold: String,
    pub clock_domain_id: String,
    /// Baseline state from which the moderator is derived. This is not the
    /// later Protocol-A clone snapshot, which remains protocol-specific work.
    pub baseline_snapshot: H1TimedArtifact,
    pub moderator: H1ModeratorArtifact,
    /// Required by Protocol B. Protocol A may omit assignment because its
    /// treatment fork is applied directly to both software clones.
    pub assignment_timestamp_ns: Option<u64>,
    pub application_timestamp_ns: u64,
    pub repeats: Vec<H1PairedNoninterferenceRepeat>,
}

/// Complete typed input to [`validate_h1_preflight`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PreflightInput {
    pub declaration: H1PreflightDeclaration,
    pub cases: Vec<H1PreflightCase>,
}

/// Stable machine-readable reasons why a preflight did not pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1PreflightReasonCode {
    SchemaVersionMismatch,
    InvalidDeclaration,
    InvalidIdentifier,
    DuplicateIdentifier,
    InvalidHash,
    InvalidArtifactUri,
    ResourceLimitExceeded,
    ClockDomainMismatch,
    SplitManifestMismatch,
    NonFiniteValue,
    OutputDimensionMismatch,
    FoldLeakage,
    ModeratorLineageViolation,
    ModeratorSnapshotMismatch,
    TimestampOrderViolation,
    MissingAssignmentTimestamp,
    InsufficientRepeats,
    EvaluationOrderCoverageMissing,
    EvaluationOrderImbalance,
    EvaluationOrderViolation,
    BlindingViolation,
    OutputDistanceMismatch,
    OutputDistanceExceeded,
    LatencyDeltaExceeded,
    LatencySlowdownExceeded,
    ControllerTimingExceeded,
    MemoryStateMismatch,
    CacheStateMismatch,
    OutputMetricContractMismatch,
    DesignManifestMismatch,
    PairedStartingStateMismatch,
    CouplingReceiptMismatch,
}

/// One validation problem. Optional IDs identify the narrowest affected unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PreflightIssue {
    pub code: H1PreflightReasonCode,
    pub case_id: Option<String>,
    pub repeat_id: Option<String>,
    pub field: String,
    pub message: String,
}

/// Explicit denominators for the preflight report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PreflightDenominators {
    pub cases_declared: usize,
    /// Cases whose own local checks passed. Run/declaration failures may still
    /// make the overall report fail.
    pub cases_local_checks_passed: usize,
    pub cases_local_checks_failed: usize,
    pub repeats_declared: usize,
    /// Repeats whose own local checks passed. Case/declaration failures do not
    /// retroactively relabel an otherwise valid paired comparison.
    pub repeats_local_checks_passed: usize,
    pub repeats_local_checks_failed: usize,
    pub output_checks_attempted: usize,
    pub latency_checks_attempted: usize,
    pub controller_timing_checks_attempted: usize,
    pub memory_checks_attempted: usize,
    pub cache_checks_attempted: usize,
}

/// Accumulated, fail-closed validation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1PreflightReport {
    pub schema_version: u32,
    pub primary_protocol: H1PrimaryProtocol,
    pub passed: bool,
    pub denominators: H1PreflightDenominators,
    pub issues: Vec<H1PreflightIssue>,
}

impl H1PreflightReport {
    pub fn is_valid(&self) -> bool {
        self.passed
    }
}

/// Validate a typed common-preflight input without performing I/O or mutation.
///
/// Every readable validation problem is accumulated in the returned report. In
/// particular, failures do not short-circuit later denominator or
/// noninterference checks.
pub fn validate_h1_preflight(input: &H1PreflightInput) -> H1PreflightReport {
    validate_h1_preflight_with_limits(input, H1PreflightLimits::default())
}

/// Validate with caller-selected finite limits. The caller must still bound the
/// encoded input before deserialization because this function receives already
/// allocated Rust values.
pub fn validate_h1_preflight_with_limits(
    input: &H1PreflightInput,
    limits: H1PreflightLimits,
) -> H1PreflightReport {
    let declaration = &input.declaration;
    let mut issues = Vec::new();
    validate_limits(limits, &mut issues);
    validate_declaration(declaration, limits, &mut issues);

    if input.cases.is_empty() {
        push_issue(
            &mut issues,
            H1PreflightReasonCode::InvalidDeclaration,
            None,
            None,
            "cases",
            "at least one preflight case is required",
        );
    }
    if input.cases.len() > limits.max_cases {
        push_issue(
            &mut issues,
            H1PreflightReasonCode::ResourceLimitExceeded,
            None,
            None,
            "cases",
            format!(
                "case count {} exceeds limit {}",
                input.cases.len(),
                limits.max_cases
            ),
        );
    }

    validate_split_manifest(declaration, &input.cases, limits, &mut issues);
    validate_design_blinding_order_manifest(declaration, &input.cases, limits, &mut issues);

    let repeats_declared = input.cases.iter().fold(0usize, |total, case| {
        total.saturating_add(case.repeats.len())
    });
    let mut denominators = H1PreflightDenominators {
        cases_declared: input.cases.len(),
        repeats_declared,
        ..H1PreflightDenominators::default()
    };
    let mut case_valid = vec![false; input.cases.len()];
    let mut case_ids: BTreeMap<&str, usize> = BTreeMap::new();
    let mut task_family_folds: FoldMemberships<'_> = BTreeMap::new();
    let mut cluster_folds: FoldMemberships<'_> = BTreeMap::new();
    let mut expected_output_dim = Some(declaration.output_metric_contract.axes.len());
    let mut repeats_processed = 0usize;

    for (case_index, case) in input.cases.iter().take(limits.max_cases).enumerate() {
        case_valid[case_index] = true;
        let issue_count_before = issues.len();
        validate_identifier(
            &case.case_id,
            "case_id",
            Some(&case.case_id),
            None,
            &mut issues,
        );
        validate_identifier(
            &case.task_family_id,
            "task_family_id",
            Some(&case.case_id),
            None,
            &mut issues,
        );
        validate_identifier(
            &case.interference_cluster_id,
            "interference_cluster_id",
            Some(&case.case_id),
            None,
            &mut issues,
        );
        validate_identifier(
            &case.outer_fold,
            "outer_fold",
            Some(&case.case_id),
            None,
            &mut issues,
        );
        validate_identifier(
            &case.clock_domain_id,
            "clock_domain_id",
            Some(&case.case_id),
            None,
            &mut issues,
        );
        if case.clock_domain_id != declaration.clock.domain_id {
            push_issue(
                &mut issues,
                H1PreflightReasonCode::ClockDomainMismatch,
                Some(&case.case_id),
                None,
                "clock_domain_id",
                "case clock domain does not match the frozen declaration",
            );
        }

        if let Some(previous_index) = case_ids.insert(&case.case_id, case_index) {
            case_valid[previous_index] = false;
            push_issue(
                &mut issues,
                H1PreflightReasonCode::DuplicateIdentifier,
                Some(&case.case_id),
                None,
                "case_id",
                "case_id must be unique",
            );
        }
        check_fold_membership(
            &mut task_family_folds,
            &case.task_family_id,
            &case.outer_fold,
            case_index,
            "task_family_id",
            &case.case_id,
            &mut case_valid,
            &mut issues,
        );
        check_fold_membership(
            &mut cluster_folds,
            &case.interference_cluster_id,
            &case.outer_fold,
            case_index,
            "interference_cluster_id",
            &case.case_id,
            &mut case_valid,
            &mut issues,
        );

        validate_timed_artifact(
            &case.baseline_snapshot,
            "baseline_snapshot",
            &case.case_id,
            &mut issues,
        );
        validate_moderator(case, declaration.primary_protocol, &mut issues);

        if case.repeats.len() < declaration.minimum_repeats {
            push_issue(
                &mut issues,
                H1PreflightReasonCode::InsufficientRepeats,
                Some(&case.case_id),
                None,
                "repeats",
                format!(
                    "case has {} repeats but the declaration requires {}",
                    case.repeats.len(),
                    declaration.minimum_repeats
                ),
            );
        }
        if case.repeats.len() > limits.max_repeats_per_case {
            push_issue(
                &mut issues,
                H1PreflightReasonCode::ResourceLimitExceeded,
                Some(&case.case_id),
                None,
                "repeats",
                format!(
                    "case repeat count {} exceeds limit {}",
                    case.repeats.len(),
                    limits.max_repeats_per_case
                ),
            );
        }

        let mut repeat_ids = BTreeSet::new();
        let mut order_counts = BTreeMap::new();
        let remaining_total = limits.max_total_repeats.saturating_sub(repeats_processed);
        let repeats_to_process = case
            .repeats
            .len()
            .min(limits.max_repeats_per_case)
            .min(remaining_total);
        for repeat in case.repeats.iter().take(repeats_to_process) {
            repeats_processed = repeats_processed.saturating_add(1);
            let repeat_issue_count_before = issues.len();
            validate_identifier(
                &repeat.repeat_id,
                "repeat_id",
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                &mut issues,
            );
            validate_identifier(
                &repeat.blinded_fixture_id,
                "blinded_fixture_id",
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                &mut issues,
            );
            if !repeat_ids.insert(&repeat.repeat_id) {
                push_issue(
                    &mut issues,
                    H1PreflightReasonCode::DuplicateIdentifier,
                    Some(&case.case_id),
                    Some(&repeat.repeat_id),
                    "repeat_id",
                    "repeat_id must be unique within a case",
                );
            }
            *order_counts
                .entry(repeat.evaluation_order)
                .or_insert(0usize) += 1;
            if !repeat.blinded {
                push_issue(
                    &mut issues,
                    H1PreflightReasonCode::BlindingViolation,
                    Some(&case.case_id),
                    Some(&repeat.repeat_id),
                    "blinded",
                    "noninterference repeats must be evaluated on blinded fixtures",
                );
            }

            validate_paired_starting_state(case, repeat, &mut issues);
            validate_evaluation_hashes(case, repeat, &mut issues);
            validate_noninterference_repeat(
                case,
                repeat,
                declaration,
                limits,
                &mut expected_output_dim,
                &mut denominators,
                &mut issues,
            );
            if issues.len() == repeat_issue_count_before {
                denominators.repeats_local_checks_passed += 1;
            }
        }

        let uninstrumented_first = order_counts
            .get(&H1EvaluationOrder::UninstrumentedFirst)
            .copied()
            .unwrap_or(0);
        let instrumented_first = order_counts
            .get(&H1EvaluationOrder::InstrumentedFirst)
            .copied()
            .unwrap_or(0);
        if uninstrumented_first == 0 || instrumented_first == 0 {
            push_issue(
                &mut issues,
                H1PreflightReasonCode::EvaluationOrderCoverageMissing,
                Some(&case.case_id),
                None,
                "repeats.evaluation_order",
                "both instrumented-first and uninstrumented-first orders are required",
            );
        } else if uninstrumented_first.abs_diff(instrumented_first) > 1 {
            push_issue(
                &mut issues,
                H1PreflightReasonCode::EvaluationOrderImbalance,
                Some(&case.case_id),
                None,
                "repeats.evaluation_order",
                format!(
                    "evaluation orders are imbalanced: uninstrumented_first={uninstrumented_first}, instrumented_first={instrumented_first}"
                ),
            );
        }
        if issues.len() != issue_count_before {
            case_valid[case_index] = false;
        }
    }

    if repeats_declared > limits.max_total_repeats {
        push_issue(
            &mut issues,
            H1PreflightReasonCode::ResourceLimitExceeded,
            None,
            None,
            "cases.repeats",
            format!(
                "total repeat count {repeats_declared} exceeds limit {}",
                limits.max_total_repeats
            ),
        );
    }
    denominators.repeats_local_checks_failed = denominators
        .repeats_declared
        .saturating_sub(denominators.repeats_local_checks_passed);
    denominators.cases_local_checks_passed = case_valid.iter().filter(|&&valid| valid).count();
    denominators.cases_local_checks_failed = denominators
        .cases_declared
        .saturating_sub(denominators.cases_local_checks_passed);

    H1PreflightReport {
        schema_version: H1_PREFLIGHT_SCHEMA_VERSION,
        primary_protocol: declaration.primary_protocol,
        passed: issues.is_empty(),
        denominators,
        issues,
    }
}

fn validate_limits(limits: H1PreflightLimits, issues: &mut Vec<H1PreflightIssue>) {
    for (field, value) in [
        ("limits.max_cases", limits.max_cases),
        ("limits.max_split_entries", limits.max_split_entries),
        ("limits.max_repeats_per_case", limits.max_repeats_per_case),
        ("limits.max_total_repeats", limits.max_total_repeats),
        ("limits.max_output_dimension", limits.max_output_dimension),
    ] {
        if value == 0 {
            push_issue(
                issues,
                H1PreflightReasonCode::InvalidDeclaration,
                None,
                None,
                field,
                "resource limits must be positive",
            );
        }
    }
}

fn validate_declaration(
    declaration: &H1PreflightDeclaration,
    limits: H1PreflightLimits,
    issues: &mut Vec<H1PreflightIssue>,
) {
    if declaration.schema_version != H1_PREFLIGHT_SCHEMA_VERSION {
        push_issue(
            issues,
            H1PreflightReasonCode::SchemaVersionMismatch,
            None,
            None,
            "declaration.schema_version",
            format!(
                "schema version {} is unsupported; expected {}",
                declaration.schema_version, H1_PREFLIGHT_SCHEMA_VERSION
            ),
        );
    }
    for (field, value) in [
        (
            "declaration.source_run_id",
            declaration.source_run_id.as_str(),
        ),
        (
            "declaration.clock.domain_id",
            declaration.clock.domain_id.as_str(),
        ),
        (
            "declaration.clock.epoch_id",
            declaration.clock.epoch_id.as_str(),
        ),
        (
            "declaration.target_population_id",
            declaration.target_population_id.as_str(),
        ),
        (
            "declaration.baseline_state_boundary",
            declaration.baseline_state_boundary.as_str(),
        ),
        (
            "declaration.application_boundary",
            declaration.application_boundary.as_str(),
        ),
        (
            "declaration.reset_boundary",
            declaration.reset_boundary.as_str(),
        ),
        (
            "declaration.treatment_site",
            declaration.treatment_site.as_str(),
        ),
        (
            "declaration.treatment_version",
            declaration.treatment_version.as_str(),
        ),
        (
            "declaration.treatment_dose_unit",
            declaration.treatment_dose_unit.as_str(),
        ),
    ] {
        validate_identifier(value, field, None, None, issues);
    }
    for (field, artifact) in [
        ("declaration.source_run", &declaration.source_run),
        ("declaration.analysis_plan", &declaration.analysis_plan),
        (
            "declaration.split_manifest.artifact",
            &declaration.split_manifest.artifact,
        ),
        (
            "declaration.target_population_manifest",
            &declaration.target_population_manifest,
        ),
        (
            "declaration.design_blinding_order_manifest.artifact",
            &declaration.design_blinding_order_manifest.artifact,
        ),
        (
            "declaration.output_metric_contract.artifact",
            &declaration.output_metric_contract.artifact,
        ),
        ("declaration.clock.contract", &declaration.clock.contract),
    ] {
        validate_artifact_ref(artifact, field, None, None, issues);
    }
    validate_output_metric_contract(declaration, limits, issues);
    validate_finite(
        declaration.treatment_dose,
        "declaration.treatment_dose",
        None,
        None,
        issues,
    );
    validate_finite_nonnegative(
        declaration.tolerances.output_distance_max,
        "declaration.tolerances.output_distance_max",
        None,
        None,
        issues,
    );
    validate_finite_nonnegative(
        declaration.tolerances.latency_relative_slowdown_max,
        "declaration.tolerances.latency_relative_slowdown_max",
        None,
        None,
        issues,
    );
    if declaration.minimum_repeats < 2 {
        push_issue(
            issues,
            H1PreflightReasonCode::InvalidDeclaration,
            None,
            None,
            "declaration.minimum_repeats",
            "minimum_repeats must be at least two so both evaluation orders are represented",
        );
    }
    if declaration.minimum_repeats > limits.max_repeats_per_case {
        push_issue(
            issues,
            H1PreflightReasonCode::ResourceLimitExceeded,
            None,
            None,
            "declaration.minimum_repeats",
            format!(
                "minimum_repeats {} exceeds per-case limit {}",
                declaration.minimum_repeats, limits.max_repeats_per_case
            ),
        );
    }
}

fn validate_output_metric_contract(
    declaration: &H1PreflightDeclaration,
    limits: H1PreflightLimits,
    issues: &mut Vec<H1PreflightIssue>,
) {
    let contract = &declaration.output_metric_contract;
    if contract.metric != declaration.output_metric {
        push_issue(
            issues,
            H1PreflightReasonCode::OutputMetricContractMismatch,
            None,
            None,
            "declaration.output_metric_contract.metric",
            "output metric contract does not match declaration.output_metric",
        );
    }
    if contract.axes.is_empty() {
        push_issue(
            issues,
            H1PreflightReasonCode::OutputMetricContractMismatch,
            None,
            None,
            "declaration.output_metric_contract.axes",
            "output metric contract must declare at least one axis",
        );
    }
    if contract.axes.len() > limits.max_output_dimension {
        push_issue(
            issues,
            H1PreflightReasonCode::ResourceLimitExceeded,
            None,
            None,
            "declaration.output_metric_contract.axes",
            format!(
                "output axis count {} exceeds limit {}",
                contract.axes.len(),
                limits.max_output_dimension
            ),
        );
    }
    let mut axis_names = BTreeSet::new();
    for axis in contract.axes.iter().take(limits.max_output_dimension) {
        validate_identifier(
            &axis.axis_name,
            "declaration.output_metric_contract.axes.axis_name",
            None,
            None,
            issues,
        );
        validate_identifier(
            &axis.unit,
            "declaration.output_metric_contract.axes.unit",
            None,
            None,
            issues,
        );
        if !axis.scale.is_finite() || axis.scale <= 0.0 {
            push_issue(
                issues,
                H1PreflightReasonCode::OutputMetricContractMismatch,
                None,
                None,
                "declaration.output_metric_contract.axes.scale",
                "every output axis scale must be finite and strictly positive",
            );
        }
        if !axis_names.insert(axis.axis_name.as_str()) {
            push_issue(
                issues,
                H1PreflightReasonCode::DuplicateIdentifier,
                None,
                None,
                "declaration.output_metric_contract.axes.axis_name",
                format!("duplicate output axis name {:?}", axis.axis_name),
            );
        }
    }
}

fn validate_timed_artifact(
    artifact: &H1TimedArtifact,
    field_prefix: &str,
    case_id: &str,
    issues: &mut Vec<H1PreflightIssue>,
) {
    validate_artifact_ref(
        &artifact.artifact,
        &format!("{field_prefix}.artifact"),
        Some(case_id),
        None,
        issues,
    );
}

fn validate_split_manifest(
    declaration: &H1PreflightDeclaration,
    cases: &[H1PreflightCase],
    limits: H1PreflightLimits,
    issues: &mut Vec<H1PreflightIssue>,
) {
    let entries = &declaration.split_manifest.entries;
    if entries.len() > limits.max_split_entries {
        push_issue(
            issues,
            H1PreflightReasonCode::ResourceLimitExceeded,
            None,
            None,
            "declaration.split_manifest.entries",
            format!(
                "split entry count {} exceeds limit {}",
                entries.len(),
                limits.max_split_entries
            ),
        );
    }
    if entries.len() != cases.len() {
        push_issue(
            issues,
            H1PreflightReasonCode::SplitManifestMismatch,
            None,
            None,
            "declaration.split_manifest.entries",
            format!(
                "split manifest has {} entries for {} cases",
                entries.len(),
                cases.len()
            ),
        );
    }

    let mut entry_by_case = BTreeMap::new();
    for entry in entries.iter().take(limits.max_split_entries) {
        for (field, value) in [
            ("split_manifest.case_id", entry.case_id.as_str()),
            (
                "split_manifest.task_family_id",
                entry.task_family_id.as_str(),
            ),
            (
                "split_manifest.interference_cluster_id",
                entry.interference_cluster_id.as_str(),
            ),
            ("split_manifest.outer_fold", entry.outer_fold.as_str()),
        ] {
            validate_identifier(value, field, Some(&entry.case_id), None, issues);
        }
        if entry_by_case
            .insert(entry.case_id.as_str(), entry)
            .is_some()
        {
            push_issue(
                issues,
                H1PreflightReasonCode::DuplicateIdentifier,
                Some(&entry.case_id),
                None,
                "split_manifest.case_id",
                "split manifest case_id must be unique",
            );
        }
    }

    let case_ids: BTreeSet<&str> = cases
        .iter()
        .take(limits.max_cases)
        .map(|case| case.case_id.as_str())
        .collect();
    for case in cases.iter().take(limits.max_cases) {
        match entry_by_case.get(case.case_id.as_str()) {
            Some(entry)
                if entry.task_family_id == case.task_family_id
                    && entry.interference_cluster_id == case.interference_cluster_id
                    && entry.outer_fold == case.outer_fold => {}
            Some(_) => push_issue(
                issues,
                H1PreflightReasonCode::SplitManifestMismatch,
                Some(&case.case_id),
                None,
                "declaration.split_manifest.entries",
                "case grouping/fold does not match its split manifest entry",
            ),
            None => push_issue(
                issues,
                H1PreflightReasonCode::SplitManifestMismatch,
                Some(&case.case_id),
                None,
                "declaration.split_manifest.entries",
                "case is absent from the split manifest",
            ),
        }
    }
    for entry in entries.iter().take(limits.max_split_entries) {
        if !case_ids.contains(entry.case_id.as_str()) {
            push_issue(
                issues,
                H1PreflightReasonCode::SplitManifestMismatch,
                Some(&entry.case_id),
                None,
                "declaration.split_manifest.entries",
                "split manifest contains a case absent from preflight input",
            );
        }
    }
}

fn validate_design_blinding_order_manifest(
    declaration: &H1PreflightDeclaration,
    cases: &[H1PreflightCase],
    limits: H1PreflightLimits,
    issues: &mut Vec<H1PreflightIssue>,
) {
    let entries = &declaration.design_blinding_order_manifest.entries;
    let expected_entry_count = cases.iter().fold(0usize, |total, case| {
        total.saturating_add(case.repeats.len())
    });
    if entries.len() > limits.max_total_repeats {
        push_issue(
            issues,
            H1PreflightReasonCode::ResourceLimitExceeded,
            None,
            None,
            "declaration.design_blinding_order_manifest.entries",
            format!(
                "design entry count {} exceeds limit {}",
                entries.len(),
                limits.max_total_repeats
            ),
        );
    }
    if entries.len() != expected_entry_count {
        push_issue(
            issues,
            H1PreflightReasonCode::DesignManifestMismatch,
            None,
            None,
            "declaration.design_blinding_order_manifest.entries",
            format!(
                "design manifest has {} entries for {expected_entry_count} repeats",
                entries.len()
            ),
        );
    }

    let mut entry_by_repeat = BTreeMap::new();
    for entry in entries.iter().take(limits.max_total_repeats) {
        for (field, value) in [
            ("design_manifest.case_id", entry.case_id.as_str()),
            ("design_manifest.repeat_id", entry.repeat_id.as_str()),
            (
                "design_manifest.blinded_fixture_id",
                entry.blinded_fixture_id.as_str(),
            ),
        ] {
            validate_identifier(
                value,
                field,
                Some(&entry.case_id),
                Some(&entry.repeat_id),
                issues,
            );
        }
        if !entry.blinded {
            push_issue(
                issues,
                H1PreflightReasonCode::BlindingViolation,
                Some(&entry.case_id),
                Some(&entry.repeat_id),
                "design_manifest.blinded",
                "design manifest entries must require blinded evaluation",
            );
        }
        let key = (entry.case_id.as_str(), entry.repeat_id.as_str());
        if entry_by_repeat.insert(key, entry).is_some() {
            push_issue(
                issues,
                H1PreflightReasonCode::DuplicateIdentifier,
                Some(&entry.case_id),
                Some(&entry.repeat_id),
                "design_manifest.case_id+repeat_id",
                "design manifest case/repeat pairs must be unique",
            );
        }
    }

    let mut observed_keys = BTreeSet::new();
    for case in cases.iter().take(limits.max_cases) {
        for repeat in case.repeats.iter().take(limits.max_repeats_per_case) {
            let key = (case.case_id.as_str(), repeat.repeat_id.as_str());
            observed_keys.insert(key);
            match entry_by_repeat.get(&key) {
                Some(entry)
                    if entry.blinded_fixture_id == repeat.blinded_fixture_id
                        && entry.blinded == repeat.blinded
                        && entry.evaluation_order == repeat.evaluation_order => {}
                Some(_) => push_issue(
                    issues,
                    H1PreflightReasonCode::DesignManifestMismatch,
                    Some(&case.case_id),
                    Some(&repeat.repeat_id),
                    "declaration.design_blinding_order_manifest.entries",
                    "repeat blinding fixture/order does not match its design manifest entry",
                ),
                None => push_issue(
                    issues,
                    H1PreflightReasonCode::DesignManifestMismatch,
                    Some(&case.case_id),
                    Some(&repeat.repeat_id),
                    "declaration.design_blinding_order_manifest.entries",
                    "repeat is absent from the design/blinding/order manifest",
                ),
            }
        }
    }
    for entry in entries.iter().take(limits.max_total_repeats) {
        if !observed_keys.contains(&(entry.case_id.as_str(), entry.repeat_id.as_str())) {
            push_issue(
                issues,
                H1PreflightReasonCode::DesignManifestMismatch,
                Some(&entry.case_id),
                Some(&entry.repeat_id),
                "declaration.design_blinding_order_manifest.entries",
                "design manifest contains a repeat absent from preflight input",
            );
        }
    }
}

fn validate_moderator(
    case: &H1PreflightCase,
    protocol: H1PrimaryProtocol,
    issues: &mut Vec<H1PreflightIssue>,
) {
    validate_artifact_ref(
        &case.moderator.artifact,
        "moderator.artifact",
        Some(&case.case_id),
        None,
        issues,
    );
    validate_hash(
        &case.moderator.source_snapshot_sha256,
        "moderator.source_snapshot_sha256",
        Some(&case.case_id),
        None,
        issues,
    );
    if case.moderator.lineage_stage != H1ModeratorLineageStage::UntreatedBaseline {
        push_issue(
            issues,
            H1PreflightReasonCode::ModeratorLineageViolation,
            Some(&case.case_id),
            None,
            "moderator.lineage_stage",
            "primary moderators must originate from the untreated baseline",
        );
    }
    if case.moderator.source_snapshot_sha256 != case.baseline_snapshot.artifact.sha256 {
        push_issue(
            issues,
            H1PreflightReasonCode::ModeratorSnapshotMismatch,
            Some(&case.case_id),
            None,
            "moderator.source_snapshot_sha256",
            "moderator lineage does not name the immutable baseline snapshot",
        );
    }
    if case.baseline_snapshot.captured_timestamp_ns > case.moderator.captured_timestamp_ns {
        push_issue(
            issues,
            H1PreflightReasonCode::TimestampOrderViolation,
            Some(&case.case_id),
            None,
            "moderator.captured_timestamp_ns",
            "moderator was captured before its declared baseline snapshot",
        );
    }
    if case.moderator.captured_timestamp_ns >= case.application_timestamp_ns {
        push_issue(
            issues,
            H1PreflightReasonCode::TimestampOrderViolation,
            Some(&case.case_id),
            None,
            "moderator.captured_timestamp_ns",
            "moderator must be captured strictly before treatment application",
        );
    }
    match case.assignment_timestamp_ns {
        Some(assignment_timestamp_ns) => {
            if case.moderator.captured_timestamp_ns >= assignment_timestamp_ns {
                push_issue(
                    issues,
                    H1PreflightReasonCode::TimestampOrderViolation,
                    Some(&case.case_id),
                    None,
                    "moderator.captured_timestamp_ns",
                    "moderator must be captured strictly before treatment assignment",
                );
            }
            if assignment_timestamp_ns >= case.application_timestamp_ns {
                push_issue(
                    issues,
                    H1PreflightReasonCode::TimestampOrderViolation,
                    Some(&case.case_id),
                    None,
                    "assignment_timestamp_ns",
                    "treatment assignment must occur strictly before application",
                );
            }
        }
        None if protocol == H1PrimaryProtocol::ProtocolB => push_issue(
            issues,
            H1PreflightReasonCode::MissingAssignmentTimestamp,
            Some(&case.case_id),
            None,
            "assignment_timestamp_ns",
            "Protocol B requires a prospective assignment timestamp",
        ),
        None => {}
    }
}

fn validate_evaluation_hashes(
    case: &H1PreflightCase,
    repeat: &H1PairedNoninterferenceRepeat,
    issues: &mut Vec<H1PreflightIssue>,
) {
    for (field, artifact) in [
        (
            "repeats.uninstrumented.memory_state",
            &repeat.uninstrumented.memory_state,
        ),
        (
            "repeats.uninstrumented.cache_state",
            &repeat.uninstrumented.cache_state,
        ),
        (
            "repeats.instrumented.memory_state",
            &repeat.instrumented.memory_state,
        ),
        (
            "repeats.instrumented.cache_state",
            &repeat.instrumented.cache_state,
        ),
    ] {
        validate_artifact_ref(
            artifact,
            field,
            Some(&case.case_id),
            Some(&repeat.repeat_id),
            issues,
        );
    }
}

fn validate_paired_starting_state(
    case: &H1PreflightCase,
    repeat: &H1PairedNoninterferenceRepeat,
    issues: &mut Vec<H1PreflightIssue>,
) {
    let paired = &repeat.paired_starting_state;
    for (field, artifact) in [
        ("repeats.paired_starting_state.artifact", &paired.artifact),
        (
            "repeats.paired_starting_state.reset_receipt",
            &paired.reset_receipt,
        ),
        (
            "repeats.paired_starting_state.rng_coupling_receipt",
            &paired.rng_coupling_receipt,
        ),
        (
            "repeats.paired_starting_state.input_coupling_receipt",
            &paired.input_coupling_receipt,
        ),
    ] {
        validate_artifact_ref(
            artifact,
            field,
            Some(&case.case_id),
            Some(&repeat.repeat_id),
            issues,
        );
    }
    validate_hash(
        &paired.source_baseline_snapshot_sha256,
        "repeats.paired_starting_state.source_baseline_snapshot_sha256",
        Some(&case.case_id),
        Some(&repeat.repeat_id),
        issues,
    );
    if paired.source_baseline_snapshot_sha256 != case.baseline_snapshot.artifact.sha256 {
        push_issue(
            issues,
            H1PreflightReasonCode::PairedStartingStateMismatch,
            Some(&case.case_id),
            Some(&repeat.repeat_id),
            "repeats.paired_starting_state.source_baseline_snapshot_sha256",
            "paired starting state is not derived from the case baseline snapshot",
        );
    }

    for (side, evaluation) in [
        ("uninstrumented", &repeat.uninstrumented),
        ("instrumented", &repeat.instrumented),
    ] {
        let receipts = &evaluation.paired_start_receipts;
        for (field, hash) in [
            ("starting_state_sha256", &receipts.starting_state_sha256),
            ("reset_receipt_sha256", &receipts.reset_receipt_sha256),
            (
                "rng_coupling_receipt_sha256",
                &receipts.rng_coupling_receipt_sha256,
            ),
            (
                "input_coupling_receipt_sha256",
                &receipts.input_coupling_receipt_sha256,
            ),
        ] {
            validate_hash(
                hash,
                &format!("repeats.{side}.paired_start_receipts.{field}"),
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                issues,
            );
        }
        if receipts.starting_state_sha256 != paired.artifact.sha256 {
            push_issue(
                issues,
                H1PreflightReasonCode::PairedStartingStateMismatch,
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                format!("repeats.{side}.paired_start_receipts.starting_state_sha256"),
                "evaluation does not declare the repeat's paired starting-state hash",
            );
        }
        for (field, declared, expected) in [
            (
                "reset_receipt_sha256",
                &receipts.reset_receipt_sha256,
                &paired.reset_receipt.sha256,
            ),
            (
                "rng_coupling_receipt_sha256",
                &receipts.rng_coupling_receipt_sha256,
                &paired.rng_coupling_receipt.sha256,
            ),
            (
                "input_coupling_receipt_sha256",
                &receipts.input_coupling_receipt_sha256,
                &paired.input_coupling_receipt.sha256,
            ),
        ] {
            if declared != expected {
                push_issue(
                    issues,
                    H1PreflightReasonCode::CouplingReceiptMismatch,
                    Some(&case.case_id),
                    Some(&repeat.repeat_id),
                    format!("repeats.{side}.paired_start_receipts.{field}"),
                    "evaluation does not declare the repeat's verified coupling receipt hash",
                );
            }
        }
    }
}

fn validate_noninterference_repeat(
    case: &H1PreflightCase,
    repeat: &H1PairedNoninterferenceRepeat,
    declaration: &H1PreflightDeclaration,
    limits: H1PreflightLimits,
    expected_output_dim: &mut Option<usize>,
    denominators: &mut H1PreflightDenominators,
    issues: &mut Vec<H1PreflightIssue>,
) {
    let case_id = Some(case.case_id.as_str());
    let repeat_id = Some(repeat.repeat_id.as_str());
    validate_evaluation_clock_and_timing(case, repeat, declaration, issues);
    validate_evaluation_order(case, repeat, issues);
    validate_output_vector(
        &repeat.uninstrumented.output,
        "repeats.uninstrumented.output",
        case_id,
        repeat_id,
        limits.max_output_dimension,
        expected_output_dim,
        issues,
    );
    validate_output_vector(
        &repeat.instrumented.output,
        "repeats.instrumented.output",
        case_id,
        repeat_id,
        limits.max_output_dimension,
        expected_output_dim,
        issues,
    );
    validate_finite_nonnegative(
        repeat.output_distance,
        "repeats.output_distance",
        case_id,
        repeat_id,
        issues,
    );

    if repeat.uninstrumented.output.len() != repeat.instrumented.output.len() {
        push_issue(
            issues,
            H1PreflightReasonCode::OutputDimensionMismatch,
            case_id,
            repeat_id,
            "repeats.output",
            "paired output vectors have different dimensions",
        );
    } else if repeat.uninstrumented.output.len() <= limits.max_output_dimension
        && repeat.instrumented.output.len() <= limits.max_output_dimension
        && repeat
            .uninstrumented
            .output
            .iter()
            .chain(&repeat.instrumented.output)
            .all(|value| value.is_finite())
    {
        match recompute_output_distance(
            &declaration.output_metric_contract,
            &repeat.uninstrumented.output,
            &repeat.instrumented.output,
        ) {
            Some(distance) if distance.is_finite() => {
                denominators.output_checks_attempted += 1;
                if !approximately_equal(distance, repeat.output_distance) {
                    push_issue(
                        issues,
                        H1PreflightReasonCode::OutputDistanceMismatch,
                        case_id,
                        repeat_id,
                        "repeats.output_distance",
                        format!(
                            "reported output distance {} does not match recomputed distance {distance}",
                            repeat.output_distance
                        ),
                    );
                }
                if distance > declaration.tolerances.output_distance_max {
                    push_issue(
                        issues,
                        H1PreflightReasonCode::OutputDistanceExceeded,
                        case_id,
                        repeat_id,
                        "repeats.output_distance",
                        format!(
                            "recomputed output distance {distance} exceeds tolerance {}",
                            declaration.tolerances.output_distance_max
                        ),
                    );
                }
            }
            _ => push_issue(
                issues,
                H1PreflightReasonCode::NonFiniteValue,
                case_id,
                repeat_id,
                "repeats.output_distance",
                "recomputed output distance is non-finite",
            ),
        }
    }

    denominators.latency_checks_attempted += 1;
    let latency_delta = repeat
        .instrumented
        .latency_ns
        .abs_diff(repeat.uninstrumented.latency_ns);
    if latency_delta > declaration.tolerances.latency_absolute_delta_ns_max {
        push_issue(
            issues,
            H1PreflightReasonCode::LatencyDeltaExceeded,
            case_id,
            repeat_id,
            "repeats.latency_ns",
            format!(
                "paired latency delta {latency_delta}ns exceeds tolerance {}ns",
                declaration.tolerances.latency_absolute_delta_ns_max
            ),
        );
    }
    let slowdown = relative_slowdown(
        repeat.uninstrumented.latency_ns,
        repeat.instrumented.latency_ns,
    );
    if slowdown > declaration.tolerances.latency_relative_slowdown_max {
        push_issue(
            issues,
            H1PreflightReasonCode::LatencySlowdownExceeded,
            case_id,
            repeat_id,
            "repeats.latency_ns",
            format!(
                "instrumentation relative slowdown {slowdown} exceeds tolerance {}",
                declaration.tolerances.latency_relative_slowdown_max
            ),
        );
    }

    denominators.controller_timing_checks_attempted += 1;
    if let (Some(uninstrumented_offset), Some(instrumented_offset)) = (
        repeat
            .uninstrumented
            .controller_timestamp_ns
            .checked_sub(repeat.uninstrumented.evaluation_start_timestamp_ns),
        repeat
            .instrumented
            .controller_timestamp_ns
            .checked_sub(repeat.instrumented.evaluation_start_timestamp_ns),
    ) {
        let controller_delta = instrumented_offset.abs_diff(uninstrumented_offset);
        if controller_delta > declaration.tolerances.controller_timing_delta_ns_max {
            push_issue(
                issues,
                H1PreflightReasonCode::ControllerTimingExceeded,
                case_id,
                repeat_id,
                "repeats.controller_timestamp_ns",
                format!(
                    "controller offset delta {controller_delta}ns exceeds tolerance {}ns",
                    declaration.tolerances.controller_timing_delta_ns_max
                ),
            );
        }
    }

    denominators.memory_checks_attempted += 1;
    if repeat.instrumented.memory_state.sha256 != repeat.uninstrumented.memory_state.sha256 {
        push_issue(
            issues,
            H1PreflightReasonCode::MemoryStateMismatch,
            case_id,
            repeat_id,
            "repeats.memory_state.sha256",
            "instrumentation changed the policy memory state hash",
        );
    }
    denominators.cache_checks_attempted += 1;
    if repeat.instrumented.cache_state.sha256 != repeat.uninstrumented.cache_state.sha256 {
        push_issue(
            issues,
            H1PreflightReasonCode::CacheStateMismatch,
            case_id,
            repeat_id,
            "repeats.cache_state.sha256",
            "instrumentation changed the policy cache state hash",
        );
    }
}

fn validate_evaluation_clock_and_timing(
    case: &H1PreflightCase,
    repeat: &H1PairedNoninterferenceRepeat,
    declaration: &H1PreflightDeclaration,
    issues: &mut Vec<H1PreflightIssue>,
) {
    for (side, evaluation) in [
        ("uninstrumented", &repeat.uninstrumented),
        ("instrumented", &repeat.instrumented),
    ] {
        validate_identifier(
            &evaluation.clock_domain_id,
            &format!("repeats.{side}.clock_domain_id"),
            Some(&case.case_id),
            Some(&repeat.repeat_id),
            issues,
        );
        if evaluation.clock_domain_id != declaration.clock.domain_id
            || evaluation.clock_domain_id != case.clock_domain_id
        {
            push_issue(
                issues,
                H1PreflightReasonCode::ClockDomainMismatch,
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                format!("repeats.{side}.clock_domain_id"),
                "evaluation clock domain does not match the case and declaration",
            );
        }
        if evaluation.evaluation_start_timestamp_ns < case.moderator.captured_timestamp_ns {
            push_issue(
                issues,
                H1PreflightReasonCode::TimestampOrderViolation,
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                format!("repeats.{side}.evaluation_start_timestamp_ns"),
                "baseline evaluation cannot start before moderator capture",
            );
        }
        if evaluation.controller_timestamp_ns < evaluation.evaluation_start_timestamp_ns {
            push_issue(
                issues,
                H1PreflightReasonCode::TimestampOrderViolation,
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                format!("repeats.{side}.controller_timestamp_ns"),
                "controller event cannot precede its evaluation start",
            );
        }
        let Some(completion_timestamp_ns) = evaluation
            .evaluation_start_timestamp_ns
            .checked_add(evaluation.latency_ns)
        else {
            push_issue(
                issues,
                H1PreflightReasonCode::TimestampOrderViolation,
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                format!("repeats.{side}.latency_ns"),
                "evaluation completion timestamp overflows u64",
            );
            continue;
        };
        if evaluation.controller_timestamp_ns > completion_timestamp_ns {
            push_issue(
                issues,
                H1PreflightReasonCode::TimestampOrderViolation,
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                format!("repeats.{side}.controller_timestamp_ns"),
                "controller event cannot occur after evaluation completion",
            );
        }
        if completion_timestamp_ns >= case.application_timestamp_ns {
            push_issue(
                issues,
                H1PreflightReasonCode::TimestampOrderViolation,
                Some(&case.case_id),
                Some(&repeat.repeat_id),
                format!("repeats.{side}.latency_ns"),
                "baseline evaluation must complete strictly before treatment application",
            );
        }
        if let Some(assignment_timestamp_ns) = case.assignment_timestamp_ns {
            if completion_timestamp_ns >= assignment_timestamp_ns {
                push_issue(
                    issues,
                    H1PreflightReasonCode::TimestampOrderViolation,
                    Some(&case.case_id),
                    Some(&repeat.repeat_id),
                    format!("repeats.{side}.latency_ns"),
                    "baseline evaluation must complete strictly before treatment assignment",
                );
            }
        }
    }
}

fn validate_evaluation_order(
    case: &H1PreflightCase,
    repeat: &H1PairedNoninterferenceRepeat,
    issues: &mut Vec<H1PreflightIssue>,
) {
    let (first_name, first, second_name, second) = match repeat.evaluation_order {
        H1EvaluationOrder::UninstrumentedFirst => (
            "uninstrumented",
            &repeat.uninstrumented,
            "instrumented",
            &repeat.instrumented,
        ),
        H1EvaluationOrder::InstrumentedFirst => (
            "instrumented",
            &repeat.instrumented,
            "uninstrumented",
            &repeat.uninstrumented,
        ),
    };
    let first_completion = first
        .evaluation_start_timestamp_ns
        .checked_add(first.latency_ns);
    if first.evaluation_start_timestamp_ns >= second.evaluation_start_timestamp_ns
        || first_completion
            .is_none_or(|completion| completion > second.evaluation_start_timestamp_ns)
    {
        push_issue(
            issues,
            H1PreflightReasonCode::EvaluationOrderViolation,
            Some(&case.case_id),
            Some(&repeat.repeat_id),
            "repeats.evaluation_order",
            format!(
                "declared {first_name}-first order is inconsistent with non-overlapping {first_name}/{second_name} timestamps"
            ),
        );
    }
}

type FoldMemberships<'a> = BTreeMap<&'a str, (&'a str, Vec<usize>, bool)>;

#[allow(clippy::too_many_arguments)]
fn check_fold_membership<'a>(
    memberships: &mut FoldMemberships<'a>,
    group_id: &'a str,
    outer_fold: &'a str,
    case_index: usize,
    field: &str,
    case_id: &str,
    case_valid: &mut [bool],
    issues: &mut Vec<H1PreflightIssue>,
) {
    if let Some((first_fold, case_indices, already_leaked)) = memberships.get_mut(group_id) {
        case_indices.push(case_index);
        if *first_fold != outer_fold || *already_leaked {
            *already_leaked = true;
            for &member_index in case_indices.iter() {
                case_valid[member_index] = false;
            }
            push_issue(
                issues,
                H1PreflightReasonCode::FoldLeakage,
                Some(case_id),
                None,
                field,
                format!(
                    "group {group_id:?} appears across outer folds (first {first_fold:?}, current {outer_fold:?})"
                ),
            );
        }
    } else {
        memberships.insert(group_id, (outer_fold, vec![case_index], false));
    }
}

fn validate_output_vector(
    output: &[f64],
    field: &str,
    case_id: Option<&str>,
    repeat_id: Option<&str>,
    max_output_dimension: usize,
    expected_output_dim: &mut Option<usize>,
    issues: &mut Vec<H1PreflightIssue>,
) {
    if output.len() > max_output_dimension {
        push_issue(
            issues,
            H1PreflightReasonCode::ResourceLimitExceeded,
            case_id,
            repeat_id,
            field,
            format!(
                "output dimension {} exceeds limit {max_output_dimension}",
                output.len()
            ),
        );
    }
    if output.is_empty() {
        push_issue(
            issues,
            H1PreflightReasonCode::OutputDimensionMismatch,
            case_id,
            repeat_id,
            field,
            "policy output vector must not be empty",
        );
    } else if let Some(expected) = *expected_output_dim {
        if output.len() != expected {
            push_issue(
                issues,
                H1PreflightReasonCode::OutputDimensionMismatch,
                case_id,
                repeat_id,
                field,
                format!(
                    "output dimension {} differs from frozen dimension {expected}",
                    output.len()
                ),
            );
        }
    } else {
        *expected_output_dim = Some(output.len());
    }
    for (index, value) in output.iter().take(max_output_dimension).enumerate() {
        if !value.is_finite() {
            push_issue(
                issues,
                H1PreflightReasonCode::NonFiniteValue,
                case_id,
                repeat_id,
                field,
                format!("output element {index} is non-finite"),
            );
        }
    }
}

fn recompute_output_distance(
    contract: &H1OutputMetricContract,
    left: &[f64],
    right: &[f64],
) -> Option<f64> {
    if left.len() != right.len()
        || left.is_empty()
        || left.len() != contract.axes.len()
        || contract
            .axes
            .iter()
            .any(|axis| !axis.scale.is_finite() || axis.scale <= 0.0)
    {
        return None;
    }
    let scaled_deltas = left
        .iter()
        .zip(right)
        .zip(&contract.axes)
        .map(|((lhs, rhs), axis)| (lhs - rhs) / axis.scale);
    match contract.metric {
        H1OutputMetric::L2 => Some(scaled_deltas.map(|delta| delta * delta).sum::<f64>().sqrt()),
        H1OutputMetric::LInf => scaled_deltas.map(f64::abs).reduce(f64::max),
    }
}

fn approximately_equal(left: f64, right: f64) -> bool {
    let scale = 1.0 + left.abs().max(right.abs());
    (left - right).abs() <= 64.0 * f64::EPSILON * scale
}

fn relative_slowdown(uninstrumented_ns: u64, instrumented_ns: u64) -> f64 {
    if instrumented_ns <= uninstrumented_ns {
        0.0
    } else if uninstrumented_ns == 0 {
        f64::INFINITY
    } else {
        (instrumented_ns - uninstrumented_ns) as f64 / uninstrumented_ns as f64
    }
}

fn validate_identifier(
    value: &str,
    field: &str,
    case_id: Option<&str>,
    repeat_id: Option<&str>,
    issues: &mut Vec<H1PreflightIssue>,
) {
    let valid = !value.is_empty()
        && value.len() <= H1_MAX_IDENTIFIER_BYTES
        && value == value.trim()
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'.' | b'_' | b'-' | b':' | b'/' | b'@' | b'+' | b'%' | b'*' | b'^'
                )
        });
    if !valid {
        push_issue(
            issues,
            H1PreflightReasonCode::InvalidIdentifier,
            case_id,
            repeat_id,
            field,
            format!(
                "identifier must be 1..={H1_MAX_IDENTIFIER_BYTES} canonical ASCII bytes, begin alphanumeric, and contain only [A-Za-z0-9._-:/@+%*^]"
            ),
        );
    }
}

fn validate_artifact_ref(
    artifact: &H1ArtifactRef,
    field_prefix: &str,
    case_id: Option<&str>,
    repeat_id: Option<&str>,
    issues: &mut Vec<H1PreflightIssue>,
) {
    validate_artifact_uri(
        &artifact.artifact_uri,
        &format!("{field_prefix}.artifact_uri"),
        case_id,
        repeat_id,
        issues,
    );
    validate_hash(
        &artifact.sha256,
        &format!("{field_prefix}.sha256"),
        case_id,
        repeat_id,
        issues,
    );
}

fn validate_artifact_uri(
    value: &str,
    field: &str,
    case_id: Option<&str>,
    repeat_id: Option<&str>,
    issues: &mut Vec<H1PreflightIssue>,
) {
    let unsafe_segment = value
        .split(['/', '\\'])
        .any(|segment| segment == "." || segment == "..");
    let valid = !value.is_empty()
        && value.len() <= H1_MAX_ARTIFACT_URI_BYTES
        && value == value.trim()
        && !value.contains('\\')
        && !value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        && !unsafe_segment;
    if !valid {
        push_issue(
            issues,
            H1PreflightReasonCode::InvalidArtifactUri,
            case_id,
            repeat_id,
            field,
            format!(
                "artifact URI must be nonempty, <= {H1_MAX_ARTIFACT_URI_BYTES} bytes, canonical, whitespace/control-free, and contain no dot or parent segments"
            ),
        );
    }
}

fn validate_hash(
    value: &str,
    field: &str,
    case_id: Option<&str>,
    repeat_id: Option<&str>,
    issues: &mut Vec<H1PreflightIssue>,
) {
    if !is_sha256(value) {
        push_issue(
            issues,
            H1PreflightReasonCode::InvalidHash,
            case_id,
            repeat_id,
            field,
            "hash must be exactly 64 lowercase hexadecimal SHA-256 characters",
        );
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_finite_nonnegative(
    value: f64,
    field: &str,
    case_id: Option<&str>,
    repeat_id: Option<&str>,
    issues: &mut Vec<H1PreflightIssue>,
) {
    if !value.is_finite() || value < 0.0 {
        push_issue(
            issues,
            H1PreflightReasonCode::NonFiniteValue,
            case_id,
            repeat_id,
            field,
            "value must be finite and non-negative",
        );
    }
}

fn validate_finite(
    value: f64,
    field: &str,
    case_id: Option<&str>,
    repeat_id: Option<&str>,
    issues: &mut Vec<H1PreflightIssue>,
) {
    if !value.is_finite() {
        push_issue(
            issues,
            H1PreflightReasonCode::NonFiniteValue,
            case_id,
            repeat_id,
            field,
            "value must be finite",
        );
    }
}

fn push_issue(
    issues: &mut Vec<H1PreflightIssue>,
    code: H1PreflightReasonCode,
    case_id: Option<&str>,
    repeat_id: Option<&str>,
    field: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(H1PreflightIssue {
        code,
        case_id: case_id.map(str::to_string),
        repeat_id: repeat_id.map(str::to_string),
        field: field.into(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn artifact(name: &str, character: char) -> H1ArtifactRef {
        H1ArtifactRef {
            artifact_uri: format!("artifacts/{name}.json"),
            sha256: hash(character),
        }
    }

    fn split_entry(id: &str, fold: &str) -> H1SplitManifestEntry {
        H1SplitManifestEntry {
            case_id: id.to_string(),
            task_family_id: format!("family-{id}"),
            interference_cluster_id: format!("cluster-{id}"),
            outer_fold: fold.to_string(),
        }
    }

    fn design_entry(
        case_id: &str,
        repeat_id: &str,
        order: H1EvaluationOrder,
    ) -> H1DesignBlindingOrderEntry {
        H1DesignBlindingOrderEntry {
            case_id: case_id.to_string(),
            repeat_id: repeat_id.to_string(),
            blinded_fixture_id: format!("blind-{repeat_id}"),
            blinded: true,
            evaluation_order: order,
        }
    }

    fn declaration() -> H1PreflightDeclaration {
        H1PreflightDeclaration {
            schema_version: H1_PREFLIGHT_SCHEMA_VERSION,
            primary_protocol: H1PrimaryProtocol::ProtocolB,
            source_run_id: "h1-preflight-test".to_string(),
            source_run: artifact("source-run", 'a'),
            analysis_plan: artifact("analysis-plan", 'b'),
            split_manifest: H1SplitManifest {
                artifact: artifact("split-manifest", 'c'),
                entries: vec![split_entry("case-1", "fold-1")],
            },
            target_population: H1TargetPopulation::FiniteBenchmark,
            target_population_id: "fixture-target-v1".to_string(),
            target_population_manifest: artifact("target-population", 'd'),
            design_blinding_order_manifest: H1DesignBlindingOrderManifest {
                artifact: artifact("design-order", 'e'),
                entries: vec![
                    design_entry("case-1", "r1", H1EvaluationOrder::UninstrumentedFirst),
                    design_entry("case-1", "r2", H1EvaluationOrder::InstrumentedFirst),
                ],
            },
            output_metric_contract: H1OutputMetricContract {
                artifact: artifact("output-metric", 'f'),
                metric: H1OutputMetric::L2,
                axes: vec![
                    H1OutputAxisScale {
                        axis_name: "action_x".to_string(),
                        scale: 1.0,
                        unit: "normalized".to_string(),
                    },
                    H1OutputAxisScale {
                        axis_name: "action_y".to_string(),
                        scale: 1.0,
                        unit: "normalized".to_string(),
                    },
                ],
            },
            clock: H1ClockDomainContract {
                domain_id: "clock-main".to_string(),
                epoch_id: "fixture-epoch-1".to_string(),
                kind: H1ClockDomainKind::Monotonic,
                contract: artifact("clock-contract", '1'),
            },
            baseline_state_boundary: "after_diagnostic_capture".to_string(),
            application_boundary: "before_policy_head".to_string(),
            reset_boundary: "before_each_baseline_evaluation".to_string(),
            treatment_site: "policy.hidden.4".to_string(),
            treatment_version: "mask-v1".to_string(),
            treatment_dose: 0.25,
            treatment_dose_unit: "fraction".to_string(),
            output_metric: H1OutputMetric::L2,
            missing_value_policy: H1MissingValuePolicy::FailRun,
            pid_abstention_policy: H1PidAbstentionPolicy::NotApplicable,
            tolerances: H1NoninterferenceTolerances {
                output_distance_max: 0.02,
                latency_absolute_delta_ns_max: 100,
                latency_relative_slowdown_max: 0.05,
                controller_timing_delta_ns_max: 10,
            },
            minimum_repeats: 2,
        }
    }

    fn evaluation(
        output: Vec<f64>,
        start_timestamp_ns: u64,
        latency_ns: u64,
        controller_offset_ns: u64,
        side: &str,
    ) -> H1PolicyEvaluation {
        H1PolicyEvaluation {
            output,
            clock_domain_id: "clock-main".to_string(),
            evaluation_start_timestamp_ns: start_timestamp_ns,
            latency_ns,
            controller_timestamp_ns: start_timestamp_ns + controller_offset_ns,
            paired_start_receipts: H1PairedStartReceiptHashes {
                starting_state_sha256: hash('6'),
                reset_receipt_sha256: hash('7'),
                rng_coupling_receipt_sha256: hash('8'),
                input_coupling_receipt_sha256: hash('9'),
            },
            memory_state: artifact(&format!("{side}-memory"), '2'),
            cache_state: artifact(&format!("{side}-cache"), '3'),
        }
    }

    fn paired_repeat(id: &str, order: H1EvaluationOrder) -> H1PairedNoninterferenceRepeat {
        let (uninstrumented_start, instrumented_start) = match order {
            H1EvaluationOrder::UninstrumentedFirst => (1_000, 3_000),
            H1EvaluationOrder::InstrumentedFirst => (3_000, 1_000),
        };
        H1PairedNoninterferenceRepeat {
            repeat_id: id.to_string(),
            blinded_fixture_id: format!("blind-{id}"),
            blinded: true,
            evaluation_order: order,
            paired_starting_state: H1PairedStartingState {
                artifact: artifact(&format!("{id}-starting-state"), '6'),
                source_baseline_snapshot_sha256: hash('4'),
                reset_receipt: artifact(&format!("{id}-reset-receipt"), '7'),
                rng_coupling_receipt: artifact(&format!("{id}-rng-receipt"), '8'),
                input_coupling_receipt: artifact(&format!("{id}-input-receipt"), '9'),
            },
            uninstrumented: evaluation(
                vec![1.0, 2.0],
                uninstrumented_start,
                1_000,
                500,
                "uninstrumented",
            ),
            instrumented: evaluation(
                vec![1.003, 2.004],
                instrumented_start,
                1_010,
                503,
                "instrumented",
            ),
            output_distance: 0.005,
        }
    }

    fn case(id: &str, fold: &str) -> H1PreflightCase {
        H1PreflightCase {
            case_id: id.to_string(),
            task_family_id: format!("family-{id}"),
            interference_cluster_id: format!("cluster-{id}"),
            outer_fold: fold.to_string(),
            clock_domain_id: "clock-main".to_string(),
            baseline_snapshot: H1TimedArtifact {
                artifact: artifact(&format!("{id}-snapshot"), '4'),
                captured_timestamp_ns: 100,
            },
            moderator: H1ModeratorArtifact {
                artifact: artifact(&format!("{id}-moderator"), '5'),
                lineage_stage: H1ModeratorLineageStage::UntreatedBaseline,
                source_snapshot_sha256: hash('4'),
                captured_timestamp_ns: 200,
            },
            assignment_timestamp_ns: Some(10_000),
            application_timestamp_ns: 20_000,
            repeats: vec![
                paired_repeat("r1", H1EvaluationOrder::UninstrumentedFirst),
                paired_repeat("r2", H1EvaluationOrder::InstrumentedFirst),
            ],
        }
    }

    fn input() -> H1PreflightInput {
        H1PreflightInput {
            declaration: declaration(),
            cases: vec![case("case-1", "fold-1")],
        }
    }

    fn codes(report: &H1PreflightReport) -> BTreeSet<H1PreflightReasonCode> {
        report.issues.iter().map(|issue| issue.code).collect()
    }

    #[test]
    fn clean_preflight_passes_with_complete_denominators() {
        let report = validate_h1_preflight(&input());
        assert!(report.is_valid(), "{:?}", report.issues);
        assert_eq!(report.denominators.cases_declared, 1);
        assert_eq!(report.denominators.cases_local_checks_passed, 1);
        assert_eq!(report.denominators.repeats_declared, 2);
        assert_eq!(report.denominators.repeats_local_checks_passed, 2);
        assert_eq!(report.denominators.output_checks_attempted, 2);
        assert_eq!(report.denominators.latency_checks_attempted, 2);
        assert_eq!(report.denominators.controller_timing_checks_attempted, 2);
        assert_eq!(report.denominators.memory_checks_attempted, 2);
        assert_eq!(report.denominators.cache_checks_attempted, 2);
    }

    #[test]
    fn serde_rejects_unknown_fields_and_nonexistent_protocols() {
        let mut value = serde_json::to_value(input()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("surprise".to_string(), serde_json::json!(true));
        assert!(serde_json::from_value::<H1PreflightInput>(value).is_err());

        let mut value = serde_json::to_value(input()).unwrap();
        value["declaration"]["primary_protocol"] = serde_json::json!("protocol_c");
        assert!(serde_json::from_value::<H1PreflightInput>(value).is_err());
    }

    #[test]
    fn schema_identifier_hash_and_declaration_failures_accumulate() {
        let mut value = input();
        value.declaration.schema_version = 2;
        value.declaration.source_run_id = "  ".to_string();
        value.declaration.analysis_plan.sha256 = "ABC".to_string();
        value.declaration.treatment_dose = f64::NAN;
        value.declaration.minimum_repeats = 1;
        value.cases[0].case_id.clear();
        let report = validate_h1_preflight(&value);
        let found = codes(&report);
        assert!(found.contains(&H1PreflightReasonCode::SchemaVersionMismatch));
        assert!(found.contains(&H1PreflightReasonCode::InvalidIdentifier));
        assert!(found.contains(&H1PreflightReasonCode::InvalidHash));
        assert!(found.contains(&H1PreflightReasonCode::NonFiniteValue));
        assert!(found.contains(&H1PreflightReasonCode::InvalidDeclaration));
    }

    #[test]
    fn target_population_and_clock_epoch_require_canonical_nonempty_ids() {
        let mut value = input();
        value.declaration.target_population_id.clear();
        value.declaration.clock.epoch_id = " invalid epoch".to_string();
        let report = validate_h1_preflight(&value);
        let invalid_fields = report
            .issues
            .iter()
            .filter(|issue| issue.code == H1PreflightReasonCode::InvalidIdentifier)
            .map(|issue| issue.field.as_str())
            .collect::<BTreeSet<_>>();
        assert!(invalid_fields.contains("declaration.target_population_id"));
        assert!(invalid_fields.contains("declaration.clock.epoch_id"));
        assert!(!report.passed);

        let mut reversed = input();
        reversed.declaration.target_population_id = "invalid target".to_string();
        reversed.declaration.clock.epoch_id.clear();
        let report = validate_h1_preflight(&reversed);
        let invalid_fields = report
            .issues
            .iter()
            .filter(|issue| issue.code == H1PreflightReasonCode::InvalidIdentifier)
            .map(|issue| issue.field.as_str())
            .collect::<BTreeSet<_>>();
        assert!(invalid_fields.contains("declaration.target_population_id"));
        assert!(invalid_fields.contains("declaration.clock.epoch_id"));
        assert!(!report.passed);
    }

    #[test]
    fn nonfinite_outputs_and_dimension_drift_fail_closed() {
        let mut value = input();
        value.cases[0].repeats[0].instrumented.output[0] = f64::INFINITY;
        value.cases[0].repeats[1].instrumented.output.push(3.0);
        let report = validate_h1_preflight(&value);
        let found = codes(&report);
        assert!(found.contains(&H1PreflightReasonCode::NonFiniteValue));
        assert!(found.contains(&H1PreflightReasonCode::OutputDimensionMismatch));
        assert!(!report.passed);
    }

    #[test]
    fn duplicate_ids_and_fold_leakage_fail_closed() {
        let mut value = input();
        let mut second = case("case-2", "fold-2");
        second.task_family_id = value.cases[0].task_family_id.clone();
        second.interference_cluster_id = value.cases[0].interference_cluster_id.clone();
        second.repeats[1].repeat_id = second.repeats[0].repeat_id.clone();
        value.cases.push(second);
        let mut third = case("case-3", "fold-1");
        third.task_family_id = value.cases[0].task_family_id.clone();
        third.interference_cluster_id = value.cases[0].interference_cluster_id.clone();
        value.cases.push(third);
        let report = validate_h1_preflight(&value);
        let found = codes(&report);
        assert!(found.contains(&H1PreflightReasonCode::FoldLeakage));
        assert!(found.contains(&H1PreflightReasonCode::DuplicateIdentifier));
        assert_eq!(report.denominators.cases_local_checks_failed, 3);
    }

    #[test]
    fn post_treatment_or_wrong_snapshot_moderator_fails_closed() {
        let mut value = input();
        value.cases[0].moderator.lineage_stage = H1ModeratorLineageStage::TreatedForwardPass;
        value.cases[0].moderator.source_snapshot_sha256 = hash('2');
        value.cases[0].moderator.captured_timestamp_ns = 10_000;
        let report = validate_h1_preflight(&value);
        let found = codes(&report);
        assert!(found.contains(&H1PreflightReasonCode::ModeratorLineageViolation));
        assert!(found.contains(&H1PreflightReasonCode::ModeratorSnapshotMismatch));
        assert!(found.contains(&H1PreflightReasonCode::TimestampOrderViolation));
    }

    #[test]
    fn protocol_b_requires_assignment_before_application() {
        let mut missing = input();
        missing.cases[0].assignment_timestamp_ns = None;
        let report = validate_h1_preflight(&missing);
        assert!(codes(&report).contains(&H1PreflightReasonCode::MissingAssignmentTimestamp));

        let mut late = input();
        late.cases[0].assignment_timestamp_ns = Some(20_000);
        let report = validate_h1_preflight(&late);
        assert!(codes(&report).contains(&H1PreflightReasonCode::TimestampOrderViolation));
    }

    #[test]
    fn protocol_a_accepts_no_assignment_but_keeps_pre_application_ordering() {
        let mut value = input();
        value.declaration.primary_protocol = H1PrimaryProtocol::ProtocolA;
        value.cases[0].assignment_timestamp_ns = None;
        let report = validate_h1_preflight(&value);
        assert!(report.passed, "{:?}", report.issues);

        value.cases[0].application_timestamp_ns = 4_000;
        let report = validate_h1_preflight(&value);
        assert!(codes(&report).contains(&H1PreflightReasonCode::TimestampOrderViolation));
    }

    #[test]
    fn clocks_actual_order_and_pre_treatment_completion_are_checked() {
        let mut wrong_clock = input();
        wrong_clock.cases[0].repeats[0].instrumented.clock_domain_id = "clock-other".to_string();
        let report = validate_h1_preflight(&wrong_clock);
        assert!(codes(&report).contains(&H1PreflightReasonCode::ClockDomainMismatch));

        let mut overlap = input();
        overlap.cases[0].repeats[0]
            .instrumented
            .evaluation_start_timestamp_ns = 1_500;
        overlap.cases[0].repeats[0]
            .instrumented
            .controller_timestamp_ns = 2_003;
        let report = validate_h1_preflight(&overlap);
        assert!(codes(&report).contains(&H1PreflightReasonCode::EvaluationOrderViolation));

        let mut after_assignment = input();
        after_assignment.cases[0].repeats[0]
            .instrumented
            .evaluation_start_timestamp_ns = 9_500;
        after_assignment.cases[0].repeats[0]
            .instrumented
            .controller_timestamp_ns = 10_003;
        let report = validate_h1_preflight(&after_assignment);
        assert!(codes(&report).contains(&H1PreflightReasonCode::TimestampOrderViolation));
    }

    #[test]
    fn identifiers_and_artifact_uris_are_canonical_and_bounded() {
        let mut value = input();
        value.cases[0].task_family_id = " family-case-1".to_string();
        value.declaration.source_run.artifact_uri = "../source-run.json".to_string();
        value.cases[0].repeats[0]
            .instrumented
            .memory_state
            .artifact_uri = "artifacts/../memory.bin".to_string();
        let report = validate_h1_preflight(&value);
        let found = codes(&report);
        assert!(found.contains(&H1PreflightReasonCode::InvalidIdentifier));
        assert!(found.contains(&H1PreflightReasonCode::InvalidArtifactUri));
    }

    #[test]
    fn split_manifest_must_match_every_case_exactly() {
        let mut value = input();
        value.declaration.split_manifest.entries[0].outer_fold = "fold-2".to_string();
        let report = validate_h1_preflight(&value);
        assert!(codes(&report).contains(&H1PreflightReasonCode::SplitManifestMismatch));
        assert!(!report.passed);
    }

    #[test]
    fn caller_limits_bound_counts_and_output_vectors() {
        let mut value = input();
        value.cases.push(case("case-2", "fold-2"));
        value
            .declaration
            .split_manifest
            .entries
            .push(split_entry("case-2", "fold-2"));
        let limits = H1PreflightLimits {
            max_cases: 1,
            max_split_entries: 1,
            max_repeats_per_case: 1,
            max_total_repeats: 1,
            max_output_dimension: 1,
        };
        let report = validate_h1_preflight_with_limits(&value, limits);
        assert!(codes(&report).contains(&H1PreflightReasonCode::ResourceLimitExceeded));
        assert!(!report.passed);
        assert_eq!(report.denominators.cases_declared, 2);
        assert_eq!(report.denominators.cases_local_checks_failed, 2);
        assert_eq!(report.denominators.repeats_declared, 4);
    }

    #[test]
    fn local_denominators_do_not_claim_a_run_level_pass() {
        let mut bad_declaration = input();
        bad_declaration.declaration.schema_version = 2;
        let report = validate_h1_preflight(&bad_declaration);
        assert!(!report.passed);
        assert_eq!(report.denominators.cases_local_checks_passed, 1);
        assert_eq!(report.denominators.repeats_local_checks_passed, 2);

        let mut bad_case = input();
        bad_case.cases[0].moderator.lineage_stage = H1ModeratorLineageStage::FutureFrame;
        let report = validate_h1_preflight(&bad_case);
        assert!(!report.passed);
        assert_eq!(report.denominators.cases_local_checks_failed, 1);
        assert_eq!(report.denominators.repeats_local_checks_passed, 2);
    }

    #[test]
    fn actual_order_coverage_must_also_be_balanced() {
        let mut value = input();
        value.cases[0]
            .repeats
            .push(paired_repeat("r3", H1EvaluationOrder::UninstrumentedFirst));
        value.cases[0]
            .repeats
            .push(paired_repeat("r4", H1EvaluationOrder::UninstrumentedFirst));
        let report = validate_h1_preflight(&value);
        assert!(codes(&report).contains(&H1PreflightReasonCode::EvaluationOrderImbalance));
    }

    #[test]
    fn duplicate_case_ids_and_bad_state_artifacts_fail_closed() {
        let mut value = input();
        value.cases.push(case("case-1", "fold-1"));
        value
            .declaration
            .split_manifest
            .entries
            .push(split_entry("case-1", "fold-1"));
        value.cases[0].repeats[0].instrumented.cache_state.sha256 = "bad".to_string();
        let report = validate_h1_preflight(&value);
        let found = codes(&report);
        assert!(found.contains(&H1PreflightReasonCode::DuplicateIdentifier));
        assert!(found.contains(&H1PreflightReasonCode::InvalidHash));
        assert_eq!(report.denominators.cases_local_checks_failed, 2);
    }

    #[test]
    fn l_inf_metric_uses_the_frozen_scaled_metric_contract() {
        let mut value = input();
        value.declaration.output_metric = H1OutputMetric::LInf;
        value.declaration.output_metric_contract.metric = H1OutputMetric::LInf;
        value.declaration.output_metric_contract.axes[0].scale = 0.003;
        value.declaration.output_metric_contract.axes[1].scale = 0.008;
        value.declaration.tolerances.output_distance_max = 1.1;
        for repeat in &mut value.cases[0].repeats {
            repeat.output_distance = recompute_output_distance(
                &value.declaration.output_metric_contract,
                &repeat.uninstrumented.output,
                &repeat.instrumented.output,
            )
            .unwrap();
        }
        let report = validate_h1_preflight(&value);
        assert!(report.passed, "{:?}", report.issues);
    }

    #[test]
    fn output_metric_contract_must_match_have_positive_scales_and_match_dimension() {
        let mut metric_mismatch = input();
        metric_mismatch.declaration.output_metric_contract.metric = H1OutputMetric::LInf;
        let report = validate_h1_preflight(&metric_mismatch);
        assert!(codes(&report).contains(&H1PreflightReasonCode::OutputMetricContractMismatch));

        let mut invalid_scale = input();
        invalid_scale.declaration.output_metric_contract.axes[0].scale = 0.0;
        let report = validate_h1_preflight(&invalid_scale);
        assert!(codes(&report).contains(&H1PreflightReasonCode::OutputMetricContractMismatch));

        let mut wrong_dimension = input();
        wrong_dimension
            .declaration
            .output_metric_contract
            .axes
            .pop();
        let report = validate_h1_preflight(&wrong_dimension);
        assert!(codes(&report).contains(&H1PreflightReasonCode::OutputDimensionMismatch));
    }

    #[test]
    fn design_manifest_must_exactly_match_repeat_blinding_and_order() {
        let mut value = input();
        value.declaration.design_blinding_order_manifest.entries[0].evaluation_order =
            H1EvaluationOrder::InstrumentedFirst;
        value.declaration.design_blinding_order_manifest.entries[1].blinded_fixture_id =
            "different-blind".to_string();
        let report = validate_h1_preflight(&value);
        assert!(codes(&report).contains(&H1PreflightReasonCode::DesignManifestMismatch));
    }

    #[test]
    fn both_sides_must_bind_the_same_start_and_coupling_receipts() {
        let mut value = input();
        let repeat = &mut value.cases[0].repeats[0];
        repeat.paired_starting_state.source_baseline_snapshot_sha256 = hash('a');
        repeat
            .instrumented
            .paired_start_receipts
            .starting_state_sha256 = hash('b');
        repeat
            .uninstrumented
            .paired_start_receipts
            .rng_coupling_receipt_sha256 = hash('c');
        let report = validate_h1_preflight(&value);
        let found = codes(&report);
        assert!(found.contains(&H1PreflightReasonCode::PairedStartingStateMismatch));
        assert!(found.contains(&H1PreflightReasonCode::CouplingReceiptMismatch));
    }

    #[test]
    fn insufficient_repeats_and_order_imbalance_fail_closed() {
        let mut value = input();
        value.cases[0].repeats.truncate(1);
        let report = validate_h1_preflight(&value);
        let found = codes(&report);
        assert!(found.contains(&H1PreflightReasonCode::InsufficientRepeats));
        assert!(found.contains(&H1PreflightReasonCode::EvaluationOrderCoverageMissing));

        let mut value = input();
        value.cases[0].repeats[1].evaluation_order = H1EvaluationOrder::UninstrumentedFirst;
        let report = validate_h1_preflight(&value);
        assert!(codes(&report).contains(&H1PreflightReasonCode::EvaluationOrderCoverageMissing));
    }

    #[test]
    fn every_noninterference_axis_fails_with_a_stable_reason() {
        let mut value = input();
        let repeat = &mut value.cases[0].repeats[0];
        repeat.blinded = false;
        repeat.instrumented.output = vec![2.0, 3.0];
        repeat.output_distance = 0.0;
        repeat.instrumented.latency_ns = 2_000;
        repeat.instrumented.controller_timestamp_ns = 20_000;
        repeat.instrumented.memory_state.sha256 = hash('6');
        repeat.instrumented.cache_state.sha256 = hash('7');
        let report = validate_h1_preflight(&value);
        let found = codes(&report);
        for expected in [
            H1PreflightReasonCode::BlindingViolation,
            H1PreflightReasonCode::OutputDistanceMismatch,
            H1PreflightReasonCode::OutputDistanceExceeded,
            H1PreflightReasonCode::LatencyDeltaExceeded,
            H1PreflightReasonCode::LatencySlowdownExceeded,
            H1PreflightReasonCode::ControllerTimingExceeded,
            H1PreflightReasonCode::MemoryStateMismatch,
            H1PreflightReasonCode::CacheStateMismatch,
        ] {
            assert!(
                found.contains(&expected),
                "missing {expected:?}: {:?}",
                report.issues
            );
        }
        assert_eq!(report.denominators.repeats_local_checks_failed, 1);
    }
}
