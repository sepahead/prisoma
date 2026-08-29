use std::collections::HashSet;
use std::io::{Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::canonical::{
    canonical_json, lower_hex, sha256_bytes, strict_json, to_value, MAX_SAFE_JSON_INTEGER,
};
use crate::contract::{
    FinishRequest, ObserveRequest, ObserverOutcome, ObserverResponse, PrepareRequest,
    RuntimeConfiguration, CONFIGURATION_SCHEMA_BYTES, CONFIGURATION_SCHEMA_ID, FINISH_OPERATION_ID,
    FINISH_REQUEST_SCHEMA_BYTES, FINISH_REQUEST_SCHEMA_ID, FINISH_RESPONSE_SCHEMA_BYTES,
    FINISH_RESPONSE_SCHEMA_ID, IPC_PROTOCOL, IPC_SCHEMA_BYTES, LAUNCH_ABI, MAX_CHANNELS,
    MAX_FRAME_BYTES, MAX_OPERATIONS_PER_GENERATION, MAX_REASON_BYTES,
    MAX_REJECTED_OPERATION_ATTEMPTS, MAX_STEPS, OBSERVE_OPERATION_ID, OBSERVE_REQUEST_SCHEMA_BYTES,
    OBSERVE_REQUEST_SCHEMA_ID, OBSERVE_RESPONSE_SCHEMA_BYTES, OBSERVE_RESPONSE_SCHEMA_ID,
    OPERATION_TIMEOUT_MS, PREPARE_OPERATION_ID, PREPARE_REQUEST_SCHEMA_BYTES,
    PREPARE_REQUEST_SCHEMA_ID, PREPARE_RESPONSE_SCHEMA_BYTES, PREPARE_RESPONSE_SCHEMA_ID, PROFILE,
};
use crate::observer::{ObserverError, ObserverRuntime};

const ABSOLUTE_MAX_FRAME_BYTES: usize = 1_048_576;
const HANDSHAKE_MIN_FRAME_BYTES: u64 = 1_024;
const CONFIGURATION_MAX_BYTES: usize = 4_096;
const FINISH_REQUEST_BYTES: usize = 32_768;
const FINISH_RESPONSE_BYTES: usize = 8_192;
const OBSERVE_REQUEST_BYTES: usize = 49_152;
const OBSERVE_RESPONSE_BYTES: usize = 8_192;
const PREPARE_REQUEST_BYTES: usize = 32_768;
const PREPARE_RESPONSE_BYTES: usize = 8_192;

const ENVELOPE_FIELDS: &[&str] = &[
    "body",
    "generation",
    "kind",
    "message_id",
    "protocol",
    "schema_version",
    "sender",
    "sequence",
];
const GENERATION_FIELDS: &[&str] = &["generation_id", "installation_id", "ordinal"];
const HANDSHAKE_BODY_FIELDS: &[&str] =
    &["challenge", "configuration", "identity", "max_frame_bytes"];
const IDENTITY_FIELDS: &[&str] = &[
    "configuration_canonical_sha256",
    "configuration_exact_sha256",
    "executable_sha256",
    "installation_id",
    "launch_abi",
    "manifest_canonical_sha256",
    "manifest_exact_sha256",
    "operation_roster_sha256",
    "package_lock_canonical_sha256",
    "package_lock_exact_sha256",
    "package_sha256",
    "profile",
    "schema_registry_sha256",
    "target_id",
];
const IDENTITY_DIGEST_FIELDS: &[&str] = &[
    "configuration_canonical_sha256",
    "configuration_exact_sha256",
    "executable_sha256",
    "manifest_canonical_sha256",
    "manifest_exact_sha256",
    "operation_roster_sha256",
    "package_lock_canonical_sha256",
    "package_lock_exact_sha256",
    "package_sha256",
    "schema_registry_sha256",
];
const CONFIGURATION_FIELDS: &[&str] = &["canonical_sha256", "document", "schema"];
const SCHEMA_REFERENCE_FIELDS: &[&str] = &["schema_id", "schema_sha256"];
const REQUEST_BODY_FIELDS: &[&str] = &[
    "bulk",
    "compute_grant",
    "control",
    "idempotency_key",
    "operation",
    "request_schema",
    "response_schema",
    "timeout_ms",
];
const OPERATION_IDENTITY_FIELDS: &[&str] = &["artifact_access", "class", "effect", "operation_id"];
const NONE_GRANT_FIELDS: &[&str] = &["mode"];
const BULK_FIELDS: &[&str] = &["inline", "references"];

/// Fatal inherited-pipe protocol error for one process generation.
#[derive(Debug, Error)]
#[error("managed observer rejected the generation: {reason}")]
pub struct ProtocolError {
    reason: &'static str,
}

impl ProtocolError {
    fn new(reason: &'static str) -> Self {
        Self { reason }
    }

    /// Return the bounded machine-readable failure reason.
    pub fn reason(&self) -> &'static str {
        self.reason
    }
}

type ProtocolResult<T> = Result<T, ProtocolError>;

/// Child-local summary of one inherited-pipe generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionReceipt {
    pub installation_id: String,
    pub generation_id: String,
    pub ordinal: u64,
    pub request_count: u64,
    pub response_count: u64,
    pub rejected_count: u64,
    pub clean_eof: bool,
    pub observer_state_cleared: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SchemaReference {
    schema_id: &'static str,
    schema_sha256: String,
}

impl SchemaReference {
    fn new(schema_id: &'static str, bytes: &'static [u8]) -> Self {
        Self {
            schema_id,
            schema_sha256: sha256_bytes(bytes),
        }
    }

    fn value(&self) -> Value {
        json!({
            "schema_id": self.schema_id,
            "schema_sha256": self.schema_sha256,
        })
    }

    fn matches(&self, value: &Value) -> bool {
        exact_object(value, SCHEMA_REFERENCE_FIELDS).is_ok_and(|source| {
            string_field(source, "schema_id") == Some(self.schema_id)
                && string_field(source, "schema_sha256") == Some(self.schema_sha256.as_str())
        })
    }
}

#[derive(Debug, Clone)]
struct OperationContract {
    operation_id: &'static str,
    request_schema: SchemaReference,
    response_schema: SchemaReference,
    max_request_bytes: usize,
    max_response_bytes: usize,
}

impl OperationContract {
    fn identity(&self) -> Value {
        json!({
            "operation_id": self.operation_id,
            "class": "observation",
            "effect": "none",
            "artifact_access": {"read": "none", "write": "none"},
        })
    }

    fn manifest_row(&self) -> Value {
        json!({
            "operation_id": self.operation_id,
            "class": "observation",
            "effect": "none",
            "artifact_access": {"read": "none", "write": "none"},
            "request_schema": self.request_schema.value(),
            "response_schema": self.response_schema.value(),
            "compute_grant": "none",
            "timeout_ms": OPERATION_TIMEOUT_MS,
            "max_cpu_time_ms": 0,
            "max_request_bytes": self.max_request_bytes,
            "max_response_bytes": self.max_response_bytes,
        })
    }
}

#[derive(Debug)]
struct HandshakeState {
    generation: Value,
    identity: Value,
    max_frame_bytes: usize,
}

fn operations() -> [OperationContract; 3] {
    [
        OperationContract {
            operation_id: FINISH_OPERATION_ID,
            request_schema: SchemaReference::new(
                FINISH_REQUEST_SCHEMA_ID,
                FINISH_REQUEST_SCHEMA_BYTES,
            ),
            response_schema: SchemaReference::new(
                FINISH_RESPONSE_SCHEMA_ID,
                FINISH_RESPONSE_SCHEMA_BYTES,
            ),
            max_request_bytes: FINISH_REQUEST_BYTES,
            max_response_bytes: FINISH_RESPONSE_BYTES,
        },
        OperationContract {
            operation_id: OBSERVE_OPERATION_ID,
            request_schema: SchemaReference::new(
                OBSERVE_REQUEST_SCHEMA_ID,
                OBSERVE_REQUEST_SCHEMA_BYTES,
            ),
            response_schema: SchemaReference::new(
                OBSERVE_RESPONSE_SCHEMA_ID,
                OBSERVE_RESPONSE_SCHEMA_BYTES,
            ),
            max_request_bytes: OBSERVE_REQUEST_BYTES,
            max_response_bytes: OBSERVE_RESPONSE_BYTES,
        },
        OperationContract {
            operation_id: PREPARE_OPERATION_ID,
            request_schema: SchemaReference::new(
                PREPARE_REQUEST_SCHEMA_ID,
                PREPARE_REQUEST_SCHEMA_BYTES,
            ),
            response_schema: SchemaReference::new(
                PREPARE_RESPONSE_SCHEMA_ID,
                PREPARE_RESPONSE_SCHEMA_BYTES,
            ),
            max_request_bytes: PREPARE_REQUEST_BYTES,
            max_response_bytes: PREPARE_RESPONSE_BYTES,
        },
    ]
}

/// Return the exact operation-roster digest compiled into this runtime.
///
/// # Errors
///
/// Returns [`ProtocolError`] if canonical serialization fails.
pub fn operation_roster_sha256() -> Result<String, ProtocolError> {
    let value = Value::Array(
        operations()
            .iter()
            .map(OperationContract::manifest_row)
            .collect(),
    );
    let bytes = canonical_json(&value).map_err(|_| protocol_error("runtime.operation-roster"))?;
    let mut digest = Sha256::new();
    digest.update(b"engram-managed-operation-roster-v1\0");
    digest.update(bytes);
    Ok(lower_hex(&digest.finalize()))
}

/// Return the exact generic IPC schema digest compiled into this runtime.
pub fn ipc_schema_sha256() -> String {
    sha256_bytes(IPC_SCHEMA_BYTES)
}

fn protocol_error(reason: &'static str) -> ProtocolError {
    ProtocolError::new(reason)
}

fn exact_object<'a>(value: &'a Value, fields: &[&str]) -> ProtocolResult<&'a Map<String, Value>> {
    let source = value
        .as_object()
        .ok_or_else(|| protocol_error("protocol.shape"))?;
    if source.len() != fields.len() || !fields.iter().all(|field| source.contains_key(*field)) {
        return Err(protocol_error("protocol.shape"));
    }
    Ok(source)
}

fn string_field<'a>(source: &'a Map<String, Value>, field: &str) -> Option<&'a str> {
    source.get(field).and_then(Value::as_str)
}

fn u64_field(source: &Map<String, Value>, field: &str) -> Option<u64> {
    source.get(field).and_then(Value::as_u64)
}

fn bool_field(source: &Map<String, Value>, field: &str) -> Option<bool> {
    source.get(field).and_then(Value::as_bool)
}

fn valid_component(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    let bytes = value.as_bytes();
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    let mut separator_seen = false;
    let mut prior_separator = false;
    for byte in bytes {
        let separator = matches!(byte, b'.' | b'_' | b'-');
        if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || separator)
            || (separator && prior_separator)
        {
            return false;
        }
        separator_seen |= separator;
        prior_separator = separator;
    }
    separator_seen && !prior_separator
}

fn valid_control_key(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    value.len() <= 64
        && first.is_ascii_alphabetic()
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

fn valid_control(value: &Value) -> bool {
    let Some(source) = value.as_object() else {
        return false;
    };
    source.len() <= 32
        && source
            .iter()
            .all(|(key, child)| valid_control_key(key) && valid_control_value(child))
}

fn valid_control_value(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.len() <= 64 && values.iter().all(valid_control_scalar),
        scalar => valid_control_scalar(scalar),
    }
}

fn valid_control_scalar(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(_) => true,
        Value::Number(number) => {
            number
                .as_i64()
                .is_some_and(|value| value.unsigned_abs() <= MAX_SAFE_JSON_INTEGER)
                || number
                    .as_u64()
                    .is_some_and(|value| value <= MAX_SAFE_JSON_INTEGER)
                || number
                    .as_f64()
                    .is_some_and(|value| value.is_finite() && value.abs() <= 1.0e300)
        }
        Value::String(value) => value.len() <= 1_024,
        Value::Array(_) | Value::Object(_) => false,
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_prefixed_hex(value: &str, prefix: &str, hex_length: usize) -> bool {
    value.len() == prefix.len() + hex_length
        && value.starts_with(prefix)
        && value[prefix.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn random_prefixed_hex(prefix: &str, byte_length: usize) -> ProtocolResult<String> {
    let mut bytes = vec![0_u8; byte_length];
    getrandom::fill(&mut bytes).map_err(|_| protocol_error("runtime.random"))?;
    Ok(format!("{prefix}{}", lower_hex(&bytes)))
}

fn mint_message_id(seen: &HashSet<String>) -> ProtocolResult<String> {
    for _ in 0..16 {
        let candidate = random_prefixed_hex("msg_", 16)?;
        if !seen.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(protocol_error("runtime.message-id"))
}

fn read_exact_or_reason<R: Read>(
    input: &mut R,
    buffer: &mut [u8],
    eof_reason: &'static str,
) -> ProtocolResult<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        let count = input
            .read(&mut buffer[offset..])
            .map_err(|_| protocol_error("frame.input-read"))?;
        if count == 0 {
            return Err(protocol_error(eof_reason));
        }
        offset += count;
    }
    Ok(())
}

fn read_frame<R: Read>(
    input: &mut R,
    max_payload_bytes: usize,
    allow_clean_eof: bool,
) -> ProtocolResult<Option<(Value, usize)>> {
    if !(1..=ABSOLUTE_MAX_FRAME_BYTES).contains(&max_payload_bytes) {
        return Err(protocol_error("frame.bound"));
    }
    let mut prefix = [0_u8; 4];
    let first = input
        .read(&mut prefix[..1])
        .map_err(|_| protocol_error("frame.input-read"))?;
    if first == 0 {
        return if allow_clean_eof {
            Ok(None)
        } else {
            Err(protocol_error("frame.eof"))
        };
    }
    read_exact_or_reason(input, &mut prefix[1..], "frame.truncated-prefix")?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 || length > max_payload_bytes {
        return Err(protocol_error("frame.length"));
    }
    let mut payload = vec![0_u8; length];
    read_exact_or_reason(input, &mut payload, "frame.truncated")?;
    let value = strict_json(&payload).map_err(|_| protocol_error("json.malformed"))?;
    if !value.is_object() {
        return Err(protocol_error("frame.object"));
    }
    Ok(Some((value, length)))
}

fn write_frame<W: Write>(
    output: &mut W,
    value: &Value,
    max_payload_bytes: usize,
) -> ProtocolResult<usize> {
    let payload = canonical_json(value).map_err(|_| protocol_error("json.canonicalization"))?;
    if payload.is_empty() || payload.len() > max_payload_bytes || payload.len() > u32::MAX as usize
    {
        return Err(protocol_error("frame.output-bound"));
    }
    output
        .write_all(&(payload.len() as u32).to_be_bytes())
        .map_err(|_| protocol_error("frame.output-write"))?;
    output
        .write_all(&payload)
        .map_err(|_| protocol_error("frame.output-write"))?;
    output
        .flush()
        .map_err(|_| protocol_error("frame.output-write"))?;
    Ok(payload.len())
}

fn deserialize_control<T: DeserializeOwned>(value: &Value) -> ProtocolResult<T> {
    if !valid_control(value) {
        return Err(protocol_error("protocol.control"));
    }
    serde_json::from_value(value.clone()).map_err(|_| protocol_error("runtime.request-schema"))
}

fn response_value<T: Serialize>(response: &T) -> ProtocolResult<Value> {
    let value = to_value(response).map_err(|_| protocol_error("runtime.response-schema"))?;
    if !valid_control(&value) {
        return Err(protocol_error("runtime.response-schema"));
    }
    Ok(value)
}

fn accept_generation(value: &Value) -> ProtocolResult<()> {
    let source = exact_object(value, GENERATION_FIELDS)?;
    if !string_field(source, "installation_id")
        .is_some_and(|value| valid_prefixed_hex(value, "inst_", 64))
        || !string_field(source, "generation_id")
            .is_some_and(|value| valid_prefixed_hex(value, "gen_", 64))
        || !u64_field(source, "ordinal")
            .is_some_and(|value| (1..=MAX_SAFE_JSON_INTEGER).contains(&value))
    {
        return Err(protocol_error("protocol.generation"));
    }
    Ok(())
}

fn accept_identity(value: &Value) -> ProtocolResult<()> {
    let source = exact_object(value, IDENTITY_FIELDS)?;
    if IDENTITY_DIGEST_FIELDS
        .iter()
        .any(|field| !string_field(source, field).is_some_and(valid_sha256))
    {
        return Err(protocol_error("protocol.digest"));
    }
    if !string_field(source, "target_id").is_some_and(valid_component)
        || string_field(source, "profile") != Some(PROFILE)
        || string_field(source, "launch_abi") != Some(LAUNCH_ABI)
        || string_field(source, "operation_roster_sha256")
            != Some(operation_roster_sha256()?.as_str())
        || !string_field(source, "installation_id")
            .is_some_and(|value| valid_prefixed_hex(value, "inst_", 64))
    {
        return Err(protocol_error("protocol.identity"));
    }
    Ok(())
}

fn accept_handshake(value: &Value) -> ProtocolResult<HandshakeState> {
    let source = exact_object(value, ENVELOPE_FIELDS)?;
    if string_field(source, "schema_version") != Some("1.0")
        || string_field(source, "protocol") != Some(IPC_PROTOCOL)
        || string_field(source, "kind") != Some("host.handshake")
        || string_field(source, "sender") != Some("host")
        || u64_field(source, "sequence") != Some(0)
        || !string_field(source, "message_id")
            .is_some_and(|value| valid_prefixed_hex(value, "msg_", 32))
    {
        return Err(protocol_error("protocol.handshake-envelope"));
    }
    let generation = source
        .get("generation")
        .ok_or_else(|| protocol_error("protocol.shape"))?;
    accept_generation(generation)?;
    let body = exact_object(
        source
            .get("body")
            .ok_or_else(|| protocol_error("protocol.shape"))?,
        HANDSHAKE_BODY_FIELDS,
    )?;
    if !string_field(body, "challenge").is_some_and(|value| valid_prefixed_hex(value, "chal_", 64))
    {
        return Err(protocol_error("protocol.identifier"));
    }
    let identity = body
        .get("identity")
        .ok_or_else(|| protocol_error("protocol.shape"))?;
    accept_identity(identity)?;
    let generation_source = generation
        .as_object()
        .ok_or_else(|| protocol_error("protocol.shape"))?;
    let identity_source = identity
        .as_object()
        .ok_or_else(|| protocol_error("protocol.shape"))?;
    if string_field(generation_source, "installation_id")
        != string_field(identity_source, "installation_id")
    {
        return Err(protocol_error("protocol.installation-join"));
    }
    let configuration = exact_object(
        body.get("configuration")
            .ok_or_else(|| protocol_error("protocol.shape"))?,
        CONFIGURATION_FIELDS,
    )?;
    let schema = SchemaReference::new(CONFIGURATION_SCHEMA_ID, CONFIGURATION_SCHEMA_BYTES);
    if !configuration
        .get("schema")
        .is_some_and(|value| schema.matches(value))
    {
        return Err(protocol_error("protocol.configuration-schema"));
    }
    let document = configuration
        .get("document")
        .ok_or_else(|| protocol_error("protocol.shape"))?;
    let canonical_document =
        canonical_json(document).map_err(|_| protocol_error("protocol.configuration-payload"))?;
    if canonical_document.len() > CONFIGURATION_MAX_BYTES {
        return Err(protocol_error("protocol.configuration-bound"));
    }
    let canonical_digest = sha256_bytes(&canonical_document);
    if string_field(configuration, "canonical_sha256") != Some(canonical_digest.as_str())
        || string_field(identity_source, "configuration_canonical_sha256")
            != Some(canonical_digest.as_str())
    {
        return Err(protocol_error("protocol.configuration-digest"));
    }
    let typed_configuration: RuntimeConfiguration = serde_json::from_value(document.clone())
        .map_err(|_| protocol_error("protocol.configuration-payload"))?;
    if !typed_configuration.validate() {
        return Err(protocol_error("protocol.configuration-payload"));
    }
    let max_frame_bytes = u64_field(body, "max_frame_bytes")
        .filter(|value| (HANDSHAKE_MIN_FRAME_BYTES..=MAX_FRAME_BYTES as u64).contains(value))
        .ok_or_else(|| protocol_error("frame.bound"))? as usize;
    Ok(HandshakeState {
        generation: generation.clone(),
        identity: identity.clone(),
        max_frame_bytes,
    })
}

fn runtime_handshake(
    host: &Value,
    state: &HandshakeState,
    message_id: String,
) -> ProtocolResult<Value> {
    let source = host
        .as_object()
        .ok_or_else(|| protocol_error("protocol.shape"))?;
    let body = source
        .get("body")
        .and_then(Value::as_object)
        .ok_or_else(|| protocol_error("protocol.shape"))?;
    let challenge =
        string_field(body, "challenge").ok_or_else(|| protocol_error("protocol.shape"))?;
    Ok(json!({
        "schema_version": "1.0",
        "protocol": IPC_PROTOCOL,
        "kind": "runtime.handshake",
        "sender": "runtime",
        "generation": state.generation,
        "sequence": 0,
        "message_id": message_id,
        "body": {
            "host_challenge": challenge,
            "runtime_nonce": random_prefixed_hex("nonce_", 32)?,
            "identity": state.identity,
            "ready_claim": false,
        },
    }))
}

#[derive(Debug)]
struct AcceptedRequest {
    sequence: u64,
    message_id: String,
    idempotency_key: String,
    operation: OperationContract,
    control: Value,
}

fn accept_request(
    value: &Value,
    payload_length: usize,
    state: &HandshakeState,
    expected_sequence: u64,
    seen_messages: &HashSet<String>,
    seen_idempotency: &HashSet<String>,
) -> ProtocolResult<AcceptedRequest> {
    let source = exact_object(value, ENVELOPE_FIELDS)?;
    if string_field(source, "schema_version") != Some("1.0")
        || string_field(source, "protocol") != Some(IPC_PROTOCOL)
        || string_field(source, "kind") != Some("operation.request")
        || string_field(source, "sender") != Some("host")
        || u64_field(source, "sequence") != Some(expected_sequence)
        || source.get("generation") != Some(&state.generation)
    {
        return Err(protocol_error("protocol.request-envelope"));
    }
    let message_id = string_field(source, "message_id")
        .filter(|value| valid_prefixed_hex(value, "msg_", 32))
        .ok_or_else(|| protocol_error("protocol.identifier"))?
        .to_owned();
    if seen_messages.contains(&message_id) {
        return Err(protocol_error("protocol.message-replay"));
    }
    let body = exact_object(
        source
            .get("body")
            .ok_or_else(|| protocol_error("protocol.shape"))?,
        REQUEST_BODY_FIELDS,
    )?;
    let idempotency_key = string_field(body, "idempotency_key")
        .filter(|value| valid_prefixed_hex(value, "idem_", 64))
        .ok_or_else(|| protocol_error("protocol.identifier"))?
        .to_owned();
    if seen_idempotency.contains(&idempotency_key) {
        return Err(protocol_error("protocol.idempotency-replay"));
    }
    let operation_value = body
        .get("operation")
        .ok_or_else(|| protocol_error("protocol.shape"))?;
    exact_object(operation_value, OPERATION_IDENTITY_FIELDS)?;
    let operation_id = operation_value
        .as_object()
        .and_then(|operation| string_field(operation, "operation_id"))
        .ok_or_else(|| protocol_error("protocol.operation"))?;
    let operation = operations()
        .into_iter()
        .find(|candidate| candidate.operation_id == operation_id)
        .ok_or_else(|| protocol_error("protocol.operation"))?;
    if payload_length > operation.max_request_bytes {
        return Err(protocol_error("protocol.request-bytes"));
    }
    if operation_value != &operation.identity()
        || !body
            .get("request_schema")
            .is_some_and(|value| operation.request_schema.matches(value))
        || !body
            .get("response_schema")
            .is_some_and(|value| operation.response_schema.matches(value))
        || u64_field(body, "timeout_ms") != Some(OPERATION_TIMEOUT_MS)
    {
        return Err(protocol_error("protocol.operation-contract"));
    }
    let grant = exact_object(
        body.get("compute_grant")
            .ok_or_else(|| protocol_error("protocol.shape"))?,
        NONE_GRANT_FIELDS,
    )?;
    if string_field(grant, "mode") != Some("none") {
        return Err(protocol_error("protocol.observation-grant"));
    }
    let bulk = exact_object(
        body.get("bulk")
            .ok_or_else(|| protocol_error("protocol.shape"))?,
        BULK_FIELDS,
    )?;
    if bool_field(bulk, "inline") != Some(false)
        || bulk
            .get("references")
            .and_then(Value::as_array)
            .is_none_or(|items| !items.is_empty())
    {
        return Err(protocol_error("protocol.bulk"));
    }
    let control = body
        .get("control")
        .filter(|value| valid_control(value))
        .ok_or_else(|| protocol_error("protocol.control"))?
        .clone();
    Ok(AcceptedRequest {
        sequence: expected_sequence,
        message_id,
        idempotency_key,
        operation,
        control,
    })
}

fn dispatch(
    runtime: &mut ObserverRuntime,
    request: &AcceptedRequest,
) -> ProtocolResult<ObserverResponse> {
    match request.operation.operation_id {
        PREPARE_OPERATION_ID => {
            let typed: PrepareRequest = deserialize_control(&request.control)?;
            let study_run_id = typed.study_run_id.clone();
            let operation_request = typed.clone();
            contain_operation(
                runtime,
                PREPARE_RESPONSE_SCHEMA_ID,
                &typed,
                &study_run_id,
                0,
                move |runtime| runtime.prepare(operation_request),
            )
        }
        OBSERVE_OPERATION_ID => {
            let typed: ObserveRequest = deserialize_control(&request.control)?;
            let study_run_id = typed.study_run_id.clone();
            let step_index = typed.step_index;
            let operation_request = typed.clone();
            contain_operation(
                runtime,
                OBSERVE_RESPONSE_SCHEMA_ID,
                &typed,
                &study_run_id,
                step_index,
                move |runtime| runtime.observe(operation_request),
            )
        }
        FINISH_OPERATION_ID => {
            let typed: FinishRequest = deserialize_control(&request.control)?;
            let study_run_id = typed.study_run_id.clone();
            let step_index = typed.step_count;
            let operation_request = typed.clone();
            contain_operation(
                runtime,
                FINISH_RESPONSE_SCHEMA_ID,
                &typed,
                &study_run_id,
                step_index,
                move |runtime| runtime.finish(operation_request),
            )
        }
        _ => Err(protocol_error("protocol.operation")),
    }
}

fn contain_operation<T, F>(
    runtime: &mut ObserverRuntime,
    response_schema: &str,
    request: &T,
    study_run_id: &str,
    step_index: u64,
    operation: F,
) -> ProtocolResult<ObserverResponse>
where
    T: Serialize,
    F: FnOnce(&mut ObserverRuntime) -> Result<ObserverResponse, ObserverError>,
{
    match catch_unwind(AssertUnwindSafe(|| operation(runtime))) {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(error)) => operation_error_response(
            runtime,
            response_schema,
            request,
            study_run_id,
            step_index,
            error,
        ),
        Err(_) => operation_error_response(
            runtime,
            response_schema,
            request,
            study_run_id,
            step_index,
            ObserverError::InternalPanicContained,
        ),
    }
}

fn operation_error_response<T: Serialize>(
    runtime: &mut ObserverRuntime,
    response_schema: &str,
    request: &T,
    study_run_id: &str,
    step_index: u64,
    error: ObserverError,
) -> ProtocolResult<ObserverResponse> {
    if error.outcome() == ObserverOutcome::Failed {
        runtime.clear();
    }
    runtime
        .error_response(response_schema, request, study_run_id, step_index, error)
        .map_err(|_| protocol_error("runtime.response-schema"))
}

fn validate_response(response: &ObserverResponse, operation: &OperationContract) -> bool {
    let expected_schema = if operation.operation_id == PREPARE_OPERATION_ID {
        PREPARE_RESPONSE_SCHEMA_ID
    } else if operation.operation_id == OBSERVE_OPERATION_ID {
        OBSERVE_RESPONSE_SCHEMA_ID
    } else {
        FINISH_RESPONSE_SCHEMA_ID
    };
    response.schema_version == expected_schema
        && response.authority == "read-only-observer"
        && response.roster_authority == "host-declared-projection"
        && !response.source_roster_authenticated
        && !response.study_run_id.is_empty()
        && response.study_run_id.len() <= 128
        && response.step_index <= MAX_STEPS
        && response.channel_count <= MAX_CHANNELS as u64
        && response.fault_count <= MAX_CHANNELS as u64
        && response.cumulative_fault_count <= MAX_STEPS * MAX_CHANNELS as u64
        && response.reason.len() <= MAX_REASON_BYTES
        && [
            &response.prior_observer_state_sha256,
            &response.observer_state_sha256,
            &response.request_sha256,
            &response.observer_receipt_sha256,
            &response.observer_transcript_sha256,
        ]
        .iter()
        .all(|value| valid_sha256(value))
        && response
            .source_receipt_sha256
            .as_deref()
            .is_none_or(valid_sha256)
        && (response.outcome != ObserverOutcome::Failed
            || (response.state_cleared && !response.terminal))
        && response.descriptive_only
        && !response.agent_bridge_command
        && !response.physical_actuation
        && !response.ncp_used
        && !response.pid_result
        && !response.source_durable_evidence_verified
        && !response.scientific_authority
        && !response.is_paper_local_evidence
        && !response.calibrated_posterior
}

/// Serve one complete Host API 2 inherited-pipe generation.
///
/// The function never starts, stops, signals, or restarts a process. The host
/// manager owns lifecycle and deadline enforcement.
///
/// # Errors
///
/// Returns [`ProtocolError`] after framing, replay, identity, schema, bound,
/// output, or contained-panic failure.
pub fn serve_managed_runtime<R: Read, W: Write>(
    input: &mut R,
    output: &mut W,
) -> Result<RuntimeSessionReceipt, ProtocolError> {
    match catch_unwind(AssertUnwindSafe(|| serve_generation(input, output))) {
        Ok(result) => result,
        Err(_) => Err(protocol_error("runtime.internal-panic-contained")),
    }
}

fn serve_generation<R: Read, W: Write>(
    input: &mut R,
    output: &mut W,
) -> ProtocolResult<RuntimeSessionReceipt> {
    serve_generation_with_dispatch(input, output, dispatch)
}

fn serve_generation_with_dispatch<R, W, D>(
    input: &mut R,
    output: &mut W,
    mut dispatch_request: D,
) -> ProtocolResult<RuntimeSessionReceipt>
where
    R: Read,
    W: Write,
    D: FnMut(&mut ObserverRuntime, &AcceptedRequest) -> ProtocolResult<ObserverResponse>,
{
    let (host_handshake, _) = read_frame(input, ABSOLUTE_MAX_FRAME_BYTES, false)?
        .ok_or_else(|| protocol_error("frame.eof"))?;
    let state = accept_handshake(&host_handshake)?;
    let host_message_id = host_handshake
        .get("message_id")
        .and_then(Value::as_str)
        .ok_or_else(|| protocol_error("protocol.identifier"))?
        .to_owned();
    let mut seen_messages = HashSet::from([host_message_id]);
    let handshake_message_id = mint_message_id(&seen_messages)?;
    let response = runtime_handshake(&host_handshake, &state, handshake_message_id.clone())?;
    write_frame(output, &response, state.max_frame_bytes)?;
    seen_messages.insert(handshake_message_id);

    let generation = state
        .generation
        .as_object()
        .ok_or_else(|| protocol_error("protocol.generation"))?;
    let installation_id = string_field(generation, "installation_id")
        .ok_or_else(|| protocol_error("protocol.generation"))?
        .to_owned();
    let generation_id = string_field(generation, "generation_id")
        .ok_or_else(|| protocol_error("protocol.generation"))?
        .to_owned();
    let ordinal =
        u64_field(generation, "ordinal").ok_or_else(|| protocol_error("protocol.generation"))?;
    let mut runtime = ObserverRuntime::new();
    let mut seen_idempotency = HashSet::new();
    let mut request_count = 0_u64;
    let mut response_count = 0_u64;
    let mut rejected_count = 0_u64;

    loop {
        let Some((request_value, payload_length)) = read_frame(input, state.max_frame_bytes, true)?
        else {
            runtime.clear();
            return Ok(RuntimeSessionReceipt {
                installation_id,
                generation_id,
                ordinal,
                request_count,
                response_count,
                rejected_count,
                clean_eof: true,
                observer_state_cleared: runtime.is_cleared(),
            });
        };
        request_count = request_count
            .checked_add(1)
            .ok_or_else(|| protocol_error("runtime.operation-count"))?;
        if request_count > MAX_OPERATIONS_PER_GENERATION {
            return Err(protocol_error("runtime.operation-count"));
        }
        let accepted = accept_request(
            &request_value,
            payload_length,
            &state,
            request_count,
            &seen_messages,
            &seen_idempotency,
        )?;
        let domain_response = dispatch_request(&mut runtime, &accepted)?;
        let fail_stop = match domain_response.outcome {
            ObserverOutcome::Succeeded => false,
            ObserverOutcome::Rejected => {
                rejected_count = rejected_count
                    .checked_add(1)
                    .ok_or_else(|| protocol_error("runtime.rejection-count"))?;
                if rejected_count > MAX_REJECTED_OPERATION_ATTEMPTS {
                    return Err(protocol_error("runtime.rejection-count"));
                }
                false
            }
            ObserverOutcome::Failed => {
                runtime.clear();
                true
            }
        };
        if !validate_response(&domain_response, &accepted.operation) {
            return Err(protocol_error("runtime.response-schema"));
        }
        let control = response_value(&domain_response)?;
        let control_bytes =
            canonical_json(&control).map_err(|_| protocol_error("runtime.response-schema"))?;
        if control_bytes.len() > accepted.operation.max_response_bytes {
            return Err(protocol_error("runtime.response-bound"));
        }
        let response_message_id = mint_message_id(&seen_messages)?;
        let response = json!({
            "schema_version": "1.0",
            "protocol": IPC_PROTOCOL,
            "kind": "operation.response",
            "sender": "runtime",
            "generation": state.generation,
            "sequence": accepted.sequence,
            "message_id": response_message_id,
            "body": {
                "request_message_id": accepted.message_id,
                "idempotency_key": accepted.idempotency_key,
                "operation": accepted.operation.identity(),
                "response_schema": accepted.operation.response_schema.value(),
                "status": domain_response.outcome.ipc_status(),
                "control": control,
                "bulk": {"inline": false, "references": []},
            },
        });
        write_frame(
            output,
            &response,
            state
                .max_frame_bytes
                .min(accepted.operation.max_response_bytes),
        )?;
        response_count = response_count
            .checked_add(1)
            .ok_or_else(|| protocol_error("runtime.operation-count"))?;
        seen_messages.insert(accepted.message_id);
        seen_messages.insert(response_message_id);
        seen_idempotency.insert(accepted.idempotency_key);
        if fail_stop {
            return Err(protocol_error("runtime.operation-failed"));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::io::{Cursor, Read};

    use super::*;
    use crate::observer::{
        closed_loop_step_id, observer_response_receipt_sha256, source_step_receipt_sha256,
        SourceStepReceipt,
    };

    fn configuration() -> Value {
        json!({
            "schema_version": CONFIGURATION_SCHEMA_ID,
            "max_channels": 64,
            "max_steps": 1024,
            "max_cleanup_receipts": 2,
            "max_reason_bytes": 256,
        })
    }

    #[test]
    fn control_admits_fixed_scalar_arrays_and_rejects_nested_values() {
        assert!(valid_control(&json!({
            "schema_version": "prisoma.observer.finish-request.v1",
            "runtime_lifecycle_values": ["engram.closed-loop-runtime-lifecycle-binding.v1", true, null],
        })));
        assert!(!valid_control(&json!({
            "runtime_lifecycle_values": {"authority": false},
        })));
        assert!(!valid_control(&json!({
            "runtime_lifecycle_values": [{"authority": false}],
        })));
    }

    fn generation() -> Value {
        json!({
            "installation_id": format!("inst_{}", "a".repeat(64)),
            "generation_id": format!("gen_{}", "b".repeat(64)),
            "ordinal": 1,
        })
    }

    fn handshake() -> Value {
        let document = configuration();
        let configuration_digest = sha256_bytes(&canonical_json(&document).expect("configuration"));
        let operation_digest = operation_roster_sha256().expect("roster");
        let installation_id = format!("inst_{}", "a".repeat(64));
        json!({
            "schema_version": "1.0",
            "protocol": IPC_PROTOCOL,
            "kind": "host.handshake",
            "sender": "host",
            "generation": generation(),
            "sequence": 0,
            "message_id": format!("msg_{}", "1".repeat(32)),
            "body": {
                "challenge": format!("chal_{}", "c".repeat(64)),
                "identity": {
                    "manifest_exact_sha256": "1".repeat(64),
                    "manifest_canonical_sha256": "2".repeat(64),
                    "package_lock_exact_sha256": "3".repeat(64),
                    "package_lock_canonical_sha256": "4".repeat(64),
                    "package_sha256": "5".repeat(64),
                    "executable_sha256": "6".repeat(64),
                    "configuration_exact_sha256": configuration_digest,
                    "configuration_canonical_sha256": configuration_digest,
                    "target_id": "macos-aarch64-darwin",
                    "profile": PROFILE,
                    "launch_abi": LAUNCH_ABI,
                    "operation_roster_sha256": operation_digest,
                    "schema_registry_sha256": "7".repeat(64),
                    "installation_id": installation_id,
                },
                "configuration": {
                    "schema": SchemaReference::new(CONFIGURATION_SCHEMA_ID, CONFIGURATION_SCHEMA_BYTES).value(),
                    "canonical_sha256": configuration_digest,
                    "document": document,
                },
                "max_frame_bytes": 65536,
            },
        })
    }

    fn wire(values: &[Value]) -> Vec<u8> {
        let mut wire = Vec::new();
        for value in values {
            write_frame(&mut wire, value, ABSOLUTE_MAX_FRAME_BYTES).expect("frame");
        }
        wire
    }

    fn output_frames(bytes: &[u8]) -> Vec<Value> {
        let mut input = Cursor::new(bytes);
        let mut frames = Vec::new();
        while let Some((value, _)) =
            read_frame(&mut input, ABSOLUTE_MAX_FRAME_BYTES, true).expect("output frame")
        {
            frames.push(value);
        }
        frames
    }

    fn operation_request(
        operation: &OperationContract,
        sequence: u64,
        marker: char,
        control: Value,
    ) -> Value {
        json!({
            "schema_version": "1.0",
            "protocol": IPC_PROTOCOL,
            "kind": "operation.request",
            "sender": "host",
            "generation": generation(),
            "sequence": sequence,
            "message_id": format!("msg_{}", marker.to_string().repeat(32)),
            "body": {
                "idempotency_key": format!("idem_{}", marker.to_string().repeat(64)),
                "operation": operation.identity(),
                "request_schema": operation.request_schema.value(),
                "response_schema": operation.response_schema.value(),
                "compute_grant": {"mode": "none"},
                "timeout_ms": OPERATION_TIMEOUT_MS,
                "control": control,
                "bulk": {"inline": false, "references": []},
            },
        })
    }

    fn prepare_control() -> Value {
        json!({
            "schema_version": PREPARE_REQUEST_SCHEMA_ID,
            "study_run_id": "study-run-01",
            "study_definition_sha256": "1".repeat(64),
            "closed_loop_definition_sha256": "2".repeat(64),
            "runtime_binding_sha256": "3".repeat(64),
            "runtime_adapter_configuration_sha256": "8".repeat(64),
            "neural_provider_identity_sha256": "4".repeat(64),
            "channel_ids": ["channel-01", "channel-02", "channel-03"],
            "subject_ids": ["drone-01", "drone-02", "drone-03"],
            "planned_step_count": 1,
            "max_steps": 8,
        })
    }

    fn maximum_prepare_control() -> Value {
        let mut control = prepare_control();
        control["study_run_id"] = json!("r".repeat(128));
        control["channel_ids"] = json!((0..MAX_CHANNELS)
            .map(|index| format!("c{index:02}-{}", "a".repeat(124)))
            .collect::<Vec<_>>());
        control["subject_ids"] = json!((0..MAX_CHANNELS)
            .map(|index| format!("s{index:02}-{}", "b".repeat(124)))
            .collect::<Vec<_>>());
        control
    }

    fn maximum_observe_control() -> Value {
        let study_run_id = "r".repeat(128);
        let mut receipt = SourceStepReceipt {
            schema_version: "engram.extension-closed-loop-step-receipt.v2".to_owned(),
            study_run_id: study_run_id.clone(),
            step_index: 1,
            step_id: closed_loop_step_id(&study_run_id, 1).expect("step id"),
            input_snapshot_sha256: "1".repeat(64),
            neural_request_sha256: "2".repeat(64),
            neural_result_sha256: "3".repeat(64),
            provider_execution_scope: "nest-exact-step-readback".to_owned(),
            provider_execution_sha256: "4".repeat(64),
            admitted_action_sha256: "5".repeat(64),
            runtime_request_sha256: "6".repeat(64),
            output_snapshot_sha256: "7".repeat(64),
            fault_codes: vec!["\\".repeat(crate::contract::MAX_FAULT_CODE_BYTES); MAX_CHANNELS],
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = source_step_receipt_sha256(&receipt).expect("source step receipt");
        json!({
            "schema_version": OBSERVE_REQUEST_SCHEMA_ID,
            "study_run_id": receipt.study_run_id,
            "step_index": receipt.step_index,
            "step_id": receipt.step_id,
            "input_snapshot_sha256": receipt.input_snapshot_sha256,
            "neural_request_sha256": receipt.neural_request_sha256,
            "neural_result_sha256": receipt.neural_result_sha256,
            "provider_execution_scope": receipt.provider_execution_scope,
            "provider_execution_sha256": receipt.provider_execution_sha256,
            "admitted_action_sha256": receipt.admitted_action_sha256,
            "runtime_request_sha256": receipt.runtime_request_sha256,
            "output_snapshot_sha256": receipt.output_snapshot_sha256,
            "fault_codes": receipt.fault_codes,
            "source_receipt_sha256": receipt.receipt_sha256,
        })
    }

    #[test]
    fn declared_request_bounds_contain_maximum_runtime_accepted_envelopes() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let observe = operations()
            .into_iter()
            .find(|operation| operation.operation_id == OBSERVE_OPERATION_ID)
            .expect("observe operation");
        let prepare_request = operation_request(&prepare, 1, '2', maximum_prepare_control());
        let observe_request = operation_request(&observe, 2, '3', maximum_observe_control());
        let prepare_bytes = canonical_json(&prepare_request)
            .expect("prepare envelope")
            .len();
        let observe_bytes = canonical_json(&observe_request)
            .expect("observe envelope")
            .len();
        assert!(prepare_bytes > 8_192 && prepare_bytes <= PREPARE_REQUEST_BYTES);
        assert!(observe_bytes > 16_384 && observe_bytes <= OBSERVE_REQUEST_BYTES);

        let mut input = Cursor::new(wire(&[handshake(), prepare_request, observe_request]));
        let mut output = Vec::new();
        let receipt = serve_managed_runtime(&mut input, &mut output)
            .expect("maximum semantic requests fit their full-envelope bounds");
        let frames = output_frames(&output);
        assert_eq!((receipt.request_count, receipt.response_count), (2, 2));
        assert_eq!(frames[1]["body"]["status"], "succeeded");
        assert_eq!(frames[2]["body"]["status"], "succeeded");

        let state = accept_handshake(&handshake()).expect("handshake state");
        for (operation, control) in [
            (prepare, maximum_prepare_control()),
            (observe, maximum_observe_control()),
        ] {
            let request = operation_request(&operation, 1, '4', control);
            let error = accept_request(
                &request,
                operation.max_request_bytes + 1,
                &state,
                1,
                &HashSet::new(),
                &HashSet::new(),
            )
            .expect_err("one byte beyond the declared envelope ceiling rejects");
            assert_eq!(error.reason(), "protocol.request-bytes");
        }
    }

    #[test]
    fn generation_accepts_handshake_prepare_and_clean_eof() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let request = operation_request(&prepare, 1, '2', prepare_control());
        let mut input = Cursor::new(wire(&[handshake(), request]));
        let mut output = Vec::new();

        let receipt = serve_managed_runtime(&mut input, &mut output).expect("session");
        let frames = output_frames(&output);

        assert_eq!(
            (
                receipt.request_count,
                receipt.response_count,
                receipt.observer_state_cleared
            ),
            (1, 1, true)
        );
        assert_eq!(frames.len(), 2);
        assert_ne!(frames[0]["message_id"], frames[1]["message_id"]);
        assert_ne!(frames[0]["message_id"], handshake()["message_id"]);
        assert_ne!(frames[1]["message_id"], handshake()["message_id"]);
    }

    #[test]
    fn generation_reserves_a_bounded_retry_after_domain_rejection() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let mut invalid_control = prepare_control();
        invalid_control["channel_ids"] = json!(["channel-02", "channel-01", "channel-03"]);
        let rejected = operation_request(&prepare, 1, '2', invalid_control);
        let corrected = operation_request(&prepare, 2, '3', prepare_control());
        let mut input = Cursor::new(wire(&[handshake(), rejected, corrected]));
        let mut output = Vec::new();

        let receipt = serve_managed_runtime(&mut input, &mut output)
            .expect("one domain rejection leaves room for correction");
        let frames = output_frames(&output);

        assert_eq!(
            (
                receipt.request_count,
                receipt.response_count,
                receipt.rejected_count,
                receipt.observer_state_cleared,
            ),
            (2, 2, 1, true)
        );
        assert_eq!(frames[1]["body"]["status"], "rejected");
        assert_eq!(frames[2]["body"]["status"], "succeeded");
    }

    #[test]
    fn generation_fails_stop_after_the_rejection_reserve() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let mut invalid_control = prepare_control();
        invalid_control["channel_ids"] = json!(["channel-02", "channel-01", "channel-03"]);
        let mut frames = vec![handshake()];
        for sequence in 1..=(MAX_REJECTED_OPERATION_ATTEMPTS + 1) {
            let mut request = operation_request(&prepare, sequence, '2', invalid_control.clone());
            request["message_id"] = json!(format!("msg_{sequence:032x}"));
            request["body"]["idempotency_key"] = json!(format!("idem_{sequence:064x}"));
            frames.push(request);
        }
        let mut input = Cursor::new(wire(&frames));
        let mut output = Vec::new();

        let error = serve_managed_runtime(&mut input, &mut output)
            .expect_err("the seventeenth domain rejection fails stop");
        let responses = output_frames(&output);

        assert_eq!(error.reason(), "runtime.rejection-count");
        assert_eq!(
            responses.len(),
            1 + MAX_REJECTED_OPERATION_ATTEMPTS as usize
        );
    }

    #[test]
    fn contained_operation_panic_clears_state_and_fails_stop() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let first = operation_request(&prepare, 1, '2', prepare_control());
        let second = operation_request(&prepare, 2, '3', prepare_control());
        let mut input = Cursor::new(wire(&[handshake(), first, second]));
        let mut output = Vec::new();
        let dispatch_count = Cell::new(0_u64);

        let error = serve_generation_with_dispatch(&mut input, &mut output, |runtime, accepted| {
            dispatch_count.set(dispatch_count.get() + 1);
            let typed: PrepareRequest = deserialize_control(&accepted.control)?;
            let study_run_id = typed.study_run_id.clone();
            let operation_request = typed.clone();
            let response = contain_operation(
                runtime,
                PREPARE_RESPONSE_SCHEMA_ID,
                &typed,
                &study_run_id,
                0,
                move |runtime| -> Result<ObserverResponse, ObserverError> {
                    runtime.prepare(operation_request)?;
                    panic!("injected post-mutation operation panic")
                },
            )?;
            assert_eq!(response.outcome, ObserverOutcome::Failed);
            assert!(response.state_cleared);
            assert_eq!(
                observer_response_receipt_sha256(&response)
                    .expect("failed semantic receipt remains reproducible"),
                response.observer_receipt_sha256
            );
            assert!(runtime.is_cleared());
            assert_eq!(
                runtime
                    .prepare(typed)
                    .expect_err("cleared state rejects a next operation"),
                ObserverError::RunFinished
            );
            Ok(response)
        })
        .expect_err("a contained operation panic terminates the generation");
        let responses = output_frames(&output);

        assert_eq!(error.reason(), "runtime.operation-failed");
        assert_eq!(dispatch_count.get(), 1);
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[1]["body"]["status"], "failed");
        assert_eq!(responses[1]["body"]["control"]["state_cleared"], true);
    }

    #[test]
    fn canonicalization_failure_clears_state_and_fails_stop() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let first = operation_request(&prepare, 1, '2', prepare_control());
        let second = operation_request(&prepare, 2, '3', prepare_control());
        let mut input = Cursor::new(wire(&[handshake(), first, second]));
        let mut output = Vec::new();
        let dispatch_count = Cell::new(0_u64);

        let error = serve_generation_with_dispatch(&mut input, &mut output, |runtime, accepted| {
            dispatch_count.set(dispatch_count.get() + 1);
            let typed: PrepareRequest = deserialize_control(&accepted.control)?;
            let study_run_id = typed.study_run_id.clone();
            runtime
                .prepare(typed.clone())
                .map_err(|_| protocol_error("runtime.test-setup"))?;
            let response = operation_error_response(
                runtime,
                PREPARE_RESPONSE_SCHEMA_ID,
                &typed,
                &study_run_id,
                0,
                ObserverError::Canonicalization,
            )?;
            assert_eq!(response.outcome, ObserverOutcome::Failed);
            assert!(response.state_cleared);
            assert_eq!(
                observer_response_receipt_sha256(&response)
                    .expect("failed semantic receipt remains reproducible"),
                response.observer_receipt_sha256
            );
            assert!(runtime.is_cleared());
            assert_eq!(
                runtime
                    .prepare(typed)
                    .expect_err("cleared state rejects a next operation"),
                ObserverError::RunFinished
            );
            Ok(response)
        })
        .expect_err("canonicalization failure terminates the generation");
        let responses = output_frames(&output);

        assert_eq!(error.reason(), "runtime.operation-failed");
        assert_eq!(dispatch_count.get(), 1);
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[1]["body"]["status"], "failed");
        assert_eq!(
            responses[1]["body"]["control"]["reason"],
            "canonicalization-failed"
        );
    }

    #[test]
    fn compiled_operation_roster_matches_authoring_contract() {
        assert_eq!(
            MAX_OPERATIONS_PER_GENERATION,
            MAX_STEPS + 2 + MAX_REJECTED_OPERATION_ATTEMPTS
        );
        assert_eq!(MAX_OPERATIONS_PER_GENERATION, 1_042);
        assert_eq!(
            operation_roster_sha256().expect("operation roster"),
            "845ddcddbd3ad6c9854281088cfee86909aa5af6be6e1ed6d6664f0cc3c5d79c"
        );
        assert_eq!(
            ipc_schema_sha256(),
            "e6950a2b3d1913ebacb82823afe648538ec789fe845ed3894b2122dd9864cfc1"
        );
    }

    #[test]
    fn request_rejects_compute_grant_for_observation() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let mut request = operation_request(&prepare, 1, '2', prepare_control());
        request["body"]["compute_grant"] = json!({
            "mode": "host-one-shot",
            "grant_id": format!("grant_{}", "5".repeat(64)),
        });
        let mut input = Cursor::new(wire(&[handshake(), request]));

        let error =
            serve_managed_runtime(&mut input, &mut Vec::new()).expect_err("compute grant rejects");

        assert_eq!(error.reason(), "protocol.shape");
    }

    #[test]
    fn request_rejects_replayed_idempotency_key() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let first = operation_request(&prepare, 1, '2', prepare_control());
        let mut second = operation_request(&prepare, 2, '3', prepare_control());
        second["body"]["idempotency_key"] = first["body"]["idempotency_key"].clone();
        let mut input = Cursor::new(wire(&[handshake(), first, second]));

        let error = serve_managed_runtime(&mut input, &mut Vec::new())
            .expect_err("replayed idempotency rejects");

        assert_eq!(error.reason(), "protocol.idempotency-replay");
    }

    #[test]
    fn request_rejects_handshake_message_id_reuse() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let mut request = operation_request(&prepare, 1, '2', prepare_control());
        request["message_id"] = handshake()["message_id"].clone();
        let mut input = Cursor::new(wire(&[handshake(), request]));

        let error = serve_managed_runtime(&mut input, &mut Vec::new())
            .expect_err("handshake message reuse rejects");

        assert_eq!(error.reason(), "protocol.message-replay");
    }

    #[test]
    fn request_rejects_full_envelope_over_operation_bound() {
        let prepare = operations()
            .into_iter()
            .find(|operation| operation.operation_id == PREPARE_OPERATION_ID)
            .expect("prepare operation");
        let mut control = prepare_control();
        control["channel_ids"] = Value::Array(
            (0..64)
                .map(|index| Value::String(format!("channel-{index:02}-{}", "x".repeat(240))))
                .collect(),
        );
        control["subject_ids"] = Value::Array(
            (0..64)
                .map(|index| Value::String(format!("drone-{index:02}-{}", "y".repeat(242))))
                .collect(),
        );
        let request = operation_request(&prepare, 1, '2', control);
        let payload = canonical_json(&request).expect("request canonicalizes");
        assert!(payload.len() > prepare.max_request_bytes);
        assert!(payload.len() < MAX_FRAME_BYTES);
        let mut input = Cursor::new(wire(&[handshake(), request]));

        let error = serve_managed_runtime(&mut input, &mut Vec::new())
            .expect_err("whole request envelope is bounded");

        assert_eq!(error.reason(), "protocol.request-bytes");
    }

    #[test]
    fn response_writer_uses_the_smaller_bound() {
        let value = json!({"payload": "x".repeat(256)});
        let length = canonical_json(&value)
            .expect("response canonicalizes")
            .len();

        let error = write_frame(&mut Vec::new(), &value, length - 1)
            .expect_err("operation response bound rejects");
        let written =
            write_frame(&mut Vec::new(), &value, length).expect("exact smaller bound accepts");

        assert_eq!(error.reason(), "frame.output-bound");
        assert_eq!(written, length);
    }

    #[test]
    fn frame_rejects_duplicate_json_member() {
        let payload = br#"{"a":1,"a":2}"#;
        let mut wire = (payload.len() as u32).to_be_bytes().to_vec();
        wire.extend_from_slice(payload);

        let error =
            read_frame(&mut Cursor::new(wire), 1_024, false).expect_err("duplicate member rejects");

        assert_eq!(error.reason(), "json.malformed");
    }

    struct PanicReader;

    impl Read for PanicReader {
        fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
            panic!("injected reader panic")
        }
    }

    #[test]
    fn serve_contains_reader_panic() {
        let error = serve_managed_runtime(&mut PanicReader, &mut Vec::new())
            .expect_err("panic is contained");

        assert_eq!(error.reason(), "runtime.internal-panic-contained");
    }
}
