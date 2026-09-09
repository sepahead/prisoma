use anyhow::Result;
use pid_bridge::{BridgeHandler, BridgeMethod, BridgeRequest, LocalBridge};
use pid_runlog::{read_events, Actor, ActorType, RunLogEvent, RunLogWriter};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::io::{self, Cursor, Write};
use std::rc::Rc;

#[derive(Default)]
struct SinkState {
    bytes: Vec<u8>,
    trace: Vec<&'static str>,
    writes: usize,
    flushes: usize,
    lines: usize,
}

struct RecoveringSink {
    state: Rc<RefCell<SinkState>>,
    fail_after_bytes: Option<usize>,
    fail_after_lines: Option<usize>,
    fail_flush: Option<usize>,
}

impl RecoveringSink {
    fn healthy() -> Self {
        Self {
            state: Rc::default(),
            fail_after_bytes: None,
            fail_after_lines: None,
            fail_flush: None,
        }
    }
}

impl Write for RecoveringSink {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        let mut state = self.state.borrow_mut();
        state.writes += 1;
        if self.fail_after_bytes == Some(state.bytes.len())
            || self.fail_after_lines == Some(state.lines)
        {
            self.fail_after_bytes = None;
            self.fail_after_lines = None;
            return Err(io::Error::other("injected recoverable write failure"));
        }
        let count = self.fail_after_bytes.map_or(input.len(), |limit| {
            input.len().min(limit - state.bytes.len())
        });
        state.bytes.extend_from_slice(&input[..count]);
        state.lines += input[..count].iter().filter(|byte| **byte == b'\n').count();
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self.state.borrow_mut();
        state.flushes += 1;
        state.trace.push("flush");
        if self.fail_flush == Some(state.flushes) {
            self.fail_flush = None;
            return Err(io::Error::other("injected recoverable flush failure"));
        }
        Ok(())
    }
}

struct Handler {
    state: Rc<RefCell<SinkState>>,
    calls: usize,
    reject_next: bool,
}

impl BridgeHandler for Handler {
    fn handle(&mut self, _: &BridgeRequest) -> Result<Value> {
        self.calls += 1;
        self.state.borrow_mut().trace.push("handle");
        if std::mem::take(&mut self.reject_next) {
            anyhow::bail!("unsupported application action");
        }
        Ok(json!({"accepted": true}))
    }
}

fn request(id: &str) -> BridgeRequest {
    BridgeRequest {
        request_id: id.into(),
        step: Some(0),
        timestamp_ns: 0,
        actor: Actor {
            actor_type: ActorType::Script,
            actor_id: "external-handler-test".into(),
            session_id: None,
        },
        method: BridgeMethod::SimStep,
        payload: json!({"dt": 0.1}),
    }
}

fn session(sink: RecoveringSink) -> (LocalBridge<RecoveringSink>, Handler) {
    let handler = Handler {
        state: Rc::clone(&sink.state),
        calls: 0,
        reject_next: false,
    };
    (LocalBridge::new(RunLogWriter::new(sink)), handler)
}

fn assert_stopped(bridge: &mut LocalBridge<RecoveringSink>, handler: &mut Handler) {
    let before = {
        let state = handler.state.borrow();
        (
            state.bytes.clone(),
            state.writes,
            state.flushes,
            handler.calls,
        )
    };
    assert!(bridge.dispatch(&request("later"), handler, 1).is_err());
    assert!(bridge.record_request(&request("manual")).is_err());
    assert!(bridge.flush().is_err());
    let state = handler.state.borrow();
    assert_eq!(
        (
            state.bytes.clone(),
            state.writes,
            state.flushes,
            handler.calls
        ),
        before,
        "a recovering sink must not reopen the failed session"
    );
}

#[test]
fn partial_request_write_stops_the_session_before_any_handler_call() {
    for accepted_bytes in [0, 13] {
        let mut sink = RecoveringSink::healthy();
        sink.fail_after_bytes = Some(accepted_bytes);
        let (mut bridge, mut handler) = session(sink);
        assert!(bridge.dispatch(&request("first"), &mut handler, 1).is_err());
        assert_eq!(handler.calls, 0);
        assert_eq!(handler.state.borrow().bytes.len(), accepted_bytes);
        assert_stopped(&mut bridge, &mut handler);
    }
}

#[test]
fn request_flush_failure_prevents_handler_dispatch() {
    let mut sink = RecoveringSink::healthy();
    sink.fail_flush = Some(1);
    let (mut bridge, mut handler) = session(sink);
    assert!(bridge.dispatch(&request("first"), &mut handler, 1).is_err());
    assert_eq!(handler.calls, 0);
    assert_stopped(&mut bridge, &mut handler);
}

#[test]
fn response_write_failure_stops_further_actions() {
    let mut sink = RecoveringSink::healthy();
    sink.fail_after_lines = Some(1);
    let (mut bridge, mut handler) = session(sink);
    assert!(bridge.dispatch(&request("first"), &mut handler, 1).is_err());
    assert_eq!(handler.calls, 1);
    assert_stopped(&mut bridge, &mut handler);
}

#[test]
fn response_flush_failure_stops_further_actions() {
    let mut sink = RecoveringSink::healthy();
    sink.fail_flush = Some(2);
    let (mut bridge, mut handler) = session(sink);
    assert!(bridge.dispatch(&request("first"), &mut handler, 1).is_err());
    assert_eq!(handler.calls, 1);
    assert_stopped(&mut bridge, &mut handler);
}

#[test]
fn healthy_dispatch_flushes_on_both_sides_of_the_handler() {
    let (mut bridge, mut handler) = session(RecoveringSink::healthy());
    assert!(
        bridge
            .dispatch(&request("first"), &mut handler, 1)
            .unwrap()
            .ok
    );
    assert_eq!(handler.state.borrow().trace, ["flush", "handle", "flush"]);
    let events = read_events(Cursor::new(&handler.state.borrow().bytes)).unwrap();
    assert!(matches!(
        events.as_slice(),
        [
            RunLogEvent::BridgeRequest { .. },
            RunLogEvent::BridgeResponse { ok: true, .. }
        ]
    ));
}

#[test]
fn invalid_identity_does_not_poison_a_healthy_session() {
    let (mut bridge, mut handler) = session(RecoveringSink::healthy());
    assert!(bridge.dispatch(&request(" "), &mut handler, 1).is_err());
    assert!(handler.state.borrow().bytes.is_empty());
    assert!(
        bridge
            .dispatch(&request("valid"), &mut handler, 1)
            .unwrap()
            .ok
    );
    assert_eq!(handler.calls, 1);
}

#[test]
fn recorded_domain_error_allows_the_next_request() {
    let (mut bridge, mut handler) = session(RecoveringSink::healthy());
    handler.reject_next = true;
    assert!(
        !bridge
            .dispatch(&request("rejected"), &mut handler, 1)
            .unwrap()
            .ok
    );
    assert!(
        bridge
            .dispatch(&request("accepted"), &mut handler, 1)
            .unwrap()
            .ok
    );
    let events = read_events(Cursor::new(&handler.state.borrow().bytes)).unwrap();
    assert_eq!(events.len(), 4);
    assert_eq!(handler.calls, 2);
}

#[test]
fn manual_append_failure_also_stops_dispatch() {
    let mut sink = RecoveringSink::healthy();
    sink.fail_after_bytes = Some(13);
    let (mut bridge, mut handler) = session(sink);
    assert!(bridge.record_request(&request("manual")).is_err());
    assert_stopped(&mut bridge, &mut handler);
}

#[test]
fn manual_flush_failure_also_stops_dispatch() {
    let mut sink = RecoveringSink::healthy();
    sink.fail_flush = Some(1);
    let (mut bridge, mut handler) = session(sink);
    bridge.record_request(&request("manual")).unwrap();
    assert!(bridge.flush().is_err());
    assert_stopped(&mut bridge, &mut handler);
}

#[test]
fn response_admission_failure_after_dispatch_poisons_the_session() {
    let sink = RecoveringSink::healthy();
    let mut handler = Handler {
        state: Rc::clone(&sink.state),
        calls: 0,
        reject_next: false,
    };
    let mut bridge = LocalBridge::with_safe_mode_and_run_log_limits(
        RunLogWriter::new(sink),
        false,
        pid_bridge::BridgeRunLogLimits {
            max_bytes: 4096,
            max_events: 1,
        },
    );
    assert!(bridge.dispatch(&request("first"), &mut handler, 1).is_err());
    assert_eq!(handler.calls, 1);
    assert_stopped(&mut bridge, &mut handler);
}

#[test]
fn request_admission_failure_keeps_an_unwritten_sink_usable() {
    let sink = RecoveringSink::healthy();
    let mut handler = Handler {
        state: Rc::clone(&sink.state),
        calls: 0,
        reject_next: false,
    };
    let mut bridge = LocalBridge::with_safe_mode_and_run_log_limits(
        RunLogWriter::new(sink),
        false,
        pid_bridge::BridgeRunLogLimits {
            max_bytes: 4096,
            max_events: 2,
        },
    );
    let mut oversized = request("oversized");
    oversized.payload = json!({"padding": "x".repeat(4096)});
    assert!(bridge.dispatch(&oversized, &mut handler, 1).is_err());
    assert!(handler.state.borrow().bytes.is_empty());
    assert!(
        bridge
            .dispatch(&request("valid"), &mut handler, 1)
            .unwrap()
            .ok
    );
    assert_eq!(handler.calls, 1);
}
