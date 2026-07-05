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
//! ([`REORDER_GRACE`] newer seqs) before emission, so a late seq-stamped readout
//! still claims its own tick. A readout arriving after emission but with matching
//! dims still patches the in-memory sample (`d_source = "seq_late"`); only
//! readouts later than that are dropped — and counted. The remaining
//! D-alignment dependency is runtime-side and external: the Engram publisher
//! must stamp each `ObservationFrame` with its driving sensor `seq`
//! (see `NCP_DEV_PROMPT.md` Gap 1); unstamped (`seq == 0`) readouts only ever
//! update the most-recent fallback.
//!
//! ## Sessions, resets, and unstamped frames
//! - `seq == 0` sensor/command frames are treated as **unstamped** (the same
//!   convention `ObservationFrame` documents upstream: "0 = no controller seq")
//!   — they cannot be joined reliably, so they are dropped and counted.
//! - A stamped seq arriving more than [`MAX_INFLIGHT`] behind the watermark is
//!   treated as a **session/seq reset**: complete in-flight ticks are flushed,
//!   per-epoch state is cleared (so an old epoch's V can never pair with a new
//!   epoch's A), and `sample_id`s carry the epoch (`ncp-{epoch}-{seq}`) so they
//!   stay unique across resets.
//! - In-flight state is bounded by [`MAX_INFLIGHT`] with **insertion-order
//!   (FIFO) eviction** — never lowest-key eviction, which would starve new
//!   low-seq ticks after a reset.
//!
//! ## Honesty provenance (per-sample `metadata`, mirrored into the run log)
//! Every kept sample carries provenance markers so a degraded axis is never
//! silently presented as real data — and the same markers are mirrored into the
//! `EmbeddingCaptured` metadata so the run log records them independently:
//! - `l_source` = `"channel"` (the language channel was present; empty-L ticks
//!   are excluded, see above).
//! - `d_source` = `"seq"` (exact alignment), `"seq_late"` (exact alignment via a
//!   post-emission patch; the streamed `EmbeddingCaptured` predates the patch and
//!   still says `recency_fallback` — the finalize report counts these),
//!   `"recency_fallback"` (the publisher sent `obs.seq == 0`, so `D` is the
//!   most-recent readout, not the driving one — Gap 1). `"absent"`-D ticks are
//!   excluded from the artifact (empty axis) and counted.
//! - `seq` and `epoch` — the join key and reset epoch for reconstruction.

use ncp_core::{ChannelValue, CommandFrame, ObservationFrame, SensorFrame};
use pid_runlog::{
    Actor, ActorType, EmbeddingVariableContract, RunLogEvent, RunLogWriter, RunStatus,
    RUN_LOG_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::Path;

/// One `(V,L,D,A)` sample — mirrors `pid-sim`'s `OfflineVldaSample` so the
/// emitted artifact runs directly through `pid-offline-harness`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Default, Clone, Serialize)]
pub struct ObserverStats {
    /// Samples written to the dataset artifact.
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
    /// Seq-stamped D readouts that arrived after their tick was emitted but
    /// still patched the in-memory sample (`d_source = "seq_late"`).
    pub late_d_patched: usize,
    /// Seq-stamped D readouts that arrived too late (tick evicted from the
    /// patch window) or with mismatched dims, and were dropped.
    pub late_d_dropped: usize,
    /// `seq == 0` sensor/command frames dropped as unstamped (unjoinable).
    pub unstamped_frames_dropped: usize,
    /// Never-completed in-flight ticks evicted by the [`MAX_INFLIGHT`] bound.
    pub evicted_incomplete: usize,
    /// Unclaimed seq-stamped D readouts evicted by the [`MAX_INFLIGHT`] bound.
    pub evicted_unclaimed_d: usize,
    /// Session/seq resets detected (each starts a new epoch).
    pub seq_resets: u32,
}

impl ObserverStats {
    fn summary(&self) -> String {
        format!(
            "kept={} excluded(empty v/l/d/a)={}/{}/{}/{} dim_mismatch={} \
             late_d(patched/dropped)={}/{} unstamped={} evicted(pending/d)={}/{} seq_resets={}",
            self.kept_samples,
            self.excluded_empty_v,
            self.excluded_empty_l,
            self.excluded_empty_d,
            self.excluded_empty_a,
            self.dim_mismatch_dropped,
            self.late_d_patched,
            self.late_d_dropped,
            self.unstamped_frames_dropped,
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

/// Append a run-log event, surfacing (rather than silently swallowing) a write
/// failure. The run log is the source of truth, so a dropped event is a real
/// integrity loss and must at least be reported on stderr — never `let _ = `.
fn log_append(w: &mut RunLogWriter<BufWriter<File>>, ev: &RunLogEvent) {
    if let Err(e) = w.append(ev) {
        eprintln!("[ncp-observer] run-log append failed (event dropped): {e}");
    }
}

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
    /// Most recent observation readout (the D axis) for the session — the
    /// best-effort fallback when an observation carries no `seq`.
    latest_d: Vec<f64>,
    /// D readouts keyed by the observation's `seq`, for exact alignment when the
    /// publisher stamps observations with the driving sensor's `seq`.
    d_by_seq: BTreeMap<i64, Vec<f64>>,
    /// Insertion order of `d_by_seq` keys, for FIFO eviction.
    d_order: VecDeque<i64>,
    /// Highest stamped seq seen this epoch (the emission watermark).
    max_seq: i64,
    /// Session/seq-reset epoch (0 for the first).
    epoch: u32,
    /// seq → index into `samples`, current epoch only, for late-D patching.
    emitted_by_seq: BTreeMap<i64, usize>,
    /// Monotonic run-log clock: the max timestamp stamped so far. Sensor `t`
    /// values can complete out of order; clamping to the running max keeps the
    /// run log valid under pid-runlog's nondecreasing-timestamp rule.
    max_ts: u64,
    /// Dims of the first kept sample == the emitted `EmbeddingContract`.
    contract_dims: Option<[usize; 4]>,
    samples: Vec<OfflineVldaSample>,
    writer: Option<RunLogWriter<BufWriter<File>>>,
    stats: ObserverStats,
    n: u64,
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
            latest_d: Vec::new(),
            d_by_seq: BTreeMap::new(),
            d_order: VecDeque::new(),
            max_seq: 0,
            epoch: 0,
            emitted_by_seq: BTreeMap::new(),
            max_ts: 0,
            contract_dims: None,
            samples: Vec::new(),
            writer: None,
            stats: ObserverStats::default(),
            n: 0,
        }
    }

    /// Attach a run-log so provenance events are emitted alongside the dataset.
    pub fn with_runlog(mut self, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut w = RunLogWriter::create(path)?;
        w.append(&RunLogEvent::RunStarted {
            schema_version: RUN_LOG_SCHEMA_VERSION,
            run_id: self.run_id.clone(),
            timestamp_ns: 0,
            config_hash: "ncp-observer".into(),
            metadata: BTreeMap::from([("source".into(), "ncp".into())]),
        })?;
        self.writer = Some(w);
        Ok(self)
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
        if seq == 0 {
            // Upstream convention (ObservationFrame.seq docs): 0 = no controller
            // seq. An unstamped sensor/command cannot be joined reliably — all
            // seq-0 frames would merge into one bogus mixed sample.
            self.stats.unstamped_frames_dropped += 1;
            return false;
        }
        // `saturating_add`: `seq` is attacker-controlled off the wire, so a
        // near-`i64::MAX` value must not overflow (debug panic in the Zenoh
        // callback / release wrap that wedges reset detection).
        if seq.saturating_add(RESET_MARGIN) < self.max_seq {
            // Session/seq reset: flush what completed, then clear per-epoch
            // state so an old epoch's V can never pair with a new epoch's A
            // (and an old epoch's D can never leak into new-epoch samples).
            self.flush_complete();
            self.pending.clear();
            self.pending_order.clear();
            self.d_by_seq.clear();
            self.d_order.clear();
            self.latest_d.clear();
            self.emitted_by_seq.clear();
            self.epoch += 1;
            self.stats.seq_resets += 1;
            self.max_seq = seq;
        } else if seq > self.max_seq {
            self.max_seq = seq;
        }
        true
    }

    /// Ingest a `SensorFrame` (perception plane). Supplies V and L for its `seq`.
    pub fn on_sensor(&mut self, sensor: &SensorFrame) {
        if !self.admit_seq(sensor.seq) {
            return;
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
    }

    /// Ingest a `CommandFrame` (action plane). Supplies A for its `seq`.
    pub fn on_command(&mut self, command: &CommandFrame) {
        if !self.admit_seq(command.seq) {
            return;
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
    }

    /// Ingest an `ObservationFrame` (neural readback). Updates the D axis.
    pub fn on_observation(&mut self, obs: &ObservationFrame) {
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
            return;
        }
        // Exact alignment when the publisher stamps the driving seq; the
        // most-recent value remains the fallback for seq-less observations.
        if obs.seq != 0 {
            if let Some(&idx) = self.emitted_by_seq.get(&obs.seq) {
                // The tick already emitted (readout later than the grace
                // window). Patch the in-memory sample when dims allow, so the
                // artifact still gets the exactly-aligned D; the streamed
                // EmbeddingCaptured predates the patch, which the finalize
                // report makes visible via the late_d_patched count.
                if self.samples[idx].d.len() == d.len() {
                    self.samples[idx].d = d.clone();
                    self.samples[idx]
                        .metadata
                        .insert("d_source".to_string(), "seq_late".to_string());
                    self.stats.late_d_patched += 1;
                } else {
                    self.stats.late_d_dropped += 1;
                }
            } else if obs.seq.saturating_add(RESET_MARGIN) < self.max_seq {
                // Older than anything we could still patch or pair (saturating:
                // obs.seq is attacker-controlled, must not overflow).
                self.stats.late_d_dropped += 1;
            } else {
                if !self.d_by_seq.contains_key(&obs.seq) {
                    self.d_order.push_back(obs.seq);
                }
                self.d_by_seq.insert(obs.seq, d.clone());
                if obs.seq > self.max_seq {
                    self.max_seq = obs.seq;
                }
            }
        }
        self.latest_d = d;
        self.enforce_bounds();
        self.emit_ready();
    }

    /// Evict oldest-inserted in-flight state once either map exceeds
    /// [`MAX_INFLIGHT`]. Completed ticks are removed by `emit_ready`, so what
    /// accumulates here is never-completed `seq`s and unclaimed readouts.
    fn enforce_bounds(&mut self) {
        while self.pending.len() > MAX_INFLIGHT {
            // Skip order entries whose key already completed (removed).
            match self.pending_order.pop_front() {
                Some(seq) => {
                    if self.pending.remove(&seq).is_some() {
                        self.stats.evicted_incomplete += 1;
                    }
                }
                None => break, // unreachable: order tracks every insertion
            }
        }
        while self.d_by_seq.len() > MAX_INFLIGHT {
            match self.d_order.pop_front() {
                Some(seq) => {
                    if self.d_by_seq.remove(&seq).is_some() {
                        self.stats.evicted_unclaimed_d += 1;
                    }
                }
                None => break,
            }
        }
        // Bound the late-D patch window the same way (oldest seqs first is
        // correct here: `emitted_by_seq` is per-epoch, so keys are comparable).
        while self.emitted_by_seq.len() > MAX_INFLIGHT {
            self.emitted_by_seq.pop_first();
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
            let p = self.pending.remove(&seq).expect("collected above");
            self.emit_sample(seq, p);
        }
    }

    /// Emit ALL currently-complete ticks regardless of the grace window (used by
    /// `finalize`, session resets, and tests).
    pub fn flush_complete(&mut self) {
        let ready: Vec<i64> = self
            .pending
            .iter()
            .filter(|(_, p)| p.v.is_some() && p.a.is_some())
            .map(|(&s, _)| s)
            .collect();
        for seq in ready {
            let p = self.pending.remove(&seq).expect("collected above");
            self.emit_sample(seq, p);
        }
    }

    fn emit_sample(&mut self, seq: i64, p: Partial) {
        // Exact D when the observation for this seq was seen; else most-recent.
        let (d, d_source) = match self.d_by_seq.remove(&seq) {
            Some(d) => (d, "seq"),
            None if self.latest_d.is_empty() => (Vec::new(), "absent"),
            None => (self.latest_d.clone(), "recency_fallback"),
        };
        let v = p.v.unwrap_or_default();
        let l = p.l.unwrap_or_default();
        let a = p.a.unwrap_or_default();

        // Empty-axis ticks can never pass pid-offline-harness's validate_dataset
        // (nonempty, consistent dims), and one such sample would poison the whole
        // artifact — exclude and count instead of fabricating an axis (Gap 2).
        let mut excluded = false;
        if v.is_empty() {
            self.stats.excluded_empty_v += 1;
            excluded = true;
        }
        if l.is_empty() {
            self.stats.excluded_empty_l += 1;
            excluded = true;
        }
        if d.is_empty() {
            self.stats.excluded_empty_d += 1;
            excluded = true;
        }
        if a.is_empty() {
            self.stats.excluded_empty_a += 1;
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
                self.stats.dim_mismatch_dropped += 1;
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
        // Honest provenance: never present a recency-aligned D as if it were
        // seq-aligned (Gap 1). Kept samples always have a present language
        // channel (empty-L ticks were excluded above).
        metadata.insert(
            "l_source".to_string(),
            if p.l_present {
                "channel"
            } else {
                "absent_zeroed"
            }
            .to_string(),
        );
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
        self.emit_runlog(&sample, p.t, &labels);
        self.emitted_by_seq.insert(seq, self.samples.len());
        self.samples.push(sample);
        self.stats.kept_samples += 1;
        self.n += 1;
    }

    fn emit_runlog(
        &mut self,
        sample: &OfflineVldaSample,
        t: f64,
        labels: &BTreeMap<String, serde_json::Value>,
    ) {
        let ts = self.stamp(t);
        let step = self.n;
        let Some(w) = self.writer.as_mut() else {
            return;
        };
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
                log_append(
                    w,
                    &RunLogEvent::EmbeddingContract {
                        timestamp_ns: ts,
                        name: "vlda".into(),
                        variables: vec![
                            var("v", dims[0]),
                            var("l", dims[1]),
                            var("d", dims[2]),
                            var("a", dims[3]),
                        ],
                        metadata: BTreeMap::new(),
                    },
                );
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
        log_append(
            w,
            &RunLogEvent::EmbeddingCaptured {
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
            },
        );
        for (name, value) in labels {
            log_append(
                w,
                &RunLogEvent::LabelObserved {
                    step,
                    timestamp_ns: ts,
                    name: name.clone(),
                    value: value.clone(),
                    metadata: BTreeMap::new(),
                },
            );
        }
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    #[cfg(test)]
    fn sample(&self, idx: usize) -> &OfflineVldaSample {
        &self.samples[idx]
    }

    /// Finish: flush complete in-flight ticks, write the `(V,L,D,A)` dataset
    /// artifact, register it in the run log (uri + sha256), and close the run
    /// log with a monotonic timestamp and a capture-quality summary message.
    pub fn finalize(&mut self, dataset_path: impl AsRef<Path>) -> anyhow::Result<ObserverStats> {
        self.flush_complete();
        let dataset = OfflineVldaDataset {
            run_id: self.run_id.clone(),
            source: "ncp".into(),
            model: self.model.clone(),
            task: self.task.clone(),
            samples: std::mem::take(&mut self.samples),
        };
        let path = dataset_path.as_ref();
        let file = File::create(path)?;
        let mut bw = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut bw, &dataset)?;
        // BufWriter's Drop swallows flush errors; flush explicitly so a short
        // write surfaces instead of silently truncating the artifact.
        bw.flush()?;
        drop(bw);
        let stats = self.stats.clone();
        if let Some(mut w) = self.writer.take() {
            let ts = self.max_ts;
            let sha256 = pid_runlog::sha256_file(path)
                .map_err(|e| eprintln!("[ncp-observer] dataset sha256 failed: {e}"))
                .ok();
            // Register the artifact so the run log (source of truth) can locate
            // and verify the dataset it describes.
            w.append(&RunLogEvent::ArtifactLogged {
                timestamp_ns: ts,
                name: "ncp_vlda_dataset".to_string(),
                kind: "dataset_json".to_string(),
                uri: path.display().to_string(),
                sha256,
                metadata: BTreeMap::from([(
                    "kept_samples".to_string(),
                    stats.kept_samples.to_string(),
                )]),
            })?;
            w.append(&RunLogEvent::RunEnded {
                run_id: self.run_id.clone(),
                timestamp_ns: ts,
                status: RunStatus::Succeeded,
                message: Some(format!(
                    "{} (V,L,D,A) samples from NCP [{}]",
                    dataset.samples.len(),
                    stats.summary()
                )),
            })?;
            w.flush()?;
        }
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

    #[test]
    fn joins_v_and_a_on_seq() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // Observation arrives first → sets D (unstamped: recency fallback).
        obs.on_observation(&observation(0, vec![5.0, 6.0]));

        // Sensor for seq=7 (V + L).
        obs.on_sensor(&sensor(
            7,
            1.0,
            &[("pose", vec![1.0, 2.0, 3.0]), ("instruction", vec![0.5])],
        ));
        obs.flush_complete();
        assert_eq!(obs.sample_count(), 0, "no command yet");

        // Command for seq=7 (A) → completes the sample (held for the grace
        // window until flushed).
        obs.on_command(&command(
            7,
            1.0,
            &[("velocity_setpoint", vec![0.1, 0.0, -0.1])],
        ));
        assert_eq!(obs.sample_count(), 0, "held for the reorder grace window");
        obs.flush_complete();
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
        obs.on_observation(&observation(7, vec![5.0, 6.0]));
        obs.on_observation(&observation(8, vec![9.0, 9.0]));
        // The seq=7 tick must pick the seq=7 D, not the most-recent (seq=8) one.
        obs.on_sensor(&sensor(
            7,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ));
        obs.on_command(&command(7, 0.0, &[("velocity_setpoint", vec![0.1])]));
        obs.flush_complete();
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
        obs.on_observation(&observation(0, vec![9.9])); // stale recency value
        obs.on_sensor(&sensor(
            7,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ));
        obs.on_command(&command(7, 0.0, &[("velocity_setpoint", vec![0.1])]));
        assert_eq!(obs.sample_count(), 0, "held for the grace window");
        // The tick's own readout arrives late but within the grace window …
        obs.on_observation(&observation(7, vec![5.5]));
        // … then the watermark advances far enough to emit seq 7.
        obs.on_sensor(&sensor(7 + REORDER_GRACE, 0.0, &[("pose", vec![1.0])]));
        assert_eq!(obs.sample_count(), 1, "emitted once past the grace window");
        assert_eq!(obs.sample(0).d, vec![5.5], "late D claimed its own tick");
        assert_eq!(
            obs.sample(0).metadata.get("d_source").map(String::as_str),
            Some("seq")
        );
    }

    #[test]
    fn observation_after_emission_patches_sample_as_seq_late() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation(0, vec![9.9])); // recency fallback source
        obs.on_sensor(&sensor(
            7,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ));
        obs.on_command(&command(7, 0.0, &[("velocity_setpoint", vec![0.1])]));
        obs.flush_complete(); // force emission before the readout arrives
        assert_eq!(obs.sample_count(), 1);
        assert_eq!(
            obs.sample(0).metadata.get("d_source").map(String::as_str),
            Some("recency_fallback")
        );
        // The seq-stamped readout arrives after emission with matching dims:
        // it must still patch the in-memory sample (the artifact is written at
        // finalize) and be counted.
        obs.on_observation(&observation(7, vec![5.5]));
        assert_eq!(obs.sample(0).d, vec![5.5]);
        assert_eq!(
            obs.sample(0).metadata.get("d_source").map(String::as_str),
            Some("seq_late")
        );
        assert_eq!(obs.stats.late_d_patched, 1);
    }

    #[test]
    fn provenance_marks_recency_fallback() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // An observation stamped seq=0 cannot be seq-aligned; it only updates
        // latest_d, so the paired tick must fall back to recency.
        obs.on_observation(&observation(0, vec![3.0]));
        obs.on_sensor(&sensor(
            4,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5, 0.5])],
        ));
        obs.on_command(&command(4, 0.0, &[("velocity_setpoint", vec![0.1])]));
        obs.flush_complete();
        assert_eq!(obs.sample_count(), 1);
        assert_eq!(obs.sample(0).d, vec![3.0], "D falls back to recency");
        assert_eq!(
            obs.sample(0).metadata.get("d_source").map(String::as_str),
            Some("recency_fallback")
        );
        assert_eq!(
            obs.sample(0).metadata.get("l_source").map(String::as_str),
            Some("channel")
        );
    }

    #[test]
    fn empty_axis_ticks_are_excluded_and_counted() {
        // A tick with no language channel yields an empty L. It must be
        // excluded from the artifact (one empty-axis sample would make
        // pid-offline-harness reject the WHOLE dataset) and counted.
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation(0, vec![3.0]));
        obs.on_sensor(&sensor(4, 0.0, &[("pose", vec![1.0])])); // no "instruction"
        obs.on_command(&command(4, 0.0, &[("velocity_setpoint", vec![0.1])]));
        obs.flush_complete();
        assert_eq!(obs.sample_count(), 0, "empty-L tick excluded");
        assert_eq!(obs.stats.excluded_empty_l, 1);
        // Same for a tick before any observation arrived (empty D).
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_sensor(&sensor(
            4,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ));
        obs.on_command(&command(4, 0.0, &[("velocity_setpoint", vec![0.1])]));
        obs.flush_complete();
        assert_eq!(obs.sample_count(), 0, "empty-D tick excluded");
        assert_eq!(obs.stats.excluded_empty_d, 1);
    }

    #[test]
    fn mismatched_seq_does_not_pair() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_sensor(&sensor(1, 0.0, &[("pose", vec![1.0])]));
        obs.on_command(&command(2, 0.0, &[("cmd", vec![0.0])]));
        obs.flush_complete();
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
        obs.on_observation(&observation(0, vec![3.0]));
        obs.on_sensor(&sensor(
            3,
            1.0,
            &[
                ("pose", vec![1.0, 2.0]),
                ("instruction", vec![0.5]),
                ("success", vec![1.0]),
            ],
        ));
        obs.on_command(&command(3, 1.0, &[("velocity_setpoint", vec![0.1])]));
        obs.flush_complete();
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
    fn inflight_maps_are_bounded() {
        // A long-running tap that sees many never-completing seqs must not grow
        // `pending`/`d_by_seq` without bound.
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        for seq in 1..(MAX_INFLIGHT as i64 + 500) {
            // Only a sensor (no matching command) → this seq never completes.
            obs.on_sensor(&sensor(seq, 0.0, &[("pose", vec![1.0])]));
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
        obs.on_observation(&observation(0, vec![3.0]));
        // Fill pending with stale, never-completing high seqs, past the cap.
        for seq in 100_000..(100_000 + MAX_INFLIGHT as i64 + 10) {
            obs.on_sensor(&sensor(seq, 0.0, &[("pose", vec![1.0])]));
        }
        // Session restarts at seq 1: the new tick must still produce a sample
        // (lowest-key eviction would evict it before its command arrived), and
        // it must NOT pair with any stale pre-reset state. The reset clears
        // latest_d, so give the new epoch its own readout.
        obs.on_sensor(&sensor(
            1,
            0.0,
            &[("pose", vec![7.0]), ("instruction", vec![0.5])],
        ));
        obs.on_observation(&observation(0, vec![4.0]));
        obs.on_command(&command(1, 0.0, &[("velocity_setpoint", vec![0.2])]));
        obs.flush_complete();
        assert_eq!(obs.sample_count(), 1, "post-reset tick must complete");
        assert_eq!(obs.stats.seq_resets, 1);
        let s = obs.sample(0);
        assert_eq!(s.sample_id, "ncp-1-1", "epoch-qualified id");
        assert_eq!(s.v, vec![7.0], "new-epoch V only");
        assert_eq!(s.d, vec![4.0], "new-epoch D only (pre-reset D was cleared)");
        assert_eq!(
            s.metadata.get("d_source").map(String::as_str),
            Some("recency_fallback"),
            "cross-epoch D must never claim exact alignment"
        );
    }

    #[test]
    fn adversarial_extreme_seq_does_not_panic() {
        // A hostile/garbage peer can send seq near i64::MAX/MIN; the reset-detection
        // and reorder arithmetic must saturate, never overflow (debug panic in the
        // Zenoh callback / release wrap that wedges the capture).
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_observation(&observation(i64::MAX, vec![1.0]));
        obs.on_sensor(&sensor(i64::MAX, 0.0, &[("pose", vec![1.0]), ("instruction", vec![0.5])]));
        obs.on_command(&command(i64::MAX, 0.0, &[("v", vec![0.1])]));
        obs.on_sensor(&sensor(i64::MIN + 1, 0.0, &[("pose", vec![2.0])]));
        obs.on_command(&command(i64::MIN + 1, 0.0, &[("v", vec![0.2])]));
        // No panic reaching here is the assertion; also flush cleanly.
        obs.flush_complete();
    }

    #[test]
    fn unstamped_sensor_and_command_frames_are_dropped() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        obs.on_sensor(&sensor(
            0,
            0.0,
            &[("pose", vec![1.0]), ("instruction", vec![0.5])],
        ));
        obs.on_command(&command(0, 0.0, &[("velocity_setpoint", vec![0.1])]));
        obs.flush_complete();
        assert_eq!(obs.sample_count(), 0, "seq-0 frames are unjoinable");
        assert_eq!(obs.stats.unstamped_frames_dropped, 2);
    }

    #[test]
    fn finalize_writes_valid_runlog_with_artifact_registration() {
        let dir = std::env::temp_dir().join(format!("ncp_observer_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let runlog = dir.join("runlog.jsonl");
        let dataset = dir.join("vlda.json");
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default())
            .with_runlog(&runlog)
            .unwrap();
        obs.on_observation(&observation(0, vec![3.0]));
        // Ticks with DESCENDING sensor times: the monotonic run-log clock must
        // clamp so validation still passes.
        for seq in [7i64, 8, 9] {
            obs.on_sensor(&sensor(
                seq,
                (10 - seq) as f64,
                &[("pose", vec![1.0]), ("instruction", vec![0.5])],
            ));
            obs.on_command(&command(seq, (10 - seq) as f64, &[("v", vec![0.1])]));
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
}
