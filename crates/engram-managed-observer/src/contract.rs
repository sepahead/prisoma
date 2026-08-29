use serde::{Deserialize, Serialize};

pub(crate) const PROFILE: &str = "engram.reviewed-native-development.v1";
pub(crate) const IPC_PROTOCOL: &str = "engram.managed-runtime-ipc.v1";
pub(crate) const LAUNCH_ABI: &str = "engram.managed-runtime-stdio.v1";
pub(crate) const CONFIGURATION_SCHEMA_ID: &str = "prisoma.observer.configuration.v1";
pub(crate) const FINISH_OPERATION_ID: &str = "prisoma.observer.finish.v1";
pub(crate) const FINISH_REQUEST_SCHEMA_ID: &str = "prisoma.observer.finish-request.v1";
pub(crate) const FINISH_RESPONSE_SCHEMA_ID: &str = "prisoma.observer.finish-response.v1";
pub(crate) const OBSERVE_OPERATION_ID: &str = "prisoma.observer.observe.v1";
pub(crate) const OBSERVE_REQUEST_SCHEMA_ID: &str = "prisoma.observer.observe-request.v1";
pub(crate) const OBSERVE_RESPONSE_SCHEMA_ID: &str = "prisoma.observer.observe-response.v1";
pub(crate) const PREPARE_OPERATION_ID: &str = "prisoma.observer.prepare.v1";
pub(crate) const PREPARE_REQUEST_SCHEMA_ID: &str = "prisoma.observer.prepare-request.v1";
pub(crate) const PREPARE_RESPONSE_SCHEMA_ID: &str = "prisoma.observer.prepare-response.v1";

pub(crate) const CONFIGURATION_SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../integrations/engram/managed-observer/contracts/configuration.schema.json"
);
pub(crate) const IPC_SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../integrations/engram/managed-observer/contracts/managed-runtime-ipc.schema.json"
);
pub(crate) const FINISH_REQUEST_SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../integrations/engram/managed-observer/contracts/finish-request.schema.json"
);
pub(crate) const FINISH_RESPONSE_SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../integrations/engram/managed-observer/contracts/finish-response.schema.json"
);
pub(crate) const OBSERVE_REQUEST_SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../integrations/engram/managed-observer/contracts/observe-request.schema.json"
);
pub(crate) const OBSERVE_RESPONSE_SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../integrations/engram/managed-observer/contracts/observe-response.schema.json"
);
pub(crate) const PREPARE_REQUEST_SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../integrations/engram/managed-observer/contracts/prepare-request.schema.json"
);
pub(crate) const PREPARE_RESPONSE_SCHEMA_BYTES: &[u8] = include_bytes!(
    "../../../integrations/engram/managed-observer/contracts/prepare-response.schema.json"
);

pub(crate) const MAX_CHANNELS: usize = 64;
pub(crate) const MAX_STEPS: u64 = 1_024;
pub(crate) const MAX_CLEANUP_RECEIPTS: usize = 2;
pub(crate) const MAX_FAULT_CODE_BYTES: usize = 128;
pub(crate) const MAX_REASON_BYTES: usize = 256;
pub(crate) const MAX_FRAME_BYTES: usize = 65_536;
pub(crate) const MAX_REJECTED_OPERATION_ATTEMPTS: u64 = 16;
pub(crate) const MAX_OPERATIONS_PER_GENERATION: u64 =
    MAX_STEPS + 2 + MAX_REJECTED_OPERATION_ATTEMPTS;
pub(crate) const OPERATION_TIMEOUT_MS: u64 = 1_000;
pub(crate) const AUTHORITY: &str = "read-only-observer";

/// Fixed host-provided limits for the observer runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfiguration {
    pub schema_version: String,
    pub max_channels: u64,
    pub max_steps: u64,
    pub max_cleanup_receipts: u64,
    pub max_reason_bytes: u64,
}

impl RuntimeConfiguration {
    pub(crate) fn validate(&self) -> bool {
        self.schema_version == CONFIGURATION_SCHEMA_ID
            && self.max_channels == MAX_CHANNELS as u64
            && self.max_steps == MAX_STEPS
            && self.max_cleanup_receipts == MAX_CLEANUP_RECEIPTS as u64
            && self.max_reason_bytes == MAX_REASON_BYTES as u64
    }
}

/// Begin observation of one immutable Engram closed-loop identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareRequest {
    pub schema_version: String,
    pub study_run_id: String,
    pub study_definition_sha256: String,
    pub closed_loop_definition_sha256: String,
    pub runtime_binding_sha256: String,
    pub runtime_adapter_configuration_sha256: String,
    pub neural_provider_identity_sha256: String,
    pub channel_ids: Vec<String>,
    pub subject_ids: Vec<String>,
    pub planned_step_count: u64,
    pub max_steps: u64,
}

/// Observe one complete, content-bound Engram closed-loop step receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserveRequest {
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
    pub source_receipt_sha256: String,
}

/// One exact Engram reviewed-runtime lifecycle binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLifecycleReceiptBinding {
    pub schema_version: String,
    pub profile: String,
    pub generation_id: String,
    pub launch_source: String,
    pub store_id: Option<String>,
    pub package_generation_id: String,
    pub generation_directory_identity_sha256: Option<String>,
    pub package_generation_lease_retained_at_launch: bool,
    pub package_generation_lease_released: bool,
    pub handshake_receipt_sha256: String,
    pub termination_receipt_sha256: String,
    pub termination_disposition: String,
    pub child_reaped: bool,
    pub containment_empty: bool,
    pub diagnostic_stream_complete: bool,
    pub private_work_directory_removed: bool,
    pub publisher_authenticated: bool,
    pub durable_process_launch_authority: bool,
    pub ncp_authority: bool,
    pub physical_authority: bool,
    pub scientific_authority: bool,
    pub binding_sha256: String,
}

/// One scalar in the fixed Host API 2 runtime-lifecycle projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuntimeLifecycleScalar {
    Text(String),
    Unsigned(u64),
    Boolean(bool),
    Null,
}

/// Verify one terminal Engram run receipt and clear all observer state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinishRequest {
    pub schema_version: String,
    pub digest_canonicalization: String,
    pub study_run_id: String,
    pub timebase_values: Vec<RuntimeLifecycleScalar>,
    pub runtime_deadline_enforcement: String,
    pub neural_deadline_enforcement: String,
    pub neural_preparation_sha256: Option<String>,
    pub neural_session_receipt_sha256: Option<String>,
    pub neural_durable_evidence_profile: String,
    pub initial_snapshot_sha256: Option<String>,
    pub last_verified_simulation_time_tics: Option<u64>,
    pub runtime_progress_disposition: String,
    pub planned_step_count: u64,
    pub step_count: u64,
    pub neural_tail_values: Vec<RuntimeLifecycleScalar>,
    pub runtime_finish_sha256: Option<String>,
    pub runtime_lifecycle_values: Vec<RuntimeLifecycleScalar>,
    pub runtime_cleanup_values: Vec<RuntimeLifecycleScalar>,
    pub neural_cleanup_values: Vec<RuntimeLifecycleScalar>,
    pub source_status: String,
    pub primary_reason_code: String,
    pub terminal_reason_code: String,
    pub cleanup_complete: bool,
    pub source_transcript_sha256: String,
    pub source_run_receipt_sha256: String,
    pub simulator_only: bool,
    pub physical_actuation: bool,
    pub ncp_qualified: bool,
    pub scientific_authority: bool,
    pub is_paper_local_evidence: bool,
    pub calibrated_posterior: bool,
}

/// Project-level outcome inside one generic managed-runtime response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObserverOutcome {
    Succeeded,
    Rejected,
    Failed,
}

impl ObserverOutcome {
    pub(crate) fn ipc_status(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Rejected => "rejected",
            Self::Failed => "failed",
        }
    }
}

/// Deterministic, descriptive observer receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverResponse {
    pub schema_version: String,
    pub outcome: ObserverOutcome,
    pub reason: String,
    pub authority: String,
    pub roster_authority: String,
    pub source_roster_authenticated: bool,
    pub study_run_id: String,
    pub step_index: u64,
    pub channel_count: u64,
    pub fault_count: u64,
    pub cumulative_fault_count: u64,
    pub source_receipt_sha256: Option<String>,
    pub prior_observer_state_sha256: String,
    pub observer_state_sha256: String,
    pub request_sha256: String,
    pub observer_receipt_sha256: String,
    /// Rolling digest of accepted semantic receipts, excluding rejected attempts.
    pub observer_transcript_sha256: String,
    pub terminal: bool,
    pub state_cleared: bool,
    pub descriptive_only: bool,
    pub agent_bridge_command: bool,
    pub physical_actuation: bool,
    pub ncp_used: bool,
    pub pid_result: bool,
    /// False because the child receives no external durable-evidence bundle.
    pub source_durable_evidence_verified: bool,
    pub scientific_authority: bool,
    pub is_paper_local_evidence: bool,
    pub calibrated_posterior: bool,
}
