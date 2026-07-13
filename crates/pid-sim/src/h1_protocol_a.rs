//! Deterministic finite-benchmark reference execution for H1 Protocol A.
//!
//! This module executes a frozen-snapshot **software** contrast. It does not identify a physical
//! individual treatment effect, execute Protocol B, or establish H1 evidence. The reference path
//! deliberately rejects stochastic policies and superpopulation/transport interpretations.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::h1_preflight::{
    scaled_output_delta, scaled_output_distance, H1ArtifactRef, H1ModeratorLineageStage,
    H1OutputMetricContract, H1PrimaryProtocol,
};

pub const H1_PROTOCOL_A_SCHEMA_VERSION: u32 = 1;

const MAX_IDENTIFIER_BYTES: usize = 256;
const MAX_CASES_DEFAULT: usize = 10_000;
const MAX_AUDITS_PER_CASE_DEFAULT: usize = 1_000;
const MAX_FEATURE_DIMENSION_DEFAULT: usize = 4_096;
const MAX_OUTPUT_DIMENSION_DEFAULT: usize = 4_096;
const MAX_NORMAL_MATRIX_CELLS_DEFAULT: usize = 16_384;
const MAX_OUTER_FOLDS_DEFAULT: usize = 32;
const MAX_SCORING_WORK_UNITS_DEFAULT: usize = 100_000_000;
const MAX_TOTAL_AUDITS_DEFAULT: usize = 10_000;
const MAX_EXECUTION_WORK_UNITS_DEFAULT: usize = 100_000_000;
const MAX_RETAINED_RESPONSE_VALUES_DEFAULT: usize = 1_000_000;

/// The only population supported by this reference execution path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1ProtocolAScope {
    DeterministicFiniteBenchmark,
}

/// Protocol-A treatment-version order, distinct from preflight instrumentation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1ProtocolATreatmentOrder {
    ControlFirst,
    TreatedFirst,
}

/// The only interpretation permitted for the checked reference fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1ProtocolAInterpretation {
    SyntheticFrozenSnapshotAlgorithmicResponseOnly,
}

/// Exact binding to a separately passed H1 common-preflight artifact chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAPreflightBinding {
    pub run_id: String,
    pub primary_protocol: H1PrimaryProtocol,
    pub input: H1ArtifactRef,
    pub summary: H1ArtifactRef,
    pub runlog: H1ArtifactRef,
    pub evidence_bundle_hash: String,
}

/// Frozen control and treated versions for the reference internal-state perturbation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolATreatmentPair {
    pub control_version: String,
    pub treated_version: String,
    pub treatment_site: String,
    pub state_axis: usize,
    /// Treated state is `state_axis * (1 - dose)`; valid range is `(0, 1]`.
    pub dose: f64,
    pub dose_unit: String,
}

/// Frozen analysis and execution choices for the deterministic reference study.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAPlan {
    pub scope: H1ProtocolAScope,
    pub target_population_id: String,
    pub policy_id: String,
    pub policy_spec_sha256: String,
    pub instrumentation_id: String,
    pub instrumentation_spec_sha256: String,
    pub execution_context: String,
    pub clock_domain_id: String,
    pub clone_boundary: String,
    pub application_boundary: String,
    pub reset_boundary: String,
    pub treatment: H1ProtocolATreatmentPair,
    pub output_metric_contract: H1OutputMetricContract,
    pub minimum_audits_per_case: usize,
    pub maximum_output_drift: f64,
    pub maximum_response_drift: f64,
    pub ridge_penalty: f64,
    pub minimum_useful_mse_improvement: f64,
    pub permitted_interpretation: H1ProtocolAInterpretation,
}

/// A deterministic linear reference policy whose mutable state is restored for each side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAReferencePolicy {
    /// One row per output axis, one column per policy-input axis.
    pub input_weights: Vec<Vec<f64>>,
    /// One row per output axis, one column per clone-state axis.
    pub state_weights: Vec<Vec<f64>>,
    pub bias: Vec<f64>,
    /// Mutation applied after each evaluation to exercise independent restoration.
    pub post_evaluation_state_update: Vec<f64>,
}

/// One repeatability/order audit for a case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAAuditPlan {
    pub audit_id: String,
    pub treatment_order: H1ProtocolATreatmentOrder,
    /// A recorded execution-context label. The reference binary does not claim subprocess proof.
    pub execution_context: String,
    pub input_stream_id: String,
    /// Deterministic Protocol A requires exactly zero policy RNG draws.
    pub observed_rng_draws: u64,
}

/// One finite-benchmark case and its distinct Protocol-A clone state `W_i`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolACase {
    pub case_id: String,
    pub task_family_id: String,
    pub interference_cluster_id: String,
    pub outer_fold: String,
    pub moderator: Vec<f64>,
    pub design_features: Vec<f64>,
    pub moderator_lineage_stage: H1ModeratorLineageStage,
    pub moderator_captured_timestamp_ns: u64,
    pub clone_captured_timestamp_ns: u64,
    pub treatment_application_timestamp_ns: u64,
    pub source_baseline_snapshot_sha256: String,
    pub source_moderator_sha256: String,
    /// Canonical hash of the exact moderator values used by this runner.
    pub moderator_sha256: String,
    /// Canonical hash of the exact clone state restored for both treatment sides.
    pub clone_state_sha256: String,
    pub clone_state: Vec<f64>,
    pub policy_input: Vec<f64>,
    pub audits: Vec<H1ProtocolAAuditPlan>,
}

/// Complete deterministic reference input. Exact-byte hashing belongs at the CLI boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAInput {
    pub schema_version: u32,
    pub preflight: H1ProtocolAPreflightBinding,
    pub plan: H1ProtocolAPlan,
    pub policy: H1ProtocolAReferencePolicy,
    pub cases: Vec<H1ProtocolACase>,
}

/// Caller-selected finite limits applied after bounded deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct H1ProtocolALimits {
    pub max_cases: usize,
    pub max_audits_per_case: usize,
    pub max_feature_dimension: usize,
    pub max_output_dimension: usize,
    pub max_normal_matrix_cells: usize,
    pub max_outer_folds: usize,
    pub max_scoring_work_units: usize,
    pub max_total_audits: usize,
    pub max_execution_work_units: usize,
    pub max_retained_response_values: usize,
}

impl Default for H1ProtocolALimits {
    fn default() -> Self {
        Self {
            max_cases: MAX_CASES_DEFAULT,
            max_audits_per_case: MAX_AUDITS_PER_CASE_DEFAULT,
            max_feature_dimension: MAX_FEATURE_DIMENSION_DEFAULT,
            max_output_dimension: MAX_OUTPUT_DIMENSION_DEFAULT,
            max_normal_matrix_cells: MAX_NORMAL_MATRIX_CELLS_DEFAULT,
            max_outer_folds: MAX_OUTER_FOLDS_DEFAULT,
            max_scoring_work_units: MAX_SCORING_WORK_UNITS_DEFAULT,
            max_total_audits: MAX_TOTAL_AUDITS_DEFAULT,
            max_execution_work_units: MAX_EXECUTION_WORK_UNITS_DEFAULT,
            max_retained_response_values: MAX_RETAINED_RESPONSE_VALUES_DEFAULT,
        }
    }
}

/// Stable fail-closed reasons for validation, execution, or held-out scoring failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1ProtocolAReasonCode {
    SchemaVersionMismatch,
    InvalidDeclaration,
    InvalidIdentifier,
    InvalidHash,
    ResourceLimitExceeded,
    DuplicateCaseId,
    DuplicateAuditId,
    FoldLeakage,
    InsufficientOuterFolds,
    ModeratorLineageViolation,
    TimestampOrderViolation,
    DimensionMismatch,
    NonFiniteValue,
    RngDrawObserved,
    InsufficientAudits,
    TreatmentOrderImbalance,
    ExecutionFailed,
    DeterministicOutputDrift,
    DeterministicResponseDrift,
    ScoringFailed,
    CalibrationUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAIssue {
    pub code: H1ProtocolAReasonCode,
    pub case_id: Option<String>,
    pub audit_id: Option<String>,
    pub field: String,
    pub message: String,
}

/// Deterministic precision is applicability metadata, not a numeric zero standard error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H1ProtocolAPrecision {
    NotApplicableDeterministic { observed_rng_draws: u64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAAuditReceipt {
    pub audit_id: String,
    pub treatment_order: H1ProtocolATreatmentOrder,
    pub execution_context: String,
    pub input_stream_id: String,
    pub clone_state_sha256: String,
    pub control_pre_state_sha256: String,
    pub treated_pre_state_sha256: String,
    pub policy_input_sha256: String,
    pub control_treatment_receipt_sha256: String,
    pub treated_treatment_receipt_sha256: String,
    pub control_output_sha256: String,
    pub treated_output_sha256: String,
    pub control_post_state_sha256: String,
    pub treated_post_state_sha256: String,
    pub signed_scaled_delta: Vec<f64>,
    pub response: f64,
    pub precision: H1ProtocolAPrecision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAProducedCase {
    pub case_id: String,
    pub outer_fold: String,
    pub response: f64,
    pub maximum_output_drift: f64,
    pub maximum_response_drift: f64,
    pub baseline_prediction: Option<f64>,
    pub diagnostic_prediction: Option<f64>,
    pub receipts: Vec<H1ProtocolAAuditReceipt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H1ProtocolACaseOutcome {
    Produced {
        result: H1ProtocolAProducedCase,
    },
    Abstained {
        case_id: String,
        reason: H1ProtocolAReasonCode,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAFoldScore {
    pub outer_fold: String,
    pub training_cases: usize,
    pub heldout_cases: usize,
    pub baseline_mse: f64,
    pub diagnostic_mse: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H1ProtocolACalibrationAbstentionReason {
    InsufficientCases,
    ZeroPredictionVariance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H1ProtocolACalibration {
    Produced {
        intercept: f64,
        slope: f64,
    },
    Abstained {
        reason: H1ProtocolACalibrationAbstentionReason,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAAggregateScore {
    pub cases_scored: usize,
    pub baseline_mse: f64,
    pub diagnostic_mse: f64,
    pub mse_improvement: f64,
    pub minimum_useful_mse_improvement: f64,
    pub useful_margin_met: bool,
    pub diagnostic_calibration: H1ProtocolACalibration,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolADenominators {
    pub cases_declared: usize,
    pub cases_executed: usize,
    pub cases_abstained: usize,
    pub audits_declared: usize,
    pub treatment_pairs_executed: usize,
    pub outer_folds_scored: usize,
    pub cases_scored: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H1ProtocolAReport {
    pub schema_version: u32,
    pub software_execution_passed: bool,
    pub synthetic_fixture_only: bool,
    pub establishes_h1_evidence: bool,
    pub permitted_interpretation: H1ProtocolAInterpretation,
    pub denominators: H1ProtocolADenominators,
    pub case_outcomes: Vec<H1ProtocolACaseOutcome>,
    pub fold_scores: Vec<H1ProtocolAFoldScore>,
    pub aggregate_score: Option<H1ProtocolAAggregateScore>,
    pub issues: Vec<H1ProtocolAIssue>,
}

impl H1ProtocolAReport {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.software_execution_passed && self.issues.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct TreatmentReceipt<'a> {
    case_id: &'a str,
    audit_id: &'a str,
    version: &'a str,
    site: &'a str,
    state_axis: usize,
    dose: f64,
    dose_unit: &'a str,
    clone_state_sha256: &'a str,
    policy_input_sha256: &'a str,
}

#[derive(Debug, Clone)]
struct ReferencePolicyInstance<'a> {
    spec: &'a H1ProtocolAReferencePolicy,
    state: Vec<f64>,
}

impl ReferencePolicyInstance<'_> {
    fn evaluate(
        &mut self,
        input: &[f64],
        treatment: &H1ProtocolATreatmentPair,
        treated: bool,
    ) -> Option<Vec<f64>> {
        let mut effective_state = self.state.clone();
        if treated {
            let value = effective_state.get_mut(treatment.state_axis)?;
            *value *= 1.0 - treatment.dose;
        }
        let mut output = Vec::with_capacity(self.spec.bias.len());
        for output_axis in 0..self.spec.bias.len() {
            let input_term = self.spec.input_weights[output_axis]
                .iter()
                .zip(input)
                .map(|(weight, value)| weight * value)
                .sum::<f64>();
            let state_term = self.spec.state_weights[output_axis]
                .iter()
                .zip(&effective_state)
                .map(|(weight, value)| weight * value)
                .sum::<f64>();
            output.push(self.spec.bias[output_axis] + input_term + state_term);
        }
        for (state, update) in self
            .state
            .iter_mut()
            .zip(&self.spec.post_evaluation_state_update)
        {
            *state += update;
        }
        Some(output)
    }
}

/// Execute the deterministic reference protocol with default finite limits.
#[must_use]
pub fn run_h1_protocol_a(input: &H1ProtocolAInput) -> H1ProtocolAReport {
    run_h1_protocol_a_with_limits(input, H1ProtocolALimits::default())
}

/// Execute the deterministic reference protocol with caller-selected finite limits.
#[must_use]
pub fn run_h1_protocol_a_with_limits(
    input: &H1ProtocolAInput,
    limits: H1ProtocolALimits,
) -> H1ProtocolAReport {
    let mut issues = Vec::new();
    validate_input(input, limits, &mut issues);
    let audits_declared = input
        .cases
        .iter()
        .fold(0usize, |sum, case| sum.saturating_add(case.audits.len()));
    let mut denominators = H1ProtocolADenominators {
        cases_declared: input.cases.len(),
        audits_declared,
        ..H1ProtocolADenominators::default()
    };

    if !issues.is_empty() {
        return H1ProtocolAReport {
            schema_version: H1_PROTOCOL_A_SCHEMA_VERSION,
            software_execution_passed: false,
            synthetic_fixture_only: true,
            establishes_h1_evidence: false,
            permitted_interpretation: input.plan.permitted_interpretation,
            denominators,
            case_outcomes: Vec::new(),
            fold_scores: Vec::new(),
            aggregate_score: None,
            issues,
        };
    }

    let mut produced = Vec::with_capacity(input.cases.len());
    let mut outcomes = Vec::with_capacity(input.cases.len());
    for case in &input.cases {
        match execute_case(input, case) {
            Some(result) => {
                denominators.cases_executed += 1;
                denominators.treatment_pairs_executed += result.receipts.len();
                produced.push(result.clone());
                outcomes.push(H1ProtocolACaseOutcome::Produced { result });
            }
            None => {
                denominators.cases_abstained += 1;
                push_issue(
                    &mut issues,
                    H1ProtocolAReasonCode::ExecutionFailed,
                    Some(&case.case_id),
                    None,
                    "case",
                    "reference policy execution or canonical receipt hashing failed",
                );
                outcomes.push(H1ProtocolACaseOutcome::Abstained {
                    case_id: case.case_id.clone(),
                    reason: H1ProtocolAReasonCode::ExecutionFailed,
                });
            }
        }
    }

    check_repeatability(input, &produced, &mut issues);
    let (fold_scores, aggregate_score) = if issues.is_empty() {
        match score_outer_folds(input, &mut produced) {
            Some(scoring) => {
                denominators.outer_folds_scored = scoring.fold_scores.len();
                denominators.cases_scored = scoring.aggregate.cases_scored;
                for outcome in &mut outcomes {
                    if let H1ProtocolACaseOutcome::Produced { result } = outcome {
                        if let Some(scored) = produced
                            .iter()
                            .find(|produced| produced.case_id == result.case_id)
                        {
                            result.baseline_prediction = scored.baseline_prediction;
                            result.diagnostic_prediction = scored.diagnostic_prediction;
                        }
                    }
                }
                (scoring.fold_scores, Some(scoring.aggregate))
            }
            None => {
                push_issue(
                    &mut issues,
                    H1ProtocolAReasonCode::ScoringFailed,
                    None,
                    None,
                    "scoring",
                    "outer-fold ridge scoring failed or calibration was degenerate",
                );
                (Vec::new(), None)
            }
        }
    } else {
        (Vec::new(), None)
    };

    H1ProtocolAReport {
        schema_version: H1_PROTOCOL_A_SCHEMA_VERSION,
        software_execution_passed: issues.is_empty(),
        synthetic_fixture_only: true,
        establishes_h1_evidence: false,
        permitted_interpretation: input.plan.permitted_interpretation,
        denominators,
        case_outcomes: outcomes,
        fold_scores,
        aggregate_score,
        issues,
    }
}

fn validate_input(
    input: &H1ProtocolAInput,
    limits: H1ProtocolALimits,
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    if input.schema_version != H1_PROTOCOL_A_SCHEMA_VERSION {
        push_issue(
            issues,
            H1ProtocolAReasonCode::SchemaVersionMismatch,
            None,
            None,
            "schema_version",
            "unsupported Protocol-A schema version",
        );
    }
    if input.preflight.primary_protocol != H1PrimaryProtocol::ProtocolA {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InvalidDeclaration,
            None,
            None,
            "preflight.primary_protocol",
            "Protocol A requires a separately passed Protocol-A common preflight",
        );
    }
    for (field, value) in [
        ("preflight.run_id", input.preflight.run_id.as_str()),
        (
            "plan.target_population_id",
            input.plan.target_population_id.as_str(),
        ),
        ("plan.policy_id", input.plan.policy_id.as_str()),
        (
            "plan.instrumentation_id",
            input.plan.instrumentation_id.as_str(),
        ),
        (
            "plan.execution_context",
            input.plan.execution_context.as_str(),
        ),
        ("plan.clock_domain_id", input.plan.clock_domain_id.as_str()),
        ("plan.clone_boundary", input.plan.clone_boundary.as_str()),
        (
            "plan.application_boundary",
            input.plan.application_boundary.as_str(),
        ),
        ("plan.reset_boundary", input.plan.reset_boundary.as_str()),
        (
            "plan.treatment.control_version",
            input.plan.treatment.control_version.as_str(),
        ),
        (
            "plan.treatment.treated_version",
            input.plan.treatment.treated_version.as_str(),
        ),
        (
            "plan.treatment.treatment_site",
            input.plan.treatment.treatment_site.as_str(),
        ),
        (
            "plan.treatment.dose_unit",
            input.plan.treatment.dose_unit.as_str(),
        ),
    ] {
        validate_identifier(value, field, None, None, issues);
    }
    for (field, artifact) in [
        ("preflight.input", &input.preflight.input),
        ("preflight.summary", &input.preflight.summary),
        ("preflight.runlog", &input.preflight.runlog),
    ] {
        validate_artifact(artifact, field, issues);
    }
    validate_hash(
        &input.preflight.evidence_bundle_hash,
        "preflight.evidence_bundle_hash",
        None,
        None,
        issues,
    );
    validate_hash(
        &input.plan.policy_spec_sha256,
        "plan.policy_spec_sha256",
        None,
        None,
        issues,
    );
    validate_hash(
        &input.plan.instrumentation_spec_sha256,
        "plan.instrumentation_spec_sha256",
        None,
        None,
        issues,
    );
    if canonical_hash(&input.policy).as_deref() != Some(input.plan.policy_spec_sha256.as_str()) {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InvalidHash,
            None,
            None,
            "plan.policy_spec_sha256",
            "policy specification hash does not bind the inline reference policy",
        );
    }
    if input.plan.treatment.control_version == input.plan.treatment.treated_version {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InvalidDeclaration,
            None,
            None,
            "plan.treatment",
            "control and treated versions must be distinct",
        );
    }
    if !input.plan.treatment.dose.is_finite()
        || input.plan.treatment.dose <= 0.0
        || input.plan.treatment.dose > 1.0
    {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InvalidDeclaration,
            None,
            None,
            "plan.treatment.dose",
            "reference attenuation dose must be finite and in (0, 1]",
        );
    }
    for (field, value) in [
        ("plan.maximum_output_drift", input.plan.maximum_output_drift),
        (
            "plan.maximum_response_drift",
            input.plan.maximum_response_drift,
        ),
        ("plan.ridge_penalty", input.plan.ridge_penalty),
        (
            "plan.minimum_useful_mse_improvement",
            input.plan.minimum_useful_mse_improvement,
        ),
    ] {
        if !value.is_finite() || value < 0.0 {
            push_issue(
                issues,
                H1ProtocolAReasonCode::InvalidDeclaration,
                None,
                None,
                field,
                "value must be finite and nonnegative",
            );
        }
    }
    if input.plan.ridge_penalty <= 0.0 {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InvalidDeclaration,
            None,
            None,
            "plan.ridge_penalty",
            "ridge penalty must be positive",
        );
    }
    if input.plan.minimum_audits_per_case < 2 {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InvalidDeclaration,
            None,
            None,
            "plan.minimum_audits_per_case",
            "at least two audits are required to cover both treatment orders",
        );
    }
    validate_policy_and_metric(input, limits, issues);
    validate_cases(input, limits, issues);
}

fn validate_policy_and_metric(
    input: &H1ProtocolAInput,
    limits: H1ProtocolALimits,
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    let output_dim = input.policy.bias.len();
    if output_dim == 0
        || output_dim > limits.max_output_dimension
        || input.policy.input_weights.len() != output_dim
        || input.policy.state_weights.len() != output_dim
        || input.plan.output_metric_contract.axes.len() != output_dim
    {
        push_issue(
            issues,
            H1ProtocolAReasonCode::DimensionMismatch,
            None,
            None,
            "policy",
            "policy rows, bias, and metric axes must share a bounded nonzero output dimension",
        );
        return;
    }
    let input_dim = input.policy.input_weights[0].len();
    let state_dim = input.policy.state_weights[0].len();
    let rows_valid = input
        .policy
        .input_weights
        .iter()
        .all(|row| row.len() == input_dim)
        && input
            .policy
            .state_weights
            .iter()
            .all(|row| row.len() == state_dim);
    if !rows_valid
        || state_dim == 0
        || input_dim > limits.max_feature_dimension
        || state_dim > limits.max_feature_dimension
        || input.policy.post_evaluation_state_update.len() != state_dim
        || input.plan.treatment.state_axis >= state_dim
    {
        push_issue(
            issues,
            H1ProtocolAReasonCode::DimensionMismatch,
            None,
            None,
            "policy",
            "policy matrix, state update, and treatment-axis dimensions are inconsistent",
        );
    }
    if input
        .policy
        .input_weights
        .iter()
        .flatten()
        .chain(input.policy.state_weights.iter().flatten())
        .chain(&input.policy.bias)
        .chain(&input.policy.post_evaluation_state_update)
        .any(|value| !value.is_finite())
    {
        push_issue(
            issues,
            H1ProtocolAReasonCode::NonFiniteValue,
            None,
            None,
            "policy",
            "all reference-policy values must be finite",
        );
    }
    if input.plan.output_metric_contract.axes.iter().any(|axis| {
        axis.axis_name.is_empty()
            || axis.unit.is_empty()
            || !axis.scale.is_finite()
            || axis.scale <= 0.0
    }) {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InvalidDeclaration,
            None,
            None,
            "plan.output_metric_contract",
            "every metric axis requires a name, unit, and positive finite scale",
        );
    }
}

fn validate_cases(
    input: &H1ProtocolAInput,
    limits: H1ProtocolALimits,
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    if input.cases.is_empty() || input.cases.len() > limits.max_cases {
        push_issue(
            issues,
            H1ProtocolAReasonCode::ResourceLimitExceeded,
            None,
            None,
            "cases",
            "case count must be nonzero and within the configured limit",
        );
    }
    let input_dim = input.policy.input_weights.first().map_or(0, Vec::len);
    let state_dim = input.policy.state_weights.first().map_or(0, Vec::len);
    let output_dim = input.policy.bias.len();
    let mut case_ids = BTreeSet::new();
    let mut audit_ids = BTreeSet::new();
    let mut family_folds = BTreeMap::<&str, &str>::new();
    let mut cluster_folds = BTreeMap::<&str, &str>::new();
    let mut folds = BTreeSet::new();
    let mut moderator_dim = None;
    let mut design_dim = None;
    let mut total_audits = Some(0_usize);
    for case in input.cases.iter().take(limits.max_cases) {
        total_audits = total_audits.and_then(|total| total.checked_add(case.audits.len()));
        for (field, value) in [
            ("case_id", case.case_id.as_str()),
            ("task_family_id", case.task_family_id.as_str()),
            (
                "interference_cluster_id",
                case.interference_cluster_id.as_str(),
            ),
            ("outer_fold", case.outer_fold.as_str()),
        ] {
            validate_identifier(value, field, Some(&case.case_id), None, issues);
        }
        if !case_ids.insert(case.case_id.as_str()) {
            push_issue(
                issues,
                H1ProtocolAReasonCode::DuplicateCaseId,
                Some(&case.case_id),
                None,
                "case_id",
                "case identifiers must be unique",
            );
        }
        folds.insert(case.outer_fold.as_str());
        check_group_fold(
            &mut family_folds,
            &case.task_family_id,
            &case.outer_fold,
            "task_family_id",
            &case.case_id,
            issues,
        );
        check_group_fold(
            &mut cluster_folds,
            &case.interference_cluster_id,
            &case.outer_fold,
            "interference_cluster_id",
            &case.case_id,
            issues,
        );
        if case.moderator_lineage_stage != H1ModeratorLineageStage::UntreatedBaseline {
            push_issue(
                issues,
                H1ProtocolAReasonCode::ModeratorLineageViolation,
                Some(&case.case_id),
                None,
                "moderator_lineage_stage",
                "primary moderator must come from the untreated baseline",
            );
        }
        if case.moderator_captured_timestamp_ns > case.clone_captured_timestamp_ns
            || case.clone_captured_timestamp_ns >= case.treatment_application_timestamp_ns
        {
            push_issue(
                issues,
                H1ProtocolAReasonCode::TimestampOrderViolation,
                Some(&case.case_id),
                None,
                "clone_captured_timestamp_ns",
                "moderator <= Protocol-A clone < treatment application is required",
            );
        }
        validate_hash(
            &case.source_baseline_snapshot_sha256,
            "source_baseline_snapshot_sha256",
            Some(&case.case_id),
            None,
            issues,
        );
        validate_hash(
            &case.source_moderator_sha256,
            "source_moderator_sha256",
            Some(&case.case_id),
            None,
            issues,
        );
        validate_hash(
            &case.moderator_sha256,
            "moderator_sha256",
            Some(&case.case_id),
            None,
            issues,
        );
        validate_hash(
            &case.clone_state_sha256,
            "clone_state_sha256",
            Some(&case.case_id),
            None,
            issues,
        );
        if canonical_hash(&case.moderator).as_deref() != Some(case.moderator_sha256.as_str()) {
            push_issue(
                issues,
                H1ProtocolAReasonCode::InvalidHash,
                Some(&case.case_id),
                None,
                "moderator_sha256",
                "moderator hash does not bind the exact values used for scoring",
            );
        }
        if canonical_hash(&case.clone_state).as_deref() != Some(case.clone_state_sha256.as_str()) {
            push_issue(
                issues,
                H1ProtocolAReasonCode::InvalidHash,
                Some(&case.case_id),
                None,
                "clone_state_sha256",
                "clone-state hash does not bind the exact state restored for both treatment sides",
            );
        }
        check_dimension(
            &mut moderator_dim,
            case.moderator.len(),
            limits.max_feature_dimension,
            "moderator",
            &case.case_id,
            issues,
        );
        check_dimension_allow_empty(
            &mut design_dim,
            case.design_features.len(),
            limits.max_feature_dimension,
            "design_features",
            &case.case_id,
            issues,
        );
        if case.clone_state.len() != state_dim || case.policy_input.len() != input_dim {
            push_issue(
                issues,
                H1ProtocolAReasonCode::DimensionMismatch,
                Some(&case.case_id),
                None,
                "clone_state",
                "case clone-state and policy-input dimensions must match the reference policy",
            );
        }
        if case
            .moderator
            .iter()
            .chain(&case.design_features)
            .chain(&case.clone_state)
            .chain(&case.policy_input)
            .any(|value| !value.is_finite())
        {
            push_issue(
                issues,
                H1ProtocolAReasonCode::NonFiniteValue,
                Some(&case.case_id),
                None,
                "case",
                "case features, clone state, and policy input must be finite",
            );
        }
        validate_audits(input, case, limits, &mut audit_ids, issues);
    }
    if folds.len() < 2 {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InsufficientOuterFolds,
            None,
            None,
            "outer_fold",
            "at least two outer folds are required for held-out scoring",
        );
    }
    if folds.len() > limits.max_outer_folds {
        push_issue(
            issues,
            H1ProtocolAReasonCode::ResourceLimitExceeded,
            None,
            None,
            "outer_fold",
            "outer-fold count exceeds the configured scoring-work limit",
        );
    }
    if total_audits.is_none_or(|audits| audits > limits.max_total_audits) {
        push_issue(
            issues,
            H1ProtocolAReasonCode::ResourceLimitExceeded,
            None,
            None,
            "audits.total",
            "total audit count exceeds the configured execution limit",
        );
    }
    let policy_terms = input_dim.checked_add(state_dim);
    let total_execution_work = total_audits.zip(policy_terms).and_then(|(audits, terms)| {
        audits
            .checked_mul(2)?
            .checked_mul(output_dim)?
            .checked_mul(terms)
    });
    if total_execution_work.is_none_or(|work| work > limits.max_execution_work_units) {
        push_issue(
            issues,
            H1ProtocolAReasonCode::ResourceLimitExceeded,
            None,
            None,
            "execution.total_work",
            "audit and policy dimensions exceed the configured execution-work budget",
        );
    }
    let retained_response_values = total_audits.and_then(|audits| audits.checked_mul(output_dim));
    if retained_response_values.is_none_or(|values| values > limits.max_retained_response_values) {
        push_issue(
            issues,
            H1ProtocolAReasonCode::ResourceLimitExceeded,
            None,
            None,
            "execution.retained_response_values",
            "audit responses exceed the configured retained-value budget",
        );
    }
    let model_columns = design_dim
        .unwrap_or(0)
        .checked_add(moderator_dim.unwrap_or(0))
        .and_then(|dimension| dimension.checked_add(1));
    let normal_matrix_cells = model_columns.and_then(|columns| columns.checked_mul(columns));
    if normal_matrix_cells.is_none_or(|cells| cells > limits.max_normal_matrix_cells) {
        push_issue(
            issues,
            H1ProtocolAReasonCode::ResourceLimitExceeded,
            None,
            None,
            "scoring.normal_matrix",
            "combined design/moderator ridge matrix exceeds the configured cell budget",
        );
    }
    let total_scoring_work = model_columns
        .zip(normal_matrix_cells)
        .and_then(|(columns, cells)| {
            let accumulation = input.cases.len().checked_mul(cells)?;
            let solve = columns.checked_mul(cells)?;
            accumulation
                .checked_add(solve)?
                .checked_mul(folds.len())?
                .checked_mul(2)
        });
    if total_scoring_work.is_none_or(|work| work > limits.max_scoring_work_units) {
        push_issue(
            issues,
            H1ProtocolAReasonCode::ResourceLimitExceeded,
            None,
            None,
            "scoring.total_work",
            "combined case, fold, accumulation, and solve work exceeds the configured budget",
        );
    }
}

fn validate_audits<'a>(
    input: &H1ProtocolAInput,
    case: &'a H1ProtocolACase,
    limits: H1ProtocolALimits,
    audit_ids: &mut BTreeSet<&'a str>,
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    if case.audits.len() < input.plan.minimum_audits_per_case {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InsufficientAudits,
            Some(&case.case_id),
            None,
            "audits",
            "case has fewer audits than the frozen minimum",
        );
    }
    if case.audits.len() > limits.max_audits_per_case {
        push_issue(
            issues,
            H1ProtocolAReasonCode::ResourceLimitExceeded,
            Some(&case.case_id),
            None,
            "audits",
            "case audit count exceeds the configured limit",
        );
    }
    let mut control_first = 0usize;
    let mut treated_first = 0usize;
    for audit in case.audits.iter().take(limits.max_audits_per_case) {
        for (field, value) in [
            ("audit_id", audit.audit_id.as_str()),
            ("execution_context", audit.execution_context.as_str()),
            ("input_stream_id", audit.input_stream_id.as_str()),
        ] {
            validate_identifier(
                value,
                field,
                Some(&case.case_id),
                Some(&audit.audit_id),
                issues,
            );
        }
        if !audit_ids.insert(audit.audit_id.as_str()) {
            push_issue(
                issues,
                H1ProtocolAReasonCode::DuplicateAuditId,
                Some(&case.case_id),
                Some(&audit.audit_id),
                "audit_id",
                "audit identifiers must be globally unique",
            );
        }
        if audit.observed_rng_draws != 0 {
            push_issue(
                issues,
                H1ProtocolAReasonCode::RngDrawObserved,
                Some(&case.case_id),
                Some(&audit.audit_id),
                "observed_rng_draws",
                "deterministic reference policy must consume zero RNG draws",
            );
        }
        if audit.execution_context != input.plan.execution_context {
            push_issue(
                issues,
                H1ProtocolAReasonCode::InvalidDeclaration,
                Some(&case.case_id),
                Some(&audit.audit_id),
                "execution_context",
                "audit execution context must match the frozen instrumentation contract",
            );
        }
        match audit.treatment_order {
            H1ProtocolATreatmentOrder::ControlFirst => control_first += 1,
            H1ProtocolATreatmentOrder::TreatedFirst => treated_first += 1,
        }
    }
    if control_first == 0 || treated_first == 0 || control_first.abs_diff(treated_first) > 1 {
        push_issue(
            issues,
            H1ProtocolAReasonCode::TreatmentOrderImbalance,
            Some(&case.case_id),
            None,
            "audits.treatment_order",
            "both treatment orders must be present and counterbalanced",
        );
    }
}

fn execute_case(
    input: &H1ProtocolAInput,
    case: &H1ProtocolACase,
) -> Option<H1ProtocolAProducedCase> {
    let clone_state_sha256 = canonical_hash(&case.clone_state)?;
    let policy_input_sha256 = canonical_hash(&case.policy_input)?;
    let mut receipts = Vec::with_capacity(case.audits.len());
    for audit in &case.audits {
        let mut control_policy = ReferencePolicyInstance {
            spec: &input.policy,
            state: case.clone_state.clone(),
        };
        let mut treated_policy = ReferencePolicyInstance {
            spec: &input.policy,
            state: case.clone_state.clone(),
        };
        let (control_output, treated_output) = match audit.treatment_order {
            H1ProtocolATreatmentOrder::ControlFirst => (
                control_policy.evaluate(&case.policy_input, &input.plan.treatment, false)?,
                treated_policy.evaluate(&case.policy_input, &input.plan.treatment, true)?,
            ),
            H1ProtocolATreatmentOrder::TreatedFirst => {
                let treated =
                    treated_policy.evaluate(&case.policy_input, &input.plan.treatment, true)?;
                let control =
                    control_policy.evaluate(&case.policy_input, &input.plan.treatment, false)?;
                (control, treated)
            }
        };
        let signed_scaled_delta = scaled_output_delta(
            &input.plan.output_metric_contract,
            &control_output,
            &treated_output,
        )?;
        let response = scaled_output_distance(
            &input.plan.output_metric_contract,
            &control_output,
            &treated_output,
        )?;
        let control_treatment_receipt_sha256 = canonical_hash(&TreatmentReceipt {
            case_id: &case.case_id,
            audit_id: &audit.audit_id,
            version: &input.plan.treatment.control_version,
            site: &input.plan.treatment.treatment_site,
            state_axis: input.plan.treatment.state_axis,
            dose: 0.0,
            dose_unit: &input.plan.treatment.dose_unit,
            clone_state_sha256: &clone_state_sha256,
            policy_input_sha256: &policy_input_sha256,
        })?;
        let treated_treatment_receipt_sha256 = canonical_hash(&TreatmentReceipt {
            case_id: &case.case_id,
            audit_id: &audit.audit_id,
            version: &input.plan.treatment.treated_version,
            site: &input.plan.treatment.treatment_site,
            state_axis: input.plan.treatment.state_axis,
            dose: input.plan.treatment.dose,
            dose_unit: &input.plan.treatment.dose_unit,
            clone_state_sha256: &clone_state_sha256,
            policy_input_sha256: &policy_input_sha256,
        })?;
        receipts.push(H1ProtocolAAuditReceipt {
            audit_id: audit.audit_id.clone(),
            treatment_order: audit.treatment_order,
            execution_context: audit.execution_context.clone(),
            input_stream_id: audit.input_stream_id.clone(),
            clone_state_sha256: clone_state_sha256.clone(),
            control_pre_state_sha256: clone_state_sha256.clone(),
            treated_pre_state_sha256: clone_state_sha256.clone(),
            policy_input_sha256: policy_input_sha256.clone(),
            control_treatment_receipt_sha256,
            treated_treatment_receipt_sha256,
            control_output_sha256: canonical_hash(&control_output)?,
            treated_output_sha256: canonical_hash(&treated_output)?,
            control_post_state_sha256: canonical_hash(&control_policy.state)?,
            treated_post_state_sha256: canonical_hash(&treated_policy.state)?,
            signed_scaled_delta,
            response,
            precision: H1ProtocolAPrecision::NotApplicableDeterministic {
                observed_rng_draws: audit.observed_rng_draws,
            },
        });
    }
    let response = receipts.first()?.response;
    let maximum_response_drift = receipts
        .iter()
        .map(|receipt| (receipt.response - response).abs())
        .fold(0.0, f64::max);
    let first = receipts.first()?;
    let maximum_output_drift = receipts
        .iter()
        .map(|receipt| {
            receipt
                .signed_scaled_delta
                .iter()
                .zip(&first.signed_scaled_delta)
                .map(|(left, right)| (left - right).abs())
                .fold(0.0, f64::max)
        })
        .fold(0.0, f64::max);
    Some(H1ProtocolAProducedCase {
        case_id: case.case_id.clone(),
        outer_fold: case.outer_fold.clone(),
        response,
        maximum_output_drift,
        maximum_response_drift,
        baseline_prediction: None,
        diagnostic_prediction: None,
        receipts,
    })
}

fn check_repeatability(
    input: &H1ProtocolAInput,
    produced: &[H1ProtocolAProducedCase],
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    for result in produced {
        let exact_hashes = result.receipts.first().is_some_and(|first| {
            result.receipts.iter().all(|receipt| {
                receipt.control_output_sha256 == first.control_output_sha256
                    && receipt.treated_output_sha256 == first.treated_output_sha256
            })
        });
        if !exact_hashes || result.maximum_output_drift > input.plan.maximum_output_drift {
            push_issue(
                issues,
                H1ProtocolAReasonCode::DeterministicOutputDrift,
                Some(&result.case_id),
                None,
                "audits",
                "deterministic outputs changed across repeatability/order audits",
            );
        }
        if result.maximum_response_drift > input.plan.maximum_response_drift {
            push_issue(
                issues,
                H1ProtocolAReasonCode::DeterministicResponseDrift,
                Some(&result.case_id),
                None,
                "audits",
                "Protocol-A response changed across repeatability/order audits",
            );
        }
    }
}

struct ScoringResult {
    fold_scores: Vec<H1ProtocolAFoldScore>,
    aggregate: H1ProtocolAAggregateScore,
}

#[derive(Debug)]
struct RidgeModel {
    means: Vec<f64>,
    scales: Vec<f64>,
    coefficients: Vec<f64>,
}

impl RidgeModel {
    fn predict(&self, features: &[f64]) -> Option<f64> {
        if features.len() != self.means.len() || self.coefficients.len() != features.len() + 1 {
            return None;
        }
        Some(
            self.coefficients[0]
                + features
                    .iter()
                    .zip(&self.means)
                    .zip(&self.scales)
                    .zip(&self.coefficients[1..])
                    .map(|(((value, mean), scale), coefficient)| {
                        coefficient * ((value - mean) / scale)
                    })
                    .sum::<f64>(),
        )
    }
}

fn score_outer_folds(
    input: &H1ProtocolAInput,
    produced: &mut [H1ProtocolAProducedCase],
) -> Option<ScoringResult> {
    let response_by_case = produced
        .iter()
        .map(|result| (result.case_id.clone(), result.response))
        .collect::<BTreeMap<_, _>>();
    let folds = input
        .cases
        .iter()
        .map(|case| case.outer_fold.as_str())
        .collect::<BTreeSet<_>>();
    let mut fold_scores = Vec::with_capacity(folds.len());
    let mut all_baseline_errors = Vec::with_capacity(input.cases.len());
    let mut all_diagnostic_errors = Vec::with_capacity(input.cases.len());
    let mut diagnostic_predictions = Vec::with_capacity(input.cases.len());
    let mut observed_responses = Vec::with_capacity(input.cases.len());
    for fold in folds {
        let train = input
            .cases
            .iter()
            .filter(|case| case.outer_fold != fold)
            .collect::<Vec<_>>();
        let heldout = input
            .cases
            .iter()
            .filter(|case| case.outer_fold == fold)
            .collect::<Vec<_>>();
        if train.is_empty() || heldout.is_empty() {
            return None;
        }
        let train_y = train
            .iter()
            .map(|case| response_by_case.get(case.case_id.as_str()).copied())
            .collect::<Option<Vec<_>>>()?;
        let baseline_x = train
            .iter()
            .map(|case| case.design_features.clone())
            .collect::<Vec<_>>();
        let diagnostic_x = train
            .iter()
            .map(|case| combined_features(case))
            .collect::<Vec<_>>();
        let baseline = fit_ridge(&baseline_x, &train_y, input.plan.ridge_penalty)?;
        let diagnostic = fit_ridge(&diagnostic_x, &train_y, input.plan.ridge_penalty)?;
        let mut fold_baseline_errors = Vec::with_capacity(heldout.len());
        let mut fold_diagnostic_errors = Vec::with_capacity(heldout.len());
        for case in heldout {
            let observed = *response_by_case.get(case.case_id.as_str())?;
            let baseline_prediction = baseline.predict(&case.design_features)?;
            let diagnostic_prediction = diagnostic.predict(&combined_features(case))?;
            if !baseline_prediction.is_finite() || !diagnostic_prediction.is_finite() {
                return None;
            }
            let result = produced
                .iter_mut()
                .find(|result| result.case_id == case.case_id)?;
            result.baseline_prediction = Some(baseline_prediction);
            result.diagnostic_prediction = Some(diagnostic_prediction);
            let baseline_error = (observed - baseline_prediction).powi(2);
            let diagnostic_error = (observed - diagnostic_prediction).powi(2);
            fold_baseline_errors.push(baseline_error);
            fold_diagnostic_errors.push(diagnostic_error);
            all_baseline_errors.push(baseline_error);
            all_diagnostic_errors.push(diagnostic_error);
            diagnostic_predictions.push(diagnostic_prediction);
            observed_responses.push(observed);
        }
        fold_scores.push(H1ProtocolAFoldScore {
            outer_fold: fold.to_string(),
            training_cases: train.len(),
            heldout_cases: fold_baseline_errors.len(),
            baseline_mse: mean(&fold_baseline_errors)?,
            diagnostic_mse: mean(&fold_diagnostic_errors)?,
        });
    }
    let baseline_mse = mean(&all_baseline_errors)?;
    let diagnostic_mse = mean(&all_diagnostic_errors)?;
    let calibration = calibration(&diagnostic_predictions, &observed_responses)?;
    let mse_improvement = baseline_mse - diagnostic_mse;
    Some(ScoringResult {
        fold_scores,
        aggregate: H1ProtocolAAggregateScore {
            cases_scored: all_baseline_errors.len(),
            baseline_mse,
            diagnostic_mse,
            mse_improvement,
            minimum_useful_mse_improvement: input.plan.minimum_useful_mse_improvement,
            useful_margin_met: mse_improvement >= input.plan.minimum_useful_mse_improvement,
            diagnostic_calibration: calibration,
        },
    })
}

fn combined_features(case: &H1ProtocolACase) -> Vec<f64> {
    case.design_features
        .iter()
        .chain(&case.moderator)
        .copied()
        .collect()
}

fn fit_ridge(features: &[Vec<f64>], targets: &[f64], penalty: f64) -> Option<RidgeModel> {
    if features.is_empty()
        || features.len() != targets.len()
        || !penalty.is_finite()
        || penalty <= 0.0
    {
        return None;
    }
    let dimension = features.first()?.len();
    if features.iter().any(|row| row.len() != dimension) {
        return None;
    }
    let mut means = vec![0.0; dimension];
    for row in features {
        for (mean, value) in means.iter_mut().zip(row) {
            *mean += value;
        }
    }
    for mean in &mut means {
        *mean /= features.len() as f64;
    }
    let mut scales = vec![0.0; dimension];
    for row in features {
        for ((scale, value), mean) in scales.iter_mut().zip(row).zip(&means) {
            *scale += (value - mean).powi(2);
        }
    }
    for scale in &mut scales {
        *scale = (*scale / features.len() as f64).sqrt();
        if *scale <= f64::EPSILON {
            *scale = 1.0;
        }
    }
    let columns = dimension + 1;
    let mut normal = vec![vec![0.0; columns]; columns];
    let mut rhs = vec![0.0; columns];
    for (row, target) in features.iter().zip(targets) {
        let standardized = std::iter::once(1.0)
            .chain(
                row.iter()
                    .zip(&means)
                    .zip(&scales)
                    .map(|((value, mean), scale)| (value - mean) / scale),
            )
            .collect::<Vec<_>>();
        for column in 0..columns {
            rhs[column] += standardized[column] * target;
            for other in 0..columns {
                normal[column][other] += standardized[column] * standardized[other];
            }
        }
    }
    for (column, row) in normal.iter_mut().enumerate().skip(1) {
        row[column] += penalty;
    }
    let coefficients = solve_linear_system(normal, rhs)?;
    Some(RidgeModel {
        means,
        scales,
        coefficients,
    })
}

fn solve_linear_system(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let size = rhs.len();
    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return None;
    }
    for pivot in 0..size {
        let pivot_row = (pivot..size).max_by(|left, right| {
            matrix[*left][pivot]
                .abs()
                .total_cmp(&matrix[*right][pivot].abs())
        })?;
        if matrix[pivot_row][pivot].abs() <= 1e-12 {
            return None;
        }
        matrix.swap(pivot, pivot_row);
        rhs.swap(pivot, pivot_row);
        let divisor = matrix[pivot][pivot];
        for value in &mut matrix[pivot][pivot..] {
            *value /= divisor;
        }
        rhs[pivot] /= divisor;
        let pivot_values = matrix[pivot][pivot..].to_vec();
        for row in 0..size {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for (value, pivot_value) in matrix[row][pivot..].iter_mut().zip(&pivot_values) {
                *value -= factor * pivot_value;
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs.iter().all(|value| value.is_finite()).then_some(rhs)
}

fn calibration(predictions: &[f64], observed: &[f64]) -> Option<H1ProtocolACalibration> {
    if predictions.len() != observed.len() {
        return None;
    }
    if predictions.len() < 3 {
        return Some(H1ProtocolACalibration::Abstained {
            reason: H1ProtocolACalibrationAbstentionReason::InsufficientCases,
        });
    }
    let prediction_mean = mean(predictions)?;
    let observed_mean = mean(observed)?;
    let variance = predictions
        .iter()
        .map(|value| (value - prediction_mean).powi(2))
        .sum::<f64>();
    if variance <= 1e-12 {
        return Some(H1ProtocolACalibration::Abstained {
            reason: H1ProtocolACalibrationAbstentionReason::ZeroPredictionVariance,
        });
    }
    let covariance = predictions
        .iter()
        .zip(observed)
        .map(|(prediction, outcome)| (prediction - prediction_mean) * (outcome - observed_mean))
        .sum::<f64>();
    let slope = covariance / variance;
    let intercept = observed_mean - slope * prediction_mean;
    (slope.is_finite() && intercept.is_finite())
        .then_some(H1ProtocolACalibration::Produced { intercept, slope })
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty())
        .then(|| values.iter().sum::<f64>() / values.len() as f64)
        .filter(|value| value.is_finite())
}

fn check_group_fold<'a>(
    groups: &mut BTreeMap<&'a str, &'a str>,
    group: &'a str,
    fold: &'a str,
    field: &str,
    case_id: &str,
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    if groups
        .insert(group, fold)
        .is_some_and(|previous| previous != fold)
    {
        push_issue(
            issues,
            H1ProtocolAReasonCode::FoldLeakage,
            Some(case_id),
            None,
            field,
            "one task family or interference cluster spans multiple outer folds",
        );
    }
}

fn check_dimension(
    expected: &mut Option<usize>,
    actual: usize,
    maximum: usize,
    field: &str,
    case_id: &str,
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    if actual == 0 || actual > maximum || expected.is_some_and(|value| value != actual) {
        push_issue(
            issues,
            H1ProtocolAReasonCode::DimensionMismatch,
            Some(case_id),
            None,
            field,
            "feature dimension must be nonzero, bounded, and identical across cases",
        );
    } else {
        *expected = Some(actual);
    }
}

fn check_dimension_allow_empty(
    expected: &mut Option<usize>,
    actual: usize,
    maximum: usize,
    field: &str,
    case_id: &str,
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    if actual > maximum || expected.is_some_and(|value| value != actual) {
        push_issue(
            issues,
            H1ProtocolAReasonCode::DimensionMismatch,
            Some(case_id),
            None,
            field,
            "design-feature dimension must be bounded and identical across cases",
        );
    } else {
        *expected = Some(actual);
    }
}

fn validate_identifier(
    value: &str,
    field: &str,
    case_id: Option<&str>,
    audit_id: Option<&str>,
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    let valid = !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value == value.trim()
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b'-' | b':' | b'/' | b'@' | b'+')
        });
    if !valid {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InvalidIdentifier,
            case_id,
            audit_id,
            field,
            "identifier is empty, oversized, or non-canonical",
        );
    }
}

fn validate_artifact(artifact: &H1ArtifactRef, field: &str, issues: &mut Vec<H1ProtocolAIssue>) {
    validate_identifier(&artifact.artifact_uri, field, None, None, issues);
    validate_hash(&artifact.sha256, field, None, None, issues);
}

fn validate_hash(
    value: &str,
    field: &str,
    case_id: Option<&str>,
    audit_id: Option<&str>,
    issues: &mut Vec<H1ProtocolAIssue>,
) {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        push_issue(
            issues,
            H1ProtocolAReasonCode::InvalidHash,
            case_id,
            audit_id,
            field,
            "expected a lowercase 64-character SHA-256 digest",
        );
    }
}

fn canonical_hash<T: Serialize>(value: &T) -> Option<String> {
    pid_runlog::canonical_json_hash_v2(value).ok()
}

fn push_issue(
    issues: &mut Vec<H1ProtocolAIssue>,
    code: H1ProtocolAReasonCode,
    case_id: Option<&str>,
    audit_id: Option<&str>,
    field: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(H1ProtocolAIssue {
        code,
        case_id: case_id.map(str::to_string),
        audit_id: audit_id.map(str::to_string),
        field: field.into(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::h1_preflight::{H1OutputAxisScale, H1OutputMetric};

    fn hash(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn artifact(name: &str, character: char) -> H1ArtifactRef {
        H1ArtifactRef {
            artifact_uri: format!("artifacts/{name}.json"),
            sha256: hash(character),
        }
    }

    fn input() -> H1ProtocolAInput {
        let policy = H1ProtocolAReferencePolicy {
            input_weights: vec![vec![0.1]],
            state_weights: vec![vec![2.0, 0.0]],
            bias: vec![0.0],
            post_evaluation_state_update: vec![100.0, -100.0],
        };
        let policy_spec_sha256 = canonical_hash(&policy).expect("hash policy");
        let states = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let cases = states
            .iter()
            .enumerate()
            .map(|(index, state)| {
                let fold = if index % 2 == 0 { "fold-a" } else { "fold-b" };
                let moderator = vec![*state];
                let clone_state = vec![*state, 1.0];
                H1ProtocolACase {
                    case_id: format!("case-{index}"),
                    task_family_id: format!("family-{index}"),
                    interference_cluster_id: format!("cluster-{index}"),
                    outer_fold: fold.to_string(),
                    moderator_sha256: canonical_hash(&moderator).expect("hash moderator"),
                    clone_state_sha256: canonical_hash(&clone_state).expect("hash clone state"),
                    moderator,
                    design_features: vec![1.0],
                    moderator_lineage_stage: H1ModeratorLineageStage::UntreatedBaseline,
                    moderator_captured_timestamp_ns: 10,
                    clone_captured_timestamp_ns: 20,
                    treatment_application_timestamp_ns: 30,
                    source_baseline_snapshot_sha256: hash('a'),
                    source_moderator_sha256: hash('b'),
                    clone_state,
                    policy_input: vec![0.5],
                    audits: vec![
                        H1ProtocolAAuditPlan {
                            audit_id: format!("audit-{index}-control-first"),
                            treatment_order: H1ProtocolATreatmentOrder::ControlFirst,
                            execution_context: "reference-process".to_string(),
                            input_stream_id: format!("stream-{index}"),
                            observed_rng_draws: 0,
                        },
                        H1ProtocolAAuditPlan {
                            audit_id: format!("audit-{index}-treated-first"),
                            treatment_order: H1ProtocolATreatmentOrder::TreatedFirst,
                            execution_context: "reference-process".to_string(),
                            input_stream_id: format!("stream-{index}"),
                            observed_rng_draws: 0,
                        },
                    ],
                }
            })
            .collect();
        H1ProtocolAInput {
            schema_version: H1_PROTOCOL_A_SCHEMA_VERSION,
            preflight: H1ProtocolAPreflightBinding {
                run_id: "h1-preflight-reference".to_string(),
                primary_protocol: H1PrimaryProtocol::ProtocolA,
                input: artifact("preflight-input", '1'),
                summary: artifact("preflight-summary", '2'),
                runlog: artifact("preflight-runlog", '3'),
                evidence_bundle_hash: hash('4'),
            },
            plan: H1ProtocolAPlan {
                scope: H1ProtocolAScope::DeterministicFiniteBenchmark,
                target_population_id: "synthetic-finite-benchmark".to_string(),
                policy_id: "linear-reference-v1".to_string(),
                policy_spec_sha256,
                instrumentation_id: "diagnostic-hook-v1".to_string(),
                instrumentation_spec_sha256: hash('6'),
                execution_context: "reference-process".to_string(),
                clock_domain_id: "fixture-monotonic-clock".to_string(),
                clone_boundary: "after-moderator-before-treatment".to_string(),
                application_boundary: "before-policy-head".to_string(),
                reset_boundary: "immutable-snapshot-restore".to_string(),
                treatment: H1ProtocolATreatmentPair {
                    control_version: "control-v1".to_string(),
                    treated_version: "attenuate-state-v1".to_string(),
                    treatment_site: "reference.state.0".to_string(),
                    state_axis: 0,
                    dose: 0.5,
                    dose_unit: "fraction".to_string(),
                },
                output_metric_contract: H1OutputMetricContract {
                    artifact: artifact("metric", '5'),
                    metric: H1OutputMetric::L2,
                    axes: vec![H1OutputAxisScale {
                        axis_name: "action-0".to_string(),
                        scale: 1.0,
                        unit: "dimensionless".to_string(),
                    }],
                },
                minimum_audits_per_case: 2,
                maximum_output_drift: 0.0,
                maximum_response_drift: 0.0,
                ridge_penalty: 1e-6,
                minimum_useful_mse_improvement: 0.1,
                permitted_interpretation:
                    H1ProtocolAInterpretation::SyntheticFrozenSnapshotAlgorithmicResponseOnly,
            },
            policy,
            cases,
        }
    }

    fn codes(report: &H1ProtocolAReport) -> BTreeSet<H1ProtocolAReasonCode> {
        report.issues.iter().map(|issue| issue.code).collect()
    }

    #[test]
    fn deterministic_finite_benchmark_executes_and_scores_without_h1_claim() {
        let report = run_h1_protocol_a(&input());
        assert!(report.is_valid(), "{:?}", report.issues);
        assert!(!report.establishes_h1_evidence);
        assert!(report.synthetic_fixture_only);
        let aggregate = report.aggregate_score.expect("valid score");
        assert!(aggregate.diagnostic_mse < aggregate.baseline_mse);
        assert!(aggregate.useful_margin_met);
    }

    #[test]
    fn checked_fixture_hashes_bind_policy_moderators_and_clone_states() {
        let fixture = serde_json::from_str::<H1ProtocolAInput>(include_str!(
            "../fixtures/h1_protocol_a_valid.json"
        ))
        .expect("parse checked Protocol-A fixture");
        assert_eq!(
            canonical_hash(&fixture.policy).expect("hash fixture policy"),
            fixture.plan.policy_spec_sha256
        );
        for case in &fixture.cases {
            assert_eq!(
                canonical_hash(&case.moderator).expect("hash fixture moderator"),
                case.moderator_sha256
            );
            assert_eq!(
                canonical_hash(&case.clone_state).expect("hash fixture clone state"),
                case.clone_state_sha256
            );
        }
    }

    #[test]
    fn independent_restore_survives_large_policy_state_mutation_and_order_reversal() {
        let report = run_h1_protocol_a(&input());
        let H1ProtocolACaseOutcome::Produced { result } = &report.case_outcomes[0] else {
            panic!("expected produced case");
        };
        assert_eq!(result.maximum_output_drift, 0.0);
        assert_eq!(result.maximum_response_drift, 0.0);
        assert_eq!(
            result.receipts[0].control_output_sha256,
            result.receipts[1].control_output_sha256
        );
        assert_eq!(
            result.receipts[0].treated_output_sha256,
            result.receipts[1].treated_output_sha256
        );
    }

    #[test]
    fn protocol_b_preflight_and_rng_draws_fail_closed() {
        let mut value = input();
        value.preflight.primary_protocol = H1PrimaryProtocol::ProtocolB;
        value.cases[0].audits[0].observed_rng_draws = 1;
        let result = codes(&run_h1_protocol_a(&value));
        assert!(result.contains(&H1ProtocolAReasonCode::InvalidDeclaration));
        assert!(result.contains(&H1ProtocolAReasonCode::RngDrawObserved));
    }

    #[test]
    fn post_treatment_moderator_and_bad_timestamps_fail_closed() {
        let mut value = input();
        value.cases[0].moderator_lineage_stage = H1ModeratorLineageStage::TreatedForwardPass;
        value.cases[0].clone_captured_timestamp_ns = 5;
        let result = codes(&run_h1_protocol_a(&value));
        assert!(result.contains(&H1ProtocolAReasonCode::ModeratorLineageViolation));
        assert!(result.contains(&H1ProtocolAReasonCode::TimestampOrderViolation));
    }

    #[test]
    fn treatment_orders_must_be_counterbalanced() {
        let mut value = input();
        value.cases[0].audits[1].treatment_order = H1ProtocolATreatmentOrder::ControlFirst;
        assert!(codes(&run_h1_protocol_a(&value))
            .contains(&H1ProtocolAReasonCode::TreatmentOrderImbalance));
    }

    #[test]
    fn duplicate_ids_and_group_fold_leakage_fail_closed() {
        let mut value = input();
        value.cases[1].case_id = value.cases[0].case_id.clone();
        value.cases[2].task_family_id = value.cases[0].task_family_id.clone();
        value.cases[2].outer_fold = "fold-b".to_string();
        let result = codes(&run_h1_protocol_a(&value));
        assert!(result.contains(&H1ProtocolAReasonCode::DuplicateCaseId));
        assert!(result.contains(&H1ProtocolAReasonCode::FoldLeakage));
    }

    #[test]
    fn dimensions_nonfinite_values_and_limits_fail_closed() {
        let mut value = input();
        value.cases[0].clone_state.pop();
        value.cases[1].moderator[0] = f64::NAN;
        let report = run_h1_protocol_a_with_limits(
            &value,
            H1ProtocolALimits {
                max_cases: 2,
                max_normal_matrix_cells: 1,
                max_outer_folds: 1,
                max_scoring_work_units: 1,
                max_total_audits: 1,
                max_execution_work_units: 1,
                max_retained_response_values: 1,
                ..H1ProtocolALimits::default()
            },
        );
        let result = codes(&report);
        assert!(result.contains(&H1ProtocolAReasonCode::DimensionMismatch));
        assert!(result.contains(&H1ProtocolAReasonCode::NonFiniteValue));
        assert!(result.contains(&H1ProtocolAReasonCode::ResourceLimitExceeded));
    }

    #[test]
    fn total_scoring_work_is_bounded_as_a_product() {
        let report = run_h1_protocol_a_with_limits(
            &input(),
            H1ProtocolALimits {
                max_scoring_work_units: 1,
                ..H1ProtocolALimits::default()
            },
        );
        assert!(report.issues.iter().any(|issue| {
            issue.code == H1ProtocolAReasonCode::ResourceLimitExceeded
                && issue.field == "scoring.total_work"
        }));
        assert!(report.aggregate_score.is_none());
    }

    #[test]
    fn total_execution_and_retained_response_work_are_bounded_as_products() {
        let report = run_h1_protocol_a_with_limits(
            &input(),
            H1ProtocolALimits {
                max_total_audits: 1,
                max_execution_work_units: 1,
                max_retained_response_values: 1,
                ..H1ProtocolALimits::default()
            },
        );
        let fields = report
            .issues
            .iter()
            .map(|issue| issue.field.as_str())
            .collect::<BTreeSet<_>>();
        assert!(fields.contains("audits.total"));
        assert!(fields.contains("execution.total_work"));
        assert!(fields.contains("execution.retained_response_values"));
        assert!(report.case_outcomes.is_empty());
    }

    #[test]
    fn homogeneous_response_keeps_proper_scores_and_abstains_only_calibration() {
        let mut value = input();
        for case in &mut value.cases {
            case.moderator[0] = 1.0;
            case.clone_state[0] = 1.0;
            case.moderator_sha256 = canonical_hash(&case.moderator).expect("hash moderator");
            case.clone_state_sha256 = canonical_hash(&case.clone_state).expect("hash clone state");
        }
        let report = run_h1_protocol_a(&value);
        assert!(report.is_valid());
        let aggregate = report.aggregate_score.expect("proper score retained");
        assert!(!aggregate.useful_margin_met);
        assert!(matches!(
            aggregate.diagnostic_calibration,
            H1ProtocolACalibration::Abstained {
                reason: H1ProtocolACalibrationAbstentionReason::ZeroPredictionVariance
            }
        ));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value = serde_json::to_value(input()).expect("serialize fixture");
        value["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<H1ProtocolAInput>(value).is_err());
    }

    #[test]
    fn output_distance_preserves_signed_delta_and_frozen_scale() {
        let mut value = input();
        value.plan.output_metric_contract.axes[0].scale = 2.0;
        let report = run_h1_protocol_a(&value);
        assert!(report.is_valid());
        let H1ProtocolACaseOutcome::Produced { result } = &report.case_outcomes[0] else {
            panic!("expected produced case");
        };
        assert!((result.receipts[0].signed_scaled_delta[0] + 0.5).abs() < 1e-12);
        assert!((result.response - 0.5).abs() < 1e-12);
    }

    #[test]
    fn fixture_scope_does_not_claim_other_target_populations() {
        let value = serde_json::to_value(input()).expect("serialize fixture");
        assert_eq!(
            value["plan"]["scope"],
            serde_json::json!("deterministic_finite_benchmark")
        );
        assert_eq!(
            value["plan"]["permitted_interpretation"],
            serde_json::json!("synthetic_frozen_snapshot_algorithmic_response_only")
        );
    }
}
