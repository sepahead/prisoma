use anyhow::Result;
use pid_runlog::{canonical_json_hash, Actor, RunLogEvent, RunLogWriter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;

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
pub struct BridgeResponse {
    pub request_id: String,
    pub step: Option<u64>,
    pub timestamp_ns: u64,
    pub ok: bool,
    pub message: Option<String>,
    pub result: Option<Value>,
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
}
