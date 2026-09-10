use anyhow::Result;
use pid_bridge::{
    BridgeHandler, BridgeRequest, BridgeRpcRequest, LocalBridge, RequestMethod, BRIDGE_METHODS,
};
use pid_runlog::{read_events, Actor, ActorType, RunLogEvent, RunLogWriter};
use serde_json::{json, Value};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::Cursor;

struct ApplicationMethod(&'static str);

impl RequestMethod for ApplicationMethod {
    fn as_str(&self) -> &str {
        self.0
    }
}

struct Handler<'a>(&'a Cell<u64>);

impl BridgeHandler<ApplicationMethod> for Handler<'_> {
    fn handle(&mut self, request: &BridgeRequest<ApplicationMethod>) -> Result<Value> {
        self.0.set(self.0.get() + 1);
        Ok(request.payload.clone())
    }
}

fn request(method: &'static str) -> BridgeRequest<ApplicationMethod> {
    BridgeRequest {
        request_id: "application-request".into(),
        step: Some(0),
        timestamp_ns: 10,
        actor: Actor {
            actor_type: ActorType::Script,
            actor_id: "declared-application".into(),
            session_id: None,
        },
        method: ApplicationMethod(method),
        payload: json!({"target": [0.1, -0.2]}),
    }
}

#[test]
fn external_method_uses_the_existing_canonical_request_and_response() {
    let calls = Cell::new(0);
    let mut handler = Handler(&calls);
    let mut bridge = LocalBridge::new(RunLogWriter::new(Vec::new()));
    let response = bridge
        .dispatch(&request("environment.advance"), &mut handler, 11)
        .unwrap();
    assert!(response.ok);
    assert_eq!(calls.get(), 1);
    let events = read_events(Cursor::new(bridge.into_inner())).unwrap();
    assert!(matches!(
        events.as_slice(),
        [RunLogEvent::BridgeRequest { method, .. }, RunLogEvent::BridgeResponse { ok: true, .. }]
            if method == "environment.advance"
    ));
}

#[test]
fn typed_extension_does_not_register_a_builtin_rpc_method() {
    assert!(!BRIDGE_METHODS.contains(&"environment.advance"));
    let rpc: BridgeRpcRequest = serde_json::from_value(json!({
        "jsonrpc": "2.0", "id": 1, "method": "environment.advance", "params": {}
    }))
    .unwrap();
    assert!(rpc.validated_method().is_err());
}

#[test]
fn application_method_defaults_to_blocked_in_safe_mode() {
    let calls = Cell::new(0);
    let mut handler = Handler(&calls);
    let mut bridge = LocalBridge::with_safe_mode(RunLogWriter::new(Vec::new()), true);
    let response = bridge
        .dispatch(&request("environment.advance"), &mut handler, 11)
        .unwrap();
    assert!(!response.ok);
    assert_eq!(calls.get(), 0);
    assert_eq!(
        read_events(Cursor::new(bridge.into_inner())).unwrap().len(),
        2
    );
}

#[test]
fn empty_application_identity_is_rejected_before_logging_or_dispatch() {
    let calls = Cell::new(0);
    let mut handler = Handler(&calls);
    let mut bridge = LocalBridge::new(RunLogWriter::new(Vec::new()));
    assert!(bridge.dispatch(&request(" "), &mut handler, 11).is_err());
    assert_eq!(calls.get(), 0);
    assert!(bridge.into_inner().is_empty());
}

#[test]
fn response_clock_observes_the_completed_handler() {
    let calls = Cell::new(0);
    let clock_calls = Cell::new(0);
    let mut handler = Handler(&calls);
    let mut bridge = LocalBridge::new(RunLogWriter::new(Vec::new()));
    let response = bridge
        .dispatch_with_clock(&request("environment.advance"), &mut handler, || {
            assert_eq!(calls.get(), 1);
            clock_calls.set(clock_calls.get() + 1);
            Ok(29)
        })
        .unwrap();
    assert_eq!(response.timestamp_ns, 29);
    assert_eq!(clock_calls.get(), 1);
}

#[test]
fn failed_or_regressing_clock_retires_an_already_dispatched_request() {
    for time in [None, Some(9)] {
        let calls = Cell::new(0);
        let mut handler = Handler(&calls);
        let mut bridge = LocalBridge::new(RunLogWriter::new(Vec::new()));
        assert!(bridge
            .dispatch_with_clock(&request("environment.advance"), &mut handler, || {
                time.ok_or_else(|| anyhow::anyhow!("unavailable clock"))
            })
            .is_err());
        assert!(bridge.poisoned());
        assert!(bridge
            .dispatch(&request("environment.advance"), &mut handler, 30)
            .is_err());
        assert_eq!(calls.get(), 1);
        assert_eq!(
            read_events(Cursor::new(bridge.into_inner())).unwrap().len(),
            1
        );
    }
}

struct RecordingHandler<'a> {
    calls: &'a Cell<u64>,
    fail_evidence: bool,
}

impl BridgeHandler<ApplicationMethod> for RecordingHandler<'_> {
    fn handle(&mut self, _: &BridgeRequest<ApplicationMethod>) -> Result<Value> {
        self.calls.set(self.calls.get() + 1);
        Ok(json!({"observed_tick": 1}))
    }

    fn recorded_events(
        &self,
        request: &BridgeRequest<ApplicationMethod>,
        response: &pid_bridge::BridgeResponse,
    ) -> Result<Vec<RunLogEvent>> {
        if self.fail_evidence {
            anyhow::bail!("observation binding failed");
        }
        Ok(vec![RunLogEvent::LabelObserved {
            step: request.step.unwrap(),
            timestamp_ns: response.timestamp_ns,
            name: "test.application_result.v1".into(),
            value: response.result.clone().unwrap(),
            metadata: BTreeMap::new(),
        }])
    }
}

#[test]
fn application_evidence_precedes_its_response_in_the_canonical_log() {
    let calls = Cell::new(0);
    let mut handler = RecordingHandler {
        calls: &calls,
        fail_evidence: false,
    };
    let mut bridge = LocalBridge::new(RunLogWriter::new(Vec::new()));
    assert!(
        bridge
            .dispatch(&request("environment.advance"), &mut handler, 11)
            .unwrap()
            .ok
    );
    let events = read_events(Cursor::new(bridge.into_inner())).unwrap();
    assert!(matches!(
        events.as_slice(),
        [
            RunLogEvent::BridgeRequest { .. },
            RunLogEvent::LabelObserved {
                timestamp_ns: 11,
                ..
            },
            RunLogEvent::BridgeResponse { ok: true, .. }
        ]
    ));
}

#[test]
fn evidence_failure_prevents_a_success_response_and_further_dispatch() {
    let calls = Cell::new(0);
    let mut handler = RecordingHandler {
        calls: &calls,
        fail_evidence: true,
    };
    let mut bridge = LocalBridge::new(RunLogWriter::new(Vec::new()));
    assert!(bridge
        .dispatch(&request("environment.advance"), &mut handler, 11)
        .is_err());
    assert!(bridge.poisoned());
    assert!(bridge
        .dispatch(&request("environment.advance"), &mut handler, 11)
        .is_err());
    assert_eq!(calls.get(), 1);
    assert_eq!(
        read_events(Cursor::new(bridge.into_inner())).unwrap().len(),
        1
    );
}

#[test]
fn evidence_uses_the_same_event_budget_as_requests_and_responses() {
    for maximum in [1, 2, 3] {
        let calls = Cell::new(0);
        let mut handler = RecordingHandler {
            calls: &calls,
            fail_evidence: false,
        };
        let mut bridge = LocalBridge::with_safe_mode_and_run_log_limits(
            RunLogWriter::new(Vec::new()),
            false,
            pid_bridge::BridgeRunLogLimits {
                max_bytes: 4096,
                max_events: maximum,
            },
        );
        let result = bridge.dispatch(&request("environment.advance"), &mut handler, 11);
        assert_eq!(result.is_ok(), maximum == 3);
        assert_eq!(bridge.poisoned(), maximum != 3);
        assert_eq!(calls.get(), 1);
        assert_eq!(
            read_events(Cursor::new(bridge.into_inner())).unwrap().len(),
            maximum
        );
    }
}
