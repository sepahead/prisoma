//! Exact-fork world-model decision reference for the Prisoma control plane.
//!
//! This module is a small software reference. It learns an action-conditioned
//! affine transition from deterministic simulator rows, commits forecasts for
//! an ordered candidate pool, labels all candidates on independent restored
//! simulator branches, and verifies the resulting canonical run log. It does
//! not establish physical truth, real-robot validity, or learned-model quality.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::path::Path;

use anyhow::{bail, Context, Result};
use pid_bridge::{BridgeMethod, BridgeRequest};
use pid_runlog::{
    canonical_json_hash_v2, read_events, validate_events, Actor, RunLogEvent, RunStatus,
    SimObjectSnapshot, RUN_LOG_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::file_snapshot::{read_bounded_regular_file, validate_strict_json_lines};
use crate::{
    bridge_request, verify_flow_gt, verify_sim_replay, DeterministicObjectSim, SimBridgeSession,
    SimObject,
};

pub const WORLD_MODEL_DECISION_SCHEMA: &str = "prisoma.world_model_decision/1";
pub const FORECAST_COMMIT_LABEL: &str = "world_model.forecast_commit";
pub const ORACLE_LABEL: &str = "world_model.oracle_label";
pub const EXECUTION_RECEIPT_LABEL: &str = "world_model.execution_receipt";
pub const REFERENCE_MODEL_FAMILY: &str = "affine_ridge_state_transition";
pub const REFERENCE_OBJECT_ID: &str = "puck";
pub const REFERENCE_RUN_ID: &str = "world-model-reference";
pub const REFERENCE_SOURCE: &str = "pid-world-model-reference";
pub const MAX_WORLD_MODEL_RUNLOG_BYTES: u64 = 16 * 1024 * 1024;
const TRAINING_SCHEMA: &str = "prisoma.world_model_training_grid/1";
const MODEL_SCHEMA: &str = "prisoma.affine_world_model/1";
const VERIFICATION_SCHEMA: &str = "prisoma.world_model_verification/1";
const EVIDENCE_SCOPE: &str =
    "deterministic_contract_conformance_not_world_model_scientific_evidence";
const ACTION_SEMANTICS: &str = "constant_velocity_candidate_fixed_before_prediction_and_execution";
const REFERENCE_SEMANTICS: &str =
    "independently_restored_deterministic_simulator_branch_not_physical_ground_truth";
const SELECTION_RULE: &str = "minimum_predicted_squared_goal_distance_then_pool_order";
const MODEL_FEATURES: usize = 7;
const MODEL_OUTPUTS: usize = 3;
const FLOAT_TOLERANCE: f64 = 1e-10;

/// Finite work limits for the executable reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldModelResourceLimits {
    pub max_training_rows: usize,
    pub max_decisions: usize,
    pub max_candidates_per_decision: usize,
    pub max_horizon_steps: u64,
}

impl Default for WorldModelResourceLimits {
    fn default() -> Self {
        Self {
            max_training_rows: 1_024,
            max_decisions: 16,
            max_candidates_per_decision: 16,
            max_horizon_steps: 16,
        }
    }
}

impl WorldModelResourceLimits {
    fn validate(self) -> Result<Self> {
        if self.max_training_rows == 0
            || self.max_decisions == 0
            || self.max_candidates_per_decision < 2
            || self.max_horizon_steps == 0
        {
            bail!("world-model resource limits must be positive and admit at least two candidates");
        }
        Ok(self)
    }
}

/// Fixed training-law declaration for the small reference model.
///
/// `action_grid` is the finite design used to identify the affine coefficients.
/// `declared_action_domain` is a separate fixture assumption about where that
/// affine law applies. The grid alone does not establish continuous support.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldModelTrainingContract {
    pub schema: String,
    pub state_grid: Vec<f64>,
    pub action_grid: Vec<f64>,
    pub declared_action_domain: [f64; 2],
    pub dt_secs: f64,
    pub ridge_penalty: f64,
}

impl Default for WorldModelTrainingContract {
    fn default() -> Self {
        Self {
            schema: TRAINING_SCHEMA.to_string(),
            state_grid: vec![-0.5, 0.0, 0.5],
            action_grid: vec![-1.0, 0.0, 1.0],
            declared_action_domain: [-1.0, 1.0],
            dt_secs: 0.2,
            ridge_penalty: 1e-12,
        }
    }
}

impl WorldModelTrainingContract {
    fn validate(&self) -> Result<()> {
        if self.schema != TRAINING_SCHEMA {
            bail!("unsupported world-model training schema {}", self.schema);
        }
        if self.state_grid.is_empty() || self.action_grid.is_empty() {
            bail!("world-model training grids must not be empty");
        }
        if self
            .state_grid
            .iter()
            .chain(&self.action_grid)
            .any(|value| !value.is_finite())
        {
            bail!("world-model training grids must contain finite values");
        }
        let [domain_min, domain_max] = self.declared_action_domain;
        if !domain_min.is_finite() || !domain_max.is_finite() || domain_min >= domain_max {
            bail!("world-model declared action domain must be finite and increasing");
        }
        if self
            .action_grid
            .iter()
            .any(|value| *value < domain_min || *value > domain_max)
        {
            bail!("world-model action grid must lie inside its declared action domain");
        }
        if !self.dt_secs.is_finite() || self.dt_secs <= 0.0 {
            bail!("world-model training dt_secs must be positive and finite");
        }
        if !self.ridge_penalty.is_finite() || self.ridge_penalty <= 0.0 {
            bail!("world-model ridge_penalty must be positive and finite");
        }
        Ok(())
    }

    fn projected_rows(&self) -> Result<usize> {
        let state_rows = checked_pow(self.state_grid.len(), 3, "state-grid rows")?;
        let action_rows = checked_pow(self.action_grid.len(), 3, "action-grid rows")?;
        state_rows
            .checked_mul(action_rows)
            .context("world-model training-row projection overflow")
    }
}

/// Learned affine transition receipt. Coefficients map `[1, x, y, z, ax, ay, az]`
/// to the next Cartesian position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AffineWorldModel {
    pub schema: String,
    pub family: String,
    pub training_rows: usize,
    pub training_sha256: String,
    pub coefficients: [[f64; MODEL_FEATURES]; MODEL_OUTPUTS],
    pub ridge_penalty: f64,
    pub declared_action_domain: [f64; 2],
}

impl AffineWorldModel {
    /// Predict one next position from one current position and domain-admitted action.
    pub fn predict_next(&self, state: [f64; 3], action: [f64; 3]) -> Result<[f64; 3]> {
        validate_model(self)?;
        self.predict_next_validated(state, action)
    }

    fn predict_next_validated(&self, state: [f64; 3], action: [f64; 3]) -> Result<[f64; 3]> {
        validate_vec3(state, "world-model state")?;
        validate_vec3(action, "world-model action")?;
        let [domain_min, domain_max] = self.declared_action_domain;
        if action
            .iter()
            .any(|component| *component < domain_min || *component > domain_max)
        {
            bail!(
                "world-model action lies outside the declared affine fixture domain [{domain_min}, {domain_max}]"
            );
        }
        let features = [
            1.0, state[0], state[1], state[2], action[0], action[1], action[2],
        ];
        let mut output = [0.0; MODEL_OUTPUTS];
        for (target, coefficients) in output.iter_mut().zip(&self.coefficients) {
            *target = coefficients
                .iter()
                .zip(features)
                .map(|(coefficient, feature)| coefficient * feature)
                .sum();
        }
        validate_vec3(output, "world-model prediction")?;
        Ok(output)
    }

    /// Roll one constant action through the learned transition for a finite horizon.
    pub fn rollout_endpoint(
        &self,
        mut state: [f64; 3],
        action: [f64; 3],
        horizon_steps: u64,
        limits: WorldModelResourceLimits,
    ) -> Result<[f64; 3]> {
        validate_model(self)?;
        let limits = limits.validate()?;
        if horizon_steps == 0 || horizon_steps > limits.max_horizon_steps {
            bail!(
                "world-model horizon {horizon_steps} is outside 1..={}",
                limits.max_horizon_steps
            );
        }
        for _ in 0..horizon_steps {
            state = self.predict_next_validated(state, action)?;
        }
        Ok(state)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrainingRow {
    state: [f64; 3],
    action: [f64; 3],
    next_state: [f64; 3],
}

/// One feasible action in the frozen, ordered candidate set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldModelCandidate {
    pub candidate_id: String,
    pub velocity: [f64; 3],
}

/// Immutable simulator fork that all candidates share.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldModelFork {
    pub decision_index: usize,
    pub step: u64,
    pub timestamp_ns: u64,
    pub target_object_id: String,
    pub objects: Vec<SimObjectSnapshot>,
}

/// One pre-outcome action-conditioned forecast and its frozen cost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateForecast {
    pub candidate_id: String,
    pub predicted_endpoint: [f64; 3],
    pub predicted_cost: f64,
}

/// Forecast-stage record. This must enter the run log before oracle labeling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastCommit {
    pub schema: String,
    pub fork: WorldModelFork,
    pub fork_sha256: String,
    pub candidate_pool: Vec<WorldModelCandidate>,
    pub candidate_pool_sha256: String,
    pub model_sha256: String,
    pub goal_position: [f64; 3],
    pub horizon_steps: u64,
    pub dt_secs: f64,
    pub nominal_candidate_id: String,
    pub forecasts: Vec<CandidateForecast>,
    pub selected_candidate_id: String,
    pub selection_rule: String,
    pub oracle_accessed: bool,
}

/// One reference-simulator outcome for a member of the committed pool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateOracleOutcome {
    pub candidate_id: String,
    pub reference_endpoint: [f64; 3],
    pub reference_cost: f64,
}

/// Post-commit label over every candidate on independent restored branches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OracleLabelRecord {
    pub schema: String,
    pub decision_index: usize,
    pub fork_sha256: String,
    pub candidate_pool_sha256: String,
    pub outcomes: Vec<CandidateOracleOutcome>,
    pub oracle_best_candidate_id: String,
    pub selected_candidate_id: String,
    pub candidate_set_regret: f64,
    pub nominal_cost_minus_selected_cost: f64,
    pub reference_model_role: String,
}

/// Canonical receipt for the one action installed through the Agent Bridge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldModelExecutionReceipt {
    pub schema: String,
    pub decision_index: usize,
    pub selected_candidate_id: String,
    pub executed_velocity: [f64; 3],
    pub resulting_step: u64,
    pub resulting_timestamp_ns: u64,
    pub resulting_position: [f64; 3],
}

/// Content-bound configuration stored in the canonical run log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldModelReferenceConfig {
    pub schema: String,
    pub evidence_scope: String,
    pub training: WorldModelTrainingContract,
    pub model: AffineWorldModel,
    pub model_sha256: String,
    pub target_object_id: String,
    pub initial_position: [f64; 3],
    pub goal_position: [f64; 3],
    pub dt_secs: f64,
    pub horizon_steps: u64,
    pub decisions: usize,
    pub max_abs_velocity: f64,
    pub resource_limits: WorldModelResourceLimits,
    pub action_semantics: String,
    pub reference_semantics: String,
}

/// Summary derived from a verified reference run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldModelVerificationReport {
    pub schema: String,
    pub valid: bool,
    pub decisions_verified: usize,
    pub candidates_verified: usize,
    pub action_sensitive_decisions: usize,
    pub final_position: [f64; 3],
    pub summed_visited_fork_candidate_set_regret: f64,
    pub issues: Vec<String>,
}

impl WorldModelVerificationReport {
    pub fn require_valid(&self) -> Result<()> {
        if self.valid {
            Ok(())
        } else {
            bail!(
                "world-model decision verification failed: {}",
                self.issues.join("; ")
            )
        }
    }
}

#[derive(Debug)]
pub struct ProposedDecision {
    config: WorldModelReferenceConfig,
    fork: WorldModelFork,
    candidate_pool: Vec<WorldModelCandidate>,
}

#[derive(Debug)]
pub struct ForecastCommitted {
    config: WorldModelReferenceConfig,
    record: ForecastCommit,
}

#[derive(Debug)]
pub struct ForecastPublished {
    config: WorldModelReferenceConfig,
    record: ForecastCommit,
}

#[derive(Debug)]
pub struct OracleLabeled {
    forecast: ForecastCommit,
    execution: WorldModelExecutionReceipt,
    oracle: OracleLabelRecord,
}

/// Selected execution that reached the run log before oracle labeling became available.
#[derive(Debug)]
pub struct DecisionExecuted {
    config: WorldModelReferenceConfig,
    forecast: ForecastCommit,
    execution: WorldModelExecutionReceipt,
}

/// Oracle record published after the committed selected action executed.
#[derive(Debug)]
pub struct OraclePublished {
    forecast: ForecastCommit,
    execution: WorldModelExecutionReceipt,
    oracle: OracleLabelRecord,
}

impl ProposedDecision {
    /// Apply the learned transition and choose only from forecast costs.
    pub fn commit_forecast(self) -> Result<ForecastCommitted> {
        let record = build_forecast_commit(&self.config, self.fork, self.candidate_pool)?;
        Ok(ForecastCommitted {
            config: self.config,
            record,
        })
    }
}

impl ForecastCommitted {
    pub fn record(&self) -> &ForecastCommit {
        &self.record
    }

    /// Publish the full forecast and candidate-pool commitment before oracle access is possible.
    pub fn publish<W: std::io::Write>(
        self,
        session: &mut SimBridgeSession<W>,
    ) -> Result<ForecastPublished> {
        write_forecast_commit(session, &self.record)?;
        Ok(ForecastPublished {
            config: self.config,
            record: self.record,
        })
    }
}

impl ForecastPublished {
    pub fn record(&self) -> &ForecastCommit {
        &self.record
    }
}

impl DecisionExecuted {
    pub fn forecast(&self) -> &ForecastCommit {
        &self.forecast
    }

    pub fn execution(&self) -> &WorldModelExecutionReceipt {
        &self.execution
    }

    /// Label all candidates on saved independent forks after selected execution.
    pub fn label_oracle(self) -> Result<OracleLabeled> {
        let oracle = build_oracle_label(&self.config, &self.forecast)?;
        Ok(OracleLabeled {
            forecast: self.forecast,
            execution: self.execution,
            oracle,
        })
    }
}

impl OracleLabeled {
    pub fn forecast(&self) -> &ForecastCommit {
        &self.forecast
    }

    pub fn oracle(&self) -> &OracleLabelRecord {
        &self.oracle
    }

    pub fn execution(&self) -> &WorldModelExecutionReceipt {
        &self.execution
    }
}

impl OraclePublished {
    pub fn forecast(&self) -> &ForecastCommit {
        &self.forecast
    }

    pub fn oracle(&self) -> &OracleLabelRecord {
        &self.oracle
    }

    pub fn execution(&self) -> &WorldModelExecutionReceipt {
        &self.execution
    }
}

/// Train the native software-reference transition model from the declared grid.
pub fn train_reference_world_model(
    contract: &WorldModelTrainingContract,
    limits: WorldModelResourceLimits,
) -> Result<AffineWorldModel> {
    contract.validate()?;
    let limits = limits.validate()?;
    let projected_rows = contract.projected_rows()?;
    if projected_rows > limits.max_training_rows {
        bail!(
            "world-model training requires {projected_rows} rows, limit is {}",
            limits.max_training_rows
        );
    }
    let rows = generate_training_rows(contract, projected_rows)?;
    let training_sha256 = canonical_json_hash_v2(&rows)?;
    let mut normal = [[0.0; MODEL_FEATURES]; MODEL_FEATURES];
    let mut rhs = [[0.0; MODEL_FEATURES]; MODEL_OUTPUTS];
    for row in &rows {
        let features = [
            1.0,
            row.state[0],
            row.state[1],
            row.state[2],
            row.action[0],
            row.action[1],
            row.action[2],
        ];
        for column in 0..MODEL_FEATURES {
            for other in 0..MODEL_FEATURES {
                normal[column][other] += features[column] * features[other];
            }
            for (target_rhs, next_state) in rhs.iter_mut().zip(row.next_state) {
                target_rhs[column] += features[column] * next_state;
            }
        }
    }
    for (index, row) in normal.iter_mut().enumerate().skip(1) {
        row[index] += contract.ridge_penalty;
    }
    let mut coefficients = [[0.0; MODEL_FEATURES]; MODEL_OUTPUTS];
    for target in 0..MODEL_OUTPUTS {
        coefficients[target] = solve_fixed_linear_system(normal, rhs[target])
            .with_context(|| format!("failed to fit world-model output {target}"))?;
    }
    let model = AffineWorldModel {
        schema: MODEL_SCHEMA.to_string(),
        family: REFERENCE_MODEL_FAMILY.to_string(),
        training_rows: rows.len(),
        training_sha256,
        coefficients,
        ridge_penalty: contract.ridge_penalty,
        declared_action_domain: contract.declared_action_domain,
    };
    validate_model(&model)?;
    Ok(model)
}

/// Build the complete content-bound configuration for the executable reference.
pub fn reference_config() -> Result<WorldModelReferenceConfig> {
    let resource_limits = WorldModelResourceLimits::default();
    let training = WorldModelTrainingContract::default();
    let model = train_reference_world_model(&training, resource_limits)?;
    let model_sha256 = canonical_json_hash_v2(&model)?;
    let config = WorldModelReferenceConfig {
        schema: WORLD_MODEL_DECISION_SCHEMA.to_string(),
        evidence_scope: EVIDENCE_SCOPE.to_string(),
        training,
        model,
        model_sha256,
        target_object_id: REFERENCE_OBJECT_ID.to_string(),
        initial_position: [0.0, 0.0, 0.0],
        goal_position: [0.6, 0.2, 0.0],
        dt_secs: 0.2,
        horizon_steps: 4,
        decisions: 4,
        max_abs_velocity: 0.3,
        resource_limits,
        action_semantics: ACTION_SEMANTICS.to_string(),
        reference_semantics: REFERENCE_SEMANTICS.to_string(),
    };
    validate_reference_config(&config)?;
    Ok(config)
}

/// Initial simulator state used by the native reference binary.
pub fn reference_sim(config: &WorldModelReferenceConfig) -> Result<DeterministicObjectSim> {
    validate_reference_config(config)?;
    let mut sim = DeterministicObjectSim::new();
    sim.upsert_object(SimObject {
        object_id: config.target_object_id.clone(),
        pose: pid_runlog::Pose {
            position: config.initial_position,
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        velocity: [0.0; 3],
    })?;
    Ok(sim)
}

/// Prepare one immutable fork and deterministic ordered candidate pool.
pub fn propose_reference_decision(
    config: &WorldModelReferenceConfig,
    sim: &DeterministicObjectSim,
    decision_index: usize,
) -> Result<ProposedDecision> {
    validate_reference_config(config)?;
    if decision_index >= config.decisions || decision_index >= config.resource_limits.max_decisions
    {
        bail!("world-model decision index {decision_index} exceeds the configured run");
    }
    let fork = WorldModelFork {
        decision_index,
        step: sim.step(),
        timestamp_ns: sim.timestamp_ns(),
        target_object_id: config.target_object_id.clone(),
        objects: snapshot_objects(sim),
    };
    let candidate_pool = generate_candidate_pool(config, &fork)?;
    Ok(ProposedDecision {
        config: config.clone(),
        fork,
        candidate_pool,
    })
}

/// Prepare a decision from the session's actual simulator state.
///
/// This keeps the executable path from maintaining a second mutable simulator.
pub fn propose_reference_session_decision<W: std::io::Write>(
    config: &WorldModelReferenceConfig,
    session: &SimBridgeSession<W>,
    decision_index: usize,
) -> Result<ProposedDecision> {
    propose_reference_decision(config, &session.handler.sim, decision_index)
}

/// Record a forecast commit and its viewer-ready predicted displacement events.
pub fn record_forecast_commit<W: std::io::Write>(
    session: &mut SimBridgeSession<W>,
    commit: ForecastCommitted,
) -> Result<ForecastPublished> {
    commit.publish(session)
}

fn write_forecast_commit<W: std::io::Write>(
    session: &mut SimBridgeSession<W>,
    commit: &ForecastCommit,
) -> Result<()> {
    let value = serde_json::to_value(commit)?;
    session.record_event(&RunLogEvent::LabelObserved {
        step: commit.fork.step,
        timestamp_ns: commit.fork.timestamp_ns,
        name: FORECAST_COMMIT_LABEL.to_string(),
        value,
        metadata: decision_metadata(commit.fork.decision_index),
    })?;
    let start = target_position(&commit.fork)?;
    for forecast in &commit.forecasts {
        session.record_event(&RunLogEvent::FlowPred {
            step: commit.fork.step,
            timestamp_ns: commit.fork.timestamp_ns,
            source: REFERENCE_MODEL_FAMILY.to_string(),
            object_id: commit.fork.target_object_id.clone(),
            horizon_steps: commit.horizon_steps,
            flow: vec![subtract(forecast.predicted_endpoint, start)],
            metadata: [
                ("candidate_id".to_string(), forecast.candidate_id.clone()),
                (
                    "candidate_pool_sha256".to_string(),
                    commit.candidate_pool_sha256.clone(),
                ),
                (
                    "decision_index".to_string(),
                    commit.fork.decision_index.to_string(),
                ),
                (
                    "forecast_stage".to_string(),
                    "pre_oracle_commit".to_string(),
                ),
            ]
            .into_iter()
            .collect(),
        })?;
    }
    session.flush()
}

/// Record post-execution oracle labels from the saved committed fork.
pub fn record_oracle_label<W: std::io::Write>(
    session: &mut SimBridgeSession<W>,
    labeled: OracleLabeled,
) -> Result<OraclePublished> {
    let oracle = labeled.oracle();
    session.record_event(&RunLogEvent::LabelObserved {
        step: labeled.execution.resulting_step,
        timestamp_ns: labeled.execution.resulting_timestamp_ns,
        name: ORACLE_LABEL.to_string(),
        value: serde_json::to_value(oracle)?,
        metadata: decision_metadata(oracle.decision_index),
    })?;
    for (name, value) in [
        ("candidate_set_regret", oracle.candidate_set_regret),
        (
            "nominal_cost_minus_selected_cost",
            oracle.nominal_cost_minus_selected_cost,
        ),
    ] {
        session.record_event(&RunLogEvent::EvaluationMetric {
            step: labeled.execution.resulting_step,
            timestamp_ns: labeled.execution.resulting_timestamp_ns,
            name: format!("world_model.{name}"),
            value,
            metadata: decision_metadata(oracle.decision_index),
        })?;
    }
    session.flush()?;
    Ok(OraclePublished {
        forecast: labeled.forecast,
        execution: labeled.execution,
        oracle: labeled.oracle,
    })
}

/// Execute the committed selection through the Agent Bridge and record its receipt.
pub fn execute_published_decision<W: std::io::Write>(
    session: &mut SimBridgeSession<W>,
    published: ForecastPublished,
    actor: &Actor,
) -> Result<DecisionExecuted> {
    let selected = selected_candidate(&published.record)?;
    let decision_index = published.record.fork.decision_index;
    let actual_fork = world_model_fork(&published.config, &session.handler.sim, decision_index);
    if actual_fork != published.record.fork {
        bail!("world-model session state differs from the committed fork before execution");
    }
    let intervention = bridge_request(
        format!("wm-{decision_index}-set-velocity"),
        BridgeMethod::InterventionApply,
        actor.clone(),
        Some(published.record.fork.step),
        published.record.fork.timestamp_ns,
        json!({
            "intervention_type": "set_velocity",
            "payload": {
                "object_id": published.record.fork.target_object_id,
                "velocity": selected.velocity,
            }
        }),
    );
    require_bridge_success(session.dispatch(&intervention)?, &intervention)?;
    let step_request = bridge_request(
        format!("wm-{decision_index}-step"),
        BridgeMethod::SimStep,
        actor.clone(),
        Some(published.record.fork.step),
        published.record.fork.timestamp_ns,
        json!({ "dt": published.config.dt_secs }),
    );
    require_bridge_success(session.dispatch(&step_request)?, &step_request)?;
    let receipt = expected_execution_receipt(&published.config, &published.record)?;
    let actual_receipt = WorldModelExecutionReceipt {
        schema: WORLD_MODEL_DECISION_SCHEMA.to_string(),
        decision_index,
        selected_candidate_id: selected.candidate_id.clone(),
        executed_velocity: selected.velocity,
        resulting_step: session.handler.sim.step(),
        resulting_timestamp_ns: session.handler.sim.timestamp_ns(),
        resulting_position: object_position(
            &session.handler.sim,
            &published.record.fork.target_object_id,
        )?,
    };
    compare_execution_receipts(&actual_receipt, &receipt)?;
    session.record_event(&RunLogEvent::LabelObserved {
        step: receipt.resulting_step,
        timestamp_ns: receipt.resulting_timestamp_ns,
        name: EXECUTION_RECEIPT_LABEL.to_string(),
        value: serde_json::to_value(&receipt)?,
        metadata: decision_metadata(decision_index),
    })?;
    session.flush()?;
    Ok(DecisionExecuted {
        config: published.config,
        forecast: published.record,
        execution: receipt,
    })
}

/// Verify one bounded run-log snapshot without trusting a sidecar summary.
pub fn verify_world_model_runlog(path: impl AsRef<Path>) -> Result<WorldModelVerificationReport> {
    let path = path.as_ref();
    let snapshot = read_bounded_regular_file(
        path,
        MAX_WORLD_MODEL_RUNLOG_BYTES,
        "world-model reference run log",
    )?;
    let bytes = snapshot.exact_bytes(MAX_WORLD_MODEL_RUNLOG_BYTES)?;
    validate_strict_json_lines(bytes, "world-model reference run log")?;
    let events = read_events(Cursor::new(bytes))?;
    snapshot.verify_path()?;
    verify_world_model_events(&events)
}

/// Verify the exact decision semantics of an already parsed event stream.
pub fn verify_world_model_events(events: &[RunLogEvent]) -> Result<WorldModelVerificationReport> {
    let validation = validate_events(events)?;
    if !validation.is_valid() {
        let issues = validation
            .issues
            .into_iter()
            .map(|issue| match issue.event_index {
                Some(index) => format!("run-log event {index}: {}", issue.message),
                None => format!("run log: {}", issue.message),
            })
            .collect();
        return Ok(invalid_report(issues));
    }
    let result = verify_world_model_events_inner(events);
    match result {
        Ok(report) => Ok(report),
        Err(error) => Ok(invalid_report(vec![error.to_string()])),
    }
}

fn verify_world_model_events_inner(events: &[RunLogEvent]) -> Result<WorldModelVerificationReport> {
    let (config_hash, config) = extract_config(events)?;
    validate_reference_config(&config)?;
    let recomputed_model = train_reference_world_model(&config.training, config.resource_limits)?;
    compare_models(&config.model, &recomputed_model)?;
    if canonical_json_hash_v2(&config.model)? != config.model_sha256 {
        bail!("world-model config model_sha256 does not bind its model receipt");
    }
    require_exact_world_model_event_counts(events, config.decisions)?;
    require_reference_run_envelope(events, &config_hash, &config)?;
    let replay = verify_sim_replay(events, FLOAT_TOLERANCE);
    if !replay.is_valid() {
        bail!(
            "world-model bridge replay failed: {}",
            replay.issues.join("; ")
        );
    }
    let flow = verify_flow_gt(events, FLOAT_TOLERANCE);
    if !flow.is_valid() {
        bail!(
            "world-model Flow_gt replay failed: {}",
            flow.issues.join("; ")
        );
    }
    let logged_config = events.iter().find_map(|event| match event {
        RunLogEvent::ConfigLogged { config, .. } => Some(config),
        _ => None,
    });
    if logged_config
        .map(canonical_json_hash_v2)
        .transpose()?
        .as_deref()
        != Some(&config_hash)
    {
        bail!("world-model logged configuration hash mismatch");
    }

    let mut cursor = 0_usize;
    let mut expected_sim = reference_sim(&config)?;
    let mut candidates_verified = 0_usize;
    let mut action_sensitive_decisions = 0_usize;
    let mut summed_visited_fork_candidate_set_regret = 0.0;
    let mut final_position = config.initial_position;
    for decision_index in 0..config.decisions {
        let (commit_index, commit) = next_typed_label::<ForecastCommit>(
            events,
            cursor,
            FORECAST_COMMIT_LABEL,
            decision_index,
        )?;
        if commit.fork.decision_index != decision_index {
            bail!("forecast decision index is not canonical");
        }
        require_label_envelope(
            &events[commit_index],
            FORECAST_COMMIT_LABEL,
            commit.fork.step,
            commit.fork.timestamp_ns,
            decision_index,
        )?;
        let expected_fork = world_model_fork(&config, &expected_sim, decision_index);
        if commit.fork != expected_fork {
            bail!("decision {decision_index} fork differs from the canonical executed state");
        }
        let expected_pool = generate_candidate_pool(&config, &expected_fork)?;
        if commit.candidate_pool != expected_pool {
            bail!("decision {decision_index} candidate pool differs from the frozen proposal rule");
        }
        let expected =
            build_forecast_commit(&config, commit.fork.clone(), commit.candidate_pool.clone())?;
        compare_forecast_commits(&commit, &expected)?;
        if commit.oracle_accessed {
            bail!("forecast commit claims oracle access before selection");
        }
        let unique_predictions = commit
            .forecasts
            .iter()
            .map(|forecast| forecast.predicted_endpoint)
            .collect::<Vec<_>>();
        if unique_predictions
            .windows(2)
            .any(|pair| !vec3_close(pair[0], pair[1]))
        {
            action_sensitive_decisions += 1;
        }
        let (receipt_index, receipt) = next_typed_label::<WorldModelExecutionReceipt>(
            events,
            commit_index + 1,
            EXECUTION_RECEIPT_LABEL,
            decision_index,
        )?;
        require_label_envelope(
            &events[receipt_index],
            EXECUTION_RECEIPT_LABEL,
            receipt.resulting_step,
            receipt.resulting_timestamp_ns,
            decision_index,
        )?;
        let expected_receipt = expected_execution_receipt(&config, &commit)?;
        compare_execution_receipts(&receipt, &expected_receipt)?;
        require_forecast_flow_events(events, commit_index + 1, receipt_index, &commit)?;
        require_bridge_execution(events, commit_index + 1, receipt_index, &commit, &receipt)?;
        let (oracle_index, oracle) = next_typed_label::<OracleLabelRecord>(
            events,
            receipt_index + 1,
            ORACLE_LABEL,
            decision_index,
        )?;
        require_label_envelope(
            &events[oracle_index],
            ORACLE_LABEL,
            receipt.resulting_step,
            receipt.resulting_timestamp_ns,
            decision_index,
        )?;
        reject_mutations(
            events,
            receipt_index + 1,
            oracle_index,
            "between selected execution and oracle labeling",
        )?;
        let expected_oracle = build_oracle_label(&config, &commit)?;
        compare_oracle_labels(&oracle, &expected_oracle)?;
        let metric_end = oracle_index
            .checked_add(3)
            .context("world-model oracle metric range overflow")?;
        if metric_end > events.len() {
            bail!("world-model oracle label lacks two following decision metrics");
        }
        require_oracle_metrics(events, oracle_index + 1, metric_end, &receipt, &oracle)?;
        candidates_verified = candidates_verified
            .checked_add(commit.candidate_pool.len())
            .context("world-model verified-candidate counter overflow")?;
        summed_visited_fork_candidate_set_regret += oracle.candidate_set_regret;
        if !summed_visited_fork_candidate_set_regret.is_finite() {
            bail!("world-model aggregate regret became non-finite");
        }
        final_position = receipt.resulting_position;
        expected_sim = sim_from_fork(&commit.fork)?;
        apply_velocity(
            &mut expected_sim,
            &config.target_object_id,
            receipt.executed_velocity,
        )?;
        expected_sim.step_fixed(config.dt_secs)?;
        cursor = metric_end;
    }
    if next_label_index(events, cursor, FORECAST_COMMIT_LABEL).is_some() {
        bail!("world-model run contains more forecast commits than configured");
    }
    let forecast_flow_count = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RunLogEvent::FlowPred { source, .. } if source == REFERENCE_MODEL_FAMILY
            )
        })
        .count();
    if forecast_flow_count != candidates_verified {
        bail!(
            "world-model run contains {forecast_flow_count} reference forecast flows, expected {candidates_verified}"
        );
    }
    if action_sensitive_decisions != config.decisions {
        bail!("one or more decisions failed the reference action-sensitivity check");
    }
    if !events.iter().any(|event| {
        matches!(
            event,
            RunLogEvent::RunEnded {
                status: RunStatus::Succeeded,
                ..
            }
        )
    }) {
        bail!("world-model run lacks a successful terminal event");
    }
    Ok(WorldModelVerificationReport {
        schema: VERIFICATION_SCHEMA.to_string(),
        valid: true,
        decisions_verified: config.decisions,
        candidates_verified,
        action_sensitive_decisions,
        final_position,
        summed_visited_fork_candidate_set_regret,
        issues: Vec::new(),
    })
}

fn validate_reference_config(config: &WorldModelReferenceConfig) -> Result<()> {
    if config.schema != WORLD_MODEL_DECISION_SCHEMA {
        bail!("unsupported world-model decision schema {}", config.schema);
    }
    config.training.validate()?;
    validate_model(&config.model)?;
    if config.evidence_scope != EVIDENCE_SCOPE
        || config.action_semantics != ACTION_SEMANTICS
        || config.reference_semantics != REFERENCE_SEMANTICS
    {
        bail!("world-model configuration changes the software-reference evidence boundary");
    }
    if config.model.ridge_penalty != config.training.ridge_penalty
        || config.dt_secs != config.training.dt_secs
        || config.model.declared_action_domain != config.training.declared_action_domain
    {
        bail!("world-model training and execution contracts disagree");
    }
    if canonical_json_hash_v2(&config.model)? != config.model_sha256 {
        bail!("world-model model_sha256 does not bind the configured model");
    }
    let limits = config.resource_limits.validate()?;
    if config.decisions == 0 || config.decisions > limits.max_decisions {
        bail!("world-model decision count is outside its resource limit");
    }
    if config.horizon_steps == 0 || config.horizon_steps > limits.max_horizon_steps {
        bail!("world-model horizon is outside its resource limit");
    }
    if !config.dt_secs.is_finite() || config.dt_secs <= 0.0 {
        bail!("world-model execution dt_secs must be positive and finite");
    }
    if !config.max_abs_velocity.is_finite() || config.max_abs_velocity <= 0.0 {
        bail!("world-model max_abs_velocity must be positive and finite");
    }
    let [domain_min, domain_max] = config.model.declared_action_domain;
    if -config.max_abs_velocity < domain_min || config.max_abs_velocity > domain_max {
        bail!("world-model candidate bound exceeds the declared affine fixture domain");
    }
    validate_vec3(config.initial_position, "world-model initial position")?;
    validate_vec3(config.goal_position, "world-model goal position")?;
    if config.target_object_id.is_empty() {
        bail!("world-model target object id must not be empty");
    }
    Ok(())
}

fn validate_model(model: &AffineWorldModel) -> Result<()> {
    if model.schema != MODEL_SCHEMA || model.family != REFERENCE_MODEL_FAMILY {
        bail!("unsupported affine world-model receipt");
    }
    if model.training_rows == 0 || !is_lowercase_sha256(&model.training_sha256) {
        bail!("affine world-model training receipt is incomplete");
    }
    if !model.ridge_penalty.is_finite() || model.ridge_penalty <= 0.0 {
        bail!("affine world-model ridge penalty must be positive and finite");
    }
    let [domain_min, domain_max] = model.declared_action_domain;
    if !domain_min.is_finite() || !domain_max.is_finite() || domain_min >= domain_max {
        bail!("affine world-model declared action domain is invalid");
    }
    if model
        .coefficients
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        bail!("affine world-model coefficients must be finite");
    }
    Ok(())
}

fn generate_training_rows(
    contract: &WorldModelTrainingContract,
    projected_rows: usize,
) -> Result<Vec<TrainingRow>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(projected_rows)
        .context("failed to reserve world-model training rows")?;
    for &x in &contract.state_grid {
        for &y in &contract.state_grid {
            for &z in &contract.state_grid {
                for &ax in &contract.action_grid {
                    for &ay in &contract.action_grid {
                        for &az in &contract.action_grid {
                            let state = [x, y, z];
                            let action = [ax, ay, az];
                            let mut sim = sim_at_state(state)?;
                            apply_velocity(&mut sim, REFERENCE_OBJECT_ID, action)?;
                            sim.step_fixed(contract.dt_secs)?;
                            rows.push(TrainingRow {
                                state,
                                action,
                                next_state: object_position(&sim, REFERENCE_OBJECT_ID)?,
                            });
                        }
                    }
                }
            }
        }
    }
    if rows.len() != projected_rows {
        bail!("world-model training generator produced an unexpected row count");
    }
    Ok(rows)
}

fn generate_candidate_pool(
    config: &WorldModelReferenceConfig,
    fork: &WorldModelFork,
) -> Result<Vec<WorldModelCandidate>> {
    let current = target_position(fork)?;
    let denominator = config.dt_secs * config.horizon_steps as f64;
    if !denominator.is_finite() || denominator <= 0.0 {
        bail!("world-model candidate horizon has invalid duration");
    }
    let direct = clamp_vec3(
        [
            (config.goal_position[0] - current[0]) / denominator,
            (config.goal_position[1] - current[1]) / denominator,
            (config.goal_position[2] - current[2]) / denominator,
        ],
        config.max_abs_velocity,
    );
    let pool = vec![
        WorldModelCandidate {
            candidate_id: "nominal_x".to_string(),
            velocity: [0.25, 0.0, 0.0],
        },
        WorldModelCandidate {
            candidate_id: "direct_goal".to_string(),
            velocity: direct,
        },
        WorldModelCandidate {
            candidate_id: "cautious_goal".to_string(),
            velocity: scale(direct, 0.5),
        },
        WorldModelCandidate {
            candidate_id: "overshoot_goal".to_string(),
            velocity: clamp_vec3(scale(direct, 1.25), config.max_abs_velocity),
        },
    ];
    validate_candidate_pool(&pool, config.resource_limits, config.max_abs_velocity)?;
    Ok(pool)
}

fn validate_candidate_pool(
    pool: &[WorldModelCandidate],
    limits: WorldModelResourceLimits,
    max_abs_velocity: f64,
) -> Result<()> {
    let limits = limits.validate()?;
    if pool.len() < 2 || pool.len() > limits.max_candidates_per_decision {
        bail!("world-model candidate count is outside its resource limit");
    }
    let mut ids = BTreeSet::new();
    for candidate in pool {
        if candidate.candidate_id.is_empty() || !ids.insert(candidate.candidate_id.as_str()) {
            bail!("world-model candidate ids must be nonempty and unique");
        }
        validate_vec3(candidate.velocity, "world-model candidate velocity")?;
        if candidate
            .velocity
            .iter()
            .any(|component| component.abs() > max_abs_velocity)
        {
            bail!("world-model candidate exceeds the configured velocity bound");
        }
    }
    for (index, candidate) in pool.iter().enumerate() {
        if pool[index + 1..]
            .iter()
            .any(|other| vec3_close(candidate.velocity, other.velocity))
        {
            bail!("world-model candidate pool must contain distinct actions");
        }
    }
    Ok(())
}

fn build_forecast_commit(
    config: &WorldModelReferenceConfig,
    fork: WorldModelFork,
    candidate_pool: Vec<WorldModelCandidate>,
) -> Result<ForecastCommit> {
    validate_reference_config(config)?;
    validate_fork(&fork, config)?;
    validate_candidate_pool(
        &candidate_pool,
        config.resource_limits,
        config.max_abs_velocity,
    )?;
    let fork_sha256 = canonical_json_hash_v2(&fork)?;
    let candidate_pool_sha256 = canonical_json_hash_v2(&candidate_pool)?;
    let start = target_position(&fork)?;
    let mut forecasts = Vec::new();
    forecasts
        .try_reserve_exact(candidate_pool.len())
        .context("failed to reserve world-model forecasts")?;
    for candidate in &candidate_pool {
        let predicted_endpoint = config.model.rollout_endpoint(
            start,
            candidate.velocity,
            config.horizon_steps,
            config.resource_limits,
        )?;
        forecasts.push(CandidateForecast {
            candidate_id: candidate.candidate_id.clone(),
            predicted_endpoint,
            predicted_cost: squared_distance(predicted_endpoint, config.goal_position)?,
        });
    }
    let selected_candidate_id = minimum_cost_id(
        forecasts
            .iter()
            .map(|forecast| (forecast.candidate_id.as_str(), forecast.predicted_cost)),
    )?
    .to_string();
    Ok(ForecastCommit {
        schema: WORLD_MODEL_DECISION_SCHEMA.to_string(),
        fork,
        fork_sha256,
        candidate_pool,
        candidate_pool_sha256,
        model_sha256: config.model_sha256.clone(),
        goal_position: config.goal_position,
        horizon_steps: config.horizon_steps,
        dt_secs: config.dt_secs,
        nominal_candidate_id: "nominal_x".to_string(),
        forecasts,
        selected_candidate_id,
        selection_rule: SELECTION_RULE.to_string(),
        oracle_accessed: false,
    })
}

fn build_oracle_label(
    config: &WorldModelReferenceConfig,
    commit: &ForecastCommit,
) -> Result<OracleLabelRecord> {
    let mut outcomes = Vec::new();
    outcomes
        .try_reserve_exact(commit.candidate_pool.len())
        .context("failed to reserve world-model oracle outcomes")?;
    for candidate in &commit.candidate_pool {
        let mut sim = sim_from_fork(&commit.fork)?;
        apply_velocity(&mut sim, &commit.fork.target_object_id, candidate.velocity)?;
        for _ in 0..commit.horizon_steps {
            sim.step_fixed(commit.dt_secs)?;
        }
        let endpoint = object_position(&sim, &commit.fork.target_object_id)?;
        outcomes.push(CandidateOracleOutcome {
            candidate_id: candidate.candidate_id.clone(),
            reference_endpoint: endpoint,
            reference_cost: squared_distance(endpoint, config.goal_position)?,
        });
    }
    let oracle_best_candidate_id = minimum_cost_id(
        outcomes
            .iter()
            .map(|outcome| (outcome.candidate_id.as_str(), outcome.reference_cost)),
    )?
    .to_string();
    let selected_cost = outcome_cost(&outcomes, &commit.selected_candidate_id)?;
    let oracle_cost = outcome_cost(&outcomes, &oracle_best_candidate_id)?;
    let nominal_cost = outcome_cost(&outcomes, &commit.nominal_candidate_id)?;
    Ok(OracleLabelRecord {
        schema: WORLD_MODEL_DECISION_SCHEMA.to_string(),
        decision_index: commit.fork.decision_index,
        fork_sha256: commit.fork_sha256.clone(),
        candidate_pool_sha256: commit.candidate_pool_sha256.clone(),
        outcomes,
        oracle_best_candidate_id,
        selected_candidate_id: commit.selected_candidate_id.clone(),
        candidate_set_regret: nonnegative_roundoff(selected_cost - oracle_cost)?,
        nominal_cost_minus_selected_cost: nominal_cost - selected_cost,
        reference_model_role:
            "declared_deterministic_simulator_reference_not_physical_ground_truth".to_string(),
    })
}

fn validate_fork(fork: &WorldModelFork, config: &WorldModelReferenceConfig) -> Result<()> {
    if fork.target_object_id != config.target_object_id || fork.objects.is_empty() {
        bail!("world-model fork does not contain the configured target");
    }
    if fork.objects.len() > 64 {
        bail!("world-model fork exceeds the 64-object software-reference limit");
    }
    let mut ids = BTreeSet::new();
    for object in &fork.objects {
        if !ids.insert(object.object_id.as_str()) {
            bail!("world-model fork contains duplicate object ids");
        }
        validate_vec3(object.pose.position, "world-model fork position")?;
        validate_vec3(object.velocity, "world-model fork velocity")?;
    }
    target_position(fork).map(|_| ())
}

fn extract_config(events: &[RunLogEvent]) -> Result<(String, WorldModelReferenceConfig)> {
    let starts = events
        .iter()
        .filter_map(|event| match event {
            RunLogEvent::RunStarted { config_hash, .. } => Some(config_hash),
            _ => None,
        })
        .collect::<Vec<_>>();
    let configs = events
        .iter()
        .filter_map(|event| match event {
            RunLogEvent::ConfigLogged {
                config_hash,
                config,
                ..
            } => Some((config_hash, config)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if starts.len() != 1 || configs.len() != 1 || starts[0] != configs[0].0 {
        bail!("world-model run must contain one consistently bound configuration");
    }
    let value = configs[0]
        .1
        .get("world_model_decision")
        .context("world-model configuration member is absent")?
        .clone();
    let config: WorldModelReferenceConfig = serde_json::from_value(value)
        .context("world-model configuration does not match its typed contract")?;
    Ok((starts[0].clone(), config))
}

fn require_forecast_flow_events(
    events: &[RunLogEvent],
    start: usize,
    end: usize,
    commit: &ForecastCommit,
) -> Result<()> {
    let flows = events[start..end]
        .iter()
        .filter_map(|event| match event {
            RunLogEvent::FlowPred {
                step,
                timestamp_ns,
                source,
                object_id,
                horizon_steps,
                flow,
                metadata,
                ..
            } if source == REFERENCE_MODEL_FAMILY => {
                Some((step, timestamp_ns, object_id, horizon_steps, flow, metadata))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if flows.len() != commit.forecasts.len() {
        bail!("forecast commit lacks one viewer-ready FlowPred per candidate");
    }
    let start_position = target_position(&commit.fork)?;
    for (flow_event, forecast) in flows.iter().zip(&commit.forecasts) {
        if *flow_event.0 != commit.fork.step
            || *flow_event.1 != commit.fork.timestamp_ns
            || flow_event.2 != &commit.fork.target_object_id
            || *flow_event.3 != commit.horizon_steps
            || flow_event.4.len() != 1
            || flow_event.5.get("candidate_id") != Some(&forecast.candidate_id)
            || flow_event.5.get("candidate_pool_sha256") != Some(&commit.candidate_pool_sha256)
            || flow_event.5.get("decision_index") != Some(&commit.fork.decision_index.to_string())
            || flow_event.5.get("forecast_stage").map(String::as_str) != Some("pre_oracle_commit")
            || !vec3_close(
                flow_event.4[0],
                subtract(forecast.predicted_endpoint, start_position),
            )
        {
            bail!("forecast FlowPred does not match the committed candidate forecast");
        }
    }
    Ok(())
}

fn require_bridge_execution(
    events: &[RunLogEvent],
    start: usize,
    end: usize,
    commit: &ForecastCommit,
    receipt: &WorldModelExecutionReceipt,
) -> Result<()> {
    let requests = events[start..end]
        .iter()
        .filter_map(|event| match event {
            RunLogEvent::BridgeRequest {
                method, payload, ..
            } => Some((method.as_str(), payload)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if requests.len() != 2 || requests[0].0 != "intervention.apply" || requests[1].0 != "sim.step" {
        bail!("world-model execution must contain one intervention and one step request");
    }
    let selected = commit
        .candidate_pool
        .iter()
        .find(|candidate| candidate.candidate_id == commit.selected_candidate_id)
        .context("world-model selected candidate is absent")?;
    let expected_intervention = json!({
        "intervention_type": "set_velocity",
        "payload": {
            "object_id": commit.fork.target_object_id,
            "velocity": selected.velocity,
        }
    });
    if requests[0].1 != &expected_intervention
        || requests[1].1 != &json!({ "dt": commit.dt_secs })
        || !vec3_close(receipt.executed_velocity, selected.velocity)
    {
        bail!("world-model bridge execution differs from the committed selection");
    }
    Ok(())
}

fn require_oracle_metrics(
    events: &[RunLogEvent],
    start: usize,
    end: usize,
    receipt: &WorldModelExecutionReceipt,
    oracle: &OracleLabelRecord,
) -> Result<()> {
    let metrics = events[start..end]
        .iter()
        .filter_map(|event| match event {
            RunLogEvent::EvaluationMetric {
                step,
                timestamp_ns,
                name,
                value,
                metadata,
            } if name.starts_with("world_model.") => {
                Some((step, timestamp_ns, name.as_str(), value, metadata))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let expected = [
        (
            "world_model.candidate_set_regret",
            oracle.candidate_set_regret,
        ),
        (
            "world_model.nominal_cost_minus_selected_cost",
            oracle.nominal_cost_minus_selected_cost,
        ),
    ];
    if metrics.len() != expected.len() {
        bail!("world-model oracle label lacks its two canonical decision metrics");
    }
    for (actual, expected) in metrics.iter().zip(expected) {
        if *actual.0 != receipt.resulting_step
            || *actual.1 != receipt.resulting_timestamp_ns
            || actual.2 != expected.0
            || !float_close(*actual.3, expected.1)
            || actual.4.len() != 1
            || actual.4.get("decision_index") != Some(&oracle.decision_index.to_string())
        {
            bail!("world-model decision metric differs from its oracle record");
        }
    }
    Ok(())
}

fn reject_mutations(events: &[RunLogEvent], start: usize, end: usize, phase: &str) -> Result<()> {
    if events[start..end].iter().any(|event| {
        matches!(
            event,
            RunLogEvent::BridgeRequest { method, .. }
                if matches!(
                    method.as_str(),
                    "sim.reset" | "sim.step" | "scene.set_object" | "intervention.apply"
                )
        )
    }) {
        bail!("world-model run mutated the canonical state {phase}");
    }
    Ok(())
}

fn require_exact_world_model_event_counts(events: &[RunLogEvent], decisions: usize) -> Result<()> {
    let label_count = events
        .iter()
        .filter(|event| matches!(event, RunLogEvent::LabelObserved { .. }))
        .count();
    let expected_labels = decisions
        .checked_mul(3)
        .context("world-model label projection overflow")?;
    if label_count != expected_labels {
        bail!(
            "world-model run contains {label_count} label records, expected exactly {expected_labels}"
        );
    }
    for name in [FORECAST_COMMIT_LABEL, ORACLE_LABEL, EXECUTION_RECEIPT_LABEL] {
        let count = events
            .iter()
            .filter(|event| {
                matches!(event, RunLogEvent::LabelObserved { name: event_name, .. } if event_name == name)
            })
            .count();
        if count != decisions {
            bail!("world-model run contains {count} {name} records, expected {decisions}");
        }
    }
    for metric_name in [
        "world_model.candidate_set_regret",
        "world_model.nominal_cost_minus_selected_cost",
    ] {
        let count = events
            .iter()
            .filter(|event| {
                matches!(event, RunLogEvent::EvaluationMetric { name, .. } if name == metric_name)
            })
            .count();
        if count != decisions {
            bail!("world-model run contains {count} {metric_name} metrics, expected {decisions}");
        }
    }
    let world_model_metric_count = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RunLogEvent::EvaluationMetric { name, .. }
                    if name.starts_with("world_model.")
            )
        })
        .count();
    let expected_world_model_metrics = decisions
        .checked_mul(2)
        .context("world-model metric projection overflow")?;
    if world_model_metric_count != expected_world_model_metrics {
        bail!(
            "world-model run contains {world_model_metric_count} world-model metrics, expected {expected_world_model_metrics}"
        );
    }
    let bridge_request_count = events
        .iter()
        .filter(|event| matches!(event, RunLogEvent::BridgeRequest { .. }))
        .count();
    let expected_bridge_requests = decisions
        .checked_mul(2)
        .context("world-model bridge-request projection overflow")?;
    if bridge_request_count != expected_bridge_requests {
        bail!(
            "world-model run contains {bridge_request_count} bridge requests, expected {expected_bridge_requests}"
        );
    }
    Ok(())
}

fn require_reference_run_envelope(
    events: &[RunLogEvent],
    config_hash: &str,
    config: &WorldModelReferenceConfig,
) -> Result<()> {
    let starts = events
        .iter()
        .filter_map(|event| match event {
            RunLogEvent::RunStarted {
                schema_version,
                run_id,
                timestamp_ns,
                config_hash,
                metadata,
            } => Some((schema_version, run_id, timestamp_ns, config_hash, metadata)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if starts.len() != 1
        || *starts[0].0 != RUN_LOG_SCHEMA_VERSION
        || starts[0].1 != REFERENCE_RUN_ID
        || *starts[0].2 != 0
        || starts[0].3 != config_hash
        || starts[0].4.len() != 1
        || starts[0].4.get("source").map(String::as_str) != Some(REFERENCE_SOURCE)
    {
        bail!("world-model run-start envelope is not canonical");
    }
    let expected_config = json!({
        "source": REFERENCE_SOURCE,
        "world_model_decision": config,
    });
    let logged_configs = events
        .iter()
        .filter_map(|event| match event {
            RunLogEvent::ConfigLogged {
                timestamp_ns,
                config_hash,
                config,
            } => Some((timestamp_ns, config_hash, config)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if logged_configs.len() != 1
        || *logged_configs[0].0 != 0
        || logged_configs[0].1 != config_hash
        || logged_configs[0].2 != &expected_config
    {
        bail!("world-model logged configuration envelope is not canonical");
    }
    Ok(())
}

fn require_label_envelope(
    event: &RunLogEvent,
    expected_name: &str,
    expected_step: u64,
    expected_timestamp_ns: u64,
    expected_decision_index: usize,
) -> Result<()> {
    let RunLogEvent::LabelObserved {
        step,
        timestamp_ns,
        name,
        metadata,
        ..
    } = event
    else {
        bail!("world-model typed label is not a label_observed event");
    };
    if name != expected_name
        || *step != expected_step
        || *timestamp_ns != expected_timestamp_ns
        || metadata.len() != 1
        || metadata.get("decision_index") != Some(&expected_decision_index.to_string())
    {
        bail!("world-model {expected_name} envelope is not canonical");
    }
    Ok(())
}

fn next_typed_label<T: for<'de> Deserialize<'de>>(
    events: &[RunLogEvent],
    start: usize,
    name: &str,
    decision_index: usize,
) -> Result<(usize, T)> {
    let index = next_label_index(events, start, name)
        .with_context(|| format!("missing {name} for decision {decision_index}"))?;
    let value = match &events[index] {
        RunLogEvent::LabelObserved { value, .. } => value.clone(),
        _ => unreachable!("next_label_index returns only LabelObserved events"),
    };
    let parsed = serde_json::from_value(value)
        .with_context(|| format!("invalid {name} for decision {decision_index}"))?;
    Ok((index, parsed))
}

fn next_label_index(events: &[RunLogEvent], start: usize, name: &str) -> Option<usize> {
    events.iter().enumerate().skip(start).find_map(|(index, event)| {
        matches!(event, RunLogEvent::LabelObserved { name: event_name, .. } if event_name == name)
            .then_some(index)
    })
}

fn compare_forecast_commits(actual: &ForecastCommit, expected: &ForecastCommit) -> Result<()> {
    if actual.schema != expected.schema
        || actual.fork != expected.fork
        || actual.fork_sha256 != expected.fork_sha256
        || actual.candidate_pool != expected.candidate_pool
        || actual.candidate_pool_sha256 != expected.candidate_pool_sha256
        || actual.model_sha256 != expected.model_sha256
        || !vec3_close(actual.goal_position, expected.goal_position)
        || actual.horizon_steps != expected.horizon_steps
        || !float_close(actual.dt_secs, expected.dt_secs)
        || actual.nominal_candidate_id != expected.nominal_candidate_id
        || actual.selected_candidate_id != expected.selected_candidate_id
        || actual.selection_rule != expected.selection_rule
        || actual.oracle_accessed != expected.oracle_accessed
        || actual.forecasts.len() != expected.forecasts.len()
    {
        bail!("world-model forecast commit differs from deterministic reconstruction");
    }
    for (actual, expected) in actual.forecasts.iter().zip(&expected.forecasts) {
        if actual.candidate_id != expected.candidate_id
            || !vec3_close(actual.predicted_endpoint, expected.predicted_endpoint)
            || !float_close(actual.predicted_cost, expected.predicted_cost)
        {
            bail!("world-model candidate forecast differs from deterministic reconstruction");
        }
    }
    Ok(())
}

fn compare_oracle_labels(actual: &OracleLabelRecord, expected: &OracleLabelRecord) -> Result<()> {
    if actual.schema != expected.schema
        || actual.decision_index != expected.decision_index
        || actual.fork_sha256 != expected.fork_sha256
        || actual.candidate_pool_sha256 != expected.candidate_pool_sha256
        || actual.oracle_best_candidate_id != expected.oracle_best_candidate_id
        || actual.selected_candidate_id != expected.selected_candidate_id
        || actual.reference_model_role != expected.reference_model_role
        || actual.outcomes.len() != expected.outcomes.len()
        || !float_close(actual.candidate_set_regret, expected.candidate_set_regret)
        || !float_close(
            actual.nominal_cost_minus_selected_cost,
            expected.nominal_cost_minus_selected_cost,
        )
    {
        bail!("world-model oracle label differs from independent reconstruction");
    }
    for (actual, expected) in actual.outcomes.iter().zip(&expected.outcomes) {
        if actual.candidate_id != expected.candidate_id
            || !vec3_close(actual.reference_endpoint, expected.reference_endpoint)
            || !float_close(actual.reference_cost, expected.reference_cost)
        {
            bail!("world-model candidate oracle outcome differs from reconstruction");
        }
    }
    Ok(())
}

fn compare_execution_receipts(
    actual: &WorldModelExecutionReceipt,
    expected: &WorldModelExecutionReceipt,
) -> Result<()> {
    if actual.schema != expected.schema
        || actual.decision_index != expected.decision_index
        || actual.selected_candidate_id != expected.selected_candidate_id
        || actual.resulting_step != expected.resulting_step
        || actual.resulting_timestamp_ns != expected.resulting_timestamp_ns
        || !vec3_close(actual.executed_velocity, expected.executed_velocity)
        || !vec3_close(actual.resulting_position, expected.resulting_position)
    {
        bail!("world-model execution receipt differs from the selected reference transition");
    }
    Ok(())
}

fn compare_models(actual: &AffineWorldModel, expected: &AffineWorldModel) -> Result<()> {
    if actual != expected {
        bail!("world-model receipt differs from exact regenerated training evidence");
    }
    Ok(())
}

fn invalid_report(issues: Vec<String>) -> WorldModelVerificationReport {
    WorldModelVerificationReport {
        schema: VERIFICATION_SCHEMA.to_string(),
        valid: false,
        decisions_verified: 0,
        candidates_verified: 0,
        action_sensitive_decisions: 0,
        final_position: [0.0; 3],
        summed_visited_fork_candidate_set_regret: 0.0,
        issues,
    }
}

fn sim_at_state(position: [f64; 3]) -> Result<DeterministicObjectSim> {
    let mut sim = DeterministicObjectSim::new();
    sim.upsert_object(SimObject {
        object_id: REFERENCE_OBJECT_ID.to_string(),
        pose: pid_runlog::Pose {
            position,
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        velocity: [0.0; 3],
    })?;
    Ok(sim)
}

fn world_model_fork(
    config: &WorldModelReferenceConfig,
    sim: &DeterministicObjectSim,
    decision_index: usize,
) -> WorldModelFork {
    WorldModelFork {
        decision_index,
        step: sim.step(),
        timestamp_ns: sim.timestamp_ns(),
        target_object_id: config.target_object_id.clone(),
        objects: snapshot_objects(sim),
    }
}

fn sim_from_fork(fork: &WorldModelFork) -> Result<DeterministicObjectSim> {
    DeterministicObjectSim::from_snapshot(fork.step, fork.timestamp_ns, &fork.objects)
}

fn apply_velocity(
    sim: &mut DeterministicObjectSim,
    object_id: &str,
    velocity: [f64; 3],
) -> Result<()> {
    sim.apply_intervention(
        "set_velocity",
        &json!({ "object_id": object_id, "velocity": velocity }),
    )?;
    Ok(())
}

fn snapshot_objects(sim: &DeterministicObjectSim) -> Vec<SimObjectSnapshot> {
    sim.objects()
        .map(|object| SimObjectSnapshot {
            object_id: object.object_id.clone(),
            pose: object.pose.clone(),
            velocity: object.velocity,
        })
        .collect()
}

fn target_position(fork: &WorldModelFork) -> Result<[f64; 3]> {
    fork.objects
        .iter()
        .find(|object| object.object_id == fork.target_object_id)
        .map(|object| object.pose.position)
        .context("world-model fork target object is absent")
}

fn object_position(sim: &DeterministicObjectSim, object_id: &str) -> Result<[f64; 3]> {
    sim.objects()
        .find(|object| object.object_id == object_id)
        .map(|object| object.pose.position)
        .with_context(|| format!("simulator object {object_id} is absent"))
}

fn selected_candidate(forecast: &ForecastCommit) -> Result<&WorldModelCandidate> {
    forecast
        .candidate_pool
        .iter()
        .find(|candidate| candidate.candidate_id == forecast.selected_candidate_id)
        .context("selected world-model candidate is absent from the committed pool")
}

fn expected_execution_receipt(
    config: &WorldModelReferenceConfig,
    forecast: &ForecastCommit,
) -> Result<WorldModelExecutionReceipt> {
    let selected = selected_candidate(forecast)?;
    let mut sim = sim_from_fork(&forecast.fork)?;
    apply_velocity(&mut sim, &forecast.fork.target_object_id, selected.velocity)?;
    sim.step_fixed(config.dt_secs)?;
    Ok(WorldModelExecutionReceipt {
        schema: WORLD_MODEL_DECISION_SCHEMA.to_string(),
        decision_index: forecast.fork.decision_index,
        selected_candidate_id: selected.candidate_id.clone(),
        executed_velocity: selected.velocity,
        resulting_step: sim.step(),
        resulting_timestamp_ns: sim.timestamp_ns(),
        resulting_position: object_position(&sim, &forecast.fork.target_object_id)?,
    })
}

fn require_bridge_success(
    response: pid_bridge::BridgeResponse,
    request: &BridgeRequest,
) -> Result<()> {
    if response.ok {
        Ok(())
    } else {
        bail!(
            "bridge rejected world-model request {}: {}",
            request.request_id,
            response.message.as_deref().unwrap_or("no message")
        )
    }
}

fn outcome_cost(outcomes: &[CandidateOracleOutcome], candidate_id: &str) -> Result<f64> {
    outcomes
        .iter()
        .find(|outcome| outcome.candidate_id == candidate_id)
        .map(|outcome| outcome.reference_cost)
        .with_context(|| format!("candidate {candidate_id} has no oracle outcome"))
}

fn minimum_cost_id<'a>(values: impl Iterator<Item = (&'a str, f64)>) -> Result<&'a str> {
    let mut best: Option<(&str, f64)> = None;
    for (id, cost) in values {
        if !cost.is_finite() {
            bail!("candidate cost for {id} is not finite");
        }
        if best.is_none_or(|(_, best_cost)| cost < best_cost) {
            best = Some((id, cost));
        }
    }
    best.map(|(id, _)| id)
        .context("candidate cost set is empty")
}

fn squared_distance(left: [f64; 3], right: [f64; 3]) -> Result<f64> {
    let value = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f64>();
    if value.is_finite() {
        Ok(value)
    } else {
        bail!("world-model squared distance is not finite")
    }
}

fn nonnegative_roundoff(value: f64) -> Result<f64> {
    if !value.is_finite() || value < -FLOAT_TOLERANCE {
        bail!("world-model candidate regret is invalid: {value}");
    }
    Ok(value.max(0.0))
}

fn solve_fixed_linear_system(
    mut matrix: [[f64; MODEL_FEATURES]; MODEL_FEATURES],
    mut rhs: [f64; MODEL_FEATURES],
) -> Option<[f64; MODEL_FEATURES]> {
    for pivot in 0..MODEL_FEATURES {
        let pivot_row = (pivot..MODEL_FEATURES).max_by(|left, right| {
            matrix[*left][pivot]
                .abs()
                .total_cmp(&matrix[*right][pivot].abs())
        })?;
        if matrix[pivot_row][pivot].abs() <= 1e-14 {
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
        for row in 0..MODEL_FEATURES {
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

fn checked_pow(base: usize, exponent: u32, name: &str) -> Result<usize> {
    base.checked_pow(exponent)
        .with_context(|| format!("{name} projection overflow"))
}

fn validate_vec3(value: [f64; 3], name: &str) -> Result<()> {
    if value.iter().any(|value| !value.is_finite()) {
        bail!("{name} must be finite");
    }
    Ok(())
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn clamp_vec3(value: [f64; 3], maximum: f64) -> [f64; 3] {
    value.map(|component| component.clamp(-maximum, maximum))
}

fn scale(value: [f64; 3], factor: f64) -> [f64; 3] {
    value.map(|component| component * factor)
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn float_close(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= FLOAT_TOLERANCE * scale
}

fn vec3_close(left: [f64; 3], right: [f64; 3]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| float_close(left, right))
}

fn decision_metadata(decision_index: usize) -> BTreeMap<String, String> {
    [("decision_index".to_string(), decision_index.to_string())]
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pid_runlog::RunLogWriter;
    use serde_json::Value;

    #[test]
    fn learned_reference_is_action_conditioned() {
        let config = reference_config().unwrap();
        let first = config
            .model
            .predict_next([0.0; 3], [0.3, 0.0, 0.0])
            .unwrap();
        let second = config
            .model
            .predict_next([0.0; 3], [-0.3, 0.0, 0.0])
            .unwrap();

        assert!(!vec3_close(first, second));
    }

    #[test]
    fn learned_reference_rejects_actions_outside_declared_domain() {
        let config = reference_config().unwrap();

        let error = config
            .model
            .predict_next([0.0; 3], [1.01, 0.0, 0.0])
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("outside the declared affine fixture domain"));
    }

    #[test]
    fn public_prediction_revalidates_the_model_receipt() {
        let mut model = reference_config().unwrap().model;
        model.schema = "unreviewed-model".to_string();

        let error = model.predict_next([0.0; 3], [0.0; 3]).unwrap_err();

        assert!(error
            .to_string()
            .contains("unsupported affine world-model receipt"));
    }

    #[test]
    fn public_prediction_rejects_a_noncanonical_training_digest() {
        let mut model = reference_config().unwrap().model;
        model.training_sha256 = "A".repeat(64);

        let error = model.predict_next([0.0; 3], [0.0; 3]).unwrap_err();

        assert!(error.to_string().contains("training receipt is incomplete"));
    }

    #[test]
    fn regenerated_model_comparison_is_exact() {
        let expected = reference_config().unwrap().model;
        let mut altered = expected.clone();
        altered.coefficients[0][0] += 1e-12;

        let error = compare_models(&altered, &expected).unwrap_err();

        assert!(error
            .to_string()
            .contains("differs from exact regenerated training evidence"));
    }

    #[test]
    fn reference_config_rejects_nearby_training_execution_contracts() {
        let mut config = reference_config().unwrap();
        config.dt_secs += 1e-12;

        let error = validate_reference_config(&config).unwrap_err();

        assert!(error
            .to_string()
            .contains("training and execution contracts disagree"));
    }

    #[test]
    fn reference_model_recovers_declared_transition() {
        let config = reference_config().unwrap();
        let predicted = config
            .model
            .predict_next([0.2, -0.1, 0.3], [0.25, -0.2, 0.1])
            .unwrap();

        assert!(vec3_close(predicted, [0.25, -0.14, 0.32]));
    }

    #[test]
    fn forecast_selection_does_not_read_oracle() {
        let config = reference_config().unwrap();
        let sim = reference_sim(&config).unwrap();
        let commit = propose_reference_decision(&config, &sim, 0)
            .unwrap()
            .commit_forecast()
            .unwrap();

        assert!(!commit.record().oracle_accessed);
    }

    #[test]
    fn fixed_pool_prediction_intervention_flips_selection() {
        let config = reference_config().unwrap();
        let sim = reference_sim(&config).unwrap();
        let proposed = propose_reference_decision(&config, &sim, 0).unwrap();
        let fork = proposed.fork.clone();
        let pool = proposed.candidate_pool.clone();
        let canonical = build_forecast_commit(&config, fork.clone(), pool.clone()).unwrap();

        // Hold the fork, candidate identities, actions, goal, horizon, and selector fixed. Change
        // only the learned transition's action response, then recompute predictions and scores.
        // This is a software-path intervention. It is not a scientifically admissible model.
        let mut intervened = config.clone();
        for output_axis in 0..MODEL_OUTPUTS {
            for action_axis in 0..MODEL_OUTPUTS {
                intervened.model.coefficients[output_axis][4 + action_axis] *= -1.0;
            }
        }
        intervened.model_sha256 = canonical_json_hash_v2(&intervened.model).unwrap();
        let counterfactual = build_forecast_commit(&intervened, fork, pool).unwrap();

        assert_eq!(
            canonical.candidate_pool_sha256,
            counterfactual.candidate_pool_sha256
        );
        assert_ne!(canonical.model_sha256, counterfactual.model_sha256);
        assert_ne!(
            canonical.selected_candidate_id,
            counterfactual.selected_candidate_id
        );
    }

    #[test]
    fn verifier_rejects_tampered_selection() {
        let mut events = reference_events().unwrap();
        for event in &mut events {
            if let RunLogEvent::LabelObserved { name, value, .. } = event {
                if name == FORECAST_COMMIT_LABEL {
                    value["selected_candidate_id"] = Value::String("nominal_x".to_string());
                    break;
                }
            }
        }

        let report = verify_world_model_events(&events).unwrap();
        assert!(!report.valid);
    }

    #[test]
    fn verifier_rejects_an_unregistered_label_side_channel() {
        let mut events = reference_events().unwrap();
        let first_commit = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    RunLogEvent::LabelObserved { name, .. } if name == FORECAST_COMMIT_LABEL
                )
            })
            .unwrap();
        events.insert(
            first_commit,
            RunLogEvent::LabelObserved {
                step: 0,
                timestamp_ns: 0,
                name: "world_model.unregistered_side_channel".to_string(),
                value: Value::Bool(true),
                metadata: BTreeMap::new(),
            },
        );

        let report = verify_world_model_events(&events).unwrap();
        assert!(!report.valid);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("label records")));
    }

    #[test]
    fn complete_reference_event_stream_verifies() {
        let events = reference_events().unwrap();

        let report = verify_world_model_events(&events).unwrap();
        assert!(report.valid, "{:?}", report.issues);
    }

    #[test]
    fn selected_execution_precedes_restored_fork_oracle_labeling() {
        let events = reference_events().unwrap();
        let first_commit = next_label_index(&events, 0, FORECAST_COMMIT_LABEL).unwrap();
        let first_receipt =
            next_label_index(&events, first_commit + 1, EXECUTION_RECEIPT_LABEL).unwrap();
        let first_oracle = next_label_index(&events, first_receipt + 1, ORACLE_LABEL).unwrap();

        assert!(first_commit < first_receipt);
        assert!(first_receipt < first_oracle);
        assert!(events[first_commit + 1..first_receipt]
            .iter()
            .any(|event| matches!(event, RunLogEvent::BridgeRequest { .. })));
        let RunLogEvent::LabelObserved {
            step: receipt_step,
            timestamp_ns: receipt_time,
            ..
        } = &events[first_receipt]
        else {
            panic!("execution receipt must be a label event");
        };
        let RunLogEvent::LabelObserved {
            step: oracle_step,
            timestamp_ns: oracle_time,
            ..
        } = &events[first_oracle]
        else {
            panic!("oracle record must be a label event");
        };
        assert_eq!((receipt_step, receipt_time), (oracle_step, oracle_time));
    }

    #[test]
    fn verifier_rejects_extra_reference_forecast_flow() {
        let mut events = reference_events().unwrap();
        let extra = events
            .iter()
            .find(|event| {
                matches!(
                    event,
                    RunLogEvent::FlowPred { source, .. } if source == REFERENCE_MODEL_FAMILY
                )
            })
            .cloned()
            .unwrap();
        let terminal = events
            .iter()
            .position(|event| matches!(event, RunLogEvent::RunEnded { .. }))
            .unwrap();
        events.insert(terminal, extra);

        let report = verify_world_model_events(&events).unwrap();

        assert!(!report.valid);
        assert!(!report.issues.is_empty());
    }

    #[test]
    fn execution_rejects_a_session_that_drifted_from_the_committed_fork() {
        let config = reference_config().unwrap();
        let proposal_sim = reference_sim(&config).unwrap();
        let commit = propose_reference_decision(&config, &proposal_sim, 0)
            .unwrap()
            .commit_forecast()
            .unwrap();
        let writer = RunLogWriter::new(Vec::new());
        let drifted_sim = sim_at_state([0.1, 0.0, 0.0]).unwrap();
        let mut session = SimBridgeSession::with_run_id(writer, drifted_sim, REFERENCE_RUN_ID);
        let published = record_forecast_commit(&mut session, commit).unwrap();
        let actor = Actor {
            actor_type: pid_runlog::ActorType::Script,
            actor_id: REFERENCE_SOURCE.to_string(),
            session_id: Some(REFERENCE_RUN_ID.to_string()),
        };

        let error = execute_published_decision(&mut session, published, &actor).unwrap_err();

        assert!(error
            .to_string()
            .contains("session state differs from the committed fork"));
    }

    fn reference_events() -> Result<Vec<RunLogEvent>> {
        let config = reference_config()?;
        let config_value = json!({
            "source": REFERENCE_SOURCE,
            "world_model_decision": &config,
        });
        let config_hash = canonical_json_hash_v2(&config_value)?;
        let mut writer = RunLogWriter::new(Vec::new());
        writer.append(&RunLogEvent::RunStarted {
            schema_version: RUN_LOG_SCHEMA_VERSION,
            run_id: REFERENCE_RUN_ID.to_string(),
            timestamp_ns: 0,
            config_hash: config_hash.clone(),
            metadata: [("source".to_string(), REFERENCE_SOURCE.to_string())]
                .into_iter()
                .collect(),
        })?;
        writer.append(&RunLogEvent::ConfigLogged {
            timestamp_ns: 0,
            config_hash,
            config: config_value,
        })?;
        let sim = reference_sim(&config)?;
        writer.append(&sim.snapshot_event())?;
        let mut session = SimBridgeSession::with_run_id(writer, sim, REFERENCE_RUN_ID);
        let actor = Actor {
            actor_type: pid_runlog::ActorType::Script,
            actor_id: REFERENCE_SOURCE.to_string(),
            session_id: Some(REFERENCE_RUN_ID.to_string()),
        };
        for decision_index in 0..config.decisions {
            let commit = propose_reference_session_decision(&config, &session, decision_index)?
                .commit_forecast()?;
            let published = record_forecast_commit(&mut session, commit)?;
            let executed = execute_published_decision(&mut session, published, &actor)?;
            let labeled = executed.label_oracle()?;
            record_oracle_label(&mut session, labeled)?;
        }
        session.finish_run(RunStatus::Succeeded, Some("reference complete".to_string()))?;
        let bytes = session.into_inner();
        read_events(Cursor::new(bytes))
    }
}
