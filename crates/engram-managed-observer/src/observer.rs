use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::canonical::{canonical_json, sha256_domain, sha256_value, to_value};
use crate::contract::{
    FinishRequest, ObserveRequest, ObserverOutcome, ObserverResponse, PrepareRequest,
    RuntimeLifecycleReceiptBinding, RuntimeLifecycleScalar, AUTHORITY, FINISH_REQUEST_SCHEMA_ID,
    FINISH_RESPONSE_SCHEMA_ID, MAX_CHANNELS, MAX_FAULT_CODE_BYTES, MAX_REASON_BYTES, MAX_STEPS,
    OBSERVE_REQUEST_SCHEMA_ID, OBSERVE_RESPONSE_SCHEMA_ID, PREPARE_REQUEST_SCHEMA_ID,
    PREPARE_RESPONSE_SCHEMA_ID,
};

const ABSENT_STATE_DOMAIN: &str = "prisoma-managed-observer-absent-state-v1";
const EMPTY_TRANSCRIPT_DOMAIN: &str = "prisoma-managed-observer-empty-transcript-v1";
const RUN_IDENTITY_DOMAIN: &str = "prisoma-managed-observer-run-v1";
const STATE_DOMAIN: &str = "prisoma-managed-observer-state-v1";
const RECEIPT_DOMAIN: &str = "prisoma-managed-observer-receipt-v1";
const TRANSCRIPT_DOMAIN: &str = "prisoma-managed-observer-transcript-v1";
const CLOSED_LOOP_DIGEST_CANONICALIZATION: &str = "engram.managed-runtime-json.v1";
const CLOSED_LOOP_STEP_SCHEMA: &str = "engram.extension-closed-loop-step-receipt.v2";
const CLOSED_LOOP_RUN_SCHEMA: &str = "engram.extension-closed-loop-run-receipt.v2";
const CLOSED_LOOP_CLEANUP_SCHEMA: &str = "engram.closed-loop-cleanup.v2";
const CLOSED_LOOP_EXECUTION_SCHEMA: &str = "engram.closed-loop-neural-execution-binding.v1";
const MAX_STEP_DURATION_TICS: u64 = 10_000_000;
const MAX_SIMULATION_TIME_TICS: u64 = MAX_STEPS * MAX_STEP_DURATION_TICS;

/// Exact logical-clock and causal schedule projected by Engram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosedLoopTimebase {
    pub schema_version: String,
    pub tic_unit: String,
    pub runtime_step_duration_tics: u64,
    pub neural_step_duration_tics: u64,
    pub clock_relation: String,
    pub coupling: String,
    pub causality_policy: String,
    pub dispatch_order: String,
    pub observation_sample_phase: String,
    pub action_application: String,
}

/// One exact Engram closed-loop step receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceStepReceipt {
    pub schema_version: String,
    pub study_run_id: String,
    pub step_index: u64,
    pub step_id: String,
    pub input_snapshot_sha256: String,
    pub neural_request_sha256: String,
    pub neural_result_sha256: String,
    pub provider_execution_scope: String,
    pub provider_execution_sha256: String,
    pub admitted_action_sha256: String,
    pub runtime_request_sha256: String,
    pub output_snapshot_sha256: String,
    pub fault_codes: Vec<String>,
    pub receipt_sha256: String,
}

/// One exact Engram component-cleanup receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupReceipt {
    pub schema_version: String,
    pub component: String,
    pub owner_identity_sha256: String,
    pub mode: String,
    pub attempted: bool,
    pub confirmed: bool,
    pub containment_empty: bool,
    pub reason_code: String,
    pub runtime_lifecycle: Option<RuntimeLifecycleReceiptBinding>,
    pub provider_terminal_receipt_sha256: Option<String>,
    pub provider_lifecycle_receipt_sha256: Option<String>,
    pub receipt_sha256: String,
}

/// One exact Engram binding for every host-accepted neural execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeuralExecutionReceiptBinding {
    pub schema_version: String,
    pub step_index: u64,
    pub step_id: String,
    pub neural_request_sha256: String,
    pub neural_result_sha256: String,
    pub provider_execution_scope: String,
    pub provider_execution_sha256: String,
    pub binding_sha256: String,
}

/// One exact Engram terminal closed-loop receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRunReceipt {
    pub schema_version: String,
    pub digest_canonicalization: String,
    pub study_run_id: String,
    pub study_definition_sha256: String,
    pub closed_loop_definition_sha256: String,
    pub runtime_binding_sha256: String,
    pub runtime_adapter_configuration_sha256: String,
    pub neural_provider_identity_sha256: String,
    pub timebase: ClosedLoopTimebase,
    pub planned_step_count: u64,
    pub runtime_deadline_enforcement: String,
    pub neural_deadline_enforcement: String,
    pub neural_preparation_sha256: Option<String>,
    pub neural_session_receipt_sha256: Option<String>,
    pub neural_durable_evidence_profile: String,
    pub initial_snapshot_sha256: Option<String>,
    pub last_verified_simulation_time_tics: Option<u64>,
    pub runtime_progress_disposition: String,
    pub steps: Vec<SourceStepReceipt>,
    pub neural_executions: Vec<NeuralExecutionReceiptBinding>,
    pub runtime_finish_sha256: Option<String>,
    pub runtime_lifecycle: Option<RuntimeLifecycleReceiptBinding>,
    pub cleanup: Vec<CleanupReceipt>,
    pub status: String,
    pub primary_reason_code: String,
    pub terminal_reason_code: String,
    pub cleanup_complete: bool,
    pub transcript_sha256: String,
    pub simulator_only: bool,
    pub physical_actuation: bool,
    pub ncp_qualified: bool,
    pub scientific_authority: bool,
    pub is_paper_local_evidence: bool,
    pub calibrated_posterior: bool,
    pub receipt_sha256: String,
}

/// Bounded semantic failure from the observer state machine.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ObserverError {
    #[error("observer input is invalid")]
    InvalidInput,
    #[error("an observer run is already active")]
    RunAlreadyActive,
    #[error("there is no active observer run")]
    NoActiveRun,
    #[error("the observer run has already finished")]
    RunFinished,
    #[error("the source run identity differs from the prepared identity")]
    RunIdentityMismatch,
    #[error("the source channel roster differs from the prepared roster")]
    ChannelRosterMismatch,
    #[error("the source step is not the next expected step")]
    StepOutOfOrder,
    #[error("the prepared step budget is exhausted")]
    StepBudgetExhausted,
    #[error("the source step identifier is invalid")]
    SourceStepIdMismatch,
    #[error("the source step receipt digest is invalid")]
    SourceStepDigestMismatch,
    #[error("the source snapshot chain differs from the accepted step chain")]
    SourceSnapshotMismatch,
    #[error("the source cleanup receipt digest is invalid")]
    SourceCleanupDigestMismatch,
    #[error("the source transcript digest is invalid")]
    SourceTranscriptDigestMismatch,
    #[error("the source run receipt digest is invalid")]
    SourceRunDigestMismatch,
    #[error("the source authority boundary is invalid")]
    SourceAuthorityMismatch,
    #[error("the source terminal contract is invalid")]
    SourceTerminalMismatch,
    #[error("observer canonicalization failed")]
    Canonicalization,
    #[error("a panic was contained at the observer boundary")]
    InternalPanicContained,
}

impl ObserverError {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid-input",
            Self::RunAlreadyActive => "run-already-active",
            Self::NoActiveRun => "no-active-run",
            Self::RunFinished => "run-finished",
            Self::RunIdentityMismatch => "run-identity-mismatch",
            Self::ChannelRosterMismatch => "channel-roster-mismatch",
            Self::StepOutOfOrder => "step-out-of-order",
            Self::StepBudgetExhausted => "step-budget-exhausted",
            Self::SourceStepIdMismatch => "source-step-id-mismatch",
            Self::SourceStepDigestMismatch => "source-step-digest-mismatch",
            Self::SourceSnapshotMismatch => "source-snapshot-mismatch",
            Self::SourceCleanupDigestMismatch => "source-cleanup-digest-mismatch",
            Self::SourceTranscriptDigestMismatch => "source-transcript-digest-mismatch",
            Self::SourceRunDigestMismatch => "source-run-digest-mismatch",
            Self::SourceAuthorityMismatch => "source-authority-mismatch",
            Self::SourceTerminalMismatch => "source-terminal-mismatch",
            Self::Canonicalization => "canonicalization-failed",
            Self::InternalPanicContained => "internal-panic-contained",
        }
    }

    pub(crate) fn outcome(self) -> ObserverOutcome {
        match self {
            Self::Canonicalization | Self::InternalPanicContained => ObserverOutcome::Failed,
            _ => ObserverOutcome::Rejected,
        }
    }
}

#[derive(Debug)]
struct ActiveRun {
    study_run_id: String,
    study_definition_sha256: String,
    closed_loop_definition_sha256: String,
    runtime_binding_sha256: String,
    runtime_adapter_configuration_sha256: String,
    neural_provider_identity_sha256: String,
    channel_ids: Vec<String>,
    subject_ids: Vec<String>,
    planned_step_count: u64,
    max_steps: u64,
    run_identity_sha256: String,
    steps: Vec<SourceStepReceipt>,
    state_sha256: String,
    transcript_sha256: String,
    cumulative_fault_count: u64,
}

impl Drop for ActiveRun {
    fn drop(&mut self) {
        self.channel_ids.clear();
        self.subject_ids.clear();
        self.steps.clear();
        self.study_run_id.clear();
        self.study_definition_sha256.clear();
        self.closed_loop_definition_sha256.clear();
        self.runtime_binding_sha256.clear();
        self.runtime_adapter_configuration_sha256.clear();
        self.neural_provider_identity_sha256.clear();
        self.run_identity_sha256.clear();
        self.state_sha256.clear();
        self.transcript_sha256.clear();
        self.planned_step_count = 0;
        self.max_steps = 0;
        self.cumulative_fault_count = 0;
    }
}

#[derive(Debug)]
enum Phase {
    Idle,
    Active(Box<ActiveRun>),
    Cleared,
}

/// Stateful read-only observer for one managed-runtime generation.
#[derive(Debug)]
pub struct ObserverRuntime {
    phase: Phase,
}

impl ObserverRuntime {
    /// Construct an observer with no active source run.
    pub fn new() -> Self {
        Self { phase: Phase::Idle }
    }

    /// Bind one immutable source run and its ordered channel roster.
    ///
    /// # Errors
    ///
    /// Returns [`ObserverError`] for invalid identities, roster drift, or an
    /// illegal lifecycle transition.
    pub fn prepare(&mut self, request: PrepareRequest) -> Result<ObserverResponse, ObserverError> {
        match self.phase {
            Phase::Idle => {}
            Phase::Active(_) => return Err(ObserverError::RunAlreadyActive),
            Phase::Cleared => return Err(ObserverError::RunFinished),
        }
        validate_prepare(&request)?;
        let request_sha256 = request_sha256(&request)?;
        let prior_state_sha256 = absent_state_sha256()?;
        let prior_transcript_sha256 = empty_transcript_sha256()?;
        let request_bytes = canonical_bytes(&request)?;
        let run_identity_sha256 = sha256_domain(RUN_IDENTITY_DOMAIN, &[&request_bytes])
            .map_err(|_| ObserverError::Canonicalization)?;
        let state_sha256 = sha256_domain(
            STATE_DOMAIN,
            &[
                run_identity_sha256.as_bytes(),
                prior_state_sha256.as_bytes(),
                request_sha256.as_bytes(),
            ],
        )
        .map_err(|_| ObserverError::Canonicalization)?;
        let receipt_sha256 = observer_receipt_sha256(
            PREPARE_RESPONSE_SCHEMA_ID,
            ObserverOutcome::Succeeded,
            "prepared",
            &request.study_run_id,
            0,
            request.channel_ids.len() as u64,
            0,
            0,
            &request_sha256,
            &prior_state_sha256,
            &state_sha256,
            None,
            false,
            false,
            AUTHORITY,
            "host-declared-projection",
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        )?;
        let transcript_sha256 = advance_transcript(&prior_transcript_sha256, &receipt_sha256)?;
        let response = build_response(ResponseParts {
            schema_version: PREPARE_RESPONSE_SCHEMA_ID,
            outcome: ObserverOutcome::Succeeded,
            reason: "prepared",
            study_run_id: &request.study_run_id,
            step_index: 0,
            channel_count: request.channel_ids.len() as u64,
            fault_count: 0,
            cumulative_fault_count: 0,
            source_receipt_sha256: None,
            prior_state_sha256: &prior_state_sha256,
            state_sha256: &state_sha256,
            request_sha256: &request_sha256,
            receipt_sha256: &receipt_sha256,
            transcript_sha256: &transcript_sha256,
            terminal: false,
            state_cleared: false,
        });
        self.phase = Phase::Active(Box::new(ActiveRun {
            study_run_id: request.study_run_id,
            study_definition_sha256: request.study_definition_sha256,
            closed_loop_definition_sha256: request.closed_loop_definition_sha256,
            runtime_binding_sha256: request.runtime_binding_sha256,
            runtime_adapter_configuration_sha256: request.runtime_adapter_configuration_sha256,
            neural_provider_identity_sha256: request.neural_provider_identity_sha256,
            channel_ids: request.channel_ids,
            subject_ids: request.subject_ids,
            planned_step_count: request.planned_step_count,
            max_steps: request.max_steps,
            run_identity_sha256,
            steps: Vec::new(),
            state_sha256,
            transcript_sha256,
            cumulative_fault_count: 0,
        }));
        Ok(response)
    }

    /// Verify and admit one exact next-step receipt.
    ///
    /// # Errors
    ///
    /// Returns [`ObserverError`] for replay, gaps, digest drift, roster drift,
    /// or an illegal lifecycle transition.
    pub fn observe(&mut self, request: ObserveRequest) -> Result<ObserverResponse, ObserverError> {
        let active = match &self.phase {
            Phase::Active(active) => active,
            Phase::Idle => return Err(ObserverError::NoActiveRun),
            Phase::Cleared => return Err(ObserverError::RunFinished),
        };
        validate_observe(active, &request)?;
        let source_receipt = SourceStepReceipt {
            schema_version: CLOSED_LOOP_STEP_SCHEMA.to_owned(),
            study_run_id: request.study_run_id.clone(),
            step_index: request.step_index,
            step_id: request.step_id.clone(),
            input_snapshot_sha256: request.input_snapshot_sha256.clone(),
            neural_request_sha256: request.neural_request_sha256.clone(),
            neural_result_sha256: request.neural_result_sha256.clone(),
            provider_execution_scope: request.provider_execution_scope.clone(),
            provider_execution_sha256: request.provider_execution_sha256.clone(),
            admitted_action_sha256: request.admitted_action_sha256.clone(),
            runtime_request_sha256: request.runtime_request_sha256.clone(),
            output_snapshot_sha256: request.output_snapshot_sha256.clone(),
            fault_codes: request.fault_codes.clone(),
            receipt_sha256: request.source_receipt_sha256.clone(),
        };
        if source_step_receipt_sha256(&source_receipt)? != source_receipt.receipt_sha256 {
            return Err(ObserverError::SourceStepDigestMismatch);
        }
        let request_sha256 = request_sha256(&request)?;
        let prior_state_sha256 = active.state_sha256.clone();
        let state_sha256 = sha256_domain(
            STATE_DOMAIN,
            &[
                active.run_identity_sha256.as_bytes(),
                prior_state_sha256.as_bytes(),
                source_receipt.receipt_sha256.as_bytes(),
            ],
        )
        .map_err(|_| ObserverError::Canonicalization)?;
        let fault_count = request
            .fault_codes
            .iter()
            .filter(|code| code.as_str() != "none")
            .count() as u64;
        let cumulative_fault_count = active
            .cumulative_fault_count
            .checked_add(fault_count)
            .ok_or(ObserverError::InvalidInput)?;
        let receipt_sha256 = observer_receipt_sha256(
            OBSERVE_RESPONSE_SCHEMA_ID,
            ObserverOutcome::Succeeded,
            "observed",
            &request.study_run_id,
            request.step_index,
            active.channel_ids.len() as u64,
            fault_count,
            cumulative_fault_count,
            &request_sha256,
            &prior_state_sha256,
            &state_sha256,
            Some(&source_receipt.receipt_sha256),
            false,
            false,
            AUTHORITY,
            "host-declared-projection",
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        )?;
        let transcript_sha256 = advance_transcript(&active.transcript_sha256, &receipt_sha256)?;
        let response = build_response(ResponseParts {
            schema_version: OBSERVE_RESPONSE_SCHEMA_ID,
            outcome: ObserverOutcome::Succeeded,
            reason: "observed",
            study_run_id: &request.study_run_id,
            step_index: request.step_index,
            channel_count: active.channel_ids.len() as u64,
            fault_count,
            cumulative_fault_count,
            source_receipt_sha256: Some(&source_receipt.receipt_sha256),
            prior_state_sha256: &prior_state_sha256,
            state_sha256: &state_sha256,
            request_sha256: &request_sha256,
            receipt_sha256: &receipt_sha256,
            transcript_sha256: &transcript_sha256,
            terminal: false,
            state_cleared: false,
        });
        let Phase::Active(active) = &mut self.phase else {
            return Err(ObserverError::NoActiveRun);
        };
        active.steps.push(source_receipt);
        active.state_sha256 = state_sha256;
        active.transcript_sha256 = transcript_sha256;
        active.cumulative_fault_count = cumulative_fault_count;
        Ok(response)
    }

    /// Verify the terminal source receipt and clear retained run data.
    ///
    /// # Errors
    ///
    /// Returns [`ObserverError`] for incomplete lineage, a source digest
    /// mismatch, authority drift, or an illegal lifecycle transition.
    pub fn finish(&mut self, request: FinishRequest) -> Result<ObserverResponse, ObserverError> {
        let active = match &self.phase {
            Phase::Active(active) => active,
            Phase::Idle => return Err(ObserverError::NoActiveRun),
            Phase::Cleared => return Err(ObserverError::RunFinished),
        };
        let runtime_lifecycle = decode_runtime_lifecycle(&request.runtime_lifecycle_values)?;
        let timebase = decode_timebase(&request.timebase_values)?;
        validate_finish_identity(active, &request, &timebase)?;
        let neural_executions = neural_execution_receipts(active, &request)?;
        let cleanup = cleanup_receipts(&request, runtime_lifecycle.clone())?;
        let source_receipt = SourceRunReceipt {
            schema_version: CLOSED_LOOP_RUN_SCHEMA.to_owned(),
            digest_canonicalization: request.digest_canonicalization.clone(),
            study_run_id: request.study_run_id.clone(),
            study_definition_sha256: active.study_definition_sha256.clone(),
            closed_loop_definition_sha256: active.closed_loop_definition_sha256.clone(),
            runtime_binding_sha256: active.runtime_binding_sha256.clone(),
            runtime_adapter_configuration_sha256: active
                .runtime_adapter_configuration_sha256
                .clone(),
            neural_provider_identity_sha256: active.neural_provider_identity_sha256.clone(),
            timebase,
            planned_step_count: request.planned_step_count,
            runtime_deadline_enforcement: request.runtime_deadline_enforcement.clone(),
            neural_deadline_enforcement: request.neural_deadline_enforcement.clone(),
            neural_preparation_sha256: request.neural_preparation_sha256.clone(),
            neural_session_receipt_sha256: request.neural_session_receipt_sha256.clone(),
            neural_durable_evidence_profile: request.neural_durable_evidence_profile.clone(),
            initial_snapshot_sha256: request.initial_snapshot_sha256.clone(),
            last_verified_simulation_time_tics: request.last_verified_simulation_time_tics,
            runtime_progress_disposition: request.runtime_progress_disposition.clone(),
            steps: active.steps.clone(),
            neural_executions,
            runtime_finish_sha256: request.runtime_finish_sha256.clone(),
            runtime_lifecycle,
            cleanup,
            status: request.source_status.clone(),
            primary_reason_code: request.primary_reason_code.clone(),
            terminal_reason_code: request.terminal_reason_code.clone(),
            cleanup_complete: request.cleanup_complete,
            transcript_sha256: request.source_transcript_sha256.clone(),
            simulator_only: request.simulator_only,
            physical_actuation: request.physical_actuation,
            ncp_qualified: request.ncp_qualified,
            scientific_authority: request.scientific_authority,
            is_paper_local_evidence: request.is_paper_local_evidence,
            calibrated_posterior: request.calibrated_posterior,
            receipt_sha256: request.source_run_receipt_sha256.clone(),
        };
        validate_source_run_receipt(&source_receipt)?;
        let request_sha256 = request_sha256(&request)?;
        let prior_state_sha256 = active.state_sha256.clone();
        let state_sha256 = sha256_domain(
            STATE_DOMAIN,
            &[
                active.run_identity_sha256.as_bytes(),
                prior_state_sha256.as_bytes(),
                source_receipt.receipt_sha256.as_bytes(),
            ],
        )
        .map_err(|_| ObserverError::Canonicalization)?;
        let receipt_sha256 = observer_receipt_sha256(
            FINISH_RESPONSE_SCHEMA_ID,
            ObserverOutcome::Succeeded,
            "finished",
            &request.study_run_id,
            request.step_count,
            active.channel_ids.len() as u64,
            0,
            active.cumulative_fault_count,
            &request_sha256,
            &prior_state_sha256,
            &state_sha256,
            Some(&source_receipt.receipt_sha256),
            true,
            true,
            AUTHORITY,
            "host-declared-projection",
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        )?;
        let transcript_sha256 = advance_transcript(&active.transcript_sha256, &receipt_sha256)?;
        let response = build_response(ResponseParts {
            schema_version: FINISH_RESPONSE_SCHEMA_ID,
            outcome: ObserverOutcome::Succeeded,
            reason: "finished",
            study_run_id: &request.study_run_id,
            step_index: request.step_count,
            channel_count: active.channel_ids.len() as u64,
            fault_count: 0,
            cumulative_fault_count: active.cumulative_fault_count,
            source_receipt_sha256: Some(&source_receipt.receipt_sha256),
            prior_state_sha256: &prior_state_sha256,
            state_sha256: &state_sha256,
            request_sha256: &request_sha256,
            receipt_sha256: &receipt_sha256,
            transcript_sha256: &transcript_sha256,
            terminal: true,
            state_cleared: true,
        });
        self.phase = Phase::Cleared;
        Ok(response)
    }

    pub(crate) fn error_response<T: Serialize>(
        &self,
        response_schema: &str,
        request: &T,
        study_run_id: &str,
        step_index: u64,
        error: ObserverError,
    ) -> Result<ObserverResponse, ObserverError> {
        let request_sha256 = request_sha256(request)?;
        let (state_sha256, transcript_sha256, channel_count, cumulative_fault_count) =
            match &self.phase {
                Phase::Active(active) => (
                    active.state_sha256.clone(),
                    active.transcript_sha256.clone(),
                    active.channel_ids.len() as u64,
                    active.cumulative_fault_count,
                ),
                Phase::Idle | Phase::Cleared => {
                    (absent_state_sha256()?, empty_transcript_sha256()?, 0, 0)
                }
            };
        let receipt_sha256 = observer_receipt_sha256(
            response_schema,
            error.outcome(),
            error.reason(),
            study_run_id,
            step_index,
            channel_count,
            0,
            cumulative_fault_count,
            &request_sha256,
            &state_sha256,
            &state_sha256,
            None,
            false,
            matches!(self.phase, Phase::Cleared),
            AUTHORITY,
            "host-declared-projection",
            false,
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
        )?;
        Ok(build_response(ResponseParts {
            schema_version: response_schema,
            outcome: error.outcome(),
            reason: error.reason(),
            study_run_id,
            step_index,
            channel_count,
            fault_count: 0,
            cumulative_fault_count,
            source_receipt_sha256: None,
            prior_state_sha256: &state_sha256,
            state_sha256: &state_sha256,
            request_sha256: &request_sha256,
            receipt_sha256: &receipt_sha256,
            transcript_sha256: &transcript_sha256,
            terminal: false,
            state_cleared: matches!(self.phase, Phase::Cleared),
        }))
    }

    pub(crate) fn clear(&mut self) {
        self.phase = Phase::Cleared;
    }

    pub(crate) fn is_cleared(&self) -> bool {
        matches!(self.phase, Phase::Cleared)
    }
}

impl Default for ObserverRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ObserverRuntime {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Derive Engram's stable closed-loop step identifier.
///
/// # Errors
///
/// Returns [`ObserverError::Canonicalization`] if canonical encoding fails.
pub fn closed_loop_step_id(run_id: &str, step_index: u64) -> Result<String, ObserverError> {
    let value = json!({
        "domain": "engram-extension-closed-loop-step-v2",
        "run_id": run_id,
        "step_index": step_index,
    });
    let digest = sha256_value(&value).map_err(|_| ObserverError::Canonicalization)?;
    Ok(format!("step_{}", &digest[..32]))
}

/// Recompute an Engram step receipt digest from all source fields.
///
/// # Errors
///
/// Returns [`ObserverError::Canonicalization`] if canonical encoding fails.
pub fn source_step_receipt_sha256(receipt: &SourceStepReceipt) -> Result<String, ObserverError> {
    let value = json!({
        "schema_version": receipt.schema_version,
        "study_run_id": receipt.study_run_id,
        "step_index": receipt.step_index,
        "step_id": receipt.step_id,
        "input_snapshot_sha256": receipt.input_snapshot_sha256,
        "neural_request_sha256": receipt.neural_request_sha256,
        "neural_result_sha256": receipt.neural_result_sha256,
        "provider_execution_scope": receipt.provider_execution_scope,
        "provider_execution_sha256": receipt.provider_execution_sha256,
        "admitted_action_sha256": receipt.admitted_action_sha256,
        "runtime_request_sha256": receipt.runtime_request_sha256,
        "output_snapshot_sha256": receipt.output_snapshot_sha256,
        "fault_codes": receipt.fault_codes,
    });
    sha256_value(&value).map_err(|_| ObserverError::Canonicalization)
}

fn neural_execution_binding_sha256(
    receipt: &NeuralExecutionReceiptBinding,
) -> Result<String, ObserverError> {
    let value = json!({
        "schema_version": receipt.schema_version,
        "step_index": receipt.step_index,
        "step_id": receipt.step_id,
        "neural_request_sha256": receipt.neural_request_sha256,
        "neural_result_sha256": receipt.neural_result_sha256,
        "provider_execution_scope": receipt.provider_execution_scope,
        "provider_execution_sha256": receipt.provider_execution_sha256,
    });
    sha256_value(&value).map_err(|_| ObserverError::Canonicalization)
}

/// Recompute an Engram cleanup receipt digest from all source fields.
///
/// # Errors
///
/// Returns [`ObserverError::Canonicalization`] if canonical encoding fails.
pub fn source_cleanup_receipt_sha256(receipt: &CleanupReceipt) -> Result<String, ObserverError> {
    let value = json!({
        "schema_version": receipt.schema_version,
        "component": receipt.component,
        "owner_identity_sha256": receipt.owner_identity_sha256,
        "mode": receipt.mode,
        "attempted": receipt.attempted,
        "confirmed": receipt.confirmed,
        "containment_empty": receipt.containment_empty,
        "reason_code": receipt.reason_code,
        "runtime_lifecycle": receipt.runtime_lifecycle,
        "provider_terminal_receipt_sha256": receipt.provider_terminal_receipt_sha256,
        "provider_lifecycle_receipt_sha256": receipt.provider_lifecycle_receipt_sha256,
    });
    sha256_value(&value).map_err(|_| ObserverError::Canonicalization)
}

/// Recompute an Engram terminal run receipt digest from all source fields.
///
/// # Errors
///
/// Returns [`ObserverError::Canonicalization`] if canonical encoding fails.
pub fn source_run_receipt_sha256(receipt: &SourceRunReceipt) -> Result<String, ObserverError> {
    let value = json!({
        "schema_version": receipt.schema_version,
        "digest_canonicalization": receipt.digest_canonicalization,
        "study_run_id": receipt.study_run_id,
        "study_definition_sha256": receipt.study_definition_sha256,
        "closed_loop_definition_sha256": receipt.closed_loop_definition_sha256,
        "runtime_binding_sha256": receipt.runtime_binding_sha256,
        "runtime_adapter_configuration_sha256": receipt.runtime_adapter_configuration_sha256,
        "neural_provider_identity_sha256": receipt.neural_provider_identity_sha256,
        "timebase": receipt.timebase,
        "planned_step_count": receipt.planned_step_count,
        "runtime_deadline_enforcement": receipt.runtime_deadline_enforcement,
        "neural_deadline_enforcement": receipt.neural_deadline_enforcement,
        "neural_preparation_sha256": receipt.neural_preparation_sha256,
        "neural_session_receipt_sha256": receipt.neural_session_receipt_sha256,
        "neural_durable_evidence_profile": receipt.neural_durable_evidence_profile,
        "initial_snapshot_sha256": receipt.initial_snapshot_sha256,
        "last_verified_simulation_time_tics": receipt.last_verified_simulation_time_tics,
        "runtime_progress_disposition": receipt.runtime_progress_disposition,
        "steps": receipt.steps,
        "neural_executions": receipt.neural_executions,
        "runtime_finish_sha256": receipt.runtime_finish_sha256,
        "runtime_lifecycle": receipt.runtime_lifecycle,
        "cleanup": receipt.cleanup,
        "status": receipt.status,
        "primary_reason_code": receipt.primary_reason_code,
        "terminal_reason_code": receipt.terminal_reason_code,
        "cleanup_complete": receipt.cleanup_complete,
        "transcript_sha256": receipt.transcript_sha256,
        "simulator_only": receipt.simulator_only,
        "physical_actuation": receipt.physical_actuation,
        "ncp_qualified": receipt.ncp_qualified,
        "scientific_authority": receipt.scientific_authority,
        "is_paper_local_evidence": receipt.is_paper_local_evidence,
        "calibrated_posterior": receipt.calibrated_posterior,
    });
    sha256_value(&value).map_err(|_| ObserverError::Canonicalization)
}

fn validate_prepare(request: &PrepareRequest) -> Result<(), ObserverError> {
    if request.schema_version != PREPARE_REQUEST_SCHEMA_ID
        || !valid_entity_id(&request.study_run_id)
        || !valid_digest_roster(&[
            &request.study_definition_sha256,
            &request.closed_loop_definition_sha256,
            &request.runtime_binding_sha256,
            &request.runtime_adapter_configuration_sha256,
            &request.neural_provider_identity_sha256,
        ])
        || request.runtime_binding_sha256 == request.neural_provider_identity_sha256
        || !(1..=MAX_CHANNELS).contains(&request.channel_ids.len())
        || request.channel_ids.len() != request.subject_ids.len()
        || !valid_sorted_entity_roster(&request.channel_ids)
        || !valid_unique_entity_roster(&request.subject_ids)
        || !(1..=request.max_steps).contains(&request.planned_step_count)
        || !(1..=MAX_STEPS).contains(&request.max_steps)
    {
        return Err(ObserverError::InvalidInput);
    }
    Ok(())
}

fn validate_observe(active: &ActiveRun, request: &ObserveRequest) -> Result<(), ObserverError> {
    if request.schema_version != OBSERVE_REQUEST_SCHEMA_ID
        || !valid_digest_roster(&[
            &request.input_snapshot_sha256,
            &request.neural_request_sha256,
            &request.neural_result_sha256,
            &request.provider_execution_sha256,
            &request.admitted_action_sha256,
            &request.runtime_request_sha256,
            &request.output_snapshot_sha256,
            &request.source_receipt_sha256,
        ])
        || request.fault_codes.len() != active.channel_ids.len()
        || !valid_provider_execution_scope(&request.provider_execution_scope)
        || !request
            .fault_codes
            .iter()
            .all(|code| valid_fault_code(code))
    {
        return Err(ObserverError::InvalidInput);
    }
    if request.study_run_id != active.study_run_id {
        return Err(ObserverError::RunIdentityMismatch);
    }
    let expected_step = active.steps.len() as u64 + 1;
    if request.step_index != expected_step {
        return Err(ObserverError::StepOutOfOrder);
    }
    if request.step_index > active.max_steps || request.step_index > active.planned_step_count {
        return Err(ObserverError::StepBudgetExhausted);
    }
    if request.step_id != closed_loop_step_id(&request.study_run_id, request.step_index)? {
        return Err(ObserverError::SourceStepIdMismatch);
    }
    if active
        .steps
        .last()
        .is_some_and(|prior| request.input_snapshot_sha256 != prior.output_snapshot_sha256)
    {
        return Err(ObserverError::SourceSnapshotMismatch);
    }
    Ok(())
}

fn validate_finish_identity(
    active: &ActiveRun,
    request: &FinishRequest,
    timebase: &ClosedLoopTimebase,
) -> Result<(), ObserverError> {
    if request.schema_version != FINISH_REQUEST_SCHEMA_ID
        || request.digest_canonicalization != CLOSED_LOOP_DIGEST_CANONICALIZATION
        || !valid_timebase(timebase)
        || !valid_digest_roster(&[
            &request.source_transcript_sha256,
            &request.source_run_receipt_sha256,
        ])
        || !request
            .neural_preparation_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        || !request
            .neural_session_receipt_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        || !valid_neural_durable_evidence_profile(&request.neural_durable_evidence_profile)
        || !request
            .initial_snapshot_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        || !request
            .runtime_finish_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        || request
            .last_verified_simulation_time_tics
            .is_some_and(|value| value > MAX_SIMULATION_TIME_TICS)
        || !valid_runtime_progress_disposition(&request.runtime_progress_disposition)
        || request.step_count != active.steps.len() as u64
        || request.planned_step_count != active.planned_step_count
        || !(1..=active.max_steps).contains(&request.planned_step_count)
        || request.step_count > active.max_steps
        || !valid_deadline_enforcement(&request.runtime_deadline_enforcement)
        || !valid_deadline_enforcement(&request.neural_deadline_enforcement)
        || !valid_source_status(&request.source_status)
        || !valid_reason_code(&request.primary_reason_code)
        || !valid_reason_code(&request.terminal_reason_code)
    {
        return Err(ObserverError::InvalidInput);
    }
    if request.study_run_id != active.study_run_id {
        return Err(ObserverError::RunIdentityMismatch);
    }
    if !request.simulator_only
        || request.physical_actuation
        || request.ncp_qualified
        || request.scientific_authority
        || request.is_paper_local_evidence
        || request.calibrated_posterior
    {
        return Err(ObserverError::SourceAuthorityMismatch);
    }
    Ok(())
}

fn decode_timebase(values: &[RuntimeLifecycleScalar]) -> Result<ClosedLoopTimebase, ObserverError> {
    let [RuntimeLifecycleScalar::Text(schema_version), RuntimeLifecycleScalar::Text(tic_unit), RuntimeLifecycleScalar::Unsigned(runtime_step_duration_tics), RuntimeLifecycleScalar::Unsigned(neural_step_duration_tics), RuntimeLifecycleScalar::Text(clock_relation), RuntimeLifecycleScalar::Text(coupling), RuntimeLifecycleScalar::Text(causality_policy), RuntimeLifecycleScalar::Text(dispatch_order), RuntimeLifecycleScalar::Text(observation_sample_phase), RuntimeLifecycleScalar::Text(action_application)] =
        values
    else {
        return Err(ObserverError::InvalidInput);
    };
    let timebase = ClosedLoopTimebase {
        schema_version: schema_version.clone(),
        tic_unit: tic_unit.clone(),
        runtime_step_duration_tics: *runtime_step_duration_tics,
        neural_step_duration_tics: *neural_step_duration_tics,
        clock_relation: clock_relation.clone(),
        coupling: coupling.clone(),
        causality_policy: causality_policy.clone(),
        dispatch_order: dispatch_order.clone(),
        observation_sample_phase: observation_sample_phase.clone(),
        action_application: action_application.clone(),
    };
    if !valid_timebase(&timebase) {
        return Err(ObserverError::InvalidInput);
    }
    Ok(timebase)
}

fn neural_execution_receipts(
    active: &ActiveRun,
    request: &FinishRequest,
) -> Result<Vec<NeuralExecutionReceiptBinding>, ObserverError> {
    let mut receipts = Vec::with_capacity(active.steps.len().saturating_add(1));
    for step in &active.steps {
        let mut receipt = NeuralExecutionReceiptBinding {
            schema_version: CLOSED_LOOP_EXECUTION_SCHEMA.to_owned(),
            step_index: step.step_index,
            step_id: step.step_id.clone(),
            neural_request_sha256: step.neural_request_sha256.clone(),
            neural_result_sha256: step.neural_result_sha256.clone(),
            provider_execution_scope: step.provider_execution_scope.clone(),
            provider_execution_sha256: step.provider_execution_sha256.clone(),
            binding_sha256: String::new(),
        };
        receipt.binding_sha256 = neural_execution_binding_sha256(&receipt)?;
        receipts.push(receipt);
    }
    if request.neural_tail_values.len() != 6 {
        return Err(ObserverError::InvalidInput);
    }
    if !request
        .neural_tail_values
        .iter()
        .all(|value| matches!(value, RuntimeLifecycleScalar::Null))
    {
        match request.neural_tail_values.as_slice() {
            [RuntimeLifecycleScalar::Unsigned(step_index), RuntimeLifecycleScalar::Text(step_id), RuntimeLifecycleScalar::Text(neural_request_sha256), RuntimeLifecycleScalar::Text(neural_result_sha256), RuntimeLifecycleScalar::Text(provider_execution_scope), RuntimeLifecycleScalar::Text(provider_execution_sha256)] =>
            {
                let expected_step = active.steps.len() as u64 + 1;
                if *step_index != expected_step
                    || *step_index > MAX_STEPS
                    || *step_id != closed_loop_step_id(&active.study_run_id, *step_index)?
                    || !valid_digest_roster(&[
                        neural_request_sha256,
                        neural_result_sha256,
                        provider_execution_sha256,
                    ])
                    || !valid_provider_execution_scope(provider_execution_scope)
                {
                    return Err(ObserverError::SourceTerminalMismatch);
                }
                let mut receipt = NeuralExecutionReceiptBinding {
                    schema_version: CLOSED_LOOP_EXECUTION_SCHEMA.to_owned(),
                    step_index: *step_index,
                    step_id: step_id.clone(),
                    neural_request_sha256: neural_request_sha256.clone(),
                    neural_result_sha256: neural_result_sha256.clone(),
                    provider_execution_scope: provider_execution_scope.clone(),
                    provider_execution_sha256: provider_execution_sha256.clone(),
                    binding_sha256: String::new(),
                };
                receipt.binding_sha256 = neural_execution_binding_sha256(&receipt)?;
                receipts.push(receipt);
            }
            _ => return Err(ObserverError::InvalidInput),
        }
    }
    Ok(receipts)
}

fn cleanup_receipts(
    request: &FinishRequest,
    source_runtime_lifecycle: Option<RuntimeLifecycleReceiptBinding>,
) -> Result<Vec<CleanupReceipt>, ObserverError> {
    let runtime = decode_cleanup(
        &request.runtime_cleanup_values,
        "runtime",
        source_runtime_lifecycle,
    )?;
    let neural = decode_cleanup(&request.neural_cleanup_values, "neural", None)?;
    Ok(vec![runtime, neural])
}

fn decode_cleanup(
    values: &[RuntimeLifecycleScalar],
    expected_component: &str,
    runtime_lifecycle: Option<RuntimeLifecycleReceiptBinding>,
) -> Result<CleanupReceipt, ObserverError> {
    let [RuntimeLifecycleScalar::Text(schema_version), RuntimeLifecycleScalar::Text(component), RuntimeLifecycleScalar::Text(owner_identity_sha256), RuntimeLifecycleScalar::Text(mode), RuntimeLifecycleScalar::Boolean(attempted), RuntimeLifecycleScalar::Boolean(confirmed), RuntimeLifecycleScalar::Boolean(containment_empty), RuntimeLifecycleScalar::Text(reason_code), lifecycle_binding, provider_terminal, provider_lifecycle, RuntimeLifecycleScalar::Text(receipt_sha256)] =
        values
    else {
        return Err(ObserverError::InvalidInput);
    };
    let optional_text = |value: &RuntimeLifecycleScalar| match value {
        RuntimeLifecycleScalar::Text(value) => Some(Some(value.clone())),
        RuntimeLifecycleScalar::Null => Some(None),
        RuntimeLifecycleScalar::Unsigned(_) | RuntimeLifecycleScalar::Boolean(_) => None,
    };
    let lifecycle_binding = optional_text(lifecycle_binding).ok_or(ObserverError::InvalidInput)?;
    let expected_lifecycle_binding = runtime_lifecycle
        .as_ref()
        .map(|lifecycle| lifecycle.binding_sha256.clone());
    if lifecycle_binding != expected_lifecycle_binding || component != expected_component {
        return Err(ObserverError::SourceTerminalMismatch);
    }
    let receipt = CleanupReceipt {
        schema_version: schema_version.clone(),
        component: component.clone(),
        owner_identity_sha256: owner_identity_sha256.clone(),
        mode: mode.clone(),
        attempted: *attempted,
        confirmed: *confirmed,
        containment_empty: *containment_empty,
        reason_code: reason_code.clone(),
        runtime_lifecycle,
        provider_terminal_receipt_sha256: optional_text(provider_terminal)
            .ok_or(ObserverError::InvalidInput)?,
        provider_lifecycle_receipt_sha256: optional_text(provider_lifecycle)
            .ok_or(ObserverError::InvalidInput)?,
        receipt_sha256: receipt_sha256.clone(),
    };
    if !valid_cleanup(&receipt) {
        return Err(ObserverError::InvalidInput);
    }
    if source_cleanup_receipt_sha256(&receipt)? != receipt.receipt_sha256 {
        return Err(ObserverError::SourceCleanupDigestMismatch);
    }
    Ok(receipt)
}

fn validate_source_run_receipt(receipt: &SourceRunReceipt) -> Result<(), ObserverError> {
    if receipt.schema_version != CLOSED_LOOP_RUN_SCHEMA
        || receipt.digest_canonicalization != CLOSED_LOOP_DIGEST_CANONICALIZATION
        || !valid_entity_id(&receipt.study_run_id)
        || !valid_digest_roster(&[
            &receipt.study_definition_sha256,
            &receipt.closed_loop_definition_sha256,
            &receipt.runtime_binding_sha256,
            &receipt.runtime_adapter_configuration_sha256,
            &receipt.neural_provider_identity_sha256,
            &receipt.transcript_sha256,
            &receipt.receipt_sha256,
        ])
        || receipt.runtime_binding_sha256 == receipt.neural_provider_identity_sha256
        || !receipt
            .neural_preparation_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        || !receipt
            .neural_session_receipt_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        || !valid_neural_durable_evidence_profile(&receipt.neural_durable_evidence_profile)
        || !receipt
            .initial_snapshot_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        || !receipt
            .runtime_finish_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        || !valid_deadline_enforcement(&receipt.runtime_deadline_enforcement)
        || !valid_deadline_enforcement(&receipt.neural_deadline_enforcement)
        || !valid_source_status(&receipt.status)
        || !valid_reason_code(&receipt.primary_reason_code)
        || !valid_reason_code(&receipt.terminal_reason_code)
        || !valid_timebase(&receipt.timebase)
    {
        return Err(ObserverError::SourceTerminalMismatch);
    }
    let [runtime_cleanup, neural_cleanup] = receipt.cleanup.as_slice() else {
        return Err(ObserverError::SourceTerminalMismatch);
    };
    if !valid_cleanup(runtime_cleanup) || !valid_cleanup(neural_cleanup) {
        return Err(ObserverError::SourceTerminalMismatch);
    }
    if source_cleanup_receipt_sha256(runtime_cleanup)? != runtime_cleanup.receipt_sha256
        || source_cleanup_receipt_sha256(neural_cleanup)? != neural_cleanup.receipt_sha256
    {
        return Err(ObserverError::SourceCleanupDigestMismatch);
    }
    for step in &receipt.steps {
        if step.schema_version != CLOSED_LOOP_STEP_SCHEMA
            || step.study_run_id != receipt.study_run_id
            || step.step_id != closed_loop_step_id(&step.study_run_id, step.step_index)?
            || !valid_provider_execution_scope(&step.provider_execution_scope)
            || !valid_digest_roster(&[
                &step.input_snapshot_sha256,
                &step.neural_request_sha256,
                &step.neural_result_sha256,
                &step.provider_execution_sha256,
                &step.admitted_action_sha256,
                &step.runtime_request_sha256,
                &step.output_snapshot_sha256,
                &step.receipt_sha256,
            ])
            || !(1..=MAX_CHANNELS).contains(&step.fault_codes.len())
            || !step.fault_codes.iter().all(|code| valid_fault_code(code))
        {
            return Err(ObserverError::SourceTerminalMismatch);
        }
        if source_step_receipt_sha256(step)? != step.receipt_sha256 {
            return Err(ObserverError::SourceStepDigestMismatch);
        }
    }
    let step_indexes_are_contiguous = receipt
        .steps
        .iter()
        .enumerate()
        .all(|(index, step)| step.step_index == index as u64 + 1);
    let step_snapshot_chain_is_complete = receipt.steps.first().is_none_or(|first| {
        receipt.initial_snapshot_sha256.as_deref() == Some(first.input_snapshot_sha256.as_str())
            && receipt
                .steps
                .windows(2)
                .all(|pair| pair[1].input_snapshot_sha256 == pair[0].output_snapshot_sha256)
    });
    let step_count_is_within_plan = receipt.steps.len() as u64 <= receipt.planned_step_count;
    for execution in &receipt.neural_executions {
        if execution.schema_version != CLOSED_LOOP_EXECUTION_SCHEMA
            || execution.step_id
                != closed_loop_step_id(&receipt.study_run_id, execution.step_index)?
            || !valid_provider_execution_scope(&execution.provider_execution_scope)
            || !valid_digest_roster(&[
                &execution.neural_request_sha256,
                &execution.neural_result_sha256,
                &execution.provider_execution_sha256,
                &execution.binding_sha256,
            ])
            || neural_execution_binding_sha256(execution)? != execution.binding_sha256
        {
            return Err(ObserverError::SourceTerminalMismatch);
        }
    }
    let execution_indexes_are_contiguous = receipt
        .neural_executions
        .iter()
        .enumerate()
        .all(|(index, execution)| execution.step_index == index as u64 + 1);
    let execution_count_matches_runtime_progress = receipt.steps.len()
        <= receipt.neural_executions.len()
        && receipt.neural_executions.len() <= receipt.steps.len().saturating_add(1);
    let executions_match_steps =
        receipt
            .steps
            .iter()
            .zip(&receipt.neural_executions)
            .all(|(step, execution)| {
                step.step_index == execution.step_index
                    && step.step_id == execution.step_id
                    && step.neural_request_sha256 == execution.neural_request_sha256
                    && step.neural_result_sha256 == execution.neural_result_sha256
                    && step.provider_execution_scope == execution.provider_execution_scope
                    && step.provider_execution_sha256 == execution.provider_execution_sha256
            });
    let expected_verified_time = if receipt.initial_snapshot_sha256.is_some() {
        (receipt.steps.len() as u64).checked_mul(receipt.timebase.runtime_step_duration_tics)
    } else {
        None
    };
    let progress_matches = if receipt.runtime_finish_sha256.is_some() {
        receipt.runtime_progress_disposition == "finished-and-host-verified"
    } else if matches!(
        receipt.runtime_progress_disposition.as_str(),
        "unknown-after-dispatch" | "unknown-after-operation-attempt"
    ) {
        true
    } else if receipt.initial_snapshot_sha256.is_none() {
        receipt.runtime_progress_disposition == "not-started"
    } else {
        receipt.runtime_progress_disposition == "last-host-verified"
    };
    let preparation_is_present = receipt.neural_preparation_sha256.is_some();
    let session_is_present = receipt.neural_session_receipt_sha256.is_some();
    let expected_runtime_mode = if receipt.runtime_finish_sha256.is_some() {
        "finish"
    } else {
        "generation-kill"
    };
    let cleanup_complete = receipt
        .cleanup
        .iter()
        .all(|item| item.confirmed && item.containment_empty);
    let finish_lineage_is_complete = receipt.initial_snapshot_sha256.is_some()
        && preparation_is_present
        && session_is_present
        && receipt.steps.len() as u64 == receipt.planned_step_count;
    let completed_lineage_is_complete = finish_lineage_is_complete
        && receipt.runtime_finish_sha256.is_some()
        && receipt.cleanup_complete;
    let step_lineage_is_complete = receipt.steps.is_empty()
        || (receipt.initial_snapshot_sha256.is_some()
            && preparation_is_present
            && session_is_present);
    let expected_status = if !receipt.cleanup_complete {
        "failed"
    } else if receipt.runtime_finish_sha256.is_some() {
        "completed"
    } else if receipt.primary_reason_code == "loop.cancelled" {
        "cancelled"
    } else if receipt.primary_reason_code == "runtime.overload" {
        "overloaded"
    } else {
        "failed"
    };
    let expected_terminal_reason = if receipt.cleanup_complete {
        receipt.primary_reason_code.as_str()
    } else {
        "cleanup.unconfirmed"
    };
    if !(1..=MAX_STEPS).contains(&receipt.planned_step_count)
        || !step_indexes_are_contiguous
        || !step_snapshot_chain_is_complete
        || !step_count_is_within_plan
        || !execution_indexes_are_contiguous
        || !execution_count_matches_runtime_progress
        || !executions_match_steps
        || receipt.last_verified_simulation_time_tics != expected_verified_time
        || !progress_matches
        || preparation_is_present != session_is_present
        || runtime_cleanup.component != "runtime"
        || neural_cleanup.component != "neural"
        || runtime_cleanup.owner_identity_sha256 != receipt.runtime_binding_sha256
        || neural_cleanup.owner_identity_sha256 != receipt.neural_provider_identity_sha256
        || runtime_cleanup.owner_identity_sha256 == neural_cleanup.owner_identity_sha256
        || runtime_cleanup.mode != expected_runtime_mode
        || neural_cleanup.mode != "close"
        || runtime_cleanup.runtime_lifecycle != receipt.runtime_lifecycle
        || neural_cleanup.runtime_lifecycle.is_some()
        || receipt.cleanup_complete != cleanup_complete
        || (receipt.runtime_finish_sha256.is_some()
            != (receipt.primary_reason_code == "loop.completed"))
        || receipt.status != expected_status
        || (preparation_is_present && receipt.initial_snapshot_sha256.is_none())
        || (receipt.runtime_finish_sha256.is_some() && !finish_lineage_is_complete)
        || (receipt.status == "completed" && !completed_lineage_is_complete)
        || !step_lineage_is_complete
        || receipt.terminal_reason_code != expected_terminal_reason
    {
        return Err(ObserverError::SourceTerminalMismatch);
    }
    if !receipt.simulator_only
        || receipt.physical_actuation
        || receipt.ncp_qualified
        || receipt.scientific_authority
        || receipt.is_paper_local_evidence
        || receipt.calibrated_posterior
    {
        return Err(ObserverError::SourceAuthorityMismatch);
    }
    let transcript = json!({
        "domain": "engram-extension-closed-loop-transcript-v5",
        "digest_canonicalization": receipt.digest_canonicalization,
        "planned_step_count": receipt.planned_step_count,
        "timebase": receipt.timebase,
        "neural_preparation_sha256": receipt.neural_preparation_sha256,
        "neural_session_receipt_sha256": receipt.neural_session_receipt_sha256,
        "neural_durable_evidence_profile": receipt.neural_durable_evidence_profile,
        "initial_snapshot_sha256": receipt.initial_snapshot_sha256,
        "last_verified_simulation_time_tics": receipt.last_verified_simulation_time_tics,
        "runtime_progress_disposition": receipt.runtime_progress_disposition,
        "step_receipts": receipt.steps.iter().map(|step| step.receipt_sha256.clone()).collect::<Vec<_>>(),
        "neural_execution_bindings": receipt.neural_executions.iter().map(|item| item.binding_sha256.clone()).collect::<Vec<_>>(),
        "runtime_finish_sha256": receipt.runtime_finish_sha256,
        "runtime_lifecycle_binding_sha256": receipt.runtime_lifecycle.as_ref().map(|item| item.binding_sha256.clone()),
        "cleanup_receipts": receipt.cleanup.iter().map(|item| item.receipt_sha256.clone()).collect::<Vec<_>>(),
        "status": receipt.status,
        "primary_reason_code": receipt.primary_reason_code,
        "terminal_reason_code": receipt.terminal_reason_code,
    });
    if sha256_value(&transcript).map_err(|_| ObserverError::Canonicalization)?
        != receipt.transcript_sha256
    {
        return Err(ObserverError::SourceTranscriptDigestMismatch);
    }
    if source_run_receipt_sha256(receipt)? != receipt.receipt_sha256 {
        return Err(ObserverError::SourceRunDigestMismatch);
    }
    Ok(())
}

fn valid_cleanup(receipt: &CleanupReceipt) -> bool {
    let lifecycle_matches = receipt.runtime_lifecycle.as_ref().is_none_or(|lifecycle| {
        valid_runtime_lifecycle(lifecycle)
            && receipt.confirmed == runtime_lifecycle_confirms_cleanup(lifecycle)
            && match receipt.mode.as_str() {
                "finish" => lifecycle.termination_disposition == "clean-exit",
                "generation-kill" => matches!(
                    lifecycle.termination_disposition.as_str(),
                    "terminated" | "killed"
                ),
                _ => false,
            }
    });
    receipt.schema_version == CLOSED_LOOP_CLEANUP_SCHEMA
        && matches!(receipt.component.as_str(), "runtime" | "neural")
        && valid_sha256(&receipt.owner_identity_sha256)
        && matches!(
            receipt.mode.as_str(),
            "finish" | "generation-kill" | "close"
        )
        && receipt.attempted
        && valid_reason_code(&receipt.reason_code)
        && receipt
            .provider_terminal_receipt_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        && receipt
            .provider_lifecycle_receipt_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        && valid_sha256(&receipt.receipt_sha256)
        && (!receipt.confirmed || receipt.containment_empty)
        && (receipt.component != "neural" || receipt.runtime_lifecycle.is_none())
        && lifecycle_matches
}

#[cfg(test)]
fn encode_runtime_lifecycle(
    receipt: Option<&RuntimeLifecycleReceiptBinding>,
) -> Vec<RuntimeLifecycleScalar> {
    let Some(receipt) = receipt else {
        return vec![RuntimeLifecycleScalar::Null; 22];
    };
    vec![
        RuntimeLifecycleScalar::Text(receipt.schema_version.clone()),
        RuntimeLifecycleScalar::Text(receipt.profile.clone()),
        RuntimeLifecycleScalar::Text(receipt.generation_id.clone()),
        RuntimeLifecycleScalar::Text(receipt.launch_source.clone()),
        receipt
            .store_id
            .clone()
            .map_or(RuntimeLifecycleScalar::Null, RuntimeLifecycleScalar::Text),
        RuntimeLifecycleScalar::Text(receipt.package_generation_id.clone()),
        receipt
            .generation_directory_identity_sha256
            .clone()
            .map_or(RuntimeLifecycleScalar::Null, RuntimeLifecycleScalar::Text),
        RuntimeLifecycleScalar::Boolean(receipt.package_generation_lease_retained_at_launch),
        RuntimeLifecycleScalar::Boolean(receipt.package_generation_lease_released),
        RuntimeLifecycleScalar::Text(receipt.handshake_receipt_sha256.clone()),
        RuntimeLifecycleScalar::Text(receipt.termination_receipt_sha256.clone()),
        RuntimeLifecycleScalar::Text(receipt.termination_disposition.clone()),
        RuntimeLifecycleScalar::Boolean(receipt.child_reaped),
        RuntimeLifecycleScalar::Boolean(receipt.containment_empty),
        RuntimeLifecycleScalar::Boolean(receipt.diagnostic_stream_complete),
        RuntimeLifecycleScalar::Boolean(receipt.private_work_directory_removed),
        RuntimeLifecycleScalar::Boolean(receipt.publisher_authenticated),
        RuntimeLifecycleScalar::Boolean(receipt.durable_process_launch_authority),
        RuntimeLifecycleScalar::Boolean(receipt.ncp_authority),
        RuntimeLifecycleScalar::Boolean(receipt.physical_authority),
        RuntimeLifecycleScalar::Boolean(receipt.scientific_authority),
        RuntimeLifecycleScalar::Text(receipt.binding_sha256.clone()),
    ]
}

fn decode_runtime_lifecycle(
    values: &[RuntimeLifecycleScalar],
) -> Result<Option<RuntimeLifecycleReceiptBinding>, ObserverError> {
    if values.len() != 22 {
        return Err(ObserverError::InvalidInput);
    }
    if values
        .iter()
        .all(|value| matches!(value, RuntimeLifecycleScalar::Null))
    {
        return Ok(None);
    }
    let text = |index: usize| match &values[index] {
        RuntimeLifecycleScalar::Text(value) => Some(value.clone()),
        _ => None,
    };
    let optional_text = |index: usize| match &values[index] {
        RuntimeLifecycleScalar::Text(value) => Some(Some(value.clone())),
        RuntimeLifecycleScalar::Null => Some(None),
        RuntimeLifecycleScalar::Unsigned(_) | RuntimeLifecycleScalar::Boolean(_) => None,
    };
    let boolean = |index: usize| match values[index] {
        RuntimeLifecycleScalar::Boolean(value) => Some(value),
        RuntimeLifecycleScalar::Text(_)
        | RuntimeLifecycleScalar::Unsigned(_)
        | RuntimeLifecycleScalar::Null => None,
    };
    let receipt = RuntimeLifecycleReceiptBinding {
        schema_version: text(0).ok_or(ObserverError::InvalidInput)?,
        profile: text(1).ok_or(ObserverError::InvalidInput)?,
        generation_id: text(2).ok_or(ObserverError::InvalidInput)?,
        launch_source: text(3).ok_or(ObserverError::InvalidInput)?,
        store_id: optional_text(4).ok_or(ObserverError::InvalidInput)?,
        package_generation_id: text(5).ok_or(ObserverError::InvalidInput)?,
        generation_directory_identity_sha256: optional_text(6)
            .ok_or(ObserverError::InvalidInput)?,
        package_generation_lease_retained_at_launch: boolean(7)
            .ok_or(ObserverError::InvalidInput)?,
        package_generation_lease_released: boolean(8).ok_or(ObserverError::InvalidInput)?,
        handshake_receipt_sha256: text(9).ok_or(ObserverError::InvalidInput)?,
        termination_receipt_sha256: text(10).ok_or(ObserverError::InvalidInput)?,
        termination_disposition: text(11).ok_or(ObserverError::InvalidInput)?,
        child_reaped: boolean(12).ok_or(ObserverError::InvalidInput)?,
        containment_empty: boolean(13).ok_or(ObserverError::InvalidInput)?,
        diagnostic_stream_complete: boolean(14).ok_or(ObserverError::InvalidInput)?,
        private_work_directory_removed: boolean(15).ok_or(ObserverError::InvalidInput)?,
        publisher_authenticated: boolean(16).ok_or(ObserverError::InvalidInput)?,
        durable_process_launch_authority: boolean(17).ok_or(ObserverError::InvalidInput)?,
        ncp_authority: boolean(18).ok_or(ObserverError::InvalidInput)?,
        physical_authority: boolean(19).ok_or(ObserverError::InvalidInput)?,
        scientific_authority: boolean(20).ok_or(ObserverError::InvalidInput)?,
        binding_sha256: text(21).ok_or(ObserverError::InvalidInput)?,
    };
    if !valid_runtime_lifecycle(&receipt) {
        return Err(ObserverError::InvalidInput);
    }
    Ok(Some(receipt))
}

fn valid_runtime_lifecycle(receipt: &RuntimeLifecycleReceiptBinding) -> bool {
    let store_authority_is_closed = if receipt.launch_source == "package-store-lease" {
        receipt
            .store_id
            .as_deref()
            .is_some_and(|value| valid_prefixed_sha256(value, "extstore_"))
            && receipt
                .generation_directory_identity_sha256
                .as_deref()
                .is_some_and(valid_sha256)
            && receipt.package_generation_lease_retained_at_launch
    } else if receipt.launch_source == "packed-bundle-path" {
        receipt.store_id.is_none()
            && receipt.generation_directory_identity_sha256.is_none()
            && !receipt.package_generation_lease_retained_at_launch
            && !receipt.package_generation_lease_released
    } else {
        false
    };
    receipt.schema_version == "engram.closed-loop-runtime-lifecycle-binding.v1"
        && receipt.profile == "engram.reviewed-native-development.v1"
        && valid_prefixed_sha256(&receipt.generation_id, "gen_")
        && valid_prefixed_sha256(&receipt.package_generation_id, "pkggen_")
        && valid_digest_roster(&[
            &receipt.handshake_receipt_sha256,
            &receipt.termination_receipt_sha256,
            &receipt.binding_sha256,
        ])
        && matches!(
            receipt.termination_disposition.as_str(),
            "clean-exit" | "terminated" | "killed" | "unconfirmed"
        )
        && (!receipt.containment_empty || receipt.child_reaped)
        && !receipt.publisher_authenticated
        && !receipt.durable_process_launch_authority
        && !receipt.ncp_authority
        && !receipt.physical_authority
        && !receipt.scientific_authority
        && store_authority_is_closed
        && runtime_lifecycle_binding_sha256(receipt)
            .is_ok_and(|digest| digest == receipt.binding_sha256)
}

fn runtime_lifecycle_confirms_cleanup(receipt: &RuntimeLifecycleReceiptBinding) -> bool {
    receipt.termination_disposition != "unconfirmed"
        && receipt.child_reaped
        && receipt.containment_empty
        && receipt.diagnostic_stream_complete
        && receipt.private_work_directory_removed
        && (receipt.launch_source != "package-store-lease"
            || receipt.package_generation_lease_released)
}

fn runtime_lifecycle_binding_sha256(
    receipt: &RuntimeLifecycleReceiptBinding,
) -> Result<String, ObserverError> {
    let value = json!({
        "schema_version": receipt.schema_version,
        "profile": receipt.profile,
        "generation_id": receipt.generation_id,
        "launch_source": receipt.launch_source,
        "store_id": receipt.store_id,
        "package_generation_id": receipt.package_generation_id,
        "generation_directory_identity_sha256": receipt.generation_directory_identity_sha256,
        "package_generation_lease_retained_at_launch": receipt.package_generation_lease_retained_at_launch,
        "package_generation_lease_released": receipt.package_generation_lease_released,
        "handshake_receipt_sha256": receipt.handshake_receipt_sha256,
        "termination_receipt_sha256": receipt.termination_receipt_sha256,
        "termination_disposition": receipt.termination_disposition,
        "child_reaped": receipt.child_reaped,
        "containment_empty": receipt.containment_empty,
        "diagnostic_stream_complete": receipt.diagnostic_stream_complete,
        "private_work_directory_removed": receipt.private_work_directory_removed,
        "publisher_authenticated": receipt.publisher_authenticated,
        "durable_process_launch_authority": receipt.durable_process_launch_authority,
        "ncp_authority": receipt.ncp_authority,
        "physical_authority": receipt.physical_authority,
        "scientific_authority": receipt.scientific_authority,
    });
    sha256_value(&value).map_err(|_| ObserverError::Canonicalization)
}

fn valid_deadline_enforcement(value: &str) -> bool {
    matches!(
        value,
        "host-generation-kill" | "cooperative-observed" | "deterministic-test"
    )
}

fn valid_provider_execution_scope(value: &str) -> bool {
    matches!(value, "decoded-proposal-only" | "nest-exact-step-readback")
}

fn valid_neural_durable_evidence_profile(value: &str) -> bool {
    matches!(value, "none" | "engram.nest-closed-loop-evidence-bundle.v2")
}

fn valid_runtime_progress_disposition(value: &str) -> bool {
    matches!(
        value,
        "not-started"
            | "last-host-verified"
            | "finished-and-host-verified"
            | "unknown-after-dispatch"
            | "unknown-after-operation-attempt"
    )
}

fn valid_timebase(value: &ClosedLoopTimebase) -> bool {
    value.schema_version == "engram.extension-closed-loop-timebase.v1"
        && value.tic_unit == "microsecond"
        && (1..=MAX_STEP_DURATION_TICS).contains(&value.runtime_step_duration_tics)
        && (1..=MAX_STEP_DURATION_TICS).contains(&value.neural_step_duration_tics)
        && value.clock_relation == "independent-controller-and-runtime-logical-clocks"
        && value.coupling == "one-controller-epoch-per-runtime-interval"
        && value.causality_policy == "sample-runtime-run-controller-apply-zoh-v1"
        && value.dispatch_order == "observe-controller-action-runtime"
        && value.observation_sample_phase == "runtime-interval-start"
        && value.action_application == "after-controller-completion-zoh-over-runtime-interval"
}

fn valid_source_status(value: &str) -> bool {
    matches!(value, "completed" | "cancelled" | "overloaded" | "failed")
}

fn valid_sorted_entity_roster(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
        && values.iter().all(|value| valid_entity_id(value))
}

fn valid_unique_entity_roster(values: &[String]) -> bool {
    values.iter().all(|value| valid_entity_id(value))
        && values
            .iter()
            .enumerate()
            .all(|(index, value)| !values[..index].contains(value))
}

fn valid_entity_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_reason_code(value: &str) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.len() <= MAX_REASON_BYTES
        && value
            .chars()
            .all(|character| character >= '\u{21}' && character != '\u{7f}')
}

fn valid_fault_code(value: &str) -> bool {
    valid_reason_code(value) && value.len() <= MAX_FAULT_CODE_BYTES
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_prefixed_sha256(value: &str, prefix: &str) -> bool {
    value.strip_prefix(prefix).is_some_and(valid_sha256)
}

fn valid_digest_roster(values: &[&str]) -> bool {
    values.iter().all(|value| valid_sha256(value))
}

fn request_sha256<T: Serialize>(request: &T) -> Result<String, ObserverError> {
    let value = to_value(request).map_err(|_| ObserverError::Canonicalization)?;
    sha256_value(&value).map_err(|_| ObserverError::Canonicalization)
}

fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ObserverError> {
    let value = to_value(value).map_err(|_| ObserverError::Canonicalization)?;
    canonical_json(&value).map_err(|_| ObserverError::Canonicalization)
}

fn absent_state_sha256() -> Result<String, ObserverError> {
    sha256_domain(ABSENT_STATE_DOMAIN, &[]).map_err(|_| ObserverError::Canonicalization)
}

fn empty_transcript_sha256() -> Result<String, ObserverError> {
    sha256_domain(EMPTY_TRANSCRIPT_DOMAIN, &[]).map_err(|_| ObserverError::Canonicalization)
}

fn advance_transcript(prior: &str, receipt: &str) -> Result<String, ObserverError> {
    sha256_domain(TRANSCRIPT_DOMAIN, &[prior.as_bytes(), receipt.as_bytes()])
        .map_err(|_| ObserverError::Canonicalization)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the digest binds the complete receipt identity"
)]
fn observer_receipt_sha256(
    schema_version: &str,
    outcome: ObserverOutcome,
    reason: &str,
    study_run_id: &str,
    step_index: u64,
    channel_count: u64,
    fault_count: u64,
    cumulative_fault_count: u64,
    request_sha256: &str,
    prior_state_sha256: &str,
    state_sha256: &str,
    source_receipt_sha256: Option<&str>,
    terminal: bool,
    state_cleared: bool,
    authority: &str,
    roster_authority: &str,
    source_roster_authenticated: bool,
    descriptive_only: bool,
    agent_bridge_command: bool,
    physical_actuation: bool,
    ncp_used: bool,
    pid_result: bool,
    source_durable_evidence_verified: bool,
    scientific_authority: bool,
    is_paper_local_evidence: bool,
    calibrated_posterior: bool,
) -> Result<String, ObserverError> {
    let value = json!({
        "schema_version": schema_version,
        "outcome": outcome_name(outcome),
        "reason": reason,
        "authority": authority,
        "roster_authority": roster_authority,
        "source_roster_authenticated": source_roster_authenticated,
        "study_run_id": study_run_id,
        "step_index": step_index,
        "channel_count": channel_count,
        "fault_count": fault_count,
        "cumulative_fault_count": cumulative_fault_count,
        "request_sha256": request_sha256,
        "prior_observer_state_sha256": prior_state_sha256,
        "observer_state_sha256": state_sha256,
        "source_receipt_sha256": source_receipt_sha256,
        "terminal": terminal,
        "state_cleared": state_cleared,
        "descriptive_only": descriptive_only,
        "agent_bridge_command": agent_bridge_command,
        "physical_actuation": physical_actuation,
        "ncp_used": ncp_used,
        "pid_result": pid_result,
        "source_durable_evidence_verified": source_durable_evidence_verified,
        "scientific_authority": scientific_authority,
        "is_paper_local_evidence": is_paper_local_evidence,
        "calibrated_posterior": calibrated_posterior,
    });
    let bytes = canonical_json(&value).map_err(|_| ObserverError::Canonicalization)?;
    sha256_domain(RECEIPT_DOMAIN, &[&bytes]).map_err(|_| ObserverError::Canonicalization)
}

/// Recompute the deterministic semantic digest for one observer response.
///
/// The digest excludes only its own field and the rolling transcript digest.
///
/// # Errors
///
/// Returns [`ObserverError::Canonicalization`] if canonical encoding fails.
pub fn observer_response_receipt_sha256(
    response: &ObserverResponse,
) -> Result<String, ObserverError> {
    observer_receipt_sha256(
        &response.schema_version,
        response.outcome,
        &response.reason,
        &response.study_run_id,
        response.step_index,
        response.channel_count,
        response.fault_count,
        response.cumulative_fault_count,
        &response.request_sha256,
        &response.prior_observer_state_sha256,
        &response.observer_state_sha256,
        response.source_receipt_sha256.as_deref(),
        response.terminal,
        response.state_cleared,
        &response.authority,
        &response.roster_authority,
        response.source_roster_authenticated,
        response.descriptive_only,
        response.agent_bridge_command,
        response.physical_actuation,
        response.ncp_used,
        response.pid_result,
        response.source_durable_evidence_verified,
        response.scientific_authority,
        response.is_paper_local_evidence,
        response.calibrated_posterior,
    )
}

fn outcome_name(outcome: ObserverOutcome) -> &'static str {
    match outcome {
        ObserverOutcome::Succeeded => "succeeded",
        ObserverOutcome::Rejected => "rejected",
        ObserverOutcome::Failed => "failed",
    }
}

struct ResponseParts<'a> {
    schema_version: &'a str,
    outcome: ObserverOutcome,
    reason: &'a str,
    study_run_id: &'a str,
    step_index: u64,
    channel_count: u64,
    fault_count: u64,
    cumulative_fault_count: u64,
    source_receipt_sha256: Option<&'a str>,
    prior_state_sha256: &'a str,
    state_sha256: &'a str,
    request_sha256: &'a str,
    receipt_sha256: &'a str,
    transcript_sha256: &'a str,
    terminal: bool,
    state_cleared: bool,
}

fn build_response(parts: ResponseParts<'_>) -> ObserverResponse {
    ObserverResponse {
        schema_version: parts.schema_version.to_owned(),
        outcome: parts.outcome,
        reason: parts.reason.to_owned(),
        authority: AUTHORITY.to_owned(),
        roster_authority: "host-declared-projection".to_owned(),
        source_roster_authenticated: false,
        study_run_id: parts.study_run_id.to_owned(),
        step_index: parts.step_index,
        channel_count: parts.channel_count,
        fault_count: parts.fault_count,
        cumulative_fault_count: parts.cumulative_fault_count,
        source_receipt_sha256: parts.source_receipt_sha256.map(str::to_owned),
        prior_observer_state_sha256: parts.prior_state_sha256.to_owned(),
        observer_state_sha256: parts.state_sha256.to_owned(),
        request_sha256: parts.request_sha256.to_owned(),
        observer_receipt_sha256: parts.receipt_sha256.to_owned(),
        observer_transcript_sha256: parts.transcript_sha256.to_owned(),
        terminal: parts.terminal,
        state_cleared: parts.state_cleared,
        descriptive_only: true,
        agent_bridge_command: false,
        physical_actuation: false,
        ncp_used: false,
        pid_result: false,
        source_durable_evidence_verified: false,
        scientific_authority: false,
        is_paper_local_evidence: false,
        calibrated_posterior: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn digest(marker: char) -> String {
        marker.to_string().repeat(64)
    }

    fn test_timebase() -> ClosedLoopTimebase {
        ClosedLoopTimebase {
            schema_version: "engram.extension-closed-loop-timebase.v1".to_owned(),
            tic_unit: "microsecond".to_owned(),
            runtime_step_duration_tics: 1_000,
            neural_step_duration_tics: 1_000,
            clock_relation: "independent-controller-and-runtime-logical-clocks".to_owned(),
            coupling: "one-controller-epoch-per-runtime-interval".to_owned(),
            causality_policy: "sample-runtime-run-controller-apply-zoh-v1".to_owned(),
            dispatch_order: "observe-controller-action-runtime".to_owned(),
            observation_sample_phase: "runtime-interval-start".to_owned(),
            action_application: "after-controller-completion-zoh-over-runtime-interval".to_owned(),
        }
    }

    fn source_receipt_value_is_float_free(value: &Value) -> bool {
        match value {
            Value::Array(items) => items.iter().all(source_receipt_value_is_float_free),
            Value::Object(items) => items.values().all(source_receipt_value_is_float_free),
            Value::Number(number) => number.as_i64().is_some() || number.as_u64().is_some(),
            Value::Null | Value::Bool(_) | Value::String(_) => true,
        }
    }

    fn prepare_request(channels: usize) -> PrepareRequest {
        PrepareRequest {
            schema_version: PREPARE_REQUEST_SCHEMA_ID.to_owned(),
            study_run_id: "study-run-01".to_owned(),
            study_definition_sha256: digest('1'),
            closed_loop_definition_sha256: digest('2'),
            runtime_binding_sha256: digest('3'),
            runtime_adapter_configuration_sha256: digest('e'),
            neural_provider_identity_sha256: digest('4'),
            channel_ids: (1..=channels)
                .map(|index| format!("channel-{index:02}"))
                .collect(),
            subject_ids: (1..=channels)
                .map(|index| format!("drone-{index:02}"))
                .collect(),
            planned_step_count: 1,
            max_steps: 8,
        }
    }

    fn observe_request(step_index: u64, channels: usize) -> ObserveRequest {
        let mut source = SourceStepReceipt {
            schema_version: CLOSED_LOOP_STEP_SCHEMA.to_owned(),
            study_run_id: "study-run-01".to_owned(),
            step_index,
            step_id: closed_loop_step_id("study-run-01", step_index).expect("step id"),
            input_snapshot_sha256: digest('5'),
            neural_request_sha256: digest('6'),
            neural_result_sha256: digest('7'),
            provider_execution_scope: "decoded-proposal-only".to_owned(),
            provider_execution_sha256: digest('f'),
            admitted_action_sha256: digest('8'),
            runtime_request_sha256: digest('9'),
            output_snapshot_sha256: digest('a'),
            fault_codes: vec!["none".to_owned(); channels],
            receipt_sha256: String::new(),
        };
        source.receipt_sha256 = source_step_receipt_sha256(&source).expect("source receipt");
        ObserveRequest {
            schema_version: OBSERVE_REQUEST_SCHEMA_ID.to_owned(),
            study_run_id: source.study_run_id,
            step_index: source.step_index,
            step_id: source.step_id,
            input_snapshot_sha256: source.input_snapshot_sha256,
            neural_request_sha256: source.neural_request_sha256,
            neural_result_sha256: source.neural_result_sha256,
            provider_execution_scope: source.provider_execution_scope,
            provider_execution_sha256: source.provider_execution_sha256,
            admitted_action_sha256: source.admitted_action_sha256,
            runtime_request_sha256: source.runtime_request_sha256,
            output_snapshot_sha256: source.output_snapshot_sha256,
            fault_codes: source.fault_codes,
            source_receipt_sha256: source.receipt_sha256,
        }
    }

    fn finish_request(runtime: &ObserverRuntime) -> FinishRequest {
        let Phase::Active(active) = &runtime.phase else {
            panic!("active fixture")
        };
        let mut cleanup = vec![
            CleanupReceipt {
                schema_version: CLOSED_LOOP_CLEANUP_SCHEMA.to_owned(),
                component: "runtime".to_owned(),
                owner_identity_sha256: digest('3'),
                mode: "finish".to_owned(),
                attempted: true,
                confirmed: true,
                containment_empty: true,
                reason_code: "runtime.finished".to_owned(),
                runtime_lifecycle: None,
                provider_terminal_receipt_sha256: None,
                provider_lifecycle_receipt_sha256: None,
                receipt_sha256: String::new(),
            },
            CleanupReceipt {
                schema_version: CLOSED_LOOP_CLEANUP_SCHEMA.to_owned(),
                component: "neural".to_owned(),
                owner_identity_sha256: digest('4'),
                mode: "close".to_owned(),
                attempted: true,
                confirmed: true,
                containment_empty: true,
                reason_code: "neural.closed".to_owned(),
                runtime_lifecycle: None,
                provider_terminal_receipt_sha256: None,
                provider_lifecycle_receipt_sha256: None,
                receipt_sha256: String::new(),
            },
        ];
        for item in &mut cleanup {
            item.receipt_sha256 = source_cleanup_receipt_sha256(item).expect("cleanup receipt");
        }
        let initial_snapshot_sha256 = active
            .steps
            .first()
            .expect("finished fixture has one step")
            .input_snapshot_sha256
            .clone();
        let timebase = test_timebase();
        let neural_executions = active
            .steps
            .iter()
            .map(|step| {
                let mut execution = NeuralExecutionReceiptBinding {
                    schema_version: CLOSED_LOOP_EXECUTION_SCHEMA.to_owned(),
                    step_index: step.step_index,
                    step_id: step.step_id.clone(),
                    neural_request_sha256: step.neural_request_sha256.clone(),
                    neural_result_sha256: step.neural_result_sha256.clone(),
                    provider_execution_scope: step.provider_execution_scope.clone(),
                    provider_execution_sha256: step.provider_execution_sha256.clone(),
                    binding_sha256: String::new(),
                };
                execution.binding_sha256 =
                    neural_execution_binding_sha256(&execution).expect("execution binding");
                execution
            })
            .collect::<Vec<_>>();
        let transcript_value = json!({
            "domain": "engram-extension-closed-loop-transcript-v5",
            "digest_canonicalization": CLOSED_LOOP_DIGEST_CANONICALIZATION,
            "planned_step_count": active.planned_step_count,
            "timebase": timebase,
            "neural_preparation_sha256": digest('b'),
            "neural_session_receipt_sha256": digest('e'),
            "neural_durable_evidence_profile": "none",
            "initial_snapshot_sha256": initial_snapshot_sha256,
            "last_verified_simulation_time_tics": active.steps.len() as u64 * timebase.runtime_step_duration_tics,
            "runtime_progress_disposition": "finished-and-host-verified",
            "step_receipts": active.steps.iter().map(|step| step.receipt_sha256.clone()).collect::<Vec<_>>(),
            "neural_execution_bindings": neural_executions.iter().map(|item| item.binding_sha256.clone()).collect::<Vec<_>>(),
            "runtime_finish_sha256": digest('d'),
            "runtime_lifecycle_binding_sha256": null,
            "cleanup_receipts": cleanup.iter().map(|item| item.receipt_sha256.clone()).collect::<Vec<_>>(),
            "status": "completed",
            "primary_reason_code": "loop.completed",
            "terminal_reason_code": "loop.completed",
        });
        let transcript = sha256_value(&transcript_value).expect("transcript");
        let mut source = SourceRunReceipt {
            schema_version: CLOSED_LOOP_RUN_SCHEMA.to_owned(),
            digest_canonicalization: CLOSED_LOOP_DIGEST_CANONICALIZATION.to_owned(),
            study_run_id: active.study_run_id.clone(),
            study_definition_sha256: active.study_definition_sha256.clone(),
            closed_loop_definition_sha256: active.closed_loop_definition_sha256.clone(),
            runtime_binding_sha256: active.runtime_binding_sha256.clone(),
            runtime_adapter_configuration_sha256: active
                .runtime_adapter_configuration_sha256
                .clone(),
            neural_provider_identity_sha256: active.neural_provider_identity_sha256.clone(),
            timebase,
            planned_step_count: active.planned_step_count,
            runtime_deadline_enforcement: "host-generation-kill".to_owned(),
            neural_deadline_enforcement: "cooperative-observed".to_owned(),
            neural_preparation_sha256: Some(digest('b')),
            neural_session_receipt_sha256: Some(digest('e')),
            neural_durable_evidence_profile: "none".to_owned(),
            initial_snapshot_sha256: Some(initial_snapshot_sha256),
            last_verified_simulation_time_tics: Some(
                active.steps.len() as u64 * test_timebase().runtime_step_duration_tics,
            ),
            runtime_progress_disposition: "finished-and-host-verified".to_owned(),
            steps: active.steps.clone(),
            neural_executions,
            runtime_finish_sha256: Some(digest('d')),
            runtime_lifecycle: None,
            cleanup: cleanup.clone(),
            status: "completed".to_owned(),
            primary_reason_code: "loop.completed".to_owned(),
            terminal_reason_code: "loop.completed".to_owned(),
            cleanup_complete: true,
            transcript_sha256: transcript,
            simulator_only: true,
            physical_actuation: false,
            ncp_qualified: false,
            scientific_authority: false,
            is_paper_local_evidence: false,
            calibrated_posterior: false,
            receipt_sha256: String::new(),
        };
        source.receipt_sha256 = source_run_receipt_sha256(&source).expect("run receipt");
        finish_from_source(&source)
    }

    fn encode_timebase(timebase: &ClosedLoopTimebase) -> Vec<RuntimeLifecycleScalar> {
        vec![
            RuntimeLifecycleScalar::Text(timebase.schema_version.clone()),
            RuntimeLifecycleScalar::Text(timebase.tic_unit.clone()),
            RuntimeLifecycleScalar::Unsigned(timebase.runtime_step_duration_tics),
            RuntimeLifecycleScalar::Unsigned(timebase.neural_step_duration_tics),
            RuntimeLifecycleScalar::Text(timebase.clock_relation.clone()),
            RuntimeLifecycleScalar::Text(timebase.coupling.clone()),
            RuntimeLifecycleScalar::Text(timebase.causality_policy.clone()),
            RuntimeLifecycleScalar::Text(timebase.dispatch_order.clone()),
            RuntimeLifecycleScalar::Text(timebase.observation_sample_phase.clone()),
            RuntimeLifecycleScalar::Text(timebase.action_application.clone()),
        ]
    }

    fn encode_neural_tail(
        tail: Option<&NeuralExecutionReceiptBinding>,
    ) -> Vec<RuntimeLifecycleScalar> {
        let Some(tail) = tail else {
            return vec![RuntimeLifecycleScalar::Null; 6];
        };
        vec![
            RuntimeLifecycleScalar::Unsigned(tail.step_index),
            RuntimeLifecycleScalar::Text(tail.step_id.clone()),
            RuntimeLifecycleScalar::Text(tail.neural_request_sha256.clone()),
            RuntimeLifecycleScalar::Text(tail.neural_result_sha256.clone()),
            RuntimeLifecycleScalar::Text(tail.provider_execution_scope.clone()),
            RuntimeLifecycleScalar::Text(tail.provider_execution_sha256.clone()),
        ]
    }

    fn encode_cleanup(receipt: &CleanupReceipt) -> Vec<RuntimeLifecycleScalar> {
        let optional_text = |value: &Option<String>| {
            value
                .clone()
                .map_or(RuntimeLifecycleScalar::Null, RuntimeLifecycleScalar::Text)
        };
        vec![
            RuntimeLifecycleScalar::Text(receipt.schema_version.clone()),
            RuntimeLifecycleScalar::Text(receipt.component.clone()),
            RuntimeLifecycleScalar::Text(receipt.owner_identity_sha256.clone()),
            RuntimeLifecycleScalar::Text(receipt.mode.clone()),
            RuntimeLifecycleScalar::Boolean(receipt.attempted),
            RuntimeLifecycleScalar::Boolean(receipt.confirmed),
            RuntimeLifecycleScalar::Boolean(receipt.containment_empty),
            RuntimeLifecycleScalar::Text(receipt.reason_code.clone()),
            optional_text(
                &receipt
                    .runtime_lifecycle
                    .as_ref()
                    .map(|item| item.binding_sha256.clone()),
            ),
            optional_text(&receipt.provider_terminal_receipt_sha256),
            optional_text(&receipt.provider_lifecycle_receipt_sha256),
            RuntimeLifecycleScalar::Text(receipt.receipt_sha256.clone()),
        ]
    }

    fn finish_from_source(source: &SourceRunReceipt) -> FinishRequest {
        let neural_tail = source.neural_executions.get(source.steps.len());
        FinishRequest {
            schema_version: FINISH_REQUEST_SCHEMA_ID.to_owned(),
            digest_canonicalization: source.digest_canonicalization.clone(),
            study_run_id: source.study_run_id.clone(),
            timebase_values: encode_timebase(&source.timebase),
            runtime_deadline_enforcement: source.runtime_deadline_enforcement.clone(),
            neural_deadline_enforcement: source.neural_deadline_enforcement.clone(),
            neural_preparation_sha256: source.neural_preparation_sha256.clone(),
            neural_session_receipt_sha256: source.neural_session_receipt_sha256.clone(),
            neural_durable_evidence_profile: source.neural_durable_evidence_profile.clone(),
            initial_snapshot_sha256: source.initial_snapshot_sha256.clone(),
            last_verified_simulation_time_tics: source.last_verified_simulation_time_tics,
            runtime_progress_disposition: source.runtime_progress_disposition.clone(),
            planned_step_count: source.planned_step_count,
            step_count: source.steps.len() as u64,
            neural_tail_values: encode_neural_tail(neural_tail),
            runtime_finish_sha256: source.runtime_finish_sha256.clone(),
            runtime_lifecycle_values: encode_runtime_lifecycle(source.runtime_lifecycle.as_ref()),
            runtime_cleanup_values: encode_cleanup(&source.cleanup[0]),
            neural_cleanup_values: encode_cleanup(&source.cleanup[1]),
            source_status: source.status.clone(),
            primary_reason_code: source.primary_reason_code.clone(),
            terminal_reason_code: source.terminal_reason_code.clone(),
            cleanup_complete: source.cleanup_complete,
            source_transcript_sha256: source.transcript_sha256.clone(),
            source_run_receipt_sha256: source.receipt_sha256.clone(),
            simulator_only: source.simulator_only,
            physical_actuation: source.physical_actuation,
            ncp_qualified: source.ncp_qualified,
            scientific_authority: source.scientific_authority,
            is_paper_local_evidence: source.is_paper_local_evidence,
            calibrated_posterior: source.calibrated_posterior,
        }
    }

    fn observe_from_source(source: &SourceStepReceipt) -> ObserveRequest {
        ObserveRequest {
            schema_version: OBSERVE_REQUEST_SCHEMA_ID.to_owned(),
            study_run_id: source.study_run_id.clone(),
            step_index: source.step_index,
            step_id: source.step_id.clone(),
            input_snapshot_sha256: source.input_snapshot_sha256.clone(),
            neural_request_sha256: source.neural_request_sha256.clone(),
            neural_result_sha256: source.neural_result_sha256.clone(),
            provider_execution_scope: source.provider_execution_scope.clone(),
            provider_execution_sha256: source.provider_execution_sha256.clone(),
            admitted_action_sha256: source.admitted_action_sha256.clone(),
            runtime_request_sha256: source.runtime_request_sha256.clone(),
            output_snapshot_sha256: source.output_snapshot_sha256.clone(),
            fault_codes: source.fault_codes.clone(),
            source_receipt_sha256: source.receipt_sha256.clone(),
        }
    }

    fn resign_observe_request(request: &mut ObserveRequest) {
        let mut source = SourceStepReceipt {
            schema_version: CLOSED_LOOP_STEP_SCHEMA.to_owned(),
            study_run_id: request.study_run_id.clone(),
            step_index: request.step_index,
            step_id: request.step_id.clone(),
            input_snapshot_sha256: request.input_snapshot_sha256.clone(),
            neural_request_sha256: request.neural_request_sha256.clone(),
            neural_result_sha256: request.neural_result_sha256.clone(),
            provider_execution_scope: request.provider_execution_scope.clone(),
            provider_execution_sha256: request.provider_execution_sha256.clone(),
            admitted_action_sha256: request.admitted_action_sha256.clone(),
            runtime_request_sha256: request.runtime_request_sha256.clone(),
            output_snapshot_sha256: request.output_snapshot_sha256.clone(),
            fault_codes: request.fault_codes.clone(),
            receipt_sha256: String::new(),
        };
        source.receipt_sha256 =
            source_step_receipt_sha256(&source).expect("resign observe source receipt");
        request.source_receipt_sha256 = source.receipt_sha256;
    }

    fn prepare_from_source(source: &SourceRunReceipt) -> PrepareRequest {
        PrepareRequest {
            schema_version: PREPARE_REQUEST_SCHEMA_ID.to_owned(),
            study_run_id: source.study_run_id.clone(),
            study_definition_sha256: source.study_definition_sha256.clone(),
            closed_loop_definition_sha256: source.closed_loop_definition_sha256.clone(),
            runtime_binding_sha256: source.runtime_binding_sha256.clone(),
            runtime_adapter_configuration_sha256: source
                .runtime_adapter_configuration_sha256
                .clone(),
            neural_provider_identity_sha256: source.neural_provider_identity_sha256.clone(),
            channel_ids: vec![
                "channel-01".to_owned(),
                "channel-02".to_owned(),
                "channel-03".to_owned(),
            ],
            subject_ids: vec![
                "drone-01".to_owned(),
                "drone-02".to_owned(),
                "drone-03".to_owned(),
            ],
            planned_step_count: source.planned_step_count,
            max_steps: 8,
        }
    }

    fn resign_source_run(source: &mut SourceRunReceipt) {
        for step in &mut source.steps {
            step.receipt_sha256 =
                source_step_receipt_sha256(step).expect("resign source step receipt");
        }
        for execution in &mut source.neural_executions {
            execution.binding_sha256 =
                neural_execution_binding_sha256(execution).expect("resign execution binding");
        }
        let transcript = json!({
            "domain": "engram-extension-closed-loop-transcript-v5",
            "digest_canonicalization": source.digest_canonicalization,
            "planned_step_count": source.planned_step_count,
            "timebase": source.timebase,
            "neural_preparation_sha256": source.neural_preparation_sha256,
            "neural_session_receipt_sha256": source.neural_session_receipt_sha256,
            "neural_durable_evidence_profile": source.neural_durable_evidence_profile,
            "initial_snapshot_sha256": source.initial_snapshot_sha256,
            "last_verified_simulation_time_tics": source.last_verified_simulation_time_tics,
            "runtime_progress_disposition": source.runtime_progress_disposition,
            "step_receipts": source.steps.iter().map(|step| step.receipt_sha256.clone()).collect::<Vec<_>>(),
            "neural_execution_bindings": source.neural_executions.iter().map(|item| item.binding_sha256.clone()).collect::<Vec<_>>(),
            "runtime_finish_sha256": source.runtime_finish_sha256,
            "runtime_lifecycle_binding_sha256": source.runtime_lifecycle.as_ref().map(|item| item.binding_sha256.clone()),
            "cleanup_receipts": source.cleanup.iter().map(|item| item.receipt_sha256.clone()).collect::<Vec<_>>(),
            "status": source.status,
            "primary_reason_code": source.primary_reason_code,
            "terminal_reason_code": source.terminal_reason_code,
        });
        source.transcript_sha256 = sha256_value(&transcript).expect("resign transcript");
        source.receipt_sha256 =
            source_run_receipt_sha256(source).expect("resign source run receipt");
    }

    #[test]
    fn prepare_accepts_one_channel() {
        let response = ObserverRuntime::new()
            .prepare(prepare_request(1))
            .expect("one channel is valid");

        assert_eq!(response.channel_count, 1);
    }

    #[test]
    fn prepare_accepts_two_and_three_channels() {
        for channel_count in [2, 3] {
            let response = ObserverRuntime::new()
                .prepare(prepare_request(channel_count))
                .expect("multi-subject channel roster is valid");

            assert_eq!(response.channel_count, channel_count as u64);
        }
    }

    #[test]
    fn prepare_accepts_sixty_four_channels() {
        let response = ObserverRuntime::new()
            .prepare(prepare_request(64))
            .expect("sixty-four channels are valid");

        assert_eq!(response.channel_count, 64);
    }

    #[test]
    fn prepare_accepts_the_maximum_immutable_step_plan() {
        let mut request = prepare_request(3);
        request.planned_step_count = MAX_STEPS;
        request.max_steps = MAX_STEPS;

        let response = ObserverRuntime::new()
            .prepare(request)
            .expect("maximum step plan is valid");

        assert_eq!(response.step_index, 0);
        assert!(!response.terminal);
    }

    #[test]
    fn observer_receipt_binds_counts_terminal_cleanup_and_authority() {
        let response = ObserverRuntime::new()
            .prepare(prepare_request(3))
            .expect("prepare response");
        assert_eq!(
            observer_response_receipt_sha256(&response).expect("semantic receipt"),
            response.observer_receipt_sha256
        );

        let mut variants = Vec::new();
        let mut changed = response.clone();
        changed.channel_count += 1;
        variants.push(changed);
        let mut changed = response.clone();
        changed.fault_count += 1;
        variants.push(changed);
        let mut changed = response.clone();
        changed.cumulative_fault_count += 1;
        variants.push(changed);
        let mut changed = response.clone();
        changed.terminal = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.state_cleared = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.is_paper_local_evidence = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.authority = "changed".to_owned();
        variants.push(changed);
        let mut changed = response.clone();
        changed.roster_authority = "changed".to_owned();
        variants.push(changed);
        let mut changed = response.clone();
        changed.source_roster_authenticated = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.descriptive_only = false;
        variants.push(changed);
        let mut changed = response.clone();
        changed.agent_bridge_command = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.physical_actuation = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.ncp_used = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.pid_result = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.source_durable_evidence_verified = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.scientific_authority = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.calibrated_posterior = true;
        variants.push(changed);
        let mut changed = response.clone();
        changed.reason = "changed".to_owned();
        variants.push(changed);
        let mut changed = response.clone();
        changed.source_receipt_sha256 = Some(digest('f'));
        variants.push(changed);
        let mut changed = response.clone();
        changed.request_sha256 = digest('f');
        variants.push(changed);
        let mut changed = response.clone();
        changed.observer_state_sha256 = digest('f');
        variants.push(changed);

        for changed in variants {
            let changed_receipt =
                observer_response_receipt_sha256(&changed).expect("changed semantic receipt");
            assert_ne!(changed_receipt, response.observer_receipt_sha256);
            assert_ne!(
                advance_transcript(
                    &empty_transcript_sha256().expect("empty transcript"),
                    &changed_receipt
                )
                .expect("changed transcript"),
                response.observer_transcript_sha256
            );
        }
    }

    #[test]
    fn prepare_rejects_unsorted_channel_roster() {
        let mut request = prepare_request(3);
        request.channel_ids.swap(0, 1);
        let result = ObserverRuntime::new().prepare(request);

        assert_eq!(
            result.expect_err("unsorted roster rejects"),
            ObserverError::InvalidInput
        );
    }

    #[test]
    fn prepare_rejects_aliased_runtime_and_neural_owners() {
        let mut request = prepare_request(3);
        request.neural_provider_identity_sha256 = request.runtime_binding_sha256.clone();

        let result = ObserverRuntime::new().prepare(request);

        assert_eq!(
            result.expect_err("aliased cleanup owners reject"),
            ObserverError::InvalidInput
        );
    }

    #[test]
    fn prepare_rejects_a_plan_larger_than_its_step_budget() {
        let mut request = prepare_request(3);
        request.planned_step_count = 9;

        let result = ObserverRuntime::new().prepare(request);

        assert_eq!(
            result.expect_err("oversized immutable plan rejects"),
            ObserverError::InvalidInput
        );
    }

    #[test]
    fn observe_rejects_a_step_beyond_the_immutable_plan() {
        let mut runtime = ObserverRuntime::new();
        runtime.prepare(prepare_request(3)).expect("prepare");
        runtime
            .observe(observe_request(1, 3))
            .expect("planned step");

        let result = runtime.observe(observe_request(2, 3));

        assert_eq!(
            result.expect_err("unplanned step rejects"),
            ObserverError::StepBudgetExhausted
        );
    }

    #[test]
    fn observe_accepts_exact_next_source_receipt() {
        let mut runtime = ObserverRuntime::new();
        runtime.prepare(prepare_request(3)).expect("prepare");

        let response = runtime
            .observe(observe_request(1, 3))
            .expect("exact receipt is observed");

        assert_eq!(response.step_index, 1);
        assert!(!response.source_durable_evidence_verified);
    }

    #[test]
    fn observe_accepts_exact_fault_code_byte_boundaries() {
        for accepted in ["a".repeat(MAX_FAULT_CODE_BYTES), "é".repeat(64)] {
            let mut runtime = ObserverRuntime::new();
            runtime.prepare(prepare_request(1)).expect("prepare");
            let mut request = observe_request(1, 1);
            request.fault_codes = vec![accepted];
            resign_observe_request(&mut request);

            runtime
                .observe(request)
                .expect("an exact 128-byte fault code is valid");
        }
    }

    #[test]
    fn observe_rejects_fault_codes_over_128_utf8_bytes_without_state_loss() {
        for rejected in ["a".repeat(MAX_FAULT_CODE_BYTES + 1), "é".repeat(65)] {
            let mut runtime = ObserverRuntime::new();
            let prepared = runtime.prepare(prepare_request(1)).expect("prepare");
            let mut request = observe_request(1, 1);
            request.fault_codes = vec![rejected];
            resign_observe_request(&mut request);

            assert_eq!(
                runtime
                    .observe(request)
                    .expect_err("an oversized UTF-8 fault code rejects"),
                ObserverError::InvalidInput
            );
            let Phase::Active(active) = &runtime.phase else {
                panic!("rejected observation cannot clear state")
            };
            assert!(active.steps.is_empty());
            assert_eq!(active.state_sha256, prepared.observer_state_sha256);

            runtime
                .observe(observe_request(1, 1))
                .expect("an exact request remains admissible after rejection");
        }
    }

    #[test]
    fn observe_rejects_replayed_step() {
        let mut runtime = ObserverRuntime::new();
        runtime.prepare(prepare_request(3)).expect("prepare");
        runtime.observe(observe_request(1, 3)).expect("first step");

        let result = runtime.observe(observe_request(1, 3));

        assert_eq!(
            result.expect_err("replay rejects"),
            ObserverError::StepOutOfOrder
        );
    }

    #[test]
    fn observe_rejects_source_digest_drift() {
        let mut runtime = ObserverRuntime::new();
        runtime.prepare(prepare_request(3)).expect("prepare");
        let mut request = observe_request(1, 3);
        request.source_receipt_sha256 = digest('f');

        let result = runtime.observe(request);

        assert_eq!(
            result.expect_err("digest drift rejects"),
            ObserverError::SourceStepDigestMismatch
        );
    }

    #[test]
    fn finish_verifies_full_source_run_and_clears_state() {
        let mut runtime = ObserverRuntime::new();
        runtime.prepare(prepare_request(3)).expect("prepare");
        runtime.observe(observe_request(1, 3)).expect("step");
        let request = finish_request(&runtime);

        let response = runtime.finish(request).expect("finish");

        assert!(response.state_cleared && runtime.is_cleared());
        assert!(!response.source_durable_evidence_verified);
    }

    #[test]
    fn finish_observes_durable_profile_without_claiming_bundle_verification() {
        let mut source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");
        source.neural_durable_evidence_profile =
            "engram.nest-closed-loop-evidence-bundle.v2".to_owned();
        resign_source_run(&mut source);

        let mut runtime = ObserverRuntime::new();
        let prepared = runtime
            .prepare(prepare_from_source(&source))
            .expect("prepare source observer");
        assert!(!prepared.source_durable_evidence_verified);
        for step in &source.steps {
            runtime
                .observe(observe_from_source(step))
                .expect("observe source step");
        }

        let response = runtime
            .finish(finish_from_source(&source))
            .expect("the terminal source profile is observable");
        assert!(!response.source_durable_evidence_verified);
        assert!(response.state_cleared);
    }

    #[test]
    fn finish_rejects_unknown_durable_profile_without_state_loss() {
        let source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");
        let mut runtime = ObserverRuntime::new();
        runtime
            .prepare(prepare_from_source(&source))
            .expect("prepare source observer");
        for step in &source.steps {
            runtime
                .observe(observe_from_source(step))
                .expect("observe source step");
        }
        let Phase::Active(active) = &runtime.phase else {
            panic!("source observer remains active")
        };
        let prior_state_sha256 = active.state_sha256.clone();
        let prior_transcript_sha256 = active.transcript_sha256.clone();
        let mut invalid = source.clone();
        invalid.neural_durable_evidence_profile = "future-profile".to_owned();
        resign_source_run(&mut invalid);

        assert_eq!(
            runtime
                .finish(finish_from_source(&invalid))
                .expect_err("an unknown durable-evidence profile rejects"),
            ObserverError::InvalidInput
        );
        let Phase::Active(active) = &runtime.phase else {
            panic!("rejected finish cannot clear state")
        };
        assert_eq!(active.state_sha256, prior_state_sha256);
        assert_eq!(active.transcript_sha256, prior_transcript_sha256);

        runtime
            .finish(finish_from_source(&source))
            .expect("the exact source finish remains admissible after rejection");
    }

    #[test]
    fn finish_rejects_source_run_digest_drift() {
        let mut runtime = ObserverRuntime::new();
        runtime.prepare(prepare_request(3)).expect("prepare");
        runtime.observe(observe_request(1, 3)).expect("step");
        let mut request = finish_request(&runtime);
        request.source_run_receipt_sha256 = digest('e');

        let result = runtime.finish(request);

        assert_eq!(
            result.expect_err("run digest drift rejects"),
            ObserverError::SourceRunDigestMismatch
        );
    }

    #[test]
    fn generated_engram_receipt_verifies_neural_session_lineage() {
        let source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");

        validate_source_run_receipt(&source).expect("exact Engram receipt verifies");
        assert_eq!(
            source_run_receipt_sha256(&source).expect("source digest"),
            source.receipt_sha256
        );
        assert_eq!(
            source.neural_durable_evidence_profile,
            "engram.nest-closed-loop-evidence-bundle.v2"
        );
        assert!(source.neural_session_receipt_sha256.is_some());
        assert!(source.runtime_lifecycle.is_some());
        assert_eq!(
            source.runtime_lifecycle.as_ref(),
            source.cleanup[0].runtime_lifecycle.as_ref()
        );
        let projection = encode_runtime_lifecycle(source.runtime_lifecycle.as_ref());
        assert_eq!(
            decode_runtime_lifecycle(&projection).expect("scalar lifecycle projection"),
            source.runtime_lifecycle.clone()
        );
        let mut partial = projection;
        partial[0] = RuntimeLifecycleScalar::Null;
        assert_eq!(
            decode_runtime_lifecycle(&partial).expect_err("partial lifecycle rejects"),
            ObserverError::InvalidInput
        );

        let mut drifted = source;
        drifted.neural_session_receipt_sha256 = None;
        let error = validate_source_run_receipt(&drifted)
            .expect_err("neural session lineage drift fails closed");
        assert_eq!(error, ObserverError::SourceTerminalMismatch);
    }

    #[test]
    fn source_v2_timebase_execution_and_provider_cleanup_fields_are_closed() {
        let source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");
        validate_source_run_receipt(&source).expect("exact V2 source receipt verifies");
        assert_eq!(
            source.digest_canonicalization,
            CLOSED_LOOP_DIGEST_CANONICALIZATION
        );
        assert_eq!(source.neural_executions.len(), source.steps.len());
        assert!(source.cleanup[1].provider_terminal_receipt_sha256.is_some());
        assert!(source.cleanup[1]
            .provider_lifecycle_receipt_sha256
            .is_some());

        let mut unsigned_cleanup_tamper = source.clone();
        unsigned_cleanup_tamper.cleanup[1].provider_terminal_receipt_sha256 = Some(digest('0'));
        assert_eq!(
            validate_source_run_receipt(&unsigned_cleanup_tamper)
                .expect_err("provider terminal drift requires a new cleanup receipt"),
            ObserverError::SourceCleanupDigestMismatch
        );

        let mut rebound_cleanup = source.clone();
        rebound_cleanup.cleanup[1].provider_terminal_receipt_sha256 = Some(digest('0'));
        rebound_cleanup.cleanup[1].receipt_sha256 =
            source_cleanup_receipt_sha256(&rebound_cleanup.cleanup[1])
                .expect("rebind provider terminal receipt");
        resign_source_run(&mut rebound_cleanup);
        validate_source_run_receipt(&rebound_cleanup)
            .expect("a new fully bound descriptive provider receipt remains observable");
        assert_ne!(rebound_cleanup.receipt_sha256, source.receipt_sha256);

        let mut semantic_variants = Vec::new();
        let mut changed = source.clone();
        changed.digest_canonicalization = "engram.unknown-json.v1".to_owned();
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.timebase.schema_version = "engram.extension-closed-loop-timebase.v0".to_owned();
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.timebase.runtime_step_duration_tics = 0;
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.timebase.neural_step_duration_tics = MAX_STEP_DURATION_TICS + 1;
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.timebase.clock_relation = "shared-clock".to_owned();
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.timebase.coupling = "unbound".to_owned();
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.timebase.causality_policy = "future-policy".to_owned();
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.timebase.dispatch_order = "runtime-before-observe".to_owned();
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.timebase.observation_sample_phase = "runtime-interval-end".to_owned();
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.timebase.action_application = "immediate".to_owned();
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.last_verified_simulation_time_tics = Some(1);
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.runtime_progress_disposition = "last-host-verified".to_owned();
        semantic_variants.push(changed);
        let mut changed = source.clone();
        changed.steps[0].provider_execution_scope = "unknown-provider-scope".to_owned();
        semantic_variants.push(changed);
        let mut changed = source;
        changed.neural_executions[0].provider_execution_sha256 = digest('0');
        semantic_variants.push(changed);

        for mut changed in semantic_variants {
            resign_source_run(&mut changed);
            assert_eq!(
                validate_source_run_receipt(&changed)
                    .expect_err("re-signed V2 semantic drift must fail closed"),
                ObserverError::SourceTerminalMismatch
            );
        }
    }

    #[test]
    fn zero_step_neural_tail_is_run_bound_and_foreign_tail_rejects() {
        let mut source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-zero-step-run-receipt.generated.json"
        ))
        .expect("Engram-generated zero-step receipt");
        source.neural_preparation_sha256 = Some(digest('1'));
        source.neural_session_receipt_sha256 = Some(digest('2'));
        source.initial_snapshot_sha256 = Some(digest('3'));
        source.last_verified_simulation_time_tics = Some(0);
        source.runtime_progress_disposition = "unknown-after-operation-attempt".to_owned();
        source
            .neural_executions
            .push(NeuralExecutionReceiptBinding {
                schema_version: CLOSED_LOOP_EXECUTION_SCHEMA.to_owned(),
                step_index: 1,
                step_id: closed_loop_step_id(&source.study_run_id, 1).expect("tail step id"),
                neural_request_sha256: digest('4'),
                neural_result_sha256: digest('5'),
                provider_execution_scope: "nest-exact-step-readback".to_owned(),
                provider_execution_sha256: digest('6'),
                binding_sha256: String::new(),
            });
        resign_source_run(&mut source);
        validate_source_run_receipt(&source)
            .expect("one execution beyond zero completed runtime steps is observable");

        source.neural_executions[0].step_id =
            closed_loop_step_id("foreign-study-run", 1).expect("foreign tail step id");
        resign_source_run(&mut source);
        assert_eq!(
            validate_source_run_receipt(&source)
                .expect_err("fully re-signed foreign-run neural tail rejects"),
            ObserverError::SourceTerminalMismatch
        );
    }

    #[test]
    fn finish_scalar_projections_reject_partial_and_tampered_rows_without_state_loss() {
        let source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");
        let mut runtime = ObserverRuntime::new();
        runtime
            .prepare(prepare_from_source(&source))
            .expect("prepare observer");
        for step in &source.steps {
            runtime
                .observe(observe_from_source(step))
                .expect("observe exact source step");
        }
        let exact = finish_from_source(&source);

        let mut invalid_timebase = exact.clone();
        invalid_timebase.timebase_values[2] = RuntimeLifecycleScalar::Boolean(true);
        assert_eq!(
            runtime
                .finish(invalid_timebase)
                .expect_err("typed timebase slot drift rejects"),
            ObserverError::InvalidInput
        );

        let mut partial_tail = exact.clone();
        partial_tail.neural_tail_values[0] = RuntimeLifecycleScalar::Unsigned(3);
        assert_eq!(
            runtime
                .finish(partial_tail)
                .expect_err("partial neural tail projection rejects"),
            ObserverError::InvalidInput
        );

        let mut short_cleanup = exact.clone();
        short_cleanup.runtime_cleanup_values.pop();
        assert_eq!(
            runtime
                .finish(short_cleanup)
                .expect_err("short cleanup projection rejects"),
            ObserverError::InvalidInput
        );

        let mut unsigned_provider_tamper = exact.clone();
        unsigned_provider_tamper.neural_cleanup_values[9] =
            RuntimeLifecycleScalar::Text(digest('0'));
        assert_eq!(
            runtime
                .finish(unsigned_provider_tamper)
                .expect_err("unbound provider tail projection rejects"),
            ObserverError::SourceCleanupDigestMismatch
        );

        let response = runtime
            .finish(exact)
            .expect("exact finish remains admissible after rejected projections");
        assert!(response.terminal && response.state_cleared);
    }

    #[test]
    fn accepted_source_receipt_version_is_float_free() {
        for fixture in [
            include_str!(
                "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
            ),
            include_str!(
                "../../../integrations/engram/managed-observer/fixtures/engram-runtime-finished-neural-cleanup-failed.generated.json"
            ),
            include_str!(
                "../../../integrations/engram/managed-observer/fixtures/engram-zero-step-run-receipt.generated.json"
            ),
        ] {
            let value: Value = serde_json::from_str(fixture).expect("source receipt JSON");
            assert!(source_receipt_value_is_float_free(&value));
        }
        assert!(!source_receipt_value_is_float_free(&json!({
            "future_float": 1e-5
        })));
    }

    #[test]
    fn source_snapshot_chain_rejects_resigned_first_and_later_disconnects() {
        let source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");

        let mut disconnected_first = source.clone();
        disconnected_first.steps[0].input_snapshot_sha256 = digest('d');
        resign_source_run(&mut disconnected_first);
        assert_eq!(
            validate_source_run_receipt(&disconnected_first)
                .expect_err("resigned first-step disconnect rejects"),
            ObserverError::SourceTerminalMismatch
        );

        let mut disconnected_later = source;
        disconnected_later.steps[1].input_snapshot_sha256 = digest('e');
        resign_source_run(&mut disconnected_later);
        assert_eq!(
            validate_source_run_receipt(&disconnected_later)
                .expect_err("resigned later-step disconnect rejects"),
            ObserverError::SourceTerminalMismatch
        );

        let mut foreign_run: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");
        foreign_run.steps[1].study_run_id = "foreign-study-run".to_owned();
        foreign_run.steps[1].step_id = closed_loop_step_id(
            &foreign_run.steps[1].study_run_id,
            foreign_run.steps[1].step_index,
        )
        .expect("foreign step identifier");
        resign_source_run(&mut foreign_run);
        assert_eq!(
            validate_source_run_receipt(&foreign_run)
                .expect_err("resigned foreign-run step rejects"),
            ObserverError::SourceTerminalMismatch
        );
    }

    #[test]
    fn observe_rejects_a_resigned_later_snapshot_disconnect_before_state_mutation() {
        let source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");
        let mut runtime = ObserverRuntime::new();
        runtime
            .prepare(prepare_from_source(&source))
            .expect("prepare source observer");
        runtime
            .observe(observe_from_source(&source.steps[0]))
            .expect("observe connected first step");
        let Phase::Active(active) = &runtime.phase else {
            panic!("source observer remains active")
        };
        let prior_state_sha256 = active.state_sha256.clone();
        let prior_transcript_sha256 = active.transcript_sha256.clone();

        let mut disconnected = source.steps[1].clone();
        disconnected.input_snapshot_sha256 = digest('e');
        disconnected.receipt_sha256 =
            source_step_receipt_sha256(&disconnected).expect("resign disconnected step");
        assert_eq!(
            runtime
                .observe(observe_from_source(&disconnected))
                .expect_err("resigned later-step disconnect rejects at admission"),
            ObserverError::SourceSnapshotMismatch
        );
        let Phase::Active(active) = &runtime.phase else {
            panic!("rejected step cannot clear source observer")
        };
        assert_eq!(active.steps.len(), 1);
        assert_eq!(active.state_sha256, prior_state_sha256);
        assert_eq!(active.transcript_sha256, prior_transcript_sha256);

        runtime
            .observe(observe_from_source(&source.steps[1]))
            .expect("exact connected step remains admissible after rejection");
    }

    #[test]
    fn runtime_cleanup_mode_accepts_only_its_exact_lifecycle_dispositions() {
        let source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");
        let mut cleanup = source.cleanup[0].clone();

        assert!(valid_cleanup(&cleanup));
        for disposition in ["terminated", "killed"] {
            let lifecycle = cleanup
                .runtime_lifecycle
                .as_mut()
                .expect("fixture lifecycle");
            lifecycle.termination_disposition = disposition.to_owned();
            lifecycle.binding_sha256 =
                runtime_lifecycle_binding_sha256(lifecycle).expect("lifecycle digest");
            cleanup.mode = "generation-kill".to_owned();
            assert!(valid_cleanup(&cleanup));
        }

        let lifecycle = cleanup
            .runtime_lifecycle
            .as_mut()
            .expect("fixture lifecycle");
        lifecycle.termination_disposition = "unconfirmed".to_owned();
        lifecycle.binding_sha256 =
            runtime_lifecycle_binding_sha256(lifecycle).expect("lifecycle digest");
        cleanup.confirmed = false;
        assert!(!valid_cleanup(&cleanup));
    }

    #[test]
    fn source_terminal_invariants_reject_every_nearby_shape() {
        let completed: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-run-receipt.generated.json"
        ))
        .expect("Engram-generated source receipt");
        let zero_step: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-zero-step-run-receipt.generated.json"
        ))
        .expect("Engram-generated zero-step receipt");
        let aggregate_failure: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-runtime-finished-neural-cleanup-failed.generated.json"
        ))
        .expect("Engram-generated aggregate failure receipt");
        let mut variants = Vec::new();

        let mut changed = completed.clone();
        changed.cleanup.pop();
        variants.push(changed);
        let mut changed = completed.clone();
        changed.cleanup.push(changed.cleanup[0].clone());
        variants.push(changed);
        let mut changed = completed.clone();
        changed.cleanup.swap(0, 1);
        variants.push(changed);
        let mut changed = completed.clone();
        changed.cleanup[0].owner_identity_sha256 = digest('f');
        variants.push(changed);
        let mut changed = completed.clone();
        changed.cleanup[1].owner_identity_sha256 = changed.runtime_binding_sha256.clone();
        variants.push(changed);
        let mut changed = completed.clone();
        changed.cleanup[0].mode = "generation-kill".to_owned();
        variants.push(changed);
        let mut changed = completed.clone();
        changed.cleanup[1].mode = "finish".to_owned();
        variants.push(changed);
        let mut changed = completed.clone();
        changed.runtime_lifecycle = None;
        variants.push(changed);
        let mut changed = completed.clone();
        changed.cleanup[0].runtime_lifecycle = None;
        variants.push(changed);
        let mut changed = completed.clone();
        changed.cleanup[1].runtime_lifecycle = changed.runtime_lifecycle.clone();
        variants.push(changed);
        let mut changed = completed.clone();
        if let Some(lifecycle) = &mut changed.runtime_lifecycle {
            lifecycle.publisher_authenticated = true;
        }
        changed.cleanup[0].runtime_lifecycle = changed.runtime_lifecycle.clone();
        variants.push(changed);
        let mut changed = completed.clone();
        if let Some(lifecycle) = &mut changed.runtime_lifecycle {
            lifecycle.child_reaped = false;
        }
        changed.cleanup[0].runtime_lifecycle = changed.runtime_lifecycle.clone();
        variants.push(changed);
        let mut changed = completed.clone();
        changed.neural_preparation_sha256 = None;
        variants.push(changed);
        let mut changed = completed.clone();
        changed.neural_session_receipt_sha256 = None;
        variants.push(changed);
        let mut changed = completed.clone();
        changed.initial_snapshot_sha256 = None;
        variants.push(changed);
        let mut changed = completed.clone();
        changed.steps.clear();
        variants.push(changed);
        let mut changed = completed.clone();
        changed.planned_step_count = 0;
        variants.push(changed);
        let mut changed = completed.clone();
        changed.planned_step_count = 1;
        variants.push(changed);
        let mut changed = completed.clone();
        changed.planned_step_count = 3;
        variants.push(changed);
        let mut changed = completed.clone();
        changed.runtime_finish_sha256 = None;
        variants.push(changed);
        let mut changed = completed.clone();
        changed.terminal_reason_code = "changed".to_owned();
        variants.push(changed);
        let mut changed = completed.clone();
        changed.status = "failed".to_owned();
        variants.push(changed);
        let mut changed = completed.clone();
        changed.primary_reason_code = "runtime.overload".to_owned();
        changed.terminal_reason_code = "runtime.overload".to_owned();
        variants.push(changed);
        let mut changed = completed.clone();
        changed.status = "failed".to_owned();
        changed.neural_preparation_sha256 = None;
        changed.neural_session_receipt_sha256 = None;
        changed.initial_snapshot_sha256 = None;
        variants.push(changed);
        let mut changed = zero_step.clone();
        changed.status = "completed".to_owned();
        changed.cleanup[0].mode = "finish".to_owned();
        variants.push(changed);
        let mut changed = zero_step.clone();
        changed.cleanup[0].mode = "finish".to_owned();
        variants.push(changed);
        let mut changed = zero_step.clone();
        changed.primary_reason_code = "loop.completed".to_owned();
        changed.terminal_reason_code = "loop.completed".to_owned();
        variants.push(changed);
        let mut changed = zero_step.clone();
        changed.status = "cancelled".to_owned();
        variants.push(changed);
        let mut changed = zero_step.clone();
        changed.status = "overloaded".to_owned();
        variants.push(changed);
        let mut changed = zero_step.clone();
        changed.neural_preparation_sha256 = Some(digest('1'));
        changed.neural_session_receipt_sha256 = Some(digest('2'));
        variants.push(changed);
        let mut changed = aggregate_failure;
        changed.steps.clear();
        variants.push(changed);
        let mut changed = zero_step.clone();
        changed.cleanup[0].confirmed = false;
        changed.cleanup[0].containment_empty = false;
        changed.cleanup_complete = false;
        changed.status = "cancelled".to_owned();
        changed.primary_reason_code = "cancelled".to_owned();
        changed.terminal_reason_code = "cleanup.unconfirmed".to_owned();
        variants.push(changed);
        let mut changed = zero_step;
        changed.cleanup[0].confirmed = false;
        changed.cleanup[0].containment_empty = false;
        changed.cleanup_complete = false;
        changed.terminal_reason_code = changed.primary_reason_code.clone();
        variants.push(changed);

        for changed in variants {
            assert!(
                matches!(
                    validate_source_run_receipt(&changed)
                        .expect_err("nearby terminal shape must fail closed"),
                    ObserverError::SourceTerminalMismatch
                        | ObserverError::SourceCleanupDigestMismatch
                ),
                "nearby terminal shape returned an unrelated error"
            );
        }
    }

    #[test]
    fn runtime_finish_with_unconfirmed_neural_cleanup_is_observed_as_failed_source() {
        let source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-runtime-finished-neural-cleanup-failed.generated.json"
        ))
        .expect("Engram-generated neural cleanup failure receipt");
        validate_source_run_receipt(&source).expect("exact aggregate failure verifies");
        assert_eq!(source.status, "failed");
        assert!(source.runtime_finish_sha256.is_some());
        assert_eq!(source.cleanup[0].mode, "finish");
        assert!(!source.cleanup_complete);

        let mut runtime = ObserverRuntime::new();
        runtime
            .prepare(prepare_from_source(&source))
            .expect("prepare observer");
        for step in &source.steps {
            runtime
                .observe(observe_from_source(step))
                .expect("observe exact source step");
        }
        let response = runtime
            .finish(finish_from_source(&source))
            .expect("aggregate source failure remains observable");

        assert_eq!(response.outcome, ObserverOutcome::Succeeded);
        assert_eq!(response.reason, "finished");
        assert!(response.descriptive_only && response.state_cleared);
    }

    #[test]
    fn generated_zero_step_failure_finishes_and_count_mismatch_rejects() {
        let source: SourceRunReceipt = serde_json::from_str(include_str!(
            "../../../integrations/engram/managed-observer/fixtures/engram-zero-step-run-receipt.generated.json"
        ))
        .expect("Engram-generated zero-step receipt");
        validate_source_run_receipt(&source).expect("zero-step source verifies");

        let mut runtime = ObserverRuntime::new();
        runtime
            .prepare(prepare_from_source(&source))
            .expect("prepare observer");
        let response = runtime
            .finish(finish_from_source(&source))
            .expect("failed source run is observed descriptively");
        assert_eq!(response.outcome, ObserverOutcome::Succeeded);
        assert_eq!(response.step_index, 0);
        assert!(response.state_cleared);

        let mut mismatch_runtime = ObserverRuntime::new();
        mismatch_runtime
            .prepare(prepare_from_source(&source))
            .expect("prepare mismatch observer");
        let mut mismatch = finish_from_source(&source);
        mismatch.step_count = 1;
        let error = mismatch_runtime
            .finish(mismatch)
            .expect_err("zero-step count mismatch fails closed");
        assert_eq!(error, ObserverError::InvalidInput);

        let mut plan_mismatch_runtime = ObserverRuntime::new();
        plan_mismatch_runtime
            .prepare(prepare_from_source(&source))
            .expect("prepare plan mismatch observer");
        let mut plan_mismatch = finish_from_source(&source);
        plan_mismatch.planned_step_count = 2;
        let error = plan_mismatch_runtime
            .finish(plan_mismatch)
            .expect_err("immutable plan mismatch fails closed");
        assert_eq!(error, ObserverError::InvalidInput);
    }
}
