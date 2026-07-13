//! Deterministic offline fault observatory for pinned NCP wire fixtures.
//!
//! This module replays bounded exact-byte traces through the same callback
//! pre-admission and raw decoder used by the live observer. It deliberately
//! models logical delivery slots, not elapsed time or a network. Its reports
//! separate observer-native signals from the fixture manifest's oracle, so a
//! wholly omitted tick is never credited as a native detection.

use super::{
    atomic_write_with, classify_callback_receipt, ingest_wire_frame, publication_receipt_path,
    read_bounded, read_bounded_regular_snapshot, serialize_json_pretty_bounded,
    strict_json_preflight, sync_installed_file, BoundedBuffer, CallbackAdmission, IngressPlane,
    IngressRoutes, Mapping, Observer, ObserverLimits, ObserverStats, OfflineVldaDataset,
    OfflineVldaPublicationReceipt, OfflineVldaSample, RawIngressCounters, RawIngressDisposition,
    INGRESS_HANDOFF_CAPACITY, MAX_PUBLICATION_RECEIPT_BYTES, NCP_RELEASE_REVISION,
};
use anyhow::Context as _;
use ncp_core::keys::{valid_id_segment, Keys};
use ncp_core::{
    decode_validated, ChannelValue, CommandFrame, Map, Observation, ObservationFrame, SensorFrame,
    SessionRef, StreamPosition, CONTRACT_HASH, NCP_VERSION,
};
use pid_runlog::{
    logical_trace_hash_v3, validate_events, RunLogEvent, RunLogWriter, RunStatus,
    RUN_LOG_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

const TRACE_SCHEMA_VERSION: u32 = 1;
const REPORT_SCHEMA_VERSION: u32 = 1;
const PUBLICATION_SCHEMA_VERSION: u32 = 1;
const OUTCOME_FINGERPRINT_REVISION: &str = "ncp_observatory_outcome_v2";
const DATASET_PROJECTION_REVISION: &str = "ncp_vlda_semantic_projection_v2";
const SCIENTIFIC_PAYLOAD_PROJECTION_REVISION: &str = "ncp_vlda_samples_v2";
const SAMPLE_VALUE_PROJECTION_REVISION: &str = "ncp_vlda_sample_value_without_stream_identity_v2";
const RUNLOG_PROJECTION_REVISION: &str = "ncp_runlog_publication_normalized_v2";
const TYPED_RECEIPT_PROJECTION_REVISION: &str = "ncp_typed_receipt_v2";
const TRACE_SCOPE: &str = "deterministic_offline_ncp_wire_observatory";
const NCP_TAG: &str = "v0.8.0";
const SYNTHETIC_SESSION: &str = "observatory";
const SYNTHETIC_REALM: &str = "engram/ncp";
const EPOCH_A: &str = "00000000-0000-4000-8000-0000000000a1";
const EPOCH_B: &str = "00000000-0000-4000-8000-0000000000b2";
const GENERATION_A: &str = "00000000-0000-4000-8000-0000000000c3";
const GENERATION_B: &str = "00000000-0000-4000-8000-0000000000d4";
const BASELINE_TICKS: usize = 12;
// Reviewed hashes of the identity-normalized samples specified by the v1 fixture:
// V=[seq,1], L=[0.25], D=[seq/10], A=[seq/100], success=true,
// episode_id="synthetic-observatory", and provenance source/channel/source.
// Keeping these literal makes the content oracle independent of Observer output;
// a common decoder, channel-mapping, or join regression cannot redefine "clean".
const GOLDEN_CLEAN_SAMPLE_VALUE_HASHES: [&str; BASELINE_TICKS] = [
    "f63a1288f795428207aab4474c887fba94ec9459a18ea1d7fbad844b2ab7588b",
    "78043a3847ccc5e5972660a96f4c03abf696a27d668ea3427d045c1c8ed56a7c",
    "83b2c66b462f061e32f67fceff16f77f59263bf9660bb8be43c75d713886fefc",
    "7a8a1d582041cb8b60d476e8f7bbab05f05e5917628604a88eeb5693b673ab69",
    "8c40a3a203a2483ddf91554cced1cdebba1b2de9fc0c4acbb1c5acee965f8f3a",
    "46e8b9535268ca387d4ad16f349a2461202107e74dcb3f1c3e65654250020d97",
    "5cc652b7c7f1deebc16d614929d6364029216a4680988c5ce2dedb1523fb930e",
    "42b3c6b6940bb9bacf51f046e8a16df168266709aed4d2ee8cf7ffce53855166",
    "7bfff323e314daea031aebb313cf1c54c72582af5e4a9a83c110ad15d7af7e47",
    "c4f069c5ce9b765144346d8cae8fd156f5d683b7982923b81a7ee38c6d6d0d4c",
    "d64a5611e8503166d9741b8b30d9a7eae98c19dbac9a2c240bf9859e0829f7e1",
    "3a9477170f77f0bdc1bfdbca45707cc3e04dd3d725eeada028a1dbd7e5e08f3c",
];

/// Conservative in-memory limits for a fault-observatory run.
///
/// These are software ceilings, not performance claims or recommended trace
/// sizes. Larger traces need a bounded spool/index design rather than larger
/// unexamined allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservatoryLimits {
    pub max_trace_file_bytes: usize,
    pub max_trace_receipts: usize,
    pub max_trace_payload_bytes: usize,
    pub max_scenarios: usize,
    pub max_scheduled_deliveries: usize,
    pub max_scheduled_payload_bytes: usize,
    pub max_total_journal_records: usize,
    pub max_total_journal_projection_bytes: usize,
    pub max_compiled_schedule_bytes: usize,
    pub max_replay_outcome_bytes: usize,
    pub max_report_bytes: usize,
    pub max_outer_runlog_bytes: usize,
}

impl Default for ObservatoryLimits {
    fn default() -> Self {
        Self {
            max_trace_file_bytes: 16 * 1024 * 1024,
            max_trace_receipts: 100_000,
            max_trace_payload_bytes: 64 * 1024 * 1024,
            max_scenarios: 32,
            max_scheduled_deliveries: 1_000_000,
            max_scheduled_payload_bytes: 128 * 1024 * 1024,
            max_total_journal_records: 10_000,
            max_total_journal_projection_bytes: 8 * 1024 * 1024,
            max_compiled_schedule_bytes: 16 * 1024 * 1024,
            max_replay_outcome_bytes: 8 * 1024 * 1024,
            max_report_bytes: 16 * 1024 * 1024,
            max_outer_runlog_bytes: 64 * 1024 * 1024,
        }
    }
}

impl ObservatoryLimits {
    fn validate(self) -> anyhow::Result<Self> {
        if self.max_trace_file_bytes == 0
            || self.max_trace_receipts == 0
            || self.max_trace_payload_bytes == 0
            || self.max_scenarios == 0
            || self.max_scheduled_deliveries == 0
            || self.max_scheduled_payload_bytes == 0
            || self.max_total_journal_records == 0
            || self.max_total_journal_projection_bytes == 0
            || self.max_compiled_schedule_bytes == 0
            || self.max_replay_outcome_bytes == 0
            || self.max_report_bytes == 0
            || self.max_outer_runlog_bytes == 0
        {
            anyhow::bail!("observatory resource limits must all be positive");
        }
        Ok(self)
    }
}

/// Runtime provenance for the Prisoma consumer executing the fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerProvenance {
    pub revision: String,
    pub worktree_clean: Option<bool>,
    pub lockfile_sha256: Option<String>,
    pub executable_sha256: Option<String>,
    pub build_revision: Option<String>,
    pub build_worktree_clean: Option<bool>,
}

impl ConsumerProvenance {
    /// Construct bounded consumer provenance. Unknown revisions are represented
    /// explicitly as `not_recorded`, never as an empty or fabricated hash.
    pub fn new(revision: impl Into<String>, worktree_clean: Option<bool>) -> anyhow::Result<Self> {
        Self::with_execution(revision, worktree_clean, None, None)
    }

    /// Construct provenance with an optional exact standalone-crate lockfile hash.
    pub fn with_lockfile(
        revision: impl Into<String>,
        worktree_clean: Option<bool>,
        lockfile_sha256: Option<String>,
    ) -> anyhow::Result<Self> {
        Self::with_execution(revision, worktree_clean, lockfile_sha256, None)
    }

    /// Construct provenance with source, lockfile, and exact executable identities.
    pub fn with_execution(
        revision: impl Into<String>,
        worktree_clean: Option<bool>,
        lockfile_sha256: Option<String>,
        executable_sha256: Option<String>,
    ) -> anyhow::Result<Self> {
        Self::with_build_attestation(
            revision,
            worktree_clean,
            lockfile_sha256,
            executable_sha256,
            None,
            None,
        )
    }

    /// Construct provenance including the revision/cleanliness embedded by this
    /// crate's build script. This is a reproducibility binding, not a signature
    /// or remote attestation.
    pub fn with_build_attestation(
        revision: impl Into<String>,
        worktree_clean: Option<bool>,
        lockfile_sha256: Option<String>,
        executable_sha256: Option<String>,
        build_revision: Option<String>,
        build_worktree_clean: Option<bool>,
    ) -> anyhow::Result<Self> {
        let revision = revision.into();
        if revision.is_empty() || revision.len() > 256 {
            anyhow::bail!("consumer revision must be a non-empty bounded string");
        }
        if lockfile_sha256
            .as_deref()
            .is_some_and(|hash| !is_lower_hex(hash, 32))
        {
            anyhow::bail!("consumer lockfile hash must be lowercase SHA-256");
        }
        if executable_sha256
            .as_deref()
            .is_some_and(|hash| !is_lower_hex(hash, 32))
        {
            anyhow::bail!("consumer executable hash must be lowercase SHA-256");
        }
        if build_revision
            .as_deref()
            .is_some_and(|revision| revision.is_empty() || revision.len() > 256)
        {
            anyhow::bail!("consumer build revision must be a non-empty bounded string");
        }
        Ok(Self {
            revision,
            worktree_clean,
            lockfile_sha256,
            executable_sha256,
            build_revision,
            build_worktree_clean,
        })
    }

    fn qualifies_reproducible_fixture_evidence(&self) -> bool {
        (is_lower_hex(&self.revision, 20) || is_lower_hex(&self.revision, 32))
            && self.worktree_clean == Some(true)
            && self.lockfile_sha256.is_some()
            && self.executable_sha256.is_some()
            && self.build_revision.as_deref() == Some(self.revision.as_str())
            && self.build_worktree_clean == Some(true)
    }

    fn evidence_level(&self) -> EvidenceLevel {
        if self.qualifies_reproducible_fixture_evidence() {
            EvidenceLevel::FixtureSpecificE3StyleLocalEvidenceOnly
        } else {
            EvidenceLevel::FixtureSpecificLocalExecutionReproducibilityUnqualified
        }
    }
}

/// Declared source of an input trace. This is provenance supplied by the trace,
/// not an authenticated identity assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceOrigin {
    SyntheticFixture,
    ExternallyRecorded,
}

/// Transport/configuration condition attached to a trace or scenario.
/// Offline replay does not exercise any of these transports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportCondition {
    OfflineNoTransport,
    OpenConfigurationDeclared,
    SecureConfigurationDeclared,
}

impl TransportCondition {
    fn observer_label(self) -> &'static str {
        match self {
            Self::OfflineNoTransport => "offline synthetic replay; no transport",
            Self::OpenConfigurationDeclared => {
                "open/unauthenticated configuration (declared-only offline condition)"
            }
            Self::SecureConfigurationDeclared => {
                "secure/fail-closed client config (declared-only offline condition)"
            }
        }
    }
}

/// Explicit terminal boundary for a trace or transformed schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceEndReason {
    ProducerClose,
    TraceTruncation,
}

/// Exact route binding embedded in a trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceRoutes {
    pub sensor: String,
    pub command: String,
    pub observation: String,
}

/// Frozen channel mapping used by every replay observer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceMapping {
    pub language_channel: String,
    pub success_channel: Option<String>,
    pub episode_id: Option<String>,
}

impl TraceRoutes {
    fn ingress_routes(&self) -> anyhow::Result<IngressRoutes> {
        IngressRoutes::new(
            self.sensor.clone(),
            self.command.clone(),
            self.observation.clone(),
        )
    }

    fn for_plane(&self, plane: IngressPlane) -> &str {
        match plane {
            IngressPlane::Sensor => &self.sensor,
            IngressPlane::Command => &self.command,
            IngressPlane::Observation => &self.observation,
        }
    }
}

/// One exact raw receipt in a complete, conforming baseline trace.
/// `payload_hex` retains arbitrary bytes without JSON normalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceReceipt {
    pub ordinal: u64,
    pub logical_slot: u64,
    pub routing_key: String,
    pub expected_plane: IngressPlane,
    pub source_epoch: String,
    pub source_seq: i64,
    pub payload_hex: String,
    pub payload_sha256: String,
    pub typed_receipt_sha256: String,
}

/// Versioned exact-byte baseline trace. Faults are applied by a compiled
/// scenario registry; the input itself must be complete and conforming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireTrace {
    pub schema_version: u32,
    pub scope: String,
    pub origin: TraceOrigin,
    pub producer_revision: String,
    pub ncp_tag: String,
    pub ncp_revision: String,
    pub ncp_wire: String,
    pub ncp_contract_hash: String,
    pub realm: String,
    pub session_id: String,
    pub transport_condition: TransportCondition,
    pub routes: TraceRoutes,
    pub mapping: TraceMapping,
    pub observer_limits: ObserverLimits,
    pub terminal: TraceEndReason,
    pub receipts: Vec<TraceReceipt>,
}

/// Stable scenario registry for the v1 synthetic suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultScenario {
    CleanBaseline,
    ExactRedelivery,
    ConflictingDuplicate,
    ConflictingDuplicateAfterClosure,
    OnePlaneOmission,
    WholeTickOmission,
    ReorderWithinGrace,
    LogicalReceiptPause,
    ObservationAfterGrace,
    VersionMismatch,
    DuplicateJsonKey,
    MalformedNonUtf8,
    TraceTruncation,
    NewStreamEpoch,
    IdentityCollision,
    RouteMismatch,
    OversizedPayload,
    SecurityProfileClaimGuard,
}

impl FaultScenario {
    /// Frozen execution order for schema v1.
    pub const ALL: [Self; 18] = [
        Self::CleanBaseline,
        Self::ExactRedelivery,
        Self::ConflictingDuplicate,
        Self::ConflictingDuplicateAfterClosure,
        Self::OnePlaneOmission,
        Self::WholeTickOmission,
        Self::ReorderWithinGrace,
        Self::LogicalReceiptPause,
        Self::ObservationAfterGrace,
        Self::VersionMismatch,
        Self::DuplicateJsonKey,
        Self::MalformedNonUtf8,
        Self::TraceTruncation,
        Self::NewStreamEpoch,
        Self::IdentityCollision,
        Self::RouteMismatch,
        Self::OversizedPayload,
        Self::SecurityProfileClaimGuard,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::CleanBaseline => "clean_baseline",
            Self::ExactRedelivery => "exact_redelivery",
            Self::ConflictingDuplicate => "conflicting_duplicate",
            Self::ConflictingDuplicateAfterClosure => "conflicting_duplicate_after_closure",
            Self::OnePlaneOmission => "one_plane_omission",
            Self::WholeTickOmission => "whole_tick_omission",
            Self::ReorderWithinGrace => "reorder_within_grace",
            Self::LogicalReceiptPause => "logical_receipt_pause",
            Self::ObservationAfterGrace => "observation_after_grace",
            Self::VersionMismatch => "version_mismatch",
            Self::DuplicateJsonKey => "duplicate_json_key",
            Self::MalformedNonUtf8 => "malformed_non_utf8",
            Self::TraceTruncation => "trace_truncation",
            Self::NewStreamEpoch => "new_stream_epoch",
            Self::IdentityCollision => "identity_collision",
            Self::RouteMismatch => "route_mismatch",
            Self::OversizedPayload => "oversized_payload",
            Self::SecurityProfileClaimGuard => "security_profile_claim_guard",
        }
    }

    /// Hand-reviewed v1 hash for the built-in raw fixture and compiled plan.
    pub fn golden_schedule_sha256(self) -> &'static str {
        match self {
            Self::CleanBaseline => {
                "1dd4f3118ad1f88d0e8b5812d0a44f2df6421c0dd8506843979cf6728f66a4ed"
            }
            Self::ExactRedelivery => {
                "fedef518181e48d0e06cee9e28ecfae7b99ff7e96fc50d01b4a1ec508254efd5"
            }
            Self::ConflictingDuplicate => {
                "0a8a3aaae252b34a32f253b19d2ecdc3314b435ff4475280a902208bc85df902"
            }
            Self::ConflictingDuplicateAfterClosure => {
                "869af25893633b4b309050a602cbab5584d442bd04a11416ceab48b56608d171"
            }
            Self::OnePlaneOmission => {
                "cb2b59ee88dc25077936e7d9f9e6285fa74cd3e488284de4d496a1f0cf98531f"
            }
            Self::WholeTickOmission => {
                "0545176b1a3c93680053403f1bfdba0479659290213526bb88e4f95b81ea2637"
            }
            Self::ReorderWithinGrace => {
                "b03eeead448885973892aa261d99a4dc6ab6a36d16fdf31065a0b7d3e3259be1"
            }
            Self::LogicalReceiptPause => {
                "d6c4149c94b2bf446e9bbe373c43815452eeb3652f3fd25637cda9f9db6b33f2"
            }
            Self::ObservationAfterGrace => {
                "fe2244c983f3e9ef6fa882471c4e8bf47c92d088e952488da26eca812e9665b7"
            }
            Self::VersionMismatch => {
                "d216dcc9c0e105f07990a5c5e7c32ded247fa7c981aedf297fe3d19478deabb1"
            }
            Self::DuplicateJsonKey => {
                "fec2f45036321dfb51b68e96238b847380487c60c3e6f410523d238e03fb6afb"
            }
            Self::MalformedNonUtf8 => {
                "aaa40344fa2db897fb5598f945eeef32f31f416abe14549bb9cb62a6c6c74b78"
            }
            Self::TraceTruncation => {
                "5f7f427a322fc5fdf1e85ed7d924e6c75484fb93b1631abc4e5dc2ef42337a83"
            }
            Self::NewStreamEpoch => {
                "89c2780ece9ad77c67baa4f7051d01d34dd27bcc19907f5772691495ff33d8c3"
            }
            Self::IdentityCollision => {
                "bde887ae2f691fcb4cf8c6b80a98e56bc30d153ed569f2c1df5897b64dea7cb4"
            }
            Self::RouteMismatch => {
                "8b06d0d7ff2f74743123235dadbffd36fc35b57894ac1465dcab36cbcaee8e3a"
            }
            Self::OversizedPayload => {
                "a24849ce984fd9e2f80eb8a7391fa515605229b7b707edefd599135cd72312d4"
            }
            Self::SecurityProfileClaimGuard => {
                "b9b961dfdedba8fcfee1dc3bb701ceef69e8c40c09927e537cc6d34a0b9a3fc3"
            }
        }
    }
}

/// Whether a fault is observable by the current observer without consulting the
/// injector's baseline manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservabilityClass {
    VisibleReceipt,
    MixedVisibleAndManifestOnly,
    ManifestOnly,
    LiveTransportOnly,
    NotAssessableOffline,
    NotApplicable,
}

/// Native observer response, kept separate from the fixture oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeObserverResponse {
    Tolerated,
    DetectedRejected,
    DetectedDegraded,
    NotDetected,
    NotIdentifiable,
    NotAssessable,
    NotApplicable,
}

/// Overall comparison against the frozen fixture expectation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioVerdict {
    Matched,
    MatchedKnownLimitation,
    Mismatched,
    NotAssessable,
}

/// Machine-readable evidence scope for the complete suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    FixtureSpecificE3StyleLocalEvidenceOnly,
    FixtureSpecificLocalExecutionReproducibilityUnqualified,
}

/// Machine-readable execution state for a published report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Completed,
}

/// Typed nonclaim used instead of ambiguous free-form pass/fail strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    NotAssessed,
    NotEstablished,
}

/// Pre-admission plus worker result for one scheduled delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptDisposition {
    Applied,
    DecodeDropped,
    UnstampedObservationDropped,
    WorkerOversizedDropped,
    CapacityDropped,
    CallbackRouteMismatchDropped,
    CallbackOversizedDropped,
    WorkerError,
}

impl From<RawIngressDisposition> for ReceiptDisposition {
    fn from(value: RawIngressDisposition) -> Self {
        match value {
            RawIngressDisposition::Applied => Self::Applied,
            RawIngressDisposition::DecodeDropped => Self::DecodeDropped,
            RawIngressDisposition::UnstampedObservationDropped => Self::UnstampedObservationDropped,
            RawIngressDisposition::OversizedDropped => Self::WorkerOversizedDropped,
            RawIngressDisposition::CapacityDropped => Self::CapacityDropped,
        }
    }
}

/// Stable reason code for a per-receipt or finalization-induced stats delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObserverSignalCode {
    KeptSample,
    ZeroSampleCapture,
    ExcludedEmptyV,
    ExcludedEmptyL,
    ExcludedEmptyD,
    ExcludedEmptyA,
    DimMismatch,
    LateD,
    UnstampedFrame,
    SessionMismatch,
    RetiredEpoch,
    IngressDecode,
    IngressUnstampedObservation,
    IngressHandoff,
    Redelivery,
    ConflictingDuplicate,
    QuarantinedFrame,
    InvalidPayload,
    EvictedIncomplete,
    EvictedUnclaimedD,
    IncompleteEpochTransition,
    IncompleteFinalize,
    MissingSensor,
    MissingCommand,
    UnclaimedDEpochTransition,
    UnclaimedDFinalize,
    IngressOversized,
    RouteMismatch,
    IngressLifetimeLimit,
    CaptureCapacityDropped,
    CaptureCapacityReached,
    SampleLimit,
    ElementLimit,
    InflightElementLimit,
    EpochLimit,
    WorkerFailure,
    TeardownFailure,
    EpochTransition,
}

/// One nonzero monotonic counter delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverSignalDelta {
    pub code: ObserverSignalCode,
    pub amount: u64,
}

/// Hash-only delivery journal entry. Raw bytes stay in the bounded trace or
/// compiled schedule and are never copied into the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryRecord {
    pub delivery_ordinal: u64,
    pub source_ordinal: u64,
    pub logical_slot: u64,
    pub routing_key: String,
    pub expected_plane: IngressPlane,
    pub payload_sha256: String,
    pub typed_receipt_sha256: Option<String>,
    pub fault_ids: Vec<String>,
    pub disposition: ReceiptDisposition,
    pub observer_deltas: Vec<ObserverSignalDelta>,
    pub sample_count_delta: u64,
}

/// Exact portable delivery entry persisted for independent replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledScheduleReceipt {
    pub delivery_ordinal: u64,
    pub source_ordinal: u64,
    pub logical_slot: u64,
    pub routing_key: String,
    pub expected_plane: IngressPlane,
    pub source_epoch: String,
    pub source_seq: i64,
    pub payload_hex: String,
    pub payload_sha256: String,
    pub typed_receipt_sha256: Option<String>,
    pub fault_ids: Vec<String>,
}

/// Strict exact-byte compiled schedule bound into the outer evidence bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledScheduleArtifact {
    pub schema_version: u32,
    pub scope: String,
    pub trace_exact_sha256: String,
    pub scenario: FaultScenario,
    pub schedule_sha256: String,
    pub injection_truth: InjectionTruth,
    pub receipts: Vec<CompiledScheduleReceipt>,
}

/// Exact file identity inside the committed observatory directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactIdentity {
    pub relative_uri: String,
    pub sha256: String,
    pub bytes: u64,
}

/// The fixture oracle's exact finite denominators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InjectionTruth {
    pub baseline_receipts: usize,
    pub baseline_eligible_sample_ids: Vec<String>,
    pub dropped_source_ordinals: Vec<u64>,
    pub native_visible_dropped_source_ordinals: Vec<u64>,
    pub manifest_only_dropped_source_ordinals: Vec<u64>,
    pub duplicated_source_ordinals: Vec<u64>,
    pub modified_source_ordinals: Vec<u64>,
    pub reordered: bool,
    pub logical_displacement_slots: u64,
    pub terminal: TraceEndReason,
    pub transport_condition: TransportCondition,
    pub logical_slots_are_annotations_only: bool,
}

/// Observer-native detection assessment; the oracle is never counted as a
/// native signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetectionAssessment {
    pub observability: ObservabilityClass,
    pub native_response: NativeObserverResponse,
    pub expected_native_response: NativeObserverResponse,
    pub native_signal_codes: Vec<ObserverSignalCode>,
    pub expected_native_signal_codes: Vec<ObserverSignalCode>,
    pub native_signal_deltas: Vec<ObserverSignalDelta>,
    pub expected_native_signal_deltas: Vec<ObserverSignalDelta>,
    pub fixture_oracle_knows_injection: bool,
    pub wall_clock_latency: AssessmentStatus,
}

/// Path-independent comparison of two independent replay publications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayEquivalence {
    pub equal: bool,
    pub outcome_fingerprint_revision: String,
    pub first_outcome_fingerprint: String,
    pub second_outcome_fingerprint: String,
    pub dataset_projection_revision: String,
    pub first_dataset_semantic_hash: String,
    pub second_dataset_semantic_hash: String,
    pub scientific_payload_projection_revision: String,
    pub first_scientific_payload_hash: String,
    pub second_scientific_payload_hash: String,
    pub runlog_projection_revision: String,
    pub first_normalized_logical_trace_hash_v3: String,
    pub second_normalized_logical_trace_hash_v3: String,
    pub publication_byte_identity_expected: bool,
}

/// Cross-scenario scientific payload relation frozen before replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScientificPayloadExpectation {
    SameAsCleanBaseline,
    ScenarioSpecific,
}

/// Explicit finite sample-set accounting. These counts are not a sampled fault
/// population and support no generalized detection-rate claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleSetAccounting {
    pub baseline_eligible_ids: Vec<String>,
    pub expected_retained_ids: Vec<String>,
    pub actual_retained_ids: Vec<String>,
    pub unexpected_missing_ids: Vec<String>,
    pub unexpected_extra_ids: Vec<String>,
    pub duplicate_output_ids: Vec<String>,
}

/// Content oracle over retained samples, excluding only identity fields that a
/// deliberate stream-epoch transition is expected to change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleContentAccounting {
    pub projection_revision: String,
    pub expected_value_hashes: BTreeMap<String, String>,
    pub actual_value_hashes: BTreeMap<String, String>,
    pub mismatched_or_missing_ids: Vec<String>,
    pub unexpected_ids: Vec<String>,
    pub matches_clean_fixture_oracle: bool,
}

/// Required, absent, and unassessed provenance fields for this offline suite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceAssessment {
    pub required_field_count: usize,
    pub recorded_fields: Vec<String>,
    pub missing_fields: Vec<String>,
    pub explicitly_unassessed_fields: Vec<String>,
}

/// Structural authority/noninterference scope. This is not a live timing result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlScopeAssessment {
    pub execution_scope: String,
    pub action_plane_publications: u64,
    pub agent_bridge_requests: u64,
    pub live_control_timing_noninterference: AssessmentStatus,
}

/// Offline security scope. Configuration labels never become authentication
/// evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityAssessment {
    pub transport_exercised: bool,
    pub configuration_condition: TransportCondition,
    pub configuration_loaded: bool,
    pub configuration_selected: bool,
    pub declared_profile_label_only: bool,
    pub peer_authentication: AssessmentStatus,
    pub acl_enforcement: AssessmentStatus,
    pub security_validation: AssessmentStatus,
}

/// One complete scenario result and its two durable replay bundles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioReport {
    pub scenario: FaultScenario,
    pub schedule_hash: String,
    pub builtin_golden_schedule_hash: String,
    pub matches_builtin_golden_schedule: bool,
    pub injection_truth: InjectionTruth,
    pub delivery_journal: Vec<DeliveryRecord>,
    pub finalization_deltas: Vec<ObserverSignalDelta>,
    pub observer_stats: ObserverStats,
    pub raw_ingress_counters: RawIngressCounters,
    pub observer_integrity: String,
    pub detection: DetectionAssessment,
    pub sample_sets: SampleSetAccounting,
    pub sample_content: SampleContentAccounting,
    pub replay_equivalence: ReplayEquivalence,
    pub scientific_payload_expectation: ScientificPayloadExpectation,
    pub scientific_payload_matches_clean_baseline: Option<bool>,
    pub compiled_schedule_artifact: ArtifactIdentity,
    pub replay_artifacts: Vec<ArtifactIdentity>,
    pub security: SecurityAssessment,
    pub control_scope: ControlScopeAssessment,
    pub expectation_failures: Vec<String>,
    pub verdict: ScenarioVerdict,
}

/// Exact finite scenario accounting. `all_expectations_matched` includes
/// expected `not_assessable` claim guards, so these counts stay explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioAssessmentSummary {
    pub total: usize,
    pub assessed: usize,
    pub not_assessable: usize,
    pub matched: usize,
    pub matched_known_limitations: usize,
    pub mismatched: usize,
}

/// Top-level deterministic fixture report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservatoryReport {
    pub schema_version: u32,
    pub scope: String,
    pub evidence_level: EvidenceLevel,
    pub execution_status: ExecutionStatus,
    pub establishes_e4: bool,
    pub completes_ec1: bool,
    pub establishes_live_engram_validation: bool,
    pub establishes_security_validation: bool,
    pub changes_pid_gates: bool,
    pub consumer: ConsumerProvenance,
    pub ncp_tag: String,
    pub ncp_revision: String,
    pub ncp_wire: String,
    pub ncp_contract_hash: String,
    pub trace_exact_sha256: String,
    pub trace_canonical_sha256: String,
    pub trace_artifact: ArtifactIdentity,
    pub limits: ObservatoryLimits,
    pub observer_limits: ObserverLimits,
    pub provenance: ProvenanceAssessment,
    pub scenarios: Vec<ScenarioReport>,
    pub scenario_assessments: ScenarioAssessmentSummary,
    pub all_expectations_matched: bool,
    pub canonical_runlog_relative_uri: String,
    pub publication_receipt_relative_uri: String,
    pub limitations: Vec<String>,
}

/// Receipt installed last for the outer report/run-log/trace bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservatoryPublicationReceipt {
    pub schema_version: u32,
    pub committed: bool,
    pub report_uri: String,
    pub report_sha256: String,
    pub runlog_uri: String,
    pub runlog_sha256: String,
    pub trace_uri: String,
    pub trace_sha256: String,
    pub scenario_artifact_manifest_sha256: String,
    pub all_expectations_matched: bool,
}

/// Paths and final expectation status returned by [`run_observatory`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservatoryOutcome {
    pub report_path: PathBuf,
    pub runlog_path: PathBuf,
    pub receipt_path: PathBuf,
    pub all_expectations_matched: bool,
}

#[derive(Debug, Clone)]
struct BaselineReceipt {
    ordinal: u64,
    logical_slot: u64,
    routing_key: String,
    expected_plane: IngressPlane,
    source_epoch: String,
    source_seq: i64,
    payload: Vec<u8>,
}

#[derive(Debug, Clone)]
struct ValidatedTrace {
    trace: WireTrace,
    exact_bytes: Vec<u8>,
    exact_sha256: String,
    canonical_sha256: String,
    receipts: Vec<BaselineReceipt>,
    baseline_sample_ids: Vec<String>,
    matches_builtin_raw_receipts: bool,
}

#[derive(Debug, Clone)]
struct ScheduledReceipt {
    source_ordinal: u64,
    logical_slot: u64,
    routing_key: String,
    expected_plane: IngressPlane,
    source_epoch: String,
    source_seq: i64,
    payload: Vec<u8>,
    fault_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct ScheduledScenario {
    scenario: FaultScenario,
    receipts: Vec<ScheduledReceipt>,
    truth: InjectionTruth,
    schedule_hash: String,
    matches_builtin_golden_schedule: bool,
}

/// Persisted per-replay outcome evidence. The outer report summarizes replay A;
/// this record keeps both executions independently reconstructable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayOutcomeRecord {
    schema_version: u32,
    scope: String,
    scenario: FaultScenario,
    schedule_sha256: String,
    replay: String,
    stats: ObserverStats,
    counters: RawIngressCounters,
    journal: Vec<DeliveryRecord>,
    finalization_deltas: Vec<ObserverSignalDelta>,
    sample_ids: Vec<String>,
    sample_value_hashes: BTreeMap<String, String>,
    dataset_semantic_hash: String,
    scientific_payload_hash: String,
    normalized_logical_trace_hash_v3: String,
    outcome_fingerprint: String,
    runlog_validation_errors: usize,
    runlog_validation_warnings: usize,
}

#[derive(Debug)]
struct ReplayRun {
    stats: ObserverStats,
    counters: RawIngressCounters,
    journal: Vec<DeliveryRecord>,
    finalization_deltas: Vec<ObserverSignalDelta>,
    sample_ids: Vec<String>,
    sample_value_hashes: BTreeMap<String, String>,
    dataset_semantic_hash: String,
    scientific_payload_hash: String,
    normalized_logical_trace_hash_v3: String,
    outcome_fingerprint: String,
    runlog_validation_errors: usize,
    runlog_validation_warnings: usize,
    artifacts: Vec<ArtifactIdentity>,
}

fn checked_add_usize(left: usize, right: usize, what: &str) -> anyhow::Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| anyhow::anyhow!("{what} overflow"))
}

fn is_lower_hex(value: &str, bytes: usize) -> bool {
    value.len() == bytes.saturating_mul(2)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn hex_decode(value: &str, max_bytes: usize) -> anyhow::Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) || value.len() / 2 > max_bytes {
        anyhow::bail!("hex payload has invalid or oversized length");
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(value.len() / 2)
        .context("failed to reserve bounded hex payload")?;
    for pair in value.as_bytes().chunks_exact(2) {
        let nibble = |byte: u8| -> Option<u8> {
            match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                _ => None,
            }
        };
        let high =
            nibble(pair[0]).ok_or_else(|| anyhow::anyhow!("payload hex is not lowercase"))?;
        let low = nibble(pair[1]).ok_or_else(|| anyhow::anyhow!("payload hex is not lowercase"))?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn bounded_regular_file(path: &Path, max_bytes: usize) -> anyhow::Result<Vec<u8>> {
    read_bounded_regular_snapshot(path, max_bytes)
        .with_context(|| format!("failed to read bounded trace snapshot {}", path.display()))
}

fn stream(epoch: &str, seq: i64) -> StreamPosition {
    StreamPosition {
        epoch: epoch.to_string(),
        seq,
    }
}

fn session(generation: &str) -> SessionRef {
    SessionRef {
        generation: generation.to_string(),
    }
}

fn channel(data: Vec<f64>) -> ChannelValue {
    ChannelValue { data, unit: None }
}

fn synthetic_sensor(seq: i64) -> SensorFrame {
    let mut channels = Map::new();
    channels.insert("instruction".to_string(), channel(vec![0.25]));
    channels.insert("pose".to_string(), channel(vec![seq as f64, 1.0]));
    channels.insert("success".to_string(), channel(vec![1.0]));
    SensorFrame {
        t: seq as f64,
        channels,
        stream: stream(EPOCH_A, seq),
        session: session(GENERATION_A),
        session_id: SYNTHETIC_SESSION.to_string(),
        ..Default::default()
    }
}

fn synthetic_command(seq: i64) -> CommandFrame {
    let mut channels = Map::new();
    channels.insert(
        "velocity_setpoint".to_string(),
        channel(vec![seq as f64 / 100.0]),
    );
    CommandFrame {
        t: seq as f64,
        channels,
        stream: stream(EPOCH_A, seq),
        source: Some(stream(EPOCH_A, seq)),
        source_t: seq as f64,
        session: session(GENERATION_A),
        session_id: SYNTHETIC_SESSION.to_string(),
        ..Default::default()
    }
}

fn synthetic_observation(seq: i64) -> ObservationFrame {
    let mut records = Map::new();
    records.insert(
        "rate".to_string(),
        Observation {
            port: "rate".to_string(),
            target: "population".to_string(),
            times: vec![seq as f64],
            values: vec![seq as f64 / 10.0],
            ..Default::default()
        },
    );
    ObservationFrame {
        t: seq as f64,
        source_t: seq as f64,
        records,
        stream: stream(EPOCH_A, seq),
        source: Some(stream(EPOCH_A, seq)),
        session: session(GENERATION_A),
        session_id: SYNTHETIC_SESSION.to_string(),
        ..Default::default()
    }
}

fn typed_receipt_hash(plane: IngressPlane, payload: &[u8]) -> anyhow::Result<Option<String>> {
    if !strict_json_preflight(payload) {
        return Ok(None);
    }
    let frame = match plane {
        IngressPlane::Sensor => decode_validated::<SensorFrame>(payload)
            .ok()
            .and_then(|frame| serde_json::to_value(frame).ok()),
        IngressPlane::Command => decode_validated::<CommandFrame>(payload)
            .ok()
            .and_then(|frame| serde_json::to_value(frame).ok()),
        IngressPlane::Observation => decode_validated::<ObservationFrame>(payload)
            .ok()
            .and_then(|frame| serde_json::to_value(frame).ok()),
    };
    frame
        .map(|frame| {
            pid_runlog::canonical_json_hash_v2(&serde_json::json!({
                "projection_revision": TYPED_RECEIPT_PROJECTION_REVISION,
                "ncp_wire": NCP_VERSION,
                "ncp_contract_hash": CONTRACT_HASH,
                "plane": plane,
                "frame": frame,
            }))
            .context("failed to hash typed NCP receipt projection")
        })
        .transpose()
}

/// Construct the checked synthetic wire-0.8 baseline used when no external
/// trace is supplied. The producer revision is bound into the exact trace.
pub fn synthetic_wire_trace(producer_revision: impl Into<String>) -> anyhow::Result<WireTrace> {
    let producer_revision = producer_revision.into();
    if producer_revision.is_empty() || producer_revision.len() > 256 {
        anyhow::bail!("synthetic producer revision must be a non-empty bounded string");
    }
    let keys = Keys::try_new(SYNTHETIC_REALM.to_string())
        .map_err(|error| anyhow::anyhow!("invalid built-in NCP realm: {error}"))?;
    let routes = TraceRoutes {
        sensor: keys
            .try_sensor(SYNTHETIC_SESSION)
            .map_err(|error| anyhow::anyhow!("invalid built-in sensor route: {error}"))?,
        command: keys
            .try_command(SYNTHETIC_SESSION)
            .map_err(|error| anyhow::anyhow!("invalid built-in command route: {error}"))?,
        observation: keys
            .try_observation(SYNTHETIC_SESSION)
            .map_err(|error| anyhow::anyhow!("invalid built-in observation route: {error}"))?,
    };
    let mut receipts = Vec::with_capacity(BASELINE_TICKS * 3);
    for seq in 1..=i64::try_from(BASELINE_TICKS).unwrap_or(i64::MAX) {
        let frames = [
            (
                IngressPlane::Sensor,
                serde_json::to_vec(&synthetic_sensor(seq))?,
            ),
            (
                IngressPlane::Command,
                serde_json::to_vec(&synthetic_command(seq))?,
            ),
            (
                IngressPlane::Observation,
                serde_json::to_vec(&synthetic_observation(seq))?,
            ),
        ];
        for (plane, payload) in frames {
            let ordinal = u64::try_from(receipts.len())
                .map_err(|_| anyhow::anyhow!("synthetic trace ordinal overflow"))?;
            receipts.push(TraceReceipt {
                ordinal,
                logical_slot: ordinal,
                routing_key: routes.for_plane(plane).to_string(),
                expected_plane: plane,
                source_epoch: EPOCH_A.to_string(),
                source_seq: seq,
                payload_hex: hex_encode(&payload),
                payload_sha256: pid_runlog::sha256_hex(&payload),
                typed_receipt_sha256: typed_receipt_hash(plane, &payload)?
                    .ok_or_else(|| anyhow::anyhow!("synthetic typed frame did not validate"))?,
            });
        }
    }
    Ok(WireTrace {
        schema_version: TRACE_SCHEMA_VERSION,
        scope: TRACE_SCOPE.to_string(),
        origin: TraceOrigin::SyntheticFixture,
        producer_revision,
        ncp_tag: NCP_TAG.to_string(),
        ncp_revision: NCP_RELEASE_REVISION.to_string(),
        ncp_wire: NCP_VERSION.to_string(),
        ncp_contract_hash: CONTRACT_HASH.to_string(),
        realm: SYNTHETIC_REALM.to_string(),
        session_id: SYNTHETIC_SESSION.to_string(),
        transport_condition: TransportCondition::OfflineNoTransport,
        routes,
        mapping: TraceMapping {
            language_channel: "instruction".to_string(),
            success_channel: Some("success".to_string()),
            episode_id: Some("synthetic-observatory".to_string()),
        },
        observer_limits: ObserverLimits::default(),
        terminal: TraceEndReason::ProducerClose,
        receipts,
    })
}

fn validate_trace_bytes(
    exact_bytes: Vec<u8>,
    limits: ObservatoryLimits,
    observer_limits: ObserverLimits,
) -> anyhow::Result<ValidatedTrace> {
    if exact_bytes.len() > limits.max_trace_file_bytes {
        anyhow::bail!(
            "trace exceeds the {}-byte observatory ceiling",
            limits.max_trace_file_bytes
        );
    }
    if !strict_json_preflight(&exact_bytes) {
        anyhow::bail!("trace must be one strict JSON document without duplicate keys");
    }
    let trace: WireTrace =
        serde_json::from_slice(&exact_bytes).context("failed to decode wire-trace manifest")?;
    if trace.schema_version != TRACE_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported wire-trace schema {}, expected {}",
            trace.schema_version,
            TRACE_SCHEMA_VERSION
        );
    }
    if trace.scope != TRACE_SCOPE
        || trace.ncp_tag != NCP_TAG
        || trace.ncp_revision != NCP_RELEASE_REVISION
        || trace.ncp_wire != NCP_VERSION
        || trace.ncp_contract_hash != CONTRACT_HASH
    {
        anyhow::bail!("trace protocol identity does not match the pinned NCP wire-0.8 consumer");
    }
    if trace.producer_revision.is_empty() || trace.producer_revision.len() > 256 {
        anyhow::bail!("trace producer revision must be a non-empty bounded string");
    }
    if trace.observer_limits != observer_limits {
        anyhow::bail!("trace observer limits do not match the frozen replay limits");
    }
    if trace.mapping
        != (TraceMapping {
            language_channel: "instruction".to_string(),
            success_channel: Some("success".to_string()),
            episode_id: Some("synthetic-observatory".to_string()),
        })
    {
        anyhow::bail!("trace mapping does not match the frozen v1 observatory mapping");
    }
    if trace.session_id.is_empty()
        || trace.session_id.len() > 64
        || !valid_id_segment(&trace.session_id)
    {
        anyhow::bail!("trace session id is not a valid bounded NCP key segment");
    }
    if trace.realm.is_empty() || trace.realm.len() > 1024 {
        anyhow::bail!("trace realm must be a non-empty bounded string");
    }
    if trace.terminal != TraceEndReason::ProducerClose {
        anyhow::bail!("baseline trace must end with exactly one producer_close boundary");
    }
    let keys = Keys::try_new(trace.realm.clone())
        .map_err(|error| anyhow::anyhow!("trace realm is invalid: {error}"))?;
    let expected_routes = TraceRoutes {
        sensor: keys
            .try_sensor(&trace.session_id)
            .map_err(|error| anyhow::anyhow!("trace sensor route is invalid: {error}"))?,
        command: keys
            .try_command(&trace.session_id)
            .map_err(|error| anyhow::anyhow!("trace command route is invalid: {error}"))?,
        observation: keys
            .try_observation(&trace.session_id)
            .map_err(|error| anyhow::anyhow!("trace observation route is invalid: {error}"))?,
    };
    if trace.routes != expected_routes {
        anyhow::bail!("trace routes do not exactly match its realm/session identity");
    }
    let ingress_routes = trace.routes.ingress_routes()?;
    if trace.receipts.is_empty() || trace.receipts.len() > limits.max_trace_receipts {
        anyhow::bail!(
            "trace receipt count must be within 1..={} ",
            limits.max_trace_receipts
        );
    }

    let mut receipts = Vec::new();
    receipts
        .try_reserve_exact(trace.receipts.len())
        .context("failed to reserve bounded trace receipts")?;
    let mut payload_bytes = 0_usize;
    let mut previous_slot = None;
    let mut own_stream_last = BTreeMap::<(IngressPlane, String), i64>::new();
    let mut join_planes = BTreeMap::<(String, i64), BTreeSet<IngressPlane>>::new();
    for (index, receipt) in trace.receipts.iter().enumerate() {
        let expected_ordinal =
            u64::try_from(index).map_err(|_| anyhow::anyhow!("trace receipt ordinal overflow"))?;
        if receipt.ordinal != expected_ordinal {
            anyhow::bail!(
                "trace receipt ordinal {} is not contiguous at index {index}",
                receipt.ordinal
            );
        }
        if previous_slot.is_some_and(|slot| receipt.logical_slot < slot) {
            anyhow::bail!("baseline trace logical slots must be nondecreasing");
        }
        previous_slot = Some(receipt.logical_slot);
        if receipt.routing_key.is_empty() || receipt.routing_key.len() > 4096 {
            anyhow::bail!("trace routing key must be a non-empty bounded string");
        }
        if ingress_routes.classify(&receipt.routing_key) != Some(receipt.expected_plane) {
            anyhow::bail!(
                "trace route/plane metadata mismatch at ordinal {}",
                receipt.ordinal
            );
        }
        if receipt.source_epoch.is_empty()
            || receipt.source_epoch.len() > 64
            || receipt.source_seq < 1
        {
            anyhow::bail!(
                "trace source position is invalid at ordinal {}",
                receipt.ordinal
            );
        }
        if !is_lower_hex(&receipt.payload_sha256, 32) {
            anyhow::bail!("trace payload hash is not lowercase SHA-256");
        }
        if !is_lower_hex(&receipt.typed_receipt_sha256, 32) {
            anyhow::bail!("trace typed-receipt hash is not lowercase SHA-256");
        }
        let payload = hex_decode(&receipt.payload_hex, observer_limits.max_wire_frame_bytes)?;
        payload_bytes = checked_add_usize(payload_bytes, payload.len(), "trace payload bytes")?;
        if payload_bytes > limits.max_trace_payload_bytes {
            anyhow::bail!(
                "trace payloads exceed the {}-byte ceiling",
                limits.max_trace_payload_bytes
            );
        }
        if pid_runlog::sha256_hex(&payload) != receipt.payload_sha256 {
            anyhow::bail!("trace payload hash mismatch at ordinal {}", receipt.ordinal);
        }
        if typed_receipt_hash(receipt.expected_plane, &payload)?.as_deref()
            != Some(receipt.typed_receipt_sha256.as_str())
        {
            anyhow::bail!(
                "trace typed-receipt hash mismatch at ordinal {}",
                receipt.ordinal
            );
        }
        if !strict_json_preflight(&payload) {
            anyhow::bail!(
                "baseline trace payload {} is not strict conforming JSON",
                receipt.ordinal
            );
        }
        let (session_id, stream_position, source_position) = match receipt.expected_plane {
            IngressPlane::Sensor => {
                let frame = decode_validated::<SensorFrame>(&payload).map_err(|error| {
                    anyhow::anyhow!("baseline sensor {} is invalid: {error}", receipt.ordinal)
                })?;
                (frame.session_id, frame.stream.clone(), frame.stream)
            }
            IngressPlane::Command => {
                let frame = decode_validated::<CommandFrame>(&payload).map_err(|error| {
                    anyhow::anyhow!("baseline command {} is invalid: {error}", receipt.ordinal)
                })?;
                let source = frame.source.ok_or_else(|| {
                    anyhow::anyhow!("baseline command {} has no source", receipt.ordinal)
                })?;
                (frame.session_id, frame.stream, source)
            }
            IngressPlane::Observation => {
                let frame = decode_validated::<ObservationFrame>(&payload).map_err(|error| {
                    anyhow::anyhow!(
                        "baseline observation {} is invalid: {error}",
                        receipt.ordinal
                    )
                })?;
                let source = frame.source.ok_or_else(|| {
                    anyhow::anyhow!("baseline observation {} has no source", receipt.ordinal)
                })?;
                (frame.session_id, frame.stream, source)
            }
        };
        if session_id != trace.session_id
            || source_position.epoch != receipt.source_epoch
            || source_position.seq != receipt.source_seq
        {
            anyhow::bail!(
                "trace payload identity/source metadata mismatch at ordinal {}",
                receipt.ordinal
            );
        }
        let own_key = (receipt.expected_plane, stream_position.epoch.clone());
        if own_stream_last
            .insert(own_key, stream_position.seq)
            .is_some_and(|last| stream_position.seq <= last)
        {
            anyhow::bail!(
                "baseline own-stream positions must strictly advance at ordinal {}",
                receipt.ordinal
            );
        }
        let planes = join_planes
            .entry((receipt.source_epoch.clone(), receipt.source_seq))
            .or_default();
        if !planes.insert(receipt.expected_plane) {
            anyhow::bail!(
                "baseline trace has a duplicate plane/source receipt at ordinal {}",
                receipt.ordinal
            );
        }
        receipts.push(BaselineReceipt {
            ordinal: receipt.ordinal,
            logical_slot: receipt.logical_slot,
            routing_key: receipt.routing_key.clone(),
            expected_plane: receipt.expected_plane,
            source_epoch: receipt.source_epoch.clone(),
            source_seq: receipt.source_seq,
            payload,
        });
    }
    let all_planes = BTreeSet::from([
        IngressPlane::Sensor,
        IngressPlane::Command,
        IngressPlane::Observation,
    ]);
    if join_planes.values().any(|planes| planes != &all_planes) {
        anyhow::bail!("baseline trace must contain exactly three planes for every source tick");
    }
    let mut seqs_by_epoch = BTreeMap::<String, Vec<i64>>::new();
    for (epoch, seq) in join_planes.keys() {
        seqs_by_epoch.entry(epoch.clone()).or_default().push(*seq);
    }
    for seqs in seqs_by_epoch.values_mut() {
        seqs.sort_unstable();
        if seqs.first() != Some(&1)
            || seqs
                .windows(2)
                .any(|pair| pair[1] != pair[0].saturating_add(1))
        {
            anyhow::bail!(
                "baseline trace source universe must start at one and be contiguous per epoch"
            );
        }
    }
    let baseline_sample_ids = join_planes
        .keys()
        .map(|(epoch, seq)| format!("ncp-{epoch}-{seq}"))
        .collect();
    let canonical_sha256 = pid_runlog::canonical_json_hash_v2(&trace)
        .context("failed to hash canonical wire-trace manifest")?;
    let exact_sha256 = pid_runlog::sha256_hex(&exact_bytes);
    let matches_builtin_raw_receipts = validate_frozen_suite_baseline(&trace)?;
    Ok(ValidatedTrace {
        trace,
        exact_bytes,
        exact_sha256,
        canonical_sha256,
        receipts,
        baseline_sample_ids,
        matches_builtin_raw_receipts,
    })
}

fn validate_frozen_suite_baseline(trace: &WireTrace) -> anyhow::Result<bool> {
    let expected = synthetic_wire_trace(trace.producer_revision.clone())?;
    if trace.realm != expected.realm
        || trace.session_id != expected.session_id
        || trace.transport_condition != TransportCondition::OfflineNoTransport
        || trace.routes != expected.routes
        || trace.mapping != expected.mapping
        || trace.observer_limits != expected.observer_limits
        || trace.terminal != expected.terminal
        || trace.receipts.len() != expected.receipts.len()
    {
        anyhow::bail!(
            "trace is complete but does not match the frozen v1 observatory baseline contract"
        );
    }
    for (actual, frozen) in trace.receipts.iter().zip(&expected.receipts) {
        if actual.ordinal != frozen.ordinal
            || actual.logical_slot != frozen.logical_slot
            || actual.routing_key != frozen.routing_key
            || actual.expected_plane != frozen.expected_plane
            || actual.source_epoch != frozen.source_epoch
            || actual.source_seq != frozen.source_seq
            || actual.typed_receipt_sha256 != frozen.typed_receipt_sha256
        {
            anyhow::bail!(
                "trace receipt {} differs from the frozen v1 semantic baseline",
                actual.ordinal
            );
        }
    }
    Ok(trace
        .receipts
        .iter()
        .zip(&expected.receipts)
        .all(|(actual, frozen)| actual.payload_sha256 == frozen.payload_sha256))
}

fn stats_deltas(before: &ObserverStats, after: &ObserverStats) -> Vec<ObserverSignalDelta> {
    let mut deltas = Vec::new();
    let mut push = |code: ObserverSignalCode, before: u64, after: u64| {
        if after > before {
            deltas.push(ObserverSignalDelta {
                code,
                amount: after - before,
            });
        }
    };
    let usize_pair = |left: usize, right: usize| {
        (
            u64::try_from(left).unwrap_or(u64::MAX),
            u64::try_from(right).unwrap_or(u64::MAX),
        )
    };
    macro_rules! usize_delta {
        ($code:expr, $field:ident) => {{
            let (left, right) = usize_pair(before.$field, after.$field);
            push($code, left, right);
        }};
    }
    usize_delta!(ObserverSignalCode::KeptSample, kept_samples);
    if !before.zero_sample_capture && after.zero_sample_capture {
        push(ObserverSignalCode::ZeroSampleCapture, 0, 1);
    }
    usize_delta!(ObserverSignalCode::ExcludedEmptyV, excluded_empty_v);
    usize_delta!(ObserverSignalCode::ExcludedEmptyL, excluded_empty_l);
    usize_delta!(ObserverSignalCode::ExcludedEmptyD, excluded_empty_d);
    usize_delta!(ObserverSignalCode::ExcludedEmptyA, excluded_empty_a);
    usize_delta!(ObserverSignalCode::DimMismatch, dim_mismatch_dropped);
    usize_delta!(ObserverSignalCode::LateD, late_d_dropped);
    usize_delta!(ObserverSignalCode::UnstampedFrame, unstamped_frames_dropped);
    usize_delta!(
        ObserverSignalCode::SessionMismatch,
        session_mismatch_dropped
    );
    usize_delta!(
        ObserverSignalCode::RetiredEpoch,
        retired_epoch_frames_dropped
    );
    push(
        ObserverSignalCode::IngressDecode,
        before.ingress_decode_dropped,
        after.ingress_decode_dropped,
    );
    push(
        ObserverSignalCode::IngressUnstampedObservation,
        before.ingress_unstamped_observations_dropped,
        after.ingress_unstamped_observations_dropped,
    );
    push(
        ObserverSignalCode::IngressHandoff,
        before.ingress_handoff_dropped,
        after.ingress_handoff_dropped,
    );
    usize_delta!(ObserverSignalCode::Redelivery, redelivered_frames_dropped);
    usize_delta!(
        ObserverSignalCode::ConflictingDuplicate,
        conflicting_duplicates_dropped
    );
    usize_delta!(
        ObserverSignalCode::QuarantinedFrame,
        quarantined_frames_dropped
    );
    usize_delta!(ObserverSignalCode::InvalidPayload, invalid_payloads_dropped);
    usize_delta!(ObserverSignalCode::EvictedIncomplete, evicted_incomplete);
    usize_delta!(ObserverSignalCode::EvictedUnclaimedD, evicted_unclaimed_d);
    usize_delta!(
        ObserverSignalCode::IncompleteEpochTransition,
        incomplete_at_epoch_transition
    );
    usize_delta!(
        ObserverSignalCode::IncompleteFinalize,
        incomplete_at_finalize
    );
    usize_delta!(ObserverSignalCode::MissingSensor, incomplete_missing_sensor);
    usize_delta!(
        ObserverSignalCode::MissingCommand,
        incomplete_missing_command
    );
    usize_delta!(
        ObserverSignalCode::UnclaimedDEpochTransition,
        unclaimed_d_at_epoch_transition
    );
    usize_delta!(
        ObserverSignalCode::UnclaimedDFinalize,
        unclaimed_d_at_finalize
    );
    push(
        ObserverSignalCode::IngressOversized,
        before.ingress_oversized_dropped,
        after.ingress_oversized_dropped,
    );
    push(
        ObserverSignalCode::RouteMismatch,
        before.ingress_route_mismatch_dropped,
        after.ingress_route_mismatch_dropped,
    );
    push(
        ObserverSignalCode::IngressLifetimeLimit,
        before.ingress_lifetime_limit_dropped,
        after.ingress_lifetime_limit_dropped,
    );
    usize_delta!(
        ObserverSignalCode::CaptureCapacityDropped,
        capture_capacity_dropped
    );
    if !before.capture_capacity_reached && after.capture_capacity_reached {
        push(ObserverSignalCode::CaptureCapacityReached, 0, 1);
    }
    usize_delta!(ObserverSignalCode::SampleLimit, sample_limit_dropped);
    usize_delta!(ObserverSignalCode::ElementLimit, element_limit_dropped);
    usize_delta!(
        ObserverSignalCode::InflightElementLimit,
        inflight_element_limit_dropped
    );
    usize_delta!(ObserverSignalCode::EpochLimit, epoch_limit_dropped);
    push(
        ObserverSignalCode::WorkerFailure,
        before.capture_worker_failures,
        after.capture_worker_failures,
    );
    push(
        ObserverSignalCode::TeardownFailure,
        before.capture_teardown_failures,
        after.capture_teardown_failures,
    );
    push(
        ObserverSignalCode::EpochTransition,
        u64::from(before.seq_resets),
        u64::from(after.seq_resets),
    );
    deltas
}

fn mutate_json<F>(bytes: &[u8], mutate: F) -> anyhow::Result<Vec<u8>>
where
    F: FnOnce(&mut serde_json::Value) -> anyhow::Result<()>,
{
    let mut value: serde_json::Value =
        serde_json::from_slice(bytes).context("failed to decode scheduled JSON mutation")?;
    mutate(&mut value)?;
    serde_json::to_vec(&value).context("failed to encode scheduled JSON mutation")
}

fn object_mut<'a>(
    value: &'a mut serde_json::Value,
    name: &str,
) -> anyhow::Result<&'a mut serde_json::Map<String, serde_json::Value>> {
    value
        .get_mut(name)
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| anyhow::anyhow!("scheduled mutation expected object field {name:?}"))
}

fn set_stream_position(
    value: &mut serde_json::Value,
    field: &str,
    epoch: &str,
    seq: i64,
) -> anyhow::Result<()> {
    let position = object_mut(value, field)?;
    position.insert(
        "epoch".to_string(),
        serde_json::Value::String(epoch.to_string()),
    );
    position.insert("seq".to_string(), serde_json::json!(seq));
    Ok(())
}

fn receipt_index(
    receipts: &[ScheduledReceipt],
    seq: i64,
    plane: IngressPlane,
) -> anyhow::Result<usize> {
    receipts
        .iter()
        .position(|receipt| receipt.source_seq == seq && receipt.expected_plane == plane)
        .ok_or_else(|| anyhow::anyhow!("scenario target source seq {seq}/{plane:?} is absent"))
}

fn canonical_schedule_hash(
    scenario: FaultScenario,
    truth: &InjectionTruth,
    receipts: &[ScheduledReceipt],
) -> anyhow::Result<String> {
    #[derive(Serialize)]
    struct Entry<'a> {
        delivery_ordinal: usize,
        source_ordinal: u64,
        logical_slot: u64,
        routing_key: &'a str,
        expected_plane: IngressPlane,
        source_epoch: &'a str,
        source_seq: i64,
        payload_sha256: String,
        fault_ids: &'a [String],
    }
    let entries = receipts
        .iter()
        .enumerate()
        .map(|(delivery_ordinal, receipt)| Entry {
            delivery_ordinal,
            source_ordinal: receipt.source_ordinal,
            logical_slot: receipt.logical_slot,
            routing_key: &receipt.routing_key,
            expected_plane: receipt.expected_plane,
            source_epoch: &receipt.source_epoch,
            source_seq: receipt.source_seq,
            payload_sha256: pid_runlog::sha256_hex(&receipt.payload),
            fault_ids: &receipt.fault_ids,
        })
        .collect::<Vec<_>>();
    pid_runlog::canonical_json_hash_v2(&serde_json::json!({
        "schema_version": REPORT_SCHEMA_VERSION,
        "scenario": scenario,
        "truth": truth,
        "entries": entries,
    }))
    .context("failed to hash deterministic fault schedule")
}

fn expected_retained_ids(scenario: FaultScenario) -> Vec<String> {
    let mut ids = (1..=BASELINE_TICKS)
        .map(|seq| format!("ncp-{EPOCH_A}-{seq}"))
        .collect::<Vec<_>>();
    let remove_seq = match scenario {
        FaultScenario::ConflictingDuplicate => Some(3),
        FaultScenario::OnePlaneOmission => Some(2),
        FaultScenario::WholeTickOmission => Some(2),
        FaultScenario::ObservationAfterGrace => Some(1),
        FaultScenario::VersionMismatch => Some(4),
        FaultScenario::DuplicateJsonKey => Some(5),
        FaultScenario::MalformedNonUtf8 => Some(10),
        FaultScenario::IdentityCollision => Some(6),
        FaultScenario::RouteMismatch => Some(8),
        FaultScenario::OversizedPayload => Some(9),
        _ => None,
    };
    if let Some(seq) = remove_seq {
        ids.retain(|id| id != &format!("ncp-{EPOCH_A}-{seq}"));
    }
    match scenario {
        FaultScenario::TraceTruncation => ids.truncate(6),
        FaultScenario::NewStreamEpoch => {
            ids.truncate(6);
            ids.extend((1..=6).map(|seq| format!("ncp-{EPOCH_B}-{seq}")));
        }
        _ => {}
    }
    ids.sort();
    ids
}

fn compile_scenario(
    validated: &ValidatedTrace,
    scenario: FaultScenario,
    limits: ObservatoryLimits,
    observer_limits: ObserverLimits,
) -> anyhow::Result<ScheduledScenario> {
    let mut receipts = validated
        .receipts
        .iter()
        .map(|receipt| ScheduledReceipt {
            source_ordinal: receipt.ordinal,
            logical_slot: receipt.logical_slot,
            routing_key: receipt.routing_key.clone(),
            expected_plane: receipt.expected_plane,
            source_epoch: receipt.source_epoch.clone(),
            source_seq: receipt.source_seq,
            payload: receipt.payload.clone(),
            fault_ids: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut dropped_source_ordinals = Vec::new();
    let mut native_visible_dropped_source_ordinals = Vec::new();
    let mut manifest_only_dropped_source_ordinals = Vec::new();
    let mut duplicated_source_ordinals = Vec::new();
    let mut modified_source_ordinals = Vec::new();
    let mut reordered = false;
    let mut logical_displacement_slots = 0_u64;
    let mut terminal = TraceEndReason::ProducerClose;
    let mut transport_condition = validated.trace.transport_condition;

    match scenario {
        FaultScenario::CleanBaseline => {}
        FaultScenario::ExactRedelivery => {
            for plane in [
                IngressPlane::Observation,
                IngressPlane::Command,
                IngressPlane::Sensor,
            ] {
                let index = receipt_index(&receipts, 3, plane)?;
                let mut duplicate = receipts[index].clone();
                duplicate
                    .fault_ids
                    .push("exact_redelivery_before_closure".to_string());
                duplicated_source_ordinals.push(duplicate.source_ordinal);
                receipts.insert(index + 1, duplicate);
            }
            for plane in [
                IngressPlane::Sensor,
                IngressPlane::Command,
                IngressPlane::Observation,
            ] {
                let index = receipt_index(&receipts, 1, plane)?;
                let mut duplicate = receipts[index].clone();
                duplicate.logical_slot = receipts
                    .last()
                    .map_or(0, |receipt| receipt.logical_slot)
                    .saturating_add(1);
                duplicate
                    .fault_ids
                    .push("exact_redelivery_after_closure".to_string());
                duplicated_source_ordinals.push(duplicate.source_ordinal);
                receipts.push(duplicate);
            }
            reordered = true;
        }
        FaultScenario::ConflictingDuplicate => {
            let index = receipt_index(&receipts, 3, IngressPlane::Sensor)?;
            let mut duplicate = receipts[index].clone();
            duplicate.payload = mutate_json(&duplicate.payload, |value| {
                let channels = object_mut(value, "channels")?;
                let pose = channels
                    .get_mut("pose")
                    .and_then(serde_json::Value::as_object_mut)
                    .ok_or_else(|| anyhow::anyhow!("synthetic sensor pose channel is absent"))?;
                pose.insert("data".to_string(), serde_json::json!([999.0, 1.0]));
                Ok(())
            })?;
            duplicate
                .fault_ids
                .push("conflicting_duplicate".to_string());
            duplicated_source_ordinals.push(duplicate.source_ordinal);
            modified_source_ordinals.push(duplicate.source_ordinal);
            receipts.insert(index + 1, duplicate);
        }
        FaultScenario::ConflictingDuplicateAfterClosure => {
            let index = receipt_index(&receipts, 1, IngressPlane::Sensor)?;
            let mut duplicate = receipts[index].clone();
            duplicate.payload = mutate_json(&duplicate.payload, |value| {
                let channels = object_mut(value, "channels")?;
                let pose = channels
                    .get_mut("pose")
                    .and_then(serde_json::Value::as_object_mut)
                    .ok_or_else(|| anyhow::anyhow!("synthetic sensor pose channel is absent"))?;
                pose.insert("data".to_string(), serde_json::json!([777.0, 1.0]));
                Ok(())
            })?;
            duplicate.logical_slot = receipts
                .last()
                .map_or(0, |receipt| receipt.logical_slot)
                .saturating_add(1);
            duplicate
                .fault_ids
                .push("conflicting_duplicate_after_closure".to_string());
            duplicated_source_ordinals.push(duplicate.source_ordinal);
            modified_source_ordinals.push(duplicate.source_ordinal);
            receipts.push(duplicate);
            reordered = true;
        }
        FaultScenario::OnePlaneOmission => {
            let index = receipt_index(&receipts, 2, IngressPlane::Observation)?;
            dropped_source_ordinals.push(receipts[index].source_ordinal);
            native_visible_dropped_source_ordinals.push(receipts[index].source_ordinal);
            receipts.remove(index);
        }
        FaultScenario::WholeTickOmission => {
            let mut removed = Vec::new();
            receipts.retain(|receipt| {
                if receipt.source_seq == 2 {
                    removed.push(receipt.source_ordinal);
                    false
                } else {
                    true
                }
            });
            manifest_only_dropped_source_ordinals = removed.clone();
            dropped_source_ordinals = removed;
        }
        FaultScenario::ReorderWithinGrace => {
            let insertion = receipt_index(&receipts, 4, IngressPlane::Sensor)?;
            let slot = receipts[insertion].logical_slot;
            let mut group = Vec::new();
            for plane in [
                IngressPlane::Observation,
                IngressPlane::Command,
                IngressPlane::Sensor,
            ] {
                let index = receipt_index(&receipts, 4, plane)?;
                let mut receipt = receipts.remove(index);
                receipt.logical_slot = slot;
                receipt.fault_ids.push("within_grace_reorder".to_string());
                group.push(receipt);
            }
            for (offset, receipt) in group.into_iter().enumerate() {
                receipts.insert(insertion + offset, receipt);
            }
            reordered = true;
        }
        FaultScenario::LogicalReceiptPause => {
            let start = receipt_index(&receipts, 5, IngressPlane::Sensor)?;
            logical_displacement_slots = 1_000;
            for receipt in &mut receipts[start..] {
                receipt.logical_slot = receipt
                    .logical_slot
                    .checked_add(logical_displacement_slots)
                    .ok_or_else(|| anyhow::anyhow!("logical pause slot overflow"))?;
                receipt.fault_ids.push("logical_receipt_pause".to_string());
            }
        }
        FaultScenario::ObservationAfterGrace => {
            let index = receipt_index(&receipts, 1, IngressPlane::Observation)?;
            let mut delayed = receipts.remove(index);
            let previous_slot = delayed.logical_slot;
            delayed.logical_slot = receipts
                .last()
                .map_or(0, |receipt| receipt.logical_slot)
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("delayed observation slot overflow"))?;
            logical_displacement_slots = delayed.logical_slot.saturating_sub(previous_slot);
            delayed
                .fault_ids
                .push("observation_after_grace".to_string());
            modified_source_ordinals.push(delayed.source_ordinal);
            receipts.push(delayed);
            reordered = true;
        }
        FaultScenario::VersionMismatch => {
            let index = receipt_index(&receipts, 4, IngressPlane::Sensor)?;
            receipts[index].payload = mutate_json(&receipts[index].payload, |value| {
                let object = value
                    .as_object_mut()
                    .ok_or_else(|| anyhow::anyhow!("scheduled frame is not an object"))?;
                object.insert(
                    "ncp_version".to_string(),
                    serde_json::Value::String("0.7".to_string()),
                );
                Ok(())
            })?;
            receipts[index]
                .fault_ids
                .push("version_mismatch".to_string());
            modified_source_ordinals.push(receipts[index].source_ordinal);
        }
        FaultScenario::DuplicateJsonKey => {
            let index = receipt_index(&receipts, 5, IngressPlane::Command)?;
            let original = &receipts[index].payload;
            let object_start = original
                .iter()
                .position(|byte| !byte.is_ascii_whitespace())
                .ok_or_else(|| anyhow::anyhow!("synthetic command JSON is empty"))?;
            if original.get(object_start) != Some(&b'{') {
                anyhow::bail!("synthetic command JSON did not start with an object");
            }
            let prefix = br#"{"kind":"command_frame","#;
            let mut payload = Vec::new();
            payload
                .try_reserve(prefix.len().saturating_add(original.len()))
                .context("failed to reserve duplicate-key fixture")?;
            payload.extend_from_slice(&original[..object_start]);
            payload.extend_from_slice(prefix);
            payload.extend_from_slice(&original[object_start + 1..]);
            receipts[index].payload = payload;
            receipts[index]
                .fault_ids
                .push("duplicate_json_key".to_string());
            modified_source_ordinals.push(receipts[index].source_ordinal);
        }
        FaultScenario::MalformedNonUtf8 => {
            let index = receipt_index(&receipts, 10, IngressPlane::Sensor)?;
            receipts[index].payload = vec![0xff, 0xfe, 0xfd];
            receipts[index]
                .fault_ids
                .push("malformed_non_utf8".to_string());
            modified_source_ordinals.push(receipts[index].source_ordinal);
        }
        FaultScenario::TraceTruncation => {
            let last = receipt_index(&receipts, 7, IngressPlane::Sensor)?;
            for receipt in &receipts[last + 1..] {
                dropped_source_ordinals.push(receipt.source_ordinal);
                if receipt.source_seq == 7 && receipt.expected_plane == IngressPlane::Command {
                    native_visible_dropped_source_ordinals.push(receipt.source_ordinal);
                } else {
                    manifest_only_dropped_source_ordinals.push(receipt.source_ordinal);
                }
            }
            receipts.truncate(last + 1);
            terminal = TraceEndReason::TraceTruncation;
        }
        FaultScenario::NewStreamEpoch => {
            for receipt in &mut receipts {
                if receipt.source_seq < 7 {
                    continue;
                }
                let new_seq = receipt.source_seq - 6;
                let plane = receipt.expected_plane;
                receipt.payload = mutate_json(&receipt.payload, |value| {
                    set_stream_position(value, "stream", EPOCH_B, new_seq)?;
                    if plane != IngressPlane::Sensor {
                        set_stream_position(value, "source", EPOCH_B, new_seq)?;
                    }
                    Ok(())
                })?;
                receipt.source_epoch = EPOCH_B.to_string();
                receipt.source_seq = new_seq;
                receipt.fault_ids.push("new_stream_epoch".to_string());
                modified_source_ordinals.push(receipt.source_ordinal);
            }
        }
        FaultScenario::IdentityCollision => {
            for receipt in receipts
                .iter_mut()
                .filter(|receipt| receipt.source_seq == 6)
            {
                receipt.payload = mutate_json(&receipt.payload, |value| {
                    let session = object_mut(value, "session")?;
                    session.insert(
                        "generation".to_string(),
                        serde_json::Value::String(GENERATION_B.to_string()),
                    );
                    Ok(())
                })?;
                receipt.fault_ids.push("identity_collision".to_string());
                modified_source_ordinals.push(receipt.source_ordinal);
            }
        }
        FaultScenario::RouteMismatch => {
            let index = receipt_index(&receipts, 8, IngressPlane::Sensor)?;
            receipts[index].routing_key.push_str("/named");
            receipts[index].fault_ids.push("route_mismatch".to_string());
            modified_source_ordinals.push(receipts[index].source_ordinal);
        }
        FaultScenario::OversizedPayload => {
            let index = receipt_index(&receipts, 9, IngressPlane::Sensor)?;
            let oversized = observer_limits
                .max_wire_frame_bytes
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("oversized fixture length overflow"))?;
            receipts[index].payload = vec![b' '; oversized];
            receipts[index]
                .fault_ids
                .push("oversized_payload".to_string());
            modified_source_ordinals.push(receipts[index].source_ordinal);
        }
        FaultScenario::SecurityProfileClaimGuard => {
            transport_condition = TransportCondition::SecureConfigurationDeclared;
        }
    }

    dropped_source_ordinals.sort_unstable();
    native_visible_dropped_source_ordinals.sort_unstable();
    manifest_only_dropped_source_ordinals.sort_unstable();
    duplicated_source_ordinals.sort_unstable();
    duplicated_source_ordinals.dedup();
    modified_source_ordinals.sort_unstable();
    modified_source_ordinals.dedup();
    if receipts.len() > limits.max_scheduled_deliveries {
        anyhow::bail!(
            "scenario {} expands beyond {} deliveries",
            scenario.id(),
            limits.max_scheduled_deliveries
        );
    }
    let mut scheduled_bytes = 0_usize;
    for receipt in &receipts {
        scheduled_bytes = checked_add_usize(
            scheduled_bytes,
            receipt.payload.len(),
            "scheduled payload bytes",
        )?;
        if scheduled_bytes > limits.max_scheduled_payload_bytes {
            anyhow::bail!(
                "scenario {} exceeds the {}-byte scheduled-payload ceiling",
                scenario.id(),
                limits.max_scheduled_payload_bytes
            );
        }
        if receipt.routing_key.is_empty() || receipt.routing_key.len() > 4096 {
            anyhow::bail!("scheduled routing key is empty or oversized");
        }
        if receipt.fault_ids.len() > 8
            || receipt
                .fault_ids
                .iter()
                .any(|fault| fault.is_empty() || fault.len() > 128)
        {
            anyhow::bail!("scheduled fault labels exceed their finite contract");
        }
    }
    let truth = InjectionTruth {
        baseline_receipts: validated.receipts.len(),
        baseline_eligible_sample_ids: validated.baseline_sample_ids.clone(),
        dropped_source_ordinals,
        native_visible_dropped_source_ordinals,
        manifest_only_dropped_source_ordinals,
        duplicated_source_ordinals,
        modified_source_ordinals,
        reordered,
        logical_displacement_slots,
        terminal,
        transport_condition,
        logical_slots_are_annotations_only: true,
    };
    let schedule_hash = canonical_schedule_hash(scenario, &truth, &receipts)?;
    let matches_builtin_golden_schedule = schedule_hash == scenario.golden_schedule_sha256();
    if validated.matches_builtin_raw_receipts && !matches_builtin_golden_schedule {
        anyhow::bail!(
            "compiled scenario {} drifted from its hand-reviewed v1 golden hash: got {}, expected {}",
            scenario.id(),
            schedule_hash,
            scenario.golden_schedule_sha256()
        );
    }
    Ok(ScheduledScenario {
        scenario,
        receipts,
        truth,
        schedule_hash,
        matches_builtin_golden_schedule,
    })
}

fn preflight_suite_resources(
    validated: &ValidatedTrace,
    limits: ObservatoryLimits,
    observer_limits: ObserverLimits,
) -> anyhow::Result<()> {
    const DELIVERY_RECORD_FIXED_PROJECTION_BYTES: usize = 4096;
    let mut total_records = 0_usize;
    let mut total_projection_bytes = 0_usize;
    for scenario in FaultScenario::ALL {
        let scheduled = compile_scenario(validated, scenario, limits, observer_limits)?;
        total_records = checked_add_usize(
            total_records,
            scheduled.receipts.len(),
            "suite delivery-journal records",
        )?;
        if total_records > limits.max_total_journal_records {
            anyhow::bail!(
                "suite delivery journals exceed the {}-record ceiling",
                limits.max_total_journal_records
            );
        }
        for receipt in &scheduled.receipts {
            let fault_label_bytes =
                receipt.fault_ids.iter().try_fold(0_usize, |total, fault| {
                    checked_add_usize(total, fault.len(), "suite fault-label bytes")
                })?;
            let estimate = checked_add_usize(
                DELIVERY_RECORD_FIXED_PROJECTION_BYTES,
                receipt.routing_key.len(),
                "delivery-journal projection bytes",
            )?;
            let estimate = checked_add_usize(
                estimate,
                fault_label_bytes,
                "delivery-journal projection bytes",
            )?;
            total_projection_bytes = checked_add_usize(
                total_projection_bytes,
                estimate,
                "suite delivery-journal projection bytes",
            )?;
            if total_projection_bytes > limits.max_total_journal_projection_bytes {
                anyhow::bail!(
                    "suite delivery-journal projection exceeds the {}-byte ceiling",
                    limits.max_total_journal_projection_bytes
                );
            }
        }
    }
    Ok(())
}

fn compiled_schedule_bytes(
    validated: &ValidatedTrace,
    scheduled: &ScheduledScenario,
    limits: ObservatoryLimits,
) -> anyhow::Result<Vec<u8>> {
    let receipts = scheduled
        .receipts
        .iter()
        .enumerate()
        .map(|(delivery_ordinal, receipt)| {
            Ok(CompiledScheduleReceipt {
                delivery_ordinal: u64::try_from(delivery_ordinal)
                    .map_err(|_| anyhow::anyhow!("compiled delivery ordinal overflow"))?,
                source_ordinal: receipt.source_ordinal,
                logical_slot: receipt.logical_slot,
                routing_key: receipt.routing_key.clone(),
                expected_plane: receipt.expected_plane,
                source_epoch: receipt.source_epoch.clone(),
                source_seq: receipt.source_seq,
                payload_hex: hex_encode(&receipt.payload),
                payload_sha256: pid_runlog::sha256_hex(&receipt.payload),
                typed_receipt_sha256: typed_receipt_hash(receipt.expected_plane, &receipt.payload)?,
                fault_ids: receipt.fault_ids.clone(),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    serialize_json_pretty_bounded(
        &CompiledScheduleArtifact {
            schema_version: REPORT_SCHEMA_VERSION,
            scope: TRACE_SCOPE.to_string(),
            trace_exact_sha256: validated.exact_sha256.clone(),
            scenario: scheduled.scenario,
            schedule_sha256: scheduled.schedule_hash.clone(),
            injection_truth: scheduled.truth.clone(),
            receipts,
        },
        limits.max_compiled_schedule_bytes,
    )
    .context("failed to serialize bounded compiled fault schedule")
}

fn artifact_identity(
    root: &Path,
    path: &Path,
    max_bytes: usize,
) -> anyhow::Result<ArtifactIdentity> {
    let bytes = read_bounded(path, max_bytes)?;
    artifact_identity_from_bytes(root, path, &bytes)
}

fn artifact_identity_from_bytes(
    root: &Path,
    path: &Path,
    bytes: &[u8],
) -> anyhow::Result<ArtifactIdentity> {
    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("artifact {} escaped output root", path.display()))?;
    let relative_uri = relative
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("artifact relative path is not valid UTF-8"))?
        .to_string();
    Ok(ArtifactIdentity {
        relative_uri,
        sha256: pid_runlog::sha256_hex(bytes),
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
    })
}

struct VerifiedReplayBundle {
    dataset: OfflineVldaDataset,
    events: Vec<RunLogEvent>,
    dataset_bytes: Vec<u8>,
    runlog_bytes: Vec<u8>,
    receipt_bytes: Vec<u8>,
}

fn verify_replay_bundle(
    dataset_path: &Path,
    runlog_path: &Path,
    expected_integrity: &str,
    limits: ObserverLimits,
) -> anyhow::Result<VerifiedReplayBundle> {
    let receipt_path = publication_receipt_path(dataset_path);
    let receipt_bytes = read_bounded(&receipt_path, MAX_PUBLICATION_RECEIPT_BYTES)?;
    if !strict_json_preflight(&receipt_bytes) {
        anyhow::bail!("observer publication receipt is not strict JSON");
    }
    let receipt: OfflineVldaPublicationReceipt = serde_json::from_slice(&receipt_bytes)
        .context("failed to decode observer publication receipt")?;
    let dataset_bytes = read_bounded(dataset_path, limits.max_artifact_bytes)?;
    let runlog_bytes = read_bounded(runlog_path, limits.max_runlog_bytes)?;
    let dataset_uri = std::fs::canonicalize(dataset_path)?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("dataset canonical path is not UTF-8"))?
        .to_string();
    let runlog_uri = std::fs::canonicalize(runlog_path)?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("run-log canonical path is not UTF-8"))?
        .to_string();
    let receipt_uri = std::fs::canonicalize(&receipt_path)?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("receipt canonical path is not UTF-8"))?
        .to_string();
    if receipt.schema_version != 1
        || !receipt.committed
        || receipt.dataset_uri != dataset_uri
        || receipt.runlog_uri != runlog_uri
        || receipt.dataset_sha256 != pid_runlog::sha256_hex(&dataset_bytes)
        || receipt.runlog_sha256 != pid_runlog::sha256_hex(&runlog_bytes)
        || receipt.capture_integrity != expected_integrity
    {
        anyhow::bail!("observer replay bundle publication receipt failed exact verification");
    }
    if !strict_json_preflight(&dataset_bytes) {
        anyhow::bail!("observer replay dataset is not strict JSON");
    }
    let dataset: OfflineVldaDataset =
        serde_json::from_slice(&dataset_bytes).context("failed to decode replay dataset")?;
    if dataset.publication_receipt != receipt_uri
        || dataset.capture_integrity != expected_integrity
        || dataset.support != BTreeMap::new()
    {
        anyhow::bail!("observer replay dataset does not match its publication receipt/scope");
    }
    let events = pid_runlog::read_events(std::io::Cursor::new(&runlog_bytes))
        .context("failed to read replay run-log snapshot")?;
    let validation = validate_events(&events).context("failed to validate replay run log")?;
    if !validation.is_valid() {
        anyhow::bail!("observer replay run log is schema-invalid");
    }
    let starts = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RunLogEvent::RunStarted { run_id, .. } if run_id == &dataset.run_id
            )
        })
        .count();
    let expected_status = if matches!(expected_integrity, "complete" | "complete_with_warning") {
        RunStatus::Succeeded
    } else {
        RunStatus::Failed
    };
    let ends = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RunLogEvent::RunEnded { run_id, status, .. }
                    if run_id == &dataset.run_id && status == &expected_status
            )
        })
        .count();
    let artifacts = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                RunLogEvent::ArtifactLogged {
                    uri,
                    sha256: Some(sha256),
                    metadata,
                    ..
                } if uri == &dataset_uri
                    && sha256 == &receipt.dataset_sha256
                    && metadata.get("capture_integrity").map(String::as_str)
                        == Some(expected_integrity)
            )
        })
        .count();
    if starts != 1 || ends != 1 || artifacts != 1 {
        anyhow::bail!("observer replay run log does not bind its exact dataset/run identity");
    }
    Ok(VerifiedReplayBundle {
        dataset,
        events,
        dataset_bytes,
        runlog_bytes,
        receipt_bytes,
    })
}

fn dataset_semantic_hash(dataset: &OfflineVldaDataset) -> anyhow::Result<String> {
    let mut projection = dataset.clone();
    projection.publication_receipt = "<publication-bound-path-excluded>".to_string();
    pid_runlog::canonical_json_hash_v2(&serde_json::json!({
        "projection_revision": DATASET_PROJECTION_REVISION,
        "dataset": projection,
    }))
    .context("failed to hash path-independent dataset projection")
}

fn scientific_payload_hash(dataset: &OfflineVldaDataset) -> anyhow::Result<String> {
    pid_runlog::canonical_json_hash_v2(&serde_json::json!({
        "projection_revision": SCIENTIFIC_PAYLOAD_PROJECTION_REVISION,
        "samples": dataset.samples,
    }))
    .context("failed to hash scientific sample payload projection")
}

fn sample_value_hash(sample: &OfflineVldaSample) -> anyhow::Result<String> {
    let mut metadata = sample.metadata.clone();
    metadata.remove("epoch");
    metadata.remove("seq");
    pid_runlog::canonical_json_hash_v2(&serde_json::json!({
        "projection_revision": SAMPLE_VALUE_PROJECTION_REVISION,
        "episode_id": sample.episode_id,
        "v": sample.v,
        "l": sample.l,
        "d": sample.d,
        "a": sample.a,
        "labels": sample.labels,
        "metadata": metadata,
    }))
    .context("failed to hash identity-normalized sample content")
}

fn normalized_runlog_hash(
    events: &[RunLogEvent],
    semantic_dataset_hash: &str,
) -> anyhow::Result<String> {
    let mut normalized = events.to_vec();
    for event in &mut normalized {
        if let RunLogEvent::ArtifactLogged { uri, sha256, .. } = event {
            *uri = "<publication-bound-path-excluded>".to_string();
            *sha256 = Some(semantic_dataset_hash.to_string());
        }
    }
    logical_trace_hash_v3(&normalized)
        .context("failed to hash publication-normalized logical run log")
}

#[allow(clippy::too_many_arguments)]
fn outcome_fingerprint(
    scenario: FaultScenario,
    schedule_hash: &str,
    terminal: TraceEndReason,
    stats: &ObserverStats,
    counters: &RawIngressCounters,
    journal: &[DeliveryRecord],
    finalization_deltas: &[ObserverSignalDelta],
    sample_ids: &[String],
    sample_value_hashes: &BTreeMap<String, String>,
    dataset_semantic_hash: &str,
    scientific_payload_hash: &str,
    normalized_logical_trace_hash_v3: &str,
    runlog_validation_errors: usize,
    runlog_validation_warnings: usize,
) -> anyhow::Result<String> {
    pid_runlog::canonical_json_hash_v2(&serde_json::json!({
        "revision": OUTCOME_FINGERPRINT_REVISION,
        "scenario": scenario,
        "schedule_hash": schedule_hash,
        "terminal": terminal,
        "stats": stats,
        "raw_ingress_counters": counters,
        "delivery_journal": journal,
        "finalization_deltas": finalization_deltas,
        "sample_ids": sample_ids,
        "sample_value_hashes": sample_value_hashes,
        "dataset_semantic_hash": dataset_semantic_hash,
        "scientific_payload_hash": scientific_payload_hash,
        "normalized_logical_trace_hash_v3": normalized_logical_trace_hash_v3,
        "runlog_validation_errors": runlog_validation_errors,
        "runlog_validation_warnings": runlog_validation_warnings,
    }))
    .context("failed to hash path-independent replay outcome")
}

fn replay_once(
    root: &Path,
    validated: &ValidatedTrace,
    scheduled: &ScheduledScenario,
    replay_name: &str,
    observatory_limits: ObservatoryLimits,
    observer_limits: ObserverLimits,
) -> anyhow::Result<ReplayRun> {
    let replay_dir = root.join(scheduled.scenario.id()).join(replay_name);
    ensure_directory(&replay_dir)?;
    let dataset_path = replay_dir.join("dataset.json");
    let runlog_path = replay_dir.join("runlog.jsonl");
    let routes = validated.trace.routes.ingress_routes()?;
    let mapping = Mapping {
        language_channel: validated.trace.mapping.language_channel.clone(),
        success_channel: validated.trace.mapping.success_channel.clone(),
        episode_id: validated.trace.mapping.episode_id.clone(),
    };
    let mut observer = Observer::new(
        format!("ncp-fault-observatory-{}", scheduled.scenario.id()),
        "synthetic-wire-0.8",
        "protocol-fault-observatory",
        mapping,
    )
    .with_limits(observer_limits)?
    .with_expected_session(validated.trace.session_id.clone())?
    .with_capture_transport(
        validated.trace.realm.clone(),
        scheduled.truth.transport_condition.observer_label(),
        INGRESS_HANDOFF_CAPACITY,
    )?
    .with_runlog(&runlog_path)?;
    let mut counters = RawIngressCounters::default();
    let mut journal = Vec::new();
    journal
        .try_reserve_exact(scheduled.receipts.len())
        .context("failed to reserve bounded delivery journal")?;
    for (delivery_index, receipt) in scheduled.receipts.iter().enumerate() {
        let before = observer.stats().clone();
        let before_samples = observer.sample_count();
        let admission = classify_callback_receipt(
            &routes,
            &receipt.routing_key,
            receipt.payload.len(),
            observer_limits.max_wire_frame_bytes,
        );
        let mut worker_error = None;
        let disposition = match admission {
            CallbackAdmission::RouteMismatchDropped => {
                observer.record_callback_drops(0, 1, 0)?;
                ReceiptDisposition::CallbackRouteMismatchDropped
            }
            CallbackAdmission::OversizedDropped => {
                observer.record_callback_drops(1, 0, 0)?;
                ReceiptDisposition::CallbackOversizedDropped
            }
            CallbackAdmission::Admitted(plane) => {
                match ingest_wire_frame(&mut observer, plane, &receipt.payload, &mut counters) {
                    Ok(outcome) => outcome.into(),
                    Err(error) => {
                        observer.record_capture_worker_failure()?;
                        worker_error = Some(error);
                        ReceiptDisposition::WorkerError
                    }
                }
            }
        };
        let after = observer.stats().clone();
        let after_samples = observer.sample_count();
        journal.push(DeliveryRecord {
            delivery_ordinal: u64::try_from(delivery_index)
                .map_err(|_| anyhow::anyhow!("delivery ordinal overflow"))?,
            source_ordinal: receipt.source_ordinal,
            logical_slot: receipt.logical_slot,
            routing_key: receipt.routing_key.clone(),
            expected_plane: receipt.expected_plane,
            payload_sha256: pid_runlog::sha256_hex(&receipt.payload),
            typed_receipt_sha256: typed_receipt_hash(receipt.expected_plane, &receipt.payload)?,
            fault_ids: receipt.fault_ids.clone(),
            disposition,
            observer_deltas: stats_deltas(&before, &after),
            sample_count_delta: u64::try_from(after_samples.saturating_sub(before_samples))
                .unwrap_or(u64::MAX),
        });
        if worker_error.is_some() {
            break;
        }
    }
    let before_finalize = observer.stats().clone();
    let stats = observer.finalize(&dataset_path)?;
    let finalization_deltas = stats_deltas(&before_finalize, &stats);
    let verified = verify_replay_bundle(
        &dataset_path,
        &runlog_path,
        stats.capture_integrity(),
        observer_limits,
    )?;
    let dataset = verified.dataset;
    let events = verified.events;
    let dataset_semantic_hash = dataset_semantic_hash(&dataset)?;
    let scientific_payload_hash = scientific_payload_hash(&dataset)?;
    let normalized_logical_trace_hash_v3 = normalized_runlog_hash(&events, &dataset_semantic_hash)?;
    let validation = validate_events(&events)?;
    let sample_value_hashes = dataset
        .samples
        .iter()
        .map(|sample| Ok((sample.sample_id.clone(), sample_value_hash(sample)?)))
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
    let mut sample_ids = dataset
        .samples
        .iter()
        .map(|sample| sample.sample_id.clone())
        .collect::<Vec<_>>();
    sample_ids.sort();
    let outcome_fingerprint = outcome_fingerprint(
        scheduled.scenario,
        &scheduled.schedule_hash,
        scheduled.truth.terminal,
        &stats,
        &counters,
        &journal,
        &finalization_deltas,
        &sample_ids,
        &sample_value_hashes,
        &dataset_semantic_hash,
        &scientific_payload_hash,
        &normalized_logical_trace_hash_v3,
        validation.errors,
        validation.warnings,
    )?;
    let outcome_path = replay_dir.join("outcome.json");
    let outcome_record = ReplayOutcomeRecord {
        schema_version: REPORT_SCHEMA_VERSION,
        scope: TRACE_SCOPE.to_string(),
        scenario: scheduled.scenario,
        schedule_sha256: scheduled.schedule_hash.clone(),
        replay: replay_name.to_string(),
        stats: stats.clone(),
        counters: counters.clone(),
        journal: journal.clone(),
        finalization_deltas: finalization_deltas.clone(),
        sample_ids: sample_ids.clone(),
        sample_value_hashes: sample_value_hashes.clone(),
        dataset_semantic_hash: dataset_semantic_hash.clone(),
        scientific_payload_hash: scientific_payload_hash.clone(),
        normalized_logical_trace_hash_v3: normalized_logical_trace_hash_v3.clone(),
        outcome_fingerprint: outcome_fingerprint.clone(),
        runlog_validation_errors: validation.errors,
        runlog_validation_warnings: validation.warnings,
    };
    let outcome_bytes = serialize_json_pretty_bounded(
        &outcome_record,
        observatory_limits.max_replay_outcome_bytes,
    )?;
    atomic_write_bytes(&outcome_path, &outcome_bytes, "observatory replay outcome")?;
    let receipt_path = publication_receipt_path(&dataset_path);
    let artifacts = vec![
        artifact_identity(root, &dataset_path, observer_limits.max_artifact_bytes)?,
        artifact_identity(root, &runlog_path, observer_limits.max_runlog_bytes)?,
        artifact_identity(root, &receipt_path, MAX_PUBLICATION_RECEIPT_BYTES)?,
        artifact_identity(
            root,
            &outcome_path,
            observatory_limits.max_replay_outcome_bytes,
        )?,
    ];
    Ok(ReplayRun {
        stats,
        counters,
        journal,
        finalization_deltas,
        sample_ids,
        sample_value_hashes,
        dataset_semantic_hash,
        scientific_payload_hash,
        normalized_logical_trace_hash_v3,
        outcome_fingerprint,
        runlog_validation_errors: validation.errors,
        runlog_validation_warnings: validation.warnings,
        artifacts,
    })
}

#[derive(Debug)]
struct ScenarioExpectation {
    stats: ObserverStats,
    observability: ObservabilityClass,
    native_response: NativeObserverResponse,
    matched_verdict: ScenarioVerdict,
}

fn scenario_expectation(scenario: FaultScenario) -> ScenarioExpectation {
    let mut stats = ObserverStats::default();
    let (kept_samples, observability, native_response, matched_verdict) = match scenario {
        FaultScenario::CleanBaseline => (
            12,
            ObservabilityClass::NotApplicable,
            NativeObserverResponse::NotApplicable,
            ScenarioVerdict::Matched,
        ),
        FaultScenario::ExactRedelivery => {
            stats.redelivered_frames_dropped = 6;
            (
                12,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::Tolerated,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::ConflictingDuplicate => {
            stats.conflicting_duplicates_dropped = 1;
            stats.quarantined_frames_dropped = 2;
            (
                11,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::DetectedRejected,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::ConflictingDuplicateAfterClosure => {
            stats.conflicting_duplicates_dropped = 1;
            (
                12,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::DetectedRejected,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::OnePlaneOmission => {
            stats.excluded_empty_d = 1;
            (
                11,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::DetectedDegraded,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::WholeTickOmission => (
            11,
            ObservabilityClass::ManifestOnly,
            NativeObserverResponse::NotDetected,
            ScenarioVerdict::MatchedKnownLimitation,
        ),
        FaultScenario::ReorderWithinGrace => (
            12,
            ObservabilityClass::VisibleReceipt,
            NativeObserverResponse::Tolerated,
            ScenarioVerdict::Matched,
        ),
        FaultScenario::LogicalReceiptPause => (
            12,
            ObservabilityClass::NotAssessableOffline,
            NativeObserverResponse::NotAssessable,
            ScenarioVerdict::NotAssessable,
        ),
        FaultScenario::ObservationAfterGrace => {
            stats.excluded_empty_d = 1;
            stats.late_d_dropped = 1;
            (
                11,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::DetectedDegraded,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::VersionMismatch | FaultScenario::MalformedNonUtf8 => {
            stats.ingress_decode_dropped = 1;
            stats.incomplete_at_finalize = 1;
            stats.incomplete_missing_sensor = 1;
            stats.unclaimed_d_at_finalize = 1;
            (
                11,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::DetectedRejected,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::DuplicateJsonKey => {
            stats.ingress_decode_dropped = 1;
            stats.incomplete_at_finalize = 1;
            stats.incomplete_missing_command = 1;
            stats.unclaimed_d_at_finalize = 1;
            (
                11,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::DetectedRejected,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::TraceTruncation => {
            stats.incomplete_at_finalize = 1;
            stats.incomplete_missing_command = 1;
            (
                6,
                ObservabilityClass::MixedVisibleAndManifestOnly,
                NativeObserverResponse::DetectedDegraded,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::NewStreamEpoch => {
            stats.seq_resets = 1;
            (
                12,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::Tolerated,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::IdentityCollision => {
            stats.session_mismatch_dropped = 3;
            (
                11,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::DetectedDegraded,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::RouteMismatch | FaultScenario::OversizedPayload => {
            stats.incomplete_at_finalize = 1;
            stats.incomplete_missing_sensor = 1;
            stats.unclaimed_d_at_finalize = 1;
            if scenario == FaultScenario::RouteMismatch {
                stats.ingress_route_mismatch_dropped = 1;
            } else {
                stats.ingress_oversized_dropped = 1;
            }
            (
                11,
                ObservabilityClass::VisibleReceipt,
                NativeObserverResponse::DetectedRejected,
                ScenarioVerdict::Matched,
            )
        }
        FaultScenario::SecurityProfileClaimGuard => (
            12,
            ObservabilityClass::LiveTransportOnly,
            NativeObserverResponse::NotAssessable,
            ScenarioVerdict::NotAssessable,
        ),
    };
    stats.kept_samples = kept_samples;
    ScenarioExpectation {
        stats,
        observability,
        native_response,
        matched_verdict,
    }
}

fn observed_native_response(
    observability: ObservabilityClass,
    integrity: &str,
    native_signal_deltas: &[ObserverSignalDelta],
) -> NativeObserverResponse {
    match observability {
        ObservabilityClass::NotApplicable => NativeObserverResponse::NotApplicable,
        ObservabilityClass::NotAssessableOffline | ObservabilityClass::LiveTransportOnly => {
            NativeObserverResponse::NotAssessable
        }
        ObservabilityClass::ManifestOnly if native_signal_deltas.is_empty() => {
            NativeObserverResponse::NotDetected
        }
        ObservabilityClass::ManifestOnly
        | ObservabilityClass::MixedVisibleAndManifestOnly
        | ObservabilityClass::VisibleReceipt => match integrity {
            "invalid" => NativeObserverResponse::DetectedRejected,
            "degraded" => NativeObserverResponse::DetectedDegraded,
            "complete" | "complete_with_warning" => NativeObserverResponse::Tolerated,
            _ => NativeObserverResponse::NotIdentifiable,
        },
    }
}

fn duplicate_ids(ids: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for id in ids {
        if !seen.insert(id.clone()) {
            duplicates.insert(id.clone());
        }
    }
    duplicates.into_iter().collect()
}

fn set_difference(left: &[String], right: &[String]) -> Vec<String> {
    let right = right.iter().cloned().collect::<BTreeSet<_>>();
    left.iter()
        .filter(|value| !right.contains(*value))
        .cloned()
        .collect()
}

fn journal_conservation_failures(scheduled: &ScheduledScenario, replay: &ReplayRun) -> Vec<String> {
    let mut failures = Vec::new();
    if replay.journal.len() != scheduled.receipts.len() {
        failures.push(format!(
            "delivery journal contains {} records for {} scheduled deliveries",
            replay.journal.len(),
            scheduled.receipts.len()
        ));
    }
    for (index, (scheduled_receipt, recorded)) in
        scheduled.receipts.iter().zip(&replay.journal).enumerate()
    {
        let expected_typed = match typed_receipt_hash(
            scheduled_receipt.expected_plane,
            &scheduled_receipt.payload,
        ) {
            Ok(value) => value,
            Err(_) => {
                failures.push(format!(
                    "delivery journal record {index} has an unhashable typed projection"
                ));
                None
            }
        };
        let expected_disposition = if scheduled_receipt
            .fault_ids
            .iter()
            .any(|fault| fault == "route_mismatch")
        {
            ReceiptDisposition::CallbackRouteMismatchDropped
        } else if scheduled_receipt.payload.len() > ObserverLimits::default().max_wire_frame_bytes {
            ReceiptDisposition::CallbackOversizedDropped
        } else if expected_typed.is_none() {
            ReceiptDisposition::DecodeDropped
        } else {
            ReceiptDisposition::Applied
        };
        let metadata_matches = recorded.delivery_ordinal
            == u64::try_from(index).unwrap_or(u64::MAX)
            && recorded.source_ordinal == scheduled_receipt.source_ordinal
            && recorded.logical_slot == scheduled_receipt.logical_slot
            && recorded.routing_key == scheduled_receipt.routing_key
            && recorded.expected_plane == scheduled_receipt.expected_plane
            && recorded.payload_sha256 == pid_runlog::sha256_hex(&scheduled_receipt.payload)
            && recorded.typed_receipt_sha256 == expected_typed
            && recorded.fault_ids == scheduled_receipt.fault_ids
            && recorded.disposition == expected_disposition;
        if !metadata_matches {
            failures.push(format!(
                "delivery journal record {index} does not match its compiled schedule entry"
            ));
        }
    }

    let mut expected = RawIngressCounters::default();
    for (scheduled_receipt, recorded) in scheduled.receipts.iter().zip(&replay.journal) {
        if matches!(
            recorded.disposition,
            ReceiptDisposition::CallbackRouteMismatchDropped
                | ReceiptDisposition::CallbackOversizedDropped
                | ReceiptDisposition::CapacityDropped
        ) {
            continue;
        }
        expected.frames_seen = expected.frames_seen.saturating_add(1);
        expected.raw_bytes_seen = expected
            .raw_bytes_seen
            .saturating_add(u64::try_from(scheduled_receipt.payload.len()).unwrap_or(u64::MAX));
        match (recorded.disposition, scheduled_receipt.expected_plane) {
            (ReceiptDisposition::DecodeDropped, IngressPlane::Sensor) => {
                expected.sensor_decode_failures = expected.sensor_decode_failures.saturating_add(1);
            }
            (ReceiptDisposition::DecodeDropped, IngressPlane::Command) => {
                expected.command_decode_failures =
                    expected.command_decode_failures.saturating_add(1);
            }
            (ReceiptDisposition::DecodeDropped, IngressPlane::Observation) => {
                expected.observation_decode_failures =
                    expected.observation_decode_failures.saturating_add(1);
            }
            (ReceiptDisposition::UnstampedObservationDropped, _) => {
                expected.observation_unstamped = expected.observation_unstamped.saturating_add(1);
            }
            (ReceiptDisposition::WorkerOversizedDropped, _) => {
                expected.oversized_frames = expected.oversized_frames.saturating_add(1);
            }
            _ => {}
        }
    }
    if replay.counters != expected {
        failures.push("raw-ingress counters do not conserve the delivery journal".to_string());
    }
    let aggregate = |deltas: &[ObserverSignalDelta]| {
        let mut totals = BTreeMap::<ObserverSignalCode, u64>::new();
        for delta in deltas {
            totals
                .entry(delta.code)
                .and_modify(|amount| *amount = amount.saturating_add(delta.amount))
                .or_insert(delta.amount);
        }
        totals
    };
    let recorded_deltas = replay
        .journal
        .iter()
        .flat_map(|record| record.observer_deltas.iter().cloned())
        .chain(replay.finalization_deltas.iter().cloned())
        .collect::<Vec<_>>();
    if aggregate(&recorded_deltas)
        != aggregate(&stats_deltas(&ObserverStats::default(), &replay.stats))
    {
        failures.push("observer signal deltas do not conserve final stats".to_string());
    }
    let journal_samples = replay.journal.iter().fold(0_u64, |total, record| {
        total.saturating_add(record.sample_count_delta)
    });
    let finalized_samples = replay
        .finalization_deltas
        .iter()
        .filter(|delta| delta.code == ObserverSignalCode::KeptSample)
        .fold(0_u64, |total, delta| total.saturating_add(delta.amount));
    if journal_samples.saturating_add(finalized_samples)
        != u64::try_from(replay.stats.kept_samples).unwrap_or(u64::MAX)
        || replay.stats.kept_samples != replay.sample_ids.len()
    {
        failures.push("sample-count deltas do not conserve retained samples".to_string());
    }
    failures
}

fn assemble_scenario_report(
    scheduled: ScheduledScenario,
    compiled_schedule_artifact: ArtifactIdentity,
    first: ReplayRun,
    second: ReplayRun,
) -> ScenarioReport {
    let expectation = scenario_expectation(scheduled.scenario);
    let mut failures = journal_conservation_failures(&scheduled, &first);
    failures.extend(journal_conservation_failures(&scheduled, &second));
    let integrity = first.stats.capture_integrity().to_string();
    let expected_integrity = expectation.stats.capture_integrity();
    if integrity != expected_integrity {
        failures.push(format!(
            "capture_integrity={integrity:?}, expected {:?}",
            expected_integrity
        ));
    }
    if first.stats != expectation.stats {
        failures.push("observer counters differ from the exact frozen oracle".to_string());
    }
    let native_signal_deltas = stats_deltas(&ObserverStats::default(), &first.stats)
        .into_iter()
        .filter(|delta| delta.code != ObserverSignalCode::KeptSample)
        .collect::<Vec<_>>();
    let expected_native_signal_deltas = stats_deltas(&ObserverStats::default(), &expectation.stats)
        .into_iter()
        .filter(|delta| delta.code != ObserverSignalCode::KeptSample)
        .collect::<Vec<_>>();
    let native_signal_codes = native_signal_deltas
        .iter()
        .map(|delta| delta.code)
        .collect::<Vec<_>>();
    let expected_native_signal_codes = expected_native_signal_deltas
        .iter()
        .map(|delta| delta.code)
        .collect::<Vec<_>>();
    let native_response =
        observed_native_response(expectation.observability, &integrity, &native_signal_deltas);
    if native_response != expectation.native_response {
        failures.push(format!(
            "native_response={native_response:?}, expected {:?}",
            expectation.native_response
        ));
    }
    let expected_ids = expected_retained_ids(scheduled.scenario);
    let unexpected_missing_ids = set_difference(&expected_ids, &first.sample_ids);
    let unexpected_extra_ids = set_difference(&first.sample_ids, &expected_ids);
    let duplicate_output_ids = duplicate_ids(&first.sample_ids);
    if !unexpected_missing_ids.is_empty()
        || !unexpected_extra_ids.is_empty()
        || !duplicate_output_ids.is_empty()
    {
        failures.push("actual sample-id set differs from the frozen scenario oracle".to_string());
    }
    if first.runlog_validation_errors != 0 || second.runlog_validation_errors != 0 {
        failures.push("one replay run log has validation errors".to_string());
    }
    let replay_equal = first.outcome_fingerprint == second.outcome_fingerprint
        && first.stats == second.stats
        && first.counters == second.counters
        && first.journal == second.journal
        && first.finalization_deltas == second.finalization_deltas
        && first.dataset_semantic_hash == second.dataset_semantic_hash
        && first.scientific_payload_hash == second.scientific_payload_hash
        && first.normalized_logical_trace_hash_v3 == second.normalized_logical_trace_hash_v3
        && first.sample_ids == second.sample_ids
        && first.sample_value_hashes == second.sample_value_hashes
        && first.runlog_validation_errors == second.runlog_validation_errors
        && first.runlog_validation_warnings == second.runlog_validation_warnings;
    if !replay_equal {
        failures.push("independent path-normalized replay outcomes differ".to_string());
    }
    let verdict = if failures.is_empty() {
        expectation.matched_verdict
    } else {
        ScenarioVerdict::Mismatched
    };
    let mut artifacts = first.artifacts.clone();
    artifacts.extend(second.artifacts.clone());
    ScenarioReport {
        scenario: scheduled.scenario,
        schedule_hash: scheduled.schedule_hash,
        builtin_golden_schedule_hash: scheduled.scenario.golden_schedule_sha256().to_string(),
        matches_builtin_golden_schedule: scheduled.matches_builtin_golden_schedule,
        injection_truth: scheduled.truth.clone(),
        delivery_journal: first.journal,
        finalization_deltas: first.finalization_deltas,
        observer_stats: first.stats,
        raw_ingress_counters: first.counters,
        observer_integrity: integrity,
        detection: DetectionAssessment {
            observability: expectation.observability,
            native_response,
            expected_native_response: expectation.native_response,
            native_signal_codes,
            expected_native_signal_codes,
            native_signal_deltas,
            expected_native_signal_deltas,
            fixture_oracle_knows_injection: scheduled.scenario != FaultScenario::CleanBaseline,
            wall_clock_latency: AssessmentStatus::NotAssessed,
        },
        sample_sets: SampleSetAccounting {
            baseline_eligible_ids: scheduled.truth.baseline_eligible_sample_ids,
            expected_retained_ids: expected_ids,
            actual_retained_ids: first.sample_ids,
            unexpected_missing_ids,
            unexpected_extra_ids,
            duplicate_output_ids,
        },
        sample_content: SampleContentAccounting {
            projection_revision: SAMPLE_VALUE_PROJECTION_REVISION.to_string(),
            expected_value_hashes: BTreeMap::new(),
            actual_value_hashes: first.sample_value_hashes,
            mismatched_or_missing_ids: Vec::new(),
            unexpected_ids: Vec::new(),
            matches_clean_fixture_oracle: false,
        },
        replay_equivalence: ReplayEquivalence {
            equal: replay_equal,
            outcome_fingerprint_revision: OUTCOME_FINGERPRINT_REVISION.to_string(),
            first_outcome_fingerprint: first.outcome_fingerprint,
            second_outcome_fingerprint: second.outcome_fingerprint,
            dataset_projection_revision: DATASET_PROJECTION_REVISION.to_string(),
            first_dataset_semantic_hash: first.dataset_semantic_hash,
            second_dataset_semantic_hash: second.dataset_semantic_hash,
            scientific_payload_projection_revision: SCIENTIFIC_PAYLOAD_PROJECTION_REVISION
                .to_string(),
            first_scientific_payload_hash: first.scientific_payload_hash,
            second_scientific_payload_hash: second.scientific_payload_hash,
            runlog_projection_revision: RUNLOG_PROJECTION_REVISION.to_string(),
            first_normalized_logical_trace_hash_v3: first.normalized_logical_trace_hash_v3,
            second_normalized_logical_trace_hash_v3: second.normalized_logical_trace_hash_v3,
            publication_byte_identity_expected: false,
        },
        scientific_payload_expectation: match scheduled.scenario {
            FaultScenario::CleanBaseline
            | FaultScenario::ExactRedelivery
            | FaultScenario::ReorderWithinGrace
            | FaultScenario::LogicalReceiptPause
            | FaultScenario::SecurityProfileClaimGuard => {
                ScientificPayloadExpectation::SameAsCleanBaseline
            }
            _ => ScientificPayloadExpectation::ScenarioSpecific,
        },
        scientific_payload_matches_clean_baseline: None,
        compiled_schedule_artifact,
        replay_artifacts: artifacts,
        security: SecurityAssessment {
            transport_exercised: false,
            configuration_condition: scheduled.truth.transport_condition,
            configuration_loaded: false,
            configuration_selected: false,
            declared_profile_label_only: true,
            peer_authentication: AssessmentStatus::NotAssessed,
            acl_enforcement: AssessmentStatus::NotAssessed,
            security_validation: AssessmentStatus::NotEstablished,
        },
        control_scope: ControlScopeAssessment {
            execution_scope: "offline sequential read-only ingress replay".to_string(),
            action_plane_publications: 0,
            agent_bridge_requests: 0,
            live_control_timing_noninterference: AssessmentStatus::NotAssessed,
        },
        expectation_failures: failures,
        verdict,
    }
}

fn expected_sample_value_hashes(
    scenario: FaultScenario,
    expected_retained_ids: &[String],
) -> anyhow::Result<BTreeMap<String, String>> {
    let baseline_values = (1..=BASELINE_TICKS)
        .zip(GOLDEN_CLEAN_SAMPLE_VALUE_HASHES)
        .map(|(seq, hash)| (format!("ncp-{EPOCH_A}-{seq}"), hash.to_string()))
        .collect::<BTreeMap<_, _>>();
    let mut expected_values = BTreeMap::new();
    for actual_id in expected_retained_ids {
        let source_id = if scenario == FaultScenario::NewStreamEpoch {
            (1..=6)
                .find_map(|seq| {
                    (actual_id == &format!("ncp-{EPOCH_B}-{seq}"))
                        .then(|| format!("ncp-{EPOCH_A}-{}", seq + 6))
                })
                .unwrap_or_else(|| actual_id.clone())
        } else {
            actual_id.clone()
        };
        let expected_hash = baseline_values.get(&source_id).ok_or_else(|| {
            anyhow::anyhow!(
                "clean fixture is missing content oracle source {source_id:?} for {}",
                scenario.id()
            )
        })?;
        expected_values.insert(actual_id.clone(), expected_hash.clone());
    }
    Ok(expected_values)
}

fn apply_cross_scenario_payload_expectations(
    scenarios: &mut [ScenarioReport],
) -> anyhow::Result<()> {
    let baseline = scenarios
        .iter()
        .find(|scenario| scenario.scenario == FaultScenario::CleanBaseline)
        .ok_or_else(|| anyhow::anyhow!("frozen suite is missing its clean baseline"))?;
    let baseline_hash = baseline
        .replay_equivalence
        .first_scientific_payload_hash
        .clone();
    for scenario in scenarios {
        let expected_values = expected_sample_value_hashes(
            scenario.scenario,
            &scenario.sample_sets.expected_retained_ids,
        )?;
        let mismatched_or_missing_ids = expected_values
            .iter()
            .filter(|(id, expected)| {
                scenario.sample_content.actual_value_hashes.get(*id) != Some(*expected)
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        let unexpected_ids = scenario
            .sample_content
            .actual_value_hashes
            .keys()
            .filter(|id| !expected_values.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        let content_matches = mismatched_or_missing_ids.is_empty() && unexpected_ids.is_empty();
        scenario.sample_content.expected_value_hashes = expected_values;
        scenario.sample_content.mismatched_or_missing_ids = mismatched_or_missing_ids;
        scenario.sample_content.unexpected_ids = unexpected_ids;
        scenario.sample_content.matches_clean_fixture_oracle = content_matches;
        if !content_matches {
            scenario.expectation_failures.push(
                "retained sample values differ from the clean-fixture content oracle".to_string(),
            );
            scenario.verdict = ScenarioVerdict::Mismatched;
        }
        if scenario.scientific_payload_expectation
            == ScientificPayloadExpectation::SameAsCleanBaseline
        {
            let matches =
                scenario.replay_equivalence.first_scientific_payload_hash == baseline_hash;
            scenario.scientific_payload_matches_clean_baseline = Some(matches);
            if !matches {
                scenario
                    .expectation_failures
                    .push("scientific sample payload differs from the clean baseline".to_string());
                scenario.verdict = ScenarioVerdict::Mismatched;
            }
        }
    }
    Ok(())
}

fn summarize_scenario_assessments(scenarios: &[ScenarioReport]) -> ScenarioAssessmentSummary {
    let not_assessable = scenarios
        .iter()
        .filter(|scenario| scenario.verdict == ScenarioVerdict::NotAssessable)
        .count();
    ScenarioAssessmentSummary {
        total: scenarios.len(),
        assessed: scenarios.len().saturating_sub(not_assessable),
        not_assessable,
        matched: scenarios
            .iter()
            .filter(|scenario| scenario.verdict == ScenarioVerdict::Matched)
            .count(),
        matched_known_limitations: scenarios
            .iter()
            .filter(|scenario| scenario.verdict == ScenarioVerdict::MatchedKnownLimitation)
            .count(),
        mismatched: scenarios
            .iter()
            .filter(|scenario| scenario.verdict == ScenarioVerdict::Mismatched)
            .count(),
    }
}

fn ensure_directory(path: &Path) -> anyhow::Result<bool> {
    let created = match std::fs::create_dir(path) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = std::fs::symlink_metadata(path).with_context(|| {
                format!("failed to inspect existing directory {}", path.display())
            })?;
            if !metadata.file_type().is_dir() {
                anyhow::bail!(
                    "existing observatory path {} must be a real directory",
                    path.display()
                );
            }
            false
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to create directory {}", path.display()));
        }
    };
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .with_context(|| format!("failed to fsync directory {}", path.display()))?;
        if created {
            let parent = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .with_context(|| format!("failed to fsync directory {}", parent.display()))?;
        }
    }
    Ok(created)
}

fn create_output_root(path: &Path) -> anyhow::Result<(PathBuf, bool)> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("observatory output directory must not be empty");
    }
    let created = ensure_directory(path)?;
    let root = std::fs::canonicalize(path)
        .with_context(|| format!("failed to pin observatory output root {}", path.display()))?;
    Ok((root, created))
}

fn atomic_write_bytes(path: &Path, bytes: &[u8], context: &str) -> anyhow::Result<()> {
    let write_result = atomic_write_with(path, |writer| {
        writer
            .write_all(bytes)
            .with_context(|| format!("failed to write {context}"))
    });
    if write_result.is_ok() {
        return Ok(());
    }
    let existing = read_bounded(path, bytes.len()).with_context(|| {
        format!(
            "failed to recover possibly installed {context} after write error: {}",
            write_result.unwrap_err()
        )
    })?;
    if existing != bytes {
        anyhow::bail!("existing {context} differs from the exact retry bytes");
    }
    sync_installed_file(path)
        .with_context(|| format!("failed to re-establish durability for exact {context} retry"))
}

fn provenance_assessment(consumer: &ConsumerProvenance) -> ProvenanceAssessment {
    let mut recorded_fields = vec![
        "ncp_tag",
        "ncp_revision",
        "ncp_wire",
        "ncp_contract_hash",
        "consumer_revision",
        "trace_origin",
        "producer_revision",
        "realm",
        "session_id",
        "exact_routes",
        "source_ordinals",
        "logical_delivery_slots",
        "raw_payload_sha256",
        "scenario_schedule_hash",
        "terminal_boundary",
        "artifact_sha256",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let mut missing_fields = Vec::new();
    if consumer.worktree_clean.is_some() {
        recorded_fields.push("consumer_worktree_state".to_string());
    } else {
        missing_fields.push("consumer_worktree_state".to_string());
    }
    if consumer.lockfile_sha256.is_some() {
        recorded_fields.push("consumer_lockfile_sha256".to_string());
    } else {
        missing_fields.push("consumer_lockfile_sha256".to_string());
    }
    if consumer.executable_sha256.is_some() {
        recorded_fields.push("consumer_executable_sha256".to_string());
    } else {
        missing_fields.push("consumer_executable_sha256".to_string());
    }
    if consumer.build_revision.is_some() {
        recorded_fields.push("consumer_build_revision".to_string());
    } else {
        missing_fields.push("consumer_build_revision".to_string());
    }
    if consumer.build_worktree_clean.is_some() {
        recorded_fields.push("consumer_build_worktree_state".to_string());
    } else {
        missing_fields.push("consumer_build_worktree_state".to_string());
    }
    let explicitly_unassessed_fields = [
        "wall_clock_receipt_timestamp",
        "source_clock_uncertainty",
        "clock_synchronization",
        "negotiated_qos",
        "live_reconnect_history",
        "authenticated_peer_identity",
        "acl_enforcement",
        "live_control_timing_noninterference",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    ProvenanceAssessment {
        required_field_count: recorded_fields.len()
            + missing_fields.len()
            + explicitly_unassessed_fields.len(),
        recorded_fields,
        missing_fields,
        explicitly_unassessed_fields,
    }
}

fn report_limitations(evidence_level: EvidenceLevel) -> Vec<String> {
    let mut limitations = vec![
        "logical slots and delivery order are not wall-clock latency".to_string(),
        "manifest-only omissions are not native observer detections".to_string(),
        "trace truncation detects one partial boundary while its wholly omitted tail remains manifest-only; it is not a live disconnect/reconnect experiment".to_string(),
        "logical receipt-pause slots are annotations only and do not drive replay timing"
            .to_string(),
        "the declared secure-profile label exercises report claim guarding only; no configuration or transport is opened".to_string(),
        "offline read-only execution does not establish live control-timing noninterference"
            .to_string(),
        "no external producer, intervention, outcome, or live Engram path is exercised"
            .to_string(),
        "no PID estimate is requested and no population/measure/estimator/application gate changes"
            .to_string(),
    ];
    if evidence_level == EvidenceLevel::FixtureSpecificLocalExecutionReproducibilityUnqualified {
        limitations.push(
            "runtime/build commit agreement, clean runtime/build worktrees, lockfile hash, and executable hash were not all established; this execution is below E3-style reproducibility"
                .to_string(),
        );
    }
    limitations
}

fn artifact_manifest_hash(scenarios: &[ScenarioReport]) -> anyhow::Result<String> {
    let artifacts = scenarios
        .iter()
        .flat_map(|scenario| {
            std::iter::once(&scenario.compiled_schedule_artifact)
                .chain(scenario.replay_artifacts.iter())
                .map(move |artifact| (scenario.scenario, artifact))
        })
        .collect::<Vec<_>>();
    pid_runlog::canonical_json_hash_v2(&artifacts)
        .context("failed to hash scenario artifact manifest")
}

fn build_outer_runlog(
    report: &ObservatoryReport,
    report_identity: &ArtifactIdentity,
    trace_identity: &ArtifactIdentity,
    root: &Path,
    max_bytes: usize,
) -> anyhow::Result<Vec<u8>> {
    let run_id = format!("ncp-fault-observatory-{}", &report.trace_exact_sha256[..12]);
    let scenario_configs = report
        .scenarios
        .iter()
        .map(|scenario| {
            let frozen_expectation = scenario_expectation(scenario.scenario);
            serde_json::json!({
                "scenario": scenario.scenario,
                "schedule_hash": scenario.schedule_hash,
                "observability": scenario.detection.observability,
                "expected_verdict": frozen_expectation.matched_verdict,
            })
        })
        .collect::<Vec<_>>();
    let config = serde_json::json!({
        "component": "ncp-fault-observatory",
        "scope": TRACE_SCOPE,
        "report_schema_version": REPORT_SCHEMA_VERSION,
        "consumer": report.consumer,
        "ncp": {
            "tag": report.ncp_tag,
            "revision": report.ncp_revision,
            "wire": report.ncp_wire,
            "contract_hash": report.ncp_contract_hash,
        },
        "trace_exact_sha256": report.trace_exact_sha256,
        "trace_canonical_sha256": report.trace_canonical_sha256,
        "limits": report.limits,
        "observer_limits": report.observer_limits,
        "scenarios": scenario_configs,
        "logical_time_only": true,
        "live_transport": "not_exercised",
        "security": "not_assessed",
        "live_control_timing_noninterference": "not_assessed",
    });
    let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
    let mut events = vec![
        RunLogEvent::RunStarted {
            schema_version: RUN_LOG_SCHEMA_VERSION,
            run_id: run_id.clone(),
            timestamp_ns: 0,
            config_hash: config_hash.clone(),
            metadata: BTreeMap::from([
                ("source".to_string(), "ncp_fault_observatory".to_string()),
                ("authority".to_string(), "read_only_offline".to_string()),
            ]),
        },
        RunLogEvent::ConfigLogged {
            timestamp_ns: 0,
            config_hash,
            config,
        },
    ];
    let mut step = 0_u64;
    for scenario in &report.scenarios {
        for delivery in &scenario.delivery_journal {
            step = step
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("outer run-log step overflow"))?;
            let signal_codes = delivery
                .observer_deltas
                .iter()
                .map(|delta| format!("{:?}:{}", delta.code, delta.amount))
                .collect::<Vec<_>>()
                .join(",");
            events.push(RunLogEvent::FrameObserved {
                step,
                timestamp_ns: step,
                observation_hash: Some(delivery.payload_sha256.clone()),
                metadata: BTreeMap::from([
                    ("scenario".to_string(), scenario.scenario.id().to_string()),
                    (
                        "source_ordinal".to_string(),
                        delivery.source_ordinal.to_string(),
                    ),
                    (
                        "delivery_ordinal".to_string(),
                        delivery.delivery_ordinal.to_string(),
                    ),
                    (
                        "logical_slot".to_string(),
                        delivery.logical_slot.to_string(),
                    ),
                    ("routing_key".to_string(), delivery.routing_key.clone()),
                    (
                        "expected_plane".to_string(),
                        format!("{:?}", delivery.expected_plane).to_lowercase(),
                    ),
                    ("fault_ids".to_string(), delivery.fault_ids.join(",")),
                    (
                        "disposition".to_string(),
                        format!("{:?}", delivery.disposition).to_lowercase(),
                    ),
                    ("observer_signal_deltas".to_string(), signal_codes),
                ]),
            });
        }
        step = step
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("outer run-log step overflow"))?;
        events.push(RunLogEvent::FrameObserved {
            step,
            timestamp_ns: step,
            observation_hash: Some(
                scenario
                    .replay_equivalence
                    .first_outcome_fingerprint
                    .clone(),
            ),
            metadata: BTreeMap::from([
                ("scenario".to_string(), scenario.scenario.id().to_string()),
                ("record_kind".to_string(), "scenario_outcome".to_string()),
                (
                    "observer_integrity".to_string(),
                    scenario.observer_integrity.clone(),
                ),
                ("verdict".to_string(), format!("{:?}", scenario.verdict)),
                (
                    "kept_samples".to_string(),
                    scenario.observer_stats.kept_samples.to_string(),
                ),
            ]),
        });
    }
    let mut artifact_events = vec![trace_identity.clone(), report_identity.clone()];
    artifact_events.extend(report.scenarios.iter().flat_map(|scenario| {
        std::iter::once(scenario.compiled_schedule_artifact.clone())
            .chain(scenario.replay_artifacts.iter().cloned())
    }));
    for artifact in artifact_events {
        step = step
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("outer run-log step overflow"))?;
        let path = root.join(&artifact.relative_uri);
        let uri = std::fs::canonicalize(&path)?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("outer artifact path is not UTF-8"))?
            .to_string();
        events.push(RunLogEvent::ArtifactLogged {
            timestamp_ns: step,
            name: format!(
                "ncp_observatory_{}",
                artifact.relative_uri.replace(['/', '.'], "_")
            ),
            kind: "observatory_evidence".to_string(),
            uri,
            sha256: Some(artifact.sha256),
            metadata: BTreeMap::from([("bytes".to_string(), artifact.bytes.to_string())]),
        });
    }
    events.push(RunLogEvent::RunEnded {
        run_id,
        timestamp_ns: step.saturating_add(1),
        status: if report.all_expectations_matched {
            RunStatus::Succeeded
        } else {
            RunStatus::Failed
        },
        message: Some(format!(
            "{} deterministic offline NCP scenarios; expectations_matched={}",
            report.scenarios.len(),
            report.all_expectations_matched
        )),
    });
    let validation = validate_events(&events)?;
    if !validation.is_valid() {
        anyhow::bail!("constructed observatory run log is schema-invalid");
    }
    let mut writer = RunLogWriter::new(BoundedBuffer::new(max_bytes));
    for event in &events {
        writer.append(event)?;
    }
    writer.flush()?;
    Ok(writer.into_inner().into_inner())
}

struct OuterPublicationPaths<'a> {
    report: &'a Path,
    runlog: &'a Path,
    trace: &'a Path,
    receipt: &'a Path,
}

struct VerifiedOuterArtifacts {
    report: ObservatoryReport,
    manifest_hash: String,
    report_bytes: Vec<u8>,
    runlog_bytes: Vec<u8>,
    trace_bytes: Vec<u8>,
}

fn verify_compiled_schedule(
    bytes: &[u8],
    trace_exact_sha256: &str,
    report: &ScenarioReport,
    limits: ObservatoryLimits,
    observer_limits: ObserverLimits,
) -> anyhow::Result<()> {
    if !strict_json_preflight(bytes) {
        anyhow::bail!("compiled fault schedule is not strict JSON");
    }
    let artifact: CompiledScheduleArtifact =
        serde_json::from_slice(bytes).context("failed to decode compiled fault schedule")?;
    if artifact.schema_version != REPORT_SCHEMA_VERSION
        || artifact.scope != TRACE_SCOPE
        || artifact.trace_exact_sha256 != trace_exact_sha256
        || artifact.scenario != report.scenario
        || artifact.schedule_sha256 != report.schedule_hash
        || report.builtin_golden_schedule_hash != report.scenario.golden_schedule_sha256()
        || report.matches_builtin_golden_schedule
            != (report.schedule_hash == report.builtin_golden_schedule_hash)
        || artifact.injection_truth != report.injection_truth
        || artifact.receipts.len() > limits.max_scheduled_deliveries
    {
        anyhow::bail!("compiled fault schedule identity does not match its report");
    }
    let mut receipts = Vec::new();
    receipts
        .try_reserve_exact(artifact.receipts.len())
        .context("failed to reserve verified compiled schedule")?;
    let mut payload_bytes = 0_usize;
    for (index, receipt) in artifact.receipts.iter().enumerate() {
        if receipt.delivery_ordinal != u64::try_from(index).unwrap_or(u64::MAX) {
            anyhow::bail!("compiled fault schedule delivery ordinals are not contiguous");
        }
        let per_receipt_limit = if report.scenario == FaultScenario::OversizedPayload {
            observer_limits
                .max_wire_frame_bytes
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("oversized verification limit overflow"))?
        } else {
            observer_limits.max_wire_frame_bytes
        };
        let payload = hex_decode(&receipt.payload_hex, per_receipt_limit)?;
        payload_bytes = checked_add_usize(
            payload_bytes,
            payload.len(),
            "verified schedule payload bytes",
        )?;
        if payload_bytes > limits.max_scheduled_payload_bytes
            || pid_runlog::sha256_hex(&payload) != receipt.payload_sha256
            || typed_receipt_hash(receipt.expected_plane, &payload)? != receipt.typed_receipt_sha256
        {
            anyhow::bail!("compiled fault schedule payload identity is invalid");
        }
        receipts.push(ScheduledReceipt {
            source_ordinal: receipt.source_ordinal,
            logical_slot: receipt.logical_slot,
            routing_key: receipt.routing_key.clone(),
            expected_plane: receipt.expected_plane,
            source_epoch: receipt.source_epoch.clone(),
            source_seq: receipt.source_seq,
            payload,
            fault_ids: receipt.fault_ids.clone(),
        });
    }
    let recomputed = canonical_schedule_hash(report.scenario, &report.injection_truth, &receipts)?;
    if recomputed != report.schedule_hash || report.delivery_journal.len() != receipts.len() {
        anyhow::bail!("compiled fault schedule hash/journal count does not match its report");
    }
    for (scheduled, journal) in receipts.iter().zip(&report.delivery_journal) {
        if scheduled.source_ordinal != journal.source_ordinal
            || scheduled.logical_slot != journal.logical_slot
            || scheduled.routing_key != journal.routing_key
            || scheduled.expected_plane != journal.expected_plane
            || pid_runlog::sha256_hex(&scheduled.payload) != journal.payload_sha256
            || scheduled.fault_ids != journal.fault_ids
        {
            anyhow::bail!("compiled fault schedule differs from its delivery journal");
        }
    }
    if report.scenario == FaultScenario::OversizedPayload {
        let oversized = receipts
            .iter()
            .zip(&report.delivery_journal)
            .filter(|(scheduled, journal)| {
                scheduled.payload.len() > observer_limits.max_wire_frame_bytes
                    && journal.disposition == ReceiptDisposition::CallbackOversizedDropped
            })
            .count();
        if oversized != 1 {
            anyhow::bail!("compiled oversized scenario lacks its one callback rejection");
        }
    }
    Ok(())
}

fn verify_no_unbound_entries(
    root: &Path,
    allowed_files: &BTreeSet<String>,
    allowed_directories: &BTreeSet<String>,
) -> anyhow::Result<()> {
    let mut pending = vec![root.to_path_buf()];
    let mut entries_seen = 0_usize;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .with_context(|| format!("failed to enumerate {}", directory.display()))?
        {
            let entry = entry?;
            entries_seen = entries_seen
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("observatory tree entry count overflow"))?;
            if entries_seen > 512 {
                anyhow::bail!("observatory output tree exceeds its finite entry ceiling");
            }
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)?;
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("observatory output path is not UTF-8"))?
                .to_string();
            if metadata.file_type().is_dir() {
                if !allowed_directories.contains(&relative) {
                    anyhow::bail!("unbound observatory directory {relative:?}");
                }
                pending.push(path);
            } else if metadata.file_type().is_file() {
                if !allowed_files.contains(&relative) {
                    anyhow::bail!("unbound observatory artifact {relative:?}");
                }
            } else {
                anyhow::bail!("observatory output contains a symlink or non-regular entry");
            }
        }
    }
    Ok(())
}

fn reserved_temp_target(relative: &Path, allowed_files: &BTreeSet<String>) -> Option<String> {
    let name = relative.file_name()?.to_str()?;
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    for allowed in allowed_files {
        let allowed_path = Path::new(allowed);
        if allowed_path.parent().unwrap_or_else(|| Path::new("")) != parent {
            continue;
        }
        let target_name = allowed_path.file_name()?.to_str()?;
        let prefix = format!(".{target_name}.tmp-");
        let Some(suffix) = name.strip_prefix(&prefix) else {
            continue;
        };
        let mut parts = suffix.split('-');
        let pid = parts.next()?;
        let nonce = parts.next()?;
        if parts.next().is_none()
            && !pid.is_empty()
            && !nonce.is_empty()
            && pid.bytes().all(|byte| byte.is_ascii_digit())
            && nonce.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Some(allowed.clone());
        }
    }
    None
}

/// Remove only regular files in the writer's exact reserved temporary-name
/// grammar. The observatory output root is a managed namespace: callers must
/// not create `.<allowed-target>.tmp-<pid>-<nonce>` entries. Explicit recovery
/// treats those names as disposable because process death may leave their bytes
/// partial or empty. Every final target is independently reconstructed before
/// any removal; the one not-yet-installed outer receipt is allowed only after
/// this run has independently computed its exact pending bytes. Public
/// verification never calls this mutating path.
fn recover_reserved_temporary_entries(
    root: &Path,
    allowed_files: &BTreeSet<String>,
    allowed_directories: &BTreeSet<String>,
    pending_receipt_bytes: Option<&[u8]>,
) -> anyhow::Result<()> {
    let mut pending = vec![root.to_path_buf()];
    let mut candidates = Vec::new();
    let mut entries_seen = 0_usize;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            entries_seen = entries_seen
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("observatory recovery entry count overflow"))?;
            if entries_seen > 512 {
                anyhow::bail!("observatory recovery tree exceeds its finite entry ceiling");
            }
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)?;
            let relative = path.strip_prefix(root)?;
            let relative_text = relative
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("observatory temporary path is not UTF-8"))?;
            if metadata.file_type().is_dir() {
                if allowed_directories.contains(relative_text) {
                    pending.push(path);
                } else {
                    anyhow::bail!("unbound observatory directory {relative_text:?}");
                }
                continue;
            }
            if allowed_files.contains(relative_text) {
                if !metadata.file_type().is_file() {
                    anyhow::bail!("observatory output contains a symlink or non-regular entry");
                }
                continue;
            }
            let reserved_target = metadata
                .file_type()
                .is_file()
                .then(|| reserved_temp_target(relative, allowed_files))
                .flatten();
            if reserved_target.is_some() {
                let target = reserved_target
                    .ok_or_else(|| anyhow::anyhow!("reserved temporary target disappeared"))?;
                candidates.push((
                    path.clone(),
                    directory.clone(),
                    relative_text.to_string(),
                    target,
                ));
            } else if metadata.file_type().is_file() {
                anyhow::bail!("unbound observatory artifact {relative_text:?}");
            } else {
                anyhow::bail!("observatory output contains a symlink or non-regular entry");
            }
        }
    }
    for (_, _, relative, target) in &candidates {
        let target_path = root.join(target);
        let reconstructed = std::fs::symlink_metadata(&target_path)
            .is_ok_and(|metadata| metadata.file_type().is_file());
        let pending_outer_receipt = target == "observatory.publication.json"
            && pending_receipt_bytes.is_some_and(|bytes| !bytes.is_empty());
        if !reconstructed && !pending_outer_receipt {
            anyhow::bail!(
                "reserved temporary artifact {relative:?} has no independently reconstructed target"
            );
        }
    }
    let mut touched_directories = BTreeSet::new();
    for (path, directory, relative, _) in candidates {
        std::fs::remove_file(&path).with_context(|| {
            format!("failed to remove stale observatory temporary {relative:?}")
        })?;
        touched_directories.insert(directory);
    }
    #[cfg(unix)]
    for directory in touched_directories {
        File::open(&directory)
            .and_then(|file| file.sync_all())
            .with_context(|| {
                format!(
                    "failed to fsync recovered observatory directory {}",
                    directory.display()
                )
            })?;
    }
    Ok(())
}

fn reconstruct_replay_run(
    root: &Path,
    scenario: FaultScenario,
    replay: &str,
    scheduled: &ScheduledScenario,
    report: &ScenarioReport,
    limits: ObservatoryLimits,
    observer_limits: ObserverLimits,
) -> anyhow::Result<ReplayRun> {
    let replay_root = root.join(scenario.id()).join(replay);
    let dataset_path = replay_root.join("dataset.json");
    let runlog_path = replay_root.join("runlog.jsonl");
    let receipt_path = publication_receipt_path(&dataset_path);
    let outcome_path = replay_root.join("outcome.json");
    let outcome_bytes = read_bounded(&outcome_path, limits.max_replay_outcome_bytes)?;
    if !strict_json_preflight(&outcome_bytes) {
        anyhow::bail!("replay outcome record is not strict JSON");
    }
    let recorded: ReplayOutcomeRecord =
        serde_json::from_slice(&outcome_bytes).context("failed to decode replay outcome record")?;
    if recorded.schema_version != REPORT_SCHEMA_VERSION
        || recorded.scope != TRACE_SCOPE
        || recorded.scenario != scenario
        || recorded.schedule_sha256 != scheduled.schedule_hash
        || recorded.replay != replay
    {
        anyhow::bail!("replay outcome record identity is inconsistent");
    }
    let verified = verify_replay_bundle(
        &dataset_path,
        &runlog_path,
        &report.observer_integrity,
        observer_limits,
    )?;
    if verified.dataset.run_id != format!("ncp-fault-observatory-{}", scenario.id())
        || verified.dataset.source != "ncp"
        || verified.dataset.model != "synthetic-wire-0.8"
        || verified.dataset.task != "protocol-fault-observatory"
    {
        anyhow::bail!("replay dataset identity does not match its frozen scenario");
    }
    let dataset_semantic_hash = dataset_semantic_hash(&verified.dataset)?;
    let scientific_payload_hash = scientific_payload_hash(&verified.dataset)?;
    let normalized_logical_trace_hash_v3 =
        normalized_runlog_hash(&verified.events, &dataset_semantic_hash)?;
    let validation = validate_events(&verified.events)?;
    let sample_value_hashes = verified
        .dataset
        .samples
        .iter()
        .map(|sample| Ok((sample.sample_id.clone(), sample_value_hash(sample)?)))
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
    let mut sample_ids = verified
        .dataset
        .samples
        .iter()
        .map(|sample| sample.sample_id.clone())
        .collect::<Vec<_>>();
    sample_ids.sort();
    let outcome_fingerprint = outcome_fingerprint(
        scenario,
        &scheduled.schedule_hash,
        scheduled.truth.terminal,
        &recorded.stats,
        &recorded.counters,
        &recorded.journal,
        &recorded.finalization_deltas,
        &sample_ids,
        &sample_value_hashes,
        &dataset_semantic_hash,
        &scientific_payload_hash,
        &normalized_logical_trace_hash_v3,
        validation.errors,
        validation.warnings,
    )?;
    if recorded.sample_ids != sample_ids
        || recorded.sample_value_hashes != sample_value_hashes
        || recorded.dataset_semantic_hash != dataset_semantic_hash
        || recorded.scientific_payload_hash != scientific_payload_hash
        || recorded.normalized_logical_trace_hash_v3 != normalized_logical_trace_hash_v3
        || recorded.outcome_fingerprint != outcome_fingerprint
        || recorded.runlog_validation_errors != validation.errors
        || recorded.runlog_validation_warnings != validation.warnings
    {
        anyhow::bail!("replay outcome record does not reconstruct from its durable bundle");
    }
    let artifacts = vec![
        artifact_identity_from_bytes(root, &dataset_path, &verified.dataset_bytes)?,
        artifact_identity_from_bytes(root, &runlog_path, &verified.runlog_bytes)?,
        artifact_identity_from_bytes(root, &receipt_path, &verified.receipt_bytes)?,
        artifact_identity_from_bytes(root, &outcome_path, &outcome_bytes)?,
    ];
    Ok(ReplayRun {
        stats: recorded.stats,
        counters: recorded.counters,
        journal: recorded.journal,
        finalization_deltas: recorded.finalization_deltas,
        sample_ids,
        sample_value_hashes,
        dataset_semantic_hash,
        scientific_payload_hash,
        normalized_logical_trace_hash_v3,
        outcome_fingerprint,
        runlog_validation_errors: validation.errors,
        runlog_validation_warnings: validation.warnings,
        artifacts,
    })
}

fn verify_outer_artifacts(
    report_path: &Path,
    runlog_path: &Path,
    trace_path: &Path,
    limits: ObservatoryLimits,
    recover_temporary_entries: bool,
    pending_receipt_bytes: Option<&[u8]>,
) -> anyhow::Result<VerifiedOuterArtifacts> {
    let root = report_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("observatory report has no parent directory"))?;
    let report_bytes = read_bounded(report_path, limits.max_report_bytes)?;
    if !strict_json_preflight(&report_bytes) {
        anyhow::bail!("outer observatory report is not strict JSON");
    }
    let report: ObservatoryReport =
        serde_json::from_slice(&report_bytes).context("failed to decode observatory report")?;
    let report_identity = artifact_identity_from_bytes(root, report_path, &report_bytes)?;
    let expected_scenarios = FaultScenario::ALL.into_iter().collect::<Vec<_>>();
    let actual_scenarios = report
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario)
        .collect::<Vec<_>>();
    if report.schema_version != REPORT_SCHEMA_VERSION
        || report.scope != TRACE_SCOPE
        || report.evidence_level != report.consumer.evidence_level()
        || report.execution_status != ExecutionStatus::Completed
        || report.establishes_e4
        || report.completes_ec1
        || report.establishes_live_engram_validation
        || report.establishes_security_validation
        || report.changes_pid_gates
        || report.ncp_tag != NCP_TAG
        || report.ncp_revision != NCP_RELEASE_REVISION
        || report.ncp_wire != NCP_VERSION
        || report.ncp_contract_hash != CONTRACT_HASH
        || report.limits != limits
        || report.observer_limits != ObserverLimits::default()
        || report.provenance != provenance_assessment(&report.consumer)
        || report.limitations != report_limitations(report.evidence_level)
        || report.canonical_runlog_relative_uri != "observatory-runlog.jsonl"
        || report.publication_receipt_relative_uri != "observatory.publication.json"
        || actual_scenarios != expected_scenarios
        || report.scenario_assessments != summarize_scenario_assessments(&report.scenarios)
        || report.all_expectations_matched
            != report
                .scenarios
                .iter()
                .all(|scenario| scenario.expectation_failures.is_empty())
    {
        anyhow::bail!("outer observatory report contract is inconsistent");
    }
    let expected_report_name = format!("observatory-report-{}.json", report_identity.sha256);
    if report_path.file_name().and_then(|name| name.to_str()) != Some(expected_report_name.as_str())
        || runlog_path != root.join(&report.canonical_runlog_relative_uri)
    {
        anyhow::bail!("outer report or run-log path does not match its content identity");
    }
    let trace_bytes = read_bounded(trace_path, limits.max_trace_file_bytes)?;
    let actual_trace = artifact_identity_from_bytes(root, trace_path, &trace_bytes)?;
    let validated_trace = validate_trace_bytes(trace_bytes.clone(), limits, report.observer_limits)
        .context("published trace failed frozen-baseline validation")?;
    let expected_trace_name = format!("wire-trace-{}.json", validated_trace.exact_sha256);
    if actual_trace != report.trace_artifact
        || report.trace_exact_sha256 != validated_trace.exact_sha256
        || report.trace_canonical_sha256 != validated_trace.canonical_sha256
        || trace_path.file_name().and_then(|name| name.to_str())
            != Some(expected_trace_name.as_str())
    {
        anyhow::bail!("published trace does not match the report artifact identity");
    }
    let mut reconstructed_scenarios = Vec::new();
    reconstructed_scenarios
        .try_reserve_exact(report.scenarios.len())
        .context("failed to reserve reconstructed scenario reports")?;
    let mut allowed_files = BTreeSet::from([
        report_identity.relative_uri.clone(),
        actual_trace.relative_uri.clone(),
        report.canonical_runlog_relative_uri.clone(),
        report.publication_receipt_relative_uri.clone(),
    ]);
    let mut allowed_directories = BTreeSet::new();
    for stored in &report.scenarios {
        let scenario = stored.scenario;
        let scenario_id = scenario.id();
        allowed_directories.insert(scenario_id.to_string());
        let scheduled =
            compile_scenario(&validated_trace, scenario, limits, report.observer_limits)?;
        let schedule_path = root.join(scenario_id).join("compiled-schedule.json");
        let schedule_bytes = read_bounded(&schedule_path, limits.max_compiled_schedule_bytes)?;
        let expected_schedule_bytes =
            compiled_schedule_bytes(&validated_trace, &scheduled, limits)?;
        if schedule_bytes != expected_schedule_bytes {
            anyhow::bail!("compiled scenario {scenario_id} differs from the frozen compiler");
        }
        let schedule_identity =
            artifact_identity_from_bytes(root, &schedule_path, &schedule_bytes)?;
        if schedule_identity != stored.compiled_schedule_artifact {
            anyhow::bail!("compiled scenario {scenario_id} failed exact artifact verification");
        }
        verify_compiled_schedule(
            &schedule_bytes,
            &validated_trace.exact_sha256,
            stored,
            limits,
            report.observer_limits,
        )?;
        allowed_files.insert(schedule_identity.relative_uri.clone());
        for replay in ["replay_a", "replay_b"] {
            allowed_directories.insert(format!("{scenario_id}/{replay}"));
        }
        let first = reconstruct_replay_run(
            root,
            scenario,
            "replay_a",
            &scheduled,
            stored,
            limits,
            report.observer_limits,
        )?;
        let second = reconstruct_replay_run(
            root,
            scenario,
            "replay_b",
            &scheduled,
            stored,
            limits,
            report.observer_limits,
        )?;
        for artifact in first.artifacts.iter().chain(&second.artifacts) {
            allowed_files.insert(artifact.relative_uri.clone());
        }
        reconstructed_scenarios.push(assemble_scenario_report(
            scheduled,
            schedule_identity,
            first,
            second,
        ));
    }
    apply_cross_scenario_payload_expectations(&mut reconstructed_scenarios)?;
    if reconstructed_scenarios != report.scenarios {
        anyhow::bail!("published scenario reports do not reconstruct from their evidence");
    }
    let runlog_bytes = read_bounded(runlog_path, limits.max_outer_runlog_bytes)?;
    let outer_events = pid_runlog::read_events(std::io::Cursor::new(&runlog_bytes))?;
    let validation = validate_events(&outer_events)?;
    if !validation.is_valid() {
        anyhow::bail!("published outer observatory run log is invalid");
    }
    let expected_runlog = build_outer_runlog(
        &report,
        &report_identity,
        &actual_trace,
        root,
        limits.max_outer_runlog_bytes,
    )?;
    if runlog_bytes != expected_runlog {
        anyhow::bail!("published outer run log does not reconstruct from the report");
    }
    if recover_temporary_entries {
        recover_reserved_temporary_entries(
            root,
            &allowed_files,
            &allowed_directories,
            pending_receipt_bytes,
        )?;
    }
    verify_no_unbound_entries(root, &allowed_files, &allowed_directories)?;
    let manifest_hash = artifact_manifest_hash(&reconstructed_scenarios)?;
    Ok(VerifiedOuterArtifacts {
        report,
        manifest_hash,
        report_bytes,
        runlog_bytes,
        trace_bytes,
    })
}

fn verify_outer_publication_snapshot(
    paths: OuterPublicationPaths<'_>,
    receipt_bytes: &[u8],
    expected_manifest_hash: Option<&str>,
    limits: ObservatoryLimits,
    recover_temporary_entries: bool,
) -> anyhow::Result<VerifiedOuterArtifacts> {
    if !strict_json_preflight(receipt_bytes) {
        anyhow::bail!("outer publication receipt is not strict JSON");
    }
    let receipt: ObservatoryPublicationReceipt = serde_json::from_slice(receipt_bytes)?;
    let canonical_uri = |path: &Path| -> anyhow::Result<String> {
        std::fs::canonicalize(path)?
            .to_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("outer publication path is not UTF-8"))
    };
    let verified = verify_outer_artifacts(
        paths.report,
        paths.runlog,
        paths.trace,
        limits,
        recover_temporary_entries,
        None,
    )?;
    if receipt.schema_version != PUBLICATION_SCHEMA_VERSION
        || !receipt.committed
        || paths.receipt
            != paths
                .report
                .parent()
                .ok_or_else(|| anyhow::anyhow!("outer report has no publication root"))?
                .join("observatory.publication.json")
        || receipt.report_uri != canonical_uri(paths.report)?
        || receipt.runlog_uri != canonical_uri(paths.runlog)?
        || receipt.trace_uri != canonical_uri(paths.trace)?
        || receipt.report_sha256 != pid_runlog::sha256_hex(&verified.report_bytes)
        || receipt.runlog_sha256 != pid_runlog::sha256_hex(&verified.runlog_bytes)
        || receipt.trace_sha256 != pid_runlog::sha256_hex(&verified.trace_bytes)
        || expected_manifest_hash
            .is_some_and(|expected| receipt.scenario_artifact_manifest_sha256 != expected)
        || receipt.scenario_artifact_manifest_sha256 != verified.manifest_hash
        || receipt.all_expectations_matched != verified.report.all_expectations_matched
    {
        anyhow::bail!("outer observatory publication receipt failed exact verification");
    }
    Ok(verified)
}

/// Revalidate an already committed default-limit observatory bundle in place.
///
/// Publication receipts intentionally bind canonical paths, so moving a bundle
/// requires a new, explicitly audited publication rather than silent rebasing.
/// This read-only verifier snapshots every compiled schedule, replay outcome,
/// replay dataset, inner run log, inner receipt, outer report, trace, outer run
/// log, and the receipt installed last. It fails on stale writer scratch files;
/// only an explicit run/recovery path may remove those reserved entries.
pub fn verify_observatory_publication(root: &Path) -> anyhow::Result<ObservatoryOutcome> {
    let limits = ObservatoryLimits::default().validate()?;
    let canonical_root = std::fs::canonicalize(root)
        .with_context(|| format!("failed to locate observatory root {}", root.display()))?;
    if !std::fs::metadata(&canonical_root)?.is_dir() {
        anyhow::bail!("observatory publication root is not a directory");
    }
    let receipt_path = canonical_root.join("observatory.publication.json");
    let receipt_bytes = read_bounded(&receipt_path, MAX_PUBLICATION_RECEIPT_BYTES)?;
    if !strict_json_preflight(&receipt_bytes) {
        anyhow::bail!("outer publication receipt is not strict JSON");
    }
    let receipt: ObservatoryPublicationReceipt = serde_json::from_slice(&receipt_bytes)
        .context("failed to decode outer publication receipt")?;
    let bound_top_level_path = |label: &str, uri: &str| -> anyhow::Result<PathBuf> {
        let canonical = std::fs::canonicalize(uri)
            .with_context(|| format!("failed to locate receipt-bound {label}"))?;
        if canonical.parent() != Some(canonical_root.as_path()) {
            anyhow::bail!("receipt-bound {label} is not a top-level artifact of this publication");
        }
        Ok(canonical)
    };
    let report_path = bound_top_level_path("report", &receipt.report_uri)?;
    let runlog_path = bound_top_level_path("run log", &receipt.runlog_uri)?;
    let trace_path = bound_top_level_path("trace", &receipt.trace_uri)?;
    let expected_report_name = format!("observatory-report-{}.json", receipt.report_sha256);
    let expected_trace_name = format!("wire-trace-{}.json", receipt.trace_sha256);
    if report_path.file_name().and_then(|name| name.to_str()) != Some(expected_report_name.as_str())
        || runlog_path != canonical_root.join("observatory-runlog.jsonl")
        || trace_path.file_name().and_then(|name| name.to_str())
            != Some(expected_trace_name.as_str())
    {
        anyhow::bail!("outer publication receipt names an unexpected artifact path");
    }
    let verified = verify_outer_publication_snapshot(
        OuterPublicationPaths {
            report: &report_path,
            runlog: &runlog_path,
            trace: &trace_path,
            receipt: &receipt_path,
        },
        &receipt_bytes,
        None,
        limits,
        false,
    )?;
    Ok(ObservatoryOutcome {
        report_path,
        runlog_path,
        receipt_path,
        all_expectations_matched: verified.report.all_expectations_matched,
    })
}

fn recover_committed_publication(
    root: &Path,
    validated: &ValidatedTrace,
    consumer: &ConsumerProvenance,
    limits: ObservatoryLimits,
) -> anyhow::Result<ObservatoryOutcome> {
    let receipt_path = root.join("observatory.publication.json");
    let receipt_bytes = read_bounded(&receipt_path, MAX_PUBLICATION_RECEIPT_BYTES)?;
    if !strict_json_preflight(&receipt_bytes) {
        anyhow::bail!("existing observatory receipt is not strict JSON");
    }
    let receipt: ObservatoryPublicationReceipt = serde_json::from_slice(&receipt_bytes)?;
    let report_path = PathBuf::from(&receipt.report_uri);
    let runlog_path = root.join("observatory-runlog.jsonl");
    let trace_path = root.join(format!("wire-trace-{}.json", validated.exact_sha256));
    let canonical_root = std::fs::canonicalize(root)?;
    for path in [&report_path, &runlog_path, &trace_path, &receipt_path] {
        let canonical = std::fs::canonicalize(path)?;
        if !canonical.starts_with(&canonical_root) {
            anyhow::bail!("existing committed observatory artifact escaped its output root");
        }
    }
    let verified = verify_outer_publication_snapshot(
        OuterPublicationPaths {
            report: &report_path,
            runlog: &runlog_path,
            trace: &trace_path,
            receipt: &receipt_path,
        },
        &receipt_bytes,
        None,
        limits,
        true,
    )?;
    if verified.report.consumer != *consumer
        || verified.report.trace_exact_sha256 != validated.exact_sha256
        || verified.report.trace_canonical_sha256 != validated.canonical_sha256
    {
        anyhow::bail!("existing committed observatory bundle is not an exact retry");
    }
    sync_installed_file(&receipt_path)?;
    Ok(ObservatoryOutcome {
        report_path,
        runlog_path,
        receipt_path,
        all_expectations_matched: verified.report.all_expectations_matched,
    })
}

/// Run the complete deterministic offline suite and publish its receipt-last
/// evidence bundle into a new directory.
///
/// `trace_path=None` uses the checked synthetic wire-0.8 fixture. A supplied
/// trace must be a bounded regular file and an exact typed-semantic variant of
/// that frozen v1 baseline; raw JSON representation may differ. The frozen
/// scenario registry injects all faults only after that validation.
pub fn run_observatory(
    output_dir: impl AsRef<Path>,
    trace_path: Option<&Path>,
    consumer: ConsumerProvenance,
) -> anyhow::Result<ObservatoryOutcome> {
    run_observatory_with_limits(
        output_dir.as_ref(),
        trace_path,
        consumer,
        ObservatoryLimits::default(),
        ObserverLimits::default(),
    )
}

fn run_observatory_with_limits(
    output_dir: &Path,
    trace_path: Option<&Path>,
    consumer: ConsumerProvenance,
    limits: ObservatoryLimits,
    observer_limits: ObserverLimits,
) -> anyhow::Result<ObservatoryOutcome> {
    let limits = limits.validate()?;
    if FaultScenario::ALL.len() > limits.max_scenarios {
        anyhow::bail!("frozen scenario registry exceeds the configured scenario ceiling");
    }
    let exact_trace_bytes = match trace_path {
        Some(path) => bounded_regular_file(path, limits.max_trace_file_bytes)?,
        None => {
            let trace = synthetic_wire_trace(consumer.revision.clone())?;
            serialize_json_pretty_bounded(&trace, limits.max_trace_file_bytes)?
        }
    };
    let validated = validate_trace_bytes(exact_trace_bytes, limits, observer_limits)?;
    preflight_suite_resources(&validated, limits, observer_limits)?;
    let (root, created) = create_output_root(output_dir)?;
    if !created && root.join("observatory.publication.json").exists() {
        return recover_committed_publication(&root, &validated, &consumer, limits);
    }
    let trace_name = format!("wire-trace-{}.json", validated.exact_sha256);
    let trace_output = root.join(&trace_name);
    atomic_write_bytes(
        &trace_output,
        &validated.exact_bytes,
        "observatory input trace",
    )?;
    let trace_identity = artifact_identity(&root, &trace_output, limits.max_trace_file_bytes)?;

    let mut scenarios = Vec::new();
    scenarios
        .try_reserve_exact(FaultScenario::ALL.len())
        .context("failed to reserve bounded scenario reports")?;
    for scenario in FaultScenario::ALL {
        let scheduled = compile_scenario(&validated, scenario, limits, observer_limits)?;
        let scenario_dir = root.join(scenario.id());
        ensure_directory(&scenario_dir)?;
        let schedule_path = scenario_dir.join("compiled-schedule.json");
        let schedule_bytes = compiled_schedule_bytes(&validated, &scheduled, limits)?;
        atomic_write_bytes(
            &schedule_path,
            &schedule_bytes,
            "compiled observatory schedule",
        )?;
        let schedule_artifact =
            artifact_identity(&root, &schedule_path, limits.max_compiled_schedule_bytes)?;
        let first = replay_once(
            &root,
            &validated,
            &scheduled,
            "replay_a",
            limits,
            observer_limits,
        )?;
        let second = replay_once(
            &root,
            &validated,
            &scheduled,
            "replay_b",
            limits,
            observer_limits,
        )?;
        scenarios.push(assemble_scenario_report(
            scheduled,
            schedule_artifact,
            first,
            second,
        ));
    }
    apply_cross_scenario_payload_expectations(&mut scenarios)?;
    let all_expectations_matched = scenarios
        .iter()
        .all(|scenario| scenario.expectation_failures.is_empty());
    let scenario_assessments = summarize_scenario_assessments(&scenarios);
    let evidence_level = consumer.evidence_level();
    let limitations = report_limitations(evidence_level);
    let report = ObservatoryReport {
        schema_version: REPORT_SCHEMA_VERSION,
        scope: TRACE_SCOPE.to_string(),
        evidence_level,
        execution_status: ExecutionStatus::Completed,
        establishes_e4: false,
        completes_ec1: false,
        establishes_live_engram_validation: false,
        establishes_security_validation: false,
        changes_pid_gates: false,
        consumer: consumer.clone(),
        ncp_tag: NCP_TAG.to_string(),
        ncp_revision: NCP_RELEASE_REVISION.to_string(),
        ncp_wire: NCP_VERSION.to_string(),
        ncp_contract_hash: CONTRACT_HASH.to_string(),
        trace_exact_sha256: validated.exact_sha256.clone(),
        trace_canonical_sha256: validated.canonical_sha256.clone(),
        trace_artifact: trace_identity.clone(),
        limits,
        observer_limits,
        provenance: provenance_assessment(&consumer),
        scenarios,
        scenario_assessments,
        all_expectations_matched,
        canonical_runlog_relative_uri: "observatory-runlog.jsonl".to_string(),
        publication_receipt_relative_uri: "observatory.publication.json".to_string(),
        limitations,
    };
    let report_bytes = serialize_json_pretty_bounded(&report, limits.max_report_bytes)?;
    let report_sha256 = pid_runlog::sha256_hex(&report_bytes);
    let report_path = root.join(format!("observatory-report-{report_sha256}.json"));
    atomic_write_bytes(&report_path, &report_bytes, "observatory report")?;
    let report_identity = artifact_identity(&root, &report_path, limits.max_report_bytes)?;
    let runlog_path = root.join(&report.canonical_runlog_relative_uri);
    let runlog_bytes = build_outer_runlog(
        &report,
        &report_identity,
        &trace_identity,
        &root,
        limits.max_outer_runlog_bytes,
    )?;
    atomic_write_bytes(&runlog_path, &runlog_bytes, "observatory canonical run log")?;
    let scenario_artifact_manifest_sha256 = artifact_manifest_hash(&report.scenarios)?;
    let receipt_path = root.join(&report.publication_receipt_relative_uri);
    let canonical_uri = |path: &Path| -> anyhow::Result<String> {
        std::fs::canonicalize(path)?
            .to_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("observatory publication path is not UTF-8"))
    };
    let receipt = ObservatoryPublicationReceipt {
        schema_version: PUBLICATION_SCHEMA_VERSION,
        committed: true,
        report_uri: canonical_uri(&report_path)?,
        report_sha256: report_identity.sha256.clone(),
        runlog_uri: canonical_uri(&runlog_path)?,
        runlog_sha256: pid_runlog::sha256_hex(&runlog_bytes),
        trace_uri: canonical_uri(&trace_output)?,
        trace_sha256: trace_identity.sha256.clone(),
        scenario_artifact_manifest_sha256: scenario_artifact_manifest_sha256.clone(),
        all_expectations_matched,
    };
    let receipt_bytes = serialize_json_pretty_bounded(&receipt, MAX_PUBLICATION_RECEIPT_BYTES)?;
    let verified = verify_outer_artifacts(
        &report_path,
        &runlog_path,
        &trace_output,
        limits,
        true,
        Some(&receipt_bytes),
    )?;
    if verified.report != report || verified.manifest_hash != scenario_artifact_manifest_sha256 {
        anyhow::bail!("pre-commit observatory artifact verification changed the report contract");
    }
    atomic_write_bytes(
        &receipt_path,
        &receipt_bytes,
        "observatory publication receipt",
    )?;
    let installed_receipt_bytes = read_bounded(&receipt_path, MAX_PUBLICATION_RECEIPT_BYTES)?;
    verify_outer_publication_snapshot(
        OuterPublicationPaths {
            report: &report_path,
            runlog: &runlog_path,
            trace: &trace_output,
            receipt: &receipt_path,
        },
        &installed_receipt_bytes,
        Some(&scenario_artifact_manifest_sha256),
        limits,
        false,
    )?;
    Ok(ObservatoryOutcome {
        report_path,
        runlog_path,
        receipt_path,
        all_expectations_matched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_path(name: &str) -> PathBuf {
        let nonce = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "ncp_observatory_{name}_{}_{nonce}",
            std::process::id()
        ))
    }

    fn test_consumer() -> ConsumerProvenance {
        ConsumerProvenance::with_build_attestation(
            "11".repeat(20),
            Some(true),
            Some("00".repeat(32)),
            Some("22".repeat(32)),
            Some("11".repeat(20)),
            Some(true),
        )
        .unwrap()
    }

    fn encoded_trace() -> Vec<u8> {
        serialize_json_pretty_bounded(
            &synthetic_wire_trace("test-revision").unwrap(),
            ObservatoryLimits::default().max_trace_file_bytes,
        )
        .unwrap()
    }

    #[test]
    fn synthetic_trace_is_complete_strict_and_current_wire() {
        let limits = ObservatoryLimits::default();
        let validated =
            validate_trace_bytes(encoded_trace(), limits, ObserverLimits::default()).unwrap();
        assert_eq!(validated.receipts.len(), BASELINE_TICKS * 3);
        assert_eq!(validated.baseline_sample_ids.len(), BASELINE_TICKS);
        assert_eq!(validated.trace.ncp_wire, "0.8");
        assert_eq!(validated.trace.ncp_contract_hash, CONTRACT_HASH);
        assert_eq!(validated.trace.terminal, TraceEndReason::ProducerClose);
        assert!(validated
            .trace
            .receipts
            .iter()
            .all(|receipt| is_lower_hex(&receipt.typed_receipt_sha256, 32)));
    }

    #[test]
    fn trace_rejects_raw_and_typed_hash_tampering_and_wrong_terminal() {
        let limits = ObservatoryLimits::default();
        let observer_limits = ObserverLimits::default();
        for mutation in 0..3 {
            let mut trace = synthetic_wire_trace("test-revision").unwrap();
            match mutation {
                0 => trace.receipts[0].payload_sha256 = "11".repeat(32),
                1 => trace.receipts[0].typed_receipt_sha256 = "22".repeat(32),
                2 => trace.terminal = TraceEndReason::TraceTruncation,
                _ => unreachable!(),
            }
            let bytes = serialize_json_pretty_bounded(&trace, limits.max_trace_file_bytes).unwrap();
            assert!(validate_trace_bytes(bytes, limits, observer_limits).is_err());
        }
    }

    #[test]
    fn raw_representation_hash_changes_while_typed_receipt_hash_stays_equal() {
        let limits = ObservatoryLimits::default();
        let observer_limits = ObserverLimits::default();
        let original = validate_trace_bytes(encoded_trace(), limits, observer_limits).unwrap();
        let mut trace = original.trace.clone();
        let receipt_index = trace
            .receipts
            .iter()
            .position(|receipt| {
                receipt.expected_plane == IngressPlane::Command && receipt.source_seq == 5
            })
            .unwrap();
        let raw = hex_decode(
            &trace.receipts[receipt_index].payload_hex,
            observer_limits.max_wire_frame_bytes,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        let mut alternate = b" \n".to_vec();
        alternate.extend(serde_json::to_vec_pretty(&value).unwrap());
        assert_ne!(raw, alternate);
        trace.receipts[receipt_index].payload_hex = hex_encode(&alternate);
        trace.receipts[receipt_index].payload_sha256 = pid_runlog::sha256_hex(&alternate);
        let alternate_bytes =
            serialize_json_pretty_bounded(&trace, limits.max_trace_file_bytes).unwrap();
        let alternate = validate_trace_bytes(alternate_bytes, limits, observer_limits).unwrap();
        assert_ne!(original.exact_sha256, alternate.exact_sha256);
        assert_eq!(
            original.trace.receipts[receipt_index].typed_receipt_sha256,
            alternate.trace.receipts[receipt_index].typed_receipt_sha256
        );
        let duplicate = compile_scenario(
            &alternate,
            FaultScenario::DuplicateJsonKey,
            limits,
            observer_limits,
        )
        .unwrap();
        assert!(!duplicate.matches_builtin_golden_schedule);
    }

    #[test]
    fn complete_but_semantically_different_trace_rejects_before_publication() {
        let limits = ObservatoryLimits::default();
        let observer_limits = ObserverLimits::default();
        let mut trace = synthetic_wire_trace("different-valid-baseline").unwrap();
        let payload = hex_decode(
            &trace.receipts[0].payload_hex,
            observer_limits.max_wire_frame_bytes,
        )
        .unwrap();
        let changed = mutate_json(&payload, |value| {
            let channels = object_mut(value, "channels")?;
            let pose = channels
                .get_mut("pose")
                .and_then(serde_json::Value::as_object_mut)
                .ok_or_else(|| anyhow::anyhow!("synthetic pose channel is absent"))?;
            pose.insert("data".to_string(), serde_json::json!([123.0, 1.0]));
            Ok(())
        })
        .unwrap();
        trace.receipts[0].payload_hex = hex_encode(&changed);
        trace.receipts[0].payload_sha256 = pid_runlog::sha256_hex(&changed);
        trace.receipts[0].typed_receipt_sha256 = typed_receipt_hash(IngressPlane::Sensor, &changed)
            .unwrap()
            .unwrap();
        let input = unique_path("different_valid_trace.json");
        let output = unique_path("different_valid_trace_output");
        std::fs::write(
            &input,
            serialize_json_pretty_bounded(&trace, limits.max_trace_file_bytes).unwrap(),
        )
        .unwrap();
        let error = run_observatory(&output, Some(&input), test_consumer()).unwrap_err();
        assert!(error.to_string().contains("frozen v1 semantic baseline"));
        assert!(!output.exists());
        std::fs::remove_file(input).unwrap();
    }

    #[test]
    fn compiled_schedules_are_bounded_and_hash_stable() {
        let limits = ObservatoryLimits::default();
        let observer_limits = ObserverLimits::default();
        let trace = validate_trace_bytes(encoded_trace(), limits, observer_limits).unwrap();
        for scenario in FaultScenario::ALL {
            let first = compile_scenario(&trace, scenario, limits, observer_limits).unwrap();
            let second = compile_scenario(&trace, scenario, limits, observer_limits).unwrap();
            assert_eq!(first.schedule_hash, second.schedule_hash);
            assert_eq!(first.schedule_hash, scenario.golden_schedule_sha256());
            assert!(first.matches_builtin_golden_schedule);
            assert!(first.receipts.len() <= limits.max_scheduled_deliveries);
        }
        let whole = compile_scenario(
            &trace,
            FaultScenario::WholeTickOmission,
            limits,
            observer_limits,
        )
        .unwrap();
        assert_eq!(whole.truth.dropped_source_ordinals.len(), 3);
        let exact = compile_scenario(
            &trace,
            FaultScenario::ExactRedelivery,
            limits,
            observer_limits,
        )
        .unwrap();
        assert_eq!(exact.truth.duplicated_source_ordinals.len(), 6);
    }

    #[test]
    fn reviewed_sample_hashes_bind_the_fixture_values_independently() {
        for (seq, golden_hash) in (1_i64..=12).zip(GOLDEN_CLEAN_SAMPLE_VALUE_HASHES) {
            let sample = OfflineVldaSample {
                sample_id: format!("ncp-{EPOCH_A}-{seq}"),
                episode_id: Some("synthetic-observatory".to_string()),
                v: vec![seq as f64, 1.0],
                l: vec![0.25],
                d: vec![seq as f64 / 10.0],
                a: vec![seq as f64 / 100.0],
                labels: BTreeMap::from([("success".to_string(), serde_json::json!(true))]),
                metadata: BTreeMap::from([
                    ("epoch".to_string(), EPOCH_A.to_string()),
                    ("seq".to_string(), seq.to_string()),
                    ("source".to_string(), "ncp".to_string()),
                    ("l_source".to_string(), "channel".to_string()),
                    ("d_source".to_string(), "source".to_string()),
                ]),
            };
            assert_eq!(sample_value_hash(&sample).unwrap(), golden_hash);
        }
    }

    #[test]
    fn evidence_level_downgrades_dirty_or_unlocked_consumers() {
        assert_eq!(
            test_consumer().evidence_level(),
            EvidenceLevel::FixtureSpecificE3StyleLocalEvidenceOnly
        );
        for consumer in [
            ConsumerProvenance::with_build_attestation(
                "11".repeat(20),
                Some(false),
                Some("00".repeat(32)),
                Some("22".repeat(32)),
                Some("11".repeat(20)),
                Some(true),
            )
            .unwrap(),
            ConsumerProvenance::with_build_attestation(
                "not_recorded",
                Some(true),
                Some("00".repeat(32)),
                Some("22".repeat(32)),
                Some("11".repeat(20)),
                Some(true),
            )
            .unwrap(),
            ConsumerProvenance::with_build_attestation(
                "11".repeat(20),
                Some(true),
                None,
                Some("22".repeat(32)),
                Some("11".repeat(20)),
                Some(true),
            )
            .unwrap(),
            ConsumerProvenance::with_build_attestation(
                "11".repeat(20),
                Some(true),
                Some("00".repeat(32)),
                None,
                Some("11".repeat(20)),
                Some(true),
            )
            .unwrap(),
            ConsumerProvenance::with_build_attestation(
                "11".repeat(20),
                Some(true),
                Some("00".repeat(32)),
                Some("22".repeat(32)),
                Some("33".repeat(20)),
                Some(true),
            )
            .unwrap(),
        ] {
            assert_eq!(
                consumer.evidence_level(),
                EvidenceLevel::FixtureSpecificLocalExecutionReproducibilityUnqualified
            );
        }
    }

    #[test]
    fn complete_suite_publishes_replay_equivalent_honest_report() {
        let output = unique_path("complete_suite");
        let outcome = run_observatory(&output, None, test_consumer()).unwrap();
        assert!(outcome.all_expectations_matched);
        let report_bytes = read_bounded(
            &outcome.report_path,
            ObservatoryLimits::default().max_report_bytes,
        )
        .unwrap();
        assert!(strict_json_preflight(&report_bytes));
        let report: ObservatoryReport = serde_json::from_slice(&report_bytes).unwrap();
        assert_eq!(report.scenarios.len(), FaultScenario::ALL.len());
        assert_eq!(
            report.evidence_level,
            EvidenceLevel::FixtureSpecificE3StyleLocalEvidenceOnly
        );
        assert_eq!(report.execution_status, ExecutionStatus::Completed);
        assert!(!report.establishes_e4);
        assert!(!report.completes_ec1);
        assert!(!report.establishes_live_engram_validation);
        assert!(!report.establishes_security_validation);
        assert!(!report.changes_pid_gates);
        assert_eq!(
            report.scenario_assessments,
            ScenarioAssessmentSummary {
                total: 18,
                assessed: 16,
                not_assessable: 2,
                matched: 15,
                matched_known_limitations: 1,
                mismatched: 0,
            }
        );
        assert!(report
            .scenarios
            .iter()
            .all(|scenario| scenario.replay_equivalence.equal));
        assert!(report.scenarios.iter().all(|scenario| {
            scenario.matches_builtin_golden_schedule
                && scenario.sample_content.matches_clean_fixture_oracle
                && scenario.injection_truth.logical_slots_are_annotations_only
                && scenario.control_scope.action_plane_publications == 0
                && scenario.control_scope.agent_bridge_requests == 0
                && scenario.control_scope.live_control_timing_noninterference
                    == AssessmentStatus::NotAssessed
        }));
        let whole = report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario == FaultScenario::WholeTickOmission)
            .unwrap();
        assert_eq!(whole.observer_integrity, "complete");
        assert_eq!(whole.observer_stats.kept_samples, 11);
        assert_eq!(
            whole.detection.observability,
            ObservabilityClass::ManifestOnly
        );
        assert_eq!(
            whole.detection.native_response,
            NativeObserverResponse::NotDetected
        );
        assert_eq!(whole.verdict, ScenarioVerdict::MatchedKnownLimitation);
        assert!(whole
            .injection_truth
            .native_visible_dropped_source_ordinals
            .is_empty());
        assert_eq!(
            whole
                .injection_truth
                .manifest_only_dropped_source_ordinals
                .len(),
            3
        );
        let exact = report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario == FaultScenario::ExactRedelivery)
            .unwrap();
        assert_eq!(exact.observer_stats.redelivered_frames_dropped, 6);
        assert_eq!(exact.scientific_payload_matches_clean_baseline, Some(true));
        let post_close_conflict = report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario == FaultScenario::ConflictingDuplicateAfterClosure)
            .unwrap();
        assert_eq!(post_close_conflict.observer_integrity, "invalid");
        assert_eq!(post_close_conflict.observer_stats.kept_samples, 12);
        let logical_pause = report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario == FaultScenario::LogicalReceiptPause)
            .unwrap();
        assert_eq!(logical_pause.verdict, ScenarioVerdict::NotAssessable);
        assert_eq!(
            logical_pause.detection.wall_clock_latency,
            AssessmentStatus::NotAssessed
        );
        assert_eq!(
            logical_pause.detection.native_response,
            NativeObserverResponse::NotAssessable
        );
        let truncation = report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario == FaultScenario::TraceTruncation)
            .unwrap();
        assert_eq!(
            truncation.detection.observability,
            ObservabilityClass::MixedVisibleAndManifestOnly
        );
        assert_eq!(
            truncation
                .injection_truth
                .native_visible_dropped_source_ordinals,
            vec![19]
        );
        assert_eq!(
            truncation
                .injection_truth
                .manifest_only_dropped_source_ordinals
                .len(),
            16
        );
        let security = report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario == FaultScenario::SecurityProfileClaimGuard)
            .unwrap();
        assert_eq!(security.verdict, ScenarioVerdict::NotAssessable);
        assert_eq!(
            security.security.configuration_condition,
            TransportCondition::SecureConfigurationDeclared
        );
        assert!(!security.security.transport_exercised);
        assert!(!security.security.configuration_loaded);
        assert!(!security.security.configuration_selected);
        assert!(security.security.declared_profile_label_only);
        assert_eq!(
            security.security.peer_authentication,
            AssessmentStatus::NotAssessed
        );
        assert_eq!(
            security.security.acl_enforcement,
            AssessmentStatus::NotAssessed
        );
        assert_eq!(
            security.security.security_validation,
            AssessmentStatus::NotEstablished
        );
        assert!(pid_runlog::validate_events_from_path(&outcome.runlog_path)
            .unwrap()
            .is_valid());
        assert_eq!(verify_observatory_publication(&output).unwrap(), outcome);
        let mut runlogs = vec![outcome.runlog_path.clone()];
        for scenario in &report.scenarios {
            runlogs.extend(
                scenario
                    .replay_artifacts
                    .iter()
                    .filter(|artifact| artifact.relative_uri.ends_with("runlog.jsonl"))
                    .map(|artifact| output.join(&artifact.relative_uri)),
            );
        }
        assert_eq!(runlogs.len(), 1 + FaultScenario::ALL.len() * 2);
        for runlog in runlogs {
            let events = pid_runlog::read_events_from_path(&runlog).unwrap();
            assert!(events.iter().all(|event| !matches!(
                event,
                RunLogEvent::PidMetric { .. } | RunLogEvent::PidEstimate { .. }
            )));
        }
        let exact_retry = run_observatory(&output, None, test_consumer()).unwrap();
        assert_eq!(exact_retry, outcome);

        let crafted_temporary = output.join("clean_baseline/replay_a/.dataset.json.tmp-999-1");
        std::fs::write(&crafted_temporary, b"incomplete writer scratch").unwrap();
        let read_only_error = verify_observatory_publication(&output).unwrap_err();
        assert!(format!("{read_only_error:#}").contains("unbound observatory artifact"));
        assert!(crafted_temporary.is_file());
        let recovered_partial = run_observatory(&output, None, test_consumer()).unwrap();
        assert_eq!(recovered_partial, outcome);
        assert!(!crafted_temporary.exists());

        let stale_receipt = output.join(".observatory.publication.json.tmp-999-2");
        std::fs::write(&stale_receipt, b"partial receipt").unwrap();
        let missing_for_retry = output.join("clean_baseline/replay_a/dataset.json");
        let stale_dataset = output.join("clean_baseline/replay_a/.dataset.json.tmp-999-3");
        std::fs::write(&stale_dataset, []).unwrap();
        std::fs::remove_file(&outcome.receipt_path).unwrap();
        std::fs::remove_file(&missing_for_retry).unwrap();
        let resumed = run_observatory(&output, None, test_consumer()).unwrap();
        assert_eq!(resumed, outcome);
        assert!(missing_for_retry.is_file());
        assert!(!stale_dataset.exists());
        assert!(!stale_receipt.exists());

        let tampered = output.join("exact_redelivery/replay_b/dataset.json");
        std::fs::remove_file(&tampered).unwrap();
        let public_error = verify_observatory_publication(&output).unwrap_err();
        assert!(format!("{public_error:#}").contains("exact_redelivery/replay_b/dataset.json"));
        std::fs::remove_dir_all(&output).unwrap();
    }

    #[test]
    fn trace_resource_limits_fail_before_output_publication() {
        let output = unique_path("tiny_limit");
        let limits = ObservatoryLimits {
            max_trace_receipts: 1,
            ..ObservatoryLimits::default()
        };
        let error = run_observatory_with_limits(
            &output,
            None,
            test_consumer(),
            limits,
            ObserverLimits::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("receipt count"));
        assert!(!output.exists());
    }

    #[test]
    fn aggregate_schedule_limits_fail_before_output_publication() {
        let output = unique_path("tiny_aggregate_limit");
        let limits = ObservatoryLimits {
            max_total_journal_records: 1,
            ..ObservatoryLimits::default()
        };
        let error = run_observatory_with_limits(
            &output,
            None,
            test_consumer(),
            limits,
            ObserverLimits::default(),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("suite delivery journals exceed"));
        assert!(!output.exists());
    }

    #[test]
    fn nested_unknown_trace_fields_fail_closed() {
        let limits = ObservatoryLimits::default();
        let observer_limits = ObserverLimits::default();
        let mut value: serde_json::Value = serde_json::from_slice(&encoded_trace()).unwrap();
        value
            .get_mut("observer_limits")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap()
            .insert("future_unreviewed_limit".to_string(), serde_json::json!(1));
        let bytes = serde_json::to_vec(&value).unwrap();
        let error = validate_trace_bytes(bytes, limits, observer_limits).unwrap_err();
        assert!(format!("{error:#}").contains("unknown field"));
    }

    #[cfg(unix)]
    #[test]
    fn trace_input_rejects_symlinks() {
        let input = unique_path("trace_input.json");
        let link = unique_path("trace_link.json");
        let output = unique_path("trace_symlink_output");
        std::fs::write(&input, encoded_trace()).unwrap();
        std::os::unix::fs::symlink(&input, &link).unwrap();
        let error = run_observatory(&output, Some(&link), test_consumer()).unwrap_err();
        assert!(format!("{error:#}").contains("symlinks are rejected"));
        assert!(!output.exists());
        std::fs::remove_file(link).unwrap();
        std::fs::remove_file(input).unwrap();
    }
}
