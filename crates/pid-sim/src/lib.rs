use anyhow::{bail, Context, Result};
use pid_bridge::{
    BridgeHandler, BridgeMethod, BridgeRequest, BridgeResponse, BridgeRpcRequest,
    BridgeRpcResponse, LocalBridge,
};
use pid_runlog::{Actor, Pose, RunLogEvent, RunLogWriter, SimObjectSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};

pub mod offline_harness;
pub mod toy_harness;

pub const FLOW_PRED_SOURCE: &str = "constant_velocity_baseline";

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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SimReplayReport {
    pub seeded_from_step: Option<u64>,
    pub checked_actions: usize,
    pub checked_snapshots: usize,
    pub checked_objects: usize,
    pub final_logged_step: Option<u64>,
    pub final_replayed_step: Option<u64>,
    pub issues: Vec<String>,
}

impl SimReplayReport {
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

pub fn deterministic_sim_config(
    source: &str,
    transport: Option<&str>,
    fixed_dt_secs: Option<f64>,
    planned_steps: Option<u64>,
    safe_mode: Option<bool>,
) -> Value {
    json!({
        "source": source,
        "transport": transport,
        "sim": {
            "backend": "deterministic_object",
            "crate_version": env!("CARGO_PKG_VERSION"),
            "deterministic": true,
            "integrator": "constant_velocity_euler",
            "flow_gt": "pose_delta",
            "flow_pred": FLOW_PRED_SOURCE,
            "collision_geometry": "point_proxy",
            "solver": {
                "contact_solver": "none",
                "iterations": 0
            }
        },
        "run": {
            "fixed_dt_secs": fixed_dt_secs,
            "planned_steps": planned_steps,
            "safe_mode": safe_mode
        }
    })
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

    pub fn from_snapshot(step: u64, timestamp_ns: u64, objects: &[SimObjectSnapshot]) -> Self {
        let mut sim = Self {
            step,
            timestamp_ns,
            objects: BTreeMap::new(),
        };
        for object in objects {
            sim.upsert_object(SimObject {
                object_id: object.object_id.clone(),
                pose: object.pose.clone(),
                velocity: object.velocity,
            });
        }
        sim
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

    pub fn flow_pred_events(&self) -> Vec<RunLogEvent> {
        self.flow_gt
            .iter()
            .map(|record| RunLogEvent::FlowPred {
                step: self.step,
                timestamp_ns: self.timestamp_ns,
                source: FLOW_PRED_SOURCE.to_string(),
                object_id: record.object_id.clone(),
                horizon_steps: 1,
                flow: vec![record.displacement],
                metadata: [("baseline".to_string(), "true".to_string())]
                    .into_iter()
                    .collect(),
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
                    "flow_pred_records": step.flow_gt.len(),
                }))
            }
            BridgeMethod::SceneSetObject => {
                let object = serde_json::from_value::<SimObject>(request.payload.clone())?;
                self.sim.upsert_object(object);
                Ok(self.status_json())
            }
            BridgeMethod::LogReplay => {
                let run_log_uri = request
                    .payload
                    .get("run_log_uri")
                    .and_then(Value::as_str)
                    .context("log.replay requires string run_log_uri")?;
                let summary = pid_runlog::summarize_path(run_log_uri)?;
                Ok(json!({
                    "trace_hash": summary.trace_hash,
                    "events": summary.event_count,
                    "valid": summary.validation_errors == 0,
                    "validation_errors": summary.validation_errors,
                    "validation_warnings": summary.validation_warnings,
                    "config_hash": summary.config_hash,
                }))
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

    pub fn with_safe_mode(
        writer: RunLogWriter<W>,
        sim: DeterministicObjectSim,
        safe_mode: bool,
    ) -> Self {
        Self {
            bridge: LocalBridge::with_safe_mode(writer, safe_mode),
            handler: SimBridgeHandler::new(sim),
        }
    }

    pub fn safe_mode(&self) -> bool {
        self.bridge.safe_mode()
    }

    pub fn set_safe_mode(&mut self, safe_mode: bool) {
        self.bridge.set_safe_mode(safe_mode);
    }

    pub fn dispatch(&mut self, request: &BridgeRequest) -> Result<BridgeResponse> {
        self.bridge.record_request(request)?;
        if self.bridge.safe_mode() && !request.safe_mode_allowed() {
            let response = BridgeResponse::blocked_by_safe_mode(
                request,
                request.timestamp_ns.max(self.handler.sim.timestamp_ns()),
            );
            self.bridge.record_response(&response)?;
            return Ok(response);
        }
        if request.method == BridgeMethod::LogReplay {
            self.bridge.flush()?;
        }
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
                self.record_action(request)?;
                self.bridge
                    .record_event(&self.handler.sim.snapshot_event())?;
                for event in self.handler.sim.pose_events() {
                    self.bridge.record_event(&event)?;
                }
                if let Some(step) = &self.handler.last_step {
                    for event in step.flow_events() {
                        self.bridge.record_event(&event)?;
                    }
                    for event in step.flow_pred_events() {
                        self.bridge.record_event(&event)?;
                    }
                }
            }
            BridgeMethod::SceneSetObject if response.ok => {
                self.record_action(request)?;
                self.bridge
                    .record_event(&self.handler.sim.snapshot_event())?;
                for event in self.handler.sim.pose_events() {
                    self.bridge.record_event(&event)?;
                }
            }
            BridgeMethod::SimReset if response.ok => {
                self.record_action(request)?;
                self.bridge
                    .record_event(&self.handler.sim.snapshot_event())?;
            }
            _ => {}
        }

        Ok(response)
    }

    pub fn step(&self) -> u64 {
        self.handler.sim.step()
    }

    pub fn timestamp_ns(&self) -> u64 {
        self.handler.sim.timestamp_ns()
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

    fn record_action(&mut self, request: &BridgeRequest) -> Result<()> {
        self.bridge.record_event(&RunLogEvent::ActionApplied {
            step: self.handler.sim.step(),
            timestamp_ns: self.handler.sim.timestamp_ns(),
            actor: request.actor.clone(),
            action_type: request.method.as_str().to_string(),
            payload_hash: request.payload_hash()?,
            payload: request.payload.clone(),
        })
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

pub fn dispatch_rpc_lines<R, O, L>(
    input: R,
    output: &mut O,
    session: &mut SimBridgeSession<L>,
    actor: Actor,
) -> Result<usize>
where
    R: BufRead,
    O: Write,
    L: Write,
{
    let mut handled = 0usize;
    for (idx, line) in input.lines().enumerate() {
        let line = line.with_context(|| format!("failed to read JSON-RPC line {}", idx + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let response = dispatch_rpc_text_request_with_context(
            &line,
            &format!("line-{}", idx + 1),
            "line",
            idx + 1,
            session,
            actor.clone(),
        );
        serde_json::to_writer(&mut *output, &response).context("failed to write RPC response")?;
        output
            .write_all(b"\n")
            .context("failed to write RPC response newline")?;
        handled += 1;
    }
    Ok(handled)
}

pub fn dispatch_rpc_text_request<L>(
    text: &str,
    request_index: usize,
    session: &mut SimBridgeSession<L>,
    actor: Actor,
) -> BridgeRpcResponse
where
    L: Write,
{
    dispatch_rpc_text_request_with_context(
        text,
        &format!("message-{request_index}"),
        "message",
        request_index,
        session,
        actor,
    )
}

fn dispatch_rpc_text_request_with_context<L>(
    text: &str,
    failure_id: &str,
    context_name: &str,
    request_index: usize,
    session: &mut SimBridgeSession<L>,
    actor: Actor,
) -> BridgeRpcResponse
where
    L: Write,
{
    match serde_json::from_str::<BridgeRpcRequest>(text) {
        Ok(rpc) => {
            let id = rpc.id.clone();
            match rpc.into_bridge_request(actor, Some(session.step()), session.timestamp_ns()) {
                Ok(request) => BridgeRpcResponse::from_bridge_response(
                    &session
                        .dispatch(&request)
                        .unwrap_or_else(|err| BridgeResponse {
                            request_id: id,
                            step: Some(session.step()),
                            timestamp_ns: session.timestamp_ns(),
                            ok: false,
                            message: Some(err.to_string()),
                            result: None,
                        }),
                ),
                Err(err) => BridgeRpcResponse::failure(id, -32601, err.to_string()),
            }
        }
        Err(err) => BridgeRpcResponse::failure(
            failure_id.to_string(),
            -32700,
            format!("invalid JSON-RPC request at {context_name} {request_index}: {err}"),
        ),
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
            report.issues.push(format!(
                "missing previous snapshot for {object_id} before step {step}"
            ));
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

pub fn verify_sim_replay(events: &[RunLogEvent], tolerance: f64) -> SimReplayReport {
    let mut report = SimReplayReport::default();
    if !tolerance.is_finite() || tolerance < 0.0 {
        report
            .issues
            .push("tolerance must be nonnegative and finite".to_string());
        return report;
    }

    let mut sim: Option<DeterministicObjectSim> = None;
    for event in events {
        match event {
            RunLogEvent::SimSnapshot {
                step,
                timestamp_ns,
                objects,
                ..
            } => {
                report.final_logged_step = Some(*step);
                if let Some(current) = &sim {
                    compare_snapshot(
                        current,
                        *step,
                        *timestamp_ns,
                        objects,
                        tolerance,
                        &mut report,
                    );
                } else {
                    report.seeded_from_step = Some(*step);
                    sim = Some(DeterministicObjectSim::from_snapshot(
                        *step,
                        *timestamp_ns,
                        objects,
                    ));
                }
            }
            RunLogEvent::ActionApplied {
                action_type,
                payload,
                ..
            } => apply_replay_action(&mut sim, action_type, payload, &mut report),
            _ => {}
        }
    }

    if let Some(current) = sim {
        report.final_replayed_step = Some(current.step());
        if report.final_logged_step != Some(current.step()) {
            report.issues.push(format!(
                "final replayed step {} does not match logged step {:?}",
                current.step(),
                report.final_logged_step
            ));
        }
    } else {
        report
            .issues
            .push("missing sim_snapshot seed for deterministic replay".to_string());
    }
    report
}

fn apply_replay_action(
    sim: &mut Option<DeterministicObjectSim>,
    action_type: &str,
    payload: &Value,
    report: &mut SimReplayReport,
) {
    match action_type {
        "sim.step" | "sim_step" => {
            let Some(current) = sim.as_mut() else {
                report
                    .issues
                    .push("sim.step action appeared before any sim_snapshot seed".to_string());
                return;
            };
            let dt = payload.get("dt").and_then(Value::as_f64).unwrap_or(0.1);
            match current.step_fixed(dt) {
                Ok(_) => report.checked_actions += 1,
                Err(err) => report
                    .issues
                    .push(format!("failed to replay sim.step: {err}")),
            }
        }
        "sim.reset" | "sim_reset" => {
            let Some(current) = sim.as_mut() else {
                report
                    .issues
                    .push("sim.reset action appeared before any sim_snapshot seed".to_string());
                return;
            };
            current.reset();
            report.checked_actions += 1;
        }
        "scene.set_object" | "scene_set_object" => {
            match serde_json::from_value::<SimObject>(payload.clone()) {
                Ok(object) => {
                    sim.get_or_insert_with(DeterministicObjectSim::new)
                        .upsert_object(object);
                    report.checked_actions += 1;
                }
                Err(err) => report
                    .issues
                    .push(format!("failed to replay scene.set_object: {err}")),
            }
        }
        _ => {}
    }
}

fn compare_snapshot(
    sim: &DeterministicObjectSim,
    step: u64,
    timestamp_ns: u64,
    objects: &[SimObjectSnapshot],
    tolerance: f64,
    report: &mut SimReplayReport,
) {
    report.checked_snapshots += 1;
    if sim.step() != step {
        report.issues.push(format!(
            "snapshot step {step} does not match replayed step {}",
            sim.step()
        ));
    }
    if sim.timestamp_ns() != timestamp_ns {
        report.issues.push(format!(
            "snapshot timestamp {timestamp_ns} does not match replayed timestamp {}",
            sim.timestamp_ns()
        ));
    }
    if sim.objects.len() != objects.len() {
        report.issues.push(format!(
            "snapshot object count {} does not match replayed count {} at step {step}",
            objects.len(),
            sim.objects.len()
        ));
    }

    let mut logged = BTreeMap::new();
    for object in objects {
        if logged.insert(object.object_id.as_str(), object).is_some() {
            report.issues.push(format!(
                "duplicate snapshot object {} at step {step}",
                object.object_id
            ));
        }
    }
    for expected in sim.objects.values() {
        let Some(actual) = logged.get(expected.object_id.as_str()) else {
            report.issues.push(format!(
                "missing snapshot object {} at step {step}",
                expected.object_id
            ));
            continue;
        };
        report.checked_objects += 1;
        compare_slice(
            &expected.pose.position,
            &actual.pose.position,
            tolerance,
            &format!("position for {} at step {step}", expected.object_id),
            report,
        );
        compare_slice(
            &expected.pose.orientation_xyzw,
            &actual.pose.orientation_xyzw,
            tolerance,
            &format!("orientation for {} at step {step}", expected.object_id),
            report,
        );
        compare_slice(
            &expected.velocity,
            &actual.velocity,
            tolerance,
            &format!("velocity for {} at step {step}", expected.object_id),
            report,
        );
    }
    for actual in objects {
        if !sim.objects.contains_key(&actual.object_id) {
            report.issues.push(format!(
                "unexpected snapshot object {} at step {step}",
                actual.object_id
            ));
        }
    }
}

fn compare_slice(
    expected: &[f64],
    actual: &[f64],
    tolerance: f64,
    label: &str,
    report: &mut SimReplayReport,
) {
    if expected.len() != actual.len()
        || expected
            .iter()
            .zip(actual)
            .any(|(left, right)| (left - right).abs() > tolerance)
    {
        report.issues.push(format!("snapshot mismatch in {label}"));
    }
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
    use pid_bridge::{BridgeMethod, BridgeRpcResponse};
    use pid_runlog::{
        canonical_json_hash, read_events, replay_events, ActorType, RunLogEvent, RunLogWriter,
        RunStatus, RUN_LOG_SCHEMA_VERSION,
    };
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
    fn deterministic_sim_config_records_backend_solver() {
        let config =
            deterministic_sim_config("test", Some("stdio_jsonl"), Some(0.1), Some(5), Some(true));
        assert_eq!(config["sim"]["backend"], "deterministic_object");
        assert_eq!(config["sim"]["solver"]["contact_solver"], "none");
        assert_eq!(config["run"]["fixed_dt_secs"], 0.1);
        assert_eq!(config["run"]["safe_mode"], true);
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
        events.extend(step.flow_pred_events());
        let state = replay_events(&events);
        assert_eq!(state.sim_snapshots, 1);
        assert_eq!(state.object_poses.len(), 2);
        assert_eq!(state.flow_gt_records, 2);
        assert_eq!(state.flow_pred_records, 2);
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
        assert_eq!(state.flow_pred_records, 6);
    }

    #[test]
    fn bridge_session_safe_mode_blocks_sim_step() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-safe-mode-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::with_safe_mode(writer, demo_sim(), true);
        let request = bridge_request(
            "req-safe-step",
            BridgeMethod::SimStep,
            actor,
            Some(0),
            0,
            json!({ "dt": 0.1 }),
        );
        let response = session.dispatch(&request).unwrap();
        assert!(!response.ok);
        assert_eq!(session.step(), 0);
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events);
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.actions.len(), 0);
        assert_eq!(state.sim_snapshots, 0);
    }

    #[test]
    fn bridge_session_safe_mode_allows_log_replay() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("pid-sim-log-replay-{stamp}.jsonl"));
        let config = json!({ "test": "log_replay" });
        let config_hash = canonical_json_hash(&config).unwrap();
        let mut replay_writer = RunLogWriter::create(&path).unwrap();
        replay_writer
            .append(&RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "replay-source".to_string(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: BTreeMap::new(),
            })
            .unwrap();
        replay_writer
            .append(&RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            })
            .unwrap();
        replay_writer
            .append(&RunLogEvent::RunEnded {
                run_id: "replay-source".to_string(),
                timestamp_ns: 1,
                status: RunStatus::Succeeded,
                message: None,
            })
            .unwrap();
        replay_writer.flush().unwrap();

        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-safe-replay-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::with_safe_mode(writer, demo_sim(), true);
        let request = bridge_request(
            "req-log-replay",
            BridgeMethod::LogReplay,
            actor,
            Some(0),
            0,
            json!({ "run_log_uri": path.display().to_string() }),
        );
        let response = session.dispatch(&request).unwrap();
        assert!(response.ok, "{:?}", response.message);
        let result = response.result.unwrap();
        assert_eq!(result["events"], 3);
        assert_eq!(result["valid"], true);
        assert_eq!(result["validation_errors"], 0);
        assert_eq!(result["trace_hash"].as_str().unwrap().len(), 64);

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events);
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.actions.len(), 0);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bridge_session_logs_scene_and_reset_as_actions() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-scene-test".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::new(writer, sim);
        let set_object = bridge_request(
            "req-set-object",
            BridgeMethod::SceneSetObject,
            actor.clone(),
            Some(0),
            0,
            json!({
                "object_id": "green_cube",
                "pose": {
                    "position": [0.4, 0.0, 0.025],
                    "orientation_xyzw": [0.0, 0.0, 0.0, 1.0]
                },
                "velocity": [0.0, 0.0, 0.0]
            }),
        );
        assert!(session.dispatch(&set_object).unwrap().ok);
        let reset = bridge_request(
            "req-reset",
            BridgeMethod::SimReset,
            actor,
            Some(0),
            0,
            json!({}),
        );
        assert!(session.dispatch(&reset).unwrap().ok);

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events);
        assert_eq!(state.actions.len(), 2);
        assert!(state
            .actions
            .iter()
            .any(|action| action.action_type == "scene.set_object"));
        assert!(state
            .actions
            .iter()
            .any(|action| action.action_type == "sim.reset"));
        let report = verify_sim_replay(&events, 1e-12);
        assert!(report.is_valid(), "{:?}", report.issues);
    }

    #[test]
    fn rpc_line_processor_dispatches_requests_and_logs_replayable_events() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-rpc-test".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::new(writer, sim);
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":"status","method":"sim.status","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":"step","method":"sim.step","params":{"dt":0.1}}"#,
            "\n"
        );
        let mut output = Vec::new();
        let handled =
            dispatch_rpc_lines(Cursor::new(input), &mut output, &mut session, actor).unwrap();
        assert_eq!(handled, 2);

        let lines = String::from_utf8(output).unwrap();
        let responses = lines
            .lines()
            .map(|line| serde_json::from_str::<BridgeRpcResponse>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responses.len(), 2);
        assert!(responses.iter().all(BridgeRpcResponse::is_ok));
        assert_eq!(responses[0].id, "status");
        assert_eq!(responses[1].id, "step");

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let report = verify_sim_replay(&events, 1e-12);
        assert!(report.is_valid(), "{:?}", report.issues);
        assert_eq!(report.checked_actions, 1);
        assert_eq!(report.checked_snapshots, 1);
    }

    #[test]
    fn rpc_line_processor_safe_mode_blocks_mutation() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-rpc-safe-test".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::with_safe_mode(writer, sim, true);
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":"status","method":"sim.status","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":"step","method":"sim.step","params":{"dt":0.1}}"#,
            "\n"
        );
        let mut output = Vec::new();
        let handled =
            dispatch_rpc_lines(Cursor::new(input), &mut output, &mut session, actor).unwrap();
        assert_eq!(handled, 2);

        let lines = String::from_utf8(output).unwrap();
        let responses = lines
            .lines()
            .map(|line| serde_json::from_str::<BridgeRpcResponse>(line).unwrap())
            .collect::<Vec<_>>();
        assert!(responses[0].is_ok());
        assert!(!responses[1].is_ok());
        assert!(responses[1]
            .error
            .as_ref()
            .unwrap()
            .message
            .contains("safe mode blocked"));

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events);
        assert_eq!(state.actions.len(), 0);
        assert_eq!(state.sim_snapshots, 1);
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

    #[test]
    fn sim_replay_verifier_checks_logged_actions_against_snapshots() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-replay-test".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::new(writer, sim);
        for idx in 0..3 {
            let request = bridge_request(
                format!("req-replay-{idx}"),
                BridgeMethod::SimStep,
                actor.clone(),
                Some(idx),
                idx * 100_000_000,
                json!({ "dt": 0.1 }),
            );
            assert!(session.dispatch(&request).unwrap().ok);
        }
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let report = verify_sim_replay(&events, 1e-12);
        assert!(report.is_valid(), "{:?}", report.issues);
        assert_eq!(report.seeded_from_step, Some(0));
        assert_eq!(report.checked_actions, 3);
        assert_eq!(report.checked_snapshots, 3);
        assert_eq!(report.final_logged_step, Some(3));
        assert_eq!(report.final_replayed_step, Some(3));
    }

    #[test]
    fn sim_replay_verifier_reports_snapshot_mismatch() {
        let mut sim = demo_sim();
        let mut events = vec![sim.snapshot_event()];
        let payload = json!({ "dt": 0.1 });
        events.push(RunLogEvent::ActionApplied {
            step: 1,
            timestamp_ns: 100_000_000,
            actor: Actor {
                actor_type: ActorType::Script,
                actor_id: "sim-replay-test".to_string(),
                session_id: None,
            },
            action_type: "sim.step".to_string(),
            payload_hash: pid_runlog::canonical_json_hash(&payload).unwrap(),
            payload,
        });
        sim.step_fixed(0.2).unwrap();
        events.push(sim.snapshot_event());

        let report = verify_sim_replay(&events, 1e-12);
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("snapshot timestamp")));
    }
}
