//! # ncp-observer — prisoma's passive NCP tap
//!
//! A future conforming NEST/Engram publisher could become an optional `(V,L,D,A)`
//! source. Unlike the critical-path `experiments/safe_adapter` reference producer,
//! this integration remains exploratory, off-path, and PID-disabled by default.
//! This crate is a **read-only observer**: it subscribes to the NCP data-plane keys
//! (`…/session/{id}/{sensor,command,observation}`) and converts each closed-loop
//! tick into an `OfflineVldaSample`, writing both
//!
//! 1. an `OfflineVldaDataset` JSON artifact. The harness can run non-PID
//!    diagnostics/baselines after verifying its publication receipt. Continuous
//!    KSG/shared-exclusions requests abstain until a real producer supplies honest
//!    population support; quantized discrete `I_min` remains a non-evidentiary
//!    diagnostic with population `NotEvaluated` and application `Blocked`, and
//! 2. canonical run-log events (the source of truth): one `EmbeddingContract`
//!    declaring the `(V,L,D,A)` variables, an `EmbeddingCaptured` per kept sample,
//!    a `LabelObserved` per success label, and — at finalize — an `ArtifactLogged`
//!    registering the dataset artifact (uri + sha256) so the run log can locate
//!    and verify the data it describes.
//!
//! It honors prisoma's three rules — the run log is the source of truth, the
//! observer never drives anything (the Agent Bridge stays the only control
//! plane), and the NCP-specific mapping lives here in prisoma, not in Engram.
//!
//! ## Mapping (V, L, D, A)
//! - **V** (vision/sensory) ← `SensorFrame` channels (all but the language and
//!   success channels), flattened.
//! - **L** (language/instruction) ← a named `SensorFrame` channel (default
//!   `instruction`). A tick with **no** language channel yields an empty `L`,
//!   and empty-axis ticks are **excluded from the artifact** (counted, reported
//!   at finalize) because `pid-offline-harness` requires nonempty, consistent-dim
//!   axes. A zero/hash backfill would fabricate a language axis and is not a
//!   conformance repair; a no-language session yields an empty artifact with a
//!   loud exclusion count.
//! - **D** (dynamics / internal state) ← `ObservationFrame` record-port readouts —
//!   neural state before the motor head. Its world-model status is untested and
//!   requires separate architecture evidence plus held-out probes/interventions.
//! - **A** (action) ← `CommandFrame` channels, flattened.
//!
//! ## Alignment (the correctness rule)
//! V and A are joined on the driving **sensor `StreamPosition`** (`{epoch, seq}`)
//! — wire 0.8's typed source-correlation key. A `SensorFrame` IS the origin, so it
//! contributes its OWN `stream`; a `CommandFrame.source` echoes the
//! `SensorFrame.stream` it was computed from, and an `ObservationFrame.source`
//! echoes the driving `SensorFrame.stream` too. A sample therefore pairs the
//! action (and the neural readout) with the exact sensor that produced it — on the
//! full `{epoch, seq}`, never on the bare seq (a sensor restart reuses seqs under a
//! fresh epoch) and never by arrival time (which the DROP QoS on the perception
//! plane would corrupt). Because the observation plane rides a lower-priority QoS
//! class than the action plane, a tick's D routinely *arrives after* its command:
//! completed ticks are held for a **reorder grace window** (`REORDER_GRACE` newer
//! source seqs) before emission, so a late source-stamped readout still claims its
//! own tick. Passenger command/D receipts may also overtake the first sensor of a
//! fresh epoch; they remain isolated under the full key until that sensor
//! authorizes transition. After a tick emits, its artifact row and canonical event
//! are immutable: exact complete-frame redelivery is idempotent and conflicting
//! evidence invalidates the capture without patching the row. An
//! observation with NO `source` is the pull/RPC form and is dropped (source
//! ABSENCE, not the retired `seq == 0` sentinel); there is no recency fallback or
//! future-D pairing.
//!
//! ## Sessions, epochs, and unstamped frames
//! - An unstamped join position is dropped and counted: a `SensorFrame` whose own
//!   `stream` is unset (`stream.seq < 1`), a `CommandFrame` with no `source`
//!   (open-loop / uncorrelatable), and an `ObservationFrame` with no `source`
//!   (pull/RPC) all fail the source-correlation join.
//! - Wire 0.8 carries a canonical `stream.epoch` per incarnation, so a sensor
//!   restart is detected DIRECTLY as a change of the active epoch (retiring the
//!   0.7 `RESET_MARGIN` seq-distance heuristic). On an epoch transition complete
//!   old-epoch ticks are flushed, unrelated partial state is discarded while
//!   already-buffered receipts for the newly authorized epoch are retained, the old epoch is retired (a late
//!   straggler from it is dropped, never re-triggering a reset), and `sample_id`s
//!   carry the epoch (`ncp-{epoch}-{seq}`) so they stay unique across restarts.
//! - Every frame's payload `session_id` must equal the explicitly bound capture
//!   session. Passenger generations are retained per key, while the live
//!   `session.generation` is locked only by the first validated authorizing
//!   sensor; stale/foreign-session frames are dropped and counted.
//! - In-flight state is bounded by `MAX_INFLIGHT` plus a global resident-element
//!   ceiling, with **insertion-order
//!   (FIFO) eviction** — never lowest-key eviction, which would starve new
//!   low-seq ticks after a restart.
//!
//! ## Honesty provenance (per-sample `metadata`, mirrored into the run log)
//! Every kept sample carries provenance markers so a degraded axis is never
//! silently presented as real data — and the same markers are mirrored into the
//! `EmbeddingCaptured` metadata so the run log records them independently:
//! - `l_source` = `"channel"` (the language channel was present; empty-L ticks
//!   are excluded, see above).
//! - `d_source` = `"source"` only. Missing or unstamped D never enters an
//!   artifact; those ticks are excluded and counted.
//! - `seq` and `epoch` — the driving sensor `{epoch, seq}` join key, for
//!   reconstruction.
//!
//! `ObserverStats::capture_integrity` grades only visible raw receipts and join
//! state. It does not detect a wholly missing plane tick or attest receipt timing,
//! reconnect/QoS state, clock sync, peer authentication, or live protocol
//! conformance. The deterministic observatory exercises bounded fixture semantics
//! only and preserves those live-transport nonclaims.

use anyhow::Context as _;
use ncp_core::keys::valid_id_segment;
use ncp_core::{
    decode_validated, ChannelValue, CommandFrame, ObservationFrame, SensorFrame, WireFrame,
    CONTRACT_HASH, NCP_VERSION,
};
use pid_runlog::{
    Actor, ActorType, EmbeddingVariableContract, RunLogEvent, RunLogWriter, RunStatus,
    RUN_LOG_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, ErrorKind, Read as _, Write as _};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub mod observatory;

/// One `(V,L,D,A)` sample — mirrors `pid-sim`'s `OfflineVldaSample`; the
/// harness accepts the containing NCP dataset only after publication verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaSample {
    pub sample_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,
    pub v: Vec<f64>,
    pub l: Vec<f64>,
    pub d: Vec<f64>,
    pub a: Vec<f64>,
    #[serde(default)]
    pub labels: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

/// The dataset wrapper `pid-offline-harness` consumes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaDataset {
    pub run_id: String,
    pub source: String,
    pub model: String,
    pub task: String,
    /// Visible-receipt/join integrity grade. This is not an end-to-end delivery
    /// completeness claim; consumers must reject degraded/invalid NCP captures.
    pub capture_integrity: String,
    /// NCP does not infer population support from observed samples. The offline
    /// harness therefore abstains from continuous KSG/shared-exclusions requests
    /// until a real producer supplies an independently justified declaration.
    /// Quantized discrete `I_min` can run only as a non-evidentiary diagnostic:
    /// population is `NotEvaluated` and application remains `Blocked`.
    #[serde(default)]
    pub support: BTreeMap<String, String>,
    /// Commit receipt installed only after both the dataset and canonical run log
    /// are durable. The offline harness verifies it before accepting NCP input.
    pub publication_receipt: String,
    pub samples: Vec<OfflineVldaSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaPublicationReceipt {
    pub schema_version: u32,
    pub committed: bool,
    pub dataset_uri: String,
    pub dataset_sha256: String,
    pub runlog_uri: String,
    pub runlog_sha256: String,
    pub capture_integrity: String,
}

const PUBLICATION_RECEIPT_SCHEMA_VERSION: u32 = 1;
const MAX_PUBLICATION_RECEIPT_BYTES: usize = 64 * 1024;

fn publication_receipt_path(dataset_path: &Path) -> PathBuf {
    let mut path = dataset_path.as_os_str().to_os_string();
    path.push(".publication.json");
    PathBuf::from(path)
}

type StreamKey = (String, i64);

/// How NCP channels map onto `(V,L,D,A)`.
#[derive(Debug, Clone)]
pub struct Mapping {
    /// `SensorFrame` channel carrying the language/instruction embedding.
    pub language_channel: String,
    /// `SensorFrame` channel carrying a per-tick success label (optional).
    pub success_channel: Option<String>,
    /// One NEST trial id → `episode_id`.
    pub episode_id: Option<String>,
}

impl Default for Mapping {
    fn default() -> Self {
        Self {
            language_channel: "instruction".into(),
            success_channel: None,
            episode_id: None,
        }
    }
}

/// Visible-receipt and join-quality counters reported by [`Observer::finalize`].
/// These counters do not prove end-to-end delivery completeness: whole-plane
/// gaps, local receipt timing, reconnect history, negotiated QoS, and clock sync
/// remain unassessed. The deterministic protocol-fault observatory uses logical
/// delivery slots and a separate fixture oracle; it does not turn those missing
/// live-transport measurements into observer-native evidence.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverStats {
    /// Samples retained for the dataset artifact (and preserved across failed
    /// finalization attempts).
    pub kept_samples: usize,
    /// Finalization saw no retained sample. A zero-receipt/zero-row run is not an
    /// analyzable successful capture even when no specific drop was observable.
    pub zero_sample_capture: bool,
    /// Ticks excluded because an axis was empty (per axis). Empty-axis samples
    /// can never pass `pid-offline-harness`'s `validate_dataset`, and one such
    /// sample would poison the whole artifact.
    pub excluded_empty_v: usize,
    pub excluded_empty_l: usize,
    pub excluded_empty_d: usize,
    pub excluded_empty_a: usize,
    /// Ticks excluded because their dims differed from the emitted
    /// `EmbeddingContract` (first kept sample's dims).
    pub dim_mismatch_dropped: usize,
    /// Seq-stamped D readouts that arrived after a tick was closed without a
    /// recorded D receipt and were dropped without mutating an artifact row.
    pub late_d_dropped: usize,
    /// Frames dropped as unstamped/uncorrelatable: a sensor whose own `stream`
    /// is unset, or a command/observation with no `source` (open-loop / pull-RPC).
    pub unstamped_frames_dropped: usize,
    /// Frames dropped because their payload `session_id` did not match the
    /// captured session, or their `session.generation` was a stale/foreign
    /// incarnation (or either identity field was missing).
    pub session_mismatch_dropped: usize,
    /// Late stragglers from an already-retired `stream.epoch` — dropped rather
    /// than thrashing the active stream back to an old incarnation.
    pub retired_epoch_frames_dropped: usize,
    /// Wire frames rejected by the binary's validated decoder before they
    /// reached the observer state machine.
    pub ingress_decode_dropped: u64,
    /// Pull/RPC-form observations (no `source`) rejected by the binary at the
    /// observation-plane medium boundary.
    pub ingress_unstamped_observations_dropped: u64,
    /// Frames rejected because the bounded callback-to-worker handoff was full
    /// or closed.
    pub ingress_handoff_dropped: u64,
    /// Exact full-frame duplicates on any plane, before or after tick closure.
    /// They are idempotent transport redeliveries and never mutate or duplicate
    /// a `(V,L,D,A)` row.
    pub redelivered_frames_dropped: usize,
    /// A second frame for the same plane and `{epoch, seq}` carried different
    /// evidence, before or after closure. The tick is capture-invalid and no
    /// last-write-wins replacement or emitted-row mutation is admitted.
    pub conflicting_duplicates_dropped: usize,
    /// Further evidence for a tick already quarantined by a conflict. Kept
    /// separate from ordinary exact redelivery so the diagnostic is not softened.
    pub quarantined_frames_dropped: usize,
    /// Decoded frames whose timestamps, channel values, shapes, or numeric
    /// contents violated the finite payload contract.
    pub invalid_payloads_dropped: usize,
    /// Never-completed in-flight ticks evicted by the `MAX_INFLIGHT` bound.
    pub evicted_incomplete: usize,
    /// Unclaimed seq-stamped D readouts evicted by the `MAX_INFLIGHT` bound.
    pub evicted_unclaimed_d: usize,
    /// Incomplete V/A joins discarded on an epoch transition or finalization.
    pub incomplete_at_epoch_transition: usize,
    pub incomplete_at_finalize: usize,
    /// Which half of the V↔A join was absent when an incomplete tick was
    /// discarded (includes eviction, transition, capacity, and finalization).
    pub incomplete_missing_sensor: usize,
    pub incomplete_missing_command: usize,
    /// Unclaimed D readouts discarded at an epoch transition or finalization.
    pub unclaimed_d_at_epoch_transition: usize,
    pub unclaimed_d_at_finalize: usize,
    /// Raw wire frames rejected before JSON decode because their byte length
    /// exceeded the declared ingress ceiling.
    pub ingress_oversized_dropped: u64,
    /// Session-glob receipts whose routing key was not one of the three exact
    /// data-plane keys bound for this capture.
    pub ingress_route_mismatch_dropped: u64,
    /// Raw ingress exceeded its finite lifetime frame/byte budget.
    pub ingress_lifetime_limit_dropped: u64,
    /// Frames dropped after a finite capture budget was exhausted.
    pub capture_capacity_dropped: usize,
    /// At least one finite lifetime budget sealed further frame admission.
    pub capture_capacity_reached: bool,
    /// Candidate samples that triggered the sample-count or total-element cap.
    pub sample_limit_dropped: usize,
    pub element_limit_dropped: usize,
    /// A typed frame would have exceeded the global resident in-flight element
    /// ceiling. The capture seals before retaining the candidate vectors.
    pub inflight_element_limit_dropped: usize,
    /// A new epoch was rejected after the finite incarnation budget was used.
    pub epoch_limit_dropped: usize,
    /// A state-machine error stopped the owning capture worker. Finalization is
    /// still attempted so the canonical run log records a failed run.
    pub capture_worker_failures: u64,
    /// Signal or Zenoh teardown failed before finalization completed.
    pub capture_teardown_failures: u64,
    /// Epoch transitions detected (each retires the old incarnation and starts a
    /// new one); the wire-0.8 successor to 0.7's seq-distance reset heuristic.
    pub seq_resets: u32,
}

impl ObserverStats {
    pub fn capture_integrity(&self) -> &'static str {
        if self.conflicting_duplicates_dropped > 0
            || self.invalid_payloads_dropped > 0
            || self.ingress_decode_dropped > 0
            || self.ingress_oversized_dropped > 0
            || self.ingress_route_mismatch_dropped > 0
            || self.ingress_lifetime_limit_dropped > 0
            || self.capture_capacity_dropped > 0
            || self.sample_limit_dropped > 0
            || self.element_limit_dropped > 0
            || self.inflight_element_limit_dropped > 0
            || self.epoch_limit_dropped > 0
            || self.capture_worker_failures > 0
            || self.capture_teardown_failures > 0
        {
            "invalid"
        } else if self.excluded_empty_v > 0
            || self.excluded_empty_l > 0
            || self.excluded_empty_d > 0
            || self.excluded_empty_a > 0
            || self.dim_mismatch_dropped > 0
            || self.late_d_dropped > 0
            || self.unstamped_frames_dropped > 0
            || self.session_mismatch_dropped > 0
            || self.retired_epoch_frames_dropped > 0
            || self.ingress_unstamped_observations_dropped > 0
            || self.ingress_handoff_dropped > 0
            || self.evicted_incomplete > 0
            || self.evicted_unclaimed_d > 0
            || self.incomplete_at_epoch_transition > 0
            || self.incomplete_at_finalize > 0
            || self.unclaimed_d_at_epoch_transition > 0
            || self.unclaimed_d_at_finalize > 0
            || self.zero_sample_capture
        {
            "degraded"
        } else if self.redelivered_frames_dropped > 0
            || self.seq_resets > 0
            || self.capture_capacity_reached
        {
            "complete_with_warning"
        } else {
            "complete"
        }
    }

    fn summary(&self) -> String {
        format!(
            "kept={} zero_sample={} excluded(empty v/l/d/a)={}/{}/{}/{} dim_mismatch={} \
             late_d_dropped={} unstamped={} session_mismatch={} retired_epoch={} \
             ingress(decode/nosource/handoff)={}/{}/{} \
             redelivered={} conflicting_duplicates={} quarantined={} invalid_payloads={} \
             evicted(pending/d)={}/{} incomplete(epoch/final)={}/{} \
             missing(sensor/command)={}/{} unclaimed_d(epoch/final)={}/{} \
             oversized={} route_mismatch={} ingress_lifetime_limit={} \
             capacity(reached/dropped)={}/{} \
             limit(sample/elements/inflight/epochs)={}/{}/{}/{} worker/teardown_failures={}/{} \
             epoch_transitions={}",
            self.kept_samples,
            self.zero_sample_capture,
            self.excluded_empty_v,
            self.excluded_empty_l,
            self.excluded_empty_d,
            self.excluded_empty_a,
            self.dim_mismatch_dropped,
            self.late_d_dropped,
            self.unstamped_frames_dropped,
            self.session_mismatch_dropped,
            self.retired_epoch_frames_dropped,
            self.ingress_decode_dropped,
            self.ingress_unstamped_observations_dropped,
            self.ingress_handoff_dropped,
            self.redelivered_frames_dropped,
            self.conflicting_duplicates_dropped,
            self.quarantined_frames_dropped,
            self.invalid_payloads_dropped,
            self.evicted_incomplete,
            self.evicted_unclaimed_d,
            self.incomplete_at_epoch_transition,
            self.incomplete_at_finalize,
            self.incomplete_missing_sensor,
            self.incomplete_missing_command,
            self.unclaimed_d_at_epoch_transition,
            self.unclaimed_d_at_finalize,
            self.ingress_oversized_dropped,
            self.ingress_route_mismatch_dropped,
            self.ingress_lifetime_limit_dropped,
            self.capture_capacity_reached,
            self.capture_capacity_dropped,
            self.sample_limit_dropped,
            self.element_limit_dropped,
            self.inflight_element_limit_dropped,
            self.epoch_limit_dropped,
            self.capture_worker_failures,
            self.capture_teardown_failures,
            self.seq_resets,
        )
    }
}

/// Finite resource contract for one observer capture. These are software-safety
/// ceilings, not recommended scientific sample sizes or performance claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverLimits {
    pub max_wire_frame_bytes: usize,
    pub max_wire_frames: u64,
    pub max_total_wire_bytes: u64,
    pub max_axis_values: usize,
    /// Global closed-receipt ceiling for the finite capture, retained across
    /// epoch transitions so old conflicting evidence remains classifiable.
    pub max_closed_ticks: usize,
    pub max_inflight_elements: usize,
    pub max_samples: usize,
    pub max_total_sample_elements: usize,
    pub max_artifact_bytes: usize,
    pub max_runlog_bytes: usize,
}

impl Default for ObserverLimits {
    fn default() -> Self {
        Self {
            max_wire_frame_bytes: 1024 * 1024,
            max_wire_frames: 1_000_000,
            max_total_wire_bytes: 8 * 1024 * 1024 * 1024,
            max_axis_values: 65_536,
            max_closed_ticks: 50_000,
            max_inflight_elements: 1_000_000,
            max_samples: 25_000,
            max_total_sample_elements: 10_000_000,
            max_artifact_bytes: 256 * 1024 * 1024,
            max_runlog_bytes: 256 * 1024 * 1024,
        }
    }
}

impl ObserverLimits {
    fn validate(self) -> anyhow::Result<Self> {
        if self.max_wire_frame_bytes == 0
            || self.max_wire_frames == 0
            || self.max_total_wire_bytes == 0
            || self.max_axis_values == 0
            || self.max_closed_ticks == 0
            || self.max_inflight_elements == 0
            || self.max_samples == 0
            || self.max_total_sample_elements == 0
            || self.max_artifact_bytes == 0
            || self.max_runlog_bytes == 0
        {
            anyhow::bail!("observer resource limits must all be positive");
        }
        if self.max_samples > self.max_closed_ticks {
            anyhow::bail!("max_samples must not exceed max_closed_ticks");
        }
        Ok(self)
    }
}

/// Bounded live callback handoff capacity, also recorded by deterministic replay
/// so transport provenance cannot drift between the two entry points.
pub const INGRESS_HANDOFF_CAPACITY: usize = 64;

/// Raw data-plane kind used by both the live Zenoh callback worker and the
/// implemented offline fault/replay runner so both share one decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngressPlane {
    Sensor,
    Command,
    Observation,
}

/// Callback-side result for one exact-route and raw-size admission check.
///
/// Both the live Zenoh callback and the offline observatory call this function
/// before constructing a worker message. It intentionally does not model queue
/// saturation: that outcome depends on the live bounded handoff, while offline
/// replay is sequential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "disposition", content = "plane")]
pub enum CallbackAdmission {
    Admitted(IngressPlane),
    RouteMismatchDropped,
    OversizedDropped,
}

/// Exact routing-key binding shared by live capture and offline raw-trace replay.
/// Named subkeys are intentionally not accepted as base-plane frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressRoutes {
    sensor: String,
    command: String,
    observation: String,
}

impl IngressRoutes {
    pub fn new(
        sensor: impl Into<String>,
        command: impl Into<String>,
        observation: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let routes = Self {
            sensor: sensor.into(),
            command: command.into(),
            observation: observation.into(),
        };
        let values = [
            routes.sensor.as_str(),
            routes.command.as_str(),
            routes.observation.as_str(),
        ];
        if values
            .iter()
            .any(|value| value.is_empty() || value.len() > 4096)
            || values[0] == values[1]
            || values[0] == values[2]
            || values[1] == values[2]
        {
            anyhow::bail!("ingress routes must be distinct non-empty bounded keys");
        }
        Ok(routes)
    }

    pub fn classify(&self, key: &str) -> Option<IngressPlane> {
        if key == self.sensor {
            Some(IngressPlane::Sensor)
        } else if key == self.command {
            Some(IngressPlane::Command)
        } else if key == self.observation {
            Some(IngressPlane::Observation)
        } else {
            None
        }
    }
}

/// Apply the production callback's exact-route and raw-size gates.
///
/// Route classification precedes size classification, matching the live
/// callback. A misrouted oversized receipt is therefore attributed to the route
/// boundary only. The function allocates nothing and never decodes payload bytes.
pub fn classify_callback_receipt(
    routes: &IngressRoutes,
    routing_key: &str,
    payload_len: usize,
    max_wire_frame_bytes: usize,
) -> CallbackAdmission {
    let Some(plane) = routes.classify(routing_key) else {
        return CallbackAdmission::RouteMismatchDropped;
    };
    if payload_len > max_wire_frame_bytes {
        CallbackAdmission::OversizedDropped
    } else {
        CallbackAdmission::Admitted(plane)
    }
}

/// Decoder/medium counters from the shared raw-wire ingress.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawIngressCounters {
    pub frames_seen: u64,
    pub raw_bytes_seen: u64,
    pub sensor_decode_failures: u64,
    pub command_decode_failures: u64,
    pub observation_decode_failures: u64,
    pub observation_unstamped: u64,
    pub oversized_frames: u64,
    pub routing_key_mismatches: u64,
    pub lifetime_limit_dropped: u64,
}

impl RawIngressCounters {
    pub fn decode_failures(&self) -> u64 {
        self.sensor_decode_failures
            .saturating_add(self.command_decode_failures)
            .saturating_add(self.observation_decode_failures)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawIngressDisposition {
    Applied,
    DecodeDropped,
    UnstampedObservationDropped,
    OversizedDropped,
    CapacityDropped,
}

/// Allocation-light JSON preflight that rejects duplicate object keys before
/// NCP's typed decoder sees the payload. `serde_json` otherwise applies
/// last-key-wins, which would make a conflicting wire receipt arrival-order and
/// parser dependent.
struct StrictJson;

impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct StrictJsonVisitor;

        impl<'de> serde::de::Visitor<'de> for StrictJsonVisitor {
            type Value = StrictJson;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("strict JSON without duplicate object keys")
            }

            fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
                Ok(StrictJson)
            }

            fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
                Ok(StrictJson)
            }

            fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
                Ok(StrictJson)
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value.is_finite() {
                    Ok(StrictJson)
                } else {
                    Err(E::custom("non-finite JSON number"))
                }
            }

            fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
                Ok(StrictJson)
            }

            fn visit_string<E>(self, _: String) -> Result<Self::Value, E> {
                Ok(StrictJson)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(StrictJson)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(StrictJson)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                StrictJson::deserialize(deserializer)
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                while sequence.next_element::<StrictJson>()?.is_some() {}
                Ok(StrictJson)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut keys = BTreeSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !keys.insert(key.clone()) {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate JSON object key {key:?}"
                        )));
                    }
                    map.next_value::<StrictJson>()?;
                }
                Ok(StrictJson)
            }
        }

        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

fn strict_json_preflight(bytes: &[u8]) -> bool {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    StrictJson::deserialize(&mut deserializer).is_ok() && deserializer.end().is_ok()
}

fn record_decode_drop(
    observer: &mut Observer,
    counters: &mut RawIngressCounters,
    plane: IngressPlane,
) {
    match plane {
        IngressPlane::Sensor => {
            counters.sensor_decode_failures = counters.sensor_decode_failures.saturating_add(1);
        }
        IngressPlane::Command => {
            counters.command_decode_failures = counters.command_decode_failures.saturating_add(1);
        }
        IngressPlane::Observation => {
            counters.observation_decode_failures =
                counters.observation_decode_failures.saturating_add(1);
        }
    }
    observer.stats.ingress_decode_dropped = observer.stats.ingress_decode_dropped.saturating_add(1);
}

/// Process one worker-admitted raw wire frame through the production decoder
/// and observation-medium gate. The observer, not caller-owned diagnostics,
/// owns the lifetime budget and canonical integrity counters. The offline
/// observatory calls this same seam after shared callback pre-admission.
pub fn ingest_wire_frame(
    observer: &mut Observer,
    plane: IngressPlane,
    bytes: &[u8],
    counters: &mut RawIngressCounters,
) -> anyhow::Result<RawIngressDisposition> {
    if observer.capture_capacity_reached {
        observer.stats.capture_capacity_dropped =
            observer.stats.capture_capacity_dropped.saturating_add(1);
        return Ok(RawIngressDisposition::CapacityDropped);
    }
    let next_frames = observer.raw_frames_seen.saturating_add(1);
    let next_bytes = observer
        .raw_bytes_seen
        .checked_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
    if next_frames > observer.limits.max_wire_frames
        || next_bytes.is_none_or(|value| value > observer.limits.max_total_wire_bytes)
    {
        counters.lifetime_limit_dropped = counters.lifetime_limit_dropped.saturating_add(1);
        observer.stats.ingress_lifetime_limit_dropped = observer
            .stats
            .ingress_lifetime_limit_dropped
            .saturating_add(1);
        observer.trip_capacity();
        return Ok(RawIngressDisposition::CapacityDropped);
    }
    observer.raw_frames_seen = next_frames;
    observer.raw_bytes_seen = next_bytes.unwrap_or(u64::MAX);
    counters.frames_seen = next_frames;
    counters.raw_bytes_seen = observer.raw_bytes_seen;
    if bytes.len() > observer.limits.max_wire_frame_bytes {
        counters.oversized_frames = counters.oversized_frames.saturating_add(1);
        observer.stats.ingress_oversized_dropped =
            observer.stats.ingress_oversized_dropped.saturating_add(1);
        return Ok(RawIngressDisposition::OversizedDropped);
    }
    if !strict_json_preflight(bytes) {
        record_decode_drop(observer, counters, plane);
        return Ok(RawIngressDisposition::DecodeDropped);
    }
    match plane {
        IngressPlane::Sensor => match decode_validated::<SensorFrame>(bytes) {
            Ok(frame) => observer.on_sensor(&frame)?,
            Err(_) => {
                record_decode_drop(observer, counters, plane);
                return Ok(RawIngressDisposition::DecodeDropped);
            }
        },
        IngressPlane::Command => match decode_validated::<CommandFrame>(bytes) {
            Ok(frame) => observer.on_command(&frame)?,
            Err(_) => {
                record_decode_drop(observer, counters, plane);
                return Ok(RawIngressDisposition::DecodeDropped);
            }
        },
        IngressPlane::Observation => match decode_validated::<ObservationFrame>(bytes) {
            Ok(frame) if frame.source.is_some() => observer.on_observation(&frame)?,
            Ok(_) => {
                counters.observation_unstamped = counters.observation_unstamped.saturating_add(1);
                observer.stats.ingress_unstamped_observations_dropped = observer
                    .stats
                    .ingress_unstamped_observations_dropped
                    .saturating_add(1);
                return Ok(RawIngressDisposition::UnstampedObservationDropped);
            }
            Err(_) => {
                record_decode_drop(observer, counters, plane);
                return Ok(RawIngressDisposition::DecodeDropped);
            }
        },
    }
    Ok(RawIngressDisposition::Applied)
}

fn flatten_except_bounded(
    channels: &BTreeMap<String, ChannelValue>,
    except: &[&str],
    max_values: usize,
) -> Option<Vec<f64>> {
    let mut out = Vec::new();
    // BTreeMap iterates in sorted key order → deterministic concatenation.
    for (name, cv) in channels {
        if cv.data.iter().any(|value| !value.is_finite()) {
            return None;
        }
        if except.contains(&name.as_str()) {
            continue;
        }
        if out.len().checked_add(cv.data.len())? > max_values {
            return None;
        }
        out.extend_from_slice(&cv.data);
    }
    Some(out)
}

fn semantic_hash_bounded<T: Serialize>(value: &T, max_bytes: usize) -> anyhow::Result<String> {
    let mut bytes = Vec::new();
    {
        let mut limited = LimitedWriter::new(&mut bytes, max_bytes);
        serde_json::to_writer(&mut limited, value)
            .context("failed to encode bounded complete-frame receipt")?;
    }
    Ok(pid_runlog::sha256_hex(&bytes))
}

/// Cap on the number of in-flight partial samples and unmatched D readouts kept
/// in memory. A long-running tap can accumulate source seqs that never complete
/// (a sensor with no matching command, or vice-versa) or observations whose tick
/// never arrives; without a bound these maps grow without limit. Eviction is
/// **insertion-order (FIFO)**, never lowest-key: after an epoch transition the
/// lowest keys are the *newest* entries, and lowest-key eviction would starve
/// every new tick while retaining the stale ones forever.
const MAX_INFLIGHT: usize = 4096;

/// How many newer source seqs must be observed before a V+A-complete tick is
/// emitted. The observation plane rides a lower QoS priority than the action
/// plane, so a tick's source-stamped D readout routinely arrives *after* its
/// command; holding completed ticks for a few seqs lets that readout claim its
/// own tick instead of being silently dropped. `finalize` flushes regardless.
const REORDER_GRACE: i64 = 8;

/// How many retired `stream.epoch`s to remember. A late straggler from a retired
/// epoch is dropped rather than mistaken for a fresh incarnation (which would
/// thrash the active stream); a modest bound covers realistic restart counts
/// without unbounded growth from a hostile stream of novel epochs.
const MAX_RETIRED_EPOCHS: usize = 64;

/// Immutable source revision behind the `v0.8.0` NCP dependency in this crate's manifest.
const NCP_RELEASE_REVISION: &str = "2f5bd586d4bb20c90362bb6f5698b7f64057ba4e";

/// Deployment transport facts recorded in the canonical configuration event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaptureTransportProvenance {
    /// Zenoh realm subscribed by the observer.
    pub realm: String,
    /// Operator-selected security posture (`open/unauthenticated` or
    /// `secure/fail-closed client config`).
    pub security_profile: String,
    /// Capacity of the bounded callback-to-owner handoff used by the capture binary.
    pub ingress_handoff_capacity: usize,
}

/// Accumulates NCP frames into `(V,L,D,A)` samples, joining V↔A (and D) on the
/// full driving-sensor `{epoch, seq}`.
pub struct Observer {
    run_id: String,
    model: String,
    task: String,
    mapping: Mapping,
    /// Full driving-sensor position → partial sample. Future-epoch command/D
    /// receipts may arrive before the authorizing sensor and remain quarantined
    /// under their own epoch without colliding with the active incarnation.
    pending: BTreeMap<StreamKey, Partial>,
    /// Insertion order of `pending` keys, for FIFO eviction.
    pending_order: VecDeque<StreamKey>,
    /// D readouts keyed by the full driving sensor `source` position.
    d_by_key: BTreeMap<StreamKey, (Vec<f64>, String, String)>,
    /// Insertion order of `d_by_key` keys, for FIFO eviction.
    d_order: VecDeque<StreamKey>,
    /// Highest source seq seen this epoch (the emission watermark).
    max_seq: i64,
    /// The live incarnation's `stream.epoch` (canonical UUIDv4); `None` until the
    /// first valid stamped sensor establishes it. Passenger command/D frames may
    /// be buffered first but never authorize a transition or advance the watermark.
    active_epoch: Option<String>,
    /// Retired epochs (past incarnations), so a late straggler is dropped rather
    /// than mistaken for a new incarnation. The capture stops admitting new
    /// epochs at [`MAX_RETIRED_EPOCHS`] rather than forgetting old identities.
    retired_epochs: BTreeSet<String>,
    /// The captured session's logical id; every frame's payload `session_id` must
    /// equal it. Bare-library tests may omit it while building in-memory state,
    /// but a canonical run log/publication cannot be configured without it.
    expected_session: Option<String>,
    /// Deployment-specific transport facts. Library-only callers may omit these;
    /// the canonical configuration then records `null` instead of inventing them.
    capture_transport: Option<CaptureTransportProvenance>,
    /// The live `session.generation`, locked by the first validated authorizing
    /// sensor; a frame from a different (stale/foreign) incarnation is rejected —
    /// one observer captures ONE session incarnation.
    expected_generation: Option<String>,
    /// Full source positions already emitted, excluded, or quarantined. These
    /// remain immutable across epoch transitions for the finite capture.
    closed_keys: BTreeSet<StreamKey>,
    /// Compact complete-frame receipts retained across retired epochs so exact
    /// redelivery stays idempotent and changed old evidence invalidates capture.
    closed_receipts: BTreeMap<StreamKey, PlaneReceipts>,
    /// Monotonic run-log clock: the max timestamp stamped so far. Sensor `t`
    /// values can complete out of order; clamping to the running max keeps the
    /// run log valid under pid-runlog's nondecreasing-timestamp rule.
    max_ts: u64,
    /// Dims of the first kept sample == the emitted `EmbeddingContract`.
    contract_dims: Option<[usize; 4]>,
    samples: Vec<OfflineVldaSample>,
    runlog_path: Option<PathBuf>,
    /// Canonical events buffered in lockstep with immutable samples. Finalize
    /// reconstructs the complete log atomically from this source on every retry.
    runlog_events: Vec<RunLogEvent>,
    stats: ObserverStats,
    n: u64,
    limits: ObserverLimits,
    total_sample_elements: usize,
    inflight_elements: usize,
    raw_frames_seen: u64,
    raw_bytes_seen: u64,
    capture_capacity_reached: bool,
    /// Set before the first artifact write. Once finalization begins, frame
    /// ingestion stays sealed even if I/O fails, so an exact retry cannot be
    /// invalidated by post-failure mutation.
    finalization_started: bool,
    /// All canonical destinations bound by the first finalization attempt.
    /// Retries cannot redirect any member of the committed bundle through a
    /// retargeted symlink or a different lexical path.
    finalize_targets: Option<FinalizeTargets>,
    finalized: bool,
}

#[derive(Default, Clone)]
struct Partial {
    v: Option<Vec<f64>>,
    /// Language channel contents; empty when the channel was absent (such ticks
    /// are excluded from the artifact and counted — see the module docs).
    l: Option<Vec<f64>>,
    l_present: bool,
    a: Option<Vec<f64>>,
    success: Option<serde_json::Value>,
    t: Option<f64>,
    sensor_hash: Option<String>,
    command_hash: Option<String>,
    generation: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct PlaneReceipts {
    sensor_hash: Option<String>,
    command_hash: Option<String>,
    observation_hash: Option<String>,
    conflicted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FinalizeTargets {
    dataset: PathBuf,
    runlog: PathBuf,
    receipt: PathBuf,
}

#[derive(Debug, Clone, Copy)]
enum InflightDiscardReason {
    EpochTransition,
    Finalize,
    Capacity,
}

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct LimitedWriter<'a, W> {
    inner: &'a mut W,
    written: usize,
    limit: usize,
}

impl<'a, W> LimitedWriter<'a, W> {
    fn new(inner: &'a mut W, limit: usize) -> Self {
        Self {
            inner,
            written: 0,
            limit,
        }
    }
}

impl<W: std::io::Write> std::io::Write for LimitedWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.written);
        if buffer.len() > remaining {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("serialized output exceeds {} bytes", self.limit),
            ));
        }
        let written = self.inner.write(buffer)?;
        self.written = self.written.saturating_add(written);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

struct BoundedBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedBuffer {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

impl std::io::Write for BoundedBuffer {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(buffer.len())
            .is_none_or(|size| size > self.limit)
        {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("reconstructed NCP run log exceeds {} bytes", self.limit),
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn create_temp_file(path: &Path) -> anyhow::Result<(PathBuf, File)> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("output path has no file name: {}", path.display()))?;
    for _ in 0..128 {
        let nonce = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut temp_name = OsString::from(".");
        temp_name.push(file_name);
        temp_name.push(format!(".tmp-{}-{nonce}", std::process::id()));
        let temp_path = parent.join(temp_name);
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to create temporary file for {}", path.display())
                });
            }
        }
    }
    anyhow::bail!(
        "failed to allocate a unique temporary file for {}",
        path.display()
    )
}

/// Write a same-directory temporary file, fsync it, install it without replacing
/// an existing destination, then fsync the directory entry. The destination is
/// untouched when the write/flush/fsync/install phase fails.
fn atomic_write_with<F>(path: &Path, write: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut BufWriter<File>) -> anyhow::Result<()>,
{
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (temp_path, file) = create_temp_file(path)?;
    let result = (|| {
        let mut writer = BufWriter::new(file);
        write(&mut writer)?;
        writer
            .flush()
            .with_context(|| format!("failed to flush temporary file for {}", path.display()))?;
        writer
            .get_ref()
            .sync_all()
            .with_context(|| format!("failed to fsync temporary file for {}", path.display()))?;
        drop(writer);
        std::fs::hard_link(&temp_path, path).with_context(|| {
            format!(
                "failed to atomically install {} without replacement from {}",
                path.display(),
                temp_path.display()
            )
        })?;
        std::fs::remove_file(&temp_path)
            .with_context(|| format!("failed to remove temporary link {}", temp_path.display()))?;
        #[cfg(unix)]
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .with_context(|| format!("failed to fsync directory {}", parent.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    result
}

/// Resolve a not-yet-created output through its canonical parent directory.
/// This catches aliases such as `artifact.json` versus `./artifact.json` and
/// symlinked parents before two logical outputs overwrite the same file.
fn output_target(path: &Path) -> anyhow::Result<PathBuf> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("output path has no file name: {}", path.display()))?;
    let canonical_parent = std::fs::canonicalize(parent)
        .with_context(|| format!("failed to resolve output directory {}", parent.display()))?;
    Ok(canonical_parent.join(file_name))
}

/// Re-establish durability when a previous no-replace install completed but
/// its final directory fsync reported an error. Exact retries may adopt only a
/// byte-for-byte matching file, then fsync both the file and directory again.
fn sync_installed_file(path: &Path) -> anyhow::Result<()> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .with_context(|| format!("failed to fsync installed file {}", path.display()))?;
    #[cfg(unix)]
    {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .with_context(|| format!("failed to fsync directory {}", parent.display()))?;
    }
    Ok(())
}

fn serialize_json_pretty_bounded<T: Serialize>(
    value: &T,
    max_bytes: usize,
) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    {
        let mut limited = LimitedWriter::new(&mut bytes, max_bytes);
        serde_json::to_writer_pretty(&mut limited, value)
            .context("failed to serialize bounded NCP observer artifact")?;
    }
    Ok(bytes)
}

#[cfg(unix)]
fn same_file_snapshot(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(not(unix))]
fn same_file_snapshot(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    left.file_type().is_file()
        && right.file_type().is_file()
        && left.len() == right.len()
        && left.modified().ok() == right.modified().ok()
}

/// Read one bounded regular-file snapshot without following an already-present
/// symlink. The opened handle and pathname identity are checked before and after
/// the bounded read so replacement/growth races fail closed.
pub fn read_bounded_regular_snapshot(path: &Path, max_bytes: usize) -> anyhow::Result<Vec<u8>> {
    let before = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect bounded input {}", path.display()))?;
    if !before.file_type().is_file() {
        anyhow::bail!(
            "bounded input {} must be a regular file (symlinks are rejected)",
            path.display()
        );
    }
    if before.len() > u64::try_from(max_bytes).unwrap_or(u64::MAX) {
        anyhow::bail!("bounded input {} exceeds {max_bytes} bytes", path.display());
    }
    let mut file = File::open(path)
        .with_context(|| format!("failed to open bounded input {}", path.display()))?;
    let opened_before = file
        .metadata()
        .with_context(|| format!("failed to inspect opened input {}", path.display()))?;
    let path_after_open = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to re-inspect bounded input {}", path.display()))?;
    if !opened_before.file_type().is_file()
        || !path_after_open.file_type().is_file()
        || !same_file_snapshot(&before, &opened_before)
        || !same_file_snapshot(&opened_before, &path_after_open)
    {
        anyhow::bail!("bounded input {} changed while opening", path.display());
    }
    let read_limit = u64::try_from(max_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(read_limit)
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read bounded input {}", path.display()))?;
    if bytes.len() > max_bytes {
        anyhow::bail!(
            "bounded input {} exceeds the {max_bytes}-byte verification limit",
            path.display()
        );
    }
    let opened_after = file
        .metadata()
        .with_context(|| format!("failed to re-inspect opened input {}", path.display()))?;
    let path_after_read = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to re-inspect bounded input {}", path.display()))?;
    if !path_after_read.file_type().is_file()
        || !same_file_snapshot(&opened_before, &opened_after)
        || !same_file_snapshot(&opened_after, &path_after_read)
    {
        anyhow::bail!("bounded input {} changed while reading", path.display());
    }
    Ok(bytes)
}

fn read_bounded(path: &Path, max_bytes: usize) -> anyhow::Result<Vec<u8>> {
    read_bounded_regular_snapshot(path, max_bytes)
}

trait FinalizeIo {
    fn write_artifact(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()>;
    fn hash_artifact(&mut self, bytes: &[u8]) -> anyhow::Result<String>;
    fn append_runlog(
        &mut self,
        events: &[RunLogEvent],
        max_bytes: usize,
    ) -> anyhow::Result<Vec<u8>>;
    fn write_runlog(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()>;
    fn write_receipt(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()>;
}

struct FsFinalizeIo;

impl FinalizeIo for FsFinalizeIo {
    fn write_artifact(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
        atomic_write_with(path, |writer| {
            writer
                .write_all(bytes)
                .context("failed to write NCP observer artifact")
        })
    }

    fn hash_artifact(&mut self, bytes: &[u8]) -> anyhow::Result<String> {
        Ok(pid_runlog::sha256_hex(bytes))
    }

    fn append_runlog(
        &mut self,
        events: &[RunLogEvent],
        max_bytes: usize,
    ) -> anyhow::Result<Vec<u8>> {
        let mut writer = RunLogWriter::new(BoundedBuffer::new(max_bytes));
        for event in events {
            writer.append(event)?;
        }
        writer.flush()?;
        Ok(writer.into_inner().into_inner())
    }

    fn write_runlog(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
        atomic_write_with(path, |writer| {
            writer
                .write_all(bytes)
                .context("failed to write reconstructed NCP observer run log")
        })
    }

    fn write_receipt(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
        atomic_write_with(path, |writer| {
            writer
                .write_all(bytes)
                .context("failed to write NCP observer publication receipt")
        })
    }
}

impl Observer {
    pub fn new(
        run_id: impl Into<String>,
        model: impl Into<String>,
        task: impl Into<String>,
        mapping: Mapping,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            model: model.into(),
            task: task.into(),
            mapping,
            pending: BTreeMap::new(),
            pending_order: VecDeque::new(),
            d_by_key: BTreeMap::new(),
            d_order: VecDeque::new(),
            max_seq: 0,
            active_epoch: None,
            retired_epochs: BTreeSet::new(),
            expected_session: None,
            capture_transport: None,
            expected_generation: None,
            closed_keys: BTreeSet::new(),
            closed_receipts: BTreeMap::new(),
            max_ts: 0,
            contract_dims: None,
            samples: Vec::new(),
            runlog_path: None,
            runlog_events: Vec::new(),
            stats: ObserverStats::default(),
            n: 0,
            limits: ObserverLimits::default(),
            total_sample_elements: 0,
            inflight_elements: 0,
            raw_frames_seen: 0,
            raw_bytes_seen: 0,
            capture_capacity_reached: false,
            finalization_started: false,
            finalize_targets: None,
            finalized: false,
        }
    }

    /// Replace the finite capture ceilings before any configuration is logged or
    /// frame is ingested.
    pub fn with_limits(mut self, limits: ObserverLimits) -> anyhow::Result<Self> {
        if self.runlog_path.is_some()
            || self.n != 0
            || !self.pending.is_empty()
            || !self.d_by_key.is_empty()
            || !self.closed_keys.is_empty()
            || !self.closed_receipts.is_empty()
            || self.stats != ObserverStats::default()
            || self.finalization_started
        {
            anyhow::bail!(
                "observer limits must be attached before the run log and frame ingestion"
            );
        }
        self.limits = limits.validate()?;
        Ok(self)
    }

    pub fn limits(&self) -> ObserverLimits {
        self.limits
    }

    /// Attach deployment transport provenance before the canonical run log is configured.
    ///
    /// # Errors
    /// Returns an error when transport provenance was already attached, the run log was
    /// configured, or frame ingestion/finalization has begun.
    pub fn with_capture_transport(
        mut self,
        realm: impl Into<String>,
        security_profile: impl Into<String>,
        ingress_handoff_capacity: usize,
    ) -> anyhow::Result<Self> {
        if self.capture_transport.is_some()
            || self.runlog_path.is_some()
            || self.n != 0
            || !self.pending.is_empty()
            || !self.d_by_key.is_empty()
            || !self.closed_keys.is_empty()
            || !self.closed_receipts.is_empty()
            || self.stats != ObserverStats::default()
            || self.finalization_started
        {
            anyhow::bail!(
                "capture transport provenance must be attached exactly once, before the run log and frame ingestion"
            );
        }
        let realm = realm.into();
        let security_profile = security_profile.into();
        if realm.is_empty() || realm.len() > 1024 {
            anyhow::bail!("capture realm must be a non-empty bounded string");
        }
        if security_profile.is_empty() || security_profile.len() > 1024 {
            anyhow::bail!("security profile must be a non-empty bounded string");
        }
        if ingress_handoff_capacity == 0 || ingress_handoff_capacity > 1_000_000 {
            anyhow::bail!("ingress handoff capacity must be in 1..=1000000");
        }
        self.capture_transport = Some(CaptureTransportProvenance {
            realm,
            security_profile,
            ingress_handoff_capacity,
        });
        Ok(self)
    }

    /// Attach a run-log so provenance events are emitted alongside the dataset.
    pub fn with_runlog(mut self, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        if self.expected_session.is_none() {
            anyhow::bail!(
                "expected session must be bound with with_expected_session before the run log"
            );
        }
        if self.runlog_path.is_some()
            || self.n != 0
            || !self.pending.is_empty()
            || !self.d_by_key.is_empty()
            || !self.closed_keys.is_empty()
            || !self.closed_receipts.is_empty()
            || self.stats != ObserverStats::default()
        {
            anyhow::bail!("run log must be attached exactly once, before frame ingestion");
        }
        for (name, value) in [
            ("run_id", self.run_id.as_str()),
            ("model", self.model.as_str()),
            ("task", self.task.as_str()),
            ("language_channel", self.mapping.language_channel.as_str()),
        ] {
            if value.is_empty() || value.len() > 4096 {
                anyhow::bail!("{name} must be a non-empty bounded string");
            }
        }
        if self
            .mapping
            .success_channel
            .as_ref()
            .is_some_and(|value| value.is_empty() || value.len() > 4096)
            || self
                .mapping
                .episode_id
                .as_ref()
                .is_some_and(|value| value.is_empty() || value.len() > 4096)
        {
            anyhow::bail!("mapping channel and episode identifiers must be bounded");
        }
        self.runlog_path = Some(path.as_ref().to_path_buf());
        let config = serde_json::json!({
            "component": "ncp-observer",
            "ncp": {
                "tag": "v0.8.0",
                "revision": NCP_RELEASE_REVISION,
                "wire": NCP_VERSION,
                "contract_hash": CONTRACT_HASH,
            },
            "run_id": self.run_id.clone(),
            "model": self.model.clone(),
            "task": self.task.clone(),
            "capture": {
                "expected_session": self.expected_session.clone(),
                "transport": self.capture_transport.clone(),
                "local_receipt_timestamps": "not_recorded",
                "clock_sync": "not_assessed",
                "reconnect_policy": "not_recorded",
                "subscription_qos": "ncp-zenoh raw session subscription; negotiated state not recorded",
                "observer_policy": {
                    "max_inflight": MAX_INFLIGHT,
                    "reorder_grace_source_seqs": REORDER_GRACE,
                    "max_retired_epochs": MAX_RETIRED_EPOCHS,
                    "resource_limits": self.limits,
                },
            },
            "mapping": {
                "language_channel": self.mapping.language_channel.clone(),
                "success_channel": self.mapping.success_channel.clone(),
                "episode_id": self.mapping.episode_id.clone(),
            },
        });
        let config_hash = pid_runlog::canonical_json_hash_v2(&config)
            .context("failed to hash NCP observer configuration")?;
        self.runlog_events.push(RunLogEvent::RunStarted {
            schema_version: RUN_LOG_SCHEMA_VERSION,
            run_id: self.run_id.clone(),
            timestamp_ns: 0,
            config_hash: config_hash.clone(),
            metadata: BTreeMap::from([("source".into(), "ncp".into())]),
        });
        self.runlog_events.push(RunLogEvent::ConfigLogged {
            timestamp_ns: 0,
            config_hash,
            config,
        });
        Ok(self)
    }

    /// Bind the captured session's logical `session_id`. Every ingested frame's
    /// payload `session_id` must then equal it (wire 0.8 carries `session_id` on
    /// the data plane too), so a frame addressed to another session is dropped and
    /// counted rather than blended into this capture. Must be set before the run log.
    ///
    /// # Errors
    /// Returns an error for an empty session or when configuration/ingestion has begun.
    pub fn with_expected_session(mut self, session_id: impl Into<String>) -> anyhow::Result<Self> {
        if self.expected_session.is_some()
            || self.runlog_path.is_some()
            || self.n != 0
            || !self.pending.is_empty()
            || !self.d_by_key.is_empty()
            || !self.closed_keys.is_empty()
            || !self.closed_receipts.is_empty()
            || self.stats != ObserverStats::default()
            || self.finalization_started
        {
            anyhow::bail!(
                "expected session must be configured exactly once, before the run log and frame ingestion"
            );
        }
        let session_id = session_id.into();
        if session_id.is_empty() || session_id.len() > 64 || !valid_id_segment(&session_id) {
            anyhow::bail!("expected session must be a valid NCP key segment of 1..=64 bytes");
        }
        self.expected_session = Some(session_id);
        Ok(self)
    }

    fn ensure_capturing(&self) -> anyhow::Result<()> {
        if self.finalized {
            anyhow::bail!("observer is finalized; refusing post-event artifact mutation");
        }
        if self.finalization_started {
            anyhow::bail!(
                "observer finalization has started; refusing mutation while an exact retry is pending"
            );
        }
        Ok(())
    }

    fn reject_after_capacity(&mut self) -> bool {
        if self.capture_capacity_reached {
            self.stats.capture_capacity_dropped =
                self.stats.capture_capacity_dropped.saturating_add(1);
            true
        } else {
            false
        }
    }

    /// Monotonic run-log timestamp for a sensor time `t` (seconds).
    fn stamp(&mut self, t: f64) -> u64 {
        let ts = (t * 1e9).max(0.0) as u64;
        self.max_ts = self.max_ts.max(ts);
        self.max_ts
    }

    /// Check the capture identity without locking a previously unseen
    /// generation. The lock is committed only after the complete typed frame has
    /// passed validation and resource checks.
    fn identity_precheck(&mut self, session_id: &str, generation: &str) -> bool {
        // `session_id` is required on the data plane; when a captured session is
        // bound it must match exactly (case-sensitive, no repair from a key).
        if session_id.is_empty()
            || self
                .expected_session
                .as_deref()
                .is_some_and(|expected| expected != session_id)
        {
            self.stats.session_mismatch_dropped =
                self.stats.session_mismatch_dropped.saturating_add(1);
            return false;
        }
        // `session.generation` fences incarnations. Lock onto the first live
        // generation seen; a frame from any other generation is stale/foreign and
        // is rejected (a reopened session needs a fresh observer run). A missing
        // generation is invalid on the 0.8 data plane.
        if generation.is_empty() {
            self.stats.session_mismatch_dropped =
                self.stats.session_mismatch_dropped.saturating_add(1);
            return false;
        }
        if self
            .expected_generation
            .as_deref()
            .is_some_and(|active| active != generation)
        {
            self.stats.session_mismatch_dropped =
                self.stats.session_mismatch_dropped.saturating_add(1);
            return false;
        }
        true
    }

    fn commit_generation(&mut self, generation: &str) {
        if self.expected_generation.is_none() {
            self.expected_generation = Some(generation.to_string());
        }
    }

    fn validated_frame_hash<T>(&mut self, frame: &T) -> Option<String>
    where
        T: WireFrame + Serialize,
    {
        if frame.validate_wire().is_err() {
            self.stats.invalid_payloads_dropped =
                self.stats.invalid_payloads_dropped.saturating_add(1);
            return None;
        }
        match semantic_hash_bounded(frame, self.limits.max_wire_frame_bytes) {
            Ok(hash) => Some(hash),
            Err(_) => {
                self.stats.invalid_payloads_dropped =
                    self.stats.invalid_payloads_dropped.saturating_add(1);
                None
            }
        }
    }

    /// Retire the outgoing `stream.epoch` without ever forgetting it during this
    /// finite capture. Exceeding the incarnation budget seals admission instead
    /// of letting a hostile churn reactivate an old epoch.
    fn retire_epoch(&mut self, epoch: String) -> bool {
        if self.retired_epochs.contains(&epoch) {
            return true;
        }
        if self.retired_epochs.len() >= MAX_RETIRED_EPOCHS {
            self.stats.epoch_limit_dropped = self.stats.epoch_limit_dropped.saturating_add(1);
            self.trip_capacity();
            return false;
        }
        self.retired_epochs.insert(epoch);
        true
    }

    fn record_missing_partial(&mut self, missing_sensor: bool, missing_command: bool) {
        if missing_sensor {
            self.stats.incomplete_missing_sensor =
                self.stats.incomplete_missing_sensor.saturating_add(1);
        }
        if missing_command {
            self.stats.incomplete_missing_command =
                self.stats.incomplete_missing_command.saturating_add(1);
        }
    }

    fn partial_elements(partial: &Partial) -> usize {
        partial.v.as_ref().map_or(0, Vec::len)
            + partial.l.as_ref().map_or(0, Vec::len)
            + partial.a.as_ref().map_or(0, Vec::len)
    }

    fn remove_pending(&mut self, key: &StreamKey) -> Option<Partial> {
        let partial = self.pending.remove(key)?;
        self.inflight_elements = self
            .inflight_elements
            .saturating_sub(Self::partial_elements(&partial));
        Some(partial)
    }

    fn remove_d(&mut self, key: &StreamKey) -> Option<(Vec<f64>, String, String)> {
        let receipt = self.d_by_key.remove(key)?;
        self.inflight_elements = self.inflight_elements.saturating_sub(receipt.0.len());
        Some(receipt)
    }

    fn reserve_inflight_elements(&mut self, additional: usize) -> bool {
        let Some(next) = self.inflight_elements.checked_add(additional) else {
            self.stats.inflight_element_limit_dropped =
                self.stats.inflight_element_limit_dropped.saturating_add(1);
            self.trip_capacity();
            return false;
        };
        if next > self.limits.max_inflight_elements {
            self.stats.inflight_element_limit_dropped =
                self.stats.inflight_element_limit_dropped.saturating_add(1);
            self.trip_capacity();
            return false;
        }
        self.inflight_elements = next;
        true
    }

    fn discard_inflight(&mut self, reason: InflightDiscardReason, keep_epoch: Option<&str>) {
        let pending_keys: Vec<StreamKey> = self
            .pending
            .keys()
            .filter(|(epoch, _)| keep_epoch != Some(epoch.as_str()))
            .cloned()
            .collect();
        let d_keys: Vec<StreamKey> = self
            .d_by_key
            .keys()
            .filter(|(epoch, _)| keep_epoch != Some(epoch.as_str()))
            .cloned()
            .collect();
        for key in &pending_keys {
            if let Some(partial) = self.remove_pending(key) {
                self.record_missing_partial(partial.v.is_none(), partial.a.is_none());
            }
        }
        for key in &d_keys {
            self.remove_d(key);
        }
        let incomplete = pending_keys.len();
        let unclaimed_d = d_keys.len();
        match reason {
            InflightDiscardReason::EpochTransition => {
                self.stats.incomplete_at_epoch_transition = self
                    .stats
                    .incomplete_at_epoch_transition
                    .saturating_add(incomplete);
                self.stats.unclaimed_d_at_epoch_transition = self
                    .stats
                    .unclaimed_d_at_epoch_transition
                    .saturating_add(unclaimed_d);
            }
            InflightDiscardReason::Finalize => {
                self.stats.incomplete_at_finalize =
                    self.stats.incomplete_at_finalize.saturating_add(incomplete);
                self.stats.unclaimed_d_at_finalize = self
                    .stats
                    .unclaimed_d_at_finalize
                    .saturating_add(unclaimed_d);
            }
            InflightDiscardReason::Capacity => {
                self.stats.evicted_incomplete =
                    self.stats.evicted_incomplete.saturating_add(incomplete);
                self.stats.evicted_unclaimed_d =
                    self.stats.evicted_unclaimed_d.saturating_add(unclaimed_d);
            }
        }
        self.pending_order
            .retain(|key| self.pending.contains_key(key));
        self.d_order.retain(|key| self.d_by_key.contains_key(key));
    }

    fn trip_capacity(&mut self) {
        if !self.capture_capacity_reached {
            self.capture_capacity_reached = true;
            self.stats.capture_capacity_reached = true;
            // A V+A-complete tick held only for the reorder grace is complete,
            // not an eviction. Seal first, flush deterministically, then classify
            // only the remaining partials/unclaimed D as capacity losses.
            self.flush_complete_unchecked();
            self.discard_inflight(InflightDiscardReason::Capacity, None);
        }
    }

    fn ensure_close_capacity(&mut self, key: &StreamKey) -> bool {
        if self.closed_keys.contains(key) {
            return true;
        }
        if self.closed_keys.len() >= self.limits.max_closed_ticks {
            self.stats.capture_capacity_dropped =
                self.stats.capture_capacity_dropped.saturating_add(1);
            self.trip_capacity();
            return false;
        }
        true
    }

    fn close_key(&mut self, key: StreamKey, receipts: PlaneReceipts) {
        if !self.ensure_close_capacity(&key) {
            return;
        }
        self.closed_keys.insert(key.clone());
        self.closed_receipts.insert(key, receipts);
        if self.closed_keys.len() >= self.limits.max_closed_ticks {
            self.trip_capacity();
        }
    }

    fn quarantine_conflicting_key(&mut self, key: StreamKey) {
        let partial = self.remove_pending(&key).unwrap_or_default();
        let observation_hash = self.remove_d(&key).map(|(_, hash, _)| hash);
        self.stats.conflicting_duplicates_dropped =
            self.stats.conflicting_duplicates_dropped.saturating_add(1);
        self.close_key(
            key,
            PlaneReceipts {
                sensor_hash: partial.sensor_hash,
                command_hash: partial.command_hash,
                observation_hash,
                conflicted: true,
            },
        );
        self.enforce_bounds();
    }

    fn classify_closed_receipt(
        &mut self,
        key: &StreamKey,
        plane: IngressPlane,
        semantic_hash: &str,
    ) -> bool {
        let Some(receipts) = self.closed_receipts.get_mut(key) else {
            return false;
        };
        if receipts.conflicted {
            self.stats.quarantined_frames_dropped =
                self.stats.quarantined_frames_dropped.saturating_add(1);
            return true;
        }
        let expected = match plane {
            IngressPlane::Sensor => receipts.sensor_hash.as_deref(),
            IngressPlane::Command => receipts.command_hash.as_deref(),
            IngressPlane::Observation => receipts.observation_hash.as_deref(),
        };
        match expected {
            Some(expected) if expected == semantic_hash => {
                self.stats.redelivered_frames_dropped =
                    self.stats.redelivered_frames_dropped.saturating_add(1);
            }
            Some(_) => {
                receipts.conflicted = true;
                self.stats.conflicting_duplicates_dropped =
                    self.stats.conflicting_duplicates_dropped.saturating_add(1);
            }
            None if plane == IngressPlane::Observation => {
                self.stats.late_d_dropped = self.stats.late_d_dropped.saturating_add(1);
            }
            None => {
                receipts.conflicted = true;
                self.stats.conflicting_duplicates_dropped =
                    self.stats.conflicting_duplicates_dropped.saturating_add(1);
            }
        }
        true
    }

    /// Establish/advance the active epoch + watermark from a valid SensorFrame.
    /// Passenger command/D receipts may be buffered under a future epoch but
    /// never authorize this transition. Wire 0.8
    /// replaces 0.7's `RESET_MARGIN` seq-distance heuristic: a restart is a change
    /// of `stream.epoch`. Returns `false` when the position is unstamped or from a
    /// retired incarnation and must be dropped.
    fn admit_sensor(&mut self, epoch: &str, seq: i64) -> bool {
        match self.active_epoch.as_deref() {
            None => {
                self.active_epoch = Some(epoch.to_string());
                self.max_seq = seq;
            }
            Some(active) if active == epoch => {
                if seq > self.max_seq {
                    self.max_seq = seq;
                }
            }
            Some(_) => {
                // A different incarnation. A late straggler from an already-retired
                // epoch must NOT re-trigger a reset (that would thrash the active
                // stream back and forth); drop and count it.
                if self.retired_epochs.contains(epoch) {
                    self.stats.retired_epoch_frames_dropped =
                        self.stats.retired_epoch_frames_dropped.saturating_add(1);
                    return false;
                }
                if self.retired_epochs.len() >= MAX_RETIRED_EPOCHS {
                    self.stats.epoch_limit_dropped =
                        self.stats.epoch_limit_dropped.saturating_add(1);
                    self.trip_capacity();
                    return false;
                }
                // Restart transition (constrained single-publisher passive tap):
                // flush the old epoch, retire it, discard unrelated state while
                // retaining already-buffered passengers for the newly authorized
                // epoch, then adopt the sensor's watermark.
                self.flush_complete_unchecked();
                if self.capture_capacity_reached {
                    return false;
                }
                self.discard_inflight(InflightDiscardReason::EpochTransition, Some(epoch));
                if let Some(old) = self.active_epoch.take() {
                    if !self.retire_epoch(old) {
                        return false;
                    }
                }
                self.stats.seq_resets = self.stats.seq_resets.saturating_add(1);
                self.active_epoch = Some(epoch.to_string());
                self.max_seq = seq;
            }
        }
        true
    }

    /// Ingest a `SensorFrame` (perception plane). Supplies V and L for its `seq`.
    pub fn on_sensor(&mut self, sensor: &SensorFrame) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        if self.reject_after_capacity() {
            return Ok(());
        }
        let Some(sensor_hash) = self.validated_frame_hash(sensor) else {
            return Ok(());
        };
        if !self.identity_precheck(&sensor.session_id, &sensor.session.generation) {
            return Ok(());
        }
        let key = (sensor.stream.epoch.clone(), sensor.stream.seq);
        if self.classify_closed_receipt(&key, IngressPlane::Sensor, &sensor_hash) {
            return Ok(());
        }

        // Validate and bound every derived axis before the sensor can lock a
        // generation, authorize an epoch transition, or advance the watermark.
        let l_channel = sensor.channels.get(&self.mapping.language_channel);
        if l_channel.is_some_and(|channel| {
            channel.data.len() > self.limits.max_axis_values
                || channel.data.iter().any(|value| !value.is_finite())
        }) {
            self.stats.invalid_payloads_dropped =
                self.stats.invalid_payloads_dropped.saturating_add(1);
            return Ok(());
        }
        let l_present = l_channel.is_some();
        let l = l_channel
            .map(|channel| channel.data.clone())
            .unwrap_or_default();
        let mut v_except: Vec<&str> = vec![self.mapping.language_channel.as_str()];
        if let Some(success_channel) = self.mapping.success_channel.as_deref() {
            v_except.push(success_channel);
        }
        let Some(v) =
            flatten_except_bounded(&sensor.channels, &v_except, self.limits.max_axis_values)
        else {
            self.stats.invalid_payloads_dropped =
                self.stats.invalid_payloads_dropped.saturating_add(1);
            return Ok(());
        };
        let success = self
            .mapping
            .success_channel
            .as_ref()
            .and_then(|channel| sensor.channels.get(channel))
            .and_then(|value| value.data.first().copied())
            .map(|value| serde_json::json!(value != 0.0));

        let passenger_generation_mismatch = self
            .pending
            .get(&key)
            .and_then(|partial| partial.generation.as_deref())
            .is_some_and(|generation| generation != sensor.session.generation)
            || self
                .d_by_key
                .get(&key)
                .is_some_and(|(_, _, generation)| generation != &sensor.session.generation);
        if passenger_generation_mismatch {
            self.quarantine_conflicting_key(key);
            return Ok(());
        }
        self.commit_generation(&sensor.session.generation);
        // A SensorFrame is the source origin and the only frame allowed to
        // authorize a fresh source epoch or move its emission watermark.
        let seq = sensor.stream.seq;
        if !self.admit_sensor(&sensor.stream.epoch, seq) {
            return Ok(());
        }
        if let Some(existing) = self.pending.get(&key) {
            if existing.v.is_some() {
                if existing.sensor_hash.as_deref() == Some(sensor_hash.as_str()) {
                    self.stats.redelivered_frames_dropped =
                        self.stats.redelivered_frames_dropped.saturating_add(1);
                } else {
                    self.quarantine_conflicting_key(key);
                }
                return Ok(());
            }
        }
        if !self.reserve_inflight_elements(v.len().saturating_add(l.len())) {
            return Ok(());
        }
        if !self.pending.contains_key(&key) {
            self.pending_order.push_back(key.clone());
        }
        let entry = self.pending.entry(key).or_default();
        entry.v = Some(v);
        entry.l = Some(l);
        entry.l_present = l_present;
        entry.t = Some(sensor.t);
        entry.sensor_hash = Some(sensor_hash);
        entry.generation = Some(sensor.session.generation.clone());
        if success.is_some() {
            entry.success = success;
        }
        self.enforce_bounds();
        self.emit_ready();
        Ok(())
    }

    /// Ingest a `CommandFrame` (action plane). Supplies A, correlated to the
    /// driving sensor via `command.source` (never the command's OWN `stream`,
    /// which is the action plane's delivery position — binding to it would create
    /// an independent counter that never joins V — the "silent zero-sample" trap
    /// (grandplan §8.7 adapter contract: adapter-specific omissions must be explicit
    /// capabilities, not null fields silently interpreted as data)).
    pub fn on_command(&mut self, command: &CommandFrame) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        if self.reject_after_capacity() {
            return Ok(());
        }
        let Some(command_hash) = self.validated_frame_hash(command) else {
            return Ok(());
        };
        if !self.identity_precheck(&command.session_id, &command.session.generation) {
            return Ok(());
        }
        // CORRELATION join: the command binds to the SENSOR that drove it. A
        // command with no `source` is open-loop / negotiated — there is no driving
        // tick to correlate a (V,L,D,A) sample against, so it is dropped (source
        // ABSENCE, the wire-0.8 successor to the retired `seq == 0` sentinel).
        let source = match command.source.as_ref() {
            Some(source) => source,
            None => {
                self.stats.unstamped_frames_dropped =
                    self.stats.unstamped_frames_dropped.saturating_add(1);
                return Ok(());
            }
        };
        let key = (source.epoch.clone(), source.seq);
        if self.classify_closed_receipt(&key, IngressPlane::Command, &command_hash) {
            return Ok(());
        }
        if self.retired_epochs.contains(&source.epoch) {
            self.stats.retired_epoch_frames_dropped =
                self.stats.retired_epoch_frames_dropped.saturating_add(1);
            return Ok(());
        }
        let Some(a) = flatten_except_bounded(&command.channels, &[], self.limits.max_axis_values)
        else {
            self.stats.invalid_payloads_dropped =
                self.stats.invalid_payloads_dropped.saturating_add(1);
            return Ok(());
        };
        if let Some(existing) = self.pending.get(&key) {
            if existing
                .generation
                .as_deref()
                .is_some_and(|generation| generation != command.session.generation)
            {
                self.quarantine_conflicting_key(key);
                return Ok(());
            }
            if existing.a.is_some() {
                if existing.command_hash.as_deref() == Some(command_hash.as_str()) {
                    self.stats.redelivered_frames_dropped =
                        self.stats.redelivered_frames_dropped.saturating_add(1);
                } else {
                    self.quarantine_conflicting_key(key);
                }
                return Ok(());
            }
        }
        if !self.reserve_inflight_elements(a.len()) {
            return Ok(());
        }
        if !self.pending.contains_key(&key) {
            self.pending_order.push_back(key.clone());
        }
        let entry = self.pending.entry(key).or_default();
        entry.a = Some(a);
        entry.command_hash = Some(command_hash);
        entry
            .generation
            .get_or_insert_with(|| command.session.generation.clone());
        if entry.t.is_none() {
            // Prefer the driving sensor's time (`source_t`) as the tick clock; fall
            // back to the command's own creation time when it was left unset.
            entry.t = Some(if command.source_t != 0.0 {
                command.source_t
            } else {
                command.t
            });
        }
        self.enforce_bounds();
        self.emit_ready();
        Ok(())
    }

    /// Ingest an `ObservationFrame` (neural readback). Updates the D axis,
    /// correlated to the driving sensor via `obs.source` (the cross-plane join
    /// key), on the full `{epoch, seq}`.
    pub fn on_observation(&mut self, obs: &ObservationFrame) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        if self.reject_after_capacity() {
            return Ok(());
        }
        let Some(observation_hash) = self.validated_frame_hash(obs) else {
            return Ok(());
        };
        if !self.identity_precheck(&obs.session_id, &obs.session.generation) {
            return Ok(());
        }
        // The pull/RPC reply form carries NO `source` (source ABSENCE, replacing
        // the retired `seq == 0` sentinel): it has no exact driving tick, so a
        // passive plane observer drops it rather than pairing future D by recency.
        let source = match obs.source.as_ref() {
            Some(source) if !source.epoch.is_empty() && source.seq >= 1 => source,
            _ => {
                self.stats.unstamped_frames_dropped =
                    self.stats.unstamped_frames_dropped.saturating_add(1);
                return Ok(());
            }
        };
        let key = (source.epoch.clone(), source.seq);
        if self.classify_closed_receipt(&key, IngressPlane::Observation, &observation_hash) {
            return Ok(());
        }
        if self.retired_epochs.contains(&source.epoch) {
            self.stats.retired_epoch_frames_dropped =
                self.stats.retired_epoch_frames_dropped.saturating_add(1);
            return Ok(());
        }
        let mut d = Vec::new();
        // Deterministic order: records is a BTreeMap keyed by port.
        for ob in obs.records.values() {
            if ob.values.iter().any(|value| !value.is_finite())
                || ob.times.iter().any(|value| !value.is_finite())
            {
                self.stats.invalid_payloads_dropped =
                    self.stats.invalid_payloads_dropped.saturating_add(1);
                return Ok(());
            }
            if !ob.values.is_empty() {
                if d.len()
                    .checked_add(ob.values.len())
                    .is_none_or(|size| size > self.limits.max_axis_values)
                {
                    self.stats.invalid_payloads_dropped =
                        self.stats.invalid_payloads_dropped.saturating_add(1);
                    return Ok(());
                }
                d.extend_from_slice(&ob.values);
            } else if !ob.times.is_empty() {
                // spikes with no analog values → use the spike count as a scalar.
                if d.len() >= self.limits.max_axis_values {
                    self.stats.invalid_payloads_dropped =
                        self.stats.invalid_payloads_dropped.saturating_add(1);
                    return Ok(());
                }
                d.push(ob.times.len() as f64);
            }
        }
        if self
            .pending
            .get(&key)
            .and_then(|partial| partial.generation.as_deref())
            .is_some_and(|generation| generation != obs.session.generation)
        {
            self.quarantine_conflicting_key(key);
            return Ok(());
        }
        if let Some((_, existing_hash, generation)) = self.d_by_key.get(&key) {
            if existing_hash == &observation_hash && generation == &obs.session.generation {
                self.stats.redelivered_frames_dropped =
                    self.stats.redelivered_frames_dropped.saturating_add(1);
            } else {
                self.quarantine_conflicting_key(key);
            }
            return Ok(());
        }
        if !self.reserve_inflight_elements(d.len()) {
            return Ok(());
        }
        if !self.d_by_key.contains_key(&key) {
            self.d_order.push_back(key.clone());
        }
        self.d_by_key
            .insert(key, (d, observation_hash, obs.session.generation.clone()));
        // Deliberately do NOT advance `max_seq` or the epoch from an observation:
        // D is a passenger on the control-loop clock, and a hostile inflated
        // source must not force a reset or premature emission.
        self.enforce_bounds();
        self.emit_ready();
        Ok(())
    }

    /// Evict oldest-inserted in-flight state once either map exceeds
    /// [`MAX_INFLIGHT`]. Completed ticks are removed by `emit_ready`, so what
    /// accumulates here is never-completed `seq`s and unclaimed readouts.
    fn enforce_bounds(&mut self) {
        // The order deques record one entry per seq ever inserted, but
        // completed seqs leave the MAPS via emit_ready/emit_sample without
        // touching the deques — in a healthy long session the deques would
        // grow one stale i64 per tick, forever. Drain the stale front
        // (amortized O(1); eviction only needs the deques' relative order of
        // still-present keys, which front-popping preserves) …
        while let Some(key) = self.pending_order.front() {
            if self.pending.contains_key(key) {
                break;
            }
            self.pending_order.pop_front();
        }
        while let Some(key) = self.d_order.front() {
            if self.d_by_key.contains_key(key) {
                break;
            }
            self.d_order.pop_front();
        }
        // … and compact outright in the pathological case where a stuck live
        // front hides unbounded stale entries behind it, so deque length is
        // strictly bounded by 2×MAX_INFLIGHT.
        if self.pending_order.len() > 2 * MAX_INFLIGHT {
            let pending = &self.pending;
            self.pending_order.retain(|key| pending.contains_key(key));
        }
        if self.d_order.len() > 2 * MAX_INFLIGHT {
            let d_by_key = &self.d_by_key;
            self.d_order.retain(|key| d_by_key.contains_key(key));
        }
        while self.pending.len() > MAX_INFLIGHT {
            // Skip order entries whose key already completed (removed).
            match self.pending_order.pop_front() {
                Some(key) => {
                    if let Some(partial) = self.remove_pending(&key) {
                        self.record_missing_partial(partial.v.is_none(), partial.a.is_none());
                        self.stats.evicted_incomplete =
                            self.stats.evicted_incomplete.saturating_add(1);
                    }
                }
                None => break, // unreachable: order tracks every insertion
            }
        }
        while self.d_by_key.len() > MAX_INFLIGHT {
            match self.d_order.pop_front() {
                Some(key) => {
                    if self.remove_d(&key).is_some() {
                        self.stats.evicted_unclaimed_d =
                            self.stats.evicted_unclaimed_d.saturating_add(1);
                    }
                }
                None => break,
            }
        }
        // Closed keys are retained exactly across the finite capture. Admission
        // seals at `max_closed_ticks`; old evidence is never forgotten
        // and therefore can never re-open a duplicate sample id.
    }

    /// Emit every V+A-complete tick old enough to have cleared the reorder
    /// grace window, in ascending seq order.
    fn emit_ready(&mut self) {
        let cutoff = self.max_seq.saturating_sub(REORDER_GRACE);
        let Some(active_epoch) = self.active_epoch.as_deref() else {
            return;
        };
        let ready: Vec<StreamKey> = self
            .pending
            .iter()
            .filter(|((epoch, seq), partial)| {
                epoch == active_epoch
                    && *seq <= cutoff
                    && partial.v.is_some()
                    && partial.a.is_some()
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in ready {
            if let Some(partial) = self.remove_pending(&key) {
                self.emit_sample(key, partial);
            }
        }
    }

    /// Emit ALL currently-complete ticks regardless of the grace window (used by
    /// `finalize`, session resets, and tests).
    pub fn flush_complete(&mut self) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        self.flush_complete_unchecked();
        Ok(())
    }

    fn flush_complete_unchecked(&mut self) {
        let Some(active_epoch) = self.active_epoch.as_deref() else {
            return;
        };
        let ready: Vec<StreamKey> = self
            .pending
            .iter()
            .filter(|((epoch, _), partial)| {
                epoch == active_epoch && partial.v.is_some() && partial.a.is_some()
            })
            .map(|(key, _)| key.clone())
            .collect();
        for key in ready {
            if let Some(partial) = self.remove_pending(&key) {
                self.emit_sample(key, partial);
            }
        }
    }

    fn emit_sample(&mut self, key: StreamKey, p: Partial) {
        // Retaining a terminal full-frame receipt is part of emitting or
        // excluding a tick. Preflight it before mutating samples/events so the
        // declared closed-receipt ceiling remains strict even during the
        // recursive capacity-seal flush.
        if !self.ensure_close_capacity(&key) {
            return;
        }
        let (epoch, seq) = key;
        // D is admissible only when its driving `source` matches this tick's exact
        // {epoch, seq} — a buffered readout from another incarnation is treated as
        // absent, never mis-joined.
        let (d, d_source, observation_hash) = match self.remove_d(&(epoch.clone(), seq)) {
            Some((d, hash, _)) => (d, "source", Some(hash)),
            _ => (Vec::new(), "absent", None),
        };
        let receipts = PlaneReceipts {
            sensor_hash: p.sensor_hash.clone(),
            command_hash: p.command_hash.clone(),
            observation_hash,
            conflicted: false,
        };
        let v = p.v.unwrap_or_default();
        let l = p.l.unwrap_or_default();
        let a = p.a.unwrap_or_default();

        // Empty-axis ticks can never pass pid-offline-harness's validate_dataset
        // (nonempty, consistent dims), and one such sample would poison the whole
        // artifact — exclude and count instead of fabricating an axis (Gap 2).
        let mut excluded = false;
        if v.is_empty() {
            self.stats.excluded_empty_v = self.stats.excluded_empty_v.saturating_add(1);
            excluded = true;
        }
        if l.is_empty() {
            self.stats.excluded_empty_l = self.stats.excluded_empty_l.saturating_add(1);
            excluded = true;
        }
        if d.is_empty() {
            self.stats.excluded_empty_d = self.stats.excluded_empty_d.saturating_add(1);
            excluded = true;
        }
        if a.is_empty() {
            self.stats.excluded_empty_a = self.stats.excluded_empty_a.saturating_add(1);
            excluded = true;
        }
        if excluded {
            self.close_key((epoch, seq), receipts);
            return;
        }
        let dims = [v.len(), l.len(), d.len(), a.len()];
        match self.contract_dims {
            None => self.contract_dims = Some(dims),
            Some(contract) if contract != dims => {
                // A sample contradicting the declared contract would fail the
                // harness's consistent-dims validation and misdescribe the run
                // log's EmbeddingContract — exclude and count.
                self.stats.dim_mismatch_dropped = self.stats.dim_mismatch_dropped.saturating_add(1);
                self.close_key((epoch, seq), receipts);
                return;
            }
            Some(_) => {}
        }
        let sample_elements = dims
            .iter()
            .try_fold(0_usize, |total, value| total.checked_add(*value));
        if self.samples.len() >= self.limits.max_samples {
            self.stats.sample_limit_dropped = self.stats.sample_limit_dropped.saturating_add(1);
            self.close_key((epoch, seq), receipts);
            self.trip_capacity();
            return;
        }
        let Some(sample_elements) = sample_elements else {
            self.stats.element_limit_dropped = self.stats.element_limit_dropped.saturating_add(1);
            self.close_key((epoch, seq), receipts);
            self.trip_capacity();
            return;
        };
        let Some(next_total_elements) = self.total_sample_elements.checked_add(sample_elements)
        else {
            self.stats.element_limit_dropped = self.stats.element_limit_dropped.saturating_add(1);
            self.close_key((epoch, seq), receipts);
            self.trip_capacity();
            return;
        };
        if next_total_elements > self.limits.max_total_sample_elements {
            self.stats.element_limit_dropped = self.stats.element_limit_dropped.saturating_add(1);
            self.close_key((epoch, seq), receipts);
            self.trip_capacity();
            return;
        }

        let mut labels = BTreeMap::new();
        if let Some(s) = p.success {
            labels.insert("success".to_string(), s);
        }
        let mut metadata = BTreeMap::new();
        metadata.insert("seq".to_string(), seq.to_string());
        metadata.insert("epoch".to_string(), epoch.clone());
        metadata.insert("source".to_string(), "ncp".to_string());
        // Honest provenance: D is exact-source ({epoch, seq}) or the tick is
        // excluded. A kept sample's L is nonempty (empty-L ticks
        // were excluded above), and a nonempty L can only come from a present
        // language channel — so `l_source` is always `"channel"` here; the
        // old `"absent_zeroed"` branch was unreachable and misled readers
        // into thinking absent-L ticks were retained-and-marked.
        debug_assert!(p.l_present, "kept sample implies present language channel");
        metadata.insert("l_source".to_string(), "channel".to_string());
        metadata.insert("d_source".to_string(), d_source.to_string());
        let sample = OfflineVldaSample {
            // Epoch-qualified so ids stay unique across incarnation restarts.
            sample_id: format!("ncp-{epoch}-{seq}"),
            episode_id: self.mapping.episode_id.clone(),
            v,
            l,
            d,
            a,
            labels: labels.clone(),
            metadata,
        };
        self.buffer_runlog(&sample, p.t.unwrap_or(0.0), &labels);
        self.samples.push(sample);
        self.total_sample_elements = next_total_elements;
        self.stats.kept_samples = self.stats.kept_samples.saturating_add(1);
        self.n = self.n.saturating_add(1);
        self.close_key((epoch, seq), receipts);
        self.enforce_bounds();
    }

    fn buffer_runlog(
        &mut self,
        sample: &OfflineVldaSample,
        t: f64,
        labels: &BTreeMap<String, serde_json::Value>,
    ) {
        let ts = self.stamp(t);
        let step = self.n;
        if self.runlog_path.is_none() {
            return;
        }
        if let Some(dims) = self.contract_dims {
            // First kept sample: declare the contract (dims are all nonzero by
            // construction — empty-axis ticks never reach this point).
            if step == 0 {
                let var = |name: &str, d: usize| EmbeddingVariableContract {
                    variable: name.to_string(),
                    source: format!("nest:{name}"),
                    dims: vec![d],
                    artifact_uri: None,
                    sha256: None,
                };
                self.runlog_events.push(RunLogEvent::EmbeddingContract {
                    timestamp_ns: ts,
                    name: "vlda".into(),
                    variables: vec![
                        var("v", dims[0]),
                        var("l", dims[1]),
                        var("d", dims[2]),
                        var("a", dims[3]),
                    ],
                    metadata: BTreeMap::new(),
                });
            }
        }
        // Mirror the honesty provenance into the run log so the source of truth
        // records how each axis was aligned, independent of the JSON artifact.
        let mut meta = BTreeMap::new();
        meta.insert("sample_id".to_string(), sample.sample_id.clone());
        for key in ["seq", "epoch", "l_source", "d_source"] {
            if let Some(value) = sample.metadata.get(key) {
                meta.insert(key.to_string(), value.clone());
            }
        }
        self.runlog_events.push(RunLogEvent::EmbeddingCaptured {
            step,
            timestamp_ns: ts,
            name: "vlda".into(),
            dims: vec![
                sample.v.len(),
                sample.l.len(),
                sample.d.len(),
                sample.a.len(),
            ],
            artifact_uri: None,
            sha256: None,
            metadata: meta,
        });
        for (name, value) in labels {
            self.runlog_events.push(RunLogEvent::LabelObserved {
                step,
                timestamp_ns: ts,
                name: name.clone(),
                value: value.clone(),
                metadata: BTreeMap::new(),
            });
        }
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    pub fn stats(&self) -> &ObserverStats {
        &self.stats
    }

    /// Record callback-side drops that happen before the worker-owned raw
    /// decoder can see a receipt. Decoder, medium, and worker lifetime counters
    /// are already owned and updated by [`ingest_wire_frame`].
    pub fn record_callback_drops(
        &mut self,
        oversized_dropped: u64,
        route_mismatch_dropped: u64,
        handoff_dropped: u64,
    ) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        self.stats.ingress_handoff_dropped = self
            .stats
            .ingress_handoff_dropped
            .saturating_add(handoff_dropped);
        self.stats.ingress_oversized_dropped = self
            .stats
            .ingress_oversized_dropped
            .saturating_add(oversized_dropped);
        self.stats.ingress_route_mismatch_dropped = self
            .stats
            .ingress_route_mismatch_dropped
            .saturating_add(route_mismatch_dropped);
        Ok(())
    }

    pub fn record_capture_worker_failure(&mut self) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        self.stats.capture_worker_failures = self.stats.capture_worker_failures.saturating_add(1);
        Ok(())
    }

    pub fn record_capture_teardown_failure(&mut self) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        self.stats.capture_teardown_failures =
            self.stats.capture_teardown_failures.saturating_add(1);
        Ok(())
    }

    #[cfg(test)]
    fn sample(&self, idx: usize) -> &OfflineVldaSample {
        &self.samples[idx]
    }

    /// Atomically finalize the dataset and its canonical run log.
    ///
    /// Samples and buffered events remain owned by the observer until every
    /// write, hash, append, install, and fsync succeeds. On error the caller may
    /// retry: the complete run log is reconstructed from the immutable event
    /// buffer, so a partial append can never create duplicates or data loss.
    pub fn finalize(&mut self, dataset_path: impl AsRef<Path>) -> anyhow::Result<ObserverStats> {
        let mut io = FsFinalizeIo;
        self.finalize_with_io(dataset_path.as_ref(), &mut io)
    }

    fn finalize_with_io<I: FinalizeIo>(
        &mut self,
        dataset_path: &Path,
        io: &mut I,
    ) -> anyhow::Result<ObserverStats> {
        if self.finalized {
            anyhow::bail!("observer is already finalized");
        }
        let runlog_path = self.runlog_path.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "canonical run log is required; attach it with with_runlog before ingestion"
            )
        })?;
        let receipt_path = publication_receipt_path(dataset_path);
        let dataset_target = output_target(dataset_path)?;
        let runlog_target = output_target(&runlog_path)?;
        let receipt_target = output_target(&receipt_path)?;
        if runlog_target == dataset_target
            || receipt_target == dataset_target
            || receipt_target == runlog_target
        {
            anyhow::bail!("dataset, run-log, and publication-receipt paths must be different");
        }
        let target_uri = |name: &str, path: &Path| -> anyhow::Result<String> {
            path.to_str().map(str::to_owned).ok_or_else(|| {
                anyhow::anyhow!(
                    "{name} canonical output path is not valid UTF-8 and cannot be represented in the publication bundle"
                )
            })
        };
        let dataset_uri = target_uri("dataset", &dataset_target)?;
        let runlog_uri = target_uri("run-log", &runlog_target)?;
        let receipt_uri = target_uri("publication-receipt", &receipt_target)?;
        let finalize_targets = FinalizeTargets {
            dataset: dataset_target.clone(),
            runlog: runlog_target.clone(),
            receipt: receipt_target.clone(),
        };
        if self.finalization_started {
            if self.finalize_targets.as_ref() != Some(&finalize_targets) {
                anyhow::bail!("finalization retry canonical bundle targets changed");
            }
        } else {
            self.flush_complete_unchecked();
            self.discard_inflight(InflightDiscardReason::Finalize, None);
            self.stats.zero_sample_capture = self.samples.is_empty();
            self.finalize_targets = Some(finalize_targets);
            self.finalization_started = true;
        }
        let stats = self.stats.clone();
        let dataset = OfflineVldaDataset {
            run_id: self.run_id.clone(),
            source: "ncp".into(),
            model: self.model.clone(),
            task: self.task.clone(),
            capture_integrity: stats.capture_integrity().to_string(),
            support: BTreeMap::new(),
            publication_receipt: receipt_uri,
            samples: self.samples.clone(),
        };
        // Construct and size-check both outputs before either final path becomes
        // visible. Deterministic serialization/hash/run-log failures therefore
        // cannot orphan an ordinary dataset without its canonical log.
        let dataset_bytes =
            serialize_json_pretty_bounded(&dataset, self.limits.max_artifact_bytes)?;
        let sha256 = io.hash_artifact(&dataset_bytes)?;
        let ts = self.max_ts;
        let mut final_events = self.runlog_events.clone();
        final_events.push(RunLogEvent::ArtifactLogged {
            timestamp_ns: ts,
            name: "ncp_vlda_dataset".to_string(),
            kind: "dataset_json".to_string(),
            uri: dataset_uri.clone(),
            sha256: Some(sha256.clone()),
            metadata: BTreeMap::from([
                ("kept_samples".to_string(), stats.kept_samples.to_string()),
                (
                    "capture_integrity".to_string(),
                    stats.capture_integrity().to_string(),
                ),
                ("capture_quality".to_string(), stats.summary()),
            ]),
        });
        let capture_integrity = stats.capture_integrity();
        final_events.push(RunLogEvent::RunEnded {
            run_id: self.run_id.clone(),
            timestamp_ns: ts,
            status: if matches!(capture_integrity, "complete" | "complete_with_warning") {
                RunStatus::Succeeded
            } else {
                RunStatus::Failed
            },
            message: Some(format!(
                "{} (V,L,D,A) samples from NCP [capture_integrity={capture_integrity}; {}]",
                dataset.samples.len(),
                stats.summary()
            )),
        });
        let runlog_bytes = io.append_runlog(&final_events, self.limits.max_runlog_bytes)?;
        let receipt = OfflineVldaPublicationReceipt {
            schema_version: PUBLICATION_RECEIPT_SCHEMA_VERSION,
            committed: true,
            dataset_uri,
            dataset_sha256: sha256,
            runlog_uri,
            runlog_sha256: pid_runlog::sha256_hex(&runlog_bytes),
            capture_integrity: capture_integrity.to_string(),
        };
        let receipt_bytes = serialize_json_pretty_bounded(&receipt, MAX_PUBLICATION_RECEIPT_BYTES)?;
        if dataset_target.exists() {
            let existing = read_bounded(&dataset_target, self.limits.max_artifact_bytes)?;
            if existing != dataset_bytes {
                anyhow::bail!(
                    "refusing to adopt non-byte-identical artifact {}",
                    dataset_target.display()
                );
            }
            sync_installed_file(&dataset_target)?;
        } else {
            io.write_artifact(&dataset_target, &dataset_bytes)?;
        }
        if runlog_target.exists() {
            let existing = read_bounded(&runlog_target, self.limits.max_runlog_bytes)?;
            if existing != runlog_bytes {
                anyhow::bail!(
                    "refusing to adopt non-byte-identical run log {}",
                    runlog_target.display()
                );
            }
            sync_installed_file(&runlog_target)?;
        } else {
            io.write_runlog(&runlog_target, &runlog_bytes)?;
        }
        if receipt_target.exists() {
            let existing = read_bounded(&receipt_target, MAX_PUBLICATION_RECEIPT_BYTES)?;
            if existing != receipt_bytes {
                anyhow::bail!(
                    "refusing to adopt non-byte-identical publication receipt {}",
                    receipt_target.display()
                );
            }
            sync_installed_file(&receipt_target)?;
        } else {
            io.write_receipt(&receipt_target, &receipt_bytes)?;
        }
        self.finalized = true;
        Ok(stats)
    }

    /// The observer is a System actor (never a control authority).
    pub fn actor(&self) -> Actor {
        Actor {
            actor_type: ActorType::System,
            actor_id: "ncp-observer".into(),
            session_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ncp_core::{Map, SessionRef, StreamPosition};

    // Canonical wire-0.8 test identity: two `stream.epoch` incarnations (A, B)
    // under ONE live `session.generation`, plus a valid `session_id`. Frames
    // stamp these so the observer's identity + stream/source joins exercise the
    // real 0.8 envelope.
    const EPOCH_A: &str = "00000000-0000-4000-8000-0000000000a1";
    const EPOCH_B: &str = "00000000-0000-4000-8000-0000000000b2";
    const GEN: &str = "00000000-0000-4000-8000-0000000000c3";
    const SID: &str = "sess";

    fn spos(epoch: &str, seq: i64) -> StreamPosition {
        StreamPosition {
            epoch: epoch.into(),
            seq,
        }
    }

    fn gen() -> SessionRef {
        SessionRef {
            generation: GEN.into(),
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FailStage {
        ArtifactWrite,
        Hash,
        Append,
        RunlogWrite,
        ReceiptWrite,
    }

    struct FailOnceIo {
        stage: Option<FailStage>,
        fs: FsFinalizeIo,
    }

    struct FailAfterRunlogInstallIo {
        failed: bool,
        fs: FsFinalizeIo,
    }

    #[cfg(unix)]
    struct RetargetOnArtifactIo {
        link: PathBuf,
        replacement: PathBuf,
        retargeted: bool,
        fs: FsFinalizeIo,
    }

    impl FailOnceIo {
        fn new(stage: FailStage) -> Self {
            Self {
                stage: Some(stage),
                fs: FsFinalizeIo,
            }
        }

        fn fail(&mut self, stage: FailStage) -> anyhow::Result<()> {
            if self.stage == Some(stage) {
                self.stage = None;
                anyhow::bail!("injected {stage:?} failure");
            }
            Ok(())
        }
    }

    impl FinalizeIo for FailOnceIo {
        fn write_artifact(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fail(FailStage::ArtifactWrite)?;
            self.fs.write_artifact(path, bytes)
        }

        fn hash_artifact(&mut self, bytes: &[u8]) -> anyhow::Result<String> {
            self.fail(FailStage::Hash)?;
            self.fs.hash_artifact(bytes)
        }

        fn append_runlog(
            &mut self,
            events: &[RunLogEvent],
            max_bytes: usize,
        ) -> anyhow::Result<Vec<u8>> {
            self.fail(FailStage::Append)?;
            self.fs.append_runlog(events, max_bytes)
        }

        fn write_runlog(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fail(FailStage::RunlogWrite)?;
            self.fs.write_runlog(path, bytes)
        }

        fn write_receipt(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fail(FailStage::ReceiptWrite)?;
            self.fs.write_receipt(path, bytes)
        }
    }

    impl FinalizeIo for FailAfterRunlogInstallIo {
        fn write_artifact(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fs.write_artifact(path, bytes)
        }

        fn hash_artifact(&mut self, bytes: &[u8]) -> anyhow::Result<String> {
            self.fs.hash_artifact(bytes)
        }

        fn append_runlog(
            &mut self,
            events: &[RunLogEvent],
            max_bytes: usize,
        ) -> anyhow::Result<Vec<u8>> {
            self.fs.append_runlog(events, max_bytes)
        }

        fn write_runlog(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fs.write_runlog(path, bytes)?;
            if !self.failed {
                self.failed = true;
                anyhow::bail!("injected post-install runlog failure");
            }
            Ok(())
        }

        fn write_receipt(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fs.write_receipt(path, bytes)
        }
    }

    #[cfg(unix)]
    impl FinalizeIo for RetargetOnArtifactIo {
        fn write_artifact(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            if !self.retargeted {
                std::fs::remove_file(&self.link)?;
                std::os::unix::fs::symlink(&self.replacement, &self.link)?;
                self.retargeted = true;
            }
            self.fs.write_artifact(path, bytes)
        }

        fn hash_artifact(&mut self, bytes: &[u8]) -> anyhow::Result<String> {
            self.fs.hash_artifact(bytes)
        }

        fn append_runlog(
            &mut self,
            events: &[RunLogEvent],
            max_bytes: usize,
        ) -> anyhow::Result<Vec<u8>> {
            self.fs.append_runlog(events, max_bytes)
        }

        fn write_runlog(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fs.write_runlog(path, bytes)
        }

        fn write_receipt(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fs.write_receipt(path, bytes)
        }
    }

    fn ch(data: Vec<f64>) -> ChannelValue {
        ChannelValue { data, unit: None }
    }

    /// A `SensorFrame` stamped with its OWN `stream` (the origin position the
    /// downstream command/observation `source` copies), on incarnation `epoch`.
    fn sensor_ep(epoch: &str, seq: i64, t: f64, channels: &[(&str, Vec<f64>)]) -> SensorFrame {
        let mut sc = Map::new();
        for (name, data) in channels {
            sc.insert((*name).into(), ch(data.clone()));
        }
        SensorFrame {
            t,
            channels: sc,
            stream: spos(epoch, seq),
            session: gen(),
            session_id: SID.into(),
            ..Default::default()
        }
    }

    fn sensor(seq: i64, t: f64, channels: &[(&str, Vec<f64>)]) -> SensorFrame {
        sensor_ep(EPOCH_A, seq, t, channels)
    }

    /// A closed-loop `CommandFrame` whose `source` echoes the driving sensor's
    /// `stream` (epoch, seq) — the correlation join key.
    fn command_ep(epoch: &str, seq: i64, t: f64, channels: &[(&str, Vec<f64>)]) -> CommandFrame {
        let mut cc = Map::new();
        for (name, data) in channels {
            cc.insert((*name).into(), ch(data.clone()));
        }
        CommandFrame {
            t,
            channels: cc,
            // The command's OWN action-plane stream is deliberately distinct from
            // its `source`; the observer must join on `source`, never `stream`.
            stream: spos(epoch, seq),
            source: Some(spos(epoch, seq)),
            source_t: t,
            session: gen(),
            session_id: SID.into(),
            ..Default::default()
        }
    }

    fn command(seq: i64, t: f64, channels: &[(&str, Vec<f64>)]) -> CommandFrame {
        command_ep(EPOCH_A, seq, t, channels)
    }

    /// An open-loop command with NO `source` — uncorrelatable to a driving sensor.
    fn command_open_loop(t: f64, channels: &[(&str, Vec<f64>)]) -> CommandFrame {
        let mut frame = command_ep(EPOCH_A, 1, t, channels);
        frame.source = None;
        frame
    }

    /// A plane `ObservationFrame` whose `source` echoes the driving sensor's
    /// `stream` (epoch, seq); its own `stream` is a distinct observation-plane
    /// position the observer does not join on.
    fn observation_ep(epoch: &str, seq: i64, values: Vec<f64>) -> ObservationFrame {
        let mut records = Map::new();
        let times = (0..values.len()).map(|index| index as f64).collect();
        records.insert(
            "rate".into(),
            ncp_core::Observation {
                port: "rate".into(),
                target: "population".into(),
                times,
                values,
                ..Default::default()
            },
        );
        ObservationFrame {
            records,
            stream: spos(epoch, seq),
            source: Some(spos(epoch, seq)),
            session: gen(),
            session_id: SID.into(),
            ..Default::default()
        }
    }

    fn observation(seq: i64, values: Vec<f64>) -> ObservationFrame {
        observation_ep(EPOCH_A, seq, values)
    }

    /// A pull/RPC-form observation with NO `source` (the wire-0.8 successor to the
    /// retired `seq == 0` sentinel).
    fn observation_pull(values: Vec<f64>) -> ObservationFrame {
        let mut frame = observation_ep(EPOCH_A, 1, values);
        frame.source = None;
        frame
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        let nonce = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "ncp_observer_{name}_{}_{nonce}",
            std::process::id()
        ))
    }

    fn observer_with_exact_sample(runlog: &Path) -> Observer {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_expected_session(SID)
            .unwrap()
            .with_runlog(runlog)
            .unwrap();
        observer.on_observation(&observation(7, vec![3.0])).unwrap();
        observer
            .on_sensor(&sensor(
                7,
                1.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        observer
            .on_command(&command(7, 1.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        observer.flush_complete().unwrap();
        observer
    }

    #[test]
    fn config_event_records_exact_ncp_and_transport_provenance() {
        let observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_expected_session(SID)
            .unwrap()
            .with_capture_transport("engram/ncp", "secure/fail-closed client config", 1024)
            .unwrap()
            .with_runlog("unused.jsonl")
            .unwrap();
        let config = observer
            .runlog_events
            .iter()
            .find_map(|event| match event {
                RunLogEvent::ConfigLogged { config, .. } => Some(config),
                _ => None,
            })
            .unwrap();

        assert_eq!(config["ncp"]["wire"], NCP_VERSION);
        assert_eq!(config["ncp"]["contract_hash"], CONTRACT_HASH);
        assert_eq!(config["ncp"]["revision"], NCP_RELEASE_REVISION);
        assert_eq!(config["capture"]["expected_session"], SID);
        assert_eq!(config["capture"]["transport"]["realm"], "engram/ncp");
        assert_eq!(
            config["capture"]["transport"]["security_profile"],
            "secure/fail-closed client config"
        );
        assert_eq!(
            config["capture"]["transport"]["ingress_handoff_capacity"],
            1024
        );
        assert_eq!(
            config["capture"]["local_receipt_timestamps"],
            "not_recorded"
        );
    }

    #[test]
    fn effective_capture_changes_produce_distinct_config_hashes() {
        let hash_for = |session: &str, realm: &str, security: &str| {
            let observer = Observer::new("run", "nest", "reach", Mapping::default())
                .with_expected_session(session)
                .unwrap()
                .with_capture_transport(realm, security, 1024)
                .unwrap()
                .with_runlog("unused.jsonl")
                .unwrap();
            observer
                .runlog_events
                .iter()
                .find_map(|event| match event {
                    RunLogEvent::RunStarted { config_hash, .. } => Some(config_hash.clone()),
                    _ => None,
                })
                .unwrap()
        };
        let hashes = BTreeSet::from([
            hash_for("session-a", "engram/ncp", "open/unauthenticated"),
            hash_for(
                "session-a",
                "engram/ncp",
                "secure/fail-closed client config",
            ),
            hash_for("session-a", "other/ncp", "open/unauthenticated"),
            hash_for("session-b", "engram/ncp", "open/unauthenticated"),
        ]);

        assert_eq!(hashes.len(), 4);
    }

    #[test]
    fn expected_session_cannot_change_after_config_hash_is_frozen() {
        let observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_expected_session(SID)
            .unwrap()
            .with_runlog("unused.jsonl")
            .unwrap();

        let error = observer
            .with_expected_session("late-session")
            .err()
            .unwrap();

        assert!(error.to_string().contains("before the run log"));
    }

    #[test]
    fn canonical_runlog_requires_an_explicit_capture_session() {
        let error = Observer::new("run", "nest", "reach", Mapping::default())
            .with_runlog("unused.jsonl")
            .err()
            .unwrap();

        assert!(error.to_string().contains("with_expected_session"));
    }

    fn assert_finalize_retry_reconstructs(stage: FailStage) {
        let dir = unique_test_dir("retry");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut observer = observer_with_exact_sample(&runlog);
        let sample_before = observer.sample(0).clone();

        let mut failing = FailOnceIo::new(stage);
        let error = observer
            .finalize_with_io(&dataset, &mut failing)
            .unwrap_err();
        assert!(error.to_string().contains("injected"), "{error:#}");
        assert_eq!(observer.sample_count(), 1, "retry state must be preserved");
        assert_eq!(observer.sample(0).sample_id, sample_before.sample_id);
        assert!(!observer.finalized, "failed finalize must remain retryable");
        if stage == FailStage::ReceiptWrite {
            assert!(
                runlog.exists(),
                "run log was durably installed before commit"
            );
            assert!(!publication_receipt_path(&dataset).exists());
        } else {
            assert!(
                !runlog.exists(),
                "failed finalize must not publish a partial canonical log"
            );
        }

        observer.finalize(&dataset).unwrap();
        let written: OfflineVldaDataset =
            serde_json::from_slice(&std::fs::read(&dataset).unwrap()).unwrap();
        assert_eq!(written.samples.len(), 1);
        assert_eq!(written.samples[0].sample_id, sample_before.sample_id);
        let events = pid_runlog::read_events_from_path(&runlog).unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, RunLogEvent::EmbeddingCaptured { .. }))
                .count(),
            1,
            "retry must reconstruct exactly one canonical sample event"
        );
        assert!(events.iter().any(|event| matches!(
            event,
            RunLogEvent::ArtifactLogged {
                sha256: Some(_),
                ..
            }
        )));
        assert!(publication_receipt_path(&dataset).exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn joins_v_and_a_on_seq() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // The exact-seq observation may arrive before the sensor/action pair.
        obs.on_observation(&observation(7, vec![5.0, 6.0])).unwrap();

        // Sensor for seq=7 (V + L).
        obs.on_sensor(&sensor(
            7,
            1.0,
            &[("pose", vec![1.0, 2.0, 3.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 0, "no command yet");

        // Command for seq=7 (A) → completes the sample (held for the grace
        // window until flushed).
        obs.on_command(&command(
            7,
            1.0,
            &[("velocity_setpoint", vec![0.1, 0.0, -0.1])],
        ))
        .unwrap();
        assert_eq!(obs.sample_count(), 0, "held for the reorder grace window");
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 1);
        let s = obs.sample(0);
        assert_eq!(s.v, vec![1.0, 2.0, 3.0]); // pose only (instruction excluded)
        assert_eq!(s.l, vec![0.5]);
        assert_eq!(s.d, vec![5.0, 6.0]); // from the observation
        assert_eq!(s.a, vec![0.1, 0.0, -0.1]);
        assert_eq!(s.sample_id, format!("ncp-{EPOCH_A}-7"));
    }

    #[test]
    fn d_aligns_on_seq_not_recency() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // Observation for seq=7 (D=[5,6]), then a later one for seq=8 (D=[9,9]).
        obs.on_observation(&observation(7, vec![5.0, 6.0])).unwrap();
        obs.on_observation(&observation(8, vec![9.0, 9.0])).unwrap();
        // The seq=7 tick must pick the seq=7 D, not the most-recent (seq=8) one.
        obs.on_sensor(&sensor(
            7,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(7, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 1);
        assert_eq!(
            obs.sample(0).d,
            vec![5.0, 6.0],
            "D must align on seq 7, not recency"
        );
        assert_eq!(
            obs.sample(0).metadata.get("d_source").map(String::as_str),
            Some("source")
        );
        assert_eq!(
            obs.sample(0).metadata.get("l_source").map(String::as_str),
            Some("channel")
        );
    }

    #[test]
    fn late_observation_within_grace_is_seq_aligned() {
        // The observation plane rides a lower QoS priority than the action
        // plane, so D often arrives AFTER the tick's command. The grace window
        // must let it claim its own tick.
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_sensor(&sensor(
            7,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(7, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        assert_eq!(obs.sample_count(), 0, "held for the grace window");
        // The tick's own readout arrives late but within the grace window …
        obs.on_observation(&observation(7, vec![5.5])).unwrap();
        // … then the watermark advances far enough to emit seq 7.
        obs.on_sensor(&sensor(7 + REORDER_GRACE, 0.0, &[("pose", vec![1.0])]))
            .unwrap();
        assert_eq!(obs.sample_count(), 1, "emitted once past the grace window");
        assert_eq!(obs.sample(0).d, vec![5.5], "late D claimed its own tick");
        assert_eq!(
            obs.sample(0).metadata.get("d_source").map(String::as_str),
            Some("source")
        );
    }

    #[test]
    fn conflicting_observation_after_emission_invalidates_without_mutating_sample() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation(7, vec![4.4])).unwrap();
        obs.on_sensor(&sensor(
            7,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(7, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 1);
        let before = obs.sample(0).clone();
        let mut changed_receipt = observation(7, vec![4.4]);
        changed_receipt.records.get_mut("rate").unwrap().target = "other-population".into();
        obs.on_observation(&changed_receipt).unwrap();
        assert_eq!(obs.sample(0).d, before.d);
        assert_eq!(
            obs.sample(0).metadata.get("d_source").map(String::as_str),
            Some("source")
        );
        assert_eq!(obs.stats.late_d_dropped, 0);
        assert_eq!(obs.stats.conflicting_duplicates_dropped, 1);
        assert_eq!(obs.stats.capture_integrity(), "invalid");
    }

    #[test]
    fn empty_then_nonempty_observation_is_a_conflict_not_last_write_wins() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation(1, Vec::new())).unwrap();
        obs.on_observation(&observation(1, vec![3.0])).unwrap();
        obs.on_sensor(&sensor(
            1,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(1, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();

        assert_eq!(obs.sample_count(), 0);
        assert_eq!(obs.stats.conflicting_duplicates_dropped, 1);
        assert_eq!(obs.stats.redelivered_frames_dropped, 0);
        assert_eq!(obs.stats.capture_integrity(), "invalid");
    }

    #[test]
    fn sourceless_observation_is_dropped_without_future_d_pairing() {
        // A pull/RPC-form observation (no `source`) has no exact driving tick and
        // must never be promoted into a later tick's D by recency.
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation_pull(vec![3.0])).unwrap();
        obs.on_sensor(&sensor(
            4,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5, 0.5])],
        ))
        .unwrap();
        obs.on_command(&command(4, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 0, "unstamped D must never be promoted");
        assert_eq!(obs.stats.unstamped_frames_dropped, 1);
        assert_eq!(obs.stats.excluded_empty_d, 1);
    }

    #[test]
    fn empty_axis_ticks_are_excluded_and_counted() {
        // A tick with no language channel yields an empty L. It must be
        // excluded from the artifact (one empty-axis sample would make
        // pid-offline-harness reject the WHOLE dataset) and counted.
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation(4, vec![3.0])).unwrap();
        obs.on_sensor(&sensor(4, 0.0, &[("pose", vec![1.0])]))
            .unwrap(); // no "instruction"
        obs.on_command(&command(4, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 0, "empty-L tick excluded");
        assert_eq!(obs.stats.excluded_empty_l, 1);
        // Same for a tick before any observation arrived (empty D).
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_sensor(&sensor(
            4,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(4, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 0, "empty-D tick excluded");
        assert_eq!(obs.stats.excluded_empty_d, 1);
    }

    #[test]
    fn mismatched_seq_does_not_pair() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_sensor(&sensor(1, 0.0, &[("pose", vec![1.0])]))
            .unwrap();
        obs.on_command(&command(2, 0.0, &[("cmd", vec![0.0])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(
            obs.sample_count(),
            0,
            "seq 1 sensor must not pair with seq 2 command"
        );
    }

    #[test]
    fn success_channel_is_excluded_from_v() {
        // With a success channel configured (as the `ncp-observe` binary does),
        // the per-tick outcome must become the `success` LABEL and must NOT leak
        // into the V feature vector.
        let mapping = Mapping {
            language_channel: "instruction".into(),
            success_channel: Some("success".into()),
            episode_id: None,
        };
        let mut obs = Observer::new("run", "nest", "reach", mapping);
        obs.on_observation(&observation(3, vec![3.0])).unwrap();
        obs.on_sensor(&sensor(
            3,
            1.0,
            &[
                ("pose", vec![1.0, 2.0]),
                ("instruction", vec![0.5]),
                ("success", vec![1.0]),
            ],
        ))
        .unwrap();
        obs.on_command(&command(3, 1.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 1);
        let s = obs.sample(0);
        // V is `pose` only — neither the language channel nor the success channel.
        assert_eq!(
            s.v,
            vec![1.0, 2.0],
            "success/instruction must not leak into V"
        );
        assert_eq!(s.l, vec![0.5]);
        assert_eq!(
            s.labels.get("success"),
            Some(&serde_json::json!(true)),
            "the success outcome must surface as a label"
        );
    }

    #[test]
    fn order_deques_stay_bounded_in_healthy_long_sessions() {
        // Every tick COMPLETES here (sensor + command + observation per seq),
        // so the maps stay small — but before the fix the order deques kept
        // one stale i64 per seq forever. They must now stay bounded too.
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        let total = MAX_INFLIGHT as i64 + 500;
        for seq in 1..=total {
            obs.on_observation(&observation(seq, vec![seq as f64]))
                .unwrap();
            obs.on_sensor(&sensor(
                seq,
                seq as f64 * 0.01,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
            obs.on_command(&command(
                seq,
                seq as f64 * 0.01,
                &[("velocity_setpoint", vec![0.1])],
            ))
            .unwrap();
        }
        assert!(
            obs.sample_count() > MAX_INFLIGHT,
            "healthy session emits (got {})",
            obs.sample_count()
        );
        let bound = 2 * MAX_INFLIGHT;
        assert!(
            obs.pending_order.len() <= bound && obs.d_order.len() <= bound,
            "order deques must stay bounded: pending_order={} d_order={}",
            obs.pending_order.len(),
            obs.d_order.len()
        );
        // In the fully-healthy steady state they should in fact be tiny
        // (bounded by the reorder grace window plus the still-pending tail).
        assert!(
            obs.pending_order.len() <= (REORDER_GRACE as usize) + 2,
            "steady-state pending_order should be ~grace-window sized, got {}",
            obs.pending_order.len()
        );
    }

    #[test]
    fn redelivered_frames_do_not_duplicate_sample_ids() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // Emit seqs 1..=3 (advance the watermark past the grace window so they
        // emit through the normal path).
        for seq in 1..=(REORDER_GRACE + 3) {
            obs.on_observation(&observation(seq, vec![seq as f64]))
                .unwrap();
            obs.on_sensor(&sensor(
                seq,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
            obs.on_command(&command(seq, 0.0, &[("velocity_setpoint", vec![0.1])]))
                .unwrap();
        }
        let emitted_before = obs.sample_count();
        assert!(emitted_before >= 3, "got {emitted_before}");
        // Transport re-delivery of an already-emitted tick's pair.
        obs.on_sensor(&sensor(
            1,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(1, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.stats.redelivered_frames_dropped, 2);
        let mut ids: Vec<&str> = (0..obs.sample_count())
            .map(|i| obs.sample(i).sample_id.as_str())
            .collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "sample_ids must stay unique");
    }

    #[test]
    fn identical_pre_emission_duplicates_are_idempotent() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        let sensor = sensor(1, 1.0, &[("pose", vec![1.0]), ("instruction", vec![0.5])]);
        let command = command(1, 1.0, &[("velocity_setpoint", vec![0.1])]);
        let observation = observation(1, vec![3.0]);

        obs.on_sensor(&sensor).unwrap();
        obs.on_sensor(&sensor).unwrap();
        obs.on_command(&command).unwrap();
        obs.on_command(&command).unwrap();
        obs.on_observation(&observation).unwrap();
        obs.on_observation(&observation).unwrap();
        obs.flush_complete().unwrap();

        assert_eq!(obs.sample_count(), 1);
        assert_eq!(obs.stats.redelivered_frames_dropped, 3);
        assert_eq!(obs.stats.conflicting_duplicates_dropped, 0);
    }

    #[test]
    fn conflicting_pre_emission_duplicates_quarantine_each_plane() {
        let sensor_a = sensor(1, 1.0, &[("pose", vec![1.0]), ("instruction", vec![0.5])]);
        let sensor_b = sensor(1, 1.0, &[("pose", vec![2.0]), ("instruction", vec![0.5])]);
        let command_a = command(1, 1.0, &[("velocity_setpoint", vec![0.1])]);
        let command_b = command(1, 1.0, &[("velocity_setpoint", vec![0.2])]);
        let observation_a = observation(1, vec![3.0]);
        let observation_b = observation(1, vec![4.0]);

        let mut sensor_conflict = Observer::new("run", "nest", "reach", Mapping::default());
        sensor_conflict.on_sensor(&sensor_a).unwrap();
        sensor_conflict.on_sensor(&sensor_b).unwrap();
        sensor_conflict.on_observation(&observation_a).unwrap();
        sensor_conflict.on_command(&command_a).unwrap();
        sensor_conflict.flush_complete().unwrap();
        assert_eq!(sensor_conflict.sample_count(), 0);
        assert_eq!(sensor_conflict.stats.conflicting_duplicates_dropped, 1);

        let mut command_conflict = Observer::new("run", "nest", "reach", Mapping::default());
        command_conflict.on_sensor(&sensor_a).unwrap();
        command_conflict.on_command(&command_a).unwrap();
        command_conflict.on_command(&command_b).unwrap();
        command_conflict.on_observation(&observation_a).unwrap();
        command_conflict.flush_complete().unwrap();
        assert_eq!(command_conflict.sample_count(), 0);
        assert_eq!(command_conflict.stats.conflicting_duplicates_dropped, 1);

        let mut observation_conflict = Observer::new("run", "nest", "reach", Mapping::default());
        observation_conflict.on_observation(&observation_a).unwrap();
        observation_conflict.on_observation(&observation_b).unwrap();
        observation_conflict.on_sensor(&sensor_a).unwrap();
        observation_conflict.on_command(&command_a).unwrap();
        observation_conflict.flush_complete().unwrap();
        assert_eq!(observation_conflict.sample_count(), 0);
        assert_eq!(observation_conflict.stats.conflicting_duplicates_dropped, 1);
        observation_conflict.on_observation(&observation_a).unwrap();
        assert_eq!(
            observation_conflict.stats.quarantined_frames_dropped, 3,
            "sensor, command, and later observation stay in the quarantined class"
        );
        assert_eq!(observation_conflict.stats.redelivered_frames_dropped, 0);
    }

    #[test]
    fn conflicting_sensor_and_command_after_emission_invalidate_without_mutation() {
        let original_sensor = sensor(1, 1.0, &[("pose", vec![1.0]), ("instruction", vec![0.5])]);
        let original_command = command(1, 1.0, &[("velocity_setpoint", vec![0.1])]);

        let mut sensor_conflict = Observer::new("run", "nest", "reach", Mapping::default());
        sensor_conflict
            .on_observation(&observation(1, vec![3.0]))
            .unwrap();
        sensor_conflict.on_sensor(&original_sensor).unwrap();
        sensor_conflict.on_command(&original_command).unwrap();
        sensor_conflict.flush_complete().unwrap();
        let original_v = sensor_conflict.sample(0).v.clone();
        sensor_conflict
            .on_sensor(&sensor(
                1,
                1.0,
                &[("pose", vec![2.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        sensor_conflict.on_sensor(&original_sensor).unwrap();
        assert_eq!(sensor_conflict.sample_count(), 1);
        assert_eq!(sensor_conflict.sample(0).v, original_v);
        assert_eq!(sensor_conflict.stats.conflicting_duplicates_dropped, 1);
        assert_eq!(sensor_conflict.stats.quarantined_frames_dropped, 1);
        assert_eq!(sensor_conflict.stats.redelivered_frames_dropped, 0);
        assert_eq!(sensor_conflict.stats.capture_integrity(), "invalid");

        let mut command_conflict = Observer::new("run", "nest", "reach", Mapping::default());
        command_conflict
            .on_observation(&observation(1, vec![3.0]))
            .unwrap();
        command_conflict.on_sensor(&original_sensor).unwrap();
        command_conflict.on_command(&original_command).unwrap();
        command_conflict.flush_complete().unwrap();
        let original_a = command_conflict.sample(0).a.clone();
        command_conflict
            .on_command(&command(1, 1.0, &[("velocity_setpoint", vec![0.2])]))
            .unwrap();
        command_conflict.on_command(&original_command).unwrap();
        assert_eq!(command_conflict.sample_count(), 1);
        assert_eq!(command_conflict.sample(0).a, original_a);
        assert_eq!(command_conflict.stats.conflicting_duplicates_dropped, 1);
        assert_eq!(command_conflict.stats.quarantined_frames_dropped, 1);
        assert_eq!(command_conflict.stats.redelivered_frames_dropped, 0);
        assert_eq!(command_conflict.stats.capture_integrity(), "invalid");
    }

    #[test]
    fn ancient_redelivery_cannot_reopen_a_closed_sample_id() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        let total = MAX_INFLIGHT as i64 + 9;
        for seq in 1..=total {
            obs.on_observation(&observation(seq, vec![seq as f64]))
                .unwrap();
            obs.on_sensor(&sensor(
                seq,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
            obs.on_command(&command(seq, 0.0, &[("velocity_setpoint", vec![0.1])]))
                .unwrap();
        }
        obs.flush_complete().unwrap();
        let before = obs.sample_count();
        assert_eq!(before, total as usize);

        obs.on_observation(&observation(1, vec![1.0])).unwrap();
        obs.on_sensor(&sensor(
            1,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(1, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();

        assert_eq!(obs.sample_count(), before);
        assert_eq!(obs.stats.redelivered_frames_dropped, 3);
        assert_eq!(obs.stats.late_d_dropped, 0);
    }

    #[test]
    fn raw_ingress_lifetime_budget_seals_capture() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_limits(ObserverLimits {
                max_wire_frames: 1,
                ..ObserverLimits::default()
            })
            .unwrap();
        let encoded = serde_json::to_vec(&sensor(
            1,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        let mut counters = RawIngressCounters::default();

        assert_eq!(
            ingest_wire_frame(&mut observer, IngressPlane::Sensor, &encoded, &mut counters,)
                .unwrap(),
            RawIngressDisposition::Applied
        );
        let mut reset_counters = RawIngressCounters::default();
        assert_eq!(
            ingest_wire_frame(
                &mut observer,
                IngressPlane::Sensor,
                &encoded,
                &mut reset_counters,
            )
            .unwrap(),
            RawIngressDisposition::CapacityDropped
        );
        assert_eq!(counters.frames_seen, 1);
        assert_eq!(reset_counters.lifetime_limit_dropped, 1);
        assert!(observer.stats.capture_capacity_reached);
        assert_eq!(observer.stats.ingress_lifetime_limit_dropped, 1);
        assert_eq!(observer.stats.capture_integrity(), "invalid");
    }

    #[test]
    fn future_epoch_command_waits_for_its_authorizing_sensor() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default());
        observer
            .on_observation(&observation_ep(EPOCH_A, 1, vec![3.0]))
            .unwrap();
        observer
            .on_sensor(&sensor_ep(
                EPOCH_A,
                1,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        observer
            .on_command(&command_ep(
                EPOCH_A,
                1,
                0.0,
                &[("velocity_setpoint", vec![0.1])],
            ))
            .unwrap();
        observer.flush_complete().unwrap();
        observer
            .on_command(&command_ep(
                EPOCH_B,
                1,
                0.0,
                &[("velocity_setpoint", vec![0.1])],
            ))
            .unwrap();
        observer
            .on_observation(&observation_ep(EPOCH_B, 1, vec![4.0]))
            .unwrap();

        assert_eq!(observer.active_epoch.as_deref(), Some(EPOCH_A));
        assert!(observer.pending.contains_key(&(EPOCH_B.to_string(), 1)));
        observer
            .on_sensor(&sensor_ep(
                EPOCH_B,
                1,
                0.0,
                &[("pose", vec![2.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        observer.flush_complete().unwrap();

        assert_eq!(observer.sample_count(), 2);
        assert_eq!(observer.sample(1).a, vec![0.1]);
        assert_eq!(observer.sample(1).d, vec![4.0]);
        assert_eq!(observer.stats.seq_resets, 1);
        assert_ne!(observer.stats.capture_integrity(), "invalid");
    }

    #[test]
    fn preactive_observations_are_keyed_by_full_stream_position() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default());
        observer
            .on_observation(&observation_ep(EPOCH_A, 1, vec![3.0]))
            .unwrap();
        observer
            .on_observation(&observation_ep(EPOCH_B, 1, vec![4.0]))
            .unwrap();
        assert_eq!(observer.d_by_key.len(), 2);
        assert_eq!(observer.stats.conflicting_duplicates_dropped, 0);

        observer
            .on_sensor(&sensor_ep(
                EPOCH_B,
                1,
                0.0,
                &[("pose", vec![2.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        observer
            .on_command(&command_ep(
                EPOCH_B,
                1,
                0.0,
                &[("velocity_setpoint", vec![0.2])],
            ))
            .unwrap();
        observer.flush_complete().unwrap();

        assert_eq!(observer.sample_count(), 1);
        assert_eq!(observer.sample(0).d, vec![4.0]);
    }

    #[test]
    fn passenger_frame_cannot_lock_or_cross_session_generation() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default());
        let mut foreign = command(1, 0.0, &[("velocity_setpoint", vec![0.1])]);
        foreign.session.generation = "00000000-0000-4000-8000-0000000000d4".into();
        observer.on_command(&foreign).unwrap();
        assert!(observer.expected_generation.is_none());

        observer
            .on_sensor(&sensor(
                1,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();

        assert!(observer.expected_generation.is_none());
        assert_eq!(observer.sample_count(), 0);
        assert_eq!(observer.stats.conflicting_duplicates_dropped, 1);
        assert_eq!(observer.stats.capture_integrity(), "invalid");
    }

    #[test]
    fn retired_epoch_receipts_classify_exact_and_conflicting_evidence() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default());
        let old_command = command_ep(EPOCH_A, 1, 0.0, &[("velocity_setpoint", vec![0.1])]);
        observer
            .on_observation(&observation_ep(EPOCH_A, 1, vec![3.0]))
            .unwrap();
        observer
            .on_sensor(&sensor_ep(
                EPOCH_A,
                1,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        observer.on_command(&old_command).unwrap();
        observer.flush_complete().unwrap();
        observer
            .on_sensor(&sensor_ep(
                EPOCH_B,
                1,
                0.0,
                &[("pose", vec![2.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();

        observer.on_command(&old_command).unwrap();
        assert_eq!(observer.stats.redelivered_frames_dropped, 1);
        let mut changed = old_command;
        changed.ttl_ms += 1.0;
        observer.on_command(&changed).unwrap();
        assert_eq!(observer.stats.conflicting_duplicates_dropped, 1);
        assert_eq!(observer.stats.capture_integrity(), "invalid");
    }

    #[test]
    fn invalid_sensor_cannot_mutate_generation_epoch_or_watermark() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default());
        observer
            .on_sensor(&sensor_ep(
                EPOCH_A,
                5,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        let before_generation = observer.expected_generation.clone();
        let mut invalid = sensor_ep(
            EPOCH_B,
            99,
            0.0,
            &[("pose", vec![2.0]), ("instruction", vec![0.5])],
        );
        invalid.channels.get_mut("pose").unwrap().data[0] = f64::NAN;
        observer.on_sensor(&invalid).unwrap();

        assert_eq!(observer.active_epoch.as_deref(), Some(EPOCH_A));
        assert_eq!(observer.max_seq, 5);
        assert_eq!(observer.expected_generation, before_generation);
        assert_eq!(observer.stats.seq_resets, 0);
        assert_eq!(observer.stats.invalid_payloads_dropped, 1);
        assert!(observer.pending.contains_key(&(EPOCH_A.to_string(), 5)));
    }

    #[test]
    fn capacity_seal_flushes_complete_grace_window_ticks() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default());
        observer.on_observation(&observation(1, vec![3.0])).unwrap();
        observer
            .on_sensor(&sensor(
                1,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        observer
            .on_command(&command(1, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        assert_eq!(observer.sample_count(), 0, "tick is inside reorder grace");

        observer.trip_capacity();

        assert_eq!(observer.sample_count(), 1);
        assert_eq!(observer.stats.evicted_incomplete, 0);
        assert_eq!(observer.stats.evicted_unclaimed_d, 0);
    }

    #[test]
    fn resident_inflight_element_budget_seals_before_retaining_candidate() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_limits(ObserverLimits {
                max_inflight_elements: 2,
                ..ObserverLimits::default()
            })
            .unwrap();
        observer
            .on_command(&command(1, 0.0, &[("velocity_setpoint", vec![0.1, 0.2])]))
            .unwrap();
        assert_eq!(observer.inflight_elements, 2);
        observer.on_observation(&observation(1, vec![3.0])).unwrap();

        assert_eq!(observer.stats.inflight_element_limit_dropped, 1);
        assert!(observer.stats.capture_capacity_reached);
        assert_eq!(observer.inflight_elements, 0);
        assert!(observer.pending.is_empty());
        assert!(observer.d_by_key.is_empty());
    }

    #[test]
    fn epoch_identity_budget_never_forgets_retired_incarnations() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default());
        for incarnation in 0..=(MAX_RETIRED_EPOCHS + 1) {
            let epoch = format!("00000000-0000-4000-8000-{incarnation:012x}");
            observer
                .on_sensor(&sensor_ep(
                    &epoch,
                    1,
                    0.0,
                    &[
                        ("pose", vec![incarnation as f64]),
                        ("instruction", vec![0.5]),
                    ],
                ))
                .unwrap();
        }

        assert_eq!(observer.retired_epochs.len(), MAX_RETIRED_EPOCHS);
        assert_eq!(observer.stats.epoch_limit_dropped, 1);
        assert!(observer.stats.capture_capacity_reached);
        assert_eq!(observer.stats.capture_integrity(), "invalid");
    }

    #[test]
    fn epoch_transition_accounts_for_every_inflight_record() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_sensor(&sensor_ep(
            EPOCH_A,
            1,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_observation(&observation_ep(EPOCH_A, 2, vec![3.0]))
            .unwrap();

        obs.on_sensor(&sensor_ep(
            EPOCH_B,
            1,
            0.0,
            &[("pose", vec![2.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();

        assert_eq!(obs.stats.incomplete_at_epoch_transition, 1);
        assert_eq!(obs.stats.incomplete_missing_command, 1);
        assert_eq!(obs.stats.unclaimed_d_at_epoch_transition, 1);
    }

    #[test]
    fn finalize_accounts_for_incomplete_v_a_and_unclaimed_d() {
        let dir = unique_test_dir("final_incomplete");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default())
            .with_expected_session(SID)
            .unwrap()
            .with_runlog(&runlog)
            .unwrap();
        obs.on_sensor(&sensor(
            1,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(2, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.on_observation(&observation(3, vec![3.0])).unwrap();

        let stats = obs.finalize(&dataset).unwrap();

        assert_eq!(stats.incomplete_at_finalize, 2);
        assert_eq!(stats.incomplete_missing_sensor, 1);
        assert_eq!(stats.incomplete_missing_command, 1);
        assert_eq!(stats.unclaimed_d_at_finalize, 1);
        assert_eq!(stats.capture_integrity(), "degraded");
        let events = pid_runlog::read_events_from_path(&runlog).unwrap();
        let summary = pid_runlog::summarize_events(&events).unwrap();
        assert_eq!(summary.status, Some(RunStatus::Failed));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn raw_ingress_and_capture_resource_limits_fail_closed() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_limits(ObserverLimits {
                max_wire_frame_bytes: 16,
                max_wire_frames: 10,
                max_total_wire_bytes: 1024,
                max_axis_values: 1,
                max_closed_ticks: 4,
                max_inflight_elements: 4,
                max_samples: 1,
                max_total_sample_elements: 4,
                max_artifact_bytes: 1024,
                max_runlog_bytes: 1024,
            })
            .unwrap();
        let mut counters = RawIngressCounters::default();
        let oversized = serde_json::to_vec(&sensor(
            1,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        assert_eq!(
            ingest_wire_frame(
                &mut observer,
                IngressPlane::Sensor,
                &oversized,
                &mut counters,
            )
            .unwrap(),
            RawIngressDisposition::OversizedDropped
        );
        assert_eq!(counters.oversized_frames, 1);

        observer
            .on_sensor(&sensor(
                1,
                0.0,
                &[("pose", vec![1.0, 2.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        assert_eq!(observer.stats.invalid_payloads_dropped, 1);
    }

    #[test]
    fn shared_raw_ingress_rejects_duplicate_json_keys() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default());
        let mut counters = RawIngressCounters::default();
        let encoded = String::from_utf8(
            serde_json::to_vec(&sensor(
                1,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap(),
        )
        .unwrap();
        let duplicate = format!(
            "{},\"kind\":\"sensor_frame\"}}",
            encoded.strip_suffix('}').unwrap()
        );

        assert_eq!(
            ingest_wire_frame(
                &mut observer,
                IngressPlane::Sensor,
                duplicate.as_bytes(),
                &mut counters,
            )
            .unwrap(),
            RawIngressDisposition::DecodeDropped
        );
        assert_eq!(counters.sensor_decode_failures, 1);
        assert_eq!(observer.sample_count(), 0);
    }

    #[test]
    fn ingress_routes_accept_only_exact_base_plane_keys() {
        let routes = IngressRoutes::new(
            "r/session/s/sensor",
            "r/session/s/command",
            "r/session/s/observation",
        )
        .unwrap();
        assert_eq!(
            routes.classify("r/session/s/sensor"),
            Some(IngressPlane::Sensor)
        );
        assert_eq!(routes.classify("r/session/s/sensor/camera"), None);
        assert_eq!(routes.classify("r/session/other/sensor"), None);
    }

    #[test]
    fn malformed_raw_ingress_cannot_finalize_as_complete_without_manual_merge() {
        let dir = unique_test_dir("raw_invalid_finalize");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_expected_session(SID)
            .unwrap()
            .with_runlog(&runlog)
            .unwrap();
        let mut diagnostics = RawIngressCounters::default();
        assert_eq!(
            ingest_wire_frame(
                &mut observer,
                IngressPlane::Sensor,
                b"not-json",
                &mut diagnostics,
            )
            .unwrap(),
            RawIngressDisposition::DecodeDropped
        );

        let stats = observer.finalize(&dataset).unwrap();

        assert_eq!(stats.capture_integrity(), "invalid");
        let artifact: OfflineVldaDataset =
            serde_json::from_slice(&std::fs::read(&dataset).unwrap()).unwrap();
        assert_eq!(artifact.capture_integrity, "invalid");
        assert!(publication_receipt_path(&dataset).exists());
        let summary =
            pid_runlog::summarize_events(&pid_runlog::read_events_from_path(&runlog).unwrap())
                .unwrap();
        assert_eq!(summary.status, Some(RunStatus::Failed));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn zero_receipt_capture_finalizes_as_failed_and_diagnostic_only() {
        let dir = unique_test_dir("zero_capture");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_expected_session(SID)
            .unwrap()
            .with_runlog(&runlog)
            .unwrap();

        let stats = observer.finalize(&dataset).unwrap();

        assert!(stats.zero_sample_capture);
        assert_eq!(stats.capture_integrity(), "degraded");
        let summary =
            pid_runlog::summarize_events(&pid_runlog::read_events_from_path(&runlog).unwrap())
                .unwrap();
        assert_eq!(summary.status, Some(RunStatus::Failed));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn sample_limit_seals_capture_without_unbounded_events() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default())
            .with_limits(ObserverLimits {
                max_samples: 1,
                max_closed_ticks: 4,
                max_total_sample_elements: 16,
                ..ObserverLimits::default()
            })
            .unwrap();
        for seq in 1..=2 {
            obs.on_observation(&observation(seq, vec![3.0])).unwrap();
            obs.on_sensor(&sensor(
                seq,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
            obs.on_command(&command(seq, 0.0, &[("velocity_setpoint", vec![0.1])]))
                .unwrap();
        }
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 1);
        assert_eq!(obs.stats.sample_limit_dropped, 1);
        assert!(obs.stats.capture_capacity_reached);
        obs.on_sensor(&sensor(
            3,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        assert_eq!(obs.stats.capture_capacity_dropped, 1);
    }

    #[test]
    fn closed_receipt_limit_remains_strict_during_capacity_flush() {
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_limits(ObserverLimits {
                max_closed_ticks: 2,
                max_samples: 2,
                ..ObserverLimits::default()
            })
            .unwrap();
        for seq in 1..=3 {
            observer
                .on_observation(&observation(seq, vec![seq as f64]))
                .unwrap();
            observer
                .on_sensor(&sensor(
                    seq,
                    0.0,
                    &[("pose", vec![1.0]), ("instruction", vec![0.5])],
                ))
                .unwrap();
            observer
                .on_command(&command(seq, 0.0, &[("velocity_setpoint", vec![0.1])]))
                .unwrap();
        }

        observer.flush_complete().unwrap();

        assert_eq!(observer.sample_count(), 2);
        assert_eq!(observer.closed_keys.len(), 2);
        assert_eq!(observer.closed_receipts.len(), 2);
        assert_eq!(observer.stats.capture_capacity_dropped, 1);
        assert_eq!(observer.stats.capture_integrity(), "invalid");
    }

    #[test]
    fn artifact_byte_limit_leaves_no_partial_output() {
        let dir = unique_test_dir("artifact_limit");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        // Construct the bounded observer in the required order and prove the
        // temporary file is removed on overflow.
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_limits(ObserverLimits {
                max_artifact_bytes: 16,
                ..ObserverLimits::default()
            })
            .unwrap()
            .with_expected_session(SID)
            .unwrap()
            .with_runlog(&runlog)
            .unwrap();
        observer.on_observation(&observation(7, vec![3.0])).unwrap();
        observer
            .on_sensor(&sensor(
                7,
                1.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        observer
            .on_command(&command(7, 1.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();

        let error = observer.finalize(&dataset).unwrap_err();
        assert!(error
            .to_string()
            .contains("serialize bounded NCP observer artifact"));
        assert!(!dataset.exists());
        assert!(!runlog.exists());
        assert!(std::fs::read_dir(&dir).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp-")));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn runlog_byte_limit_fails_before_any_publication_path_is_visible() {
        let dir = unique_test_dir("runlog_limit");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_limits(ObserverLimits {
                max_runlog_bytes: 1,
                ..ObserverLimits::default()
            })
            .unwrap()
            .with_expected_session(SID)
            .unwrap()
            .with_runlog(&runlog)
            .unwrap();
        observer.on_observation(&observation(1, vec![3.0])).unwrap();
        observer
            .on_sensor(&sensor(
                1,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
        observer
            .on_command(&command(1, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();

        let error = observer.finalize(&dataset).unwrap_err();

        assert!(format!("{error:#}").contains("reconstructed NCP run log exceeds"));
        assert!(!dataset.exists());
        assert!(!runlog.exists());
        assert!(!publication_receipt_path(&dataset).exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn inflated_observation_seq_does_not_fragment_the_session() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        for seq in 1..=5 {
            obs.on_observation(&observation(seq, vec![seq as f64]))
                .unwrap();
            obs.on_sensor(&sensor(
                seq,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
            obs.on_command(&command(seq, 0.0, &[("velocity_setpoint", vec![0.1])]))
                .unwrap();
        }
        // A garbage/inflated observation seq far above the control loop's
        // watermark must not move the clock: before the fix it made every
        // subsequent legitimate frame look like a session reset.
        obs.on_observation(&observation(1_000_000, vec![9.0]))
            .unwrap();
        for seq in 6..=20 {
            obs.on_observation(&observation(seq, vec![seq as f64]))
                .unwrap();
            obs.on_sensor(&sensor(
                seq,
                0.0,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
            obs.on_command(&command(seq, 0.0, &[("velocity_setpoint", vec![0.1])]))
                .unwrap();
        }
        obs.flush_complete().unwrap();
        assert_eq!(obs.stats.seq_resets, 0, "no epoch transition");
        assert_eq!(
            obs.active_epoch.as_deref(),
            Some(EPOCH_A),
            "single incarnation"
        );
        assert_eq!(obs.sample_count(), 20, "all ticks emitted");
    }

    #[test]
    fn inflight_maps_are_bounded() {
        // A long-running tap that sees many never-completing seqs must not grow
        // `pending`/`d_by_key` without bound.
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        for seq in 1..(MAX_INFLIGHT as i64 + 500) {
            // Only a sensor (no matching command) → this seq never completes.
            obs.on_sensor(&sensor(seq, 0.0, &[("pose", vec![1.0])]))
                .unwrap();
        }
        assert!(
            obs.pending.len() <= MAX_INFLIGHT,
            "pending must stay bounded by MAX_INFLIGHT, got {}",
            obs.pending.len()
        );
        assert_eq!(obs.sample_count(), 0, "no seq completed");
        assert!(obs.stats.evicted_incomplete >= 499);
    }

    #[test]
    fn epoch_transition_starts_new_incarnation_and_does_not_starve() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // Fill pending with stale, never-completing seqs on incarnation A, past
        // the cap.
        for seq in 100_000..(100_000 + MAX_INFLIGHT as i64 + 10) {
            obs.on_sensor(&sensor_ep(EPOCH_A, seq, 0.0, &[("pose", vec![1.0])]))
                .unwrap();
        }
        // The sensor stream restarts under a FRESH `stream.epoch` (B) at seq 1
        // (wire 0.8 signals the restart by the epoch change, not a seq jump): the
        // new tick must still produce a sample (lowest-key eviction would evict it
        // before its command arrived) and must NOT pair with any stale A state.
        obs.on_sensor(&sensor_ep(
            EPOCH_B,
            1,
            0.0,
            &[("pose", vec![7.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_observation(&observation_ep(EPOCH_B, 1, vec![4.0]))
            .unwrap();
        obs.on_command(&command_ep(
            EPOCH_B,
            1,
            0.0,
            &[("velocity_setpoint", vec![0.2])],
        ))
        .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 1, "post-transition tick must complete");
        assert_eq!(obs.stats.seq_resets, 1);
        let s = obs.sample(0);
        assert_eq!(
            s.sample_id,
            format!("ncp-{EPOCH_B}-1"),
            "epoch-qualified id"
        );
        assert_eq!(s.v, vec![7.0], "new-incarnation V only");
        assert_eq!(
            s.d,
            vec![4.0],
            "new-incarnation D only (pre-transition D was cleared)"
        );
        assert_eq!(
            s.metadata.get("d_source").map(String::as_str),
            Some("source"),
            "new-incarnation D must be joined exactly"
        );
    }

    #[test]
    fn adversarial_extreme_seq_does_not_panic() {
        // A hostile/garbage peer can send a source seq near i64::MAX/MIN; the
        // watermark + reorder arithmetic must saturate, never overflow (debug
        // panic in the Zenoh callback / release wrap that wedges the capture).
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation(i64::MAX, vec![1.0]))
            .unwrap();
        obs.on_sensor(&sensor(
            i64::MAX,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(i64::MAX, 0.0, &[("v", vec![0.1])]))
            .unwrap();
        obs.on_sensor(&sensor(i64::MIN + 1, 0.0, &[("pose", vec![2.0])]))
            .unwrap();
        obs.on_command(&command(i64::MIN + 1, 0.0, &[("v", vec![0.2])]))
            .unwrap();
        // No panic reaching here is the assertion; also flush cleanly.
        obs.flush_complete().unwrap();
    }

    #[test]
    fn unstamped_sensor_and_sourceless_command_frames_are_dropped() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // A sensor whose OWN `stream` is unset (seq 0) cannot be joined.
        obs.on_sensor(&sensor(
            0,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        // An open-loop command with NO `source` cannot be correlated to a tick.
        obs.on_command(&command_open_loop(0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 0, "unjoinable frames produce no sample");
        assert_eq!(obs.stats.unstamped_frames_dropped, 1);
        assert_eq!(
            obs.stats.invalid_payloads_dropped, 1,
            "an unset SensorFrame.stream is wire-invalid before join classification"
        );
    }

    #[test]
    fn foreign_session_and_stale_generation_frames_are_rejected() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default())
            .with_expected_session(SID)
            .unwrap();

        // A frame addressed to a DIFFERENT session_id must never blend in.
        let mut wrong_session = sensor(1, 0.0, &[("pose", vec![1.0])]);
        wrong_session.session_id = "other".into();
        obs.on_sensor(&wrong_session).unwrap();
        assert_eq!(obs.stats.session_mismatch_dropped, 1);

        // The first accepted frame locks the live generation.
        obs.on_sensor(&sensor(
            1,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_observation(&observation(1, vec![3.0])).unwrap();

        // A later frame from a STALE/foreign generation (a different incarnation)
        // is rejected, not mixed into the capture.
        let mut stale_gen = command(1, 0.0, &[("velocity_setpoint", vec![0.1])]);
        stale_gen.session.generation = "00000000-0000-4000-8000-0000000000d4".into();
        obs.on_command(&stale_gen).unwrap();
        assert_eq!(obs.stats.session_mismatch_dropped, 2);

        // The live-generation command still joins its sensor.
        obs.on_command(&command(1, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 1, "live-incarnation tick completes");
    }

    #[test]
    fn command_joins_on_source_not_its_own_stream() {
        // The correlation trap: a command's OWN action-plane `stream` differs from
        // the driving sensor `source`. Joining on `stream` would never pair V — a
        // silent zero-sample regression. Build a command whose own stream is a
        // DIFFERENT epoch/seq than its source and confirm it still pairs its sensor.
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation(5, vec![3.0])).unwrap();
        obs.on_sensor(&sensor(
            5,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        let mut cmd = command(5, 0.0, &[("velocity_setpoint", vec![0.1])]);
        // Own action-plane stream is unrelated to the driving sensor position.
        cmd.stream = spos(EPOCH_B, 999);
        obs.on_command(&cmd).unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(
            obs.sample_count(),
            1,
            "command must join on source (sensor 5), not its own stream"
        );
        assert_eq!(obs.sample(0).sample_id, format!("ncp-{EPOCH_A}-5"));
    }

    #[test]
    fn finalize_writes_valid_runlog_with_artifact_registration() {
        let dir = unique_test_dir("valid_finalize");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default())
            .with_expected_session(SID)
            .unwrap()
            .with_runlog(&runlog)
            .unwrap();
        // Ticks with DESCENDING sensor times: the monotonic run-log clock must
        // clamp so validation still passes.
        for seq in [7i64, 8, 9] {
            obs.on_observation(&observation(seq, vec![3.0])).unwrap();
            obs.on_sensor(&sensor(
                seq,
                (10 - seq) as f64,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ))
            .unwrap();
            obs.on_command(&command(seq, (10 - seq) as f64, &[("v", vec![0.1])]))
                .unwrap();
        }
        let stats = obs.finalize(&dataset).unwrap();
        assert_eq!(stats.kept_samples, 3);
        let report = pid_runlog::validate_events_from_path(&runlog).unwrap();
        assert_eq!(
            report.errors, 0,
            "ncp-observer run logs must pass canonical validation: {:?}",
            report.issues
        );
        let events = pid_runlog::read_events_from_path(&runlog).unwrap();
        assert!(
            events.iter().any(|e| matches!(
                e,
                RunLogEvent::ArtifactLogged { kind, sha256: Some(_), .. }
                    if kind.as_str() == "dataset_json"
            )),
            "finalize must register the dataset artifact with a sha256"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn artifact_write_failure_preserves_samples_for_retry() {
        assert_finalize_retry_reconstructs(FailStage::ArtifactWrite);
    }

    #[test]
    fn hash_failure_preserves_samples_for_retry() {
        assert_finalize_retry_reconstructs(FailStage::Hash);
    }

    #[test]
    fn runlog_append_failure_reconstructs_without_duplicate_events() {
        assert_finalize_retry_reconstructs(FailStage::Append);
    }

    #[test]
    fn runlog_write_failure_reconstructs_without_duplicate_events() {
        assert_finalize_retry_reconstructs(FailStage::RunlogWrite);
    }

    #[test]
    fn receipt_write_failure_reconstructs_committed_bundle_on_retry() {
        assert_finalize_retry_reconstructs(FailStage::ReceiptWrite);
    }

    #[test]
    fn finalize_without_canonical_runlog_fails_before_writing_artifact() {
        let dir = unique_test_dir("missing_runlog");
        std::fs::create_dir_all(&dir).unwrap();
        let dataset = dir.join("vlda.json");
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default());

        let error = observer.finalize(&dataset).unwrap_err();

        assert!(error.to_string().contains("canonical run log is required"));
        assert!(!dataset.exists());
        assert!(!observer.finalization_started);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn failed_finalize_seals_capture_but_remains_retryable() {
        let dir = unique_test_dir("failed_seal");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut observer = observer_with_exact_sample(&runlog);
        let mut failing = FailOnceIo::new(FailStage::ArtifactWrite);

        observer
            .finalize_with_io(&dataset, &mut failing)
            .unwrap_err();
        let mutation_error = observer
            .on_observation(&observation(8, vec![9.0]))
            .unwrap_err();

        assert!(mutation_error
            .to_string()
            .contains("finalization has started"));
        observer.finalize(&dataset).unwrap();
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn failed_finalize_cannot_be_retried_to_a_different_artifact() {
        let dir = unique_test_dir("retry_path");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let first = dir.join("first.json");
        let second = dir.join("second.json");
        let mut observer = observer_with_exact_sample(&runlog);
        let mut failing = FailOnceIo::new(FailStage::ArtifactWrite);
        observer.finalize_with_io(&first, &mut failing).unwrap_err();

        let error = observer.finalize(&second).unwrap_err();

        assert!(error
            .to_string()
            .contains("canonical bundle targets changed"));
        observer.finalize(&first).unwrap();
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn aliased_dataset_and_runlog_paths_are_rejected() {
        let dir = unique_test_dir("aliased_paths");
        std::fs::create_dir_all(&dir).unwrap();
        let dataset = dir.join("artifact.json");
        let aliased_runlog = dir.join(".").join("artifact.json");
        let mut observer = Observer::new("run", "nest", "reach", Mapping::default())
            .with_expected_session(SID)
            .unwrap()
            .with_runlog(aliased_runlog)
            .unwrap();

        let error = observer.finalize(&dataset).unwrap_err();

        assert!(error.to_string().contains("must be different"));
        assert!(!observer.finalization_started);
        std::fs::remove_dir_all(dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_bundle_target_fails_before_publication() {
        use std::os::unix::ffi::OsStringExt;

        let dir = unique_test_dir("non_utf8_target");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join(OsString::from_vec(b"vlda-\xff.json".to_vec()));
        let mut observer = observer_with_exact_sample(&runlog);

        let error = observer.finalize(&dataset).unwrap_err();

        assert!(error.to_string().contains("not valid UTF-8"));
        assert!(!dataset.exists());
        assert!(!runlog.exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn finalization_retry_rejects_retargeted_runlog_parent() {
        use std::os::unix::fs::symlink;

        let dir = unique_test_dir("retargeted_runlog_parent");
        let first = dir.join("first");
        let second = dir.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let link = dir.join("logs");
        symlink(&first, &link).unwrap();
        let runlog = link.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut observer = observer_with_exact_sample(&runlog);
        let mut failing = FailOnceIo::new(FailStage::ReceiptWrite);

        observer
            .finalize_with_io(&dataset, &mut failing)
            .unwrap_err();
        assert!(first.join("runlog.jsonl").exists());
        std::fs::remove_file(&link).unwrap();
        symlink(&second, &link).unwrap();

        let error = observer.finalize(&dataset).unwrap_err();

        assert!(error
            .to_string()
            .contains("canonical bundle targets changed"));
        assert!(!second.join("runlog.jsonl").exists());
        assert!(!publication_receipt_path(&dataset).exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn finalization_uses_pinned_targets_when_parent_is_retargeted_during_write() {
        use std::os::unix::fs::symlink;

        let dir = unique_test_dir("retarget_during_write");
        let first = dir.join("first");
        let second = dir.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let link = dir.join("output");
        symlink(&first, &link).unwrap();
        let dataset = link.join("vlda.json");
        let runlog = dir.join("runlog.jsonl");
        let mut observer = observer_with_exact_sample(&runlog);
        let mut io = RetargetOnArtifactIo {
            link: link.clone(),
            replacement: second.clone(),
            retargeted: false,
            fs: FsFinalizeIo,
        };

        observer.finalize_with_io(&dataset, &mut io).unwrap();

        let pinned_dataset = first.join("vlda.json");
        let pinned_receipt = publication_receipt_path(&pinned_dataset);
        assert!(pinned_dataset.exists());
        assert!(pinned_receipt.exists());
        assert!(!second.join("vlda.json").exists());
        let written: OfflineVldaDataset =
            serde_json::from_slice(&std::fs::read(&pinned_dataset).unwrap()).unwrap();
        assert_eq!(
            std::fs::canonicalize(&written.publication_receipt).unwrap(),
            std::fs::canonicalize(&pinned_receipt).unwrap()
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn retry_adopts_an_exact_runlog_installed_before_an_io_error() {
        let dir = unique_test_dir("post_install_retry");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut observer = observer_with_exact_sample(&runlog);
        let mut failing = FailAfterRunlogInstallIo {
            failed: false,
            fs: FsFinalizeIo,
        };

        let error = observer
            .finalize_with_io(&dataset, &mut failing)
            .unwrap_err();
        assert!(error.to_string().contains("post-install"));
        assert!(
            runlog.exists(),
            "the complete atomic install already occurred"
        );

        observer.finalize(&dataset).unwrap();
        let report = pid_runlog::validate_events_from_path(&runlog).unwrap();
        assert_eq!(report.errors, 0, "retry must preserve a canonical run log");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn retry_rejects_semantically_equal_but_nonidentical_artifact_bytes() {
        let dir = unique_test_dir("retry_exact_bytes");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut observer = observer_with_exact_sample(&runlog);
        let mut failing = FailOnceIo::new(FailStage::RunlogWrite);
        observer
            .finalize_with_io(&dataset, &mut failing)
            .unwrap_err();
        let mut changed = std::fs::read(&dataset).unwrap();
        changed.push(b'\n');
        std::fs::write(&dataset, changed).unwrap();

        let error = observer.finalize(&dataset).unwrap_err();

        assert!(error.to_string().contains("non-byte-identical artifact"));
        assert!(!publication_receipt_path(&dataset).exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn atomic_write_failure_leaves_existing_artifact_untouched() {
        let dir = unique_test_dir("atomic_failure");
        std::fs::create_dir_all(&dir).unwrap();
        let artifact = dir.join("artifact.json");
        std::fs::write(&artifact, b"original").unwrap();
        let error = atomic_write_with(&artifact, |writer| {
            writer.write_all(b"partial")?;
            anyhow::bail!("injected short write")
        })
        .unwrap_err();
        assert!(error.to_string().contains("injected short write"));
        assert_eq!(std::fs::read(&artifact).unwrap(), b"original");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn successful_finalize_seals_observer_against_post_event_mutation() {
        let dir = unique_test_dir("sealed");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut observer = observer_with_exact_sample(&runlog);
        observer.finalize(&dataset).unwrap();
        let artifact_before = std::fs::read(&dataset).unwrap();

        let error = observer
            .on_observation(&observation(7, vec![99.0]))
            .unwrap_err();
        assert!(error.to_string().contains("finalized"));
        assert_eq!(std::fs::read(&dataset).unwrap(), artifact_before);
        std::fs::remove_dir_all(dir).ok();
    }
}
