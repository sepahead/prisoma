//! Deterministic finite-benchmark reference mechanics for registry H2.
//!
//! This module implements one deliberately narrow estimand: fixed-horizon cumulative incidence
//! of one named terminal failure at prescheduled, event-free landmarks. It is a synthetic
//! arithmetic and protocol reference, not prospective capture, H2 evidence, calibration
//! validation, a comparator frontier, or a deployment claim.
//! Reported IPCW ESS values diagnose weight concentration across landmark-loss rows; they are not
//! counts of independent episodes, tasks, families, or experimental units.
//! The paired Brier-improvement missing-outcome bounds hold the fitted out-of-fold predictions
//! fixed. They do not remove censoring assumptions used during fitting, validate IPCW, or supply
//! prospective evidence.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const H2_REFERENCE_SCHEMA_VERSION: u32 = 1;
/// Version of the serialized report/CLI summary surface.
pub const H2_REFERENCE_REPORT_SCHEMA_VERSION: u32 = 3;

const MAX_IDENTIFIER_BYTES: usize = 256;
const MAX_EPISODES_DEFAULT: usize = 10_000;
const MAX_LANDMARKS_DEFAULT: usize = 10_000;
const MAX_FEATURES_DEFAULT: usize = 128;
const MAX_OUTER_FOLDS_DEFAULT: usize = 32;
const MAX_INNER_FOLDS_DEFAULT: usize = 32;
const MAX_MODEL_WORK_DEFAULT: usize = 200_000_000;
const MAX_CALIBRATION_BINS_DEFAULT: usize = 128;
const MAX_LEAD_TIME_CUTOFFS_DEFAULT: usize = 128;
const MAX_EVENT_CODES_DEFAULT: usize = 256;
const MAX_CENSORING_STRATA_DEFAULT: usize = 256;
const MIN_CENSORING_SURVIVAL_FLOOR: f64 = 1e-6;
const MAX_DECLARED_UTILITY_COMPONENT: f64 = 1e12;
const MAX_MODEL_RIDGE_PENALTY: f64 = 1e12;
const MAX_MODEL_CONVERGENCE_TOLERANCE: f64 = 1e-2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H2ReferenceScope {
    DeterministicSyntheticFiniteLandmarkBenchmark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2ArtifactBinding {
    pub artifact_uri: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2ArtifactBindings {
    pub analysis_plan: H2ArtifactBinding,
    pub event_ontology: H2ArtifactBinding,
    pub feature_contract: H2ArtifactBinding,
    pub split_manifest: H2ArtifactBinding,
}

/// Outcome-free dataset. The four planning artifacts are parsed and exact-bound by the CLI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2Dataset {
    pub schema_version: u32,
    pub scope: H2ReferenceScope,
    pub bindings: H2ArtifactBindings,
    pub episodes: Vec<H2Episode>,
}

/// Assembled semantic input after exact-byte artifact verification.
#[derive(Debug, Clone, PartialEq)]
pub struct H2ReferenceInput {
    pub dataset: H2Dataset,
    pub plan: H2AnalysisPlan,
    pub ontology: H2EventOntology,
    pub feature_contract: H2FeatureContract,
    pub split_manifest: H2SplitManifest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2AnalysisPlan {
    pub schema_version: u32,
    pub estimand: H2Estimand,
    pub landmark_schedule: H2LandmarkSchedule,
    pub validation: H2ValidationPlan,
    pub outcome_model: H2OutcomeModelPlan,
    pub censoring: H2CensoringPlan,
    pub calibration: H2CalibrationPlan,
    pub alarm_policy: H2AlarmPolicy,
    pub decision_utility: H2DecisionUtilityPlan,
    pub claim_boundary: H2ClaimBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2Estimand {
    pub kind: String,
    pub target_event_code: String,
    pub horizon_ns: u64,
    pub risk_set: String,
    pub interval: String,
    pub landmark_weighting: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2LandmarkSchedule {
    pub kind: String,
    pub offsets_ns: Vec<u64>,
    pub minimum_history_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2ValidationPlan {
    pub outer_split: String,
    pub inner_split: String,
    pub group_keys: Vec<String>,
    pub minimum_outer_folds: usize,
    pub minimum_inner_folds: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2OutcomeModelPlan {
    pub family: String,
    pub ridge_penalty: f64,
    pub intercept_penalized: bool,
    pub standardization: String,
    pub zero_variance_rule: String,
    pub maximum_iterations: usize,
    pub convergence_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2CensoringPlan {
    pub model: String,
    pub assumption: String,
    pub event_weight_time: String,
    pub event_free_weight_time: String,
    pub censor_at_horizon: String,
    pub minimum_survival: f64,
    pub weight_clipping: String,
    pub aggregate: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2CalibrationPlan {
    pub reliability_bin_edges: Vec<f64>,
    pub minimum_target_events: usize,
    pub minimum_non_target_episodes: usize,
    /// Kish weight-concentration threshold over landmark-loss rows.
    ///
    /// The schema-v1 wire key remains `minimum_effective_landmarks` for compatibility. This is
    /// not a minimum number of independent episodes, tasks, families, or experimental units.
    #[serde(rename = "minimum_effective_landmarks")]
    pub minimum_ipcw_weight_concentration_ess_landmark_rows: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2AlarmPolicy {
    /// Externally frozen thresholds avoid holdout tuning in this schema revision.
    pub baseline_threshold: f64,
    pub diagnostic_threshold: f64,
    pub comparison: String,
    pub persistence_scores: usize,
    pub maximum_inter_score_gap_ns: u64,
    pub missing_score_rule: String,
    pub after_alarm_rule: String,
    pub refractory_ns: u64,
    pub episode_reset_rule: String,
    pub minimum_actionable_lead_ns: u64,
    pub maximum_lookback_ns: u64,
    pub match_choice: String,
    pub lead_time_cutoffs_ns: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2DecisionUtilityPlan {
    pub kind: String,
    pub actionable_detection_value: f64,
    pub missed_target_cost: f64,
    pub alarm_action_cost: f64,
    pub false_alarm_cost: f64,
    pub capacity_rejection_cost: f64,
    pub maximum_fallbacks_per_episode: usize,
    pub capacity_priority: String,
    pub intervention_latency_ns: u64,
    pub normalization: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2ClaimBoundary {
    pub synthetic_fixture_only: bool,
    pub establishes_h2_evidence: bool,
    pub prospective_capture: bool,
    pub external_validation: bool,
    pub comparator_frontier_complete: bool,
    pub pid_dependency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2EventOntology {
    pub schema_version: u32,
    pub ontology_id: String,
    pub target_event_codes: Vec<String>,
    pub competing_event_codes: Vec<String>,
    pub censoring_event_codes: Vec<String>,
    pub simultaneous_first_event_rule: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H2FeatureRole {
    Baseline,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2FeatureDefinition {
    pub feature_id: String,
    pub role: H2FeatureRole,
    pub value_type: String,
    pub missing_value_rule: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2FeatureContract {
    pub schema_version: u32,
    pub contract_id: String,
    pub features: Vec<H2FeatureDefinition>,
    pub categorical_encoding: String,
    pub pid_features: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2SplitAssignment {
    pub episode_id: String,
    pub outer_fold: String,
    pub inner_fold: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2SplitManifest {
    pub schema_version: u32,
    pub manifest_id: String,
    pub assignments: Vec<H2SplitAssignment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2ObservedEvent {
    pub event_id: String,
    pub code: String,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2FeatureValue {
    pub feature_id: String,
    pub value: f64,
    pub source_start_ns: u64,
    pub source_end_ns: u64,
    pub available_at_ns: u64,
    pub source_artifact_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2Landmark {
    pub landmark_id: String,
    pub schedule_index: usize,
    pub time_ns: u64,
    pub feature_cutoff_ns: u64,
    pub features: Vec<H2FeatureValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2Episode {
    pub episode_id: String,
    pub persistent_world_id: String,
    pub task_family_id: String,
    pub policy_checkpoint_id: String,
    pub censoring_stratum: String,
    pub censoring_stratum_frozen_at_ns: u64,
    pub censoring_stratum_source_sha256: String,
    pub episode_start_ns: u64,
    pub planned_observation_end_ns: u64,
    pub observed_until_ns: u64,
    pub terminal_event: Option<H2ObservedEvent>,
    pub censoring_event: Option<H2ObservedEvent>,
    pub landmarks: Vec<H2Landmark>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct H2ReferenceLimits {
    pub max_episodes: usize,
    pub max_landmarks: usize,
    pub max_features: usize,
    pub max_outer_folds: usize,
    pub max_inner_folds: usize,
    pub max_model_work_units: usize,
    pub max_calibration_bins: usize,
    pub max_lead_time_cutoffs: usize,
    pub max_event_codes: usize,
    pub max_censoring_strata: usize,
}

impl Default for H2ReferenceLimits {
    fn default() -> Self {
        Self {
            max_episodes: MAX_EPISODES_DEFAULT,
            max_landmarks: MAX_LANDMARKS_DEFAULT,
            max_features: MAX_FEATURES_DEFAULT,
            max_outer_folds: MAX_OUTER_FOLDS_DEFAULT,
            max_inner_folds: MAX_INNER_FOLDS_DEFAULT,
            max_model_work_units: MAX_MODEL_WORK_DEFAULT,
            max_calibration_bins: MAX_CALIBRATION_BINS_DEFAULT,
            max_lead_time_cutoffs: MAX_LEAD_TIME_CUTOFFS_DEFAULT,
            max_event_codes: MAX_EVENT_CODES_DEFAULT,
            max_censoring_strata: MAX_CENSORING_STRATA_DEFAULT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H2ReasonCode {
    SchemaVersionMismatch,
    InvalidIdentifierOrHash,
    InvalidDeclaration,
    ResourceLimitExceeded,
    DuplicateId,
    UnknownEventCode,
    EventOntologyOverlap,
    AmbiguousEventTie,
    TimestampOrderViolation,
    LandmarkScheduleViolation,
    FeatureAfterCutoff,
    FeatureUnavailableAtLandmark,
    PostEventLandmark,
    EpisodeFoldLeakage,
    PersistentWorldFoldLeakage,
    TaskFamilyFoldLeakage,
    InsufficientOuterFolds,
    InsufficientInnerFolds,
    DimensionMismatch,
    NonFiniteValue,
    CensoringStratumUnsupported,
    CensoringSurvivalBelowFloor,
    OutcomeModelFitFailed,
    CalibrationUnavailable,
    AlarmThresholdUnavailable,
    AlarmFollowupIncomplete,
    UtilityFollowupIncomplete,
    NoCommonScoringSupport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2Issue {
    pub code: H2ReasonCode,
    pub episode_id: Option<String>,
    pub landmark_id: Option<String>,
    pub outer_fold: Option<String>,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H2LandmarkOutcome {
    TargetEvent {
        event_id: String,
        relative_time_ns: u64,
    },
    CompetingEvent {
        event_id: String,
        relative_time_ns: u64,
    },
    EventFreeThroughHorizon,
    OutcomeUnobservedCensored {
        relative_time_ns: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2PredictionRecord {
    pub episode_id: String,
    pub landmark_id: String,
    pub outer_fold: String,
    pub landmark_time_ns: u64,
    pub outcome: H2LandmarkOutcome,
    pub baseline_risk: f64,
    pub diagnostic_risk: f64,
    pub ipcw_weight: Option<f64>,
    pub baseline_weighted_loss: Option<f64>,
    pub diagnostic_weighted_loss: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2ModelReceipt {
    pub feature_ids: Vec<String>,
    pub dropped_zero_variance_features: Vec<String>,
    pub means: Vec<f64>,
    pub scales: Vec<f64>,
    pub coefficients: Vec<f64>,
    pub intercept: f64,
    pub iterations: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2FoldScore {
    pub outer_fold: String,
    pub eligible_landmarks: usize,
    pub observed_loss_rows: usize,
    pub censored_landmarks: usize,
    pub weight_sum: f64,
    pub maximum_weight: f64,
    /// Kish-style concentration diagnostic for the IPCW-weighted landmark-loss rows.
    /// This is not a count of independent episodes, tasks, families, or experimental units.
    pub ipcw_weight_concentration_ess_landmark_rows: f64,
    pub baseline_brier: f64,
    pub diagnostic_brier: f64,
    pub brier_improvement: f64,
    pub baseline_model: H2ModelReceipt,
    pub diagnostic_model: H2ModelReceipt,
    pub predictions: Vec<H2PredictionRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H2FoldOutcome {
    Produced {
        score: Box<H2FoldScore>,
    },
    Abstained {
        outer_fold: String,
        issues: Vec<H2Issue>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2AggregateScore {
    pub eligible_landmarks: usize,
    pub observed_loss_rows: usize,
    pub censored_landmarks: usize,
    pub weight_sum: f64,
    pub maximum_weight: f64,
    /// Kish-style concentration diagnostic after pooling IPCW-weighted landmark-loss rows.
    /// It is not an independent-sample size and must not be used as an episode/task count.
    pub ipcw_weight_concentration_ess_landmark_rows: f64,
    pub baseline_brier: f64,
    pub diagnostic_brier: f64,
    pub brier_improvement: f64,
    pub precision: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2ReliabilityBin {
    pub lower_inclusive: f64,
    pub upper_inclusive: f64,
    pub observed_rows: usize,
    pub target_rows: usize,
    pub weight_sum: f64,
    /// Kish-style concentration diagnostic for IPCW-weighted landmark rows in this bin.
    /// It is not a count of independent observations.
    pub ipcw_weight_concentration_ess_landmark_rows: f64,
    pub weighted_observed_risk: f64,
    pub weighted_mean_prediction: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H2CalibrationResult {
    ProducedReferenceReliability { bins: Vec<H2ReliabilityBin> },
    Abstained { reason: H2ReasonCode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H2ModelKind {
    Baseline,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2AlarmRecord {
    pub alarm_id: String,
    pub episode_id: String,
    pub landmark_id: String,
    pub timestamp_ns: u64,
    pub capacity_rejected: bool,
    pub matched_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H2LeadTimeRecord {
    Detected { event_id: String, lead_time_ns: u64 },
    Undetected { event_id: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2DetectionCurvePoint {
    pub minimum_lead_ns: u64,
    pub detected_events: usize,
    pub total_target_events: usize,
    pub fraction: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2AlarmSummary {
    pub model: H2ModelKind,
    pub threshold: f64,
    pub alarms_emitted: usize,
    pub alarms_executed: usize,
    pub alarms_matched: usize,
    pub alarms_unmatched: usize,
    pub alarms_late: usize,
    pub refractory_suppressed: usize,
    pub capacity_rejected: usize,
    pub target_events: usize,
    pub detected_events: usize,
    pub undetected_events: usize,
    pub lead_times: Vec<H2LeadTimeRecord>,
    pub detection_curve: Vec<H2DetectionCurvePoint>,
    pub alarms: Vec<H2AlarmRecord>,
    pub assumed_payoff_utility_per_evaluable_episode: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H2AlarmResult {
    Produced { summary: H2AlarmSummary },
    Abstained { reason: H2ReasonCode },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2Denominators {
    pub task_families: usize,
    pub persistent_worlds: usize,
    pub episodes: usize,
    pub scheduled_landmarks: usize,
    pub eligible_landmarks: usize,
    pub ineligible_landmarks: usize,
    pub target_event_outcomes: usize,
    pub competing_event_outcomes: usize,
    pub event_free_outcomes: usize,
    pub censored_outcomes: usize,
    pub unique_target_events: usize,
    pub unique_competing_events: usize,
    pub eligible_target_events: usize,
    pub eligible_competing_events: usize,
    pub outer_folds_produced: usize,
    pub outer_folds_abstained: usize,
}

/// What the observed finite benchmark identifies about mean target risk without assuming
/// conditionally independent censoring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H2TargetRiskIdentification {
    /// Every eligible landmark outcome is observed, so the finite-benchmark mean is known exactly.
    ObservedFiniteBenchmarkPoint { target_risk: f64 },
    /// Censored outcomes admit multiple compatible target risks. These are conservative binary
    /// missing-outcome bounds over eligible landmark rows; episode-level event-time constraints
    /// can make the identified set narrower.
    NotPointIdentifiedNoAssumptionBounds {
        lower_target_risk: f64,
        upper_target_risk: f64,
    },
    /// Input validation failed before eligible outcomes could be derived.
    UnavailableInvalidInput,
}

/// Identification result for the finite-benchmark mean paired Brier improvement after holding the
/// already-fitted out-of-fold predictions fixed.
///
/// Improvement is baseline squared error minus diagnostic squared error, so positive values favor
/// the diagnostic model. Observed rows contribute their observed binary outcome. Each censored row
/// contributes the minimum and maximum over both possible binary outcomes. The result is unweighted
/// over all eligible landmark rows, matching this reference's frozen landmark-weighting rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum H2FixedPredictionBrierImprovementResult {
    /// No eligible outcome is censored, so the finite-benchmark paired improvement is known.
    ObservedFiniteBenchmarkPoint {
        paired_brier_improvement: f64,
        eligible_landmark_rows: usize,
        observed_outcome_rows: usize,
        censored_outcome_rows: usize,
    },
    /// At least one eligible outcome is censored. The interval is a conservative rowwise
    /// binary-outcome bound conditional on the recorded predictions; episode-level and other
    /// cross-row restrictions can make the compatible interval narrower.
    NotPointIdentifiedConservativeMissingOutcomeBounds {
        lower_paired_brier_improvement: f64,
        upper_paired_brier_improvement: f64,
        eligible_landmark_rows: usize,
        observed_outcome_rows: usize,
        censored_outcome_rows: usize,
    },
    /// Structural input validation failed before a complete prediction surface was available.
    UnavailableInvalidInput,
    /// At least one outer fold abstained, so the all-eligible-row paired result is unavailable.
    UnavailableFoldAbstention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H2FixedPredictionConditioning {
    AlreadyFittedOutOfFoldPredictionsHeldFixed,
}

/// Scope and result of the fixed-prediction missing-outcome sensitivity calculation.
///
/// This calculation deliberately does not refit either model. Consequently, it does not remove or
/// test censoring assumptions used to fit those models, does not validate IPCW, and is not
/// prospective evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2FixedPredictionBrierImprovementIdentification {
    pub conditioning: H2FixedPredictionConditioning,
    pub removes_censoring_assumptions_used_during_model_fitting: bool,
    pub validates_ipcw: bool,
    pub prospective_evidence: bool,
    pub result: H2FixedPredictionBrierImprovementResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct H2ReferenceReport {
    pub schema_version: u32,
    pub synthetic_fixture_only: bool,
    pub establishes_h2_evidence: bool,
    pub prospective_capture: bool,
    pub external_validation: bool,
    pub comparator_frontier_complete: bool,
    pub pid_dependency: String,
    pub estimand: String,
    pub censoring_assumption_validated: bool,
    pub target_risk_identification_without_censoring_assumption: H2TargetRiskIdentification,
    pub fixed_prediction_paired_brier_improvement_identification:
        H2FixedPredictionBrierImprovementIdentification,
    pub denominators: H2Denominators,
    pub fold_outcomes: Vec<H2FoldOutcome>,
    pub aggregate_score: Option<H2AggregateScore>,
    pub diagnostic_calibration: H2CalibrationResult,
    pub alarm_results: BTreeMap<H2ModelKind, H2AlarmResult>,
    pub issues: Vec<H2Issue>,
}

impl H2ReferenceReport {
    /// Returns whether the bounded software reference completed coherently.
    ///
    /// This does not validate the censoring assumption or establish scientific evidence.
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
            && self.aggregate_score.is_some()
            && self.denominators.outer_folds_abstained == 0
            && matches!(
                &self
                    .fixed_prediction_paired_brier_improvement_identification
                    .result,
                H2FixedPredictionBrierImprovementResult::ObservedFiniteBenchmarkPoint { .. }
                    | H2FixedPredictionBrierImprovementResult::NotPointIdentifiedConservativeMissingOutcomeBounds {
                        ..
                    }
            )
    }
}

#[derive(Debug, Clone)]
struct LandmarkRow<'a> {
    episode: &'a H2Episode,
    landmark: &'a H2Landmark,
    outer_fold: &'a str,
    inner_fold: &'a str,
    outcome: H2LandmarkOutcome,
    values: Vec<f64>,
}

#[derive(Debug, Clone)]
struct ReverseKm {
    censor_steps: Vec<(u64, f64)>,
}

impl ReverseKm {
    fn fit(rows: &[&LandmarkRow<'_>]) -> Option<Self> {
        if rows.is_empty() {
            return None;
        }
        let mut observations = rows
            .iter()
            .map(|row| match row.outcome {
                H2LandmarkOutcome::TargetEvent {
                    relative_time_ns, ..
                }
                | H2LandmarkOutcome::CompetingEvent {
                    relative_time_ns, ..
                } => (relative_time_ns, false),
                H2LandmarkOutcome::EventFreeThroughHorizon => (u64::MAX, false),
                H2LandmarkOutcome::OutcomeUnobservedCensored {
                    relative_time_ns, ..
                } => (relative_time_ns, true),
            })
            .collect::<Vec<_>>();
        observations.sort_unstable_by_key(|(time, _)| *time);
        let mut survival = 1.0;
        let mut censor_steps = Vec::new();
        let mut index = 0_usize;
        while index < observations.len() {
            let time = observations[index].0;
            let mut end = index + 1;
            while end < observations.len() && observations[end].0 == time {
                end += 1;
            }
            let censor_events = observations[index..end]
                .iter()
                .filter(|(_, censored)| *censored)
                .count();
            if censor_events > 0 {
                let at_risk = observations.len() - index;
                survival *= 1.0 - censor_events as f64 / at_risk as f64;
                censor_steps.push((time, survival));
            }
            index = end;
        }
        Some(Self { censor_steps })
    }

    fn left_limit(&self, time: u64) -> f64 {
        self.censor_steps
            .iter()
            .take_while(|(step, _)| *step < time)
            .last()
            .map_or(1.0, |(_, survival)| *survival)
    }

    fn at(&self, time: u64) -> f64 {
        self.censor_steps
            .iter()
            .take_while(|(step, _)| *step <= time)
            .last()
            .map_or(1.0, |(_, survival)| *survival)
    }
}

/// Execute the bounded synthetic H2 reference.
pub fn run_h2_reference(input: &H2ReferenceInput) -> H2ReferenceReport {
    run_h2_reference_with_limits(input, H2ReferenceLimits::default())
}

pub fn run_h2_reference_with_limits(
    input: &H2ReferenceInput,
    limits: H2ReferenceLimits,
) -> H2ReferenceReport {
    let mut issues = validate_input(input, limits);
    if !issues.is_empty() {
        return failed_report(input, issues);
    }

    let assignments = input
        .split_manifest
        .assignments
        .iter()
        .map(|assignment| (assignment.episode_id.as_str(), assignment))
        .collect::<BTreeMap<_, _>>();
    let definitions = input
        .feature_contract
        .features
        .iter()
        .map(|definition| (definition.feature_id.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::new();
    let mut denominators = base_denominators(input);
    let mut eligible_target_events = BTreeSet::new();
    let mut eligible_competing_events = BTreeSet::new();
    for episode in &input.dataset.episodes {
        let assignment = assignments
            .get(episode.episode_id.as_str())
            .expect("validated split assignment");
        for landmark in &episode.landmarks {
            let Some(outcome) = derive_outcome(episode, landmark, &input.plan) else {
                denominators.ineligible_landmarks += 1;
                continue;
            };
            denominators.eligible_landmarks += 1;
            match &outcome {
                H2LandmarkOutcome::TargetEvent { event_id, .. } => {
                    denominators.target_event_outcomes += 1;
                    eligible_target_events.insert(event_id.clone());
                }
                H2LandmarkOutcome::CompetingEvent { event_id, .. } => {
                    denominators.competing_event_outcomes += 1;
                    eligible_competing_events.insert(event_id.clone());
                }
                H2LandmarkOutcome::EventFreeThroughHorizon => denominators.event_free_outcomes += 1,
                H2LandmarkOutcome::OutcomeUnobservedCensored { .. } => {
                    denominators.censored_outcomes += 1
                }
            }
            let by_id = landmark
                .features
                .iter()
                .map(|feature| (feature.feature_id.as_str(), feature.value))
                .collect::<BTreeMap<_, _>>();
            let values = input
                .feature_contract
                .features
                .iter()
                .map(|definition| {
                    debug_assert!(definitions.contains_key(definition.feature_id.as_str()));
                    by_id[definition.feature_id.as_str()]
                })
                .collect();
            rows.push(LandmarkRow {
                episode,
                landmark,
                outer_fold: &assignment.outer_fold,
                inner_fold: &assignment.inner_fold,
                outcome,
                values,
            });
        }
    }
    denominators.eligible_target_events = eligible_target_events.len();
    denominators.eligible_competing_events = eligible_competing_events.len();

    let outer_folds = input
        .split_manifest
        .assignments
        .iter()
        .map(|assignment| assignment.outer_fold.as_str())
        .collect::<BTreeSet<_>>();
    let baseline_indices = input
        .feature_contract
        .features
        .iter()
        .enumerate()
        .filter_map(|(index, definition)| {
            (definition.role == H2FeatureRole::Baseline).then_some(index)
        })
        .collect::<Vec<_>>();
    let diagnostic_indices = input
        .feature_contract
        .features
        .iter()
        .enumerate()
        .filter_map(|(index, definition)| {
            matches!(
                definition.role,
                H2FeatureRole::Baseline | H2FeatureRole::Diagnostic
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();
    let mut fold_outcomes = Vec::with_capacity(outer_folds.len());
    for outer_fold in outer_folds {
        fold_outcomes.push(score_outer_fold(
            input,
            &rows,
            outer_fold,
            &baseline_indices,
            &diagnostic_indices,
        ));
    }
    denominators.outer_folds_produced = fold_outcomes
        .iter()
        .filter(|outcome| matches!(outcome, H2FoldOutcome::Produced { .. }))
        .count();
    denominators.outer_folds_abstained = fold_outcomes.len() - denominators.outer_folds_produced;
    for outcome in &fold_outcomes {
        if let H2FoldOutcome::Abstained {
            issues: fold_issues,
            ..
        } = outcome
        {
            issues.extend(fold_issues.iter().cloned());
        }
    }
    let all_folds_produced = denominators.outer_folds_abstained == 0;
    let predictions = if all_folds_produced {
        fold_outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                H2FoldOutcome::Produced { score } => Some(score.predictions.as_slice()),
                H2FoldOutcome::Abstained { .. } => None,
            })
            .flatten()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let aggregate_score = if all_folds_produced {
        aggregate_scores(&fold_outcomes)
    } else {
        None
    };
    let diagnostic_calibration = if all_folds_produced {
        calibration_result(input, &predictions)
    } else {
        H2CalibrationResult::Abstained {
            reason: H2ReasonCode::NoCommonScoringSupport,
        }
    };
    let alarm_results = [H2ModelKind::Baseline, H2ModelKind::Diagnostic]
        .into_iter()
        .map(|model| {
            let result = if all_folds_produced {
                let threshold = match model {
                    H2ModelKind::Baseline => input.plan.alarm_policy.baseline_threshold,
                    H2ModelKind::Diagnostic => input.plan.alarm_policy.diagnostic_threshold,
                };
                alarm_result(input, &predictions, model, threshold)
            } else {
                H2AlarmResult::Abstained {
                    reason: H2ReasonCode::NoCommonScoringSupport,
                }
            };
            (model, result)
        })
        .collect();
    let target_risk_identification_without_censoring_assumption =
        target_risk_identification_without_censoring_assumption(&denominators);
    let fixed_prediction_paired_brier_improvement_identification =
        fixed_prediction_paired_brier_improvement_identification(&predictions, &denominators);
    H2ReferenceReport {
        schema_version: H2_REFERENCE_REPORT_SCHEMA_VERSION,
        synthetic_fixture_only: true,
        establishes_h2_evidence: false,
        prospective_capture: false,
        external_validation: false,
        comparator_frontier_complete: false,
        pid_dependency: "none".to_string(),
        estimand: "fixed_horizon_target_cumulative_incidence".to_string(),
        censoring_assumption_validated: false,
        target_risk_identification_without_censoring_assumption,
        fixed_prediction_paired_brier_improvement_identification,
        denominators,
        fold_outcomes,
        aggregate_score,
        diagnostic_calibration,
        alarm_results,
        issues,
    }
}

fn failed_report(input: &H2ReferenceInput, issues: Vec<H2Issue>) -> H2ReferenceReport {
    H2ReferenceReport {
        schema_version: H2_REFERENCE_REPORT_SCHEMA_VERSION,
        synthetic_fixture_only: true,
        establishes_h2_evidence: false,
        prospective_capture: false,
        external_validation: false,
        comparator_frontier_complete: false,
        pid_dependency: "none".to_string(),
        estimand: "fixed_horizon_target_cumulative_incidence".to_string(),
        censoring_assumption_validated: false,
        target_risk_identification_without_censoring_assumption:
            H2TargetRiskIdentification::UnavailableInvalidInput,
        fixed_prediction_paired_brier_improvement_identification:
            fixed_prediction_brier_improvement_with_scope(
                H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput,
            ),
        denominators: base_denominators(input),
        fold_outcomes: Vec::new(),
        aggregate_score: None,
        diagnostic_calibration: H2CalibrationResult::Abstained {
            reason: H2ReasonCode::CalibrationUnavailable,
        },
        alarm_results: [H2ModelKind::Baseline, H2ModelKind::Diagnostic]
            .into_iter()
            .map(|model| {
                (
                    model,
                    H2AlarmResult::Abstained {
                        reason: H2ReasonCode::NoCommonScoringSupport,
                    },
                )
            })
            .collect(),
        issues,
    }
}

fn target_risk_identification_without_censoring_assumption(
    denominators: &H2Denominators,
) -> H2TargetRiskIdentification {
    let eligible = denominators.eligible_landmarks;
    if eligible == 0 {
        return H2TargetRiskIdentification::UnavailableInvalidInput;
    }
    let lower_target_risk = denominators.target_event_outcomes as f64 / eligible as f64;
    if denominators.censored_outcomes == 0 {
        return H2TargetRiskIdentification::ObservedFiniteBenchmarkPoint {
            target_risk: lower_target_risk,
        };
    }
    let upper_target_count = denominators
        .target_event_outcomes
        .saturating_add(denominators.censored_outcomes)
        .min(eligible);
    H2TargetRiskIdentification::NotPointIdentifiedNoAssumptionBounds {
        lower_target_risk,
        upper_target_risk: upper_target_count as f64 / eligible as f64,
    }
}

fn fixed_prediction_brier_improvement_with_scope(
    result: H2FixedPredictionBrierImprovementResult,
) -> H2FixedPredictionBrierImprovementIdentification {
    H2FixedPredictionBrierImprovementIdentification {
        conditioning: H2FixedPredictionConditioning::AlreadyFittedOutOfFoldPredictionsHeldFixed,
        removes_censoring_assumptions_used_during_model_fitting: false,
        validates_ipcw: false,
        prospective_evidence: false,
        result,
    }
}

fn fixed_prediction_paired_brier_improvement_identification(
    predictions: &[&H2PredictionRecord],
    denominators: &H2Denominators,
) -> H2FixedPredictionBrierImprovementIdentification {
    if denominators.outer_folds_abstained > 0 {
        return fixed_prediction_brier_improvement_with_scope(
            H2FixedPredictionBrierImprovementResult::UnavailableFoldAbstention,
        );
    }
    let eligible_rows = denominators.eligible_landmarks;
    if eligible_rows == 0 || predictions.len() != eligible_rows {
        return fixed_prediction_brier_improvement_with_scope(
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput,
        );
    }

    // Fix summation order to the content-bearing row identity so input or fold traversal order does
    // not alter floating-point accumulation.
    let mut ordered = predictions.to_vec();
    ordered.sort_by(|left, right| {
        (
            left.episode_id.as_str(),
            left.landmark_time_ns,
            left.landmark_id.as_str(),
            left.outer_fold.as_str(),
        )
            .cmp(&(
                right.episode_id.as_str(),
                right.landmark_time_ns,
                right.landmark_id.as_str(),
                right.outer_fold.as_str(),
            ))
    });
    if ordered.windows(2).any(|pair| {
        (
            pair[0].episode_id.as_str(),
            pair[0].landmark_time_ns,
            pair[0].landmark_id.as_str(),
        ) == (
            pair[1].episode_id.as_str(),
            pair[1].landmark_time_ns,
            pair[1].landmark_id.as_str(),
        )
    }) {
        return fixed_prediction_brier_improvement_with_scope(
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput,
        );
    }

    let mut lower_sum = 0.0;
    let mut upper_sum = 0.0;
    let mut observed_rows = 0_usize;
    let mut censored_rows = 0_usize;
    for prediction in ordered {
        let baseline = prediction.baseline_risk;
        let diagnostic = prediction.diagnostic_risk;
        if !baseline.is_finite()
            || !diagnostic.is_finite()
            || !(0.0..=1.0).contains(&baseline)
            || !(0.0..=1.0).contains(&diagnostic)
        {
            return fixed_prediction_brier_improvement_with_scope(
                H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput,
            );
        }
        let delta_at_zero = baseline.powi(2) - diagnostic.powi(2);
        let delta_at_one = (1.0 - baseline).powi(2) - (1.0 - diagnostic).powi(2);
        // The target is binary, so these two values exhaust its support. Equivalently,
        // Δ(y) = baseline² - diagnostic² + 2y(diagnostic - baseline) is affine in y.
        if let Some(label) = outcome_label(&prediction.outcome) {
            let delta = if label { delta_at_one } else { delta_at_zero };
            lower_sum += delta;
            upper_sum += delta;
            observed_rows += 1;
        } else {
            lower_sum += delta_at_zero.min(delta_at_one);
            upper_sum += delta_at_zero.max(delta_at_one);
            censored_rows += 1;
        }
    }
    let expected_observed_rows = denominators
        .target_event_outcomes
        .checked_add(denominators.competing_event_outcomes)
        .and_then(|count| count.checked_add(denominators.event_free_outcomes));
    if observed_rows.checked_add(censored_rows) != Some(eligible_rows)
        || expected_observed_rows != Some(observed_rows)
        || denominators.censored_outcomes != censored_rows
    {
        return fixed_prediction_brier_improvement_with_scope(
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput,
        );
    }
    let denominator = eligible_rows as f64;
    let lower = lower_sum / denominator;
    let upper = upper_sum / denominator;
    let range_tolerance = 32.0 * f64::EPSILON;
    if !lower.is_finite()
        || !upper.is_finite()
        || lower < -1.0 - range_tolerance
        || upper > 1.0 + range_tolerance
        || lower > upper + range_tolerance
    {
        return fixed_prediction_brier_improvement_with_scope(
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput,
        );
    }
    let lower = lower.clamp(-1.0, 1.0);
    let upper = upper.clamp(-1.0, 1.0);
    let result = if censored_rows == 0 {
        H2FixedPredictionBrierImprovementResult::ObservedFiniteBenchmarkPoint {
            paired_brier_improvement: lower,
            eligible_landmark_rows: eligible_rows,
            observed_outcome_rows: observed_rows,
            censored_outcome_rows: 0,
        }
    } else {
        H2FixedPredictionBrierImprovementResult::NotPointIdentifiedConservativeMissingOutcomeBounds {
            lower_paired_brier_improvement: lower,
            upper_paired_brier_improvement: upper,
            eligible_landmark_rows: eligible_rows,
            observed_outcome_rows: observed_rows,
            censored_outcome_rows: censored_rows,
        }
    };
    fixed_prediction_brier_improvement_with_scope(result)
}

fn base_denominators(input: &H2ReferenceInput) -> H2Denominators {
    let episodes = &input.dataset.episodes;
    H2Denominators {
        task_families: episodes
            .iter()
            .map(|episode| episode.task_family_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        persistent_worlds: episodes
            .iter()
            .map(|episode| episode.persistent_world_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        episodes: episodes.len(),
        scheduled_landmarks: episodes.iter().map(|episode| episode.landmarks.len()).sum(),
        unique_target_events: episodes
            .iter()
            .filter_map(|episode| episode.terminal_event.as_ref())
            .filter(|event| input.ontology.target_event_codes.contains(&event.code))
            .count(),
        unique_competing_events: episodes
            .iter()
            .filter_map(|episode| episode.terminal_event.as_ref())
            .filter(|event| input.ontology.competing_event_codes.contains(&event.code))
            .count(),
        ..H2Denominators::default()
    }
}

fn issue(
    code: H2ReasonCode,
    episode_id: Option<&str>,
    landmark_id: Option<&str>,
    outer_fold: Option<&str>,
    field: impl Into<String>,
    message: impl Into<String>,
) -> H2Issue {
    H2Issue {
        code,
        episode_id: episode_id.map(str::to_string),
        landmark_id: landmark_id.map(str::to_string),
        outer_fold: outer_fold.map(str::to_string),
        field: field.into(),
        message: message.into(),
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_IDENTIFIER_BYTES && !value.chars().any(char::is_control)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn checked_sum_landmarks(input: &H2ReferenceInput) -> Option<usize> {
    input
        .dataset
        .episodes
        .iter()
        .try_fold(0_usize, |total, episode| {
            total.checked_add(episode.landmarks.len())
        })
}

fn validate_input(input: &H2ReferenceInput, limits: H2ReferenceLimits) -> Vec<H2Issue> {
    let mut issues = Vec::new();
    if [
        input.dataset.schema_version,
        input.plan.schema_version,
        input.ontology.schema_version,
        input.feature_contract.schema_version,
        input.split_manifest.schema_version,
    ]
    .into_iter()
    .any(|version| version != H2_REFERENCE_SCHEMA_VERSION)
    {
        issues.push(issue(
            H2ReasonCode::SchemaVersionMismatch,
            None,
            None,
            None,
            "schema_version",
            "every H2 reference artifact must use schema version 1",
        ));
    }
    validate_declarations(input, &mut issues);

    let landmark_count = checked_sum_landmarks(input);
    if input.dataset.episodes.is_empty()
        || input.dataset.episodes.len() > limits.max_episodes
        || landmark_count.is_none_or(|count| count == 0 || count > limits.max_landmarks)
        || input.feature_contract.features.is_empty()
        || input.feature_contract.features.len() > limits.max_features
    {
        issues.push(issue(
            H2ReasonCode::ResourceLimitExceeded,
            None,
            None,
            None,
            "dataset",
            "episode, landmark, or feature count is empty, overflowed, or exceeds its bound",
        ));
    }
    let outer_fold_count = input
        .split_manifest
        .assignments
        .iter()
        .map(|assignment| assignment.outer_fold.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let estimated_work = landmark_count
        .and_then(|landmarks| {
            let columns = input.feature_contract.features.len().checked_add(1)?;
            let hessian_work = landmarks.checked_mul(columns)?.checked_mul(columns)?;
            let solver_work = columns.checked_mul(columns)?.checked_mul(columns)?;
            hessian_work
                .checked_add(solver_work)?
                .checked_mul(input.plan.outcome_model.maximum_iterations)?
                .checked_mul(outer_fold_count)?
                .checked_mul(2)
        })
        .unwrap_or(usize::MAX);
    if estimated_work > limits.max_model_work_units {
        issues.push(issue(
            H2ReasonCode::ResourceLimitExceeded,
            None,
            None,
            None,
            "outcome_model",
            "declared dense model work exceeds the caller's bound",
        ));
    }
    let event_code_count = input
        .ontology
        .target_event_codes
        .len()
        .checked_add(input.ontology.competing_event_codes.len())
        .and_then(|count| count.checked_add(input.ontology.censoring_event_codes.len()));
    let censoring_strata = input
        .dataset
        .episodes
        .iter()
        .map(|episode| episode.censoring_stratum.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    if input.plan.calibration.reliability_bin_edges.len()
        > limits.max_calibration_bins.saturating_add(1)
        || input.plan.alarm_policy.lead_time_cutoffs_ns.len() > limits.max_lead_time_cutoffs
        || event_code_count.is_none_or(|count| count > limits.max_event_codes)
        || censoring_strata > limits.max_censoring_strata
        || input.split_manifest.assignments.len() > limits.max_episodes
        || input.plan.landmark_schedule.offsets_ns.len() > limits.max_landmarks
    {
        issues.push(issue(
            H2ReasonCode::ResourceLimitExceeded,
            None,
            None,
            None,
            "analysis_collections",
            "calibration bins, lead cutoffs, event codes, strata, splits, or schedule exceed bounds",
        ));
    }

    validate_ontology(input, &mut issues);
    validate_features(input, &mut issues);
    validate_splits(input, limits, &mut issues);
    validate_episodes(input, &mut issues);
    issues.sort_by(|left, right| {
        (
            left.code,
            &left.episode_id,
            &left.landmark_id,
            &left.outer_fold,
            &left.field,
            &left.message,
        )
            .cmp(&(
                right.code,
                &right.episode_id,
                &right.landmark_id,
                &right.outer_fold,
                &right.field,
                &right.message,
            ))
    });
    issues.dedup();
    issues
}

fn validate_declarations(input: &H2ReferenceInput, issues: &mut Vec<H2Issue>) {
    let plan = &input.plan;
    let exact = plan.estimand.kind == "fixed_horizon_target_cumulative_incidence"
        && plan.estimand.horizon_ns > 0
        && plan.estimand.risk_set == "event_and_censor_free_at_landmark"
        && plan.estimand.interval == "open_left_closed_right"
        && plan.estimand.landmark_weighting == "uniform_eligible_scheduled_landmarks"
        && plan.landmark_schedule.kind == "fixed_offsets_from_episode_start"
        && !plan.landmark_schedule.offsets_ns.is_empty()
        && plan.validation.outer_split == "grouped_task_family_k_fold"
        && plan.validation.inner_split == "grouped_k_fold"
        && plan.validation.group_keys
            == ["episode_id".to_string(), "persistent_world_id".to_string()]
        && plan.validation.minimum_outer_folds >= 2
        && plan.validation.minimum_inner_folds >= 2
        && plan.outcome_model.family == "deterministic_weighted_l2_logistic"
        && plan.outcome_model.ridge_penalty.is_finite()
        && (0.0..=MAX_MODEL_RIDGE_PENALTY).contains(&plan.outcome_model.ridge_penalty)
        && plan.outcome_model.ridge_penalty > 0.0
        && !plan.outcome_model.intercept_penalized
        && plan.outcome_model.standardization == "outer_training_mean_sd"
        && plan.outcome_model.zero_variance_rule == "drop_and_report"
        && plan.outcome_model.maximum_iterations > 0
        && plan.outcome_model.convergence_tolerance.is_finite()
        && (0.0..=MAX_MODEL_CONVERGENCE_TOLERANCE)
            .contains(&plan.outcome_model.convergence_tolerance)
        && plan.outcome_model.convergence_tolerance > 0.0
        && plan.censoring.model == "reverse_kaplan_meier_by_frozen_stratum"
        && plan.censoring.assumption == "independent_given_prelandmark_stratum"
        && plan.censoring.event_weight_time == "left_limit"
        && plan.censoring.event_free_weight_time == "horizon"
        && plan.censoring.censor_at_horizon == "outcome_unobserved_censored"
        && plan.censoring.minimum_survival.is_finite()
        && (0.0..=1.0).contains(&plan.censoring.minimum_survival)
        && plan.censoring.minimum_survival >= MIN_CENSORING_SURVIVAL_FLOOR
        && plan.censoring.weight_clipping == "forbidden"
        && plan.censoring.aggregate == "horvitz_thompson_over_all_eligible_landmarks"
        && valid_alarm_policy(&plan.alarm_policy)
        && valid_utility_plan(&plan.decision_utility)
        && plan.alarm_policy.maximum_lookback_ns <= plan.estimand.horizon_ns
        && plan.alarm_policy.minimum_actionable_lead_ns
            >= plan.decision_utility.intervention_latency_ns
        && plan.claim_boundary.synthetic_fixture_only
        && !plan.claim_boundary.establishes_h2_evidence
        && !plan.claim_boundary.prospective_capture
        && !plan.claim_boundary.external_validation
        && !plan.claim_boundary.comparator_frontier_complete
        && plan.claim_boundary.pid_dependency == "none";
    if !exact {
        issues.push(issue(
            H2ReasonCode::InvalidDeclaration,
            None,
            None,
            None,
            "analysis_plan",
            "plan is not the exact bounded fixed-horizon synthetic reference contract",
        ));
    }
    let offsets = &plan.landmark_schedule.offsets_ns;
    if offsets.windows(2).any(|window| window[0] >= window[1])
        || offsets
            .first()
            .is_some_and(|offset| *offset < plan.landmark_schedule.minimum_history_ns)
    {
        issues.push(issue(
            H2ReasonCode::LandmarkScheduleViolation,
            None,
            None,
            None,
            "analysis_plan.landmark_schedule",
            "landmark offsets must be strictly increasing and meet minimum history",
        ));
    }
    let edges = &plan.calibration.reliability_bin_edges;
    if edges.len() < 2
        || edges.first() != Some(&0.0)
        || edges.last() != Some(&1.0)
        || edges
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        || edges.windows(2).any(|window| window[0] >= window[1])
        || !plan
            .calibration
            .minimum_ipcw_weight_concentration_ess_landmark_rows
            .is_finite()
        || plan
            .calibration
            .minimum_ipcw_weight_concentration_ess_landmark_rows
            < 0.0
    {
        issues.push(issue(
            H2ReasonCode::InvalidDeclaration,
            None,
            None,
            None,
            "analysis_plan.calibration",
            "reliability edges must strictly partition [0,1] and thresholds must be finite",
        ));
    }
    for (field, binding) in [
        (
            "bindings.analysis_plan",
            &input.dataset.bindings.analysis_plan,
        ),
        (
            "bindings.event_ontology",
            &input.dataset.bindings.event_ontology,
        ),
        (
            "bindings.feature_contract",
            &input.dataset.bindings.feature_contract,
        ),
        (
            "bindings.split_manifest",
            &input.dataset.bindings.split_manifest,
        ),
    ] {
        if !valid_identifier(&binding.artifact_uri) || !valid_sha256(&binding.sha256) {
            issues.push(issue(
                H2ReasonCode::InvalidIdentifierOrHash,
                None,
                None,
                None,
                field,
                "artifact URI or SHA-256 is invalid",
            ));
        }
    }
}

fn valid_alarm_policy(policy: &H2AlarmPolicy) -> bool {
    [policy.baseline_threshold, policy.diagnostic_threshold]
        .into_iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
        && policy.comparison == "risk_greater_than_or_equal_to_threshold"
        && policy.persistence_scores > 0
        && policy.missing_score_rule == "break_streak_and_emit_no_alarm"
        && policy.after_alarm_rule == "clear_streak"
        && policy.episode_reset_rule == "clear_all_state"
        && policy.maximum_lookback_ns >= policy.minimum_actionable_lead_ns
        && policy.match_choice == "earliest_actionable_alarm"
        && !policy.lead_time_cutoffs_ns.is_empty()
        && policy
            .lead_time_cutoffs_ns
            .windows(2)
            .all(|window| window[0] < window[1])
}

fn valid_utility_plan(plan: &H2DecisionUtilityPlan) -> bool {
    [
        plan.actionable_detection_value,
        plan.missed_target_cost,
        plan.alarm_action_cost,
        plan.false_alarm_cost,
        plan.capacity_rejection_cost,
    ]
    .into_iter()
    .all(|value| value.is_finite() && (0.0..=MAX_DECLARED_UTILITY_COMPONENT).contains(&value))
        && plan.kind == "declared_warning_payoff_scenario"
        && plan.capacity_priority == "alarm_time_then_alarm_id"
        && plan.normalization == "per_evaluable_episode"
}

fn validate_ontology(input: &H2ReferenceInput, issues: &mut Vec<H2Issue>) {
    let ontology = &input.ontology;
    if !valid_identifier(&ontology.ontology_id)
        || ontology.target_event_codes.len() != 1
        || ontology.target_event_codes.first() != Some(&input.plan.estimand.target_event_code)
        || ontology.simultaneous_first_event_rule != "reject_as_ambiguous"
    {
        issues.push(issue(
            H2ReasonCode::InvalidDeclaration,
            None,
            None,
            None,
            "event_ontology",
            "ontology must name exactly the plan's target and reject simultaneous events",
        ));
    }
    let mut seen = BTreeSet::new();
    for code in ontology
        .target_event_codes
        .iter()
        .chain(&ontology.competing_event_codes)
        .chain(&ontology.censoring_event_codes)
    {
        if !valid_identifier(code) {
            issues.push(issue(
                H2ReasonCode::InvalidIdentifierOrHash,
                None,
                None,
                None,
                "event_ontology.code",
                "event code is invalid",
            ));
        }
        if !seen.insert(code) {
            issues.push(issue(
                H2ReasonCode::EventOntologyOverlap,
                None,
                None,
                None,
                "event_ontology",
                format!("event code {code:?} appears in more than one role"),
            ));
        }
    }
}

fn validate_features(input: &H2ReferenceInput, issues: &mut Vec<H2Issue>) {
    let contract = &input.feature_contract;
    if !valid_identifier(&contract.contract_id)
        || contract.categorical_encoding != "preencoded_by_frozen_contract"
        || contract.pid_features != "forbidden"
        || !contract
            .features
            .iter()
            .any(|definition| definition.role == H2FeatureRole::Baseline)
        || !contract
            .features
            .iter()
            .any(|definition| definition.role == H2FeatureRole::Diagnostic)
    {
        issues.push(issue(
            H2ReasonCode::InvalidDeclaration,
            None,
            None,
            None,
            "feature_contract",
            "feature contract must contain baseline and diagnostic scalar features and forbid PID",
        ));
    }
    let mut ids = BTreeSet::new();
    for definition in &contract.features {
        if !valid_identifier(&definition.feature_id)
            || definition.value_type != "finite_f64"
            || definition.missing_value_rule != "unsupported_fail_closed"
        {
            issues.push(issue(
                H2ReasonCode::InvalidDeclaration,
                None,
                None,
                None,
                "feature_contract.features",
                "feature definition violates the scalar complete-data reference contract",
            ));
        }
        if !ids.insert(definition.feature_id.as_str()) {
            issues.push(issue(
                H2ReasonCode::DuplicateId,
                None,
                None,
                None,
                "feature_contract.features",
                format!("duplicate feature id {:?}", definition.feature_id),
            ));
        }
    }
}

fn validate_splits(input: &H2ReferenceInput, limits: H2ReferenceLimits, issues: &mut Vec<H2Issue>) {
    if !valid_identifier(&input.split_manifest.manifest_id) {
        issues.push(issue(
            H2ReasonCode::InvalidIdentifierOrHash,
            None,
            None,
            None,
            "split_manifest.manifest_id",
            "split manifest id is invalid",
        ));
    }
    let episode_ids = input
        .dataset
        .episodes
        .iter()
        .map(|episode| episode.episode_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut assignment_ids = BTreeSet::new();
    let mut outer_folds = BTreeSet::new();
    let mut inner_folds = BTreeSet::new();
    for assignment in &input.split_manifest.assignments {
        if !valid_identifier(&assignment.episode_id)
            || !valid_identifier(&assignment.outer_fold)
            || !valid_identifier(&assignment.inner_fold)
        {
            issues.push(issue(
                H2ReasonCode::InvalidIdentifierOrHash,
                Some(&assignment.episode_id),
                None,
                Some(&assignment.outer_fold),
                "split_manifest.assignments",
                "assignment identifier is invalid",
            ));
        }
        if !assignment_ids.insert(assignment.episode_id.as_str()) {
            issues.push(issue(
                H2ReasonCode::EpisodeFoldLeakage,
                Some(&assignment.episode_id),
                None,
                Some(&assignment.outer_fold),
                "split_manifest.assignments",
                "episode has more than one split assignment",
            ));
        }
        outer_folds.insert(assignment.outer_fold.as_str());
        inner_folds.insert(assignment.inner_fold.as_str());
    }
    if assignment_ids != episode_ids {
        issues.push(issue(
            H2ReasonCode::EpisodeFoldLeakage,
            None,
            None,
            None,
            "split_manifest.assignments",
            "split manifest must assign every dataset episode exactly once and no others",
        ));
    }
    if outer_folds.len() < input.plan.validation.minimum_outer_folds {
        issues.push(issue(
            H2ReasonCode::InsufficientOuterFolds,
            None,
            None,
            None,
            "split_manifest.outer_fold",
            "too few outer folds",
        ));
    }
    if outer_folds.len() > limits.max_outer_folds || inner_folds.len() > limits.max_inner_folds {
        issues.push(issue(
            H2ReasonCode::ResourceLimitExceeded,
            None,
            None,
            None,
            "split_manifest",
            "outer or inner fold count exceeds the caller's bound",
        ));
    }
    let assignments = input
        .split_manifest
        .assignments
        .iter()
        .map(|assignment| (assignment.episode_id.as_str(), assignment))
        .collect::<BTreeMap<_, _>>();
    let mut world_fold = BTreeMap::<&str, &str>::new();
    let mut world_inner_fold = BTreeMap::<&str, &str>::new();
    let mut family_fold = BTreeMap::<&str, &str>::new();
    for episode in &input.dataset.episodes {
        let Some(assignment) = assignments.get(episode.episode_id.as_str()) else {
            continue;
        };
        if world_fold
            .insert(&episode.persistent_world_id, &assignment.outer_fold)
            .is_some_and(|previous| previous != assignment.outer_fold)
        {
            issues.push(issue(
                H2ReasonCode::PersistentWorldFoldLeakage,
                Some(&episode.episode_id),
                None,
                Some(&assignment.outer_fold),
                "persistent_world_id",
                "persistent world spans outer folds",
            ));
        }
        if world_inner_fold
            .insert(&episode.persistent_world_id, &assignment.inner_fold)
            .is_some_and(|previous| previous != assignment.inner_fold)
        {
            issues.push(issue(
                H2ReasonCode::PersistentWorldFoldLeakage,
                Some(&episode.episode_id),
                None,
                Some(&assignment.outer_fold),
                "persistent_world_id",
                "persistent world spans grouped inner folds",
            ));
        }
        if family_fold
            .insert(&episode.task_family_id, &assignment.outer_fold)
            .is_some_and(|previous| previous != assignment.outer_fold)
        {
            issues.push(issue(
                H2ReasonCode::TaskFamilyFoldLeakage,
                Some(&episode.episode_id),
                None,
                Some(&assignment.outer_fold),
                "task_family_id",
                "task family spans outer folds",
            ));
        }
    }
    for outer_fold in outer_folds {
        let training_inner_folds = input
            .split_manifest
            .assignments
            .iter()
            .filter(|assignment| assignment.outer_fold != outer_fold)
            .map(|assignment| assignment.inner_fold.as_str())
            .collect::<BTreeSet<_>>();
        if training_inner_folds.len() < input.plan.validation.minimum_inner_folds {
            issues.push(issue(
                H2ReasonCode::InsufficientInnerFolds,
                None,
                None,
                Some(outer_fold),
                "split_manifest.inner_fold",
                "outer training data contain too few grouped inner folds",
            ));
        }
    }
}

fn validate_episodes(input: &H2ReferenceInput, issues: &mut Vec<H2Issue>) {
    let ontology_codes = input
        .ontology
        .target_event_codes
        .iter()
        .chain(&input.ontology.competing_event_codes)
        .chain(&input.ontology.censoring_event_codes)
        .collect::<BTreeSet<_>>();
    let feature_ids = input
        .feature_contract
        .features
        .iter()
        .map(|definition| definition.feature_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut episode_ids = BTreeSet::new();
    let mut event_ids = BTreeSet::new();
    let mut landmark_ids = BTreeSet::new();
    for episode in &input.dataset.episodes {
        for (field, value) in [
            ("episode_id", episode.episode_id.as_str()),
            ("persistent_world_id", episode.persistent_world_id.as_str()),
            ("task_family_id", episode.task_family_id.as_str()),
            (
                "policy_checkpoint_id",
                episode.policy_checkpoint_id.as_str(),
            ),
            ("censoring_stratum", episode.censoring_stratum.as_str()),
        ] {
            if !valid_identifier(value) {
                issues.push(issue(
                    H2ReasonCode::InvalidIdentifierOrHash,
                    Some(&episode.episode_id),
                    None,
                    None,
                    field,
                    "episode identifier is invalid",
                ));
            }
        }
        if episode.censoring_stratum_frozen_at_ns > episode.episode_start_ns
            || !valid_sha256(&episode.censoring_stratum_source_sha256)
        {
            issues.push(issue(
                H2ReasonCode::FeatureUnavailableAtLandmark,
                Some(&episode.episode_id),
                None,
                None,
                "censoring_stratum",
                "censoring stratum must be content-addressed and frozen no later than episode start",
            ));
        }
        if !episode_ids.insert(episode.episode_id.as_str()) {
            issues.push(issue(
                H2ReasonCode::DuplicateId,
                Some(&episode.episode_id),
                None,
                None,
                "episode_id",
                "duplicate episode id",
            ));
        }
        if episode.episode_start_ns > episode.observed_until_ns
            || episode.observed_until_ns > episode.planned_observation_end_ns
        {
            issues.push(issue(
                H2ReasonCode::TimestampOrderViolation,
                Some(&episode.episode_id),
                None,
                None,
                "episode",
                "episode_start <= observed_until <= planned_observation_end is required",
            ));
        }
        for (field, event, expected_censoring) in [
            ("terminal_event", episode.terminal_event.as_ref(), false),
            ("censoring_event", episode.censoring_event.as_ref(), true),
        ] {
            if let Some(event) = event {
                if !valid_identifier(&event.event_id) || !event_ids.insert(event.event_id.as_str())
                {
                    issues.push(issue(
                        H2ReasonCode::DuplicateId,
                        Some(&episode.episode_id),
                        None,
                        None,
                        field,
                        "event id is invalid or duplicated",
                    ));
                }
                let known = ontology_codes.contains(&event.code);
                let role_matches = if expected_censoring {
                    input.ontology.censoring_event_codes.contains(&event.code)
                } else {
                    input.ontology.target_event_codes.contains(&event.code)
                        || input.ontology.competing_event_codes.contains(&event.code)
                };
                if !known || !role_matches {
                    issues.push(issue(
                        H2ReasonCode::UnknownEventCode,
                        Some(&episode.episode_id),
                        None,
                        None,
                        field,
                        format!(
                            "event code {:?} is unknown or has the wrong role",
                            event.code
                        ),
                    ));
                }
                if event.timestamp_ns <= episode.episode_start_ns
                    || event.timestamp_ns > episode.planned_observation_end_ns
                {
                    issues.push(issue(
                        H2ReasonCode::TimestampOrderViolation,
                        Some(&episode.episode_id),
                        None,
                        None,
                        field,
                        "event lies outside the episode observation plan",
                    ));
                }
            }
        }
        if episode
            .censoring_event
            .as_ref()
            .is_some_and(|event| event.timestamp_ns != episode.observed_until_ns)
        {
            issues.push(issue(
                H2ReasonCode::TimestampOrderViolation,
                Some(&episode.episode_id),
                None,
                None,
                "censoring_event.timestamp_ns",
                "an explicit censoring event must equal observed_until_ns",
            ));
        }
        if episode
            .terminal_event
            .as_ref()
            .is_some_and(|event| event.timestamp_ns > episode.observed_until_ns)
        {
            issues.push(issue(
                H2ReasonCode::TimestampOrderViolation,
                Some(&episode.episode_id),
                None,
                None,
                "terminal_event.timestamp_ns",
                "terminal event cannot occur after outcome observability ended",
            ));
        }
        let terminal_observed = episode
            .terminal_event
            .as_ref()
            .is_some_and(|event| event.timestamp_ns <= episode.observed_until_ns);
        if episode.observed_until_ns < episode.planned_observation_end_ns
            && episode.censoring_event.is_none()
            && !terminal_observed
        {
            issues.push(issue(
                H2ReasonCode::UnknownEventCode,
                Some(&episode.episode_id),
                None,
                None,
                "censoring_event",
                "early loss of observability requires an explicit frozen censoring code",
            ));
        }
        if episode
            .terminal_event
            .as_ref()
            .zip(episode.censoring_event.as_ref())
            .is_some_and(|(terminal, censoring)| terminal.timestamp_ns < censoring.timestamp_ns)
        {
            issues.push(issue(
                H2ReasonCode::TimestampOrderViolation,
                Some(&episode.episode_id),
                None,
                None,
                "events",
                "a censoring event after an observed terminal event is not a first event",
            ));
        }
        if episode
            .terminal_event
            .as_ref()
            .zip(episode.censoring_event.as_ref())
            .is_some_and(|(terminal, censoring)| terminal.timestamp_ns == censoring.timestamp_ns)
        {
            issues.push(issue(
                H2ReasonCode::AmbiguousEventTie,
                Some(&episode.episode_id),
                None,
                None,
                "events",
                "terminal and censoring first events tie",
            ));
        }
        validate_landmarks(input, episode, &feature_ids, &mut landmark_ids, issues);
    }
}

fn validate_landmarks<'a>(
    input: &H2ReferenceInput,
    episode: &'a H2Episode,
    feature_ids: &BTreeSet<&str>,
    landmark_ids: &mut BTreeSet<&'a str>,
    issues: &mut Vec<H2Issue>,
) {
    let first_observation_boundary = episode
        .terminal_event
        .as_ref()
        .map(|event| event.timestamp_ns)
        .into_iter()
        .chain(
            episode
                .censoring_event
                .as_ref()
                .map(|event| event.timestamp_ns),
        )
        .chain(std::iter::once(episode.observed_until_ns))
        .min()
        .unwrap_or(episode.observed_until_ns);
    let expected_landmarks = input
        .plan
        .landmark_schedule
        .offsets_ns
        .iter()
        .take_while(|offset| {
            episode
                .episode_start_ns
                .checked_add(**offset)
                .is_some_and(|time| time < first_observation_boundary)
        })
        .count();
    if episode.landmarks.len() != expected_landmarks
        || episode
            .landmarks
            .iter()
            .enumerate()
            .any(|(expected, landmark)| landmark.schedule_index != expected)
    {
        issues.push(issue(
            H2ReasonCode::LandmarkScheduleViolation,
            Some(&episode.episode_id),
            None,
            None,
            "landmarks",
            "episode must contain the exact observable prefix of the frozen schedule",
        ));
    }
    for landmark in &episode.landmarks {
        if !valid_identifier(&landmark.landmark_id)
            || !landmark_ids.insert(landmark.landmark_id.as_str())
        {
            issues.push(issue(
                H2ReasonCode::DuplicateId,
                Some(&episode.episode_id),
                Some(&landmark.landmark_id),
                None,
                "landmark_id",
                "landmark id is invalid or duplicated",
            ));
        }
        let expected_time = input
            .plan
            .landmark_schedule
            .offsets_ns
            .get(landmark.schedule_index)
            .and_then(|offset| episode.episode_start_ns.checked_add(*offset));
        if expected_time != Some(landmark.time_ns)
            || landmark.schedule_index >= episode.landmarks.len()
            || landmark.feature_cutoff_ns > landmark.time_ns
        {
            issues.push(issue(
                H2ReasonCode::LandmarkScheduleViolation,
                Some(&episode.episode_id),
                Some(&landmark.landmark_id),
                None,
                "landmark",
                "landmark index/time/cutoff does not match the frozen schedule",
            ));
        }
        if episode
            .terminal_event
            .as_ref()
            .is_some_and(|event| event.timestamp_ns <= landmark.time_ns)
            || episode
                .censoring_event
                .as_ref()
                .is_some_and(|event| event.timestamp_ns <= landmark.time_ns)
            || episode.observed_until_ns <= landmark.time_ns
        {
            issues.push(issue(
                H2ReasonCode::PostEventLandmark,
                Some(&episode.episode_id),
                Some(&landmark.landmark_id),
                None,
                "landmark.time_ns",
                "landmark is at or after a terminal/censoring boundary",
            ));
        }
        let row_feature_ids = landmark
            .features
            .iter()
            .map(|feature| feature.feature_id.as_str())
            .collect::<BTreeSet<_>>();
        if row_feature_ids != *feature_ids || landmark.features.len() != feature_ids.len() {
            issues.push(issue(
                H2ReasonCode::DimensionMismatch,
                Some(&episode.episode_id),
                Some(&landmark.landmark_id),
                None,
                "landmark.features",
                "landmark features must match the frozen contract exactly once each",
            ));
        }
        for feature in &landmark.features {
            if !feature.value.is_finite() {
                issues.push(issue(
                    H2ReasonCode::NonFiniteValue,
                    Some(&episode.episode_id),
                    Some(&landmark.landmark_id),
                    None,
                    "landmark.features.value",
                    "feature value must be finite",
                ));
            }
            if !valid_sha256(&feature.source_artifact_sha256) {
                issues.push(issue(
                    H2ReasonCode::InvalidIdentifierOrHash,
                    Some(&episode.episode_id),
                    Some(&landmark.landmark_id),
                    None,
                    "source_artifact_sha256",
                    "feature source hash must be canonical lowercase hexadecimal SHA-256",
                ));
            }
            if feature.source_start_ns > feature.source_end_ns
                || feature.source_end_ns > landmark.feature_cutoff_ns
            {
                issues.push(issue(
                    H2ReasonCode::FeatureAfterCutoff,
                    Some(&episode.episode_id),
                    Some(&landmark.landmark_id),
                    None,
                    "feature.source_end_ns",
                    "feature source ends after the frozen feature cutoff",
                ));
            }
            if feature.source_end_ns > feature.available_at_ns {
                issues.push(issue(
                    H2ReasonCode::FeatureUnavailableAtLandmark,
                    Some(&episode.episode_id),
                    Some(&landmark.landmark_id),
                    None,
                    "feature.available_at_ns",
                    "feature cannot be available before its source window ends",
                ));
            }
            if feature.available_at_ns > landmark.time_ns {
                issues.push(issue(
                    H2ReasonCode::FeatureUnavailableAtLandmark,
                    Some(&episode.episode_id),
                    Some(&landmark.landmark_id),
                    None,
                    "feature.available_at_ns",
                    "feature was not available at prediction time",
                ));
            }
        }
    }
}

fn derive_outcome(
    episode: &H2Episode,
    landmark: &H2Landmark,
    plan: &H2AnalysisPlan,
) -> Option<H2LandmarkOutcome> {
    let horizon_end = landmark.time_ns.checked_add(plan.estimand.horizon_ns)?;
    if horizon_end > episode.planned_observation_end_ns {
        return None;
    }
    let terminal = episode
        .terminal_event
        .as_ref()
        .filter(|event| event.timestamp_ns > landmark.time_ns && event.timestamp_ns <= horizon_end);
    let explicit_censoring_time = episode
        .censoring_event
        .as_ref()
        .map(|event| event.timestamp_ns);
    let censoring_time = explicit_censoring_time.unwrap_or(episode.observed_until_ns);
    let censoring_precludes_outcome =
        explicit_censoring_time.map_or(censoring_time < horizon_end, |time| time <= horizon_end);
    if censoring_precludes_outcome
        && terminal.is_none_or(|event| censoring_time < event.timestamp_ns)
    {
        return Some(H2LandmarkOutcome::OutcomeUnobservedCensored {
            relative_time_ns: censoring_time - landmark.time_ns,
        });
    }
    if let Some(event) = terminal {
        let relative_time_ns = event.timestamp_ns - landmark.time_ns;
        if event.code == plan.estimand.target_event_code {
            return Some(H2LandmarkOutcome::TargetEvent {
                event_id: event.event_id.clone(),
                relative_time_ns,
            });
        }
        return Some(H2LandmarkOutcome::CompetingEvent {
            event_id: event.event_id.clone(),
            relative_time_ns,
        });
    }
    if episode.observed_until_ns >= horizon_end {
        Some(H2LandmarkOutcome::EventFreeThroughHorizon)
    } else {
        Some(H2LandmarkOutcome::OutcomeUnobservedCensored {
            relative_time_ns: episode.observed_until_ns - landmark.time_ns,
        })
    }
}

fn score_outer_fold(
    input: &H2ReferenceInput,
    rows: &[LandmarkRow<'_>],
    outer_fold: &str,
    baseline_indices: &[usize],
    diagnostic_indices: &[usize],
) -> H2FoldOutcome {
    let training = rows
        .iter()
        .filter(|row| row.outer_fold != outer_fold)
        .collect::<Vec<_>>();
    let test = rows
        .iter()
        .filter(|row| row.outer_fold == outer_fold)
        .collect::<Vec<_>>();
    let mut fold_issues = Vec::new();
    if training.is_empty() || test.is_empty() {
        fold_issues.push(issue(
            H2ReasonCode::NoCommonScoringSupport,
            None,
            None,
            Some(outer_fold),
            "outer_fold",
            "outer fold has no training or test landmarks",
        ));
        return H2FoldOutcome::Abstained {
            outer_fold: outer_fold.to_string(),
            issues: fold_issues,
        };
    }
    let training_weights = match cross_fitted_training_weights(input, &training, outer_fold) {
        Ok(weights) => weights,
        Err(problem) => {
            return H2FoldOutcome::Abstained {
                outer_fold: outer_fold.to_string(),
                issues: vec![*problem],
            };
        }
    };
    let baseline_model =
        match WeightedLogisticModel::fit(input, &training, &training_weights, baseline_indices) {
            Ok(model) => model,
            Err(message) => {
                return H2FoldOutcome::Abstained {
                    outer_fold: outer_fold.to_string(),
                    issues: vec![issue(
                        H2ReasonCode::OutcomeModelFitFailed,
                        None,
                        None,
                        Some(outer_fold),
                        "outcome_model.baseline",
                        message,
                    )],
                };
            }
        };
    let diagnostic_model =
        match WeightedLogisticModel::fit(input, &training, &training_weights, diagnostic_indices) {
            Ok(model) => model,
            Err(message) => {
                return H2FoldOutcome::Abstained {
                    outer_fold: outer_fold.to_string(),
                    issues: vec![issue(
                        H2ReasonCode::OutcomeModelFitFailed,
                        None,
                        None,
                        Some(outer_fold),
                        "outcome_model.diagnostic",
                        message,
                    )],
                };
            }
        };

    let full_training_km = fit_stratified_km(&training);
    let mut predictions = Vec::with_capacity(test.len());
    let mut baseline_loss_sum = 0.0;
    let mut diagnostic_loss_sum = 0.0;
    let mut observed_loss_rows = 0_usize;
    let mut censored_landmarks = 0_usize;
    let mut weight_sum = 0.0;
    let mut weight_square_sum = 0.0;
    let mut maximum_weight = 0.0_f64;
    let test_len = test.len();
    for row in test {
        let Some(km) = full_training_km.get(row.episode.censoring_stratum.as_str()) else {
            fold_issues.push(issue(
                H2ReasonCode::CensoringStratumUnsupported,
                Some(&row.episode.episode_id),
                Some(&row.landmark.landmark_id),
                Some(outer_fold),
                "censoring_stratum",
                "outer-test censoring stratum is absent from outer training data",
            ));
            continue;
        };
        let (baseline_risk, diagnostic_risk) = match (
            baseline_model.predict_selected(&row.values, baseline_indices),
            diagnostic_model.predict_selected(&row.values, diagnostic_indices),
        ) {
            (Ok(baseline), Ok(diagnostic)) => (baseline, diagnostic),
            _ => {
                fold_issues.push(issue(
                    H2ReasonCode::OutcomeModelFitFailed,
                    Some(&row.episode.episode_id),
                    Some(&row.landmark.landmark_id),
                    Some(outer_fold),
                    "outcome_model.prediction",
                    "held-out standardization or prediction became non-finite",
                ));
                continue;
            }
        };
        let weight = match row_ipcw_weight(
            &row.outcome,
            km,
            input.plan.estimand.horizon_ns,
            input.plan.censoring.minimum_survival,
        ) {
            Ok(weight) => weight,
            Err(code) => {
                fold_issues.push(issue(
                    code,
                    Some(&row.episode.episode_id),
                    Some(&row.landmark.landmark_id),
                    Some(outer_fold),
                    "censoring.minimum_survival",
                    "required outer-test censoring survival is below the frozen floor",
                ));
                continue;
            }
        };
        let label = outcome_label(&row.outcome);
        let (baseline_weighted_loss, diagnostic_weighted_loss) = match (weight, label) {
            (Some(weight), Some(label)) => {
                let label = if label { 1.0 } else { 0.0 };
                let baseline_loss = weight * (label - baseline_risk).powi(2);
                let diagnostic_loss = weight * (label - diagnostic_risk).powi(2);
                baseline_loss_sum += baseline_loss;
                diagnostic_loss_sum += diagnostic_loss;
                observed_loss_rows += 1;
                weight_sum += weight;
                weight_square_sum += weight * weight;
                maximum_weight = maximum_weight.max(weight);
                (Some(baseline_loss), Some(diagnostic_loss))
            }
            (None, None) => {
                censored_landmarks += 1;
                (None, None)
            }
            _ => {
                fold_issues.push(issue(
                    H2ReasonCode::NoCommonScoringSupport,
                    Some(&row.episode.episode_id),
                    Some(&row.landmark.landmark_id),
                    Some(outer_fold),
                    "outcome",
                    "outcome label and IPCW observability disagreed",
                ));
                continue;
            }
        };
        predictions.push(H2PredictionRecord {
            episode_id: row.episode.episode_id.clone(),
            landmark_id: row.landmark.landmark_id.clone(),
            outer_fold: outer_fold.to_string(),
            landmark_time_ns: row.landmark.time_ns,
            outcome: row.outcome.clone(),
            baseline_risk,
            diagnostic_risk,
            ipcw_weight: weight,
            baseline_weighted_loss,
            diagnostic_weighted_loss,
        });
    }
    if !fold_issues.is_empty() || predictions.len() != test_len || observed_loss_rows == 0 {
        if fold_issues.is_empty() {
            fold_issues.push(issue(
                H2ReasonCode::NoCommonScoringSupport,
                None,
                None,
                Some(outer_fold),
                "outer_fold",
                "outer test fold has no observed IPCW loss rows",
            ));
        }
        return H2FoldOutcome::Abstained {
            outer_fold: outer_fold.to_string(),
            issues: fold_issues,
        };
    }
    let denominator = predictions.len() as f64;
    let baseline_brier = baseline_loss_sum / denominator;
    let diagnostic_brier = diagnostic_loss_sum / denominator;
    let ipcw_weight_concentration_ess_landmark_rows = if weight_square_sum > 0.0 {
        weight_sum * weight_sum / weight_square_sum
    } else {
        0.0
    };
    H2FoldOutcome::Produced {
        score: Box::new(H2FoldScore {
            outer_fold: outer_fold.to_string(),
            eligible_landmarks: predictions.len(),
            observed_loss_rows,
            censored_landmarks,
            weight_sum,
            maximum_weight,
            ipcw_weight_concentration_ess_landmark_rows,
            baseline_brier,
            diagnostic_brier,
            brier_improvement: baseline_brier - diagnostic_brier,
            baseline_model: baseline_model.receipt(input, baseline_indices),
            diagnostic_model: diagnostic_model.receipt(input, diagnostic_indices),
            predictions,
        }),
    }
}

fn fit_stratified_km<'a>(rows: &[&'a LandmarkRow<'a>]) -> BTreeMap<&'a str, ReverseKm> {
    let mut grouped = BTreeMap::<&str, Vec<&LandmarkRow<'_>>>::new();
    for row in rows {
        grouped
            .entry(row.episode.censoring_stratum.as_str())
            .or_default()
            .push(*row);
    }
    grouped
        .into_iter()
        .filter_map(|(stratum, rows)| ReverseKm::fit(&rows).map(|km| (stratum, km)))
        .collect()
}

fn cross_fitted_training_weights(
    input: &H2ReferenceInput,
    training: &[&LandmarkRow<'_>],
    outer_fold: &str,
) -> Result<Vec<Option<f64>>, Box<H2Issue>> {
    let mut cache = BTreeMap::<(String, String), ReverseKm>::new();
    let mut weights = Vec::with_capacity(training.len());
    for row in training {
        if matches!(
            row.outcome,
            H2LandmarkOutcome::OutcomeUnobservedCensored { .. }
        ) {
            weights.push(None);
            continue;
        }
        let key = (
            row.inner_fold.to_string(),
            row.episode.censoring_stratum.clone(),
        );
        if !cache.contains_key(&key) {
            let fit_rows = training
                .iter()
                .copied()
                .filter(|candidate| {
                    candidate.inner_fold != row.inner_fold
                        && candidate.episode.censoring_stratum == row.episode.censoring_stratum
                })
                .collect::<Vec<_>>();
            let Some(km) = ReverseKm::fit(&fit_rows) else {
                return Err(Box::new(issue(
                    H2ReasonCode::CensoringStratumUnsupported,
                    Some(&row.episode.episode_id),
                    Some(&row.landmark.landmark_id),
                    Some(outer_fold),
                    "censoring_stratum",
                    "inner-training censoring stratum is unsupported",
                )));
            };
            cache.insert(key.clone(), km);
        }
        let km = cache.get(&key).expect("inserted censoring model");
        let weight = row_ipcw_weight(
            &row.outcome,
            km,
            input.plan.estimand.horizon_ns,
            input.plan.censoring.minimum_survival,
        )
        .map_err(|code| {
            Box::new(issue(
                code,
                Some(&row.episode.episode_id),
                Some(&row.landmark.landmark_id),
                Some(outer_fold),
                "censoring.minimum_survival",
                "required cross-fitted training censoring survival is below the frozen floor",
            ))
        })?;
        weights.push(weight);
    }
    Ok(weights)
}

fn row_ipcw_weight(
    outcome: &H2LandmarkOutcome,
    km: &ReverseKm,
    horizon_ns: u64,
    minimum_survival: f64,
) -> Result<Option<f64>, H2ReasonCode> {
    let survival = match outcome {
        H2LandmarkOutcome::TargetEvent {
            relative_time_ns, ..
        }
        | H2LandmarkOutcome::CompetingEvent {
            relative_time_ns, ..
        } => km.left_limit(*relative_time_ns),
        H2LandmarkOutcome::EventFreeThroughHorizon => km.at(horizon_ns),
        H2LandmarkOutcome::OutcomeUnobservedCensored { .. } => return Ok(None),
    };
    if !survival.is_finite() || survival < minimum_survival {
        return Err(H2ReasonCode::CensoringSurvivalBelowFloor);
    }
    Ok(Some(1.0 / survival))
}

fn outcome_label(outcome: &H2LandmarkOutcome) -> Option<bool> {
    match outcome {
        H2LandmarkOutcome::TargetEvent { .. } => Some(true),
        H2LandmarkOutcome::CompetingEvent { .. } | H2LandmarkOutcome::EventFreeThroughHorizon => {
            Some(false)
        }
        H2LandmarkOutcome::OutcomeUnobservedCensored { .. } => None,
    }
}

#[derive(Debug)]
// Protocol-specific weighted prediction primitive. This is not a PID estimator and does not
// modify or stand in for the estimator source of truth in the pid-rs submodule.
struct WeightedLogisticModel {
    means: Vec<f64>,
    scales: Vec<f64>,
    active: Vec<bool>,
    coefficients: Vec<f64>,
    intercept: f64,
    iterations: usize,
}

impl WeightedLogisticModel {
    fn fit(
        input: &H2ReferenceInput,
        rows: &[&LandmarkRow<'_>],
        weights: &[Option<f64>],
        indices: &[usize],
    ) -> Result<Self, String> {
        if rows.len() != weights.len() || indices.is_empty() {
            return Err("row/weight dimensions are invalid".to_string());
        }
        let mut means = vec![0.0; indices.len()];
        for row in rows {
            for (column, index) in indices.iter().copied().enumerate() {
                means[column] += row.values[index];
            }
        }
        for mean in &mut means {
            *mean /= rows.len() as f64;
        }
        if means.iter().any(|mean| !mean.is_finite()) {
            return Err("training feature mean overflowed".to_string());
        }
        let mut scales = vec![0.0; indices.len()];
        for row in rows {
            for (column, index) in indices.iter().copied().enumerate() {
                scales[column] += (row.values[index] - means[column]).powi(2);
            }
        }
        for scale in &mut scales {
            *scale = (*scale / rows.len() as f64).sqrt();
        }
        if scales.iter().any(|scale| !scale.is_finite()) {
            return Err("training feature scale overflowed".to_string());
        }
        let active = scales.iter().map(|scale| *scale > 0.0).collect::<Vec<_>>();
        let active_count = active.iter().filter(|value| **value).count();
        let p = active_count + 1;
        let observed = rows
            .iter()
            .zip(weights)
            .filter_map(|(row, weight)| {
                weight
                    .zip(outcome_label(&row.outcome))
                    .map(|(weight, label)| (*row, weight, label))
            })
            .collect::<Vec<_>>();
        if observed.is_empty()
            || !observed
                .iter()
                .any(|(_, weight, label)| *weight > 0.0 && *label)
            || !observed
                .iter()
                .any(|(_, weight, label)| *weight > 0.0 && !*label)
        {
            return Err(
                "positive-weight training rows must contain both target classes".to_string(),
            );
        }
        let mut design = Vec::with_capacity(observed.len());
        for (row, weight, label) in observed {
            let values = indices
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(column, index)| {
                    active[column].then_some((row.values[index] - means[column]) / scales[column])
                })
                .collect::<Vec<_>>();
            if values.iter().any(|value| !value.is_finite()) {
                return Err("standardized training feature became non-finite".to_string());
            }
            design.push((values, weight, label));
        }
        let mut beta = vec![0.0; p];
        let cfg = &input.plan.outcome_model;
        for iteration in 0..cfg.maximum_iterations {
            let mut gradient = vec![0.0; p];
            let mut hessian = vec![vec![0.0; p]; p];
            for (values, sample_weight, label) in &design {
                let eta = beta[0]
                    + values
                        .iter()
                        .zip(beta.iter().skip(1))
                        .map(|(value, coefficient)| value * coefficient)
                        .sum::<f64>();
                let probability = sigmoid(eta);
                let residual = probability - if *label { 1.0 } else { 0.0 };
                let curvature = sample_weight * (probability * (1.0 - probability)).max(1e-12);
                let mut augmented = Vec::with_capacity(p);
                augmented.push(1.0);
                augmented.extend(values);
                for j in 0..p {
                    gradient[j] += sample_weight * residual * augmented[j];
                    for k in 0..p {
                        hessian[j][k] += curvature * augmented[j] * augmented[k];
                    }
                }
            }
            for j in 1..p {
                gradient[j] += cfg.ridge_penalty * beta[j];
                hessian[j][j] += cfg.ridge_penalty;
            }
            let delta = solve_linear(hessian, gradient)
                .ok_or_else(|| "weighted logistic Hessian is singular".to_string())?;
            let mut max_logit_change = 0.0_f64;
            for (values, _, _) in &design {
                let change = delta[0]
                    + values
                        .iter()
                        .zip(delta.iter().skip(1))
                        .map(|(value, coefficient)| value * coefficient)
                        .sum::<f64>();
                max_logit_change = max_logit_change.max(change.abs());
            }
            for (coefficient, change) in beta.iter_mut().zip(&delta) {
                *coefficient -= change;
            }
            if beta.iter().any(|value| !value.is_finite()) {
                return Err("weighted logistic update became non-finite".to_string());
            }
            if max_logit_change <= cfg.convergence_tolerance {
                let mut coefficients = vec![0.0; indices.len()];
                let mut active_coefficient = 1;
                for (column, is_active) in active.iter().copied().enumerate() {
                    if is_active {
                        coefficients[column] = beta[active_coefficient];
                        active_coefficient += 1;
                    }
                }
                return Ok(Self {
                    means,
                    scales,
                    active,
                    coefficients,
                    intercept: beta[0],
                    iterations: iteration + 1,
                });
            }
        }
        Err(
            "weighted logistic model did not converge within the frozen iteration limit"
                .to_string(),
        )
    }

    fn predict_selected(&self, all_values: &[f64], indices: &[usize]) -> Result<f64, ()> {
        let mut eta = self.intercept;
        for (column, index) in indices.iter().copied().enumerate() {
            if !self.active[column] {
                continue;
            }
            let standardized = (all_values[index] - self.means[column]) / self.scales[column];
            let contribution = self.coefficients[column] * standardized;
            if !standardized.is_finite() || !contribution.is_finite() {
                return Err(());
            }
            eta += contribution;
        }
        if !eta.is_finite() {
            return Err(());
        }
        let probability = sigmoid(eta);
        probability.is_finite().then_some(probability).ok_or(())
    }

    fn receipt(&self, input: &H2ReferenceInput, indices: &[usize]) -> H2ModelReceipt {
        H2ModelReceipt {
            feature_ids: indices
                .iter()
                .map(|index| input.feature_contract.features[*index].feature_id.clone())
                .collect(),
            dropped_zero_variance_features: indices
                .iter()
                .copied()
                .enumerate()
                .filter(|(column, _)| !self.active[*column])
                .map(|(_, index)| input.feature_contract.features[index].feature_id.clone())
                .collect(),
            means: self.means.clone(),
            scales: self.scales.clone(),
            coefficients: self.coefficients.clone(),
            intercept: self.intercept,
            iterations: self.iterations,
        }
    }
}

fn sigmoid(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exponential = value.exp();
        exponential / (1.0 + exponential)
    }
}

fn solve_linear(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Option<Vec<f64>> {
    let n = rhs.len();
    for column in 0..n {
        let pivot = (column..n).max_by(|left, right| {
            matrix[*left][column]
                .abs()
                .total_cmp(&matrix[*right][column].abs())
        })?;
        if !matrix[pivot][column].is_finite() || matrix[pivot][column].abs() <= 1e-14 {
            return None;
        }
        matrix.swap(column, pivot);
        rhs.swap(column, pivot);
        let divisor = matrix[column][column];
        for value in &mut matrix[column][column..] {
            *value /= divisor;
        }
        rhs[column] /= divisor;
        let pivot_row = matrix[column].clone();
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for offset in column..n {
                matrix[row][offset] -= factor * pivot_row[offset];
            }
            rhs[row] -= factor * rhs[column];
        }
    }
    rhs.iter().all(|value| value.is_finite()).then_some(rhs)
}

fn aggregate_scores(outcomes: &[H2FoldOutcome]) -> Option<H2AggregateScore> {
    let scores = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            H2FoldOutcome::Produced { score } => Some(score),
            H2FoldOutcome::Abstained { .. } => None,
        })
        .collect::<Vec<_>>();
    if scores.len() != outcomes.len() || scores.is_empty() {
        return None;
    }
    let eligible_landmarks = scores.iter().map(|score| score.eligible_landmarks).sum();
    if eligible_landmarks == 0 {
        return None;
    }
    let observed_loss_rows = scores.iter().map(|score| score.observed_loss_rows).sum();
    let censored_landmarks = scores.iter().map(|score| score.censored_landmarks).sum();
    let weight_sum = scores.iter().map(|score| score.weight_sum).sum::<f64>();
    let weight_square_sum = scores
        .iter()
        .map(|score| {
            if score.ipcw_weight_concentration_ess_landmark_rows > 0.0 {
                score.weight_sum.powi(2) / score.ipcw_weight_concentration_ess_landmark_rows
            } else {
                0.0
            }
        })
        .sum::<f64>();
    let baseline_loss_sum = scores
        .iter()
        .map(|score| score.baseline_brier * score.eligible_landmarks as f64)
        .sum::<f64>();
    let diagnostic_loss_sum = scores
        .iter()
        .map(|score| score.diagnostic_brier * score.eligible_landmarks as f64)
        .sum::<f64>();
    let denominator = eligible_landmarks as f64;
    let baseline_brier = baseline_loss_sum / denominator;
    let diagnostic_brier = diagnostic_loss_sum / denominator;
    Some(H2AggregateScore {
        eligible_landmarks,
        observed_loss_rows,
        censored_landmarks,
        weight_sum,
        maximum_weight: scores
            .iter()
            .map(|score| score.maximum_weight)
            .fold(0.0_f64, f64::max),
        ipcw_weight_concentration_ess_landmark_rows: if weight_square_sum > 0.0 {
            weight_sum.powi(2) / weight_square_sum
        } else {
            0.0
        },
        baseline_brier,
        diagnostic_brier,
        brier_improvement: baseline_brier - diagnostic_brier,
        precision: "not_applicable_deterministic_synthetic".to_string(),
    })
}

fn calibration_result(
    input: &H2ReferenceInput,
    predictions: &[&H2PredictionRecord],
) -> H2CalibrationResult {
    let observed = predictions
        .iter()
        .filter_map(|prediction| {
            prediction
                .ipcw_weight
                .zip(outcome_label(&prediction.outcome))
                .map(|(weight, label)| (*prediction, weight, label))
        })
        .collect::<Vec<_>>();
    let target_events = observed
        .iter()
        .filter_map(|(prediction, _, _)| match &prediction.outcome {
            H2LandmarkOutcome::TargetEvent { event_id, .. } => Some(event_id.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .len();
    let non_target_episodes = observed
        .iter()
        .filter(|(_, _, label)| !*label)
        .map(|(prediction, _, _)| prediction.episode_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    if target_events < input.plan.calibration.minimum_target_events
        || non_target_episodes < input.plan.calibration.minimum_non_target_episodes
    {
        return H2CalibrationResult::Abstained {
            reason: H2ReasonCode::CalibrationUnavailable,
        };
    }
    let first_risk = observed
        .first()
        .map(|(prediction, _, _)| prediction.diagnostic_risk);
    if first_risk.is_some_and(|first| {
        observed
            .iter()
            .all(|(prediction, _, _)| prediction.diagnostic_risk.to_bits() == first.to_bits())
    }) || observed
        .iter()
        .any(|(prediction, _, _)| !(0.0..1.0).contains(&prediction.diagnostic_risk))
    {
        return H2CalibrationResult::Abstained {
            reason: H2ReasonCode::CalibrationUnavailable,
        };
    }
    let weight_sum = observed.iter().map(|(_, weight, _)| *weight).sum::<f64>();
    let weight_square_sum = observed
        .iter()
        .map(|(_, weight, _)| weight * weight)
        .sum::<f64>();
    let effective = if weight_square_sum > 0.0 {
        weight_sum.powi(2) / weight_square_sum
    } else {
        0.0
    };
    if effective
        < input
            .plan
            .calibration
            .minimum_ipcw_weight_concentration_ess_landmark_rows
    {
        return H2CalibrationResult::Abstained {
            reason: H2ReasonCode::CalibrationUnavailable,
        };
    }
    let edges = &input.plan.calibration.reliability_bin_edges;
    let mut bins = Vec::new();
    for (index, edge) in edges.windows(2).enumerate() {
        let lower = edge[0];
        let upper = edge[1];
        let rows = observed
            .iter()
            .filter(|(prediction, _, _)| {
                prediction.diagnostic_risk >= lower
                    && if index + 2 == edges.len() {
                        prediction.diagnostic_risk <= upper
                    } else {
                        prediction.diagnostic_risk < upper
                    }
            })
            .collect::<Vec<_>>();
        if rows.is_empty() {
            continue;
        }
        let bin_weight = rows.iter().map(|(_, weight, _)| *weight).sum::<f64>();
        let bin_weight_square = rows
            .iter()
            .map(|(_, weight, _)| *weight * *weight)
            .sum::<f64>();
        let target_weight = rows
            .iter()
            .filter(|(_, _, label)| *label)
            .map(|(_, weight, _)| *weight)
            .sum::<f64>();
        let prediction_weight = rows
            .iter()
            .map(|(prediction, weight, _)| prediction.diagnostic_risk * *weight)
            .sum::<f64>();
        bins.push(H2ReliabilityBin {
            lower_inclusive: lower,
            upper_inclusive: upper,
            observed_rows: rows.len(),
            target_rows: rows.iter().filter(|(_, _, label)| *label).count(),
            weight_sum: bin_weight,
            ipcw_weight_concentration_ess_landmark_rows: bin_weight.powi(2) / bin_weight_square,
            weighted_observed_risk: target_weight / bin_weight,
            weighted_mean_prediction: prediction_weight / bin_weight,
        });
    }
    if bins.is_empty() {
        H2CalibrationResult::Abstained {
            reason: H2ReasonCode::CalibrationUnavailable,
        }
    } else {
        H2CalibrationResult::ProducedReferenceReliability { bins }
    }
}

fn alarm_result(
    input: &H2ReferenceInput,
    predictions: &[&H2PredictionRecord],
    model: H2ModelKind,
    threshold: f64,
) -> H2AlarmResult {
    if predictions.iter().any(|prediction| {
        matches!(
            prediction.outcome,
            H2LandmarkOutcome::OutcomeUnobservedCensored { .. }
        )
    }) {
        return H2AlarmResult::Abstained {
            reason: H2ReasonCode::AlarmFollowupIncomplete,
        };
    }
    let target_event_ids = predictions
        .iter()
        .filter_map(|prediction| match &prediction.outcome {
            H2LandmarkOutcome::TargetEvent { event_id, .. } => Some(event_id.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if target_event_ids.is_empty() {
        return H2AlarmResult::Abstained {
            reason: H2ReasonCode::AlarmThresholdUnavailable,
        };
    }
    let by_landmark = predictions
        .iter()
        .map(|prediction| (prediction.landmark_id.as_str(), *prediction))
        .collect::<BTreeMap<_, _>>();
    let mut alarms = Vec::new();
    let mut refractory_suppressed = 0_usize;
    let policy = &input.plan.alarm_policy;
    for episode in &input.dataset.episodes {
        let mut streak = 0_usize;
        let mut previous_score_time = None;
        let mut refractory_until = None;
        for landmark in &episode.landmarks {
            let Some(prediction) = by_landmark.get(landmark.landmark_id.as_str()) else {
                streak = 0;
                previous_score_time = None;
                continue;
            };
            let risk = match model {
                H2ModelKind::Baseline => prediction.baseline_risk,
                H2ModelKind::Diagnostic => prediction.diagnostic_risk,
            };
            if refractory_until.is_some_and(|end| landmark.time_ns < end) {
                if risk >= threshold {
                    refractory_suppressed += 1;
                }
                streak = 0;
                previous_score_time = None;
                continue;
            }
            if previous_score_time.is_some_and(|previous| {
                landmark.time_ns.saturating_sub(previous) > policy.maximum_inter_score_gap_ns
            }) {
                streak = 0;
            }
            previous_score_time = Some(landmark.time_ns);
            if risk >= threshold {
                streak += 1;
            } else {
                streak = 0;
            }
            if streak >= policy.persistence_scores {
                let alarm_id = format!("{:?}-alarm-{}", model, alarms.len()).to_ascii_lowercase();
                alarms.push(H2AlarmRecord {
                    alarm_id,
                    episode_id: episode.episode_id.clone(),
                    landmark_id: landmark.landmark_id.clone(),
                    timestamp_ns: landmark.time_ns,
                    capacity_rejected: false,
                    matched_event_id: None,
                });
                streak = 0;
                previous_score_time = None;
                refractory_until = Some(landmark.time_ns.saturating_add(policy.refractory_ns));
            }
        }
    }
    alarms.sort_by(|left, right| {
        (left.timestamp_ns, &left.alarm_id).cmp(&(right.timestamp_ns, &right.alarm_id))
    });
    let mut per_episode_executed = BTreeMap::<&str, usize>::new();
    let mut capacity_rejected = 0_usize;
    for alarm in &mut alarms {
        let executed = per_episode_executed
            .entry(alarm.episode_id.as_str())
            .or_default();
        if *executed >= input.plan.decision_utility.maximum_fallbacks_per_episode {
            alarm.capacity_rejected = true;
            capacity_rejected += 1;
        } else {
            *executed += 1;
        }
    }

    let mut lead_times = Vec::new();
    let mut matched = 0_usize;
    let mut late = 0_usize;
    let mut alarm_indices_by_episode = BTreeMap::<String, Vec<usize>>::new();
    for (index, alarm) in alarms.iter().enumerate() {
        if !alarm.capacity_rejected {
            alarm_indices_by_episode
                .entry(alarm.episode_id.clone())
                .or_default()
                .push(index);
        }
    }
    for episode in &input.dataset.episodes {
        let Some(event) = episode.terminal_event.as_ref().filter(|event| {
            input.ontology.target_event_codes.contains(&event.code)
                && target_event_ids.contains(event.event_id.as_str())
        }) else {
            continue;
        };
        let candidates = alarm_indices_by_episode
            .get(&episode.episode_id)
            .into_iter()
            .flatten()
            .filter_map(|index| {
                event
                    .timestamp_ns
                    .checked_sub(alarms[*index].timestamp_ns)
                    .map(|lead| (*index, lead))
            })
            .collect::<Vec<_>>();
        let selected = candidates.iter().position(|(_, lead)| {
            *lead >= policy.minimum_actionable_lead_ns && *lead <= policy.maximum_lookback_ns
        });
        late += candidates
            .iter()
            .filter(|(_, lead)| *lead < policy.minimum_actionable_lead_ns)
            .count();
        if let Some(index) = selected {
            let (alarm_index, lead) = candidates[index];
            alarms[alarm_index].matched_event_id = Some(event.event_id.clone());
            matched += 1;
            lead_times.push(H2LeadTimeRecord::Detected {
                event_id: event.event_id.clone(),
                lead_time_ns: lead,
            });
        } else {
            lead_times.push(H2LeadTimeRecord::Undetected {
                event_id: event.event_id.clone(),
                reason: "no_actionable_alarm".to_string(),
            });
        }
    }
    let target_events = lead_times.len();
    let detected_events = matched;
    let undetected_events = target_events.saturating_sub(detected_events);
    let executed = alarms
        .iter()
        .filter(|alarm| !alarm.capacity_rejected)
        .count();
    let unmatched = executed.saturating_sub(matched);
    let utility_plan = &input.plan.decision_utility;
    let utility_total = utility_plan.actionable_detection_value * matched as f64
        - utility_plan.missed_target_cost * undetected_events as f64
        - utility_plan.alarm_action_cost * executed as f64
        - utility_plan.false_alarm_cost * unmatched as f64
        - utility_plan.capacity_rejection_cost * capacity_rejected as f64;
    let evaluable_episodes = predictions
        .iter()
        .map(|prediction| prediction.episode_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let detection_curve = policy
        .lead_time_cutoffs_ns
        .iter()
        .copied()
        .map(|minimum_lead_ns| {
            let detected = lead_times
                .iter()
                .filter(|record| {
                    matches!(
                        record,
                        H2LeadTimeRecord::Detected { lead_time_ns, .. }
                            if *lead_time_ns >= minimum_lead_ns
                    )
                })
                .count();
            H2DetectionCurvePoint {
                minimum_lead_ns,
                detected_events: detected,
                total_target_events: target_events,
                fraction: if target_events > 0 {
                    detected as f64 / target_events as f64
                } else {
                    0.0
                },
            }
        })
        .collect();
    H2AlarmResult::Produced {
        summary: H2AlarmSummary {
            model,
            threshold,
            alarms_emitted: alarms.len(),
            alarms_executed: executed,
            alarms_matched: matched,
            alarms_unmatched: unmatched,
            alarms_late: late,
            refractory_suppressed,
            capacity_rejected,
            target_events,
            detected_events,
            undetected_events,
            lead_times,
            detection_curve,
            alarms,
            assumed_payoff_utility_per_evaluable_episode: utility_total / evaluable_episodes as f64,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(name: &str) -> H2ArtifactBinding {
        H2ArtifactBinding {
            artifact_uri: format!("fixtures/{name}.json"),
            sha256: "a".repeat(64),
        }
    }

    fn reference_input() -> H2ReferenceInput {
        let plan = H2AnalysisPlan {
            schema_version: H2_REFERENCE_SCHEMA_VERSION,
            estimand: H2Estimand {
                kind: "fixed_horizon_target_cumulative_incidence".to_string(),
                target_event_code: "named_failure_v1".to_string(),
                horizon_ns: 10,
                risk_set: "event_and_censor_free_at_landmark".to_string(),
                interval: "open_left_closed_right".to_string(),
                landmark_weighting: "uniform_eligible_scheduled_landmarks".to_string(),
            },
            landmark_schedule: H2LandmarkSchedule {
                kind: "fixed_offsets_from_episode_start".to_string(),
                offsets_ns: vec![10, 20, 30],
                minimum_history_ns: 10,
            },
            validation: H2ValidationPlan {
                outer_split: "grouped_task_family_k_fold".to_string(),
                inner_split: "grouped_k_fold".to_string(),
                group_keys: vec!["episode_id".to_string(), "persistent_world_id".to_string()],
                minimum_outer_folds: 2,
                minimum_inner_folds: 2,
            },
            outcome_model: H2OutcomeModelPlan {
                family: "deterministic_weighted_l2_logistic".to_string(),
                ridge_penalty: 1.0,
                intercept_penalized: false,
                standardization: "outer_training_mean_sd".to_string(),
                zero_variance_rule: "drop_and_report".to_string(),
                maximum_iterations: 200,
                convergence_tolerance: 1e-9,
            },
            censoring: H2CensoringPlan {
                model: "reverse_kaplan_meier_by_frozen_stratum".to_string(),
                assumption: "independent_given_prelandmark_stratum".to_string(),
                event_weight_time: "left_limit".to_string(),
                event_free_weight_time: "horizon".to_string(),
                censor_at_horizon: "outcome_unobserved_censored".to_string(),
                minimum_survival: 0.05,
                weight_clipping: "forbidden".to_string(),
                aggregate: "horvitz_thompson_over_all_eligible_landmarks".to_string(),
            },
            calibration: H2CalibrationPlan {
                reliability_bin_edges: vec![0.0, 0.5, 1.0],
                minimum_target_events: 2,
                minimum_non_target_episodes: 2,
                minimum_ipcw_weight_concentration_ess_landmark_rows: 2.0,
            },
            alarm_policy: H2AlarmPolicy {
                baseline_threshold: 0.99,
                diagnostic_threshold: 0.99,
                comparison: "risk_greater_than_or_equal_to_threshold".to_string(),
                persistence_scores: 1,
                maximum_inter_score_gap_ns: 10,
                missing_score_rule: "break_streak_and_emit_no_alarm".to_string(),
                after_alarm_rule: "clear_streak".to_string(),
                refractory_ns: 10,
                episode_reset_rule: "clear_all_state".to_string(),
                minimum_actionable_lead_ns: 3,
                maximum_lookback_ns: 10,
                match_choice: "earliest_actionable_alarm".to_string(),
                lead_time_cutoffs_ns: vec![3, 5, 10],
            },
            decision_utility: H2DecisionUtilityPlan {
                kind: "declared_warning_payoff_scenario".to_string(),
                actionable_detection_value: 5.0,
                missed_target_cost: 2.0,
                alarm_action_cost: 0.1,
                false_alarm_cost: 1.0,
                capacity_rejection_cost: 0.2,
                maximum_fallbacks_per_episode: 3,
                capacity_priority: "alarm_time_then_alarm_id".to_string(),
                intervention_latency_ns: 3,
                normalization: "per_evaluable_episode".to_string(),
            },
            claim_boundary: H2ClaimBoundary {
                synthetic_fixture_only: true,
                establishes_h2_evidence: false,
                prospective_capture: false,
                external_validation: false,
                comparator_frontier_complete: false,
                pid_dependency: "none".to_string(),
            },
        };
        let ontology = H2EventOntology {
            schema_version: H2_REFERENCE_SCHEMA_VERSION,
            ontology_id: "synthetic_h2_ontology_v1".to_string(),
            target_event_codes: vec!["named_failure_v1".to_string()],
            competing_event_codes: vec!["terminal_success".to_string()],
            censoring_event_codes: vec!["outcome_observation_lost".to_string()],
            simultaneous_first_event_rule: "reject_as_ambiguous".to_string(),
        };
        let feature_contract = H2FeatureContract {
            schema_version: H2_REFERENCE_SCHEMA_VERSION,
            contract_id: "synthetic_h2_features_v1".to_string(),
            features: vec![
                H2FeatureDefinition {
                    feature_id: "design_progress".to_string(),
                    role: H2FeatureRole::Baseline,
                    value_type: "finite_f64".to_string(),
                    missing_value_rule: "unsupported_fail_closed".to_string(),
                },
                H2FeatureDefinition {
                    feature_id: "diagnostic_signal".to_string(),
                    role: H2FeatureRole::Diagnostic,
                    value_type: "finite_f64".to_string(),
                    missing_value_rule: "unsupported_fail_closed".to_string(),
                },
            ],
            categorical_encoding: "preencoded_by_frozen_contract".to_string(),
            pid_features: "forbidden".to_string(),
        };
        let mut episodes = Vec::new();
        let mut assignments = Vec::new();
        for index in 0..8 {
            let is_target = matches!(index, 0 | 4);
            let is_competing = matches!(index, 1 | 5);
            let terminal_event = if is_target {
                Some(H2ObservedEvent {
                    event_id: format!("target-{index}"),
                    code: "named_failure_v1".to_string(),
                    timestamp_ns: 35,
                })
            } else if is_competing {
                Some(H2ObservedEvent {
                    event_id: format!("competing-{index}"),
                    code: "terminal_success".to_string(),
                    timestamp_ns: 35,
                })
            } else {
                None
            };
            let landmarks = [10_u64, 20, 30]
                .into_iter()
                .enumerate()
                .map(|(schedule_index, time_ns)| {
                    let diagnostic = if is_target {
                        [-0.2, 0.4, 1.2][schedule_index]
                    } else {
                        [-0.8, -0.6, -0.4][schedule_index]
                    };
                    H2Landmark {
                        landmark_id: format!("episode-{index}-landmark-{schedule_index}"),
                        schedule_index,
                        time_ns,
                        feature_cutoff_ns: time_ns,
                        features: vec![
                            H2FeatureValue {
                                feature_id: "design_progress".to_string(),
                                value: schedule_index as f64 + (index % 2) as f64 * 0.1,
                                source_start_ns: 0,
                                source_end_ns: time_ns,
                                available_at_ns: time_ns,
                                source_artifact_sha256: "b".repeat(64),
                            },
                            H2FeatureValue {
                                feature_id: "diagnostic_signal".to_string(),
                                value: diagnostic,
                                source_start_ns: 0,
                                source_end_ns: time_ns,
                                available_at_ns: time_ns,
                                source_artifact_sha256: "c".repeat(64),
                            },
                        ],
                    }
                })
                .collect();
            episodes.push(H2Episode {
                episode_id: format!("episode-{index}"),
                persistent_world_id: format!("world-{index}"),
                task_family_id: if index < 4 { "family-a" } else { "family-b" }.to_string(),
                policy_checkpoint_id: "policy-v1".to_string(),
                censoring_stratum: "stratum-a".to_string(),
                censoring_stratum_frozen_at_ns: 0,
                censoring_stratum_source_sha256: "d".repeat(64),
                episode_start_ns: 0,
                planned_observation_end_ns: 50,
                observed_until_ns: 50,
                terminal_event,
                censoring_event: None,
                landmarks,
            });
            assignments.push(H2SplitAssignment {
                episode_id: format!("episode-{index}"),
                outer_fold: if index < 4 { "outer-a" } else { "outer-b" }.to_string(),
                inner_fold: if index % 2 == 0 { "inner-a" } else { "inner-b" }.to_string(),
            });
        }
        H2ReferenceInput {
            dataset: H2Dataset {
                schema_version: H2_REFERENCE_SCHEMA_VERSION,
                scope: H2ReferenceScope::DeterministicSyntheticFiniteLandmarkBenchmark,
                bindings: H2ArtifactBindings {
                    analysis_plan: binding("analysis-plan"),
                    event_ontology: binding("event-ontology"),
                    feature_contract: binding("feature-contract"),
                    split_manifest: binding("split-manifest"),
                },
                episodes,
            },
            plan,
            ontology,
            feature_contract,
            split_manifest: H2SplitManifest {
                schema_version: H2_REFERENCE_SCHEMA_VERSION,
                manifest_id: "synthetic_h2_splits_v1".to_string(),
                assignments,
            },
        }
    }

    fn evolving_risk_censored_input(
        hidden_censored_rows_are_targets: bool,
    ) -> (H2ReferenceInput, f64) {
        let mut input = reference_input();
        input.plan.landmark_schedule.offsets_ns.truncate(1);
        for (index, episode) in input.dataset.episodes.iter_mut().enumerate() {
            episode.planned_observation_end_ns = 20;
            episode.observed_until_ns = 20;
            episode.landmarks.truncate(1);
            if let Some(event) = &mut episode.terminal_event {
                event.timestamp_ns = 15;
            }
            let diagnostic_signal = if matches!(index, 2 | 6) {
                2.0
            } else if matches!(index, 0 | 4) {
                1.0
            } else {
                -1.0
            };
            episode.landmarks[0]
                .features
                .iter_mut()
                .find(|feature| feature.feature_id == "diagnostic_signal")
                .expect("reference diagnostic feature")
                .value = diagnostic_signal;
            if matches!(index, 2 | 6) {
                episode.observed_until_ns = 15;
                episode.censoring_event = Some(H2ObservedEvent {
                    event_id: format!("censor-{index}"),
                    code: "outcome_observation_lost".to_string(),
                    timestamp_ns: 15,
                });
            }
        }
        let hidden_target_count = usize::from(hidden_censored_rows_are_targets) * 2;
        (input, (2 + hidden_target_count) as f64 / 8.0)
    }

    fn manual_prediction(
        landmark_id: &str,
        outcome: H2LandmarkOutcome,
        baseline_risk: f64,
        diagnostic_risk: f64,
    ) -> H2PredictionRecord {
        H2PredictionRecord {
            episode_id: format!("episode-{landmark_id}"),
            landmark_id: landmark_id.to_string(),
            outer_fold: "outer-a".to_string(),
            landmark_time_ns: 10,
            outcome,
            baseline_risk,
            diagnostic_risk,
            ipcw_weight: None,
            baseline_weighted_loss: None,
            diagnostic_weighted_loss: None,
        }
    }

    fn identify_manual_predictions(
        predictions: &[H2PredictionRecord],
        eligible_landmarks: usize,
        outer_folds_abstained: usize,
    ) -> H2FixedPredictionBrierImprovementIdentification {
        let mut denominators = H2Denominators {
            eligible_landmarks,
            outer_folds_abstained,
            ..H2Denominators::default()
        };
        for prediction in predictions {
            match &prediction.outcome {
                H2LandmarkOutcome::TargetEvent { .. } => {
                    denominators.target_event_outcomes += 1;
                }
                H2LandmarkOutcome::CompetingEvent { .. } => {
                    denominators.competing_event_outcomes += 1;
                }
                H2LandmarkOutcome::EventFreeThroughHorizon => {
                    denominators.event_free_outcomes += 1;
                }
                H2LandmarkOutcome::OutcomeUnobservedCensored { .. } => {
                    denominators.censored_outcomes += 1;
                }
            }
        }
        let predictions = predictions.iter().collect::<Vec<_>>();
        fixed_prediction_paired_brier_improvement_identification(&predictions, &denominators)
    }

    #[test]
    fn complete_followup_produces_grouped_scores_and_retains_undetected_events() {
        let report = run_h2_reference(&reference_input());
        assert!(report.is_valid(), "issues: {:?}", report.issues);
        assert_eq!(report.schema_version, H2_REFERENCE_REPORT_SCHEMA_VERSION);
        assert_eq!(report.denominators.outer_folds_produced, 2);
        assert_eq!(report.denominators.unique_target_events, 2);
        let aggregate = report.aggregate_score.as_ref().expect("aggregate score");
        assert!(aggregate.ipcw_weight_concentration_ess_landmark_rows > 0.0);
        assert!(
            aggregate.ipcw_weight_concentration_ess_landmark_rows
                <= aggregate.observed_loss_rows as f64
        );
        assert!(matches!(
            report.diagnostic_calibration,
            H2CalibrationResult::ProducedReferenceReliability { .. }
        ));
        let H2AlarmResult::Produced { summary } = &report.alarm_results[&H2ModelKind::Diagnostic]
        else {
            panic!("complete follow-up alarm result should produce");
        };
        assert_eq!(summary.target_events, 2);
        assert_eq!(summary.lead_times.len(), 2);
        assert!(summary
            .lead_times
            .iter()
            .all(|record| matches!(record, H2LeadTimeRecord::Undetected { .. })));
        assert!(summary
            .detection_curve
            .iter()
            .all(|point| point.total_target_events == 2));

        let serialized = serde_json::to_string(&report).expect("serialize H2 report");
        assert!(serialized.contains("\"ipcw_weight_concentration_ess_landmark_rows\""));
        assert!(!serialized.contains("\"effective_sample_size\""));
        let identification = &report.fixed_prediction_paired_brier_improvement_identification;
        assert_eq!(
            identification.conditioning,
            H2FixedPredictionConditioning::AlreadyFittedOutOfFoldPredictionsHeldFixed
        );
        assert!(!identification.removes_censoring_assumptions_used_during_model_fitting);
        assert!(!identification.validates_ipcw);
        assert!(!identification.prospective_evidence);
        assert!(matches!(
            &identification.result,
            H2FixedPredictionBrierImprovementResult::ObservedFiniteBenchmarkPoint {
                eligible_landmark_rows: 24,
                observed_outcome_rows: 24,
                censored_outcome_rows: 0,
                ..
            }
        ));
    }

    #[test]
    fn calibration_weight_concentration_threshold_retains_the_schema_v1_wire_key() {
        let calibration = serde_json::to_value(reference_input().plan.calibration)
            .expect("serialize H2 calibration plan");
        assert_eq!(
            calibration.get("minimum_effective_landmarks"),
            Some(&serde_json::json!(2.0))
        );
        assert!(calibration
            .get("minimum_ipcw_weight_concentration_ess_landmark_rows")
            .is_none());
    }

    #[test]
    fn evolving_risk_dependent_censoring_is_typed_as_not_point_identified() {
        let (low_risk_world, low_risk_truth) = evolving_risk_censored_input(false);
        let (high_risk_world, high_risk_truth) = evolving_risk_censored_input(true);
        assert_eq!(
            low_risk_world, high_risk_world,
            "latent outcomes after censoring must not alter the observed artifact"
        );
        assert!(low_risk_world.dataset.episodes.iter().all(|episode| {
            let diagnostic_signal = episode.landmarks[0]
                .features
                .iter()
                .find(|feature| feature.feature_id == "diagnostic_signal")
                .expect("reference diagnostic feature")
                .value;
            episode.censoring_event.is_some() == (diagnostic_signal > 1.5)
        }));

        let low_risk_report = run_h2_reference(&low_risk_world);
        let high_risk_report = run_h2_reference(&high_risk_world);
        assert!(
            low_risk_report.is_valid(),
            "issues: {:?}",
            low_risk_report.issues
        );
        assert_eq!(low_risk_report, high_risk_report);
        assert!(!low_risk_report.censoring_assumption_validated);
        assert_eq!(
            low_risk_report.target_risk_identification_without_censoring_assumption,
            H2TargetRiskIdentification::NotPointIdentifiedNoAssumptionBounds {
                lower_target_risk: 0.25,
                upper_target_risk: 0.5,
            }
        );
        assert_eq!((low_risk_truth, high_risk_truth), (0.25, 0.5));
        assert!(matches!(
            low_risk_report
                .fixed_prediction_paired_brier_improvement_identification
                .result,
            H2FixedPredictionBrierImprovementResult::NotPointIdentifiedConservativeMissingOutcomeBounds {
                eligible_landmark_rows: 8,
                observed_outcome_rows: 6,
                censored_outcome_rows: 2,
                ..
            }
        ));
    }

    #[test]
    fn fixed_prediction_brier_improvement_point_has_the_declared_orientation() {
        let predictions = vec![
            manual_prediction(
                "target",
                H2LandmarkOutcome::TargetEvent {
                    event_id: "target".to_string(),
                    relative_time_ns: 5,
                },
                0.2,
                0.8,
            ),
            manual_prediction(
                "non-target",
                H2LandmarkOutcome::EventFreeThroughHorizon,
                0.8,
                0.2,
            ),
        ];
        let identification = identify_manual_predictions(&predictions, 2, 0);
        let H2FixedPredictionBrierImprovementResult::ObservedFiniteBenchmarkPoint {
            paired_brier_improvement,
            eligible_landmark_rows,
            observed_outcome_rows,
            censored_outcome_rows,
        } = identification.result
        else {
            panic!("complete outcomes should point-identify the paired improvement");
        };
        assert!((paired_brier_improvement - 0.6).abs() < 1e-12);
        assert_eq!(
            (
                eligible_landmark_rows,
                observed_outcome_rows,
                censored_outcome_rows
            ),
            (2, 2, 0)
        );

        let reversed = predictions
            .iter()
            .cloned()
            .map(|mut prediction| {
                std::mem::swap(
                    &mut prediction.baseline_risk,
                    &mut prediction.diagnostic_risk,
                );
                prediction
            })
            .collect::<Vec<_>>();
        let H2FixedPredictionBrierImprovementResult::ObservedFiniteBenchmarkPoint {
            paired_brier_improvement: reversed_improvement,
            ..
        } = identify_manual_predictions(&reversed, 2, 0).result
        else {
            panic!("reversed complete outcomes should remain point identified");
        };
        assert!((reversed_improvement + 0.6).abs() < 1e-12);
    }

    #[test]
    fn fixed_prediction_missing_outcome_bounds_are_hand_calculated_and_order_stable() {
        let observed = manual_prediction(
            "observed",
            H2LandmarkOutcome::TargetEvent {
                event_id: "target".to_string(),
                relative_time_ns: 5,
            },
            0.2,
            0.8,
        );
        let censored = manual_prediction(
            "censored",
            H2LandmarkOutcome::OutcomeUnobservedCensored {
                relative_time_ns: 5,
            },
            0.2,
            0.8,
        );
        let forward = identify_manual_predictions(&[observed.clone(), censored.clone()], 2, 0);
        let reverse = identify_manual_predictions(&[censored, observed], 2, 0);
        assert_eq!(
            forward, reverse,
            "stable row identity fixes summation order"
        );
        let H2FixedPredictionBrierImprovementResult::NotPointIdentifiedConservativeMissingOutcomeBounds {
            lower_paired_brier_improvement,
            upper_paired_brier_improvement,
            eligible_landmark_rows,
            observed_outcome_rows,
            censored_outcome_rows,
        } = forward.result
        else {
            panic!("one missing binary outcome should produce bounds");
        };
        assert!(lower_paired_brier_improvement.abs() < 1e-12);
        assert!((upper_paired_brier_improvement - 0.6).abs() < 1e-12);
        assert!(lower_paired_brier_improvement <= upper_paired_brier_improvement);
        assert_eq!(
            (
                eligible_landmark_rows,
                observed_outcome_rows,
                censored_outcome_rows
            ),
            (2, 1, 1)
        );
    }

    #[test]
    fn adversarial_fixed_prediction_bounds_remain_finite_and_inside_brier_range() {
        let predictions = vec![
            manual_prediction(
                "extreme-a",
                H2LandmarkOutcome::OutcomeUnobservedCensored {
                    relative_time_ns: 1,
                },
                0.0,
                1.0,
            ),
            manual_prediction(
                "extreme-b",
                H2LandmarkOutcome::OutcomeUnobservedCensored {
                    relative_time_ns: 1,
                },
                1.0,
                0.0,
            ),
        ];
        let H2FixedPredictionBrierImprovementResult::NotPointIdentifiedConservativeMissingOutcomeBounds {
            lower_paired_brier_improvement,
            upper_paired_brier_improvement,
            ..
        } = identify_manual_predictions(&predictions, 2, 0).result
        else {
            panic!("extreme missing outcomes should produce finite bounds");
        };
        assert_eq!(lower_paired_brier_improvement, -1.0);
        assert_eq!(upper_paired_brier_improvement, 1.0);
        assert!(lower_paired_brier_improvement.is_finite());
        assert!(upper_paired_brier_improvement.is_finite());
        assert!((-1.0..=1.0).contains(&lower_paired_brier_improvement));
        assert!((-1.0..=1.0).contains(&upper_paired_brier_improvement));
    }

    #[test]
    fn fixed_prediction_identification_abstains_without_a_complete_valid_surface() {
        assert!(matches!(
            identify_manual_predictions(&[], 0, 0).result,
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput
        ));
        let prediction = manual_prediction(
            "only-row",
            H2LandmarkOutcome::EventFreeThroughHorizon,
            0.5,
            0.5,
        );
        assert!(matches!(
            identify_manual_predictions(std::slice::from_ref(&prediction), 2, 0).result,
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput
        ));
        assert!(matches!(
            identify_manual_predictions(std::slice::from_ref(&prediction), 1, 1).result,
            H2FixedPredictionBrierImprovementResult::UnavailableFoldAbstention
        ));
        assert!(matches!(
            identify_manual_predictions(&[prediction.clone(), prediction.clone()], 2, 0).result,
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput
        ));
        let mut cross_fold_duplicate = prediction.clone();
        cross_fold_duplicate.outer_fold = "outer-b".to_string();
        assert!(matches!(
            identify_manual_predictions(&[prediction.clone(), cross_fold_duplicate], 2, 0).result,
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput
        ));
        let mut invalid_prediction = prediction;
        invalid_prediction.baseline_risk = 1.01;
        assert!(matches!(
            identify_manual_predictions(&[invalid_prediction], 1, 0).result,
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput
        ));
    }

    #[test]
    fn censoring_keeps_landmark_in_ht_denominator_without_numeric_loss() {
        let mut input = reference_input();
        let episode = &mut input.dataset.episodes[2];
        episode.observed_until_ns = 25;
        episode.censoring_event = Some(H2ObservedEvent {
            event_id: "censor-2".to_string(),
            code: "outcome_observation_lost".to_string(),
            timestamp_ns: 25,
        });
        episode.landmarks.truncate(2);
        let report = run_h2_reference(&input);
        assert!(report.is_valid(), "issues: {:?}", report.issues);
        assert_eq!(report.denominators.censored_outcomes, 1);
        let censored = report
            .fold_outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                H2FoldOutcome::Produced { score } => Some(&score.predictions),
                H2FoldOutcome::Abstained { .. } => None,
            })
            .flatten()
            .find(|prediction| {
                matches!(
                    prediction.outcome,
                    H2LandmarkOutcome::OutcomeUnobservedCensored { .. }
                )
            })
            .expect("censored landmark prediction");
        assert!(censored.ipcw_weight.is_none());
        assert!(censored.baseline_weighted_loss.is_none());
        assert!(censored.diagnostic_weighted_loss.is_none());
        let produced_predictions = report
            .fold_outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                H2FoldOutcome::Produced { score } => Some(score.predictions.as_slice()),
                H2FoldOutcome::Abstained { .. } => None,
            })
            .flatten()
            .collect::<Vec<_>>();
        for episode_id in ["episode-4", "episode-5"] {
            let weight = produced_predictions
                .iter()
                .find(|prediction| {
                    prediction.episode_id == episode_id
                        && matches!(
                            prediction.outcome,
                            H2LandmarkOutcome::TargetEvent { .. }
                                | H2LandmarkOutcome::CompetingEvent { .. }
                        )
                })
                .and_then(|prediction| prediction.ipcw_weight)
                .expect("target/competing event weight");
            assert_eq!(weight, 1.0, "G(u-) excludes the censoring tie");
        }
        for episode_id in ["episode-6", "episode-7"] {
            let weight = produced_predictions
                .iter()
                .find(|prediction| prediction.episode_id == episode_id)
                .and_then(|prediction| prediction.ipcw_weight)
                .expect("event-free weight");
            assert!((weight - 1.1).abs() < 1e-12, "event-free row uses G(h)");
        }
        assert!(matches!(
            report.alarm_results[&H2ModelKind::Diagnostic],
            H2AlarmResult::Abstained {
                reason: H2ReasonCode::AlarmFollowupIncomplete
            }
        ));
    }

    #[test]
    fn administrative_completion_and_explicit_censoring_differ_at_horizon() {
        let input = reference_input();
        let mut episode = input.dataset.episodes[3].clone();
        episode.planned_observation_end_ns = 40;
        episode.observed_until_ns = 40;
        let landmark = episode.landmarks.last().expect("final landmark").clone();
        assert!(matches!(
            derive_outcome(&episode, &landmark, &input.plan),
            Some(H2LandmarkOutcome::EventFreeThroughHorizon)
        ));

        episode.planned_observation_end_ns = 50;
        episode.censoring_event = Some(H2ObservedEvent {
            event_id: "censor-at-horizon".to_string(),
            code: "outcome_observation_lost".to_string(),
            timestamp_ns: 40,
        });
        assert!(matches!(
            derive_outcome(&episode, &landmark, &input.plan),
            Some(H2LandmarkOutcome::OutcomeUnobservedCensored {
                relative_time_ns: 10
            })
        ));
    }

    #[test]
    fn future_feature_fails_before_scoring() {
        let mut input = reference_input();
        input.dataset.episodes[0].landmarks[0].features[1].source_end_ns = 11;
        let report = run_h2_reference(&input);
        assert!(!report.is_valid());
        assert!(report.aggregate_score.is_none());
        assert!(report
            .issues
            .iter()
            .any(|problem| problem.code == H2ReasonCode::FeatureAfterCutoff));
        assert!(report.fold_outcomes.is_empty());
        assert!(matches!(
            report
                .fixed_prediction_paired_brier_improvement_identification
                .result,
            H2FixedPredictionBrierImprovementResult::UnavailableInvalidInput
        ));
    }

    #[test]
    fn competing_event_is_observed_zero_not_censoring() {
        let report = run_h2_reference(&reference_input());
        let competing = report
            .fold_outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                H2FoldOutcome::Produced { score } => Some(&score.predictions),
                H2FoldOutcome::Abstained { .. } => None,
            })
            .flatten()
            .find(|prediction| {
                matches!(prediction.outcome, H2LandmarkOutcome::CompetingEvent { .. })
            })
            .expect("competing-event landmark");
        assert!(competing.ipcw_weight.is_some());
        assert!(competing.baseline_weighted_loss.is_some());
    }

    #[test]
    fn silently_truncated_observable_schedule_is_rejected() {
        let mut input = reference_input();
        input.dataset.episodes[3].landmarks.pop();
        let report = run_h2_reference(&input);
        assert!(report
            .issues
            .iter()
            .any(|problem| problem.code == H2ReasonCode::LandmarkScheduleViolation));
        assert!(report.aggregate_score.is_none());
    }

    #[test]
    fn persistent_world_must_not_cross_inner_folds() {
        let mut input = reference_input();
        input.dataset.episodes[1].persistent_world_id = "world-0".to_string();
        let report = run_h2_reference(&input);
        assert!(report
            .issues
            .iter()
            .any(|problem| problem.code == H2ReasonCode::PersistentWorldFoldLeakage));
    }

    #[test]
    fn extreme_finite_features_fail_before_numeric_output() {
        let mut input = reference_input();
        input.dataset.episodes[0].landmarks[0].features[0].value = f64::MAX;
        input.dataset.episodes[1].landmarks[0].features[0].value = -f64::MAX;
        let report = run_h2_reference(&input);
        assert!(!report.is_valid());
        assert!(report.aggregate_score.is_none());
        assert!(report
            .issues
            .iter()
            .any(|problem| problem.code == H2ReasonCode::OutcomeModelFitFailed));
        assert!(matches!(
            report
                .fixed_prediction_paired_brier_improvement_identification
                .result,
            H2FixedPredictionBrierImprovementResult::UnavailableFoldAbstention
        ));
    }

    #[test]
    fn alarm_engine_exercises_persistence_refractory_capacity_matching_and_nondetection() {
        let mut input = reference_input();
        input.plan.alarm_policy.baseline_threshold = 0.5;
        input.plan.alarm_policy.diagnostic_threshold = 0.5;
        input.plan.alarm_policy.persistence_scores = 2;
        input.plan.alarm_policy.refractory_ns = 15;
        input.plan.decision_utility.maximum_fallbacks_per_episode = 1;
        for (index, time_ns) in [(3_usize, 40_u64), (4, 50)] {
            input.dataset.episodes[2].landmarks.push(H2Landmark {
                landmark_id: format!("episode-2-alarm-landmark-{index}"),
                schedule_index: index,
                time_ns,
                feature_cutoff_ns: time_ns,
                features: Vec::new(),
            });
        }
        let mut predictions = Vec::new();
        for episode_index in [0_usize, 4, 2] {
            let episode = &input.dataset.episodes[episode_index];
            for (landmark_index, landmark) in episode.landmarks.iter().enumerate() {
                let risk = match episode_index {
                    0 if landmark_index == 0 => 0.1,
                    0 | 2 => 0.8,
                    _ => 0.1,
                };
                let outcome = if matches!(episode_index, 0 | 4)
                    && landmark_index + 1 == episode.landmarks.len()
                {
                    H2LandmarkOutcome::TargetEvent {
                        event_id: format!("target-{episode_index}"),
                        relative_time_ns: 5,
                    }
                } else {
                    H2LandmarkOutcome::EventFreeThroughHorizon
                };
                predictions.push(H2PredictionRecord {
                    episode_id: episode.episode_id.clone(),
                    landmark_id: landmark.landmark_id.clone(),
                    outer_fold: if episode_index < 4 {
                        "outer-a"
                    } else {
                        "outer-b"
                    }
                    .to_string(),
                    landmark_time_ns: landmark.time_ns,
                    outcome,
                    baseline_risk: risk,
                    diagnostic_risk: risk,
                    ipcw_weight: Some(1.0),
                    baseline_weighted_loss: Some(0.0),
                    diagnostic_weighted_loss: Some(0.0),
                });
            }
        }
        let prediction_refs = predictions.iter().collect::<Vec<_>>();
        let H2AlarmResult::Produced { summary } = alarm_result(
            &input,
            &prediction_refs,
            H2ModelKind::Diagnostic,
            input.plan.alarm_policy.diagnostic_threshold,
        ) else {
            panic!("complete alarm fixture should produce");
        };
        assert_eq!(summary.alarms_emitted, 3);
        assert_eq!(summary.alarms_executed, 2);
        assert_eq!(summary.alarms_matched, 1);
        assert_eq!(summary.capacity_rejected, 1);
        assert_eq!(summary.refractory_suppressed, 1);
        assert_eq!(summary.detected_events, 1);
        assert_eq!(summary.undetected_events, 1);
        assert!(summary.lead_times.iter().any(|record| matches!(
            record,
            H2LeadTimeRecord::Detected {
                lead_time_ns: 5,
                ..
            }
        )));
        assert!(summary
            .lead_times
            .iter()
            .any(|record| matches!(record, H2LeadTimeRecord::Undetected { .. })));
    }
}
