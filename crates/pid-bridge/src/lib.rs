use anyhow::{bail, Result};
use pid_runlog::{canonical_json_hash, Actor, RunLogEvent, RunLogWriter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::io::Write;
use std::str::FromStr;

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
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BridgeRpcResponse {
    pub jsonrpc: String,
    pub id: String,
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
    pub fn into_bridge_request(
        self,
        actor: Actor,
        step: Option<u64>,
        timestamp_ns: u64,
    ) -> Result<BridgeRequest> {
        Ok(BridgeRequest {
            request_id: self.id,
            step,
            timestamp_ns,
            actor,
            method: BridgeMethod::from_str(&self.method)?,
            payload: self.params,
        })
    }
}

impl BridgeRpcResponse {
    pub fn success(id: impl Into<String>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(id: impl Into<String>, code: i64, message: impl Into<String>) -> Self {
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

    pub fn from_bridge_response(response: &BridgeResponse) -> Self {
        if response.ok {
            Self::success(
                response.request_id.clone(),
                response.result.clone().unwrap_or(Value::Null),
            )
        } else {
            Self::failure(
                response.request_id.clone(),
                -32000,
                response
                    .message
                    .clone()
                    .unwrap_or_else(|| "bridge request failed".to_string()),
            )
        }
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
}

impl<W: Write> LocalBridge<W> {
    pub fn new(writer: RunLogWriter<W>) -> Self {
        Self { writer }
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
        let state = replay_events(&events);
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
        let state = replay_events(&events);
        assert_eq!(state.bridge_records.len(), 2);
        assert_eq!(state.bridge_records[1].ok, Some(true));
    }

    #[test]
    fn rpc_request_converts_dotted_method() {
        let rpc = BridgeRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: "rpc-1".to_string(),
            method: "sim.step".to_string(),
            params: json!({ "dt": 0.1 }),
        };
        let request = rpc.into_bridge_request(actor(), Some(0), 123).unwrap();
        assert_eq!(request.method, BridgeMethod::SimStep);
        assert_eq!(request.request_id, "rpc-1");
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
        assert_eq!(rpc.id, "ok-1");
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
