use anyhow::{bail, Result};
use pid_runlog::{canonical_json_hash, Actor, RunLogEvent, RunLogWriter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::io::Write;
use std::str::FromStr;

pub const BRIDGE_METHODS: &[&str] = &[
    "sim.status",
    "sim.reset",
    "sim.step",
    "log.start",
    "log.stop",
    "log.replay",
    "scene.set_object",
    "intervention.apply",
    "export.rerun",
];

pub const BRIDGE_TRANSPORTS: &[&str] = &["local", "stdio_jsonl", "tcp_jsonl", "websocket_jsonrpc"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeMethod {
    SimStatus,
    SimReset,
    SimStep,
    LogStart,
    LogStop,
    LogReplay,
    SceneSetObject,
    InterventionApply,
    ExportRerun,
}

impl BridgeMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            BridgeMethod::SimStatus => "sim.status",
            BridgeMethod::SimReset => "sim.reset",
            BridgeMethod::SimStep => "sim.step",
            BridgeMethod::LogStart => "log.start",
            BridgeMethod::LogStop => "log.stop",
            BridgeMethod::LogReplay => "log.replay",
            BridgeMethod::SceneSetObject => "scene.set_object",
            BridgeMethod::InterventionApply => "intervention.apply",
            BridgeMethod::ExportRerun => "export.rerun",
        }
    }

    pub fn safe_mode_allowed(&self) -> bool {
        matches!(self, BridgeMethod::SimStatus | BridgeMethod::LogReplay)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeMethodContract {
    pub method: String,
    pub request_payload: String,
    pub response_payload: String,
    pub emits_action: bool,
    pub emits_intervention: bool,
    pub safe_mode_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeContract {
    pub jsonrpc_version: String,
    /// This local profile accepts one Request object per JSONL line/WebSocket
    /// message. JSON-RPC batch arrays are deliberately unsupported.
    pub batch_supported: bool,
    /// Requests without an id execute and are logged but emit no wire response.
    pub notifications_supported: bool,
    pub transports: Vec<String>,
    pub methods: Vec<BridgeMethodContract>,
    pub runlog_schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeRunLogContract {
    pub run_log: pid_runlog::RunLogContract,
    pub bridge: BridgeContract,
}

pub fn bridge_method_contracts() -> Vec<BridgeMethodContract> {
    BRIDGE_METHODS
        .iter()
        .map(|method| BridgeMethodContract {
            method: (*method).to_string(),
            request_payload: bridge_request_payload_hint(method).to_string(),
            response_payload: bridge_response_payload_hint(method).to_string(),
            emits_action: matches!(*method, "sim.reset" | "sim.step" | "scene.set_object"),
            emits_intervention: *method == "intervention.apply",
            safe_mode_allowed: BridgeMethod::from_str(method)
                .map(|method| method.safe_mode_allowed())
                .unwrap_or(false),
        })
        .collect()
}

pub fn bridge_runlog_contract() -> BridgeRunLogContract {
    BridgeRunLogContract {
        run_log: pid_runlog::runlog_contract(),
        bridge: bridge_contract(),
    }
}

pub fn bridge_contract() -> BridgeContract {
    BridgeContract {
        jsonrpc_version: "2.0".to_string(),
        batch_supported: false,
        notifications_supported: true,
        transports: BRIDGE_TRANSPORTS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        methods: bridge_method_contracts(),
        runlog_schema_version: pid_runlog::RUN_LOG_SCHEMA_VERSION,
    }
}

fn bridge_request_payload_hint(method: &str) -> &'static str {
    match method {
        "sim.status" | "sim.reset" | "log.stop" => "{}",
        "sim.step" => r#"{"dt": number}"#,
        "log.start" => r#"{"run_id": string?, "metadata": object?}"#,
        "log.replay" => r#"{"run_log_uri": string}"#,
        "scene.set_object" => {
            r#"{"object_id": string, "pose": Pose, "velocity": [number, number, number]}"#
        }
        "intervention.apply" => {
            r#"{"intervention_type": "set_velocity"|"translate_object"|"set_pose", "payload": object}"#
        }
        "export.rerun" => r#"{"run_log_uri": string, "output_uri": string?}"#,
        _ => "object",
    }
}

fn bridge_request_parameter_names(method: &BridgeMethod) -> &'static [&'static str] {
    match method {
        BridgeMethod::SimStatus | BridgeMethod::SimReset | BridgeMethod::LogStop => &[],
        BridgeMethod::SimStep => &["dt"],
        BridgeMethod::LogStart => &["run_id", "metadata"],
        BridgeMethod::LogReplay => &["run_log_uri"],
        BridgeMethod::SceneSetObject => &["object_id", "pose", "velocity"],
        BridgeMethod::InterventionApply => &["intervention_type", "payload"],
        BridgeMethod::ExportRerun => &["run_log_uri", "output_uri"],
    }
}

fn bridge_response_payload_hint(method: &str) -> &'static str {
    match method {
        "sim.status" | "sim.reset" | "scene.set_object" => {
            r#"{"step": integer, "timestamp_ns": integer, "objects": integer}"#
        }
        "sim.step" => {
            r#"{"step": integer, "timestamp_ns": integer, "flow_gt_records": integer, "flow_pred_records": integer}"#
        }
        "log.start" => {
            r#"{"run_id": string, "active": boolean, "step": integer, "timestamp_ns": integer}"#
        }
        "log.stop" => {
            r#"{"run_id": string, "stopped": boolean, "step": integer, "timestamp_ns": integer}"#
        }
        "log.replay" => {
            r#"{"trace_hash": string, "events": integer, "valid": boolean, "config_hash": string?}"#
        }
        "intervention.apply" => {
            r#"{"accepted": boolean, "intervention_type": string, "step": integer, "timestamp_ns": integer, "objects": integer, "details": object}"#
        }
        "export.rerun" => {
            r#"{"output_uri": string, "trace_hash": string, "events": integer, "valid": boolean, "sha256": string?}"#
        }
        _ => "object",
    }
}

impl fmt::Display for BridgeMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BridgeMethod {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "sim.status" | "sim_status" => Ok(BridgeMethod::SimStatus),
            "sim.reset" | "sim_reset" => Ok(BridgeMethod::SimReset),
            "sim.step" | "sim_step" => Ok(BridgeMethod::SimStep),
            "log.start" | "log_start" => Ok(BridgeMethod::LogStart),
            "log.stop" | "log_stop" => Ok(BridgeMethod::LogStop),
            "log.replay" | "log_replay" => Ok(BridgeMethod::LogReplay),
            "scene.set_object" | "scene_set_object" => Ok(BridgeMethod::SceneSetObject),
            "intervention.apply" | "intervention_apply" => Ok(BridgeMethod::InterventionApply),
            "export.rerun" | "export_rerun" => Ok(BridgeMethod::ExportRerun),
            other => bail!("unknown bridge method: {other}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeRequest {
    pub request_id: String,
    pub step: Option<u64>,
    pub timestamp_ns: u64,
    pub actor: Actor,
    pub method: BridgeMethod,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeRpcRequest {
    pub jsonrpc: Option<String>,
    /// JSON-RPC 2.0 ids may be a String, a Number, or Null. `None` preserves the
    /// semantically distinct case where the member is absent (a notification),
    /// which executes and is logged internally but produces no wire response.
    /// Validate with [`BridgeRpcRequest::validated_id`]; anything else is a
    /// -32600 Invalid Request. A present id is echoed VERBATIM in the response.
    #[serde(
        default,
        deserialize_with = "deserialize_present_json_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<Value>,
    pub method: String,
    /// This bridge profile uses named parameters only: the member is omitted or
    /// an object. The custom deserializer preserves explicit null so it can be
    /// rejected separately from an omitted member by [`Self::validated_params`].
    #[serde(
        default,
        deserialize_with = "deserialize_present_json_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub params: Option<Value>,
}

/// Serde normally maps both a missing member and an explicit JSON null to
/// `Option::None`. JSON-RPC assigns different wire semantics to those cases,
/// so a present member is always wrapped in `Some`, including `Value::Null`.
fn deserialize_present_json_value<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Value::deserialize(deserializer).map(Some)
}

/// Render a (valid) JSON-RPC id as a bare string. Distinct JSON ids can render
/// identically (`1` vs `"1"`, explicit JSON null → `"null"`), and clients may
/// legally reuse ids — so this rendering is **not unique** and must not be used
/// as a run-log `request_id` on its own: `pid-runlog` validation hard-errors on
/// duplicate request/response ids, so a spec-valid client could invalidate the
/// log. Use [`rpc_id_to_unique_request_id`] for run-log recording; wire
/// responses echo the original [`Value`] verbatim either way.
pub fn rpc_id_to_request_id(id: &Value) -> String {
    match id {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

/// Render a (valid) JSON-RPC id as a run-log `request_id` that is **unique by
/// construction** and **type-unambiguous**: prefixed with the session-monotone
/// message index (so id reuse and cross-type collisions cannot produce
/// duplicate run-log ids) and tagged by JSON type (`n:` number, `s:` string,
/// `null` explicit-null request) so provenance stays greppable back to the wire
/// id. Notifications have no id value and must instead use
/// [`rpc_notification_to_unique_request_id`].
///
/// `1` at message 4 → `"message-4:n:1"`; `"1"` at message 5 →
/// `"message-5:s:1"`; explicit null at message 6 → `"message-6:null"`.
pub fn rpc_id_to_unique_request_id(id: &Value, message_index: usize) -> String {
    let tagged = match id {
        Value::String(s) => format!("s:{s}"),
        Value::Number(n) => format!("n:{n}"),
        Value::Null => "null".to_string(),
        other => format!("j:{other}"),
    };
    format!("message-{message_index}:{tagged}")
}

/// Construct the canonical request id for a JSON-RPC notification. Keeping
/// this distinct from an explicit `"id": null` request lets the run log
/// reconstruct whether the wire protocol emitted a response.
pub fn rpc_notification_to_unique_request_id(message_index: usize) -> String {
    format!("message-{message_index}:notification")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeRpcResponse {
    pub jsonrpc: String,
    /// Echoes the request id verbatim (String, Number, or Null per JSON-RPC 2.0).
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BridgeRpcError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeResponse {
    pub request_id: String,
    pub step: Option<u64>,
    pub timestamp_ns: u64,
    pub ok: bool,
    pub message: Option<String>,
    pub result: Option<Value>,
}

impl BridgeRpcRequest {
    /// JSON-RPC 2.0 restricts `id` to String, Number, or Null; arrays/objects/
    /// booleans are a -32600 Invalid Request.
    pub fn validated_id(&self) -> Result<Option<&Value>> {
        if self.jsonrpc.as_deref() != Some("2.0") {
            bail!("jsonrpc must be exactly \"2.0\"");
        }
        match self.id.as_ref() {
            Some(id @ (Value::String(_) | Value::Number(_) | Value::Null)) => Ok(Some(id)),
            None => Ok(None),
            Some(other) => {
                bail!("invalid JSON-RPC id (must be string, number, or null): {other}")
            }
        }
    }

    /// The local bridge profile permits omitted params or a named-parameter
    /// object. Positional arrays are rejected because no method implements a
    /// positional mapping; accepting them would let handlers silently apply
    /// defaults instead of the caller's requested values.
    pub fn validated_params(&self) -> Result<Option<&Value>> {
        match self.params.as_ref() {
            None => Ok(None),
            Some(value @ Value::Object(_)) => Ok(Some(value)),
            Some(other) => bail!("params must be an object when present, got {other}"),
        }
    }

    /// Reject top-level named parameters outside the method's exported payload
    /// contract. Individual handlers still validate required members, types,
    /// numeric domains, and nested payload schemas.
    pub fn validated_params_for_method(&self, method: &BridgeMethod) -> Result<()> {
        let Some(Value::Object(params)) = self.validated_params()? else {
            return Ok(());
        };
        let allowed = bridge_request_parameter_names(method);
        for name in params.keys() {
            if !allowed.contains(&name.as_str()) {
                bail!("unknown parameter {name:?} for method {}", method.as_str());
            }
        }
        Ok(())
    }

    pub fn validated_method(&self) -> Result<BridgeMethod> {
        BridgeMethod::from_str(&self.method)
    }

    pub fn into_bridge_request(
        self,
        actor: Actor,
        step: Option<u64>,
        timestamp_ns: u64,
    ) -> Result<BridgeRequest> {
        let id = self.validated_id()?.cloned().unwrap_or(Value::Null);
        let method = self.validated_method()?;
        self.validated_params_for_method(&method)?;
        Ok(BridgeRequest {
            request_id: rpc_id_to_request_id(&id),
            step,
            timestamp_ns,
            actor,
            method,
            payload: self.params.unwrap_or(Value::Null),
        })
    }
}

impl BridgeRpcResponse {
    pub fn success(id: impl Into<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(id: impl Into<Value>, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: None,
            error: Some(BridgeRpcError {
                code,
                message: message.into(),
            }),
        }
    }

    /// Build a response from a [`BridgeResponse`], echoing `id` VERBATIM (the
    /// original request id [`Value`] — a numeric id must come back as a number,
    /// not as the stringified `request_id` recorded in the run log).
    pub fn from_bridge_response_with_id(response: &BridgeResponse, id: Value) -> Self {
        if response.ok {
            Self::success(id, response.result.clone().unwrap_or(Value::Null))
        } else {
            Self::failure(
                id,
                -32000,
                response
                    .message
                    .clone()
                    .unwrap_or_else(|| "bridge request failed".to_string()),
            )
        }
    }

    /// Convenience for callers that only have the stringified `request_id`
    /// (e.g. tests replaying run-log records); wire dispatch paths should use
    /// [`Self::from_bridge_response_with_id`] to echo the original id type.
    pub fn from_bridge_response(response: &BridgeResponse) -> Self {
        Self::from_bridge_response_with_id(response, Value::String(response.request_id.clone()))
    }

    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }
}

pub trait BridgeHandler {
    fn handle(&mut self, request: &BridgeRequest) -> Result<Value>;
}

impl BridgeRequest {
    pub fn payload_hash(&self) -> Result<String> {
        canonical_json_hash(&self.payload)
    }

    pub fn safe_mode_allowed(&self) -> bool {
        self.method.safe_mode_allowed()
    }

    pub fn to_runlog_event(&self) -> Result<RunLogEvent> {
        Ok(RunLogEvent::BridgeRequest {
            step: self.step,
            timestamp_ns: self.timestamp_ns,
            request_id: self.request_id.clone(),
            actor: self.actor.clone(),
            method: self.method.as_str().to_string(),
            payload_hash: self.payload_hash()?,
            payload: self.payload.clone(),
        })
    }
}

impl BridgeResponse {
    pub fn blocked_by_safe_mode(request: &BridgeRequest, timestamp_ns: u64) -> Self {
        Self {
            request_id: request.request_id.clone(),
            step: request.step,
            timestamp_ns,
            ok: false,
            message: Some(format!(
                "bridge safe mode blocked method {}",
                request.method.as_str()
            )),
            result: None,
        }
    }

    pub fn result_hash(&self) -> Result<Option<String>> {
        self.result.as_ref().map(canonical_json_hash).transpose()
    }

    pub fn to_runlog_event(&self) -> Result<RunLogEvent> {
        Ok(RunLogEvent::BridgeResponse {
            step: self.step,
            timestamp_ns: self.timestamp_ns,
            request_id: self.request_id.clone(),
            ok: self.ok,
            message: self.message.clone(),
            result_hash: self.result_hash()?,
        })
    }
}

pub struct LocalBridge<W> {
    writer: RunLogWriter<W>,
    safe_mode: bool,
}

impl<W: Write> LocalBridge<W> {
    pub fn new(writer: RunLogWriter<W>) -> Self {
        Self {
            writer,
            safe_mode: false,
        }
    }

    pub fn with_safe_mode(writer: RunLogWriter<W>, safe_mode: bool) -> Self {
        Self { writer, safe_mode }
    }

    pub fn safe_mode(&self) -> bool {
        self.safe_mode
    }

    pub fn set_safe_mode(&mut self, safe_mode: bool) {
        self.safe_mode = safe_mode;
    }

    pub fn record_request(&mut self, request: &BridgeRequest) -> Result<()> {
        self.writer.append(&request.to_runlog_event()?)
    }

    pub fn record_response(&mut self, response: &BridgeResponse) -> Result<()> {
        self.writer.append(&response.to_runlog_event()?)
    }

    pub fn record_event(&mut self, event: &RunLogEvent) -> Result<()> {
        self.writer.append(event)
    }

    pub fn dispatch<H: BridgeHandler>(
        &mut self,
        request: &BridgeRequest,
        handler: &mut H,
        response_timestamp_ns: u64,
    ) -> Result<BridgeResponse> {
        self.record_request(request)?;
        if self.safe_mode && !request.safe_mode_allowed() {
            let response = BridgeResponse::blocked_by_safe_mode(request, response_timestamp_ns);
            self.record_response(&response)?;
            return Ok(response);
        }
        let handled = handler.handle(request);
        let response = match handled {
            Ok(result) => BridgeResponse {
                request_id: request.request_id.clone(),
                step: request.step,
                timestamp_ns: response_timestamp_ns,
                ok: true,
                message: None,
                result: Some(result),
            },
            Err(err) => BridgeResponse {
                request_id: request.request_id.clone(),
                step: request.step,
                timestamp_ns: response_timestamp_ns,
                ok: false,
                message: Some(err.to_string()),
                result: None,
            },
        };
        self.record_response(&response)?;
        Ok(response)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }

    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pid_runlog::{read_events, replay_events, ActorType};
    use serde_json::json;
    use std::io::Cursor;

    fn actor() -> Actor {
        Actor {
            actor_type: ActorType::Script,
            actor_id: "bridge-test".to_string(),
            session_id: Some("session".to_string()),
        }
    }

    struct EchoHandler;

    impl BridgeHandler for EchoHandler {
        fn handle(&mut self, request: &BridgeRequest) -> Result<Value> {
            Ok(json!({
                "method": request.method.as_str(),
                "payload": request.payload,
            }))
        }
    }

    #[test]
    fn bridge_records_request_and_response_events() {
        let writer = RunLogWriter::new(Vec::new());
        let mut bridge = LocalBridge::new(writer);
        let request = BridgeRequest {
            request_id: "req-1".to_string(),
            step: Some(7),
            timestamp_ns: 100,
            actor: actor(),
            method: BridgeMethod::SimStep,
            payload: json!({ "dt": 0.02 }),
        };
        let response = BridgeResponse {
            request_id: "req-1".to_string(),
            step: Some(8),
            timestamp_ns: 200,
            ok: true,
            message: None,
            result: Some(json!({ "step": 8 })),
        };
        bridge.record_request(&request).unwrap();
        bridge.record_response(&response).unwrap();
        let events = read_events(Cursor::new(bridge.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.bridge_records[0].method, "sim.step");
        assert_eq!(state.bridge_records[1].ok, Some(true));
    }

    #[test]
    fn dispatch_logs_request_and_response() {
        let writer = RunLogWriter::new(Vec::new());
        let mut bridge = LocalBridge::new(writer);
        let request = BridgeRequest {
            request_id: "req-dispatch".to_string(),
            step: Some(1),
            timestamp_ns: 10,
            actor: actor(),
            method: BridgeMethod::SimStatus,
            payload: json!({}),
        };
        let mut handler = EchoHandler;
        let response = bridge.dispatch(&request, &mut handler, 11).unwrap();
        assert!(response.ok);
        let events = read_events(Cursor::new(bridge.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.bridge_records[1].ok, Some(true));
    }

    #[test]
    fn safe_mode_blocks_mutating_dispatch() {
        let writer = RunLogWriter::new(Vec::new());
        let mut bridge = LocalBridge::with_safe_mode(writer, true);
        let request = BridgeRequest {
            request_id: "req-safe-mode".to_string(),
            step: Some(1),
            timestamp_ns: 10,
            actor: actor(),
            method: BridgeMethod::SimStep,
            payload: json!({ "dt": 0.1 }),
        };
        let mut handler = EchoHandler;
        let response = bridge.dispatch(&request, &mut handler, 11).unwrap();
        assert!(!response.ok);
        assert!(response
            .message
            .as_deref()
            .unwrap()
            .contains("safe mode blocked"));
        let events = read_events(Cursor::new(bridge.into_inner())).unwrap();
        let state = replay_events(&events).unwrap();
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.bridge_records[1].ok, Some(false));
    }

    #[test]
    fn rpc_request_converts_dotted_method() {
        let rpc = BridgeRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!("rpc-1")),
            method: "sim.step".to_string(),
            params: Some(json!({ "dt": 0.1 })),
        };
        let request = rpc.into_bridge_request(actor(), Some(0), 123).unwrap();
        assert_eq!(request.method, BridgeMethod::SimStep);
        assert_eq!(request.request_id, "rpc-1");
    }

    #[test]
    fn rpc_request_accepts_numeric_and_null_ids() {
        // JSON-RPC 2.0 ids are String | Number | Null; numeric ids are the most
        // common client convention and must not be rejected as a parse error.
        let numeric: BridgeRpcRequest =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":7,"method":"sim.status","params":{}}"#)
                .unwrap();
        assert_eq!(numeric.validated_id().unwrap().cloned(), Some(json!(7)));
        let request = numeric.into_bridge_request(actor(), Some(0), 1).unwrap();
        assert_eq!(request.request_id, "7");

        // A request without an id remains distinguishable as a notification.
        let notif: BridgeRpcRequest =
            serde_json::from_str(r#"{"jsonrpc":"2.0","method":"sim.status","params":{}}"#).unwrap();
        assert_eq!(notif.validated_id().unwrap(), None);
        assert_eq!(
            notif
                .into_bridge_request(actor(), Some(0), 1)
                .unwrap()
                .request_id,
            "null"
        );
    }

    #[test]
    fn rpc_request_rejects_structured_ids() {
        let bad: BridgeRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":[1,2],"method":"sim.status","params":{}}"#,
        )
        .unwrap();
        assert!(bad.validated_id().is_err());
        assert!(bad.into_bridge_request(actor(), Some(0), 1).is_err());
    }

    #[test]
    fn rpc_request_rejects_missing_or_wrong_protocol_version() {
        for text in [
            r#"{"id":1,"method":"sim.status","params":{}}"#,
            r#"{"jsonrpc":"1.0","id":1,"method":"sim.status","params":{}}"#,
            r#"{"jsonrpc":"2.0 ","id":1,"method":"sim.status","params":{}}"#,
        ] {
            let request: BridgeRpcRequest = serde_json::from_str(text).unwrap();
            assert!(request.validated_id().is_err(), "{text}");
            assert!(
                request.into_bridge_request(actor(), Some(0), 1).is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn rpc_request_rejects_non_structured_params() {
        for text in [
            r#"{"jsonrpc":"2.0","id":1,"method":"sim.status","params":null}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"sim.status","params":7}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"sim.status","params":"bad"}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"sim.status","params":[]}"#,
        ] {
            let request: BridgeRpcRequest = serde_json::from_str(text).unwrap();
            assert!(request.validated_params().is_err(), "{text}");
            assert!(
                request.into_bridge_request(actor(), Some(0), 1).is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn rpc_request_rejects_unknown_named_parameter() {
        let request: BridgeRpcRequest = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":1,"method":"sim.step","params":{"dtt":999}}"#,
        )
        .unwrap();
        let method = request.validated_method().unwrap();
        assert!(request.validated_params_for_method(&method).is_err());
        assert!(request.into_bridge_request(actor(), Some(0), 1).is_err());
    }

    #[test]
    fn bridge_contract_lists_methods_and_transports() {
        let contract = bridge_runlog_contract();
        assert_eq!(
            contract.run_log.schema_version,
            pid_runlog::RUN_LOG_SCHEMA_VERSION
        );
        assert!(contract
            .bridge
            .transports
            .contains(&"stdio_jsonl".to_string()));
        assert!(contract
            .bridge
            .transports
            .contains(&"tcp_jsonl".to_string()));
        assert!(contract
            .bridge
            .transports
            .contains(&"websocket_jsonrpc".to_string()));
        assert_eq!(contract.bridge.methods.len(), BRIDGE_METHODS.len());
        assert!(!contract.bridge.batch_supported);
        assert!(contract.bridge.notifications_supported);
        let step = contract
            .bridge
            .methods
            .iter()
            .find(|method| method.method == "sim.step")
            .unwrap();
        assert!(step.emits_action);
        assert!(!step.safe_mode_allowed);
        assert!(step.request_payload.contains("dt"));
        let status = contract
            .bridge
            .methods
            .iter()
            .find(|method| method.method == "sim.status")
            .unwrap();
        assert!(status.safe_mode_allowed);
        let replay = contract
            .bridge
            .methods
            .iter()
            .find(|method| method.method == "log.replay")
            .unwrap();
        assert!(replay.safe_mode_allowed);
        assert!(replay.response_payload.contains("trace_hash"));
        let export = contract
            .bridge
            .methods
            .iter()
            .find(|method| method.method == "export.rerun")
            .unwrap();
        assert!(!export.safe_mode_allowed);
        assert!(export.request_payload.contains("run_log_uri"));
        assert!(export.response_payload.contains("sha256"));
        let intervention = contract
            .bridge
            .methods
            .iter()
            .find(|method| method.method == "intervention.apply")
            .unwrap();
        assert!(!intervention.emits_action);
        assert!(intervention.emits_intervention);
        assert!(intervention.request_payload.contains("set_velocity"));
    }

    #[test]
    fn bridge_method_catalog_parses_all_methods() {
        for method in BRIDGE_METHODS {
            let parsed = BridgeMethod::from_str(method).unwrap();
            assert_eq!(parsed.as_str(), *method);
        }
    }

    #[test]
    fn rpc_response_converts_bridge_success_and_failure() {
        let success = BridgeResponse {
            request_id: "ok-1".to_string(),
            step: Some(1),
            timestamp_ns: 10,
            ok: true,
            message: None,
            result: Some(json!({ "step": 1 })),
        };
        let rpc = BridgeRpcResponse::from_bridge_response(&success);
        assert!(rpc.is_ok());
        assert_eq!(rpc.id, json!("ok-1"));
        assert_eq!(rpc.result, Some(json!({ "step": 1 })));

        let failure = BridgeResponse {
            request_id: "err-1".to_string(),
            step: Some(1),
            timestamp_ns: 10,
            ok: false,
            message: Some("bad request".to_string()),
            result: None,
        };
        let rpc = BridgeRpcResponse::from_bridge_response(&failure);
        assert!(!rpc.is_ok());
        assert_eq!(rpc.error.unwrap().message, "bad request");
    }
}
