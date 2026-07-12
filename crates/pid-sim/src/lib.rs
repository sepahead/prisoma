// The offline harness builds one large provenance `json!` literal; the default
// macro recursion limit is too small for it.
#![recursion_limit = "256"]

use anyhow::{bail, Context, Result};
use pid_bridge::{
    rpc_id_to_unique_request_id, BridgeHandler, BridgeMethod, BridgeRequest, BridgeResponse,
    BridgeRpcRequest, BridgeRpcResponse, LocalBridge,
};
use pid_runlog::{
    canonical_json_hash, Actor, Pose, RunLogEvent, RunLogWriter, RunStatus, SimObjectSnapshot,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

pub mod manipulation;
pub mod offline_harness;
pub mod physics;
pub mod power;
pub mod toy_harness;

pub const FLOW_PRED_SOURCE: &str = "constant_velocity_baseline";
pub const DEFAULT_BRIDGE_RUN_ID: &str = "sim-bridge-run";

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SetVelocityIntervention {
    object_id: String,
    velocity: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TranslateObjectIntervention {
    object_id: String,
    delta: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SetPoseIntervention {
    object_id: String,
    pose: Pose,
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
    pub checked_interventions: usize,
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
    run_id: String,
    run_ended: bool,
    /// Where this session's own run log lives, when file-backed. `export.rerun`
    /// must never be allowed to write over it: the run log is the source of
    /// truth, and `pid_rerun::save_recording` truncates its target.
    run_log_path: Option<PathBuf>,
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

    pub fn apply_intervention(
        &mut self,
        intervention_type: &str,
        payload: &Value,
    ) -> Result<Value> {
        match intervention_type {
            "set_velocity" => {
                let parsed: SetVelocityIntervention = serde_json::from_value(payload.clone())?;
                validate_object_id(&parsed.object_id)?;
                validate_vec3_finite(parsed.velocity, "velocity")?;
                let object = self
                    .objects
                    .get_mut(&parsed.object_id)
                    .with_context(|| format!("unknown object_id {}", parsed.object_id))?;
                object.velocity = parsed.velocity;
                Ok(json!({
                    "object_id": parsed.object_id,
                    "velocity": parsed.velocity,
                }))
            }
            "translate_object" => {
                let parsed: TranslateObjectIntervention =
                    serde_json::from_value(payload.clone())?;
                validate_object_id(&parsed.object_id)?;
                validate_vec3_finite(parsed.delta, "delta")?;
                let object = self
                    .objects
                    .get_mut(&parsed.object_id)
                    .with_context(|| format!("unknown object_id {}", parsed.object_id))?;
                for (position, delta) in object.pose.position.iter_mut().zip(parsed.delta) {
                    *position += delta;
                }
                Ok(json!({
                    "object_id": parsed.object_id,
                    "delta": parsed.delta,
                    "position": object.pose.position,
                }))
            }
            "set_pose" => {
                let parsed: SetPoseIntervention = serde_json::from_value(payload.clone())?;
                validate_object_id(&parsed.object_id)?;
                validate_pose_finite(&parsed.pose)?;
                let object = self
                    .objects
                    .get_mut(&parsed.object_id)
                    .with_context(|| format!("unknown object_id {}", parsed.object_id))?;
                object.pose = parsed.pose.clone();
                Ok(json!({
                    "object_id": parsed.object_id,
                    "pose": parsed.pose,
                }))
            }
            other => bail!(
                "unsupported intervention_type: {other}; supported: set_velocity, translate_object, set_pose"
            ),
        }
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

    /// Baseline flow "predictions" for this step.
    ///
    /// **Read the semantics before citing numbers from this.** The
    /// constant-velocity baseline is emitted at the *same* step whose
    /// `flow_gt` it echoes, so in the no-intervention case its prediction
    /// error is trivially zero — which is *correct* for this deterministic
    /// kinematic sim (constant velocity is the exact model), but it is a
    /// baseline record for the run-log schema, **not** evidence about any
    /// predictor. A real predictor evaluation needs a forecast emitted before
    /// the ground truth it is scored against.
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

fn validate_object_id(object_id: &str) -> Result<()> {
    if object_id.is_empty() {
        bail!("object_id must not be empty");
    }
    Ok(())
}

fn validate_vec3_finite(value: [f64; 3], name: &str) -> Result<()> {
    if value.iter().any(|value| !value.is_finite()) {
        bail!("{name} must be finite");
    }
    Ok(())
}

fn validate_pose_finite(pose: &Pose) -> Result<()> {
    validate_vec3_finite(pose.position, "pose position")?;
    if pose.orientation_xyzw.iter().any(|value| !value.is_finite()) {
        bail!("pose orientation must be finite");
    }
    Ok(())
}

impl BridgeHandler for SimBridgeHandler {
    fn handle(&mut self, request: &BridgeRequest) -> Result<Value> {
        self.last_step = None;
        match request.method {
            BridgeMethod::SimStatus => Ok(self.status_json()),
            BridgeMethod::SimReset => {
                // A reset zeroes the sim's step/timestamp counters; recording
                // post-reset events into the SAME run log would regress both
                // counters and fail pid-runlog's nondecreasing validation (and
                // poison Flow_gt verification). One run log = one monotonic
                // timeline: reset is only valid before the first step.
                if self.sim.step() > 0 {
                    bail!(
                        "sim.reset after sim.step would regress run-log step/time; \
                         finish this run (log.stop) and start a new bridge run instead"
                    );
                }
                self.sim.reset();
                Ok(self.status_json())
            }
            BridgeMethod::SimStep => {
                let dt = match request.payload.get("dt") {
                    None | Some(Value::Null) => 0.1,
                    Some(value) => value
                        .as_f64()
                        .with_context(|| format!("sim.step dt must be a number, got {value}"))?,
                };
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
                // Same fail-closed input discipline as intervention.apply: an
                // accepted-but-invalid object (empty id, non-finite pose or
                // velocity) would be rejected only later, when the emitted
                // SimSnapshot fails run-log validation — poisoning the log
                // instead of the request that caused it.
                validate_object_id(&object.object_id)?;
                validate_pose_finite(&object.pose)?;
                validate_vec3_finite(object.velocity, "object velocity")?;
                self.sim.upsert_object(object);
                Ok(self.status_json())
            }
            BridgeMethod::InterventionApply => {
                let intervention_type = request
                    .payload
                    .get("intervention_type")
                    .and_then(Value::as_str)
                    .context("intervention.apply requires string intervention_type")?;
                if intervention_type.is_empty() {
                    bail!("intervention_type must not be empty");
                }
                let payload = request
                    .payload
                    .get("payload")
                    .context("intervention.apply requires payload object")?;
                if !payload.is_object() {
                    bail!("intervention.apply payload must be an object");
                }
                let details = self.sim.apply_intervention(intervention_type, payload)?;
                Ok(json!({
                    "accepted": true,
                    "intervention_type": intervention_type,
                    "step": self.sim.step(),
                    "timestamp_ns": self.sim.timestamp_ns(),
                    "objects": self.sim.objects().count(),
                    "details": details,
                }))
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
            BridgeMethod::ExportRerun => {
                let run_log_uri = request
                    .payload
                    .get("run_log_uri")
                    .and_then(Value::as_str)
                    .context("export.rerun requires string run_log_uri")?;
                let output_uri = request
                    .payload
                    .get("output_uri")
                    .and_then(Value::as_str)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| default_rerun_output_path(run_log_uri));
                export_runlog_to_rerun(run_log_uri, &output_uri)
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
        Self::with_run_id(writer, sim, DEFAULT_BRIDGE_RUN_ID)
    }

    pub fn with_run_id(
        writer: RunLogWriter<W>,
        sim: DeterministicObjectSim,
        run_id: impl Into<String>,
    ) -> Self {
        Self {
            bridge: LocalBridge::new(writer),
            handler: SimBridgeHandler::new(sim),
            run_id: run_id.into(),
            run_ended: false,
            run_log_path: None,
        }
    }

    pub fn with_safe_mode(
        writer: RunLogWriter<W>,
        sim: DeterministicObjectSim,
        safe_mode: bool,
    ) -> Self {
        Self::with_safe_mode_and_run_id(writer, sim, safe_mode, DEFAULT_BRIDGE_RUN_ID)
    }

    pub fn with_safe_mode_and_run_id(
        writer: RunLogWriter<W>,
        sim: DeterministicObjectSim,
        safe_mode: bool,
        run_id: impl Into<String>,
    ) -> Self {
        Self {
            bridge: LocalBridge::with_safe_mode(writer, safe_mode),
            handler: SimBridgeHandler::new(sim),
            run_id: run_id.into(),
            run_ended: false,
            run_log_path: None,
        }
    }

    /// Tell the session where its own run log lives so `export.rerun` can
    /// refuse to overwrite it. File-backed transports should always set this.
    pub fn set_run_log_path(&mut self, path: impl Into<PathBuf>) {
        self.run_log_path = Some(path.into());
    }

    pub fn safe_mode(&self) -> bool {
        self.bridge.safe_mode()
    }

    pub fn set_safe_mode(&mut self, safe_mode: bool) {
        self.bridge.set_safe_mode(safe_mode);
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn run_ended(&self) -> bool {
        self.run_ended
    }

    pub fn dispatch(&mut self, request: &BridgeRequest) -> Result<BridgeResponse> {
        if self.run_ended {
            bail!("bridge run {} already ended", self.run_id);
        }
        self.bridge.record_request(request)?;
        if self.bridge.safe_mode() && !request.safe_mode_allowed() {
            let response = BridgeResponse::blocked_by_safe_mode(
                request,
                request.timestamp_ns.max(self.handler.sim.timestamp_ns()),
            );
            self.bridge.record_response(&response)?;
            return Ok(response);
        }
        if matches!(
            request.method,
            BridgeMethod::LogReplay | BridgeMethod::ExportRerun
        ) {
            self.bridge.flush()?;
        }
        let handled = match request.method {
            BridgeMethod::LogStart => self.handle_log_start(request),
            BridgeMethod::LogStop => self.handle_log_stop(request),
            BridgeMethod::ExportRerun => self
                .validate_export_target(request)
                .and_then(|()| self.handler.handle(request)),
            _ => self.handler.handle(request),
        };
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
            BridgeMethod::InterventionApply if response.ok => {
                self.record_intervention(request)?;
                self.bridge
                    .record_event(&self.handler.sim.snapshot_event())?;
                for event in self.handler.sim.pose_events() {
                    self.bridge.record_event(&event)?;
                }
            }
            BridgeMethod::ExportRerun if response.ok => {
                if let Some(result) = &response.result {
                    if let Some(output_uri) = result.get("output_uri").and_then(Value::as_str) {
                        self.bridge.record_event(&RunLogEvent::ArtifactLogged {
                            timestamp_ns: response.timestamp_ns,
                            name: "rerun_recording".to_string(),
                            kind: "rerun_rrd".to_string(),
                            uri: output_uri.to_string(),
                            sha256: result
                                .get("sha256")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                            metadata: result
                                .get("trace_hash")
                                .and_then(Value::as_str)
                                .map(|trace_hash| {
                                    [("trace_hash".to_string(), trace_hash.to_string())]
                                        .into_iter()
                                        .collect()
                                })
                                .unwrap_or_default(),
                        })?;
                    }
                }
            }
            _ => {}
        }
        if request.method == BridgeMethod::LogStop && response.ok {
            self.finish_run(
                RunStatus::Succeeded,
                Some(format!("log.stop requested by {}", request.actor.actor_id)),
            )?;
        }

        Ok(response)
    }

    pub fn step(&self) -> u64 {
        self.handler.sim.step()
    }

    pub fn timestamp_ns(&self) -> u64 {
        self.handler.sim.timestamp_ns()
    }

    /// Refuse an `export.rerun` whose resolved output path is this session's
    /// own run log: `pid_rerun::save_recording` truncates its target, and a
    /// client (any process that can reach the TCP/WS port) must not be able to
    /// destroy the source-of-truth log of the run in progress.
    fn validate_export_target(&self, request: &BridgeRequest) -> Result<()> {
        let Some(own) = &self.run_log_path else {
            return Ok(());
        };
        let run_log_uri = request
            .payload
            .get("run_log_uri")
            .and_then(Value::as_str)
            .context("export.rerun requires string run_log_uri")?;
        let output_uri = request
            .payload
            .get("output_uri")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| default_rerun_output_path(run_log_uri));
        if paths_resolve_to_same_target(&output_uri, own) {
            bail!(
                "export.rerun output_uri {} resolves to this session's own run log; refusing",
                output_uri.display()
            );
        }
        Ok(())
    }

    /// Record a rejected RPC message (malformed JSON, invalid id, unknown
    /// method) in the provenance log — grandplan's control-plane rule is
    /// "append every request and response summary to the run log". The trace
    /// is an [`RunLogEvent::ErrorLogged`] event, **not** a `BridgeResponse`:
    /// a rejected message has no recordable `BridgeRequest` (its method may be
    /// unknown, its id structurally invalid, or its JSON unparseable), and an
    /// unpaired response makes the canonical run-log validation fail — the old
    /// encoding let one probing/malformed message poison the validity of the
    /// very log this hook exists to keep audit-complete. Best-effort:
    /// a run-log write failure is reported on stderr rather than masking the
    /// protocol error already being returned to the client.
    pub fn record_rejected_rpc(&mut self, request_id: &str, message: &str) {
        if self.run_ended {
            return;
        }
        let event = RunLogEvent::ErrorLogged {
            step: Some(self.handler.sim.step()),
            timestamp_ns: self.handler.sim.timestamp_ns(),
            message: format!("rejected rpc {request_id}: {message}"),
            recoverable: true,
        };
        if let Err(err) = self.bridge.record_event(&event) {
            eprintln!("[pid-sim-bridge] failed to record rejected request in run log: {err}");
        }
    }

    pub fn record_event(&mut self, event: &RunLogEvent) -> Result<()> {
        if let RunLogEvent::RunEnded { run_id, .. } = event {
            if run_id == &self.run_id {
                self.run_ended = true;
            }
        }
        self.bridge.record_event(event)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.bridge.flush()
    }

    pub fn finish_run(&mut self, status: RunStatus, message: Option<String>) -> Result<bool> {
        if self.run_ended {
            return Ok(false);
        }
        let event = RunLogEvent::RunEnded {
            run_id: self.run_id.clone(),
            timestamp_ns: self.handler.sim.timestamp_ns(),
            status,
            message,
        };
        self.record_event(&event)?;
        Ok(true)
    }

    pub fn into_inner(self) -> W {
        self.bridge.into_inner()
    }

    fn handle_log_start(&self, request: &BridgeRequest) -> Result<Value> {
        if let Some(requested_run_id) = request.payload.get("run_id").and_then(Value::as_str) {
            if requested_run_id.is_empty() {
                bail!("log.start run_id must not be empty");
            }
            if requested_run_id != self.run_id {
                bail!(
                    "log.start run_id {requested_run_id} does not match active run {}",
                    self.run_id
                );
            }
        }
        if let Some(metadata) = request.payload.get("metadata") {
            if !metadata.is_object() {
                bail!("log.start metadata must be an object");
            }
        }
        Ok(json!({
            "run_id": self.run_id,
            "active": true,
            "step": self.handler.sim.step(),
            "timestamp_ns": self.handler.sim.timestamp_ns(),
        }))
    }

    fn handle_log_stop(&self, request: &BridgeRequest) -> Result<Value> {
        if !request.payload.is_object() {
            bail!("log.stop payload must be an object");
        }
        Ok(json!({
            "run_id": self.run_id,
            "stopped": true,
            "step": self.handler.sim.step(),
            "timestamp_ns": self.handler.sim.timestamp_ns(),
        }))
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

    fn record_intervention(&mut self, request: &BridgeRequest) -> Result<()> {
        let intervention_type = request
            .payload
            .get("intervention_type")
            .and_then(Value::as_str)
            .context("intervention.apply requires string intervention_type")?;
        let payload = request
            .payload
            .get("payload")
            .cloned()
            .context("intervention.apply requires payload object")?;
        self.bridge.record_event(&RunLogEvent::InterventionApplied {
            step: self.handler.sim.step(),
            timestamp_ns: self.handler.sim.timestamp_ns(),
            actor: request.actor.clone(),
            intervention_type: intervention_type.to_string(),
            payload_hash: canonical_json_hash(&payload)?,
            payload,
        })
    }
}

pub fn export_runlog_to_rerun(
    run_log_uri: impl AsRef<Path>,
    output_uri: impl AsRef<Path>,
) -> Result<Value> {
    let run_log_uri = run_log_uri.as_ref();
    let output_uri = output_uri.as_ref();
    if paths_refer_to_same_file(run_log_uri, output_uri) {
        bail!("export.rerun output_uri must differ from run_log_uri");
    }
    // save_recording truncates its target; requiring the Rerun extension keeps
    // a mistyped (or malicious) output_uri from overwriting arbitrary files.
    if output_uri.extension().and_then(|ext| ext.to_str()) != Some("rrd") {
        bail!(
            "export.rerun output_uri must end in .rrd, got {}",
            output_uri.display()
        );
    }
    if let Some(parent) = output_uri.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    let events = pid_runlog::read_events_from_path(run_log_uri)?;
    let summary = pid_runlog::summarize_events(&events)?;
    if summary.validation_errors > 0 {
        bail!(
            "run log failed validation ({} error(s)); refusing export",
            summary.validation_errors
        );
    }
    let manifest = pid_runlog::manifest_for_events(run_log_uri, &events)?;
    let output_uri = output_uri.display().to_string();
    let rec = pid_rerun::init_recording("prisoma_bridge_export", false)?;
    pid_rerun::RunLogRerunLogger::new(&rec).log_events_with_manifest(&events, Some(&manifest))?;
    pid_rerun::save_recording(&rec, &output_uri)?;
    let sha256 = pid_runlog::sha256_file(&output_uri)?;
    Ok(json!({
        "output_uri": output_uri,
        "sha256": sha256,
        "trace_hash": summary.trace_hash,
        "events": summary.event_count,
        "valid": summary.validation_errors == 0,
        "validation_errors": summary.validation_errors,
        "validation_warnings": summary.validation_warnings,
        "config_hash": summary.config_hash,
    }))
}

fn default_rerun_output_path(run_log_uri: &str) -> PathBuf {
    let mut path = PathBuf::from(run_log_uri);
    path.set_extension("rrd");
    path
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// Like [`paths_refer_to_same_file`], but also correct when one side does not
/// exist yet (an export target): canonicalize the parent directory (which does
/// exist for the live run log) and reattach the file name, so `./x.jsonl` and
/// `outputs/../x.jsonl` compare equal even before the target is created.
fn paths_resolve_to_same_target(left: &Path, right: &Path) -> bool {
    if paths_refer_to_same_file(left, right) {
        return true;
    }
    fn resolve(path: &Path) -> PathBuf {
        let file_name = path
            .file_name()
            .map(|n| n.to_os_string())
            .unwrap_or_default();
        let parent = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => PathBuf::from("."),
        };
        parent.canonicalize().unwrap_or(parent).join(file_name)
    }
    resolve(left) == resolve(right)
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

/// Upper bound on one JSON-RPC line (bytes, including the newline). Matches the
/// WebSocket transport's frame cap; without it a client can grow memory without
/// limit by never sending a newline.
const MAX_RPC_LINE_BYTES: u64 = 1024 * 1024;

pub fn dispatch_rpc_lines<R, O, L>(
    mut input: R,
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
    let mut idx = 0usize;
    loop {
        idx += 1;
        let mut line = String::new();
        // UFCS so `take` borrows `input` (`Take<&mut R>`) instead of moving it.
        let read = std::io::Read::take(&mut input, MAX_RPC_LINE_BYTES)
            .read_line(&mut line)
            .with_context(|| format!("failed to read JSON-RPC line {idx}"))?;
        if read == 0 {
            break;
        }
        if read as u64 == MAX_RPC_LINE_BYTES && !line.ends_with('\n') {
            bail!("JSON-RPC line {idx} exceeds {MAX_RPC_LINE_BYTES} bytes");
        }
        if line.trim().is_empty() {
            continue;
        }
        let response = dispatch_rpc_text_request_with_context(
            &line,
            &format!("line-{idx}"),
            "line",
            idx,
            session,
            actor.clone(),
        );
        serde_json::to_writer(&mut *output, &response).context("failed to write RPC response")?;
        output
            .write_all(b"\n")
            .context("failed to write RPC response newline")?;
        // Flush per response: an interactive client (TCP without half-close,
        // a REPL over stdio) deadlocks waiting for a reply that is sitting in
        // this side's BufWriter.
        output.flush().context("failed to flush RPC response")?;
        handled += 1;
        if session.run_ended() {
            break;
        }
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
            let id = match rpc.validated_id() {
                Ok(id) => id.clone(),
                Err(err) => {
                    let message = format!(
                        "invalid JSON-RPC request at {context_name} {request_index}: {err}"
                    );
                    session.record_rejected_rpc(failure_id, &message);
                    // The id is structurally invalid, so it cannot be echoed;
                    // JSON-RPC 2.0 says respond with id null.
                    return BridgeRpcResponse::failure(Value::Null, -32600, message);
                }
            };
            // Unique-by-construction run-log id: clients may reuse ids and
            // `1` vs `"1"` collide under the bare rendering, and duplicate
            // request ids hard-fail canonical run-log validation.
            let request_id = rpc_id_to_unique_request_id(&id, request_index);
            match rpc.into_bridge_request(actor, Some(session.step()), session.timestamp_ns()) {
                Ok(mut request) => {
                    request.request_id = request_id.clone();
                    let response = session.dispatch(&request).unwrap_or_else(|err| {
                        // dispatch itself failed (e.g. a run-log write
                        // error) — the response below may not be in the
                        // log; record the failure best-effort so the
                        // audit trail sees it.
                        let message = err.to_string();
                        session.record_rejected_rpc(&request_id, &message);
                        BridgeResponse {
                            request_id: request_id.clone(),
                            step: Some(session.step()),
                            timestamp_ns: session.timestamp_ns(),
                            ok: false,
                            message: Some(message),
                            result: None,
                        }
                    });
                    BridgeRpcResponse::from_bridge_response_with_id(&response, id)
                }
                Err(err) => {
                    // Unknown/unsupported method: return -32601 AND leave a
                    // trace — method probing is exactly the traffic a control-
                    // plane audit log must capture.
                    let message = err.to_string();
                    session.record_rejected_rpc(&request_id, &message);
                    BridgeRpcResponse::failure(id, -32601, message)
                }
            }
        }
        Err(err) => {
            let message =
                format!("invalid JSON-RPC request at {context_name} {request_index}: {err}");
            session.record_rejected_rpc(failure_id, &message);
            // Unparseable JSON: the request id is unknowable; JSON-RPC 2.0
            // says respond with id null (the message keeps the line/frame
            // context for correlation).
            BridgeRpcResponse::failure(Value::Null, -32700, message)
        }
    }
}

pub fn verify_flow_gt(events: &[RunLogEvent], tolerance: f64) -> FlowVerificationReport {
    let mut report = FlowVerificationReport::default();
    if !tolerance.is_finite() || tolerance < 0.0 {
        // A NaN tolerance would pass every comparison (false PASS verdicts);
        // a negative one would fail every honest log.
        report
            .issues
            .push("tolerance must be nonnegative and finite".to_string());
        return report;
    }

    // Pair each FlowGt with the snapshots as they stood IN STREAM ORDER, not
    // via a global last-wins step index: interventions and scene edits emit a
    // second SimSnapshot at the same step, and the flow logged by the NEXT
    // sim.step integrates from that later snapshot. Keying a map by step would
    // make the post-intervention snapshot retroactively "become" the step's
    // state and flag honest flows as mismatches.
    type Positions = BTreeMap<String, [f64; 3]>;
    let mut previous: Option<(u64, Positions)> = None;
    let mut current: Option<(u64, Positions)> = None;
    for event in events {
        match event {
            RunLogEvent::SimSnapshot { step, objects, .. } => {
                let positions: Positions = objects
                    .iter()
                    .map(|object| (object.object_id.clone(), object.pose.position))
                    .collect();
                match &current {
                    // Same-step re-snapshot (intervention / scene edit): it
                    // replaces the current state; `previous` is untouched.
                    Some((current_step, _)) if *current_step == *step => {
                        current = Some((*step, positions));
                    }
                    Some(_) => {
                        previous = current.take();
                        current = Some((*step, positions));
                    }
                    None => current = Some((*step, positions)),
                }
            }
            RunLogEvent::FlowGt {
                step,
                object_id,
                flow,
                ..
            } => {
                if *step == 0 {
                    continue;
                }
                let Some((current_step, current_positions)) = &current else {
                    report.issues.push(format!(
                        "missing current snapshot for {object_id} at step {step}"
                    ));
                    continue;
                };
                if current_step != step {
                    report.issues.push(format!(
                        "flow for {object_id} at step {step} does not follow its snapshot \
                         (latest snapshot is step {current_step})"
                    ));
                    continue;
                }
                let Some(current_position) = current_positions.get(object_id) else {
                    report.issues.push(format!(
                        "missing current snapshot for {object_id} at step {step}"
                    ));
                    continue;
                };
                let previous_position = previous
                    .as_ref()
                    .filter(|(previous_step, _)| *previous_step + 1 == *step)
                    .and_then(|(_, positions)| positions.get(object_id));
                let Some(previous_position) = previous_position else {
                    report.issues.push(format!(
                        "missing previous snapshot for {object_id} before step {step}"
                    ));
                    continue;
                };
                for vec in flow {
                    let expected = [
                        current_position[0] - previous_position[0],
                        current_position[1] - previous_position[1],
                        current_position[2] - previous_position[2],
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
            _ => {}
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
            RunLogEvent::InterventionApplied {
                intervention_type,
                payload,
                ..
            } => apply_replay_intervention(&mut sim, intervention_type, payload, &mut report),
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

fn apply_replay_intervention(
    sim: &mut Option<DeterministicObjectSim>,
    intervention_type: &str,
    payload: &Value,
    report: &mut SimReplayReport,
) {
    let Some(current) = sim.as_mut() else {
        report.issues.push(format!(
            "{intervention_type} intervention appeared before any sim_snapshot seed"
        ));
        return;
    };
    match current.apply_intervention(intervention_type, payload) {
        Ok(_) => report.checked_interventions += 1,
        Err(err) => report.issues.push(format!(
            "failed to replay {intervention_type} intervention: {err}"
        )),
    }
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
        canonical_json_hash, read_events, replay_events, validate_events, ActorType, RunLogEvent,
        RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION,
    };
    use serde_json::json;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    fn temp_path(prefix: &str, extension: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{stamp}.{extension}"))
    }

    fn append_run_prefix<W: std::io::Write>(
        writer: &mut RunLogWriter<W>,
        run_id: &str,
        test_name: &str,
    ) {
        let config = json!({ "test": test_name });
        let config_hash = canonical_json_hash(&config).unwrap();
        writer
            .append(&RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: run_id.to_string(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: BTreeMap::new(),
            })
            .unwrap();
        writer
            .append(&RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            })
            .unwrap();
    }

    fn write_minimal_run_log(path: &Path, test_name: &str) {
        let mut writer = RunLogWriter::create(path).unwrap();
        append_run_prefix(&mut writer, test_name, test_name);
        writer
            .append(&RunLogEvent::RunEnded {
                run_id: test_name.to_string(),
                timestamp_ns: 1,
                status: RunStatus::Succeeded,
                message: None,
            })
            .unwrap();
        writer.flush().unwrap();
    }

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
        let state = replay_events(&events).unwrap();
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
        let state = replay_events(&events).unwrap();
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
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.actions.len(), 0);
        assert_eq!(state.sim_snapshots, 0);
    }

    #[test]
    fn bridge_session_safe_mode_allows_log_replay() {
        let path = temp_path("pid-sim-log-replay", "jsonl");
        write_minimal_run_log(&path, "replay-source");

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
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.actions.len(), 0);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bridge_session_handles_log_lifecycle_methods() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-log-lifecycle-test".to_string(),
            session_id: None,
        };
        let run_id = "bridge-log-lifecycle-run";
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, run_id, "log_lifecycle");
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::with_run_id(writer, sim, run_id);

        let start = bridge_request(
            "req-log-start",
            BridgeMethod::LogStart,
            actor.clone(),
            Some(0),
            0,
            json!({ "run_id": run_id, "metadata": { "purpose": "test" } }),
        );
        let start_response = session.dispatch(&start).unwrap();
        assert!(start_response.ok, "{:?}", start_response.message);
        let start_result = start_response.result.unwrap();
        assert_eq!(start_result["run_id"], run_id);
        assert_eq!(start_result["active"], true);

        let stop = bridge_request(
            "req-log-stop",
            BridgeMethod::LogStop,
            actor,
            Some(0),
            0,
            json!({}),
        );
        let stop_response = session.dispatch(&stop).unwrap();
        assert!(stop_response.ok, "{:?}", stop_response.message);
        assert!(session.run_ended());
        assert!(!session.finish_run(RunStatus::Succeeded, None).unwrap());

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let validation = validate_events(&events).unwrap();
        assert!(validation.is_valid(), "{:?}", validation.issues);
        assert!(
            matches!(events.last(), Some(RunLogEvent::RunEnded { run_id: id, .. }) if id == run_id)
        );
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 4);
        assert_eq!(state.status, Some(RunStatus::Succeeded));
    }

    #[test]
    fn bridge_session_applies_and_replays_intervention() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-intervention-test".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::new(writer, sim);
        let intervention = bridge_request(
            "req-intervention",
            BridgeMethod::InterventionApply,
            actor.clone(),
            Some(0),
            0,
            json!({
                "intervention_type": "set_velocity",
                "payload": {
                    "object_id": "red_cube",
                    "velocity": [0.2, 0.0, 0.0]
                }
            }),
        );
        let response = session.dispatch(&intervention).unwrap();
        assert!(response.ok, "{:?}", response.message);
        let result = response.result.unwrap();
        assert_eq!(result["accepted"], true);
        assert_eq!(result["intervention_type"], "set_velocity");

        let step = bridge_request(
            "req-step-after-intervention",
            BridgeMethod::SimStep,
            actor,
            Some(0),
            0,
            json!({ "dt": 0.5 }),
        );
        assert!(session.dispatch(&step).unwrap().ok);

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert_eq!(state.interventions.len(), 1);
        assert_eq!(state.actions.len(), 1);
        let replay = verify_sim_replay(&events, 1e-12);
        assert!(replay.is_valid(), "{:?}", replay.issues);
        assert_eq!(replay.checked_interventions, 1);
        assert_eq!(replay.checked_actions, 1);
        let flow = verify_flow_gt(&events, 1e-12);
        assert!(flow.is_valid(), "{:?}", flow.issues);
    }

    #[test]
    fn bridge_session_export_rerun_writes_artifact() {
        let source = temp_path("pid-sim-export-rerun-source", "jsonl");
        let output = temp_path("pid-sim-export-rerun-output", "rrd");
        write_minimal_run_log(&source, "export-rerun-source");

        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-export-rerun-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let request = bridge_request(
            "req-export-rerun",
            BridgeMethod::ExportRerun,
            actor,
            Some(0),
            0,
            json!({
                "run_log_uri": source.display().to_string(),
                "output_uri": output.display().to_string(),
            }),
        );
        let response = session.dispatch(&request).unwrap();
        assert!(response.ok, "{:?}", response.message);
        let result = response.result.unwrap();
        assert_eq!(result["output_uri"], output.display().to_string());
        assert_eq!(result["events"], 3);
        assert_eq!(result["valid"], true);
        assert_eq!(result["validation_errors"], 0);
        assert_eq!(result["trace_hash"].as_str().unwrap().len(), 64);
        assert_eq!(result["sha256"].as_str().unwrap().len(), 64);
        assert!(output.exists());

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.actions.len(), 0);
        assert_eq!(state.artifacts.len(), 1);
        assert_eq!(state.artifacts[0].kind, "rerun_rrd");
        assert_eq!(state.artifacts[0].uri, output.display().to_string());
        assert_eq!(state.artifacts[0].sha256.as_deref().unwrap().len(), 64);
        assert!(events.iter().any(|event| matches!(
            event,
            RunLogEvent::ArtifactLogged { metadata, .. }
                if metadata.contains_key("trace_hash")
        )));

        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn bridge_session_export_rerun_refuses_own_run_log_and_non_rrd_output() {
        let source = temp_path("pid-sim-export-guard-source", "jsonl");
        write_minimal_run_log(&source, "export-guard-source");
        let own_log = temp_path("pid-sim-export-guard-own", "jsonl");
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-export-guard-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::new(writer, demo_sim());
        session.set_run_log_path(&own_log);

        // Writing over the session's own run log must be refused outright.
        let overwrite = bridge_request(
            "req-export-own-log",
            BridgeMethod::ExportRerun,
            actor.clone(),
            Some(0),
            0,
            json!({
                "run_log_uri": source.display().to_string(),
                "output_uri": own_log.display().to_string(),
            }),
        );
        let response = session.dispatch(&overwrite).unwrap();
        assert!(!response.ok);
        assert!(
            response
                .message
                .as_deref()
                .unwrap_or("")
                .contains("own run log"),
            "{:?}",
            response.message
        );

        // Any non-.rrd output is refused (save_recording truncates its target).
        let non_rrd = temp_path("pid-sim-export-guard-other", "jsonl");
        let bad_ext = bridge_request(
            "req-export-bad-ext",
            BridgeMethod::ExportRerun,
            actor,
            Some(0),
            0,
            json!({
                "run_log_uri": source.display().to_string(),
                "output_uri": non_rrd.display().to_string(),
            }),
        );
        let response = session.dispatch(&bad_ext).unwrap();
        assert!(!response.ok);
        assert!(
            response.message.as_deref().unwrap_or("").contains(".rrd"),
            "{:?}",
            response.message
        );
        assert!(!non_rrd.exists(), "refused export must not create the file");

        let _ = std::fs::remove_file(source);
    }

    #[test]
    fn rpc_dispatch_echoes_numeric_id_and_logs_rejections() {
        let mut writer = RunLogWriter::new(Vec::new());
        // A canonical run prefix, so the log can be validated end to end below.
        append_run_prefix(&mut writer, DEFAULT_BRIDGE_RUN_ID, "rpc-id-test");
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "rpc-id-test".to_string(),
            session_id: None,
        };
        // Numeric ids are valid JSON-RPC 2.0 and must be echoed VERBATIM.
        let ok = dispatch_rpc_text_request(
            r#"{"jsonrpc":"2.0","id":7,"method":"sim.status","params":{}}"#,
            1,
            &mut session,
            actor.clone(),
        );
        assert!(ok.is_ok(), "{:?}", ok.error);
        assert_eq!(ok.id, json!(7));
        // Unknown method → -32601 with the id echoed, and a run-log trace.
        let unknown = dispatch_rpc_text_request(
            r#"{"jsonrpc":"2.0","id":"probe","method":"sim.destroy","params":{}}"#,
            2,
            &mut session,
            actor.clone(),
        );
        assert_eq!(unknown.error.as_ref().unwrap().code, -32601);
        assert_eq!(unknown.id, json!("probe"));
        // Malformed JSON → -32700, id null per JSON-RPC 2.0, and a trace.
        let malformed = dispatch_rpc_text_request("{nope", 3, &mut session, actor.clone());
        assert_eq!(malformed.error.as_ref().unwrap().code, -32700);
        assert_eq!(malformed.id, Value::Null);
        // Structured (array) id → -32600 Invalid Request, id null, and a trace.
        let bad_id = dispatch_rpc_text_request(
            r#"{"jsonrpc":"2.0","id":[1],"method":"sim.status","params":{}}"#,
            4,
            &mut session,
            actor,
        );
        assert_eq!(bad_id.error.as_ref().unwrap().code, -32600);
        assert_eq!(bad_id.id, Value::Null);

        // Every rejected message must have left an error_logged trace:
        // probing/malformed traffic is exactly what the audit trail must keep.
        // (Not a bridge_response — an unpaired response would fail canonical
        // validation, letting one probe poison the log's validity.)
        session
            .finish_run(RunStatus::Succeeded, None)
            .expect("finish run");
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let rejected: Vec<String> = events
            .iter()
            .filter_map(|event| match event {
                RunLogEvent::ErrorLogged { message, .. }
                    if message.starts_with("rejected rpc ") =>
                {
                    Some(message.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(rejected.len(), 3, "{rejected:?}");
        // The unknown-method probe keeps its wire id greppable inside the
        // unique, type-tagged run-log id (message-2:s:probe).
        assert!(
            rejected
                .iter()
                .any(|m| m.contains("rejected rpc message-2:s:probe:")),
            "{rejected:?}"
        );
        assert!(
            rejected
                .iter()
                .all(|m| m.starts_with("rejected rpc message-")),
            "every rejection is message-indexed: {rejected:?}"
        );

        // The regression that motivated the ErrorLogged encoding: a log that
        // captured rejected traffic must still pass canonical validation.
        let report = pid_runlog::validate_events(&events).unwrap();
        assert!(
            report.is_valid(),
            "rejected RPCs must not poison run-log validity: {:?}",
            report.issues
        );
    }

    #[test]
    fn colliding_and_reused_rpc_ids_do_not_invalidate_the_log() {
        // JSON-RPC clients may legally send `1` and "1", reuse an id after
        // completion, and fire multiple notifications (null id). Under the old
        // bare rendering all of these produced duplicate run-log request ids,
        // which canonical validation hard-rejects — a spec-valid client could
        // invalidate the source-of-truth log.
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "rpc-collide-test".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, DEFAULT_BRIDGE_RUN_ID, "rpc-collide");
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let lines = [
            r#"{"jsonrpc":"2.0","id":1,"method":"sim.status","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":"1","method":"sim.status","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":"1","method":"sim.status","params":{}}"#,
            r#"{"jsonrpc":"2.0","method":"sim.status","params":{}}"#,
            r#"{"jsonrpc":"2.0","method":"sim.status","params":{}}"#,
        ];
        for (idx, line) in lines.iter().enumerate() {
            let response = dispatch_rpc_text_request(line, idx + 1, &mut session, actor.clone());
            assert!(response.is_ok(), "{:?}", response.error);
        }
        session
            .finish_run(RunStatus::Succeeded, None)
            .expect("finish run");
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let mut ids: Vec<String> = events
            .iter()
            .filter_map(|event| match event {
                RunLogEvent::BridgeRequest { request_id, .. } => Some(request_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(ids.len(), 5);
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "request ids must be unique: {ids:?}");
        let report = pid_runlog::validate_events(&events).unwrap();
        assert!(
            report.is_valid(),
            "colliding wire ids must not invalidate the log: {:?}",
            report.issues
        );
    }

    #[test]
    fn sim_reset_is_rejected_after_stepping() {
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "reset-guard-test".to_string(),
            session_id: None,
        };
        // Reset before the first step is allowed.
        let early = session
            .dispatch(&bridge_request(
                "reset-early",
                BridgeMethod::SimReset,
                actor.clone(),
                Some(session.step()),
                session.timestamp_ns(),
                json!({}),
            ))
            .unwrap();
        assert!(early.ok, "{:?}", early.message);
        let stepped = session
            .dispatch(&bridge_request(
                "step-1",
                BridgeMethod::SimStep,
                actor.clone(),
                Some(session.step()),
                session.timestamp_ns(),
                json!({ "dt": 0.1 }),
            ))
            .unwrap();
        assert!(stepped.ok, "{:?}", stepped.message);
        // Reset after stepping would regress run-log step/time — refuse it.
        let late = session
            .dispatch(&bridge_request(
                "reset-late",
                BridgeMethod::SimReset,
                actor,
                Some(session.step()),
                session.timestamp_ns(),
                json!({}),
            ))
            .unwrap();
        assert!(!late.ok);
        // The refusal keeps the log monotonic: no step/timestamp regressions
        // (run_started/run_ended framing is the transport binaries' job, so
        // only ordering errors are asserted here).
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let report = pid_runlog::validate_events(&events).unwrap();
        let ordering_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|issue| issue.message.contains("nondecreasing"))
            .collect();
        assert!(ordering_issues.is_empty(), "{ordering_issues:?}");
    }

    #[test]
    fn sim_step_rejects_non_numeric_dt() {
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "dt-test".to_string(),
            session_id: None,
        };
        let response = session
            .dispatch(&bridge_request(
                "step-bad-dt",
                BridgeMethod::SimStep,
                actor,
                Some(0),
                0,
                json!({ "dt": "fast" }),
            ))
            .unwrap();
        assert!(
            !response.ok,
            "a non-numeric dt must not silently become 0.1"
        );
        assert!(
            response.message.as_deref().unwrap_or("").contains("dt"),
            "{:?}",
            response.message
        );
    }

    #[test]
    fn verify_flow_gt_accepts_intervention_after_step() {
        // A pose intervention emits a second SimSnapshot at the SAME step; the
        // flows already logged for that step must not be retro-flagged, and the
        // NEXT step's flow must be measured from the post-intervention state.
        let sim = demo_sim();
        let mut writer = RunLogWriter::new(Vec::new());
        // Seed snapshot at step 0, as every transport binary writes.
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::new(writer, sim);
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "flow-intervention-test".to_string(),
            session_id: None,
        };
        for (id, method, payload) in [
            ("s1", BridgeMethod::SimStep, json!({ "dt": 0.1 })),
            (
                "i1",
                BridgeMethod::InterventionApply,
                json!({
                    "intervention_type": "translate_object",
                    "payload": { "object_id": "red_cube", "delta": [1.0, 0.0, 0.0] },
                }),
            ),
            ("s2", BridgeMethod::SimStep, json!({ "dt": 0.1 })),
        ] {
            let response = session
                .dispatch(&bridge_request(id, method, actor.clone(), None, 0, payload))
                .unwrap();
            assert!(response.ok, "{id}: {:?}", response.message);
        }
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let report = verify_flow_gt(&events, 1e-9);
        assert!(
            report.issues.is_empty(),
            "honest post-intervention log flagged: {:?}",
            report.issues
        );
        assert!(report.checked_flows > 0);
    }

    #[test]
    fn bridge_session_export_rerun_requires_run_log_uri() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-export-rerun-missing-input-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let request = bridge_request(
            "req-export-rerun-missing-input",
            BridgeMethod::ExportRerun,
            actor,
            Some(0),
            0,
            json!({ "output_uri": temp_path("pid-sim-export-rerun-missing", "rrd") }),
        );
        let response = session.dispatch(&request).unwrap();
        assert!(!response.ok);
        assert!(response
            .message
            .as_deref()
            .unwrap()
            .contains("requires string run_log_uri"));
    }

    #[test]
    fn bridge_session_export_rerun_rejects_source_overwrite() {
        let source = temp_path("pid-sim-export-rerun-overwrite-source", "jsonl");
        write_minimal_run_log(&source, "export-rerun-overwrite-source");

        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-export-rerun-overwrite-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let request = bridge_request(
            "req-export-rerun-overwrite",
            BridgeMethod::ExportRerun,
            actor,
            Some(0),
            0,
            json!({
                "run_log_uri": source.display().to_string(),
                "output_uri": source.display().to_string(),
            }),
        );
        let response = session.dispatch(&request).unwrap();
        assert!(!response.ok);
        assert!(response
            .message
            .as_deref()
            .unwrap()
            .contains("output_uri must differ"));

        let _ = std::fs::remove_file(source);
    }

    #[test]
    fn bridge_session_safe_mode_blocks_export_rerun() {
        let source = temp_path("pid-sim-safe-export-rerun-source", "jsonl");
        let output = temp_path("pid-sim-safe-export-rerun-output", "rrd");
        write_minimal_run_log(&source, "safe-export-rerun-source");

        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-safe-export-rerun-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::with_safe_mode(writer, demo_sim(), true);
        let request = bridge_request(
            "req-safe-export-rerun",
            BridgeMethod::ExportRerun,
            actor,
            Some(0),
            0,
            json!({
                "run_log_uri": source.display().to_string(),
                "output_uri": output.display().to_string(),
            }),
        );
        let response = session.dispatch(&request).unwrap();
        assert!(!response.ok);
        assert!(response
            .message
            .as_deref()
            .unwrap()
            .contains("safe mode blocked"));
        assert!(!output.exists());
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.artifacts.len(), 0);

        let _ = std::fs::remove_file(source);
    }

    #[test]
    fn scene_set_object_rejects_invalid_input_without_poisoning_the_log() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-scene-validate".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, DEFAULT_BRIDGE_RUN_ID, "scene-validate");
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::new(writer, sim);

        // Empty object_id: rejected at the request (same discipline as
        // intervention.apply), instead of being accepted and poisoning the
        // next SimSnapshot's validation.
        let bad = bridge_request(
            "req-empty-id",
            BridgeMethod::SceneSetObject,
            actor.clone(),
            Some(0),
            0,
            json!({
                "object_id": "",
                "pose": {
                    "position": [0.0, 0.0, 0.0],
                    "orientation_xyzw": [0.0, 0.0, 0.0, 1.0]
                },
                "velocity": [0.0, 0.0, 0.0]
            }),
        );
        let response = session.dispatch(&bad).unwrap();
        assert!(!response.ok, "empty object_id must be rejected");
        assert!(
            response
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("object_id"),
            "{:?}",
            response.message
        );

        // A valid request afterwards still succeeds, the rejected one left no
        // scene action, and the whole log still validates.
        let good = bridge_request(
            "req-good-object",
            BridgeMethod::SceneSetObject,
            actor,
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
        assert!(session.dispatch(&good).unwrap().ok);
        session
            .finish_run(RunStatus::Succeeded, None)
            .expect("finish run");
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        let scene_actions = state
            .actions
            .iter()
            .filter(|action| action.action_type == "scene.set_object")
            .count();
        assert_eq!(scene_actions, 1, "only the valid set_object is an action");
        let report = pid_runlog::validate_events(&events).unwrap();
        assert!(
            report.is_valid(),
            "rejected scene.set_object must not poison run-log validity: {:?}",
            report.issues
        );
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
        let state = replay_events(&events).unwrap();
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
        let state = replay_events(&events).unwrap();
        assert_eq!(state.actions.len(), 0);
        assert_eq!(state.sim_snapshots, 1);
    }

    #[test]
    fn rpc_line_processor_stops_after_log_stop() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-rpc-log-stop-test".to_string(),
            session_id: None,
        };
        let run_id = "rpc-log-stop-run";
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, run_id, "rpc_log_stop");
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::with_run_id(writer, sim, run_id);
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":"status","method":"sim.status","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":"stop","method":"log.stop","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":"step","method":"sim.step","params":{"dt":0.1}}"#,
            "\n"
        );
        let mut output = Vec::new();
        let handled =
            dispatch_rpc_lines(Cursor::new(input), &mut output, &mut session, actor).unwrap();
        assert_eq!(handled, 2);
        assert!(session.run_ended());

        let lines = String::from_utf8(output).unwrap();
        let responses = lines
            .lines()
            .map(|line| serde_json::from_str::<BridgeRpcResponse>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].id, "status");
        assert_eq!(responses[1].id, "stop");
        assert!(responses.iter().all(BridgeRpcResponse::is_ok));

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let validation = validate_events(&events).unwrap();
        assert!(validation.is_valid(), "{:?}", validation.issues);
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 4);
        assert_eq!(state.actions.len(), 0);
        assert_eq!(state.status, Some(RunStatus::Succeeded));
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
