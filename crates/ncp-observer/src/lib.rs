//! # ncp-observer — prisoma's passive NCP tap
//!
//! Engram (NEST, via the Neuro-Cybernetic Protocol) becomes another `(V,L,D,A)`
//! source for prisoma's Partial Information Decomposition — exactly the role
//! `experiments/safe_adapter` plays for SAFE rollouts. This crate is a
//! **read-only observer**: it subscribes to the NCP data-plane keys
//! (`…/session/{id}/{sensor,command,observation}`) and converts each closed-loop
//! tick into an `OfflineVldaSample`, writing both
//!
//! 1. an `OfflineVldaDataset` JSON artifact (what `pid-offline-harness` runs the
//!    `V/L/D → A` PID screens on), and
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
//!   axes. A fixed-dim zero backfill that would *retain* such ticks (with the
//!   degraded `l_source = "absent_zeroed"` marker) is tracked future work
//!   (NCP_DEV_PROMPT Gap 2); until then a no-language session yields an empty
//!   artifact with a loud exclusion count, never a silently fabricated axis.
//! - **D** (dynamics / internal world-model) ← `ObservationFrame` record-port
//!   readouts — the neural state *before* the motor head, the "internal
//!   simulation" the PID(V,D;A) probe targets.
//! - **A** (action) ← `CommandFrame` channels, flattened.
//!
//! ## Alignment (the correctness rule)
//! V and A are joined on **`seq`** — a `CommandFrame.seq` echoes the
//! `SensorFrame.seq` it was computed from, so a sample pairs the action with the
//! sensor that produced it (never by arrival time, which the DROP QoS on the
//! perception plane would corrupt). `ObservationFrame` carries `seq` too, so D
//! aligns on `seq` as well. Because the observation plane rides a lower-priority
//! QoS class than the action plane, a tick's D routinely *arrives after* its
//! command: completed ticks are therefore held for a **reorder grace window**
//! (`REORDER_GRACE` newer seqs) before emission, so a late seq-stamped readout
//! still claims its own tick. After a tick emits, its artifact row and canonical
//! event are immutable: later readouts are dropped and counted, never patched.
//! Observation-plane `seq == 0` is the pull/RPC form and is dropped; there is no
//! recency fallback or future-D pairing.
//!
//! ## Sessions, resets, and unstamped frames
//! - `seq == 0` sensor/command frames are treated as **unstamped** (the same
//!   convention `ObservationFrame` documents upstream: "0 = no controller seq")
//!   — they cannot be joined reliably, so they are dropped and counted.
//! - A stamped seq arriving more than `MAX_INFLIGHT` behind the watermark is
//!   treated as a **session/seq reset**: complete in-flight ticks are flushed,
//!   per-epoch state is cleared (so an old epoch's V can never pair with a new
//!   epoch's A), and `sample_id`s carry the epoch (`ncp-{epoch}-{seq}`) so they
//!   stay unique across resets.
//! - In-flight state is bounded by `MAX_INFLIGHT` with **insertion-order
//!   (FIFO) eviction** — never lowest-key eviction, which would starve new
//!   low-seq ticks after a reset.
//!
//! ## Honesty provenance (per-sample `metadata`, mirrored into the run log)
//! Every kept sample carries provenance markers so a degraded axis is never
//! silently presented as real data — and the same markers are mirrored into the
//! `EmbeddingCaptured` metadata so the run log records them independently:
//! - `l_source` = `"channel"` (the language channel was present; empty-L ticks
//!   are excluded, see above).
//! - `d_source` = `"seq"` only. Missing or unstamped D never enters an artifact;
//!   those ticks are excluded and counted.
//! - `seq` and `epoch` — the join key and reset epoch for reconstruction.

use anyhow::Context as _;
use ncp_core::{ChannelValue, CommandFrame, ObservationFrame, SensorFrame};
use pid_runlog::{
    Actor, ActorType, EmbeddingVariableContract, RunLogEvent, RunLogWriter, RunStatus,
    RUN_LOG_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// One `(V,L,D,A)` sample — mirrors `pid-sim`'s `OfflineVldaSample` so the
/// emitted artifact runs directly through `pid-offline-harness`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
pub struct OfflineVldaDataset {
    pub run_id: String,
    pub source: String,
    pub model: String,
    pub task: String,
    pub samples: Vec<OfflineVldaSample>,
}

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

/// Capture-quality counters, reported by [`Observer::finalize`]. Every path that
/// silently loses or degrades data in a long-running tap is counted here so the
/// operator sees it instead of discovering an inexplicably small artifact.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct ObserverStats {
    /// Samples retained for the dataset artifact (and preserved across failed
    /// finalization attempts).
    pub kept_samples: usize,
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
    /// Seq-stamped D readouts that arrived after their tick was closed, or were
    /// too old to pair exactly, and were dropped without mutating an artifact row.
    pub late_d_dropped: usize,
    /// `seq == 0` frames dropped as unstamped (unjoinable on a data plane).
    pub unstamped_frames_dropped: usize,
    /// Wire frames rejected by the binary's validated decoder before they
    /// reached the observer state machine.
    pub ingress_decode_dropped: u64,
    /// Pull/RPC-form observations (`seq == 0`) rejected by the binary at the
    /// observation-plane medium boundary.
    pub ingress_unstamped_observations_dropped: u64,
    /// Frames rejected because the bounded callback-to-worker handoff was full
    /// or closed.
    pub ingress_handoff_dropped: u64,
    /// Sensor/command frames dropped because their `seq` already emitted a
    /// sample this epoch (transport re-delivery): re-admitting one would
    /// re-emit a second sample with the same `sample_id` (`ncp-{epoch}-{seq}`),
    /// double-counting the (V,L,D,A) row and violating the harness's
    /// `sample_id` uniqueness.
    pub redelivered_frames_dropped: usize,
    /// Never-completed in-flight ticks evicted by the `MAX_INFLIGHT` bound.
    pub evicted_incomplete: usize,
    /// Unclaimed seq-stamped D readouts evicted by the `MAX_INFLIGHT` bound.
    pub evicted_unclaimed_d: usize,
    /// Session/seq resets detected (each starts a new epoch).
    pub seq_resets: u32,
}

impl ObserverStats {
    fn summary(&self) -> String {
        format!(
            "kept={} excluded(empty v/l/d/a)={}/{}/{}/{} dim_mismatch={} \
             late_d_dropped={} unstamped={} ingress(decode/seq0/handoff)={}/{}/{} \
             redelivered={} evicted(pending/d)={}/{} seq_resets={}",
            self.kept_samples,
            self.excluded_empty_v,
            self.excluded_empty_l,
            self.excluded_empty_d,
            self.excluded_empty_a,
            self.dim_mismatch_dropped,
            self.late_d_dropped,
            self.unstamped_frames_dropped,
            self.ingress_decode_dropped,
            self.ingress_unstamped_observations_dropped,
            self.ingress_handoff_dropped,
            self.redelivered_frames_dropped,
            self.evicted_incomplete,
            self.evicted_unclaimed_d,
            self.seq_resets,
        )
    }
}

fn flatten_except(channels: &BTreeMap<String, ChannelValue>, except: &[&str]) -> Vec<f64> {
    let mut out = Vec::new();
    // BTreeMap iterates in sorted key order → deterministic concatenation.
    for (name, cv) in channels {
        if except.contains(&name.as_str()) {
            continue;
        }
        out.extend_from_slice(&cv.data);
    }
    out
}

/// Cap on the number of in-flight partial samples and unmatched D readouts kept
/// in memory. A long-running tap can accumulate `seq`s that never complete (a
/// sensor with no matching command, or vice-versa) or observations whose tick
/// never arrives; without a bound these maps grow without limit. Eviction is
/// **insertion-order (FIFO)**, never lowest-key: after a session seq reset the
/// lowest keys are the *newest* entries, and lowest-key eviction would starve
/// every new tick while retaining the stale ones forever.
const MAX_INFLIGHT: usize = 4096;

/// How many newer stamped seqs must be observed before a V+A-complete tick is
/// emitted. The observation plane rides a lower QoS priority than the action
/// plane, so a tick's seq-stamped D readout routinely arrives *after* its
/// command; holding completed ticks for a few seqs lets that readout claim its
/// own tick instead of being silently dropped. `finalize` flushes regardless.
const REORDER_GRACE: i64 = 8;

/// A stamped seq this far behind the watermark is a session/seq reset, not a
/// straggler. Matches [`MAX_INFLIGHT`] so anything old enough to have been
/// evicted is treated as a new epoch.
const RESET_MARGIN: i64 = MAX_INFLIGHT as i64;

/// Accumulates NCP frames into `(V,L,D,A)` samples, joining V↔A (and D) on `seq`.
pub struct Observer {
    run_id: String,
    model: String,
    task: String,
    mapping: Mapping,
    /// seq → partial sample (sensor seen, awaiting its command, or vice-versa).
    pending: BTreeMap<i64, Partial>,
    /// Insertion order of `pending` keys, for FIFO eviction.
    pending_order: VecDeque<i64>,
    /// D readouts keyed by the observation's `seq`, for exact alignment when the
    /// publisher stamps observations with the driving sensor's `seq`.
    d_by_seq: BTreeMap<i64, Vec<f64>>,
    /// Insertion order of `d_by_seq` keys, for FIFO eviction.
    d_order: VecDeque<i64>,
    /// Highest stamped seq seen this epoch (the emission watermark).
    max_seq: i64,
    /// Session/seq-reset epoch (0 for the first).
    epoch: u32,
    /// Seqs already emitted or excluded this epoch. Once closed, a seq is
    /// immutable: redelivery and late D are dropped rather than reconstructing
    /// a second row or mutating an already-buffered event.
    closed_seqs: BTreeSet<i64>,
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
    /// Set before the first artifact write. Once finalization begins, frame
    /// ingestion stays sealed even if I/O fails, so an exact retry cannot be
    /// invalidated by post-failure mutation.
    finalization_started: bool,
    /// Canonical destination bound by the first finalization attempt. Retries
    /// cannot redirect the same event buffer to a different artifact.
    finalize_dataset_target: Option<PathBuf>,
    finalized: bool,
}

#[derive(Default)]
struct Partial {
    v: Option<Vec<f64>>,
    /// Language channel contents; empty when the channel was absent (such ticks
    /// are excluded from the artifact and counted — see the module docs).
    l: Option<Vec<f64>>,
    l_present: bool,
    a: Option<Vec<f64>>,
    success: Option<serde_json::Value>,
    t: f64,
}

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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

/// Write a same-directory temporary file, fsync it, atomically rename it into
/// place, then fsync the directory entry. The destination is untouched when the
/// write/flush/fsync phase fails.
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
        std::fs::rename(&temp_path, path).with_context(|| {
            format!(
                "failed to atomically install {} from {}",
                path.display(),
                temp_path.display()
            )
        })?;
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

/// Re-establish durability when a previous atomic install reached `rename` but
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

trait FinalizeIo {
    fn write_artifact(&mut self, path: &Path, dataset: &OfflineVldaDataset) -> anyhow::Result<()>;
    fn hash_artifact(&mut self, path: &Path) -> anyhow::Result<String>;
    fn append_runlog(&mut self, events: &[RunLogEvent]) -> anyhow::Result<Vec<u8>>;
    fn write_runlog(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()>;
}

struct FsFinalizeIo;

impl FinalizeIo for FsFinalizeIo {
    fn write_artifact(&mut self, path: &Path, dataset: &OfflineVldaDataset) -> anyhow::Result<()> {
        atomic_write_with(path, |writer| {
            serde_json::to_writer_pretty(writer, dataset)
                .context("failed to serialize NCP observer artifact")
        })
    }

    fn hash_artifact(&mut self, path: &Path) -> anyhow::Result<String> {
        pid_runlog::sha256_file(path).context("failed to hash NCP observer artifact")
    }

    fn append_runlog(&mut self, events: &[RunLogEvent]) -> anyhow::Result<Vec<u8>> {
        let mut writer = RunLogWriter::new(Vec::new());
        for event in events {
            writer.append(event)?;
        }
        writer.flush()?;
        Ok(writer.into_inner())
    }

    fn write_runlog(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
        atomic_write_with(path, |writer| {
            writer
                .write_all(bytes)
                .context("failed to write reconstructed NCP observer run log")
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
            d_by_seq: BTreeMap::new(),
            d_order: VecDeque::new(),
            max_seq: 0,
            epoch: 0,
            closed_seqs: BTreeSet::new(),
            max_ts: 0,
            contract_dims: None,
            samples: Vec::new(),
            runlog_path: None,
            runlog_events: Vec::new(),
            stats: ObserverStats::default(),
            n: 0,
            finalization_started: false,
            finalize_dataset_target: None,
            finalized: false,
        }
    }

    /// Attach a run-log so provenance events are emitted alongside the dataset.
    pub fn with_runlog(mut self, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        if self.runlog_path.is_some()
            || self.n != 0
            || !self.pending.is_empty()
            || !self.d_by_seq.is_empty()
            || !self.closed_seqs.is_empty()
            || self.stats != ObserverStats::default()
        {
            anyhow::bail!("run log must be attached exactly once, before frame ingestion");
        }
        self.runlog_path = Some(path.as_ref().to_path_buf());
        self.runlog_events.push(RunLogEvent::RunStarted {
            schema_version: RUN_LOG_SCHEMA_VERSION,
            run_id: self.run_id.clone(),
            timestamp_ns: 0,
            config_hash: "ncp-observer".into(),
            metadata: BTreeMap::from([("source".into(), "ncp".into())]),
        });
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

    /// Monotonic run-log timestamp for a sensor time `t` (seconds).
    fn stamp(&mut self, t: f64) -> u64 {
        let ts = (t * 1e9).max(0.0) as u64;
        self.max_ts = self.max_ts.max(ts);
        self.max_ts
    }

    /// Watermark + session-reset bookkeeping for a stamped (nonzero) seq.
    /// Returns `false` when the frame is unstamped and must be dropped.
    fn admit_seq(&mut self, seq: i64) -> bool {
        if seq < 1 {
            // Upstream convention (ObservationFrame.seq docs): 0 = no controller
            // seq. An unstamped/invalid sensor or command cannot be joined
            // reliably — all seq-0 frames would merge into one bogus sample.
            self.stats.unstamped_frames_dropped =
                self.stats.unstamped_frames_dropped.saturating_add(1);
            return false;
        }
        // `saturating_add`: `seq` is attacker-controlled off the wire, so a
        // near-`i64::MAX` value must not overflow (debug panic in the Zenoh
        // callback / release wrap that wedges reset detection).
        if seq.saturating_add(RESET_MARGIN) < self.max_seq {
            // Session/seq reset: flush what completed, then clear per-epoch
            // state so an old epoch's V can never pair with a new epoch's A
            // (and an old epoch's D can never leak into new-epoch samples).
            self.flush_complete_unchecked();
            self.pending.clear();
            self.pending_order.clear();
            self.d_by_seq.clear();
            self.d_order.clear();
            self.closed_seqs.clear();
            self.epoch = self.epoch.saturating_add(1);
            self.stats.seq_resets = self.stats.seq_resets.saturating_add(1);
            self.max_seq = seq;
        } else if seq > self.max_seq {
            self.max_seq = seq;
        }
        true
    }

    /// Ingest a `SensorFrame` (perception plane). Supplies V and L for its `seq`.
    pub fn on_sensor(&mut self, sensor: &SensorFrame) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        if !self.admit_seq(sensor.seq) {
            return Ok(());
        }
        if self.closed_seqs.contains(&sensor.seq) {
            // Same guard the observation path has: this seq already emitted
            // this epoch, so this is transport re-delivery — drop it instead of
            // re-creating a pending entry that would re-emit a duplicate
            // sample_id.
            self.stats.redelivered_frames_dropped =
                self.stats.redelivered_frames_dropped.saturating_add(1);
            return Ok(());
        }
        let l_channel = sensor.channels.get(&self.mapping.language_channel);
        let l_present = l_channel.is_some();
        let l = l_channel.map(|cv| cv.data.clone()).unwrap_or_default();
        // Exclude BOTH the language channel (it IS the L axis) and the success
        // channel (it IS the outcome label) from V — otherwise the per-tick
        // success outcome would leak into the V feature vector and any PID(V;A)
        // screen on this artifact would be measuring the label, not perception.
        let mut v_except: Vec<&str> = vec![self.mapping.language_channel.as_str()];
        if let Some(sc) = self.mapping.success_channel.as_deref() {
            v_except.push(sc);
        }
        let v = flatten_except(&sensor.channels, &v_except);
        let success = self
            .mapping
            .success_channel
            .as_ref()
            .and_then(|c| sensor.channels.get(c))
            .and_then(|cv| cv.data.first().copied())
            .map(|x| serde_json::json!(x != 0.0));
        if !self.pending.contains_key(&sensor.seq) {
            self.pending_order.push_back(sensor.seq);
        }
        let entry = self.pending.entry(sensor.seq).or_default();
        entry.v = Some(v);
        entry.l = Some(l);
        entry.l_present = l_present;
        entry.t = sensor.t;
        if success.is_some() {
            entry.success = success;
        }
        self.enforce_bounds();
        self.emit_ready();
        Ok(())
    }

    /// Ingest a `CommandFrame` (action plane). Supplies A for its `seq`.
    pub fn on_command(&mut self, command: &CommandFrame) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        if !self.admit_seq(command.seq) {
            return Ok(());
        }
        if self.closed_seqs.contains(&command.seq) {
            self.stats.redelivered_frames_dropped =
                self.stats.redelivered_frames_dropped.saturating_add(1);
            return Ok(());
        }
        let a = flatten_except(&command.channels, &[]);
        if !self.pending.contains_key(&command.seq) {
            self.pending_order.push_back(command.seq);
        }
        let entry = self.pending.entry(command.seq).or_default();
        entry.a = Some(a);
        if entry.t == 0.0 {
            entry.t = command.t;
        }
        self.enforce_bounds();
        self.emit_ready();
        Ok(())
    }

    /// Ingest an `ObservationFrame` (neural readback). Updates the D axis.
    pub fn on_observation(&mut self, obs: &ObservationFrame) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        if obs.seq < 1 {
            // `seq == 0` is only valid for pull/RPC replies. Promoting it to a
            // most-recent value can pair future D with an earlier tick, so a
            // passive plane observer must drop it rather than guess.
            self.stats.unstamped_frames_dropped =
                self.stats.unstamped_frames_dropped.saturating_add(1);
            return Ok(());
        }
        let mut d = Vec::new();
        // Deterministic order: records is a BTreeMap keyed by port.
        for ob in obs.records.values() {
            if !ob.values.is_empty() {
                d.extend_from_slice(&ob.values);
            } else if !ob.times.is_empty() {
                // spikes with no analog values → use the spike count as a scalar.
                d.push(ob.times.len() as f64);
            }
        }
        if d.is_empty() {
            return Ok(());
        }
        if self.closed_seqs.contains(&obs.seq)
            || obs.seq.saturating_add(RESET_MARGIN) < self.max_seq
        {
            // Once a row's canonical event exists it is immutable. A late
            // exact-seq readout is evidence of loss/reordering, not authority
            // to patch the artifact behind the run log's back.
            self.stats.late_d_dropped = self.stats.late_d_dropped.saturating_add(1);
            return Ok(());
        }
        if !self.d_by_seq.contains_key(&obs.seq) {
            self.d_order.push_back(obs.seq);
        }
        self.d_by_seq.insert(obs.seq, d);
        // Deliberately do NOT advance `max_seq` from an observation: D is a
        // passenger on the control-loop clock, and a hostile inflated seq must
        // not force a reset or premature emission.
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
        while let Some(seq) = self.pending_order.front() {
            if self.pending.contains_key(seq) {
                break;
            }
            self.pending_order.pop_front();
        }
        while let Some(seq) = self.d_order.front() {
            if self.d_by_seq.contains_key(seq) {
                break;
            }
            self.d_order.pop_front();
        }
        // … and compact outright in the pathological case where a stuck live
        // front hides unbounded stale entries behind it, so deque length is
        // strictly bounded by 2×MAX_INFLIGHT.
        if self.pending_order.len() > 2 * MAX_INFLIGHT {
            let pending = &self.pending;
            self.pending_order.retain(|seq| pending.contains_key(seq));
        }
        if self.d_order.len() > 2 * MAX_INFLIGHT {
            let d_by_seq = &self.d_by_seq;
            self.d_order.retain(|seq| d_by_seq.contains_key(seq));
        }
        while self.pending.len() > MAX_INFLIGHT {
            // Skip order entries whose key already completed (removed).
            match self.pending_order.pop_front() {
                Some(seq) => {
                    if self.pending.remove(&seq).is_some() {
                        self.stats.evicted_incomplete =
                            self.stats.evicted_incomplete.saturating_add(1);
                    }
                }
                None => break, // unreachable: order tracks every insertion
            }
        }
        while self.d_by_seq.len() > MAX_INFLIGHT {
            match self.d_order.pop_front() {
                Some(seq) => {
                    if self.d_by_seq.remove(&seq).is_some() {
                        self.stats.evicted_unclaimed_d =
                            self.stats.evicted_unclaimed_d.saturating_add(1);
                    }
                }
                None => break,
            }
        }
        // Bound the closed-seq replay guard. Keys are per-epoch and therefore
        // comparable; oldest numeric seqs leave first.
        while self.closed_seqs.len() > MAX_INFLIGHT {
            self.closed_seqs.pop_first();
        }
    }

    /// Emit every V+A-complete tick old enough to have cleared the reorder
    /// grace window, in ascending seq order.
    fn emit_ready(&mut self) {
        let cutoff = self.max_seq.saturating_sub(REORDER_GRACE);
        let ready: Vec<i64> = self
            .pending
            .range(..=cutoff)
            .filter(|(_, p)| p.v.is_some() && p.a.is_some())
            .map(|(&s, _)| s)
            .collect();
        for seq in ready {
            if let Some(partial) = self.pending.remove(&seq) {
                self.emit_sample(seq, partial);
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
        let ready: Vec<i64> = self
            .pending
            .iter()
            .filter(|(_, p)| p.v.is_some() && p.a.is_some())
            .map(|(&s, _)| s)
            .collect();
        for seq in ready {
            if let Some(partial) = self.pending.remove(&seq) {
                self.emit_sample(seq, partial);
            }
        }
    }

    fn emit_sample(&mut self, seq: i64, p: Partial) {
        self.closed_seqs.insert(seq);
        // D is admissible only when it carries this exact driving seq.
        let (d, d_source) = match self.d_by_seq.remove(&seq) {
            Some(d) => (d, "seq"),
            None => (Vec::new(), "absent"),
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
                return;
            }
            Some(_) => {}
        }

        let mut labels = BTreeMap::new();
        if let Some(s) = p.success {
            labels.insert("success".to_string(), s);
        }
        let mut metadata = BTreeMap::new();
        metadata.insert("seq".to_string(), seq.to_string());
        metadata.insert("epoch".to_string(), self.epoch.to_string());
        metadata.insert("source".to_string(), "ncp".to_string());
        // Honest provenance: D is exact-seq or the tick is excluded. A kept
        // sample's L is nonempty (empty-L ticks
        // were excluded above), and a nonempty L can only come from a present
        // language channel — so `l_source` is always `"channel"` here; the
        // old `"absent_zeroed"` branch was unreachable and misled readers
        // into thinking absent-L ticks were retained-and-marked.
        debug_assert!(p.l_present, "kept sample implies present language channel");
        metadata.insert("l_source".to_string(), "channel".to_string());
        metadata.insert("d_source".to_string(), d_source.to_string());
        let sample = OfflineVldaSample {
            // Epoch-qualified so ids stay unique across session seq resets.
            sample_id: format!("ncp-{}-{seq}", self.epoch),
            episode_id: self.mapping.episode_id.clone(),
            v,
            l,
            d,
            a,
            labels: labels.clone(),
            metadata,
        };
        self.buffer_runlog(&sample, p.t, &labels);
        self.samples.push(sample);
        self.stats.kept_samples = self.stats.kept_samples.saturating_add(1);
        self.n = self.n.saturating_add(1);
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

    /// Merge callback/decoder drop counts into the canonical finalization
    /// summary before writing the artifact and run log.
    pub fn record_ingress_drops(
        &mut self,
        decode_dropped: u64,
        unstamped_observations_dropped: u64,
        handoff_dropped: u64,
    ) -> anyhow::Result<()> {
        self.ensure_capturing()?;
        self.stats.ingress_decode_dropped = self
            .stats
            .ingress_decode_dropped
            .saturating_add(decode_dropped);
        self.stats.ingress_unstamped_observations_dropped = self
            .stats
            .ingress_unstamped_observations_dropped
            .saturating_add(unstamped_observations_dropped);
        self.stats.ingress_handoff_dropped = self
            .stats
            .ingress_handoff_dropped
            .saturating_add(handoff_dropped);
        Ok(())
    }

    #[cfg(test)]
    fn sample(&self, idx: usize) -> &OfflineVldaSample {
        &self.samples[idx]
    }

    /// Atomically finalize the dataset and its canonical run log.
    ///
    /// Samples and buffered events remain owned by the observer until every
    /// write, hash, append, fsync, and rename succeeds. On error the caller may
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
        let dataset_target = output_target(dataset_path)?;
        let runlog_target = output_target(&runlog_path)?;
        if runlog_target == dataset_target {
            anyhow::bail!("dataset and run-log paths must be different");
        }
        if self.finalization_started {
            if self.finalize_dataset_target.as_ref() != Some(&dataset_target) {
                anyhow::bail!(
                    "finalization retry must use the original dataset path {}",
                    self.finalize_dataset_target.as_deref().map_or_else(
                        || "<unknown>".to_string(),
                        |path| path.display().to_string()
                    )
                );
            }
        } else {
            if dataset_path.exists() {
                anyhow::bail!(
                    "refusing to overwrite an existing artifact {}",
                    dataset_path.display()
                );
            }
            if runlog_path.exists() {
                anyhow::bail!("refusing to overwrite an existing run log");
            }
            self.flush_complete_unchecked();
            self.finalize_dataset_target = Some(dataset_target);
            self.finalization_started = true;
        }
        let dataset = OfflineVldaDataset {
            run_id: self.run_id.clone(),
            source: "ncp".into(),
            model: self.model.clone(),
            task: self.task.clone(),
            samples: self.samples.clone(),
        };
        if dataset_path.exists() {
            let existing: OfflineVldaDataset =
                serde_json::from_slice(&std::fs::read(dataset_path).with_context(|| {
                    format!(
                        "failed to read existing artifact {} for retry",
                        dataset_path.display()
                    )
                })?)
                .with_context(|| {
                    format!(
                        "existing artifact {} is not a valid observer dataset",
                        dataset_path.display()
                    )
                })?;
            if existing != dataset {
                anyhow::bail!(
                    "refusing to overwrite non-matching artifact {}",
                    dataset_path.display()
                );
            }
            sync_installed_file(dataset_path)?;
        } else {
            io.write_artifact(dataset_path, &dataset)?;
        }
        let stats = self.stats.clone();
        let ts = self.max_ts;
        let sha256 = io.hash_artifact(dataset_path)?;
        let mut final_events = self.runlog_events.clone();
        final_events.push(RunLogEvent::ArtifactLogged {
            timestamp_ns: ts,
            name: "ncp_vlda_dataset".to_string(),
            kind: "dataset_json".to_string(),
            uri: dataset_path.display().to_string(),
            sha256: Some(sha256),
            metadata: BTreeMap::from([
                ("kept_samples".to_string(), stats.kept_samples.to_string()),
                ("capture_quality".to_string(), stats.summary()),
            ]),
        });
        final_events.push(RunLogEvent::RunEnded {
            run_id: self.run_id.clone(),
            timestamp_ns: ts,
            status: RunStatus::Succeeded,
            message: Some(format!(
                "{} (V,L,D,A) samples from NCP [{}]",
                dataset.samples.len(),
                stats.summary()
            )),
        });
        let runlog_bytes = io.append_runlog(&final_events)?;
        if runlog_path.exists() {
            let existing = std::fs::read(&runlog_path).with_context(|| {
                format!(
                    "failed to read existing run log {} for retry",
                    runlog_path.display()
                )
            })?;
            if existing != runlog_bytes {
                anyhow::bail!(
                    "refusing to overwrite non-matching run log {}",
                    runlog_path.display()
                );
            }
            sync_installed_file(&runlog_path)?;
        } else {
            io.write_runlog(&runlog_path, &runlog_bytes)?;
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
    use ncp_core::Map;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FailStage {
        ArtifactWrite,
        Hash,
        Append,
        RunlogWrite,
    }

    struct FailOnceIo {
        stage: Option<FailStage>,
        fs: FsFinalizeIo,
    }

    struct FailAfterRunlogInstallIo {
        failed: bool,
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
        fn write_artifact(
            &mut self,
            path: &Path,
            dataset: &OfflineVldaDataset,
        ) -> anyhow::Result<()> {
            self.fail(FailStage::ArtifactWrite)?;
            self.fs.write_artifact(path, dataset)
        }

        fn hash_artifact(&mut self, path: &Path) -> anyhow::Result<String> {
            self.fail(FailStage::Hash)?;
            self.fs.hash_artifact(path)
        }

        fn append_runlog(&mut self, events: &[RunLogEvent]) -> anyhow::Result<Vec<u8>> {
            self.fail(FailStage::Append)?;
            self.fs.append_runlog(events)
        }

        fn write_runlog(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fail(FailStage::RunlogWrite)?;
            self.fs.write_runlog(path, bytes)
        }
    }

    impl FinalizeIo for FailAfterRunlogInstallIo {
        fn write_artifact(
            &mut self,
            path: &Path,
            dataset: &OfflineVldaDataset,
        ) -> anyhow::Result<()> {
            self.fs.write_artifact(path, dataset)
        }

        fn hash_artifact(&mut self, path: &Path) -> anyhow::Result<String> {
            self.fs.hash_artifact(path)
        }

        fn append_runlog(&mut self, events: &[RunLogEvent]) -> anyhow::Result<Vec<u8>> {
            self.fs.append_runlog(events)
        }

        fn write_runlog(&mut self, path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
            self.fs.write_runlog(path, bytes)?;
            if !self.failed {
                self.failed = true;
                anyhow::bail!("injected post-install runlog failure");
            }
            Ok(())
        }
    }

    fn ch(data: Vec<f64>) -> ChannelValue {
        ChannelValue { data, unit: None }
    }

    fn sensor(seq: i64, t: f64, channels: &[(&str, Vec<f64>)]) -> SensorFrame {
        let mut sc = Map::new();
        for (name, data) in channels {
            sc.insert((*name).into(), ch(data.clone()));
        }
        SensorFrame {
            seq,
            t,
            channels: sc,
            ..Default::default()
        }
    }

    fn command(seq: i64, t: f64, channels: &[(&str, Vec<f64>)]) -> CommandFrame {
        let mut cc = Map::new();
        for (name, data) in channels {
            cc.insert((*name).into(), ch(data.clone()));
        }
        CommandFrame {
            seq,
            t,
            channels: cc,
            ..Default::default()
        }
    }

    fn observation(seq: i64, values: Vec<f64>) -> ObservationFrame {
        let mut records = Map::new();
        records.insert(
            "rate".into(),
            ncp_core::Observation {
                values,
                ..Default::default()
            },
        );
        ObservationFrame {
            seq,
            records,
            ..Default::default()
        }
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
        assert!(
            !runlog.exists(),
            "failed finalize must not publish a partial canonical log"
        );

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
        assert_eq!(s.sample_id, "ncp-0-7");
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
            Some("seq")
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
            Some("seq")
        );
    }

    #[test]
    fn observation_after_emission_is_dropped_without_mutating_sample() {
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
        obs.on_observation(&observation(7, vec![5.5])).unwrap();
        assert_eq!(obs.sample(0).d, before.d);
        assert_eq!(
            obs.sample(0).metadata.get("d_source").map(String::as_str),
            Some("seq")
        );
        assert_eq!(obs.stats.late_d_dropped, 1);
    }

    #[test]
    fn seq_zero_observation_is_dropped_without_future_d_pairing() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation(0, vec![3.0])).unwrap();
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
        assert_eq!(obs.stats.seq_resets, 0, "no session reset");
        assert_eq!(obs.epoch, 0, "single epoch");
        assert_eq!(obs.sample_count(), 20, "all ticks emitted");
    }

    #[test]
    fn inflight_maps_are_bounded() {
        // A long-running tap that sees many never-completing seqs must not grow
        // `pending`/`d_by_seq` without bound.
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
    fn seq_reset_starts_new_epoch_and_does_not_starve() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // Fill pending with stale, never-completing high seqs, past the cap.
        for seq in 100_000..(100_000 + MAX_INFLIGHT as i64 + 10) {
            obs.on_sensor(&sensor(seq, 0.0, &[("pose", vec![1.0])]))
                .unwrap();
        }
        // Session restarts at seq 1: the new tick must still produce a sample
        // (lowest-key eviction would evict it before its command arrived), and
        // it must NOT pair with any stale pre-reset state.
        obs.on_sensor(&sensor(
            1,
            0.0,
            &[("pose", vec![7.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_observation(&observation(1, vec![4.0])).unwrap();
        obs.on_command(&command(1, 0.0, &[("velocity_setpoint", vec![0.2])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 1, "post-reset tick must complete");
        assert_eq!(obs.stats.seq_resets, 1);
        let s = obs.sample(0);
        assert_eq!(s.sample_id, "ncp-1-1", "epoch-qualified id");
        assert_eq!(s.v, vec![7.0], "new-epoch V only");
        assert_eq!(s.d, vec![4.0], "new-epoch D only (pre-reset D was cleared)");
        assert_eq!(
            s.metadata.get("d_source").map(String::as_str),
            Some("seq"),
            "new-epoch D must be joined exactly"
        );
    }

    #[test]
    fn adversarial_extreme_seq_does_not_panic() {
        // A hostile/garbage peer can send seq near i64::MAX/MIN; the reset-detection
        // and reorder arithmetic must saturate, never overflow (debug panic in the
        // Zenoh callback / release wrap that wedges the capture).
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
    fn unstamped_sensor_and_command_frames_are_dropped() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_sensor(&sensor(
            0,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ))
        .unwrap();
        obs.on_command(&command(0, 0.0, &[("velocity_setpoint", vec![0.1])]))
            .unwrap();
        obs.flush_complete().unwrap();
        assert_eq!(obs.sample_count(), 0, "seq-0 frames are unjoinable");
        assert_eq!(obs.stats.unstamped_frames_dropped, 2);
    }

    #[test]
    fn finalize_writes_valid_runlog_with_artifact_registration() {
        let dir = unique_test_dir("valid_finalize");
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default())
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

        assert!(error.to_string().contains("original dataset path"));
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
            .with_runlog(aliased_runlog)
            .unwrap();

        let error = observer.finalize(&dataset).unwrap_err();

        assert!(error.to_string().contains("must be different"));
        assert!(!observer.finalization_started);
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
