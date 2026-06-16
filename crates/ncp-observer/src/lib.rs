//! # ncp-observer — pid_vla's passive NCP tap
//!
//! Engram (NEST, via the Neuro-Control Protocol) becomes another `(V,L,D,A)`
//! source for pid_vla's Partial Information Decomposition — exactly the role
//! `experiments/safe_adapter` plays for SAFE rollouts. This crate is a
//! **read-only observer**: it subscribes to the NCP data-plane keys
//! (`…/session/{id}/{sensor,command,observation}`) and converts each closed-loop
//! tick into an `OfflineVldaSample`, writing both
//!
//! 1. an `OfflineVldaDataset` JSON artifact (what `pid-offline-harness` runs the
//!    `V/L/D → A` PID screens on), and
//! 2. canonical run-log events (the source of truth): one `EmbeddingContract`
//!    declaring the `(V,L,D,A)` variables, an `EmbeddingCaptured` per sample, and
//!    a `LabelObserved` per success label.
//!
//! It honors pid_vla's three rules — the run log is the source of truth, the
//! observer never drives anything (the Agent Bridge stays the only control
//! plane), and the NCP-specific mapping lives here in pid_vla, not in Engram.
//!
//! ## Mapping (V, L, D, A)
//! - **V** (vision/sensory) ← `SensorFrame` channels (all but the language
//!   channel), flattened.
//! - **L** (language/instruction) ← a named `SensorFrame` channel (default
//!   `instruction`); zeros if absent.
//! - **D** (dynamics / internal world-model) ← `ObservationFrame` record-port
//!   readouts — the neural state *before* the motor head, the "internal
//!   simulation" the PID(V,D;A) probe targets.
//! - **A** (action) ← `CommandFrame` channels, flattened.
//!
//! ## Alignment (the correctness rule)
//! V and A are joined on **`seq`** — a `CommandFrame.seq` echoes the
//! `SensorFrame.seq` it was computed from, so a sample pairs the action with the
//! sensor that produced it (never by arrival time, which the DROP QoS on the
//! perception plane would corrupt). `ObservationFrame` carries no `seq` today, so
//! D is paired with the most recent observation seen for the session
//! (best-effort); precise D alignment is a noted protocol enhancement (stamp
//! observations with the driving `seq`).

use ncp_core::{ChannelValue, CommandFrame, ObservationFrame, SensorFrame};
use pid_runlog::{
    Actor, ActorType, EmbeddingVariableContract, RunLogEvent, RunLogWriter, RunStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufWriter;
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

fn flatten_except(channels: &BTreeMap<String, ChannelValue>, except: Option<&str>) -> Vec<f64> {
    let mut out = Vec::new();
    // BTreeMap iterates in sorted key order → deterministic concatenation.
    for (name, cv) in channels {
        if Some(name.as_str()) == except {
            continue;
        }
        out.extend_from_slice(&cv.data);
    }
    out
}

/// Accumulates NCP frames into `(V,L,D,A)` samples, joining V↔A on `seq`.
pub struct Observer {
    run_id: String,
    model: String,
    task: String,
    mapping: Mapping,
    /// seq → partial sample (sensor seen, awaiting its command, or vice-versa).
    pending: BTreeMap<i64, Partial>,
    /// Most recent observation readout (the D axis) for the session — the
    /// best-effort fallback when an observation carries no `seq`.
    latest_d: Vec<f64>,
    /// D readouts keyed by the observation's `seq`, for exact alignment when the
    /// publisher stamps observations with the driving sensor's `seq`.
    d_by_seq: BTreeMap<i64, Vec<f64>>,
    samples: Vec<OfflineVldaSample>,
    writer: Option<RunLogWriter<BufWriter<File>>>,
    contract_emitted: bool,
    n: u64,
}

#[derive(Default)]
struct Partial {
    v: Option<Vec<f64>>,
    l: Option<Vec<f64>>,
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
            latest_d: Vec::new(),
            d_by_seq: BTreeMap::new(),
            samples: Vec::new(),
            writer: None,
            contract_emitted: false,
            n: 0,
        }
    }

    /// Attach a run-log so provenance events are emitted alongside the dataset.
    pub fn with_runlog(mut self, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut w = RunLogWriter::create(path)?;
        w.append(&RunLogEvent::RunStarted {
            schema_version: 1,
            run_id: self.run_id.clone(),
            timestamp_ns: 0,
            config_hash: "ncp-observer".into(),
            metadata: BTreeMap::from([("source".into(), "ncp".into())]),
        })?;
        self.writer = Some(w);
        Ok(self)
    }

    /// Ingest a `SensorFrame` (perception plane). Supplies V and L for its `seq`.
    pub fn on_sensor(&mut self, sensor: &SensorFrame) {
        let l = sensor
            .channels
            .get(&self.mapping.language_channel)
            .map(|cv| cv.data.clone())
            .unwrap_or_default();
        let v = flatten_except(&sensor.channels, Some(&self.mapping.language_channel));
        let success = self
            .mapping
            .success_channel
            .as_ref()
            .and_then(|c| sensor.channels.get(c))
            .and_then(|cv| cv.data.first().copied())
            .map(|x| serde_json::json!(x != 0.0));
        let entry = self.pending.entry(sensor.seq).or_default();
        entry.v = Some(v);
        entry.l = Some(l);
        entry.t = sensor.t;
        if success.is_some() {
            entry.success = success;
        }
        self.try_complete(sensor.seq);
    }

    /// Ingest a `CommandFrame` (action plane). Supplies A for its `seq`.
    pub fn on_command(&mut self, command: &CommandFrame) {
        let a = flatten_except(&command.channels, None);
        let entry = self.pending.entry(command.seq).or_default();
        entry.a = Some(a);
        if entry.t == 0.0 {
            entry.t = command.t;
        }
        self.try_complete(command.seq);
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
        if !d.is_empty() {
            // Exact alignment when the publisher stamps the driving seq; the
            // most-recent value remains the fallback for seq-less observations.
            if obs.seq != 0 {
                self.d_by_seq.insert(obs.seq, d.clone());
            }
            self.latest_d = d;
        }
    }

    fn try_complete(&mut self, seq: i64) {
        let ready = self
            .pending
            .get(&seq)
            .map(|p| p.v.is_some() && p.a.is_some())
            .unwrap_or(false);
        if !ready {
            return;
        }
        let p = self.pending.remove(&seq).unwrap();
        // Exact D when the observation for this seq was seen; else most-recent.
        let d = self
            .d_by_seq
            .remove(&seq)
            .unwrap_or_else(|| self.latest_d.clone());
        let mut labels = BTreeMap::new();
        if let Some(s) = p.success {
            labels.insert("success".to_string(), s);
        }
        let mut metadata = BTreeMap::new();
        metadata.insert("seq".to_string(), seq.to_string());
        metadata.insert("source".to_string(), "ncp".to_string());
        let sample = OfflineVldaSample {
            sample_id: format!("ncp-{seq}"),
            episode_id: self.mapping.episode_id.clone(),
            v: p.v.unwrap_or_default(),
            l: p.l.unwrap_or_default(),
            d,
            a: p.a.unwrap_or_default(),
            labels: labels.clone(),
            metadata,
        };
        self.emit_runlog(&sample, p.t, &labels);
        self.samples.push(sample);
        self.n += 1;
    }

    fn emit_runlog(
        &mut self,
        sample: &OfflineVldaSample,
        t: f64,
        labels: &BTreeMap<String, serde_json::Value>,
    ) {
        let Some(w) = self.writer.as_mut() else {
            return;
        };
        let ts = (t * 1e9).max(0.0) as u64;
        if !self.contract_emitted {
            let var = |name: &str, dims: usize| EmbeddingVariableContract {
                variable: name.to_string(),
                source: format!("nest:{name}"),
                dims: vec![dims],
                artifact_uri: None,
                sha256: None,
            };
            let _ = w.append(&RunLogEvent::EmbeddingContract {
                timestamp_ns: ts,
                name: "vlda".into(),
                variables: vec![
                    var("v", sample.v.len()),
                    var("l", sample.l.len()),
                    var("d", sample.d.len()),
                    var("a", sample.a.len()),
                ],
                metadata: BTreeMap::new(),
            });
            self.contract_emitted = true;
        }
        let mut meta = BTreeMap::new();
        meta.insert("sample_id".to_string(), sample.sample_id.clone());
        let _ = w.append(&RunLogEvent::EmbeddingCaptured {
            step: self.n,
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
            let _ = w.append(&RunLogEvent::LabelObserved {
                step: self.n,
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

    /// Finish: write the `(V,L,D,A)` dataset artifact and close the run log.
    pub fn finalize(&mut self, dataset_path: impl AsRef<Path>) -> anyhow::Result<()> {
        let dataset = OfflineVldaDataset {
            run_id: self.run_id.clone(),
            source: "ncp".into(),
            model: self.model.clone(),
            task: self.task.clone(),
            samples: self.samples.clone(),
        };
        let file = File::create(dataset_path.as_ref())?;
        serde_json::to_writer_pretty(BufWriter::new(file), &dataset)?;
        if let Some(mut w) = self.writer.take() {
            w.append(&RunLogEvent::RunEnded {
                run_id: self.run_id.clone(),
                timestamp_ns: 0,
                status: RunStatus::Succeeded,
                message: Some(format!("{} (V,L,D,A) samples from NCP", self.samples.len())),
            })?;
            w.flush()?;
        }
        Ok(())
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

    #[test]
    fn joins_v_and_a_on_seq() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // Observation arrives first → sets D.
        let mut records = Map::new();
        records.insert(
            "rate".into(),
            ncp_core::Observation {
                values: vec![5.0, 6.0],
                ..Default::default()
            },
        );
        obs.on_observation(&ObservationFrame {
            records,
            ..Default::default()
        });

        // Sensor for seq=7 (V + L).
        let mut sc = Map::new();
        sc.insert("pose".into(), ch(vec![1.0, 2.0, 3.0]));
        sc.insert("instruction".into(), ch(vec![0.5]));
        obs.on_sensor(&SensorFrame {
            seq: 7,
            t: 1.0,
            channels: sc,
            ..Default::default()
        });
        assert_eq!(obs.sample_count(), 0, "no command yet");

        // Command for seq=7 (A) → completes the sample.
        let mut cc = Map::new();
        cc.insert("velocity_setpoint".into(), ch(vec![0.1, 0.0, -0.1]));
        obs.on_command(&CommandFrame {
            seq: 7,
            t: 1.0,
            channels: cc,
            ..Default::default()
        });
        assert_eq!(obs.sample_count(), 1);
        let s = &obs.samples[0];
        assert_eq!(s.v, vec![1.0, 2.0, 3.0]); // pose only (instruction excluded)
        assert_eq!(s.l, vec![0.5]);
        assert_eq!(s.d, vec![5.0, 6.0]); // from the observation
        assert_eq!(s.a, vec![0.1, 0.0, -0.1]);
        assert_eq!(s.sample_id, "ncp-7");
    }

    #[test]
    fn d_aligns_on_seq_not_recency() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        // Observation for seq=7 (D=[5,6]), then a later one for seq=8 (D=[9,9]).
        let mut r7 = Map::new();
        r7.insert(
            "rate".into(),
            ncp_core::Observation {
                values: vec![5.0, 6.0],
                ..Default::default()
            },
        );
        obs.on_observation(&ObservationFrame {
            seq: 7,
            records: r7,
            ..Default::default()
        });
        let mut r8 = Map::new();
        r8.insert(
            "rate".into(),
            ncp_core::Observation {
                values: vec![9.0, 9.0],
                ..Default::default()
            },
        );
        obs.on_observation(&ObservationFrame {
            seq: 8,
            records: r8,
            ..Default::default()
        });
        // The seq=7 tick must pick the seq=7 D, not the most-recent (seq=8) one.
        let mut sc = Map::new();
        sc.insert("pose".into(), ch(vec![1.0]));
        obs.on_sensor(&SensorFrame {
            seq: 7,
            channels: sc,
            ..Default::default()
        });
        let mut cc = Map::new();
        cc.insert("velocity_setpoint".into(), ch(vec![0.1]));
        obs.on_command(&CommandFrame {
            seq: 7,
            channels: cc,
            ..Default::default()
        });
        assert_eq!(obs.sample_count(), 1);
        assert_eq!(
            obs.samples[0].d,
            vec![5.0, 6.0],
            "D must align on seq 7, not recency"
        );
    }

    #[test]
    fn mismatched_seq_does_not_pair() {
        let mut obs = Observer::new("run", "nest", "reach", Mapping::default());
        let mut sc = Map::new();
        sc.insert("pose".into(), ch(vec![1.0]));
        obs.on_sensor(&SensorFrame {
            seq: 1,
            channels: sc,
            ..Default::default()
        });
        let mut cc = Map::new();
        cc.insert("cmd".into(), ch(vec![0.0]));
        obs.on_command(&CommandFrame {
            seq: 2,
            channels: cc,
            ..Default::default()
        });
        assert_eq!(
            obs.sample_count(),
            0,
            "seq 1 sensor must not pair with seq 2 command"
        );
    }
}
