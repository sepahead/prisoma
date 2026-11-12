//! Capture of NCP 2 closed-loop runs into a bounded journal whose records are chained by digest.
//!
//! CREBAIN's `crebain-ncp-loop` writes one NCP 2 transcript for each session of a run (one for
//! each world, contract `crebain.body`, and one for each controller, contract
//! `crebain.controller`) and a `manifest.json`. [`import`] checks such a run independently, with
//! the NCP 2 transcript verifier and its own check of the causal chain, and then writes the run's
//! steps into a new journal. [`verify_journal`] checks a journal without the run.
//!
//! The host records each request before it sends it (NCP 2 specification, section 10), so a
//! complete transcript holds every request that a world or a controller may have executed. The
//! journal records what the transcripts show. Neither proves who produced the frames: the
//! transcripts and the journal are not signed.
//!
//! This crate issues no request to any program and opens no network connection.

#![forbid(unsafe_code)]

use ncp::frame::{decode, Decoded, MAX_BOUND};
use ncp::transcript::{read, verify, RECEIVED, SENT};
use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// The journal profile, also the domain of every record digest.
pub const PROFILE: &str = "prisoma.ncp-loop-capture.v1";
/// The default and largest journal size: 128 MiB.
pub const MAX_CAPTURE_BYTES: u64 = 128 * 1024 * 1024;
/// The largest transcript that [`import`] reads.
pub const MAX_TRANSCRIPT_BYTES: u64 = 256 * 1024 * 1024;
const BODY_APPLICATION: &str = "crebain.body";
const CONTROLLER_APPLICATION: &str = "crebain.controller";

/// A capture failure, with a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureError(pub String);

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CaptureError {}

fn fail<T>(reason: impl Into<String>) -> Result<T, CaptureError> {
    Err(CaptureError(reason.into()))
}

/// What a capture or a verification covered.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Report {
    /// The number of worlds.
    pub worlds: usize,
    /// The number of controllers.
    pub controllers: usize,
    /// The number of steps.
    pub steps: u64,
    /// The number of journal records.
    pub records: u64,
    /// The size of the journal in bytes.
    pub bytes: u64,
    /// The digest of the last record.
    pub last_digest: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn is_identity(value: &Value) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn numbers(value: &Value, what: &str) -> Result<Vec<f64>, CaptureError> {
    value
        .as_array()
        .and_then(|items| items.iter().map(Value::as_f64).collect::<Option<Vec<_>>>())
        .filter(|items| items.iter().all(|v| v.is_finite()))
        .ok_or_else(|| CaptureError(format!("{what}: finite numbers expected")))
}

fn strings(value: &Value, what: &str) -> Result<Vec<String>, CaptureError> {
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|v| v.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| CaptureError(format!("{what}: strings expected")))
}

/// One exchange of a client transcript.
struct Exchange {
    request: Decoded,
    response: Decoded,
}

impl Exchange {
    fn op(&self) -> &str {
        self.request.message["op"].as_str().unwrap_or_default()
    }
    fn body(&self) -> &Value {
        &self.request.message["body"]
    }
    fn result(&self) -> &Value {
        &self.response.message["body"]
    }
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Read and check one transcript: one client session whose every request got an `ok` response.
fn exchanges(
    path: &Path,
    application: &str,
    contract: &str,
) -> Result<(Vec<Exchange>, String), CaptureError> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_owned();
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_TRANSCRIPT_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|error| CaptureError(format!("{name}: {error}")))?;
    if bytes.len() as u64 > MAX_TRANSCRIPT_BYTES {
        return fail(format!("{name}: larger than {MAX_TRANSCRIPT_BYTES} bytes"));
    }
    let sessions = verify(path).map_err(|error| CaptureError(format!("{name}: {error}")))?;
    let (session, report) = match sessions.iter().next() {
        Some(entry) if sessions.len() == 1 => entry,
        _ => {
            return fail(format!(
                "{name}: {} sessions instead of one",
                sessions.len()
            ))
        }
    };
    if report.role != "client" || !report.problems.is_empty() {
        return fail(format!("{name}: session {session}: {:?}", report.problems));
    }
    let mut list = Vec::new();
    let mut pending: Option<Decoded> = None;
    for record in read(path).map_err(|error| CaptureError(format!("{name}: {error}")))? {
        let decoded = decode(record.frame, MAX_BOUND)
            .map_err(|error| CaptureError(format!("{name}: {error}")))?;
        match (record.kind, pending.take()) {
            (SENT, None) => pending = Some(decoded),
            (RECEIVED, Some(request)) => list.push(Exchange {
                request,
                response: decoded,
            }),
            _ => return fail(format!("{name}: requests and responses do not alternate")),
        }
    }
    if pending.is_some() {
        return fail(format!("{name}: the last request has no response"));
    }
    for exchange in &list {
        let response = &exchange.response.message;
        if response["outcome"] != "ok" || response["request"] != exchange.request.identity.as_str()
        {
            return fail(format!(
                "{name}: seq {}: not an ok response to its request",
                exchange.request.message["seq"]
            ));
        }
    }
    let open = list
        .first()
        .ok_or_else(|| CaptureError(format!("{name}: empty")))?;
    if open.op() != "open"
        || open.body()["application"] != application
        || open.body()["contract"] != contract
    {
        return fail(format!(
            "{name}: the session does not open {application} with the contract of the manifest"
        ));
    }
    if list.last().map(|e| e.op() != "close").unwrap_or(true) {
        return fail(format!("{name}: the session does not end with close"));
    }
    Ok((list, hex(&Sha256::digest(&bytes))))
}

struct Controller {
    world: usize,
    drone_ids: Vec<String>,
}

struct ControllerStep {
    controller: usize,
    tick: u64,
    request_identity: String,
    request: Value,
    acceleration: Vec<f64>,
}

/// The observation of `drone_ids`, from a world's observation, as a controller receives it.
fn observed(
    observation: &Value,
    world_ids: &[String],
    drone_ids: &[String],
) -> Result<Value, CaptureError> {
    let available = observation["estimate_available"]
        .as_array()
        .ok_or_else(|| CaptureError("observation".into()))?;
    let position = numbers(&observation["position_m"], "observation position")?;
    let velocity = numbers(&observation["velocity_mps"], "observation velocity")?;
    let (mut a, mut p, mut v) = (Vec::new(), Vec::new(), Vec::new());
    for drone in drone_ids {
        let at = world_ids
            .iter()
            .position(|d| d == drone)
            .ok_or_else(|| CaptureError(format!("drone {drone}")))?;
        a.push(
            available
                .get(at)
                .cloned()
                .ok_or_else(|| CaptureError("observation availability".into()))?,
        );
        p.extend_from_slice(
            position
                .get(at * 3..at * 3 + 3)
                .ok_or_else(|| CaptureError("observation position".into()))?,
        );
        v.extend_from_slice(
            velocity
                .get(at * 3..at * 3 + 3)
                .ok_or_else(|| CaptureError("observation velocity".into()))?,
        );
    }
    Ok(json!({"estimate_available": a, "position_m": p, "velocity_mps": v}))
}

/// The checked content of a run: the header and the records of each step and of the finish.
struct Run {
    header: Value,
    steps: Vec<Value>,
    finish: Value,
    worlds: usize,
    controllers: usize,
}

fn check_run(dir: &Path) -> Result<Run, CaptureError> {
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(dir.join("manifest.json"))
            .map_err(|error| CaptureError(format!("manifest.json: {error}")))?,
    )
    .map_err(|error| CaptureError(format!("manifest.json: {error}")))?;
    let steps = manifest["steps"]
        .as_u64()
        .filter(|s| (1..=1024).contains(s))
        .ok_or_else(|| CaptureError("manifest steps".into()))?;
    let tick_ms = manifest["tick_ms"]
        .as_u64()
        .ok_or_else(|| CaptureError("manifest tick_ms".into()))?;
    let body_contract = manifest["body_contract"]
        .as_str()
        .filter(|_| is_identity(&manifest["body_contract"]))
        .ok_or_else(|| CaptureError("manifest body_contract".into()))?;
    let controller_contract = manifest["controller_contract"]
        .as_str()
        .filter(|_| is_identity(&manifest["controller_contract"]))
        .ok_or_else(|| CaptureError("manifest controller_contract".into()))?;
    let world_rows = manifest["worlds"]
        .as_array()
        .filter(|w| !w.is_empty())
        .ok_or_else(|| CaptureError("manifest worlds".into()))?;
    let controller_rows = manifest["controllers"]
        .as_array()
        .filter(|c| !c.is_empty())
        .ok_or_else(|| CaptureError("manifest controllers".into()))?;
    let mut names = BTreeMap::new();
    let mut transcript = |row: &Value| -> Result<String, CaptureError> {
        let name = row["transcript"]
            .as_str()
            .filter(|n| safe_name(n))
            .ok_or_else(|| CaptureError("manifest transcript name".into()))?;
        if names.insert(name.to_owned(), String::new()).is_some() {
            return fail(format!("transcript {name} named twice"));
        }
        Ok(name.to_owned())
    };

    // Controllers: every step response by identity.
    let mut controllers = Vec::new();
    let mut by_response: BTreeMap<String, ControllerStep> = BTreeMap::new();
    let mut digests = BTreeMap::new();
    for (index, row) in controller_rows.iter().enumerate() {
        let name = transcript(row)?;
        let world = row["world"]
            .as_u64()
            .map(|w| w as usize)
            .filter(|w| *w < world_rows.len())
            .ok_or_else(|| CaptureError(format!("controller {index}: world")))?;
        let drone_ids = strings(&row["drone_ids"], "controller drone_ids")?;
        let (list, digest) = exchanges(
            &dir.join(&name),
            CONTROLLER_APPLICATION,
            controller_contract,
        )?;
        digests.insert(name.clone(), digest);
        if list.len() as u64 != steps + 3 || list[1].op() != "prepare" {
            return fail(format!(
                "controller {index}: open, prepare, {steps} steps, close expected"
            ));
        }
        for (tick, exchange) in (1..).zip(&list[2..list.len() - 1]) {
            let (body, result) = (exchange.body(), exchange.result());
            if exchange.op() != "step"
                || body["tick_index"] != tick
                || strings(&body["drone_ids"], "step drone_ids")? != drone_ids
                || !is_identity(&body["source_state_digest"])
                || result["tick_index"] != tick
                || result["source_state_digest"] != body["source_state_digest"]
                || result["drone_ids"] != body["drone_ids"]
            {
                return fail(format!(
                    "controller {index}: step {tick} is out of order or does not echo its request"
                ));
            }
            let acceleration = numbers(&result["acceleration_mps2"], "acceleration")?;
            if acceleration.len() != drone_ids.len() * 3 {
                return fail(format!(
                    "controller {index}: step {tick}: 3 components for each drone expected"
                ));
            }
            by_response.insert(
                exchange.response.identity.clone(),
                ControllerStep {
                    controller: index,
                    tick,
                    request_identity: exchange.request.identity.clone(),
                    request: body.clone(),
                    acceleration,
                },
            );
        }
        controllers.push(Controller { world, drone_ids });
    }

    // Worlds: the chain from each observation to the next.
    let mut step_worlds: Vec<Vec<Value>> = vec![Vec::new(); steps as usize];
    let mut step_controllers: Vec<Vec<Value>> = vec![Vec::new(); steps as usize];
    let mut finishes = Vec::new();
    let mut used: BTreeMap<String, bool> = by_response.keys().map(|k| (k.clone(), false)).collect();
    for (w, row) in world_rows.iter().enumerate() {
        let name = transcript(row)?;
        let world_ids = strings(&row["drone_ids"], "world drone_ids")?;
        let (list, digest) = exchanges(&dir.join(&name), BODY_APPLICATION, body_contract)?;
        digests.insert(name.clone(), digest);
        if list.len() as u64 != steps + 4 {
            return fail(format!(
                "world {w}: open, prepare, {steps} steps, finish, close expected"
            ));
        }
        let prepare = &list[1];
        if prepare.op() != "prepare"
            || strings(&prepare.body()["drone_ids"], "prepare drone_ids")? != world_ids
            || prepare.body()["planned_steps"] != steps
            || prepare.body()["tick_ms"] != tick_ms
        {
            return fail(format!(
                "world {w}: the prepare request does not match the manifest"
            ));
        }
        let mut observation = prepare.result()["observation"].clone();
        let mut state = observation["state_digest"].clone();
        if !is_identity(&state) || prepare.result()["frame"]["state_digest"] != state {
            return fail(format!(
                "world {w}: the prepare result has no consistent state digest"
            ));
        }
        for (tick, exchange) in (1..).zip(&list[2..list.len() - 2]) {
            let body = exchange.body();
            if exchange.op() != "step"
                || body["tick_index"] != tick
                || body["source_state_digest"] != state
            {
                return fail(format!(
                    "world {w}: step {tick} is out of order or not from the current state"
                ));
            }
            let intent = numbers(&body["actuator_intent_acceleration_mps2"], "intent")?;
            if intent.len() != world_ids.len() * 3 {
                return fail(format!(
                    "world {w}: step {tick}: 3 components for each drone expected"
                ));
            }
            let proposals = body["proposals"]
                .as_array()
                .ok_or_else(|| CaptureError("proposals".into()))?;
            let mut covered = vec![false; world_ids.len()];
            for proposal in proposals {
                let identity = proposal["response"].as_str().unwrap_or_default();
                let found = by_response.get(identity).ok_or_else(|| {
                    CaptureError(format!(
                        "world {w}: step {tick}: no controller response {identity}"
                    ))
                })?;
                let controller = &controllers[found.controller];
                let drone_ids = strings(&proposal["drone_ids"], "proposal drone_ids")?;
                if controller.world != w
                    || found.tick != tick
                    || found.request["source_state_digest"] != state
                    || drone_ids != controller.drone_ids
                {
                    return fail(format!("world {w}: step {tick}: response {identity} answers another world, step, state, or drone set"));
                }
                let seen = observed(&observation, &world_ids, &drone_ids)?;
                for key in ["estimate_available", "position_m", "velocity_mps"] {
                    if found.request[key] != seen[key] {
                        return fail(format!(
                            "world {w}: step {tick}: controller {} received another observation",
                            found.controller
                        ));
                    }
                }
                for (k, drone) in drone_ids.iter().enumerate() {
                    let at = world_ids
                        .iter()
                        .position(|d| d == drone)
                        .ok_or_else(|| CaptureError(format!("drone {drone}")))?;
                    if std::mem::replace(&mut covered[at], true) {
                        return fail(format!("world {w}: step {tick}: drone {drone} moved twice"));
                    }
                    if intent[at * 3..at * 3 + 3] != found.acceleration[k * 3..k * 3 + 3] {
                        return fail(format!("world {w}: step {tick}: drone {drone} got another acceleration than its controller proposed"));
                    }
                }
                if used.insert(identity.to_owned(), true) != Some(false) {
                    return fail(format!(
                        "world {w}: step {tick}: response {identity} applied twice"
                    ));
                }
                step_controllers[tick as usize - 1].push(json!({
                    "controller": found.controller,
                    "request": found.request_identity,
                    "response": identity,
                    "drone_ids": drone_ids,
                    "acceleration_mps2": found.acceleration,
                }));
            }
            if !covered.iter().all(|c| *c) {
                return fail(format!("world {w}: step {tick}: a drone has no proposal"));
            }
            let result = exchange.result();
            let frame = &result["frame"];
            let next = result["observation"].clone();
            if frame["prior_state_digest"] != state
                || frame["state_digest"] != next["state_digest"]
                || next["tick_index"] != tick
                || !is_identity(&next["state_digest"])
            {
                return fail(format!(
                    "world {w}: step {tick}: the result does not continue the state chain"
                ));
            }
            step_worlds[tick as usize - 1].push(json!({
                "world": w,
                "request": exchange.request.identity,
                "response": exchange.response.identity,
                "source_state_digest": state,
                "state_digest": next["state_digest"],
                "actuator_intent_acceleration_mps2": intent,
                "actuator_output_acceleration_mps2": frame["actuator_output_acceleration_mps2"],
                "true_position_m": frame["simulated_position_m"],
                "observation": next,
                "innovations": result["innovations"],
            }));
            observation = next;
            state = observation["state_digest"].clone();
        }
        let finish = &list[list.len() - 2];
        if finish.op() != "finish"
            || finish.body()["completed_steps"] != steps
            || finish.result()["finish"]["cleaned_up"] != true
            || finish.result()["finish"]["state_digest"] != state
        {
            return fail(format!(
                "world {w}: the run does not finish cleanly from its last state"
            ));
        }
        finishes.push(json!({"world": w, "final_state_digest": state, "cleaned_up": true}));
    }
    if let Some((identity, _)) = used.iter().find(|(_, applied)| !**applied) {
        return fail(format!("controller response {identity} was never applied"));
    }
    let header = json!({"profile": PROFILE, "manifest": manifest, "transcripts": digests});
    let records = (1..=steps)
        .map(|tick| json!({"tick_index": tick, "worlds": step_worlds[tick as usize - 1], "controllers": step_controllers[tick as usize - 1]}))
        .collect();
    Ok(Run {
        header,
        steps: records,
        finish: json!({"completed_steps": steps, "worlds": finishes}),
        worlds: world_rows.len(),
        controllers: controller_rows.len(),
    })
}

/// The digest of a record without its `record_digest` member, in the journal's domain.
fn record_digest(record: &Map<String, Value>) -> String {
    let mut without = record.clone();
    without.remove("record_digest");
    let mut hasher = Sha256::new();
    hasher.update(PROFILE.as_bytes());
    hasher.update(b"\n");
    hasher.update(serde_json::to_vec(&Value::Object(without)).unwrap_or_default());
    hex(&hasher.finalize())
}

fn record(ordinal: u64, previous: Option<&str>, kind: &str, data: Value) -> (String, String) {
    let mut map = Map::new();
    map.insert("ordinal".into(), json!(ordinal));
    map.insert("previous".into(), json!(previous));
    map.insert("kind".into(), json!(kind));
    map.insert("data".into(), data);
    let digest = record_digest(&map);
    map.insert("record_digest".into(), json!(digest));
    (Value::Object(map).to_string(), digest)
}

fn create(path: &Path) -> Result<File, CaptureError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| CaptureError(format!("{}: {error}", path.display())))
}

/// Check the run in `run` and write its journal to `journal`, which must not exist.
///
/// The check completes before the journal exists, and a journal larger than `max_bytes` is
/// refused before it is created. A failure while writing leaves a journal without its finish
/// record, which [`verify_journal`] rejects.
pub fn import(run: &Path, journal: &Path, max_bytes: u64) -> Result<Report, CaptureError> {
    if max_bytes == 0 || max_bytes > MAX_CAPTURE_BYTES {
        return fail(format!(
            "the quota must be from 1 to {MAX_CAPTURE_BYTES} bytes"
        ));
    }
    let checked = check_run(run)?;
    let mut lines = Vec::new();
    let mut previous: Option<String> = None;
    let steps = checked.steps.len() as u64;
    let data = std::iter::once(("header", checked.header))
        .chain(checked.steps.into_iter().map(|s| ("step", s)))
        .chain(std::iter::once(("finish", checked.finish)));
    for (ordinal, (kind, value)) in (0u64..).zip(data) {
        let (line, digest) = record(ordinal, previous.as_deref(), kind, value);
        lines.push(line);
        previous = Some(digest);
    }
    let bytes: u64 = lines.iter().map(|l| l.len() as u64 + 1).sum();
    if bytes > max_bytes {
        return fail(format!(
            "the journal needs {bytes} bytes, more than the quota of {max_bytes}"
        ));
    }
    let mut file = create(journal)?;
    for line in &lines {
        file.write_all(line.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|error| CaptureError(error.to_string()))?;
    }
    file.sync_all()
        .map_err(|error| CaptureError(error.to_string()))?;
    let parent = journal
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|error| CaptureError(error.to_string()))?;
    Ok(Report {
        worlds: checked.worlds,
        controllers: checked.controllers,
        steps,
        records: lines.len() as u64,
        bytes,
        last_digest: previous.unwrap_or_default(),
    })
}

/// Check a journal: its record digests, their chain, the order of its records, and its finish.
pub fn verify_journal(path: &Path) -> Result<Report, CaptureError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| CaptureError(format!("{}: {error}", path.display())))?;
    let mut text = String::new();
    (&mut file)
        .take(MAX_CAPTURE_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|error| CaptureError(error.to_string()))?;
    if text.len() as u64 > MAX_CAPTURE_BYTES || !text.ends_with('\n') {
        return fail("the journal is larger than its limit or does not end with a complete line");
    }
    let mut previous: Option<String> = None;
    let mut steps = 0u64;
    let (mut worlds, mut controllers, mut finished) = (0usize, 0usize, None);
    let lines: Vec<&str> = text.split_terminator('\n').collect();
    for (ordinal, line) in (0u64..).zip(&lines) {
        let value: Value = serde_json::from_str(line)
            .map_err(|error| CaptureError(format!("record {ordinal}: {error}")))?;
        let map = value
            .as_object()
            .ok_or_else(|| CaptureError(format!("record {ordinal}: not an object")))?;
        let keys: Vec<&str> = map.keys().map(String::as_str).collect();
        if keys != ["data", "kind", "ordinal", "previous", "record_digest"]
            || map["ordinal"] != ordinal
            || map["previous"] != json!(previous)
        {
            return fail(format!("record {ordinal}: members, ordinal, or chain"));
        }
        let digest = record_digest(map);
        if map["record_digest"] != digest.as_str()
            || serde_json::to_vec(&value).ok().as_deref() != Some(line.as_bytes())
        {
            return fail(format!(
                "record {ordinal}: the digest or the encoding does not match"
            ));
        }
        let data = &map["data"];
        match (ordinal, map["kind"].as_str(), finished) {
            (0, Some("header"), None) => {
                if data["profile"] != PROFILE {
                    return fail("the header names another profile");
                }
                worlds = data["manifest"]["worlds"].as_array().map_or(0, Vec::len);
                controllers = data["manifest"]["controllers"]
                    .as_array()
                    .map_or(0, Vec::len);
            }
            (n, Some("step"), None) if n > 0 && data["tick_index"] == steps + 1 => steps += 1,
            (n, Some("finish"), None) if n > 0 && data["completed_steps"] == steps => {
                finished = Some(n)
            }
            _ => return fail(format!("record {ordinal}: out of order")),
        }
        previous = Some(digest);
    }
    if finished.is_none() {
        return fail("the journal has no finish record");
    }
    Ok(Report {
        worlds,
        controllers,
        steps,
        records: lines.len() as u64,
        bytes: text.len() as u64,
        last_digest: previous.unwrap_or_default(),
    })
}
