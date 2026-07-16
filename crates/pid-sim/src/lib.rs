// The offline harness builds one large provenance `json!` literal; the default
// macro recursion limit is too small for it.
#![recursion_limit = "256"]

use anyhow::{bail, Context, Result};
use pid_bridge::{
    rpc_id_to_unique_request_id, rpc_notification_to_unique_request_id, BridgeHandler,
    BridgeMethod, BridgeRequest, BridgeResponse, BridgeRpcRequest, BridgeRpcResponse, LocalBridge,
};
use pid_runlog::{
    canonical_json_hash_v2, Actor, Pose, RunLogEvent, RunLogWriter, RunStatus, SimObjectSnapshot,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

pub mod h1_preflight;
pub mod h1_protocol_a;
pub mod h2_reference;
#[path = "power.rs"]
pub mod legacy_sensitivity;
pub mod manipulation;
pub mod offline_harness;
pub mod physics;
pub mod toy_harness;

pub const FLOW_PRED_SOURCE: &str = "constant_velocity_baseline";
pub const DEFAULT_BRIDGE_RUN_ID: &str = "sim-bridge-run";

/// File-backed run-log sink whose `flush` also fsyncs file contents and file
/// metadata. The bridge calls `flush` before every wire response, so the three
/// executable transports do not acknowledge control while provenance remains
/// only in a userspace buffer. This does not fsync the parent directory and is
/// not a cross-file transaction with exported artifacts.
pub struct FsyncFileWriter {
    writer: BufWriter<File>,
}

impl FsyncFileWriter {
    pub fn new(file: File) -> Self {
        Self {
            writer: BufWriter::new(file),
        }
    }
}

impl Write for FsyncFileWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buffer)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().sync_all()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
struct SetVelocityIntervention {
    object_id: String,
    velocity: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TranslateObjectIntervention {
    object_id: String,
    delta: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    terminal_write_attempted: bool,
    stop_requested: Option<String>,
    poisoned: bool,
    /// Set only while a newly installed export is crossing its provenance
    /// commit boundary. Cleanup is allowed only before an `artifact_logged`
    /// write starts; after that, a generic writer cannot reveal whether a full
    /// line reached the sink, so the file is retained to avoid a false link.
    pending_export: Option<PendingExport>,
    /// Where this session's own run log lives, when file-backed. `export.rerun`
    /// must never be allowed to write over it: the run log is the source of
    /// truth and export outputs are separate no-replace artifacts.
    run_log_path: Option<PathBuf>,
    /// Canonical directory containing every path that file-bearing RPCs may
    /// read or create. File methods fail closed until this is configured.
    artifact_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct PendingExport {
    path: PathBuf,
    sha256: String,
    provenance_write_attempted: bool,
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
                let position = [
                    object.pose.position[0] + parsed.delta[0],
                    object.pose.position[1] + parsed.delta[1],
                    object.pose.position[2] + parsed.delta[2],
                ];
                validate_vec3_finite(position, "translated object position")?;
                object.pose.position = position;
                Ok(json!({
                    "object_id": parsed.object_id,
                    "delta": parsed.delta,
                    "position": position,
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
        let rounded_dt_ns = (dt_secs * 1_000_000_000.0).round();
        // `as u64` saturates oversized floats, which would hide an invalid dt
        // until a later timestamp addition panics in debug or wraps in release.
        // The f64 representation of u64::MAX rounds up to 2^64, so require a
        // strict inequality before casting.
        if !rounded_dt_ns.is_finite() || rounded_dt_ns < 1.0 || rounded_dt_ns >= u64::MAX as f64 {
            bail!("dt_secs must round to a representable positive nanosecond interval");
        }
        let dt_ns = rounded_dt_ns as u64;
        let next_step = self
            .step
            .checked_add(1)
            .context("simulation step overflow")?;
        let next_timestamp_ns = self
            .timestamp_ns
            .checked_add(dt_ns)
            .context("simulation timestamp overflow")?;

        // Validate every numeric effect before mutating any object. This keeps
        // the public simulator API transactional even outside SimBridgeSession's
        // cloned-handler staging.
        let mut flow_gt = Vec::with_capacity(self.objects.len());
        let mut next_positions = Vec::with_capacity(self.objects.len());
        for object in self.objects.values() {
            let displacement = [
                object.velocity[0] * dt_secs,
                object.velocity[1] * dt_secs,
                object.velocity[2] * dt_secs,
            ];
            validate_vec3_finite(displacement, "step displacement")?;
            let position = [
                object.pose.position[0] + displacement[0],
                object.pose.position[1] + displacement[1],
                object.pose.position[2] + displacement[2],
            ];
            validate_vec3_finite(position, "stepped object position")?;
            next_positions.push(position);
            flow_gt.push(FlowGtRecord {
                object_id: object.object_id.clone(),
                displacement,
            });
        }
        for (object, position) in self.objects.values_mut().zip(next_positions) {
            object.pose.position = position;
        }
        self.step = next_step;
        self.timestamp_ns = next_timestamp_ns;
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

fn validate_bridge_payload_keys(
    payload: &Value,
    method: &str,
    allowed: &[&str],
    allow_omitted: bool,
) -> Result<()> {
    let params = match payload {
        Value::Null if allow_omitted => return Ok(()),
        Value::Object(params) => params,
        other => bail!("{method} parameters must be an object, got {other}"),
    };
    for name in params.keys() {
        if !allowed.contains(&name.as_str()) {
            bail!("unknown parameter {name:?} for method {method}");
        }
    }
    Ok(())
}

impl BridgeHandler for SimBridgeHandler {
    fn handle(&mut self, request: &BridgeRequest) -> Result<Value> {
        self.last_step = None;
        match request.method {
            BridgeMethod::SimStatus => {
                validate_bridge_payload_keys(&request.payload, "sim.status", &[], true)?;
                Ok(self.status_json())
            }
            BridgeMethod::SimReset => {
                validate_bridge_payload_keys(&request.payload, "sim.reset", &[], true)?;
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
                validate_bridge_payload_keys(&request.payload, "sim.step", &["dt"], false)?;
                let value = request
                    .payload
                    .get("dt")
                    .context("sim.step requires numeric dt")?;
                let dt = value
                    .as_f64()
                    .with_context(|| format!("sim.step dt must be a number, got {value}"))?;
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
                validate_bridge_payload_keys(
                    &request.payload,
                    "scene.set_object",
                    &["object_id", "pose", "velocity"],
                    false,
                )?;
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
                validate_bridge_payload_keys(
                    &request.payload,
                    "intervention.apply",
                    &["intervention_type", "payload"],
                    false,
                )?;
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
                bail!("log.replay requires SimBridgeSession artifact-root confinement")
            }
            BridgeMethod::ExportRerun => {
                bail!("export.rerun requires SimBridgeSession artifact-root confinement")
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
            terminal_write_attempted: false,
            stop_requested: None,
            poisoned: false,
            pending_export: None,
            run_log_path: None,
            artifact_root: None,
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
            terminal_write_attempted: false,
            stop_requested: None,
            poisoned: false,
            pending_export: None,
            run_log_path: None,
            artifact_root: None,
        }
    }

    /// Tell the session where its own run log lives so `export.rerun` can
    /// refuse to overwrite it. File-backed transports should always set this.
    pub fn set_run_log_path(&mut self, path: impl Into<PathBuf>) {
        self.run_log_path = Some(path.into());
    }

    /// Restrict `log.replay` and `export.rerun` to one canonical directory in
    /// a non-adversarial local filesystem.
    ///
    /// The root must be an existing directory reached without traversing a
    /// symlink. RPC paths are later checked component by component, so a
    /// symlink observed inside the root cannot be used to escape it.
    /// Descriptor-relative race resistance against a concurrent filesystem
    /// adversary is outside this E0 boundary. File-bearing RPCs remain disabled
    /// until a root is set.
    pub fn set_artifact_root(&mut self, root: impl AsRef<Path>) -> Result<()> {
        self.artifact_root = Some(canonical_artifact_root(root)?);
        Ok(())
    }

    pub fn artifact_root(&self) -> Option<&Path> {
        self.artifact_root.as_deref()
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

    pub fn stop_requested(&self) -> bool {
        self.stop_requested.is_some()
    }

    pub fn poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn dispatch(&mut self, request: &BridgeRequest) -> Result<BridgeResponse> {
        if self.run_ended {
            bail!("bridge run {} already ended", self.run_id);
        }
        if self.poisoned {
            bail!(
                "bridge run {} is poisoned by an earlier provenance-write failure",
                self.run_id
            );
        }
        match self.dispatch_inner(request) {
            Ok(response) => Ok(response),
            Err(error) => {
                // A domain/request error is encoded as a successfully logged
                // BridgeResponse below. Therefore every Err escaping the inner
                // path is an infrastructure/provenance failure. Mutations are
                // staged on a clone and installed only after every intended
                // reconstruction event append returns success. Any partial
                // prefix accepted before a later failure is indeterminate; the
                // session is poisoned and no wire response is returned.
                self.poisoned = true;
                match self.rollback_pending_export() {
                    Ok(()) => Err(error),
                    Err(cleanup_error) => Err(error.context(format!(
                        "failed to remove unlogged export artifact: {cleanup_error:#}"
                    ))),
                }
            }
        }
    }

    fn dispatch_inner(&mut self, request: &BridgeRequest) -> Result<BridgeResponse> {
        self.bridge.record_request(request)?;
        if self.bridge.safe_mode() && !request.safe_mode_allowed() {
            let response = BridgeResponse::blocked_by_safe_mode(
                request,
                request.timestamp_ns.max(self.handler.sim.timestamp_ns()),
            );
            self.bridge.record_response(&response)?;
            self.bridge.flush()?;
            return Ok(response);
        }
        if matches!(
            request.method,
            BridgeMethod::LogReplay | BridgeMethod::ExportRerun
        ) {
            self.bridge.flush()?;
        }
        let mut staged_handler = self.handler.clone();
        let handled = match request.method {
            BridgeMethod::LogStart => self.handle_log_start(request),
            BridgeMethod::LogStop => self.handle_log_stop(request),
            BridgeMethod::LogReplay => self.handle_log_replay(request),
            BridgeMethod::ExportRerun => self.handle_export_rerun(request),
            _ => staged_handler.handle(request),
        };
        let mutating_handler_method = matches!(
            request.method,
            BridgeMethod::SimStep
                | BridgeMethod::SceneSetObject
                | BridgeMethod::SimReset
                | BridgeMethod::InterventionApply
        );
        let (response_step, response_timestamp_ns) = if handled.is_ok() {
            (staged_handler.sim.step(), staged_handler.sim.timestamp_ns())
        } else {
            (self.handler.sim.step(), self.handler.sim.timestamp_ns())
        };
        let response = match handled {
            Ok(result) => BridgeResponse {
                request_id: request.request_id.clone(),
                step: Some(response_step),
                timestamp_ns: response_timestamp_ns,
                ok: true,
                message: None,
                result: Some(result),
            },
            Err(err) => BridgeResponse {
                request_id: request.request_id.clone(),
                step: Some(response_step),
                timestamp_ns: response_timestamp_ns,
                ok: false,
                message: Some(err.to_string()),
                result: None,
            },
        };
        // A successful response is the acknowledgement record and therefore
        // comes LAST: every event needed to reconstruct the effect is appended
        // before it. The handler mutation remains staged while those effect
        // events are written.
        match request.method {
            BridgeMethod::SimStep if response.ok => {
                self.record_action(request, &staged_handler.sim)?;
                self.bridge
                    .record_event(&staged_handler.sim.snapshot_event())?;
                for event in staged_handler.sim.pose_events() {
                    self.bridge.record_event(&event)?;
                }
                if let Some(step) = &staged_handler.last_step {
                    for event in step.flow_events() {
                        self.bridge.record_event(&event)?;
                    }
                    for event in step.flow_pred_events() {
                        self.bridge.record_event(&event)?;
                    }
                }
            }
            BridgeMethod::SceneSetObject if response.ok => {
                self.record_action(request, &staged_handler.sim)?;
                self.bridge
                    .record_event(&staged_handler.sim.snapshot_event())?;
                for event in staged_handler.sim.pose_events() {
                    self.bridge.record_event(&event)?;
                }
            }
            BridgeMethod::SimReset if response.ok => {
                self.record_action(request, &staged_handler.sim)?;
                self.bridge
                    .record_event(&staged_handler.sim.snapshot_event())?;
            }
            BridgeMethod::InterventionApply if response.ok => {
                self.record_intervention(request, &staged_handler.sim)?;
                self.bridge
                    .record_event(&staged_handler.sim.snapshot_event())?;
                for event in staged_handler.sim.pose_events() {
                    self.bridge.record_event(&event)?;
                }
            }
            BridgeMethod::ExportRerun if response.ok => {
                // Once this write starts, an error may still mean that a full
                // JSON line reached the sink before its newline/flush failed.
                // Retain the installed artifact from this point onward so any
                // surviving provenance never names a deliberately deleted file.
                let pending = {
                    let pending = self
                        .pending_export
                        .as_mut()
                        .context("successful export.rerun omitted pending artifact")?;
                    pending.provenance_write_attempted = true;
                    pending.clone()
                };
                self.bridge.record_event(&RunLogEvent::ArtifactLogged {
                    timestamp_ns: response.timestamp_ns,
                    name: "rerun_recording".to_string(),
                    kind: "rerun_rrd".to_string(),
                    uri: pending.path.display().to_string(),
                    sha256: Some(pending.sha256),
                    metadata: response
                        .result
                        .as_ref()
                        .and_then(|result| result.get("trace_hash_v2"))
                        .and_then(Value::as_str)
                        .map(|trace_hash| {
                            [
                                ("trace_hash_v2".to_string(), trace_hash.to_string()),
                                (
                                    "trace_hash_revision".to_string(),
                                    "replay_trace_v2".to_string(),
                                ),
                            ]
                            .into_iter()
                            .collect()
                        })
                        .unwrap_or_default(),
                })?;
            }
            _ => {}
        }
        // Successful effect-event appends are the conservative local
        // state-retention threshold. From here on, retain the mutation even if
        // the response append or flush fails: an arbitrary `Write` error cannot
        // prove that earlier accepted evidence was absent from the sink, so
        // rollback would risk false provenance.
        if response.ok && mutating_handler_method {
            self.handler = staged_handler;
        }
        self.bridge.record_response(&response)?;
        // A wire success must never precede `W::flush` returning success. The
        // meaning and durability of flush are defined by the supplied sink.
        // The executable transports use FsyncFileWriter (file sync, but no
        // parent-directory or cross-file transaction); generic callers may not.
        self.bridge.flush()?;
        if request.method == BridgeMethod::ExportRerun && response.ok {
            self.pending_export = None;
        }
        if request.method == BridgeMethod::LogStop && response.ok {
            self.stop_requested = Some(format!(
                "log.stop response delivered for {}",
                request.actor.actor_id
            ));
        }

        Ok(response)
    }

    fn rollback_pending_export(&mut self) -> Result<()> {
        let Some(pending) = self.pending_export.as_ref().cloned() else {
            return Ok(());
        };
        if pending.provenance_write_attempted {
            // Keep the file: a failing `Write` cannot tell us whether the
            // complete ArtifactLogged line reached the sink. Deleting here
            // could turn surviving provenance into a false reference.
            self.pending_export = None;
            return Ok(());
        }
        let metadata = match std::fs::symlink_metadata(&pending.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.pending_export = None;
                return Ok(());
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to inspect pending export {}",
                        pending.path.display()
                    )
                });
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            bail!(
                "pending export changed type; refusing cleanup: {}",
                pending.path.display()
            );
        }
        let actual_sha256 = pid_runlog::sha256_file(&pending.path)?;
        if actual_sha256 != pending.sha256 {
            bail!(
                "pending export changed bytes; refusing cleanup: {}",
                pending.path.display()
            );
        }
        std::fs::remove_file(&pending.path).with_context(|| {
            format!("failed to remove pending export {}", pending.path.display())
        })?;
        self.pending_export = None;
        Ok(())
    }

    pub fn step(&self) -> u64 {
        self.handler.sim.step()
    }

    pub fn timestamp_ns(&self) -> u64 {
        self.handler.sim.timestamp_ns()
    }

    fn handle_log_replay(&self, request: &BridgeRequest) -> Result<Value> {
        validate_bridge_payload_keys(&request.payload, "log.replay", &["run_log_uri"], false)?;
        let run_log_uri = request
            .payload
            .get("run_log_uri")
            .and_then(Value::as_str)
            .context("log.replay requires string run_log_uri")?;
        let root = self
            .artifact_root
            .as_deref()
            .context("log.replay is disabled until an artifact root is configured")?;
        let run_log_path = resolve_existing_artifact_path(root, Path::new(run_log_uri))?;
        let (events, _) = snapshot_runlog(&run_log_path)?;
        let summary = pid_runlog::summarize_events(&events)?;
        Ok(json!({
            "trace_hash": summary.trace_hash,
            "trace_hash_v2": summary.trace_hash_v2,
            "trace_hash_revision": "replay_trace_v2",
            "events": summary.event_count,
            "valid": summary.validation_errors == 0,
            "validation_errors": summary.validation_errors,
            "validation_warnings": summary.validation_warnings,
            "config_hash": summary.config_hash,
        }))
    }

    fn handle_export_rerun(&mut self, request: &BridgeRequest) -> Result<Value> {
        validate_bridge_payload_keys(
            &request.payload,
            "export.rerun",
            &["run_log_uri", "output_uri"],
            false,
        )?;
        let run_log_uri = request
            .payload
            .get("run_log_uri")
            .and_then(Value::as_str)
            .context("export.rerun requires string run_log_uri")?;
        let root = self
            .artifact_root
            .as_deref()
            .context("export.rerun is disabled until an artifact root is configured")?;
        let run_log_path = resolve_existing_artifact_path(root, Path::new(run_log_uri))?;
        let requested_output = match request.payload.get("output_uri") {
            None => default_rerun_output_path(&run_log_path),
            Some(Value::String(path)) => PathBuf::from(path),
            Some(_) => bail!("export.rerun output_uri must be a string when provided"),
        };
        let output_candidate = artifact_candidate(root, &requested_output)?;
        if paths_resolve_to_same_target(&output_candidate, &run_log_path) {
            bail!("export.rerun output_uri must differ from run_log_uri");
        }
        let output_path = resolve_new_artifact_path(root, &requested_output)?;
        if let Some(own) = &self.run_log_path {
            if paths_resolve_to_same_target(&output_path, own) {
                bail!(
                    "export.rerun output_uri {} resolves to this session's own run log; refusing",
                    output_path.display()
                );
            }
        }
        if paths_refer_to_same_file(&run_log_path, &output_path) {
            bail!(
                "export.rerun output_uri {} must differ from run_log_uri",
                output_path.display()
            );
        }
        let exported = export_runlog_to_rerun_inner(&run_log_path, &output_path)?;
        self.pending_export = Some(PendingExport {
            path: exported.output_path,
            sha256: exported.sha256,
            provenance_write_attempted: false,
        });
        Ok(exported.response)
    }

    /// Record a rejected RPC message (malformed JSON, invalid id, unknown
    /// method) in the provenance log — grandplan's control-plane rule is
    /// "append every request and response summary to the run log". The trace
    /// is an [`RunLogEvent::ErrorLogged`] event, **not** a `BridgeResponse`:
    /// a rejected message has no recordable `BridgeRequest` (its method may be
    /// unknown, its id structurally invalid, or its JSON unparseable), and an
    /// unpaired response makes the canonical run-log validation fail — the old
    /// encoding let one probing/malformed message poison the validity of the
    /// very log this hook exists to keep audit-complete. A provenance-write
    /// failure poisons the session and is returned instead of being hidden
    /// behind the protocol error.
    pub fn record_rejected_rpc(&mut self, request_id: &str, message: &str) -> Result<()> {
        if self.run_ended || self.poisoned {
            bail!("bridge run {} is no longer writable", self.run_id);
        }
        let event = RunLogEvent::ErrorLogged {
            step: Some(self.handler.sim.step()),
            timestamp_ns: self.handler.sim.timestamp_ns(),
            message: format!("rejected rpc {request_id}: {message}"),
            recoverable: true,
        };
        if let Err(error) = self.bridge.record_event(&event) {
            self.poisoned = true;
            return Err(error).context("failed to record rejected RPC in canonical run log");
        }
        if let Err(error) = self.bridge.flush() {
            self.poisoned = true;
            return Err(error).context("failed to flush rejected RPC to canonical run log");
        }
        Ok(())
    }

    pub fn record_event(&mut self, event: &RunLogEvent) -> Result<()> {
        if matches!(event, RunLogEvent::RunEnded { .. }) {
            bail!("use finish_run to append and flush the terminal run event");
        }
        if self.poisoned {
            bail!(
                "bridge run {} is poisoned by an earlier provenance-write failure",
                self.run_id
            );
        }
        if let Err(error) = self.bridge.record_event(event) {
            self.poisoned = true;
            return Err(error).context("failed to append canonical bridge event");
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if let Err(error) = self.bridge.flush() {
            self.poisoned = true;
            return Err(error).context("failed to flush canonical bridge run log");
        }
        Ok(())
    }

    pub fn finish_run(&mut self, status: RunStatus, message: Option<String>) -> Result<bool> {
        if self.run_ended {
            return Ok(false);
        }
        if self.terminal_write_attempted {
            bail!(
                "bridge run {} terminal write was already attempted and cannot be retried safely",
                self.run_id
            );
        }
        if self.poisoned {
            bail!(
                "bridge run {} cannot be sealed after an earlier provenance-write failure",
                self.run_id
            );
        }
        let event = RunLogEvent::RunEnded {
            run_id: self.run_id.clone(),
            timestamp_ns: self.handler.sim.timestamp_ns(),
            status,
            message,
        };
        // Once an arbitrary Write reports an error we cannot tell how much of
        // its JSON line reached the sink, so retrying could duplicate a
        // complete terminal event. Permit exactly one terminal write attempt.
        self.terminal_write_attempted = true;
        if let Err(error) = self.bridge.record_event(&event) {
            self.poisoned = true;
            return Err(error).context("failed to seal canonical bridge run log");
        }
        if let Err(error) = self.bridge.flush() {
            self.poisoned = true;
            return Err(error).context("failed to flush canonical bridge run-log seal");
        }
        self.run_ended = true;
        self.stop_requested = None;
        Ok(true)
    }

    pub fn into_inner(self) -> W {
        self.bridge.into_inner()
    }

    fn handle_log_start(&self, request: &BridgeRequest) -> Result<Value> {
        validate_bridge_payload_keys(&request.payload, "log.start", &["run_id", "metadata"], true)?;
        if let Some(value) = request.payload.get("run_id") {
            match value {
                Value::String(requested_run_id) => {
                    if requested_run_id.is_empty() {
                        bail!("log.start run_id must not be empty");
                    }
                    if requested_run_id != &self.run_id {
                        bail!(
                            "log.start run_id {requested_run_id} does not match active run {}",
                            self.run_id
                        );
                    }
                }
                other => bail!("log.start run_id must be a string when provided, got {other}"),
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
        validate_bridge_payload_keys(&request.payload, "log.stop", &[], true)?;
        Ok(json!({
            "run_id": self.run_id,
            "stopped": true,
            "step": self.handler.sim.step(),
            "timestamp_ns": self.handler.sim.timestamp_ns(),
        }))
    }

    fn record_action(
        &mut self,
        request: &BridgeRequest,
        sim: &DeterministicObjectSim,
    ) -> Result<()> {
        self.bridge.record_event(&RunLogEvent::ActionApplied {
            step: sim.step(),
            timestamp_ns: sim.timestamp_ns(),
            actor: request.actor.clone(),
            action_type: request.method.as_str().to_string(),
            payload_hash: request.payload_hash()?,
            payload: request.payload.clone(),
        })
    }

    fn record_intervention(
        &mut self,
        request: &BridgeRequest,
        sim: &DeterministicObjectSim,
    ) -> Result<()> {
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
            step: sim.step(),
            timestamp_ns: sim.timestamp_ns(),
            actor: request.actor.clone(),
            intervention_type: intervention_type.to_string(),
            payload_hash: canonical_json_hash_v2(&payload)?,
            payload,
        })
    }
}

/// Canonicalize the root used to confine file-bearing bridge RPCs.
///
/// # Errors
///
/// Returns an error when `root` is missing, is not a directory, traverses a
/// symlink, or cannot be canonicalized.
pub fn canonical_artifact_root(root: impl AsRef<Path>) -> Result<PathBuf> {
    let root = root.as_ref();
    let root = if root.as_os_str().is_empty() {
        Path::new(".")
    } else {
        root
    };
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to resolve current directory for artifact root")?
            .join(root)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::ParentDir => bail!(
                "artifact root must not contain parent traversal: {}",
                root.display()
            ),
            Component::CurDir => continue,
            Component::Prefix(_) | Component::RootDir => {
                current.push(component.as_os_str());
            }
            Component::Normal(segment) => {
                current.push(segment);
                let metadata = std::fs::symlink_metadata(&current).with_context(|| {
                    format!("failed to inspect artifact root {}", current.display())
                })?;
                if metadata.file_type().is_symlink() {
                    bail!(
                        "artifact root must not traverse a symlink: {}",
                        current.display()
                    );
                }
            }
        }
    }
    let metadata = std::fs::metadata(&absolute)
        .with_context(|| format!("failed to inspect artifact root {}", root.display()))?;
    if !metadata.is_dir() {
        bail!("artifact root must be a directory: {}", root.display());
    }
    absolute
        .canonicalize()
        .with_context(|| format!("failed to canonicalize artifact root {}", root.display()))
}

/// Resolve a not-yet-created artifact to its existing canonical parent.
///
/// Operator-facing binaries use this before `create_new`: platform-standard
/// parent aliases such as macOS `/var` are resolved once, and every later open
/// uses the returned canonical path rather than traversing that alias again.
pub fn canonical_new_artifact_path(path: impl AsRef<Path>) -> Result<(PathBuf, PathBuf)> {
    let path = path.as_ref();
    let file_name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .context("artifact output must name a file")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let canonical_parent = parent
        .canonicalize()
        .with_context(|| format!("artifact output parent must exist: {}", parent.display()))?;
    let root = canonical_artifact_root(&canonical_parent)?;
    Ok((root.join(file_name), root))
}

fn reject_unsafe_artifact_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("artifact path must not be empty");
    }
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        bail!(
            "artifact path must not contain parent traversal: {}",
            path.display()
        );
    }
    Ok(())
}

fn artifact_candidate(root: &Path, requested: &Path) -> Result<PathBuf> {
    reject_unsafe_artifact_path(requested)?;
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    candidate.strip_prefix(root).with_context(|| {
        format!(
            "artifact path {} escapes configured root {}",
            requested.display(),
            root.display()
        )
    })?;
    Ok(candidate)
}

fn reject_symlink_components(root: &Path, candidate: &Path) -> Result<()> {
    let relative = candidate.strip_prefix(root).with_context(|| {
        format!(
            "artifact path {} escapes configured root {}",
            candidate.display(),
            root.display()
        )
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(segment) = component else {
            bail!("artifact path has a non-canonical component");
        };
        current.push(segment);
        let metadata = std::fs::symlink_metadata(&current)
            .with_context(|| format!("failed to inspect artifact path {}", current.display()))?;
        if metadata.file_type().is_symlink() {
            bail!(
                "artifact path must not traverse a symlink: {}",
                current.display()
            );
        }
    }
    Ok(())
}

fn resolve_existing_artifact_path(root: &Path, requested: &Path) -> Result<PathBuf> {
    let candidate = artifact_candidate(root, requested)?;
    reject_symlink_components(root, &candidate)?;
    let metadata = std::fs::metadata(&candidate)
        .with_context(|| format!("failed to inspect artifact {}", candidate.display()))?;
    if !metadata.is_file() {
        bail!("artifact is not a regular file: {}", candidate.display());
    }
    let canonical = candidate
        .canonicalize()
        .with_context(|| format!("failed to canonicalize artifact {}", candidate.display()))?;
    if !canonical.starts_with(root) {
        bail!(
            "artifact path {} escapes configured root {}",
            requested.display(),
            root.display()
        );
    }
    Ok(canonical)
}

fn resolve_new_artifact_path(root: &Path, requested: &Path) -> Result<PathBuf> {
    let candidate = artifact_candidate(root, requested)?;
    match std::fs::symlink_metadata(&candidate) {
        Ok(_) => bail!(
            "artifact output already exists; refusing to overwrite {}",
            candidate.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!("failed to inspect artifact output {}", candidate.display())
            });
        }
    }
    let parent = candidate
        .parent()
        .context("artifact output must have a parent directory")?;
    reject_symlink_components(root, parent)?;
    let canonical_parent = parent.canonicalize().with_context(|| {
        format!(
            "artifact output parent must already exist: {}",
            parent.display()
        )
    })?;
    if !canonical_parent.starts_with(root) {
        bail!(
            "artifact output {} escapes configured root {}",
            requested.display(),
            root.display()
        );
    }
    let file_name = candidate
        .file_name()
        .context("artifact output must name a file")?;
    Ok(canonical_parent.join(file_name))
}

fn install_new_artifact(path: &Path, bytes: &[u8]) -> Result<String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to stage new artifact in {}", parent.display()))?;
    staged
        .write_all(bytes)
        .with_context(|| format!("failed to stage new artifact {}", path.display()))?;
    staged
        .as_file()
        .sync_all()
        .with_context(|| format!("failed to sync staged artifact {}", path.display()))?;
    let expected_sha256 = pid_runlog::sha256_hex(bytes);
    let _file = staged.persist_noclobber(path).map_err(|error| {
        anyhow::Error::new(error.error).context(format!(
            "artifact output already exists or cannot be installed: {}",
            path.display()
        ))
    })?;
    // No fallible operation follows persistence: an error return must never
    // strand a final-path artifact that then blocks a no-replace retry. The
    // staged file itself was synced above; directory durability is not claimed.
    Ok(expected_sha256)
}

fn snapshot_runlog(run_log_uri: &Path) -> Result<(Vec<RunLogEvent>, Vec<u8>)> {
    let symlink_metadata = std::fs::symlink_metadata(run_log_uri)
        .with_context(|| format!("failed to inspect run log {}", run_log_uri.display()))?;
    if symlink_metadata.file_type().is_symlink() || !symlink_metadata.is_file() {
        bail!(
            "export.rerun source must be a non-symlink regular file: {}",
            run_log_uri.display()
        );
    }

    let mut source = OpenOptions::new()
        .read(true)
        .open(run_log_uri)
        .with_context(|| format!("failed to open run log {}", run_log_uri.display()))?;
    let source_identity = same_file::Handle::from_file(
        source
            .try_clone()
            .context("failed to clone run-log snapshot handle")?,
    )
    .context("failed to identify open run-log snapshot")?;
    let limits = pid_runlog::RunLogLimits::default();
    let start_len = source
        .metadata()
        .with_context(|| format!("failed to stat run log {}", run_log_uri.display()))?
        .len();
    if start_len > limits.max_file_bytes {
        bail!(
            "run log exceeds the {}-byte export limit: {}",
            limits.max_file_bytes,
            run_log_uri.display()
        );
    }
    let capacity = usize::try_from(start_len).context("run-log size does not fit in memory")?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .context("failed to reserve run-log snapshot buffer")?;
    (&mut source)
        .take(limits.max_file_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to snapshot run log {}", run_log_uri.display()))?;
    if bytes.len() as u128 > u128::from(limits.max_file_bytes) {
        bail!(
            "run log exceeds the {}-byte export limit: {}",
            limits.max_file_bytes,
            run_log_uri.display()
        );
    }
    let end_len = source
        .metadata()
        .with_context(|| format!("failed to restat run log {}", run_log_uri.display()))?
        .len();
    if start_len != end_len || u64::try_from(bytes.len()).ok() != Some(start_len) {
        bail!(
            "run log changed while it was being snapshotted: {}",
            run_log_uri.display()
        );
    }
    let named_identity = same_file::Handle::from_path(run_log_uri)
        .with_context(|| format!("failed to re-identify run log {}", run_log_uri.display()))?;
    if source_identity != named_identity {
        bail!(
            "run log path changed while it was being snapshotted: {}",
            run_log_uri.display()
        );
    }

    let events = pid_runlog::read_events_with_limits(std::io::Cursor::new(&bytes), limits)?;
    Ok((events, bytes))
}

fn manifest_for_snapshot(
    run_log_uri: &Path,
    events: &[RunLogEvent],
    bytes: &[u8],
) -> Result<pid_runlog::RunManifest> {
    let mut snapshot = tempfile::NamedTempFile::new()
        .context("failed to create exact run-log manifest snapshot")?;
    snapshot
        .write_all(bytes)
        .context("failed to write exact run-log manifest snapshot")?;
    snapshot
        .as_file()
        .sync_all()
        .context("failed to sync exact run-log manifest snapshot")?;
    let mut manifest = pid_runlog::manifest_for_events(snapshot.path(), events)?;
    manifest.run_log_uri = run_log_uri.display().to_string();
    Ok(manifest)
}

pub fn export_runlog_to_rerun(
    run_log_uri: impl AsRef<Path>,
    output_uri: impl AsRef<Path>,
) -> Result<Value> {
    Ok(export_runlog_to_rerun_inner(run_log_uri, output_uri)?.response)
}

struct ExportedRerun {
    output_path: PathBuf,
    sha256: String,
    response: Value,
}

fn export_runlog_to_rerun_inner(
    run_log_uri: impl AsRef<Path>,
    output_uri: impl AsRef<Path>,
) -> Result<ExportedRerun> {
    let run_log_uri = run_log_uri.as_ref();
    let output_uri = output_uri.as_ref();
    if paths_refer_to_same_file(run_log_uri, output_uri) {
        bail!("export.rerun output_uri must differ from run_log_uri");
    }
    // Constrain this file-producing method to its declared artifact type. The
    // later staged no-clobber install independently preserves every existing
    // destination.
    if output_uri.extension().and_then(|ext| ext.to_str()) != Some("rrd") {
        bail!(
            "export.rerun output_uri must end in .rrd, got {}",
            output_uri.display()
        );
    }
    let parent = output_uri
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.is_dir() {
        bail!(
            "export.rerun output parent must already exist: {}",
            parent.display()
        );
    }
    let (events, run_log_bytes) = snapshot_runlog(run_log_uri)?;
    let manifest = manifest_for_snapshot(run_log_uri, &events, &run_log_bytes)?;
    let summary = pid_runlog::summarize_events(&events)?;
    if summary.validation_errors > 0 {
        bail!(
            "run log failed validation ({} error(s)); refusing export",
            summary.validation_errors
        );
    }
    let rec = pid_rerun::init_recording("prisoma_bridge_export", false)?;
    pid_rerun::RunLogRerunLogger::new(&rec)
        .without_external_artifact_loading()
        .log_events_with_manifest(&events, Some(&manifest))?;
    let recording = pid_rerun::finalize_recording_bytes(&rec)?;
    let sha256 = install_new_artifact(output_uri, &recording)?;
    let output_path = output_uri.to_path_buf();
    let output_uri = output_path.display().to_string();
    let response = json!({
        "output_uri": output_uri,
        "sha256": sha256,
        "trace_hash": summary.trace_hash,
        "trace_hash_v2": summary.trace_hash_v2,
        "trace_hash_revision": "replay_trace_v2",
        "events": summary.event_count,
        "valid": summary.validation_errors == 0,
        "validation_errors": summary.validation_errors,
        "validation_warnings": summary.validation_warnings,
        "config_hash": summary.config_hash,
    });
    Ok(ExportedRerun {
        output_path,
        sha256,
        response,
    })
}

fn default_rerun_output_path(run_log_uri: impl AsRef<Path>) -> PathBuf {
    let mut path = run_log_uri.as_ref().to_path_buf();
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
            let message = format!("JSON-RPC line {idx} exceeds {MAX_RPC_LINE_BYTES} bytes");
            session.record_rejected_rpc(&format!("line-{idx}"), &message)?;
            bail!(message);
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
        )?;
        if let Some(response) = response {
            serde_json::to_writer(&mut *output, &response)
                .context("failed to write RPC response")?;
            output
                .write_all(b"\n")
                .context("failed to write RPC response newline")?;
            // Flush per response: an interactive client (TCP without half-close,
            // a REPL over stdio) deadlocks waiting for a reply that is sitting in
            // this side's BufWriter.
            output.flush().context("failed to flush RPC response")?;
        }
        handled += 1;
        if session.stop_requested() || session.run_ended() {
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
) -> Result<Option<BridgeRpcResponse>>
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
) -> Result<Option<BridgeRpcResponse>>
where
    L: Write,
{
    let value = match serde_json::from_str::<Value>(text) {
        Ok(value) => value,
        Err(err) => {
            let message =
                format!("invalid JSON-RPC syntax at {context_name} {request_index}: {err}");
            session.record_rejected_rpc(failure_id, &message)?;
            // Unparseable JSON: the request id is unknowable; JSON-RPC 2.0
            // says respond with id null (the message keeps the line/frame
            // context for correlation).
            return Ok(Some(BridgeRpcResponse::failure(
                Value::Null,
                -32700,
                message,
            )));
        }
    };
    match serde_json::from_value::<BridgeRpcRequest>(value) {
        Ok(rpc) => {
            let id = match rpc.validated_id() {
                Ok(id) => id.cloned(),
                Err(err) => {
                    let message = format!(
                        "invalid JSON-RPC request at {context_name} {request_index}: {err}"
                    );
                    session.record_rejected_rpc(failure_id, &message)?;
                    // The id is structurally invalid, so it cannot be echoed;
                    // JSON-RPC 2.0 says respond with id null.
                    return Ok(Some(BridgeRpcResponse::failure(
                        Value::Null,
                        -32600,
                        message,
                    )));
                }
            };
            // Unique-by-construction run-log id: clients may reuse ids and
            // `1` vs `"1"` collide under the bare rendering, and duplicate
            // request ids hard-fail canonical run-log validation. Preserve
            // absent ids separately from explicit null so the log also says
            // whether a wire response was suppressed.
            let request_id = match id.as_ref() {
                Some(id) => rpc_id_to_unique_request_id(id, request_index),
                None => rpc_notification_to_unique_request_id(request_index),
            };
            if let Err(err) = rpc.validated_params() {
                let message =
                    format!("invalid JSON-RPC params at {context_name} {request_index}: {err}");
                session.record_rejected_rpc(&request_id, &message)?;
                return Ok(id.map(|id| BridgeRpcResponse::failure(id, -32602, message)));
            }
            let method = match rpc.validated_method() {
                Ok(method) => method,
                Err(err) => {
                    // Unknown/unsupported method: return -32601 AND leave a
                    // trace — method probing is exactly the traffic a control-
                    // plane audit log must capture.
                    let message = err.to_string();
                    session.record_rejected_rpc(&request_id, &message)?;
                    return Ok(id.map(|id| BridgeRpcResponse::failure(id, -32601, message)));
                }
            };
            if let Err(err) = rpc.validated_params_for_method(&method) {
                let message =
                    format!("invalid JSON-RPC params at {context_name} {request_index}: {err}");
                session.record_rejected_rpc(&request_id, &message)?;
                return Ok(id.map(|id| BridgeRpcResponse::failure(id, -32602, message)));
            }
            let request = BridgeRequest {
                request_id,
                step: Some(session.step()),
                timestamp_ns: session.timestamp_ns(),
                actor,
                method,
                payload: rpc.params.unwrap_or(Value::Null),
            };
            let response = session.dispatch(&request)?;
            Ok(id.map(|id| BridgeRpcResponse::from_bridge_response_with_id(&response, id)))
        }
        Err(err) => {
            let message =
                format!("invalid JSON-RPC request at {context_name} {request_index}: {err}");
            session.record_rejected_rpc(failure_id, &message)?;
            // Syntactically valid JSON that is not one supported single
            // Request object (including a batch) is Invalid Request, not a
            // Parse error. Its request id is not safely recoverable.
            Ok(Some(BridgeRpcResponse::failure(
                Value::Null,
                -32600,
                message,
            )))
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
                    .filter(|(previous_step, _)| previous_step.checked_add(1) == Some(*step))
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
    verify_bridge_effect_lineage(events, &mut report);
    report
}

#[derive(Debug, Clone)]
struct PendingBridgeEffect {
    request_id: String,
    actor: Actor,
    method: BridgeMethod,
    payload_hash: String,
    payload: Value,
    effects: usize,
}

fn verify_bridge_effect_lineage(events: &[RunLogEvent], report: &mut SimReplayReport) {
    if !events.iter().any(|event| {
        matches!(
            event,
            RunLogEvent::BridgeRequest { .. } | RunLogEvent::BridgeResponse { .. }
        )
    }) {
        return;
    }

    let mut pending: Option<PendingBridgeEffect> = None;
    for (event_index, event) in events.iter().enumerate() {
        match event {
            RunLogEvent::BridgeRequest {
                request_id,
                actor,
                method,
                payload_hash,
                payload,
                ..
            } => {
                if let Some(previous) = pending.take() {
                    report.issues.push(format!(
                        "bridge request {} at event {event_index} arrived before pending request {} received a response",
                        request_id, previous.request_id
                    ));
                }
                let method = match BridgeMethod::from_str(method) {
                    Ok(method) => method,
                    Err(error) => {
                        report.issues.push(format!(
                            "bridge request {request_id} at event {event_index} has unsupported method: {error}"
                        ));
                        continue;
                    }
                };
                pending = Some(PendingBridgeEffect {
                    request_id: request_id.clone(),
                    actor: actor.clone(),
                    method,
                    payload_hash: payload_hash.clone(),
                    payload: payload.clone(),
                    effects: 0,
                });
            }
            RunLogEvent::ActionApplied {
                actor,
                action_type,
                payload_hash,
                payload,
                ..
            } => {
                let Some(request) = pending.as_mut() else {
                    report.issues.push(format!(
                        "action {action_type} at event {event_index} has no pending bridge request"
                    ));
                    continue;
                };
                request.effects += 1;
                let expected_action = match request.method {
                    BridgeMethod::SimStep => Some("sim.step"),
                    BridgeMethod::SimReset => Some("sim.reset"),
                    BridgeMethod::SceneSetObject => Some("scene.set_object"),
                    _ => None,
                };
                if expected_action != Some(action_type.as_str()) {
                    report.issues.push(format!(
                        "bridge request {} ({}) produced unexpected action {action_type} at event {event_index}",
                        request.request_id,
                        request.method.as_str()
                    ));
                }
                if actor != &request.actor
                    || payload_hash != &request.payload_hash
                    || payload != &request.payload
                {
                    report.issues.push(format!(
                        "action at event {event_index} does not exactly bind actor and payload to bridge request {}",
                        request.request_id
                    ));
                }
            }
            RunLogEvent::InterventionApplied {
                actor,
                intervention_type,
                payload_hash,
                payload,
                ..
            } => {
                let Some(request) = pending.as_mut() else {
                    report.issues.push(format!(
                        "intervention {intervention_type} at event {event_index} has no pending bridge request"
                    ));
                    continue;
                };
                request.effects += 1;
                if request.method != BridgeMethod::InterventionApply {
                    report.issues.push(format!(
                        "bridge request {} ({}) produced unexpected intervention {intervention_type} at event {event_index}",
                        request.request_id,
                        request.method.as_str()
                    ));
                    continue;
                }
                let expected_type = request
                    .payload
                    .get("intervention_type")
                    .and_then(Value::as_str);
                let expected_payload = request.payload.get("payload");
                let expected_hash = expected_payload.map(canonical_json_hash_v2).transpose();
                if actor != &request.actor
                    || expected_type != Some(intervention_type.as_str())
                    || expected_payload != Some(payload)
                    || expected_hash.as_ref().ok().and_then(|hash| hash.as_ref())
                        != Some(payload_hash)
                {
                    report.issues.push(format!(
                        "intervention at event {event_index} does not exactly bind actor, type, and payload to bridge request {}",
                        request.request_id
                    ));
                }
                if let Err(error) = expected_hash {
                    report.issues.push(format!(
                        "bridge request {} intervention payload could not be hashed: {error}",
                        request.request_id
                    ));
                }
            }
            RunLogEvent::BridgeResponse { request_id, ok, .. } => {
                let Some(request) = pending.take() else {
                    report.issues.push(format!(
                        "bridge response {request_id} at event {event_index} has no pending request"
                    ));
                    continue;
                };
                if request.request_id != *request_id {
                    report.issues.push(format!(
                        "bridge response {request_id} at event {event_index} does not match pending request {}",
                        request.request_id
                    ));
                }
                let expected_effects = usize::from(
                    *ok && matches!(
                        request.method,
                        BridgeMethod::SimStep
                            | BridgeMethod::SimReset
                            | BridgeMethod::SceneSetObject
                            | BridgeMethod::InterventionApply
                    ),
                );
                if request.effects != expected_effects {
                    report.issues.push(format!(
                        "bridge request {} ({}) recorded {} effect event(s), expected {expected_effects} before its ok={ok} response",
                        request.request_id,
                        request.method.as_str(),
                        request.effects
                    ));
                }
            }
            _ => {}
        }
    }
    if let Some(request) = pending {
        report.issues.push(format!(
            "bridge request {} ({}) has no response",
            request.request_id,
            request.method.as_str()
        ));
    }
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
            if let Err(error) = validate_bridge_payload_keys(payload, "sim.step", &["dt"], false) {
                report
                    .issues
                    .push(format!("failed to replay sim.step: {error}"));
                return;
            }
            let Some(dt) = payload.get("dt").and_then(Value::as_f64) else {
                report
                    .issues
                    .push("failed to replay sim.step: dt must be a number".to_string());
                return;
            };
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
            if let Err(error) = validate_bridge_payload_keys(
                payload,
                "scene.set_object",
                &["object_id", "pose", "velocity"],
                false,
            ) {
                report
                    .issues
                    .push(format!("failed to replay scene.set_object: {error}"));
                return;
            }
            match serde_json::from_value::<SimObject>(payload.clone()) {
                Ok(object) => {
                    if let Err(error) = validate_object_id(&object.object_id)
                        .and_then(|()| validate_pose_finite(&object.pose))
                        .and_then(|()| validate_vec3_finite(object.velocity, "object velocity"))
                    {
                        report
                            .issues
                            .push(format!("failed to replay scene.set_object: {error}"));
                        return;
                    }
                    sim.get_or_insert_with(DeterministicObjectSim::new)
                        .upsert_object(object);
                    report.checked_actions += 1;
                }
                Err(err) => report
                    .issues
                    .push(format!("failed to replay scene.set_object: {err}")),
            }
        }
        _ => report
            .issues
            .push(format!("unsupported replay action type {action_type}")),
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
        canonical_json_hash_v2, read_events, replay_events, validate_events, ActorType,
        RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION,
    };
    use serde_json::json;
    use std::io::{self, Cursor};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct LineGateControl {
        bytes: Arc<Mutex<Vec<u8>>>,
        completed_lines: Arc<AtomicUsize>,
        allowed_lines: Arc<AtomicUsize>,
    }

    struct LineGateWriter {
        control: LineGateControl,
    }

    impl LineGateWriter {
        fn new() -> (Self, LineGateControl) {
            let control = LineGateControl {
                bytes: Arc::new(Mutex::new(Vec::new())),
                completed_lines: Arc::new(AtomicUsize::new(0)),
                allowed_lines: Arc::new(AtomicUsize::new(usize::MAX)),
            };
            (
                Self {
                    control: control.clone(),
                },
                control,
            )
        }
    }

    impl Write for LineGateWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            let limit = self.control.allowed_lines.load(Ordering::SeqCst);
            let mut lines = self.control.completed_lines.load(Ordering::SeqCst);
            if lines >= limit {
                return Err(io::Error::other("injected canonical run-log write failure"));
            }
            let mut accepted = 0usize;
            for byte in buffer {
                if lines >= limit {
                    break;
                }
                accepted += 1;
                if *byte == b'\n' {
                    lines += 1;
                }
            }
            if accepted == 0 {
                return Err(io::Error::other("injected canonical run-log write failure"));
            }
            self.control
                .bytes
                .lock()
                .expect("line-gate buffer lock")
                .extend_from_slice(&buffer[..accepted]);
            self.control.completed_lines.store(lines, Ordering::SeqCst);
            Ok(accepted)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct FlushGateControl {
        bytes: Arc<Mutex<Vec<u8>>>,
        fail_flush: Arc<std::sync::atomic::AtomicBool>,
    }

    struct FlushGateWriter {
        control: FlushGateControl,
    }

    impl FlushGateWriter {
        fn new() -> (Self, FlushGateControl) {
            let control = FlushGateControl {
                bytes: Arc::new(Mutex::new(Vec::new())),
                fail_flush: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            };
            (
                Self {
                    control: control.clone(),
                },
                control,
            )
        }
    }

    impl Write for FlushGateWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.control
                .bytes
                .lock()
                .expect("flush-gate buffer lock")
                .extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            if self.control.fail_flush.load(Ordering::SeqCst) {
                Err(io::Error::other("injected canonical run-log flush failure"))
            } else {
                Ok(())
            }
        }
    }

    #[derive(Clone)]
    struct NthFlushControl {
        bytes: Arc<Mutex<Vec<u8>>>,
        flushes: Arc<AtomicUsize>,
        fail_on_flush: Arc<AtomicUsize>,
    }

    struct NthFlushWriter {
        control: NthFlushControl,
    }

    impl NthFlushWriter {
        fn new(fail_on_flush: usize) -> (Self, NthFlushControl) {
            let control = NthFlushControl {
                bytes: Arc::new(Mutex::new(Vec::new())),
                flushes: Arc::new(AtomicUsize::new(0)),
                fail_on_flush: Arc::new(AtomicUsize::new(fail_on_flush)),
            };
            (
                Self {
                    control: control.clone(),
                },
                control,
            )
        }
    }

    impl Write for NthFlushWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.control
                .bytes
                .lock()
                .expect("nth-flush buffer lock")
                .extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            let flush = self.control.flushes.fetch_add(1, Ordering::SeqCst) + 1;
            if flush == self.control.fail_on_flush.load(Ordering::SeqCst) {
                Err(io::Error::other(
                    "injected final canonical run-log flush failure",
                ))
            } else {
                Ok(())
            }
        }
    }

    #[derive(Clone)]
    struct ByteGateControl {
        bytes: Arc<Mutex<Vec<u8>>>,
        allowed_bytes: Arc<AtomicUsize>,
    }

    struct ByteGateWriter {
        control: ByteGateControl,
    }

    impl ByteGateWriter {
        fn new() -> (Self, ByteGateControl) {
            let control = ByteGateControl {
                bytes: Arc::new(Mutex::new(Vec::new())),
                allowed_bytes: Arc::new(AtomicUsize::new(usize::MAX)),
            };
            (
                Self {
                    control: control.clone(),
                },
                control,
            )
        }
    }

    impl Write for ByteGateWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            let allowed = self.control.allowed_bytes.load(Ordering::SeqCst);
            let mut bytes = self.control.bytes.lock().expect("byte-gate buffer lock");
            let remaining = allowed.saturating_sub(bytes.len());
            if remaining == 0 {
                return Err(io::Error::other(
                    "injected partial canonical run-log write failure",
                ));
            }
            let accepted = remaining.min(buffer.len());
            bytes.extend_from_slice(&buffer[..accepted]);
            Ok(accepted)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn temp_path(prefix: &str, extension: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("{prefix}-{stamp}.{extension}"))
    }

    fn set_temp_artifact_root<W: std::io::Write>(session: &mut SimBridgeSession<W>, path: &Path) {
        session.set_artifact_root(path.parent().unwrap()).unwrap();
    }

    fn append_run_prefix<W: std::io::Write>(
        writer: &mut RunLogWriter<W>,
        run_id: &str,
        test_name: &str,
    ) {
        let config = json!({ "test": test_name });
        let config_hash = canonical_json_hash_v2(&config).unwrap();
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
    fn bridge_provenance_write_failure_keeps_staged_state_until_effect_evidence_completes() {
        let mut baseline_writer = RunLogWriter::new(Vec::new());
        let sim = demo_sim();
        baseline_writer.append(&sim.snapshot_event()).unwrap();
        let mut baseline = SimBridgeSession::new(baseline_writer, sim.clone());
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "poison-test".to_string(),
            session_id: None,
        };
        let step = bridge_request(
            "poisoned-step",
            BridgeMethod::SimStep,
            actor.clone(),
            Some(0),
            0,
            json!({ "dt": 0.1 }),
        );
        baseline.dispatch(&step).unwrap();
        let baseline_events = read_events(Cursor::new(baseline.into_inner())).unwrap();
        let response_index = baseline_events
            .iter()
            .position(|event| matches!(event, RunLogEvent::BridgeResponse { .. }))
            .expect("baseline response");
        let initial_lines = 1usize;
        let pre_response_lines = response_index - initial_lines;

        // Fail before every append boundary from BridgeRequest through the
        // final BridgeResponse. State stays staged until every effect event is
        // accepted; once they are, a response-write failure must not roll the
        // mutation back underneath that provenance.
        for allowed_after_initial in 0..=pre_response_lines {
            let (sink, control) = LineGateWriter::new();
            let mut writer = RunLogWriter::new(sink);
            writer.append(&sim.snapshot_event()).unwrap();
            control
                .allowed_lines
                .store(initial_lines + allowed_after_initial, Ordering::SeqCst);
            let mut session = SimBridgeSession::new(writer, sim.clone());

            let error = session.dispatch(&step).unwrap_err();

            assert!(format!("{error:#}").contains("injected canonical run-log"));
            assert!(session.poisoned());
            let expected_step = usize::from(allowed_after_initial == pre_response_lines) as u64;
            assert_eq!(
                session.step(),
                expected_step,
                "unexpected commit at append boundary {allowed_after_initial}"
            );
        }
    }

    #[test]
    fn bridge_provenance_failure_remains_unsealed_and_explicit() {
        let (sink, control) = LineGateWriter::new();
        let mut writer = RunLogWriter::new(sink);
        append_run_prefix(&mut writer, "poisoned-run", "poisoned-run");
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let initial_lines = control.completed_lines.load(Ordering::SeqCst);
        // Permit the request event, then fail before effect evidence. The
        // mutation remains staged and the session is poisoned.
        control
            .allowed_lines
            .store(initial_lines + 1, Ordering::SeqCst);
        let mut session = SimBridgeSession::with_run_id(writer, sim, "poisoned-run");
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "poison-test".to_string(),
            session_id: None,
        };
        let step = bridge_request(
            "poisoned-step",
            BridgeMethod::SimStep,
            actor.clone(),
            Some(0),
            0,
            json!({ "dt": 0.1 }),
        );

        let error = session.dispatch(&step).unwrap_err();

        assert!(format!("{error:#}").contains("injected canonical run-log"));
        assert!(session.poisoned());
        assert_eq!(
            session.step(),
            0,
            "unlogged simulator mutation must roll back"
        );
        let status = bridge_request(
            "poisoned-status",
            BridgeMethod::SimStatus,
            actor,
            Some(0),
            0,
            json!({}),
        );
        assert!(session
            .dispatch(&status)
            .unwrap_err()
            .to_string()
            .contains("poisoned"));

        control.allowed_lines.store(usize::MAX, Ordering::SeqCst);
        let seal_error = session
            .finish_run(RunStatus::Failed, Some("write failure".to_string()))
            .unwrap_err();
        assert!(seal_error
            .to_string()
            .contains("cannot be sealed after an earlier provenance-write failure"));
        assert!(!session.run_ended());

        let bytes = control.bytes.lock().unwrap().clone();
        let events = read_events(Cursor::new(bytes)).unwrap();
        let report = validate_events(&events).unwrap();
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.message.contains("bridge request without response")),
            "a storage failure after BridgeRequest must remain explicit: {:?}",
            report.issues
        );
        assert!(
            report.issues.iter().any(|issue| issue
                .message
                .contains("expected exactly one run_ended event, got 0")),
            "the indeterminate terminal state must remain explicit: {:?}",
            report.issues
        );
        let state = replay_events(&events).unwrap();
        assert_eq!(state.status, None);
        assert!(state.actions.is_empty());
    }

    #[test]
    fn bridge_terminal_write_failure_cannot_be_retried_ambiguously() {
        let (sink, control) = LineGateWriter::new();
        let mut writer = RunLogWriter::new(sink);
        append_run_prefix(&mut writer, "terminal-failure", "terminal-failure");
        let initial_lines = control.completed_lines.load(Ordering::SeqCst);
        control.allowed_lines.store(initial_lines, Ordering::SeqCst);
        let mut session = SimBridgeSession::with_run_id(writer, demo_sim(), "terminal-failure");

        assert!(session
            .finish_run(RunStatus::Failed, Some("write failure".to_string()))
            .is_err());
        assert!(!session.run_ended());
        control.allowed_lines.store(usize::MAX, Ordering::SeqCst);
        let retry = session
            .finish_run(RunStatus::Failed, Some("retry".to_string()))
            .unwrap_err();
        assert!(retry.to_string().contains("cannot be retried safely"));
    }

    #[test]
    fn terminal_newline_failure_can_leave_a_parseable_indeterminate_status() {
        let (sink, control) = ByteGateWriter::new();
        let mut writer = RunLogWriter::new(sink);
        append_run_prefix(&mut writer, "terminal-newline", "terminal-newline");
        let initial_bytes = control.bytes.lock().unwrap().len();
        let terminal = RunLogEvent::RunEnded {
            run_id: "terminal-newline".to_string(),
            timestamp_ns: 0,
            status: RunStatus::Succeeded,
            message: None,
        };
        let mut encoded_writer = RunLogWriter::new(Vec::new());
        encoded_writer.append(&terminal).unwrap();
        let terminal_line = encoded_writer.into_inner();
        assert_eq!(terminal_line.last(), Some(&b'\n'));
        control
            .allowed_bytes
            .store(initial_bytes + terminal_line.len() - 1, Ordering::SeqCst);
        let mut session = SimBridgeSession::with_run_id(writer, demo_sim(), "terminal-newline");

        assert!(session.finish_run(RunStatus::Succeeded, None).is_err());

        let bytes = control.bytes.lock().unwrap().clone();
        let events = read_events(Cursor::new(bytes)).unwrap();
        assert!(matches!(
            events.last(),
            Some(RunLogEvent::RunEnded {
                status: RunStatus::Succeeded,
                ..
            })
        ));
        assert!(session.poisoned());
        assert!(!session.run_ended());
        assert!(session
            .finish_run(RunStatus::Failed, Some("retry".to_string()))
            .unwrap_err()
            .to_string()
            .contains("cannot be retried safely"));
    }

    #[test]
    fn terminal_flush_failure_can_leave_a_complete_indeterminate_status() {
        let (sink, control) = FlushGateWriter::new();
        let mut writer = RunLogWriter::new(sink);
        append_run_prefix(&mut writer, "terminal-flush", "terminal-flush");
        let mut session = SimBridgeSession::with_run_id(writer, demo_sim(), "terminal-flush");
        control.fail_flush.store(true, Ordering::SeqCst);

        assert!(session.finish_run(RunStatus::Succeeded, None).is_err());

        let bytes = control.bytes.lock().unwrap().clone();
        let events = read_events(Cursor::new(bytes)).unwrap();
        assert!(matches!(
            events.last(),
            Some(RunLogEvent::RunEnded {
                status: RunStatus::Succeeded,
                ..
            })
        ));
        assert!(session.poisoned());
        assert!(!session.run_ended());
    }

    #[test]
    fn poisoned_bridge_cannot_claim_a_terminal_status() {
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, "poisoned-status", "poisoned-status");
        let mut session = SimBridgeSession::with_run_id(writer, demo_sim(), "poisoned-status");
        session.poisoned = true;

        let error = session
            .finish_run(RunStatus::Succeeded, Some("incorrect success".to_string()))
            .unwrap_err();

        assert!(error
            .to_string()
            .contains("cannot be sealed after an earlier provenance-write failure"));
        assert!(!session.terminal_write_attempted);
        assert!(session
            .finish_run(RunStatus::Failed, Some("provenance failure".to_string()))
            .unwrap_err()
            .to_string()
            .contains("cannot be sealed after an earlier provenance-write failure"));
    }

    #[test]
    fn partial_provenance_write_poisoning_does_not_claim_a_readable_run_log() {
        let (sink, control) = ByteGateWriter::new();
        let mut writer = RunLogWriter::new(sink);
        append_run_prefix(&mut writer, "partial-write", "partial-write");
        let initial_bytes = control.bytes.lock().unwrap().len();
        control
            .allowed_bytes
            .store(initial_bytes + 17, Ordering::SeqCst);
        let mut session = SimBridgeSession::with_run_id(writer, demo_sim(), "partial-write");
        let request = bridge_request(
            "partial-status",
            BridgeMethod::SimStatus,
            Actor {
                actor_type: ActorType::Script,
                actor_id: "partial-write".to_string(),
                session_id: None,
            },
            Some(0),
            0,
            json!({}),
        );

        let error = session.dispatch(&request).unwrap_err();

        assert!(format!("{error:#}").contains("partial canonical run-log"));
        assert!(session.poisoned());
        let bytes = control.bytes.lock().unwrap().clone();
        assert!(
            read_events(Cursor::new(bytes)).is_err(),
            "a mid-line storage failure is explicitly outside the valid-log guarantee"
        );
    }

    #[test]
    fn rpc_output_is_suppressed_when_provenance_flush_fails() {
        let (sink, control) = FlushGateWriter::new();
        let mut writer = RunLogWriter::new(sink);
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::new(writer, sim);
        control.fail_flush.store(true, Ordering::SeqCst);
        let input = concat!(
            r#"{"jsonrpc":"2.0","id":"step","method":"sim.step","params":{"dt":0.1}}"#,
            "\n"
        );
        let mut output = Vec::new();

        let error = dispatch_rpc_lines(
            Cursor::new(input),
            &mut output,
            &mut session,
            Actor {
                actor_type: ActorType::Script,
                actor_id: "flush-failure".to_string(),
                session_id: None,
            },
        )
        .unwrap_err();

        assert!(format!("{error:#}").contains("flush failure"));
        assert!(
            output.is_empty(),
            "no wire success may precede provenance flush"
        );
        assert!(session.poisoned());
        assert_eq!(
            session.step(),
            1,
            "accepted effect evidence prevents ambiguous rollback"
        );
        let bytes = control.bytes.lock().unwrap().clone();
        let events = read_events(Cursor::new(bytes)).unwrap();
        assert!(events
            .iter()
            .any(|event| matches!(event, RunLogEvent::BridgeResponse { ok: true, .. })));
    }

    #[test]
    fn bridge_export_retains_artifact_after_provenance_write_is_attempted() {
        let source = temp_path("pid-sim-export-rollback-source", "jsonl");
        write_minimal_run_log(&source, "export-rollback-source");
        for (allowed_request_lines, boundary) in [
            (1usize, "artifact_logged write"),
            (2usize, "bridge_response write"),
        ] {
            let output = temp_path(
                &format!("pid-sim-export-rollback-{allowed_request_lines}"),
                "rrd",
            );
            let (sink, control) = LineGateWriter::new();
            let mut writer = RunLogWriter::new(sink);
            let run_id = format!("export-rollback-run-{allowed_request_lines}");
            append_run_prefix(&mut writer, &run_id, &run_id);
            let initial_lines = control.completed_lines.load(Ordering::SeqCst);
            control
                .allowed_lines
                .store(initial_lines + allowed_request_lines, Ordering::SeqCst);
            let mut session = SimBridgeSession::with_run_id(writer, demo_sim(), &run_id);
            set_temp_artifact_root(&mut session, &source);
            let request = bridge_request(
                format!("export-rollback-{allowed_request_lines}"),
                BridgeMethod::ExportRerun,
                Actor {
                    actor_type: ActorType::Script,
                    actor_id: "export-rollback-test".to_string(),
                    session_id: None,
                },
                Some(0),
                0,
                json!({
                    "run_log_uri": source.display().to_string(),
                    "output_uri": output.display().to_string(),
                }),
            );

            assert!(session.dispatch(&request).is_err(), "{boundary}");
            assert!(session.poisoned(), "{boundary}");
            assert!(
                output.exists(),
                "artifact must be retained after attempted {boundary}"
            );
            assert_eq!(pid_runlog::sha256_file(&output).unwrap().len(), 64);
            let _ = std::fs::remove_file(output);
        }
        let _ = std::fs::remove_file(source);
    }

    #[test]
    fn bridge_export_creates_no_artifact_when_preexport_provenance_flush_fails() {
        let source = temp_path("pid-sim-export-preflush-source", "jsonl");
        let output = temp_path("pid-sim-export-preflush-output", "rrd");
        write_minimal_run_log(&source, "export-preflush-source");
        let (sink, control) = FlushGateWriter::new();
        let writer = RunLogWriter::new(sink);
        let mut session = SimBridgeSession::new(writer, demo_sim());
        set_temp_artifact_root(&mut session, &source);
        control.fail_flush.store(true, Ordering::SeqCst);
        let request = bridge_request(
            "export-preflush",
            BridgeMethod::ExportRerun,
            Actor {
                actor_type: ActorType::Script,
                actor_id: "export-preflush-test".to_string(),
                session_id: None,
            },
            Some(0),
            0,
            json!({
                "run_log_uri": source.display().to_string(),
                "output_uri": output.display().to_string(),
            }),
        );

        let error = session.dispatch(&request).unwrap_err();

        assert!(format!("{error:#}").contains("flush failure"));
        assert!(session.poisoned());
        assert!(
            !output.exists(),
            "export side effect must follow the pre-export provenance flush"
        );
        let _ = std::fs::remove_file(source);
    }

    #[test]
    fn bridge_export_final_flush_failure_retains_artifact_and_suppresses_response() {
        let source = temp_path("pid-sim-export-final-flush-source", "jsonl");
        let output = temp_path("pid-sim-export-final-flush-output", "rrd");
        write_minimal_run_log(&source, "export-final-flush-source");
        let (sink, control) = NthFlushWriter::new(2);
        let writer = RunLogWriter::new(sink);
        let mut session = SimBridgeSession::new(writer, demo_sim());
        set_temp_artifact_root(&mut session, &source);
        let request = bridge_request(
            "export-final-flush",
            BridgeMethod::ExportRerun,
            Actor {
                actor_type: ActorType::Script,
                actor_id: "export-final-flush-test".to_string(),
                session_id: None,
            },
            Some(0),
            0,
            json!({
                "run_log_uri": source.display().to_string(),
                "output_uri": output.display().to_string(),
            }),
        );

        let error = session.dispatch(&request).unwrap_err();

        assert!(format!("{error:#}").contains("final canonical run-log flush failure"));
        assert_eq!(control.flushes.load(Ordering::SeqCst), 2);
        assert!(session.poisoned());
        assert!(output.exists());
        let events = read_events(Cursor::new(control.bytes.lock().unwrap().clone())).unwrap();
        let artifact_index = events
            .iter()
            .position(|event| matches!(event, RunLogEvent::ArtifactLogged { .. }))
            .unwrap();
        let response_index = events
            .iter()
            .position(|event| matches!(event, RunLogEvent::BridgeResponse { ok: true, .. }))
            .unwrap();
        assert!(artifact_index < response_index);
        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(output);
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
    fn fixed_step_rejects_unrepresentable_or_overflowing_time_without_mutation() {
        for dt in [1e-12, 1e300] {
            let mut sim = demo_sim();
            let before = sim.clone();
            assert!(sim.step_fixed(dt).is_err(), "dt={dt}");
            assert_eq!(sim, before, "rejected dt must be transactional: {dt}");
        }

        let mut timestamp_overflow = demo_sim();
        timestamp_overflow.timestamp_ns = u64::MAX - 5;
        let before = timestamp_overflow.clone();
        assert!(timestamp_overflow.step_fixed(1e-8).is_err());
        assert_eq!(timestamp_overflow, before);

        let mut step_overflow = demo_sim();
        step_overflow.step = u64::MAX;
        let before = step_overflow.clone();
        assert!(step_overflow.step_fixed(0.1).is_err());
        assert_eq!(step_overflow, before);
    }

    #[test]
    fn fixed_step_rejects_nonfinite_effects_without_partial_object_updates() {
        let mut sim = demo_sim();
        sim.upsert_object(SimObject {
            object_id: "overflowing".to_string(),
            pose: Pose {
                position: [0.0, 0.0, 0.0],
                orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
            },
            velocity: [f64::MAX, 0.0, 0.0],
        });
        let before = sim.clone();

        let error = sim.step_fixed(2.0).unwrap_err();

        assert!(error.to_string().contains("step displacement"));
        assert_eq!(sim, before);
    }

    #[test]
    fn translate_object_rejects_nonfinite_result_without_mutation() {
        let mut sim = demo_sim();
        sim.objects.get_mut("red_cube").unwrap().pose.position = [f64::MAX, 0.0, 0.0];
        let before = sim.clone();

        let error = sim
            .apply_intervention(
                "translate_object",
                &json!({ "object_id": "red_cube", "delta": [f64::MAX, 0.0, 0.0] }),
            )
            .unwrap_err();

        assert!(error.to_string().contains("translated object position"));
        assert_eq!(sim, before);
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
        set_temp_artifact_root(&mut session, &path);
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
        assert_eq!(result["trace_hash_v2"].as_str().unwrap().len(), 64);
        assert_eq!(result["trace_hash_revision"], "replay_trace_v2");
        assert_eq!(
            result["trace_hash_v2"],
            pid_runlog::summarize_path(&path).unwrap().trace_hash_v2
        );

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.actions.len(), 0);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bridge_file_methods_fail_closed_without_artifact_root() {
        let path = temp_path("pid-sim-log-replay-no-root", "jsonl");
        write_minimal_run_log(&path, "replay-no-root");
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-replay-no-root-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::with_safe_mode(writer, demo_sim(), true);
        let request = bridge_request(
            "req-log-replay-no-root",
            BridgeMethod::LogReplay,
            actor,
            Some(0),
            0,
            json!({ "run_log_uri": path.display().to_string() }),
        );

        let response = session.dispatch(&request).unwrap();

        assert!(
            response
                .message
                .as_deref()
                .unwrap_or("")
                .contains("disabled until an artifact root is configured"),
            "{:?}",
            response.message
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bridge_log_replay_rejects_out_of_root_path() {
        let root = temp_path("pid-sim-artifact-root", "dir");
        std::fs::create_dir(&root).unwrap();
        let outside = temp_path("pid-sim-artifact-outside", "jsonl");
        write_minimal_run_log(&outside, "outside-root");
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-replay-outside-root-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::with_safe_mode(writer, demo_sim(), true);
        session.set_artifact_root(&root).unwrap();
        let request = bridge_request(
            "req-log-replay-outside-root",
            BridgeMethod::LogReplay,
            actor,
            Some(0),
            0,
            json!({ "run_log_uri": outside.display().to_string() }),
        );

        let response = session.dispatch(&request).unwrap();

        assert!(
            response
                .message
                .as_deref()
                .unwrap_or("")
                .contains("escapes configured root"),
            "{:?}",
            response.message
        );
        let _ = std::fs::remove_file(outside);
        let _ = std::fs::remove_dir(root);
    }

    #[test]
    fn bridge_log_replay_rejects_parent_traversal() {
        let root = temp_path("pid-sim-artifact-traversal-root", "dir");
        std::fs::create_dir(&root).unwrap();
        let source = root.join("source.jsonl");
        write_minimal_run_log(&source, "traversal-source");
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-replay-traversal-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::with_safe_mode(writer, demo_sim(), true);
        session.set_artifact_root(&root).unwrap();
        let request = bridge_request(
            "req-log-replay-traversal",
            BridgeMethod::LogReplay,
            actor,
            Some(0),
            0,
            json!({ "run_log_uri": "nested/../source.jsonl" }),
        );

        let response = session.dispatch(&request).unwrap();

        assert!(
            response
                .message
                .as_deref()
                .unwrap_or("")
                .contains("parent traversal"),
            "{:?}",
            response.message
        );
        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_dir(root);
    }

    #[cfg(unix)]
    #[test]
    fn bridge_log_replay_rejects_symlink_source() {
        let root = temp_path("pid-sim-artifact-symlink-root", "dir");
        std::fs::create_dir(&root).unwrap();
        let outside = temp_path("pid-sim-artifact-symlink-source", "jsonl");
        write_minimal_run_log(&outside, "symlink-source");
        let link = root.join("linked.jsonl");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-replay-symlink-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::with_safe_mode(writer, demo_sim(), true);
        session.set_artifact_root(&root).unwrap();
        let request = bridge_request(
            "req-log-replay-symlink",
            BridgeMethod::LogReplay,
            actor,
            Some(0),
            0,
            json!({ "run_log_uri": "linked.jsonl" }),
        );

        let response = session.dispatch(&request).unwrap();

        assert!(
            response
                .message
                .as_deref()
                .unwrap_or("")
                .contains("must not traverse a symlink"),
            "{:?}",
            response.message
        );
        let _ = std::fs::remove_file(link);
        let _ = std::fs::remove_file(outside);
        let _ = std::fs::remove_dir(root);
    }

    #[cfg(unix)]
    #[test]
    fn bridge_artifact_root_rejects_symlink_directory() {
        let root = temp_path("pid-sim-artifact-real-root", "dir");
        std::fs::create_dir(&root).unwrap();
        let link = temp_path("pid-sim-artifact-linked-root", "dir");
        std::os::unix::fs::symlink(&root, &link).unwrap();
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::new(writer, demo_sim());

        let error = session.set_artifact_root(&link).unwrap_err();

        assert!(error.to_string().contains("must not traverse a symlink"));
        let _ = std::fs::remove_file(link);
        let _ = std::fs::remove_dir(root);
    }

    #[cfg(unix)]
    #[test]
    fn bridge_artifact_root_rejects_intermediate_symlink() {
        let container = temp_path("pid-sim-artifact-root-container", "dir");
        let real = container.join("real");
        let nested = real.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let link = container.join("linked");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let error = canonical_artifact_root(link.join("nested")).unwrap_err();

        assert!(error.to_string().contains("must not traverse a symlink"));
        let _ = std::fs::remove_file(link);
        let _ = std::fs::remove_dir(nested);
        let _ = std::fs::remove_dir(real);
        let _ = std::fs::remove_dir(container);
    }

    #[test]
    fn canonical_new_artifact_path_accepts_platform_temp_directory_alias() {
        // macOS commonly exposes /var as a symlink to /private/var. The
        // operator-supplied parent is canonicalized before strict checks so a
        // standard temp_dir path works without weakening checks below it.
        let requested = std::env::temp_dir().join(format!(
            "pid-sim-new-artifact-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));

        let (prepared, root) = canonical_new_artifact_path(&requested).unwrap();

        assert_eq!(root, std::env::temp_dir().canonicalize().unwrap());
        assert_eq!(prepared.parent(), Some(root.as_path()));
        assert_eq!(prepared.file_name(), requested.file_name());
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
        assert!(session.stop_requested());
        assert!(!session.run_ended());
        assert!(session.finish_run(RunStatus::Succeeded, None).unwrap());
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
        set_temp_artifact_root(&mut session, &source);
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
        let source_trace_hash_v2 = pid_runlog::summarize_path(&source).unwrap().trace_hash_v2;
        assert_eq!(result["trace_hash_v2"], source_trace_hash_v2);
        assert_eq!(result["trace_hash_revision"], "replay_trace_v2");
        let claimed_sha256 = result["sha256"].as_str().unwrap().to_string();
        assert_eq!(claimed_sha256.len(), 64);
        assert!(output.exists());
        assert_eq!(pid_runlog::sha256_file(&output).unwrap(), claimed_sha256);
        assert!(std::fs::read(&output).unwrap().starts_with(b"RRF"));

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.actions.len(), 0);
        assert_eq!(state.artifacts.len(), 1);
        assert_eq!(state.artifacts[0].kind, "rerun_rrd");
        assert_eq!(state.artifacts[0].uri, output.display().to_string());
        assert_eq!(
            state.artifacts[0].sha256.as_deref(),
            Some(claimed_sha256.as_str())
        );
        assert!(events.iter().any(|event| matches!(
            event,
            RunLogEvent::ArtifactLogged { metadata, .. }
                if metadata.get("trace_hash_v2") == Some(&source_trace_hash_v2)
                    && metadata.get("trace_hash_revision").map(String::as_str)
                        == Some("replay_trace_v2")
        )));
        let artifact_index = events
            .iter()
            .position(|event| matches!(event, RunLogEvent::ArtifactLogged { .. }))
            .unwrap();
        let response_index = events
            .iter()
            .position(|event| matches!(event, RunLogEvent::BridgeResponse { .. }))
            .unwrap();
        assert!(
            artifact_index < response_index,
            "artifact provenance must precede the success acknowledgement"
        );

        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn bridge_session_export_rerun_preserves_preexisting_output() {
        let root = temp_path("pid-sim-export-preserve-root", "dir");
        std::fs::create_dir(&root).unwrap();
        let source = root.join("source.jsonl");
        let output = root.join("existing.rrd");
        write_minimal_run_log(&source, "export-preserve-source");
        std::fs::write(&output, b"sentinel").unwrap();
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-export-preserve-test".to_string(),
            session_id: None,
        };
        let writer = RunLogWriter::new(Vec::new());
        let mut session = SimBridgeSession::new(writer, demo_sim());
        session.set_artifact_root(&root).unwrap();
        let request = bridge_request(
            "req-export-preserve",
            BridgeMethod::ExportRerun,
            actor,
            Some(0),
            0,
            json!({
                "run_log_uri": "source.jsonl",
                "output_uri": "existing.rrd",
            }),
        );

        let response = session.dispatch(&request).unwrap();

        assert!(
            !response.ok && std::fs::read(&output).unwrap() == b"sentinel",
            "response={response:?}"
        );
        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(output);
        let _ = std::fs::remove_dir(root);
    }

    #[test]
    fn export_runlog_to_rerun_preserves_preexisting_target_for_direct_callers() {
        let source = temp_path("pid-sim-export-direct-source", "jsonl");
        let output = temp_path("pid-sim-export-direct-existing", "rrd");
        write_minimal_run_log(&source, "export-direct-source");
        std::fs::write(&output, b"sentinel").unwrap();

        let result = export_runlog_to_rerun(&source, &output);

        assert!(
            result.is_err() && std::fs::read(&output).unwrap() == b"sentinel",
            "result={result:?}"
        );
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
        set_temp_artifact_root(&mut session, &source);

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

        // Any non-.rrd output is refused: this file-producing method is limited
        // to its declared Rerun artifact type.
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
        )
        .unwrap()
        .unwrap();
        assert!(ok.is_ok(), "{:?}", ok.error);
        assert_eq!(ok.id, json!(7));
        // Unknown method → -32601 with the id echoed, and a run-log trace.
        let unknown = dispatch_rpc_text_request(
            r#"{"jsonrpc":"2.0","id":"probe","method":"sim.destroy","params":{}}"#,
            2,
            &mut session,
            actor.clone(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(unknown.error.as_ref().unwrap().code, -32601);
        assert_eq!(unknown.id, json!("probe"));
        // Malformed JSON → -32700, id null per JSON-RPC 2.0, and a trace.
        let malformed = dispatch_rpc_text_request("{nope", 3, &mut session, actor.clone())
            .unwrap()
            .unwrap();
        assert_eq!(malformed.error.as_ref().unwrap().code, -32700);
        assert_eq!(malformed.id, Value::Null);
        // Structured (array) id → -32600 Invalid Request, id null, and a trace.
        let bad_id = dispatch_rpc_text_request(
            r#"{"jsonrpc":"2.0","id":[1],"method":"sim.status","params":{}}"#,
            4,
            &mut session,
            actor,
        )
        .unwrap()
        .unwrap();
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
    fn rpc_dispatch_distinguishes_parse_errors_from_invalid_request_shapes() {
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, DEFAULT_BRIDGE_RUN_ID, "rpc-invalid-shapes");
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "rpc-invalid-shapes".to_string(),
            session_id: None,
        };

        let parse = dispatch_rpc_text_request("{", 1, &mut session, actor.clone())
            .unwrap()
            .unwrap();
        assert_eq!(parse.error.as_ref().unwrap().code, -32700);

        for (index, text) in ["{}", "42", "[]"].into_iter().enumerate() {
            let invalid = dispatch_rpc_text_request(text, index + 2, &mut session, actor.clone())
                .unwrap()
                .unwrap();
            assert_eq!(invalid.error.as_ref().unwrap().code, -32600, "{text}");
        }
        let invalid_params = dispatch_rpc_text_request(
            r#"{"jsonrpc":"2.0","id":"bad-params","method":"sim.status","params":7}"#,
            5,
            &mut session,
            actor,
        )
        .unwrap()
        .unwrap();
        assert_eq!(invalid_params.error.as_ref().unwrap().code, -32602);

        session.finish_run(RunStatus::Succeeded, None).unwrap();
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let report = validate_events(&events).unwrap();
        assert!(report.is_valid(), "{:?}", report.issues);
    }

    #[test]
    fn rpc_invalid_step_params_cannot_silently_mutate_with_defaults() {
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, DEFAULT_BRIDGE_RUN_ID, "rpc-positional-params");
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::new(writer, sim);
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "rpc-positional-params".to_string(),
            session_id: None,
        };

        for (index, text, expected_code) in [
            (
                1,
                r#"{"jsonrpc":"2.0","id":"array-step","method":"sim.step","params":[999]}"#,
                -32602,
            ),
            (
                2,
                r#"{"jsonrpc":"2.0","id":"misspelled-step","method":"sim.step","params":{"dtt":999}}"#,
                -32602,
            ),
            (
                3,
                r#"{"jsonrpc":"2.0","id":"missing-step","method":"sim.step","params":{}}"#,
                -32000,
            ),
            (
                4,
                r#"{"jsonrpc":"2.0","id":"null-step","method":"sim.step","params":{"dt":null}}"#,
                -32000,
            ),
        ] {
            let response = dispatch_rpc_text_request(text, index, &mut session, actor.clone())
                .unwrap()
                .unwrap();
            assert_eq!(
                response.error.as_ref().unwrap().code,
                expected_code,
                "{text}"
            );
            assert_eq!(session.step(), 0, "{text}");
            assert!(!session.poisoned(), "{text}");
        }
        let status = dispatch_rpc_text_request(
            r#"{"jsonrpc":"2.0","id":"status","method":"sim.status","params":{}}"#,
            5,
            &mut session,
            actor,
        )
        .unwrap()
        .unwrap();
        assert!(status.is_ok());

        session.finish_run(RunStatus::Succeeded, None).unwrap();
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert!(state.actions.is_empty());
        let report = validate_events(&events).unwrap();
        assert!(report.is_valid(), "{:?}", report.issues);
    }

    #[test]
    fn colliding_and_reused_rpc_ids_do_not_invalidate_the_log() {
        // JSON-RPC clients may legally send `1` and "1", reuse an id after
        // completion, and fire multiple notifications (missing id). Under the old
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
            let response =
                dispatch_rpc_text_request(line, idx + 1, &mut session, actor.clone()).unwrap();
            if line.contains("\"id\"") {
                let response = response.expect("requests with ids receive a response");
                assert!(response.is_ok(), "{:?}", response.error);
            } else {
                assert!(response.is_none(), "notifications must be silent");
            }
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
        assert!(ids.contains(&"message-4:notification".to_string()));
        assert!(ids.contains(&"message-5:notification".to_string()));
        assert!(!ids.iter().any(|id| id.ends_with(":null")));
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
                actor.clone(),
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
                actor.clone(),
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
        let oversized = session
            .dispatch(&bridge_request(
                "step-oversized-dt",
                BridgeMethod::SimStep,
                actor,
                Some(0),
                0,
                json!({ "dt": 1e300 }),
            ))
            .unwrap();
        assert!(!oversized.ok, "oversized dt must fail as a domain error");
        assert_eq!(session.step(), 0);
        assert!(!session.poisoned());
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert!(state.actions.is_empty());
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
        set_temp_artifact_root(&mut session, &source);
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
    fn translate_object_overflow_is_a_transactional_domain_rejection() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-translate-overflow".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, DEFAULT_BRIDGE_RUN_ID, "translate-overflow");
        let sim = demo_sim();
        writer.append(&sim.snapshot_event()).unwrap();
        let mut session = SimBridgeSession::new(writer, sim);

        let set_object = bridge_request(
            "req-max-position",
            BridgeMethod::SceneSetObject,
            actor.clone(),
            Some(0),
            0,
            json!({
                "object_id": "red_cube",
                "pose": {
                    "position": [f64::MAX, 0.0, 0.0],
                    "orientation_xyzw": [0.0, 0.0, 0.0, 1.0]
                },
                "velocity": [0.0, 0.0, 0.0]
            }),
        );
        assert!(session.dispatch(&set_object).unwrap().ok);

        let translate = bridge_request(
            "req-overflow-translation",
            BridgeMethod::InterventionApply,
            actor.clone(),
            Some(0),
            0,
            json!({
                "intervention_type": "translate_object",
                "payload": { "object_id": "red_cube", "delta": [f64::MAX, 0.0, 0.0] }
            }),
        );
        let rejected = session.dispatch(&translate).unwrap();
        assert!(!rejected.ok);
        assert!(rejected
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("translated object position"));
        assert!(!session.poisoned());

        let status = bridge_request(
            "req-after-overflow",
            BridgeMethod::SimStatus,
            actor,
            Some(0),
            0,
            json!({}),
        );
        assert!(session.dispatch(&status).unwrap().ok);
        session
            .finish_run(RunStatus::Succeeded, None)
            .expect("finish run");

        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert!(state.interventions.is_empty());
        assert_eq!(
            state.object_poses["red_cube"].pose.position,
            [f64::MAX, 0.0, 0.0]
        );
        let report = pid_runlog::validate_events(&events).unwrap();
        assert!(report.is_valid(), "{:?}", report.issues);
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
    fn rpc_line_processor_silences_notification_then_answers_explicit_null_request() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-rpc-notification-test".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, DEFAULT_BRIDGE_RUN_ID, "rpc-notification");
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let input = concat!(
            r#"{"jsonrpc":"2.0","method":"sim.status","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":null,"method":"sim.status","params":{}}"#,
            "\n"
        );
        let mut output = Vec::new();

        let handled =
            dispatch_rpc_lines(Cursor::new(input), &mut output, &mut session, actor).unwrap();

        assert_eq!(handled, 2);
        let responses = String::from_utf8(output).unwrap();
        let lines = responses.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 1, "notification must not emit a response");
        let response: BridgeRpcResponse = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(response.id, Value::Null);
        assert!(response.is_ok());

        session.finish_run(RunStatus::Succeeded, None).unwrap();
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        let request_ids = events
            .iter()
            .filter_map(|event| match event {
                RunLogEvent::BridgeRequest { request_id, .. } => Some(request_id.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            request_ids,
            vec!["message-1:notification", "message-2:null"]
        );
        let report = validate_events(&events).unwrap();
        assert!(report.is_valid(), "{:?}", report.issues);
    }

    #[test]
    fn bridge_rpc_line_processor_rejects_and_logs_oversized_line() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "oversized-rpc-test".to_string(),
            session_id: None,
        };
        let mut writer = RunLogWriter::new(Vec::new());
        append_run_prefix(&mut writer, DEFAULT_BRIDGE_RUN_ID, "oversized-rpc");
        let mut session = SimBridgeSession::new(writer, demo_sim());
        let input = vec![b' '; MAX_RPC_LINE_BYTES as usize];
        let mut output = Vec::new();

        let error =
            dispatch_rpc_lines(Cursor::new(input), &mut output, &mut session, actor).unwrap_err();

        assert!(error.to_string().contains("exceeds"));
        assert!(output.is_empty());
        let events = read_events(Cursor::new(session.into_inner())).unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            RunLogEvent::ErrorLogged { message, .. } if message.contains("exceeds")
        )));
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
        assert!(session.stop_requested());
        assert!(!session.run_ended());
        session
            .finish_run(
                RunStatus::Succeeded,
                Some("transport completed".to_string()),
            )
            .unwrap();
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
    fn flow_verifier_rejects_wrapped_step_adjacency_without_panicking() {
        let mut previous = demo_sim().snapshot_event();
        let mut current = previous.clone();
        if let RunLogEvent::SimSnapshot { step, .. } = &mut previous {
            *step = u64::MAX;
        }
        if let RunLogEvent::SimSnapshot { step, .. } = &mut current {
            *step = 1;
        }
        let events = vec![
            previous,
            current,
            RunLogEvent::FlowGt {
                step: 1,
                timestamp_ns: 1,
                object_id: "cube".to_string(),
                flow: vec![[0.0; 3]],
            },
        ];

        let report = verify_flow_gt(&events, 1e-12);

        assert!(!report.is_valid());
        assert!(!report.issues.is_empty());
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
            payload_hash: pid_runlog::canonical_json_hash_v2(&payload).unwrap(),
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

    #[test]
    fn sim_replay_verifier_rejects_missing_step_dt_instead_of_inventing_a_default() {
        let mut expected = demo_sim();
        let mut events = vec![expected.snapshot_event()];
        let payload = json!({});
        events.push(RunLogEvent::ActionApplied {
            step: 1,
            timestamp_ns: 100_000_000,
            actor: Actor {
                actor_type: ActorType::Script,
                actor_id: "sim-replay-test".to_string(),
                session_id: None,
            },
            action_type: "sim.step".to_string(),
            payload_hash: pid_runlog::canonical_json_hash_v2(&payload).unwrap(),
            payload,
        });
        expected.step_fixed(0.1).unwrap();
        events.push(expected.snapshot_event());

        let report = verify_sim_replay(&events, 1e-12);

        assert!(!report.is_valid());
        assert_eq!(report.checked_actions, 0);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("dt must be a number")));
    }

    #[test]
    fn sim_replay_verifier_rejects_unknown_action_types() {
        let sim = demo_sim();
        let mut events = vec![sim.snapshot_event()];
        let payload = json!({});
        events.push(RunLogEvent::ActionApplied {
            step: 0,
            timestamp_ns: 0,
            actor: Actor {
                actor_type: ActorType::Script,
                actor_id: "sim-replay-test".to_string(),
                session_id: None,
            },
            action_type: "sim.unimplemented".to_string(),
            payload_hash: pid_runlog::canonical_json_hash_v2(&payload).unwrap(),
            payload,
        });
        events.push(sim.snapshot_event());

        let report = verify_sim_replay(&events, 1e-12);

        assert!(!report.is_valid());
        assert_eq!(report.checked_actions, 0);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("unsupported replay action type")));
    }

    #[test]
    fn sim_replay_verifier_rejects_effect_payload_substitution_between_request_and_response() {
        let actor = Actor {
            actor_type: ActorType::Script,
            actor_id: "sim-replay-lineage".to_string(),
            session_id: Some("lineage".to_string()),
        };
        let requested_payload = json!({"dt": 0.1});
        let applied_payload = json!({"dt": 0.2});
        let mut sim = demo_sim();
        let mut events = vec![
            sim.snapshot_event(),
            RunLogEvent::BridgeRequest {
                step: Some(0),
                timestamp_ns: 0,
                request_id: "lineage-request".to_string(),
                actor: actor.clone(),
                method: "sim.step".to_string(),
                payload_hash: pid_runlog::canonical_json_hash_v2(&requested_payload).unwrap(),
                payload: requested_payload,
            },
            RunLogEvent::ActionApplied {
                step: 1,
                timestamp_ns: 200_000_000,
                actor,
                action_type: "sim.step".to_string(),
                payload_hash: pid_runlog::canonical_json_hash_v2(&applied_payload).unwrap(),
                payload: applied_payload,
            },
        ];
        sim.step_fixed(0.2).unwrap();
        events.push(sim.snapshot_event());
        events.push(RunLogEvent::BridgeResponse {
            step: Some(1),
            timestamp_ns: 200_000_000,
            request_id: "lineage-request".to_string(),
            ok: true,
            message: None,
            result_hash: None,
        });

        let report = verify_sim_replay(&events, 1e-12);

        assert!(!report.is_valid());
        assert_eq!(report.checked_actions, 1);
        assert!(report.issues.iter().any(
            |issue| issue.contains("does not exactly bind actor and payload to bridge request")
        ));
    }
}
