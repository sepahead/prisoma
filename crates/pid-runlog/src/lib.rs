use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

pub const RUN_LOG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    HumanGui,
    Script,
    LlmTool,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub actor_type: ActorType,
    pub actor_id: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Succeeded,
    Failed,
    Aborted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub position: [f64; 3],
    pub orientation_xyzw: [f64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunLogEvent {
    RunStarted {
        schema_version: u32,
        run_id: String,
        timestamp_ns: u64,
        config_hash: String,
        metadata: BTreeMap<String, String>,
    },
    RunEnded {
        run_id: String,
        timestamp_ns: u64,
        status: RunStatus,
        message: Option<String>,
    },
    ConfigLogged {
        timestamp_ns: u64,
        config_hash: String,
        config: serde_json::Value,
    },
    FrameObserved {
        step: u64,
        timestamp_ns: u64,
        observation_hash: Option<String>,
        metadata: BTreeMap<String, String>,
    },
    ActionApplied {
        step: u64,
        timestamp_ns: u64,
        actor: Actor,
        action_type: String,
        payload_hash: String,
        payload: serde_json::Value,
    },
    ObjectPose {
        step: u64,
        timestamp_ns: u64,
        object_id: String,
        pose: Pose,
    },
    FlowGt {
        step: u64,
        timestamp_ns: u64,
        object_id: String,
        flow: Vec<[f64; 3]>,
    },
    PidMetric {
        step: u64,
        timestamp_ns: u64,
        name: String,
        value: f64,
        metadata: BTreeMap<String, String>,
    },
    GeometryMetric {
        step: u64,
        timestamp_ns: u64,
        name: String,
        value: f64,
        metadata: BTreeMap<String, String>,
    },
    InterventionApplied {
        step: u64,
        timestamp_ns: u64,
        actor: Actor,
        intervention_type: String,
        payload_hash: String,
        payload: serde_json::Value,
    },
    ArtifactLogged {
        timestamp_ns: u64,
        name: String,
        kind: String,
        uri: String,
        sha256: Option<String>,
        metadata: BTreeMap<String, String>,
    },
    ErrorLogged {
        step: Option<u64>,
        timestamp_ns: u64,
        message: String,
        recoverable: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoseRecord {
    pub step: u64,
    pub timestamp_ns: u64,
    pub pose: Pose,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricRecord {
    pub step: u64,
    pub timestamp_ns: u64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub step: u64,
    pub timestamp_ns: u64,
    pub actor: Actor,
    pub action_type: String,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionRecord {
    pub step: u64,
    pub timestamp_ns: u64,
    pub actor: Actor,
    pub intervention_type: String,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub timestamp_ns: u64,
    pub name: String,
    pub kind: String,
    pub uri: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReplayState {
    pub schema_version: Option<u32>,
    pub run_id: Option<String>,
    pub config_hash: Option<String>,
    pub status: Option<RunStatus>,
    pub last_step: Option<u64>,
    pub last_timestamp_ns: Option<u64>,
    pub events_seen: usize,
    pub object_poses: BTreeMap<String, PoseRecord>,
    pub pid_metrics: BTreeMap<String, MetricRecord>,
    pub geometry_metrics: BTreeMap<String, MetricRecord>,
    pub actions: Vec<ActionRecord>,
    pub interventions: Vec<InterventionRecord>,
    pub artifacts: Vec<ArtifactRecord>,
    pub errors: Vec<String>,
    pub flow_gt_records: usize,
}

impl ReplayState {
    pub fn apply(&mut self, event: &RunLogEvent) {
        self.events_seen += 1;
        self.last_timestamp_ns = Some(event.timestamp_ns());
        if let Some(step) = event.step() {
            self.last_step = Some(step);
        }

        match event {
            RunLogEvent::RunStarted {
                schema_version,
                run_id,
                config_hash,
                ..
            } => {
                self.schema_version = Some(*schema_version);
                self.run_id = Some(run_id.clone());
                self.config_hash = Some(config_hash.clone());
            }
            RunLogEvent::RunEnded { status, .. } => {
                self.status = Some(status.clone());
            }
            RunLogEvent::ConfigLogged { config_hash, .. } => {
                self.config_hash = Some(config_hash.clone());
            }
            RunLogEvent::FrameObserved { .. } => {}
            RunLogEvent::ActionApplied {
                step,
                timestamp_ns,
                actor,
                action_type,
                payload_hash,
                ..
            } => self.actions.push(ActionRecord {
                step: *step,
                timestamp_ns: *timestamp_ns,
                actor: actor.clone(),
                action_type: action_type.clone(),
                payload_hash: payload_hash.clone(),
            }),
            RunLogEvent::ObjectPose {
                step,
                timestamp_ns,
                object_id,
                pose,
            } => {
                self.object_poses.insert(
                    object_id.clone(),
                    PoseRecord {
                        step: *step,
                        timestamp_ns: *timestamp_ns,
                        pose: pose.clone(),
                    },
                );
            }
            RunLogEvent::FlowGt { .. } => {
                self.flow_gt_records += 1;
            }
            RunLogEvent::PidMetric {
                step,
                timestamp_ns,
                name,
                value,
                ..
            } => {
                self.pid_metrics.insert(
                    name.clone(),
                    MetricRecord {
                        step: *step,
                        timestamp_ns: *timestamp_ns,
                        value: *value,
                    },
                );
            }
            RunLogEvent::GeometryMetric {
                step,
                timestamp_ns,
                name,
                value,
                ..
            } => {
                self.geometry_metrics.insert(
                    name.clone(),
                    MetricRecord {
                        step: *step,
                        timestamp_ns: *timestamp_ns,
                        value: *value,
                    },
                );
            }
            RunLogEvent::InterventionApplied {
                step,
                timestamp_ns,
                actor,
                intervention_type,
                payload_hash,
                ..
            } => self.interventions.push(InterventionRecord {
                step: *step,
                timestamp_ns: *timestamp_ns,
                actor: actor.clone(),
                intervention_type: intervention_type.clone(),
                payload_hash: payload_hash.clone(),
            }),
            RunLogEvent::ArtifactLogged {
                timestamp_ns,
                name,
                kind,
                uri,
                sha256,
                ..
            } => self.artifacts.push(ArtifactRecord {
                timestamp_ns: *timestamp_ns,
                name: name.clone(),
                kind: kind.clone(),
                uri: uri.clone(),
                sha256: sha256.clone(),
            }),
            RunLogEvent::ErrorLogged { message, .. } => self.errors.push(message.clone()),
        }
    }
}

impl RunLogEvent {
    pub fn timestamp_ns(&self) -> u64 {
        match self {
            RunLogEvent::RunStarted { timestamp_ns, .. }
            | RunLogEvent::RunEnded { timestamp_ns, .. }
            | RunLogEvent::ConfigLogged { timestamp_ns, .. }
            | RunLogEvent::FrameObserved { timestamp_ns, .. }
            | RunLogEvent::ActionApplied { timestamp_ns, .. }
            | RunLogEvent::ObjectPose { timestamp_ns, .. }
            | RunLogEvent::FlowGt { timestamp_ns, .. }
            | RunLogEvent::PidMetric { timestamp_ns, .. }
            | RunLogEvent::GeometryMetric { timestamp_ns, .. }
            | RunLogEvent::InterventionApplied { timestamp_ns, .. }
            | RunLogEvent::ArtifactLogged { timestamp_ns, .. }
            | RunLogEvent::ErrorLogged { timestamp_ns, .. } => *timestamp_ns,
        }
    }

    pub fn step(&self) -> Option<u64> {
        match self {
            RunLogEvent::FrameObserved { step, .. }
            | RunLogEvent::ActionApplied { step, .. }
            | RunLogEvent::ObjectPose { step, .. }
            | RunLogEvent::FlowGt { step, .. }
            | RunLogEvent::PidMetric { step, .. }
            | RunLogEvent::GeometryMetric { step, .. }
            | RunLogEvent::InterventionApplied { step, .. } => Some(*step),
            RunLogEvent::ErrorLogged { step, .. } => *step,
            RunLogEvent::RunStarted { .. }
            | RunLogEvent::RunEnded { .. }
            | RunLogEvent::ConfigLogged { .. }
            | RunLogEvent::ArtifactLogged { .. } => None,
        }
    }
}

pub struct RunLogWriter<W> {
    writer: W,
}

impl RunLogWriter<BufWriter<File>> {
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::create(path.as_ref())
            .with_context(|| format!("failed to create run log {}", path.as_ref().display()))?;
        Ok(Self::new(BufWriter::new(file)))
    }
}

impl<W: Write> RunLogWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn append(&mut self, event: &RunLogEvent) -> Result<()> {
        serde_json::to_writer(&mut self.writer, event)
            .context("failed to serialize run-log event")?;
        self.writer
            .write_all(b"\n")
            .context("failed to write run-log newline")?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().context("failed to flush run log")
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}

pub fn read_events_from_path(path: impl AsRef<Path>) -> Result<Vec<RunLogEvent>> {
    let file = File::open(path.as_ref())
        .with_context(|| format!("failed to open run log {}", path.as_ref().display()))?;
    read_events(BufReader::new(file))
}

pub fn read_events<R: BufRead>(reader: R) -> Result<Vec<RunLogEvent>> {
    let mut events = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("failed to read run-log line {}", idx + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let event = serde_json::from_str(&line)
            .with_context(|| format!("invalid run-log event at line {}", idx + 1))?;
        events.push(event);
    }
    Ok(events)
}

pub fn replay_events(events: &[RunLogEvent]) -> ReplayState {
    let mut state = ReplayState::default();
    for event in events {
        state.apply(event);
    }
    state
}

pub fn replay_trace_hash(events: &[RunLogEvent]) -> Result<String> {
    canonical_json_hash(&replay_events(events))
}

pub fn canonical_json_hash<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value).context("failed to serialize value for hashing")?;
    Ok(sha256_hex(&bytes))
}

pub fn sha256_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = File::open(path.as_ref())
        .with_context(|| format!("failed to open artifact {}", path.as_ref().display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file
            .read(&mut buf)
            .with_context(|| format!("failed to read artifact {}", path.as_ref().display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(to_hex(&hasher.finalize()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    to_hex(&hasher.finalize())
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    fn actor() -> Actor {
        Actor {
            actor_type: ActorType::Script,
            actor_id: "test".to_string(),
            session_id: Some("s1".to_string()),
        }
    }

    fn sample_events() -> Vec<RunLogEvent> {
        vec![
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "run-1".to_string(),
                timestamp_ns: 1,
                config_hash: "cfg".to_string(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::ActionApplied {
                step: 0,
                timestamp_ns: 2,
                actor: actor(),
                action_type: "sim.step".to_string(),
                payload_hash: "payload".to_string(),
                payload: json!({ "dt": 0.01 }),
            },
            RunLogEvent::ObjectPose {
                step: 0,
                timestamp_ns: 3,
                object_id: "cube".to_string(),
                pose: Pose {
                    position: [1.0, 2.0, 3.0],
                    orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                },
            },
            RunLogEvent::PidMetric {
                step: 0,
                timestamp_ns: 4,
                name: "redundancy".to_string(),
                value: 0.25,
                metadata: BTreeMap::new(),
            },
        ]
    }

    #[test]
    fn jsonl_round_trip_preserves_events() {
        let events = sample_events();
        let mut writer = RunLogWriter::new(Vec::new());
        for event in &events {
            writer.append(event).unwrap();
        }
        let bytes = writer.into_inner();
        let decoded = read_events(Cursor::new(bytes)).unwrap();
        assert_eq!(decoded, events);
    }

    #[test]
    fn replay_tracks_latest_state() {
        let events = sample_events();
        let state = replay_events(&events);
        assert_eq!(state.run_id.as_deref(), Some("run-1"));
        assert_eq!(state.last_step, Some(0));
        assert_eq!(state.actions.len(), 1);
        assert_eq!(state.object_poses["cube"].pose.position, [1.0, 2.0, 3.0]);
        assert_eq!(state.pid_metrics["redundancy"].value, 0.25);
    }

    #[test]
    fn replay_trace_hash_is_stable() {
        let events = sample_events();
        let h1 = replay_trace_hash(&events).unwrap();
        let h2 = replay_trace_hash(&events).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn malformed_json_reports_line_number() {
        let mut writer = RunLogWriter::new(Vec::new());
        writer.append(&sample_events()[0]).unwrap();
        let mut bytes = writer.into_inner();
        bytes.extend_from_slice(b"not-json\n");
        let err = read_events(Cursor::new(bytes)).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("line 2"));
    }
}
