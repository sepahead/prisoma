//! Optional trusted Python application callbacks through the canonical Rust bridge.
//!
//! This package records execution. It does not infer scientific outcomes or retry
//! uncertain external effects. Python, NCP, and sensor types stay outside the core.

mod storage;

use anyhow::{bail, ensure, Context, Result};
use pid_bridge::{
    parse_strict_json_value, BridgeHandler, BridgeRequest, BridgeResponse, BridgeRunLogLimits,
    LocalBridge, RequestMethod,
};
use pid_runlog::{
    canonical_json_hash_v2, inspect_event_stream, Actor, ActorType, RunLogEvent, RunLogEventStream,
    RunLogLimits, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyString;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::thread::{self, ThreadId};
use std::time::Instant;
use storage::SynchronizedFile;

const CONFIG_SCHEMA: &str = "prisoma.application_bridge.v1";
const RESULT_NAME: &str = "prisoma.application_result.v1";
const MAX_JSON: usize = 64 * 1024;
const MAX_CALLS: u64 = 1024;
const MAX_LOG_BYTES: u64 = MAX_JSON as u64 + 8192 + MAX_CALLS * (2 * MAX_JSON as u64 + 4096);

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    schema: String,
    run_id: String,
    actor_id: String,
    methods: Vec<String>,
    max_calls: u64,
    application: Value,
}

fn token(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"_.-".contains(&v))
}

fn object(text: &str) -> Result<Value> {
    ensure!(text.len() <= MAX_JSON, "JSON exceeds 65536 bytes");
    let value = parse_strict_json_value(text)?;
    ensure!(value.is_object(), "JSON must be an object");
    ensure!(
        serde_json::to_vec(&value)?.len() <= MAX_JSON,
        "encoded JSON exceeds 65536 bytes"
    );
    // The pinned owner defines numeric and canonical hash semantics.
    canonical_json_hash_v2(&value)?;
    Ok(value)
}

impl Configuration {
    fn decode(text: &str) -> Result<Self> {
        let value: Self = serde_json::from_value(object(text)?)?;
        ensure!(
            value.schema == CONFIG_SCHEMA,
            "configuration schema mismatch"
        );
        ensure!(
            token(&value.run_id, 96) && token(&value.actor_id, 96),
            "run or actor identity"
        );
        ensure!(
            (1..=MAX_CALLS).contains(&value.max_calls),
            "max_calls must be 1 through 1024"
        );
        ensure!(
            (1..=16).contains(&value.methods.len()),
            "method roster must contain 1 through 16 names"
        );
        let mut names = BTreeSet::new();
        for name in &value.methods {
            ensure!(
                token(name, 64) && names.insert(name),
                "invalid or duplicate application method"
            );
        }
        ensure!(
            value.application.is_object(),
            "application configuration must be an object"
        );
        Ok(value)
    }

    fn limits(&self) -> BridgeRunLogLimits {
        BridgeRunLogLimits {
            max_bytes: MAX_JSON as u64 + 8192 + self.max_calls * (2 * MAX_JSON as u64 + 4096),
            max_events: 3 * self.max_calls as usize + 4,
        }
    }
}

struct ApplicationMethod(String);

impl RequestMethod for ApplicationMethod {
    fn as_str(&self) -> &str {
        &self.0
    }
}

struct PythonHandler<'py> {
    callback: &'py Bound<'py, PyAny>,
    error: Option<PyErr>,
}

impl PythonHandler<'_> {
    fn invoke(&self, request: &BridgeRequest<ApplicationMethod>) -> PyResult<Value> {
        let encoded = serde_json::to_string(&request.payload).map_err(runtime)?;
        let returned = self.callback.call1((encoded,))?;
        let string = returned.cast::<PyString>()?;
        // Borrow the Python UTF-8 data before allocating an owned Rust value.
        object(string.to_str()?).map_err(runtime)
    }
}

impl BridgeHandler<ApplicationMethod> for PythonHandler<'_> {
    fn handle(&mut self, request: &BridgeRequest<ApplicationMethod>) -> Result<Value> {
        match self.invoke(request) {
            Ok(value) => Ok(value),
            Err(error) => {
                self.error = Some(error);
                bail!("application callback or result validation failed")
            }
        }
    }

    fn recorded_events(
        &self,
        request: &BridgeRequest<ApplicationMethod>,
        response: &BridgeResponse,
    ) -> Result<Vec<RunLogEvent>> {
        let Some(result) = &response.result else {
            return Ok(Vec::new());
        };
        Ok(vec![RunLogEvent::LabelObserved {
            step: request.step.context("application call index")?,
            timestamp_ns: response.timestamp_ns,
            name: RESULT_NAME.to_owned(),
            value: json!({"request_id": request.request_id, "method": request.method.as_str(), "result": result}),
            metadata: BTreeMap::from([
                ("record_role".to_owned(), "execution_receipt".to_owned()),
                ("is_outcome_label".to_owned(), "false".to_owned()),
            ]),
        }])
    }
}

fn runtime(error: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}
fn input(error: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(error.to_string())
}
fn elapsed(start: Instant) -> Result<u64> {
    Ok(u64::try_from(start.elapsed().as_nanos())?)
}

/// One synchronous, thread-owned application control session.
#[pyclass(module = "prisoma_agent_bridge._native")]
struct Bridge {
    bridge: Option<LocalBridge<SynchronizedFile>>,
    configuration: Configuration,
    path: PathBuf,
    start: Instant,
    owner: ThreadId,
    calls: u64,
    retired: bool,
}

impl Bridge {
    fn active(&self) -> PyResult<()> {
        if thread::current().id() != self.owner {
            return Err(runtime("bridge belongs to another thread"));
        }
        if self.retired || self.bridge.is_none() {
            return Err(runtime("bridge is retired"));
        }
        Ok(())
    }

    fn create(path: &Path, text: &str) -> Result<Self> {
        let configuration = Configuration::decode(text)?;
        let mut frozen = serde_json::to_value(&configuration)?;
        frozen["clock"] = json!({"origin": "bridge_creation", "unit": "ns", "kind": "host_elapsed_monotonic", "simulation_time": false});
        object(&serde_json::to_string(&frozen)?)?;
        let hash = canonical_json_hash_v2(&frozen)?;
        let path = storage::new_path(path)?;
        let start = Instant::now();
        let file = SynchronizedFile::create(&path)?;
        let mut bridge = LocalBridge::with_safe_mode_and_run_log_limits(
            RunLogWriter::new(file),
            false,
            configuration.limits(),
        );
        bridge.record_event(&RunLogEvent::RunStarted {
            schema_version: RUN_LOG_SCHEMA_VERSION,
            run_id: configuration.run_id.clone(),
            timestamp_ns: 0,
            config_hash: hash.clone(),
            metadata: BTreeMap::from([("application_bridge".to_owned(), CONFIG_SCHEMA.to_owned())]),
        })?;
        bridge.record_event(&RunLogEvent::ConfigLogged {
            timestamp_ns: 0,
            config_hash: hash,
            config: frozen,
        })?;
        bridge.flush()?;
        Ok(Self {
            bridge: Some(bridge),
            configuration,
            path,
            start,
            owner: thread::current().id(),
            calls: 0,
            retired: false,
        })
    }

    fn complete(&mut self, artifact: Option<&str>) -> Result<String> {
        let artifact_event = if let Some(name) = artifact {
            let path = storage::artifact_path(&self.path, name)?;
            let (bytes, hash) = storage::artifact_digest(&path)?;
            Some(RunLogEvent::ArtifactLogged {
                timestamp_ns: elapsed(self.start)?,
                name: "application_capture".to_owned(),
                kind: "application_supplied_bytes".to_owned(),
                uri: name.to_owned(),
                sha256: Some(hash),
                metadata: BTreeMap::from([("bytes".to_owned(), bytes.to_string())]),
            })
        } else {
            None
        };
        let bridge = self.bridge.as_mut().context("bridge is retired")?;
        if let Some(event) = artifact_event {
            bridge.record_event(&event)?;
        }
        bridge.record_event(&RunLogEvent::RunEnded {
            run_id: self.configuration.run_id.clone(),
            timestamp_ns: elapsed(self.start)?,
            status: RunStatus::Succeeded,
            message: None,
        })?;
        bridge.flush()?;
        let usage = bridge.run_log_usage();
        Ok(serde_json::to_string(
            &json!({"calls": self.calls, "events": usage.events, "bytes": usage.bytes,
            "application_completion_validated": false, "scientific_validation": false}),
        )?)
    }
}

#[pymethods]
impl Bridge {
    #[new]
    fn new(path: PathBuf, configuration_json: &str) -> PyResult<Self> {
        Self::create(&path, configuration_json).map_err(input)
    }

    /// Dispatch a declared method. The callback consumes the recorded payload JSON.
    fn dispatch(
        &mut self,
        py: Python<'_>,
        method: &str,
        payload_json: &str,
        callback: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        self.active()?;
        if !self.configuration.methods.iter().any(|item| item == method) {
            return Err(input("undeclared application method"));
        }
        if self.calls >= self.configuration.max_calls {
            return Err(input("application call budget exhausted"));
        }
        if !callback.is_callable() {
            return Err(input("callback must be callable"));
        }
        let payload = object(payload_json).map_err(input)?;
        let call = self.calls + 1;
        let request = BridgeRequest {
            request_id: format!("application-{call}"),
            step: Some(call),
            timestamp_ns: elapsed(self.start).map_err(runtime)?,
            actor: Actor {
                actor_type: ActorType::Script,
                actor_id: self.configuration.actor_id.clone(),
                session_id: None,
            },
            method: ApplicationMethod(method.to_owned()),
            payload,
        };
        // After dispatch starts, every uncertain path keeps the adapter retired.
        self.retired = true;
        let start = self.start;
        let mut handler = PythonHandler {
            callback,
            error: None,
        };
        let result = self
            .bridge
            .as_mut()
            .ok_or_else(|| runtime("bridge is retired"))?
            .dispatch_with_clock(&request, &mut handler, || elapsed(start));
        self.calls = call;
        match result {
            Err(error) => {
                let error = runtime(error);
                if let Some(cause) = handler.error {
                    error.set_cause(py, Some(cause));
                }
                Err(error)
            }
            Ok(response) => {
                if let Some(error) = handler.error {
                    return Err(error);
                }
                if !response.ok {
                    return Err(runtime("application command failed"));
                }
                let encoded = serde_json::to_string(&response).map_err(runtime)?;
                self.retired = false;
                Ok(encoded)
            }
        }
    }

    /// Close a caller-completed application run, optionally binding a sibling file.
    #[pyo3(signature = (artifact=None))]
    fn finish(&mut self, artifact: Option<&str>) -> PyResult<String> {
        self.active()?;
        self.retired = true;
        let result = self.complete(artifact).map_err(runtime);
        self.bridge.take();
        result
    }

    /// Retain an incomplete prefix. This does not execute application cleanup.
    fn close(&mut self) -> PyResult<()> {
        if thread::current().id() != self.owner {
            return Err(runtime("bridge belongs to another thread"));
        }
        self.retired = true;
        self.bridge.take();
        Ok(())
    }
}

/// Inspect canonical events. Visitor effects are provisional until this returns.
#[pyfunction]
fn inspect_runlog(path: PathBuf, visit: &Bound<'_, PyAny>) -> PyResult<String> {
    if !visit.is_callable() {
        return Err(input("visitor must be callable"));
    }
    let limits = RunLogLimits::default()
        .with_max_file_bytes(MAX_LOG_BYTES)
        .with_max_line_bytes(2 * MAX_JSON + 4096)
        .with_max_events(3 * MAX_CALLS as usize + 4);
    let (mut file, initial) = storage::read_private(&path, MAX_LOG_BYTES).map_err(runtime)?;
    let inspection = inspect_event_stream(BufReader::new(&mut file), limits).map_err(runtime)?;
    if !inspection.validation.is_valid() {
        return Err(runtime("canonical run-log validation failed"));
    }
    file.seek(SeekFrom::Start(0)).map_err(runtime)?;
    for event in RunLogEventStream::new(BufReader::new(&mut file), limits).map_err(runtime)? {
        let event = event.map_err(runtime)?;
        visit.call1((serde_json::to_string(&event).map_err(runtime)?,))?;
    }
    storage::rejoin(&path, &file, &initial).map_err(runtime)?;
    serde_json::to_string(&json!({"events": inspection.validation.events,
        "hash_identities": inspection.hash_identities,
        "application_completion_validated": false, "scientific_validation": false}))
    .map_err(runtime)
}

/// Observe one bounded private sibling artifact without accepting application claims.
#[pyfunction]
fn artifact_identity(log_path: PathBuf, name: &str) -> PyResult<String> {
    let log = storage::new_path(&log_path).map_err(input)?;
    let path = storage::artifact_path(&log, name).map_err(input)?;
    let (bytes, sha256) = storage::artifact_digest(&path).map_err(runtime)?;
    serde_json::to_string(&json!({"bytes": bytes, "sha256": sha256})).map_err(runtime)
}

/// Compute the pinned canonical identity of a bounded application JSON object.
#[pyfunction]
fn hash_object(value_json: &str) -> PyResult<String> {
    canonical_json_hash_v2(&object(value_json).map_err(input)?).map_err(runtime)
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Bridge>()?;
    module.add_function(wrap_pyfunction!(inspect_runlog, module)?)?;
    module.add_function(wrap_pyfunction!(artifact_identity, module)?)?;
    module.add_function(wrap_pyfunction!(hash_object, module)?)?;
    Ok(())
}
