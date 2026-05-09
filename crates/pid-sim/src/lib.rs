use anyhow::{bail, Result};
use pid_bridge::{BridgeHandler, BridgeMethod, BridgeRequest, BridgeResponse, LocalBridge};
use pid_runlog::{Actor, Pose, RunLogEvent, RunLogWriter, SimObjectSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Write;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimObject {
    pub object_id: String,
    pub pose: Pose,
    pub velocity: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowGtRecord {
    pub object_id: String,
    pub displacement: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimStepResult {
    pub step: u64,
    pub timestamp_ns: u64,
    pub flow_gt: Vec<FlowGtRecord>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowVerificationReport {
    pub checked_flows: usize,
    pub issues: Vec<String>,
}

impl FlowVerificationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeterministicObjectSim {
    step: u64,
    timestamp_ns: u64,
    objects: BTreeMap<String, SimObject>,
}

#[derive(Debug, Clone, Default)]
pub struct SimBridgeHandler {
    pub sim: DeterministicObjectSim,
    pub last_step: Option<SimStepResult>,
}

pub struct SimBridgeSession<W> {
    bridge: LocalBridge<W>,
    handler: SimBridgeHandler,
}

impl Default for DeterministicObjectSim {
    fn default() -> Self {
        Self::new()
    }
}

impl DeterministicObjectSim {
    pub fn new() -> Self {
        Self {
            step: 0,
            timestamp_ns: 0,
            objects: BTreeMap::new(),
        }
    }

    pub fn step(&self) -> u64 {
        self.step
    }

    pub fn timestamp_ns(&self) -> u64 {
        self.timestamp_ns
    }

    pub fn objects(&self) -> impl Iterator<Item = &SimObject> {
        self.objects.values()
    }

    pub fn upsert_object(&mut self, object: SimObject) {
        self.objects.insert(object.object_id.clone(), object);
    }

    pub fn reset(&mut self) {
        self.step = 0;
        self.timestamp_ns = 0;
        self.objects.clear();
    }

    pub fn step_fixed(&mut self, dt_secs: f64) -> Result<SimStepResult> {
        if !dt_secs.is_finite() || dt_secs <= 0.0 {
            bail!("dt_secs must be positive and finite");
        }

        let dt_ns = (dt_secs * 1_000_000_000.0).round() as u64;
        let mut flow_gt = Vec::with_capacity(self.objects.len());
        for object in self.objects.values_mut() {
            let displacement = [
                object.velocity[0] * dt_secs,
                object.velocity[1] * dt_secs,
                object.velocity[2] * dt_secs,
            ];
            object.pose.position[0] += displacement[0];
            object.pose.position[1] += displacement[1];
            object.pose.position[2] += displacement[2];
            flow_gt.push(FlowGtRecord {
                object_id: object.object_id.clone(),
                displacement,
            });
        }
        self.step += 1;
        self.timestamp_ns += dt_ns;
        Ok(SimStepResult {
            step: self.step,
            timestamp_ns: self.timestamp_ns,
            flow_gt,
        })
    }

    pub fn snapshot_event(&self) -> RunLogEvent {
        RunLogEvent::SimSnapshot {
            step: self.step,
            timestamp_ns: self.timestamp_ns,
            objects: self
                .objects
                .values()
                .map(|object| SimObjectSnapshot {
                    object_id: object.object_id.clone(),
                    pose: object.pose.clone(),
                    velocity: object.velocity,
                })
                .collect(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn pose_events(&self) -> Vec<RunLogEvent> {
        self.objects
            .values()
            .map(|object| RunLogEvent::ObjectPose {
                step: self.step,
                timestamp_ns: self.timestamp_ns,
                object_id: object.object_id.clone(),
                pose: object.pose.clone(),
            })
            .collect()
    }
}

impl SimStepResult {
    pub fn flow_events(&self) -> Vec<RunLogEvent> {
        self.flow_gt
            .iter()
            .map(|record| RunLogEvent::FlowGt {
                step: self.step,
                timestamp_ns: self.timestamp_ns,
                object_id: record.object_id.clone(),
                flow: vec![record.displacement],
            })
            .collect()
    }
}

impl BridgeHandler for SimBridgeHandler {
    fn handle(&mut self, request: &BridgeRequest) -> Result<Value> {
        self.last_step = None;
        match request.method {
            BridgeMethod::SimStatus => Ok(self.status_json()),
            BridgeMethod::SimReset => {
                self.sim.reset();
                Ok(self.status_json())
            }
            BridgeMethod::SimStep => {
                let dt = request
                    .payload
                    .get("dt")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.1);
                let step = self.sim.step_fixed(dt)?;
                self.last_step = Some(step.clone());
                Ok(json!({
                    "step": step.step,
                    "timestamp_ns": step.timestamp_ns,
                    "flow_gt_records": step.flow_gt.len(),
                }))
            }
            BridgeMethod::SceneSetObject => {
                let object = serde_json::from_value::<SimObject>(request.payload.clone())?;
                self.sim.upsert_object(object);
                Ok(self.status_json())
            }
            _ => bail!("unsupported sim bridge method: {}", request.method.as_str()),
        }
    }
}

impl SimBridgeHandler {
    pub fn new(sim: DeterministicObjectSim) -> Self {
        Self {
            sim,
            last_step: None,
        }
    }

    fn status_json(&self) -> Value {
        json!({
            "step": self.sim.step(),
            "timestamp_ns": self.sim.timestamp_ns(),
            "objects": self.sim.objects().count(),
        })
    }
}

impl<W: Write> SimBridgeSession<W> {
    pub fn new(writer: RunLogWriter<W>, sim: DeterministicObjectSim) -> Self {
        Self {
            bridge: LocalBridge::new(writer),
            handler: SimBridgeHandler::new(sim),
        }
    }

    pub fn dispatch(&mut self, request: &BridgeRequest) -> Result<BridgeResponse> {
        self.bridge.record_request(request)?;
        let handled = self.handler.handle(request);
        let response = match handled {
            Ok(result) => BridgeResponse {
                request_id: request.request_id.clone(),
                step: Some(self.handler.sim.step()),
                timestamp_ns: self.handler.sim.timestamp_ns(),
                ok: true,
                message: None,
                result: Some(result),
            },
            Err(err) => BridgeResponse {
                request_id: request.request_id.clone(),
                step: Some(self.handler.sim.step()),
                timestamp_ns: self.handler.sim.timestamp_ns(),
                ok: false,
                message: Some(err.to_string()),
                result: None,
            },
        };
        self.bridge.record_response(&response)?;

        match request.method {
            BridgeMethod::SimStep if response.ok => {
                let payload_hash = request.payload_hash()?;
                self.bridge.record_event(&RunLogEvent::ActionApplied {
                    step: self.handler.sim.step(),
                    timestamp_ns: self.handler.sim.timestamp_ns(),
                    actor: request.actor.clone(),
                    action_type: request.method.as_str().to_string(),
                    payload_hash,
                    payload: request.payload.clone(),
                })?;
                self.bridge
                    .record_event(&self.handler.sim.snapshot_event())?;
                for event in self.handler.sim.pose_events() {
                    self.bridge.record_event(&event)?;
                }
                if let Some(step) = &self.handler.last_step {
                    for event in step.flow_events() {
                        self.bridge.record_event(&event)?;
                    }
                }
            }
            BridgeMethod::SceneSetObject if response.ok => {
                self.bridge
                    .record_event(&self.handler.sim.snapshot_event())?;
            }
            _ => {}
        }

        Ok(response)
    }

    pub fn record_event(&mut self, event: &RunLogEvent) -> Result<()> {
        self.bridge.record_event(event)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.bridge.flush()
    }

    pub fn into_inner(self) -> W {
        self.bridge.into_inner()
    }
}

pub fn bridge_request(
    request_id: impl Into<String>,
    method: BridgeMethod,
    actor: Actor,
    step: Option<u64>,
    timestamp_ns: u64,
    payload: Value,
) -> BridgeRequest {
    BridgeRequest {
        request_id: request_id.into(),
        step,
        timestamp_ns,
        actor,
        method,
        payload,
    }
}

pub fn verify_flow_gt(events: &[RunLogEvent], tolerance: f64) -> FlowVerificationReport {
    let mut snapshots: BTreeMap<u64, BTreeMap<String, [f64; 3]>> = BTreeMap::new();
    for event in events {
        if let RunLogEvent::SimSnapshot { step, objects, .. } = event {
            snapshots.insert(
                *step,
                objects
                    .iter()
                    .map(|object| (object.object_id.clone(), object.pose.position))
                    .collect(),
            );
        }
    }

    let mut report = FlowVerificationReport::default();
    for event in events {
        let RunLogEvent::FlowGt {
            step,
            object_id,
            flow,
            ..
        } = event
        else {
            continue;
        };
        if *step == 0 {
            continue;
        }
        let Some(current) = snapshots.get(step).and_then(|s| s.get(object_id)) else {
            report.issues.push(format!(
                "missing current snapshot for {object_id} at step {step}"
            ));
            continue;
        };
        let Some(previous) = snapshots.get(&(*step - 1)).and_then(|s| s.get(object_id)) else {
            continue;
        };
        for vec in flow {
            let expected = [
                current[0] - previous[0],
                current[1] - previous[1],
                current[2] - previous[2],
            ];
            report.checked_flows += 1;
            if (vec[0] - expected[0]).abs() > tolerance
                || (vec[1] - expected[1]).abs() > tolerance
                || (vec[2] - expected[2]).abs() > tolerance
            {
                report
                    .issues
                    .push(format!("flow mismatch for {object_id} at step {step}"));
            }
        }
    }
    report
}

pub fn demo_sim() -> DeterministicObjectSim {
    let mut sim = DeterministicObjectSim::new();
    sim.upsert_object(SimObject {
        object_id: "red_cube".to_string(),
        pose: Pose {
            position: [0.0, 0.0, 0.025],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        velocity: [0.1, 0.0, 0.0],
    });
    sim.upsert_object(SimObject {
        object_id: "blue_cube".to_string(),
        pose: Pose {
            position: [0.2, 0.0, 0.025],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        velocity: [0.0, 0.05, 0.0],
    });
    sim
}

#[cfg(test)]
mod tests {
    use super::*;
    use pid_bridge::BridgeMethod;
    use pid_runlog::{read_events, replay_events, ActorType, RunLogWriter};
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn fixed_step_is_deterministic() {
        let mut a = demo_sim();
        let mut b = demo_sim();
        for _ in 0..3 {
            a.step_fixed(0.1).unwrap();
            b.step_fixed(0.1).unwrap();
        }
        assert_eq!(a, b);
        assert_eq!(a.step(), 3);
    }

    #[test]
    fn flow_gt_matches_velocity_times_dt() {
        let mut sim = demo_sim();
        let result = sim.step_fixed(0.5).unwrap();
        let red = result
            .flow_gt
            .iter()
            .find(|record| record.object_id == "red_cube")
            .unwrap();
        assert_eq!(red.displacement, [0.05, 0.0, 0.0]);
    }

    #[test]
    fn sim_events_replay_into_runlog_state() {
        let mut sim = demo_sim();
        let step = sim.step_fixed(0.1).unwrap();
        let mut events = vec![sim.snapshot_event()];
        events.extend(sim.pose_events());
        events.extend(step.flow_events());
        let state = replay_events(&events);
        assert_eq!(state.sim_snapshots, 1);
        assert_eq!(state.object_poses.len(), 2);
        assert_eq!(state.flow_gt_records, 2);
    }

    #[test]
    fn bridge_session_logs_sim_step_events() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::new(writer, demo_sim());
        for idx in 0..3 {
            let request = bridge_request(
                format!("req-{idx}"),
                BridgeMethod::SimStep,
                actor.clone(),
                Some(idx),
                idx * 100_000_000,
                json!({ "dt": 0.1 }),
            );
            assert!(session.dispatch(&request).unwrap().ok);
        }
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events);
        assert_eq!(state.bridge_records.len(), 6);
        assert_eq!(state.actions.len(), 3);
        assert_eq!(state.sim_snapshots, 3);
        assert_eq!(state.flow_gt_records, 6);
    }

    #[test]
    fn flow_verifier_checks_snapshot_deltas() {
        let mut sim = demo_sim();
        let mut events = vec![sim.snapshot_event()];
        for _ in 0..3 {
            let step = sim.step_fixed(0.1).unwrap();
            events.push(sim.snapshot_event());
            events.extend(step.flow_events());
        }
        let report = verify_flow_gt(&events, 1e-12);
        assert!(report.is_valid(), "{:?}", report.issues);
        assert_eq!(report.checked_flows, 6);
    }
}
