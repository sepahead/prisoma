use ncp_local::local::{LocalBinding, LocalRequest, LocalResponse};
use ncp_local::local_data::{PrepareData, Snapshot};
use serde::{Deserialize, Serialize};

/// Exact installed capture profile.
pub const APPLICATION_PROFILE: &str = "prisoma.local-causal-capture.v1";
/// Hard quota ceiling: 128 MiB.
pub const MAX_CAPTURE_BYTES: u64 = 128 * 1024 * 1024;
/// Total reserved journal bytes per complete logical step.
pub const STEP_BYTES: u64 = 65_536;
/// Journal allowance for a pre-execution reservation.
pub const RESERVE_BYTES: u64 = 1024;
/// Journal allowance for the complete captured exchanges.
pub const CAPTURE_BYTES: u64 = STEP_BYTES - RESERVE_BYTES;
/// Journal allowance for either terminal record.
pub const TERMINAL_BYTES: u64 = 16_384;

/// Original request and exact retained producer response.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Exchange {
    pub request: LocalRequest,
    pub response: LocalResponse,
}

/// Prepared peer identities, plans, initial state, and fixed capture quota.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptureConfiguration {
    pub body_preparation: Exchange,
    pub neural_preparation: Exchange,
    pub monitor_preparation: Exchange,
    pub max_capture_bytes: u64,
}

/// Reserve one exact next logical step before producer advancement.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReserveData {
    pub plan_digest: String,
    pub step: u64,
}

/// Complete coupled step and its producer-owned monitor assessment.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptureData {
    pub plan_digest: String,
    pub step: u64,
    pub source_snapshot: Snapshot,
    pub neural: Exchange,
    pub body: Exchange,
    pub monitor: Exchange,
}

/// Full planned completion with all three producer terminal exchanges.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FinishData {
    pub plan_digest: String,
    pub completed_steps: u64,
    pub neural_finish: Exchange,
    pub body_finish: Exchange,
    pub monitor_finish: Exchange,
}

/// Immutable first journal record, with no external file path.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub binding: LocalBinding,
    pub preparation: PrepareData,
}

/// Terminal abort describes its known prefix without resolving the suffix.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AbortData {
    pub plan_digest: String,
    pub completed_steps: u64,
    pub reserved_step: Option<u64>,
    pub remaining_suffix: String,
}

/// Closed journal grammar. Every payload preserves original producer content.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Event {
    Header(Box<Header>),
    Reserve(ReserveData),
    Capture(Box<CaptureData>),
    Finish(Box<FinishData>),
    Abort(AbortData),
}

/// Nonrecursive content-bound journal row.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JournalRecord {
    pub schema: String,
    pub ordinal: u64,
    pub previous_digest: Option<String>,
    pub event: Event,
    pub record_digest: String,
}

/// Descriptive local capture completion, independent of scientific validity.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    pub schema: String,
    pub plan_digest: String,
    pub planned_steps: u64,
    pub captured_steps: u64,
    pub store_completion: String,
    pub execution_plan_complete: bool,
    pub remaining_suffix: Option<String>,
    pub journal_digest: String,
    pub journal_bytes: u64,
    pub scientific_validation: bool,
}
