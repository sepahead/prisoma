use anyhow::{bail, ensure, Context, Result};
use pid_runlog::{
    HashIdentity, HashRevision, RunLogEvent, RunLogLimits, RunLogSummary, RunManifest,
    SimObjectSnapshot,
};
use rerun::RecordingStream;
use rerun_types::{
    archetypes::{Points3D, Scalars, TextLog},
    components::Color,
};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    path::{Component, Path, PathBuf},
    time::Duration,
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

/// Maximum number of relevance values accepted and surfaced per attribution.
const MAX_RELEVANCE_POINTS: usize = 1024;

/// NumPy v1 headers produced by `numpy.save` are normally well below 1 KiB.
/// Four KiB leaves room for a moderately dimensional shape without accepting an
/// arbitrarily large attacker-controlled header.
const MAX_NPY_HEADER_BYTES: usize = 4 * 1024;

/// Twelve KiB accommodates the 64-byte-aligned bounded header plus exactly
/// 1024 little-endian `f64` values.
const MAX_RELEVANCE_FILE_BYTES: usize = 12 * 1024;

/// Batch preflight retains relevance arrays so each artifact is read before any
/// Rerun write and never read again while emitting. Bound that retained state.
const MAX_PREPARED_RELEVANCE_BYTES: usize = 8 * 1024 * 1024;

/// Viewer conversion has a tighter fail-closed envelope than the canonical
/// run-log reader. This prevents a bounded JSONL input from amplifying into an
/// impractical number of Rerun messages or large intermediate strings.
const MAX_RERUN_EVENTS: usize = 100_000;
const MAX_RERUN_PROJECTED_LOG_CALLS: usize = 250_000;
const MAX_RERUN_PROJECTED_EVENT_BYTES: usize = 64 * 1024 * 1024;
/// Application-generated entity paths are separately bounded because flow
/// events repeat an encoded object/source identifier once per vector. Merely
/// bounding the input JSON and call count would otherwise permit a small event
/// to expand into hundreds of GiB of transient path strings.
const MAX_RERUN_PROJECTED_ENTITY_PATH_BYTES: usize = 64 * 1024 * 1024;
const MAX_RERUN_MANIFEST_BYTES: usize = 16 * 1024 * 1024;

/// Every non-dynamic path emitted by this adapter is shorter than 64 bytes.
/// Using this upper bound keeps the aggregate calculation simple and
/// conservative for summary, manifest, and fixed-path event calls.
const MAX_FIXED_ENTITY_PATH_BYTES: usize = 64;

const NPY_V1_PREFIX_BYTES: usize = 10;
const NPY_HEADER_PREFIX: &str = "{'descr': '<f8', 'fortran_order': False, 'shape': ";

fn replay_trace_v2_provenance(summary: &RunLogSummary) -> Result<String> {
    let identity = &summary
        .hash_identities
        .as_ref()
        .context("run-log summary omitted explicit hash identities")?
        .replay_lossless;
    identity.validate()?;
    ensure!(
        identity.revision == HashRevision::ReplayTraceV2,
        "run-log summary replay identity has the wrong revision"
    );
    ensure!(
        identity.digest == summary.trace_hash_v2,
        "run-log summary replay identity does not match trace_hash_v2"
    );
    Ok(format!(
        "algorithm=sha256 revision=replay_trace_v2 digest={}",
        identity.digest
    ))
}

/// Parse the exact NumPy v1.0 framing emitted by `numpy.save` for a plain,
/// little-endian, C-order `float64` array.
fn parse_npy_f64(bytes: &[u8]) -> Result<(Vec<f64>, Vec<usize>)> {
    if bytes.len() < NPY_V1_PREFIX_BYTES || &bytes[..6] != b"\x93NUMPY" {
        bail!("not a NumPy v1.0 file");
    }
    if bytes[6..8] != [1, 0] {
        bail!(
            "unsupported NumPy version {}.{}; only 1.0 is accepted",
            bytes[6],
            bytes[7]
        );
    }

    let header_len = usize::from(u16::from_le_bytes([bytes[8], bytes[9]]));
    if header_len == 0 || header_len > MAX_NPY_HEADER_BYTES {
        bail!("NumPy header length {header_len} exceeds the supported bound");
    }
    let data_start = NPY_V1_PREFIX_BYTES
        .checked_add(header_len)
        .context("NumPy header offset overflow")?;
    if data_start > bytes.len() {
        bail!("truncated NumPy header");
    }
    if data_start % 64 != 0 {
        bail!("NumPy v1.0 header is not 64-byte aligned");
    }

    let header = std::str::from_utf8(&bytes[NPY_V1_PREFIX_BYTES..data_start])
        .context("NumPy header is not UTF-8")?;
    let shape = parse_npy_header(header)?;
    let count = shape
        .iter()
        .try_fold(1usize, |acc, &dimension| acc.checked_mul(dimension))
        .context("NumPy shape product overflow")?;
    if count == 0 || count > MAX_RELEVANCE_POINTS {
        bail!(
            "NumPy relevance array contains {count} values; supported range is 1..={MAX_RELEVANCE_POINTS}"
        );
    }
    let data_bytes = count
        .checked_mul(size_of::<f64>())
        .context("NumPy data length overflow")?;
    let expected_len = data_start
        .checked_add(data_bytes)
        .context("NumPy file length overflow")?;
    if bytes.len() != expected_len {
        bail!(
            "NumPy file length mismatch: expected {expected_len} bytes, found {}",
            bytes.len()
        );
    }

    let mut values = Vec::with_capacity(count);
    for chunk in bytes[data_start..].chunks_exact(size_of::<f64>()) {
        let value = f64::from_le_bytes(
            chunk
                .try_into()
                .context("internal NumPy chunk length mismatch")?,
        );
        if !value.is_finite() {
            bail!("NumPy relevance values must all be finite");
        }
        values.push(value);
    }
    Ok((values, shape))
}

fn parse_npy_header(header: &str) -> Result<Vec<usize>> {
    let unpadded = header
        .strip_suffix('\n')
        .context("NumPy header must end with one newline")?
        .trim_end_matches(' ');
    if unpadded.contains(['\n', '\r', '\t']) {
        bail!("NumPy header contains unsupported whitespace");
    }
    let shape = unpadded
        .strip_prefix(NPY_HEADER_PREFIX)
        .and_then(|rest| rest.strip_suffix(", }"))
        .context("NumPy header is not the supported plain <f8 C-order dictionary")?;
    parse_npy_shape(shape)
}

fn parse_npy_shape(shape: &str) -> Result<Vec<usize>> {
    let inner = shape
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .context("NumPy shape is not a tuple")?;
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let parts = inner.split(',').collect::<Vec<_>>();
    let trailing_comma = parts.last().is_some_and(|part| part.trim().is_empty());
    let dimensions = parts.len() - usize::from(trailing_comma);
    if dimensions == 0 || (dimensions == 1 && !trailing_comma) || (dimensions > 1 && trailing_comma)
    {
        bail!("NumPy shape is not in canonical tuple form");
    }

    let parsed = parts[..dimensions]
        .iter()
        .map(|part| {
            let dimension = part.trim();
            if dimension.is_empty() || !dimension.bytes().all(|byte| byte.is_ascii_digit()) {
                bail!("NumPy shape contains a non-decimal dimension");
            }
            dimension
                .parse::<usize>()
                .context("NumPy shape dimension does not fit usize")
        })
        .collect::<Result<Vec<_>>>()?;
    let canonical = if parsed.len() == 1 {
        format!("({},)", parsed[0])
    } else {
        format!(
            "({})",
            parsed
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    ensure!(shape == canonical, "NumPy shape tuple is not canonical");
    Ok(parsed)
}

fn read_bounded_file(path: &Path) -> Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let file = options
        .open(path)
        .with_context(|| format!("failed to open relevance artifact {}", path.display()))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("failed to inspect relevance artifact {}", path.display()))?;
    if !metadata.is_file() {
        bail!(
            "relevance artifact is not a regular file: {}",
            path.display()
        );
    }
    if metadata.len() > MAX_RELEVANCE_FILE_BYTES as u64 {
        bail!(
            "relevance artifact exceeds the {MAX_RELEVANCE_FILE_BYTES}-byte limit: {}",
            path.display()
        );
    }

    let expected_len = usize::try_from(metadata.len()).context("artifact length overflow")?;
    let mut bytes = Vec::with_capacity(expected_len);
    file.take((MAX_RELEVANCE_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read relevance artifact {}", path.display()))?;
    if bytes.len() > MAX_RELEVANCE_FILE_BYTES {
        bail!(
            "relevance artifact exceeds the {MAX_RELEVANCE_FILE_BYTES}-byte limit: {}",
            path.display()
        );
    }
    if bytes.len() != expected_len {
        bail!("relevance artifact changed while it was being read");
    }
    Ok(bytes)
}

fn confined_regular_artifact(base: &Path, uri: &str) -> Result<PathBuf> {
    let relative = Path::new(uri);
    if relative.is_absolute() {
        bail!("absolute attribution artifact paths are not allowed");
    }

    let mut candidate = base.to_path_buf();
    let mut components = relative.components().peekable();
    if components.peek().is_none() {
        bail!("attribution artifact path is empty");
    }
    while let Some(component) = components.next() {
        let Component::Normal(segment) = component else {
            bail!("attribution artifact path must contain only normal relative segments");
        };
        candidate.push(segment);
        let metadata = fs::symlink_metadata(&candidate).with_context(|| {
            format!(
                "failed to inspect relevance artifact {}",
                candidate.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            bail!("symlinked attribution artifact paths are not allowed");
        }
        if components.peek().is_some() {
            if !metadata.is_dir() {
                bail!("attribution artifact parent is not a directory");
            }
        } else if !metadata.is_file() {
            bail!("attribution artifact is not a regular file");
        }
    }

    let canonical = fs::canonicalize(&candidate).with_context(|| {
        format!(
            "failed to canonicalize relevance artifact {}",
            candidate.display()
        )
    })?;
    if !canonical.starts_with(base) {
        bail!("attribution artifact escapes the run-log directory");
    }
    Ok(canonical)
}

fn entity_segment(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(2 + value.len());
    encoded.push_str("s_");
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-') {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX[usize::from(byte >> 4)].to_ascii_uppercase() as char);
            encoded.push(HEX[usize::from(byte & 0x0f)].to_ascii_uppercase() as char);
        }
    }
    encoded
}

fn entity_segment_len(value: &str) -> Result<usize> {
    value.as_bytes().iter().try_fold(2usize, |length, byte| {
        let encoded_byte_len = if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-') {
            1
        } else {
            3
        };
        length
            .checked_add(encoded_byte_len)
            .context("encoded Rerun entity-segment length overflow")
    })
}

fn decimal_digits(mut value: usize) -> usize {
    let mut digits = 1;
    while value >= 10 {
        value /= 10;
        digits += 1;
    }
    digits
}

fn indexed_entity_path_bytes(prefix_len: usize, count: usize) -> Result<usize> {
    (0..count).try_fold(0usize, |total, index| {
        let path_len = prefix_len
            .checked_add(decimal_digits(index))
            .context("indexed Rerun entity-path length overflow")?;
        total
            .checked_add(path_len)
            .context("indexed Rerun entity-path aggregate overflow")
    })
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn relevance_shape_label(shape: &[usize]) -> String {
    shape
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("x")
}

fn attribution_identity_segment(
    method: &str,
    target_output: &str,
    layer: Option<&str>,
    modality: Option<&str>,
    baseline: Option<&str>,
) -> Result<String> {
    // Length-prefixed tuple encoding supplied by serde_json makes the identity
    // unambiguous without placing attacker-controlled strings in entity paths.
    // Repeated observations of the same attribution definition stay on one
    // time series; distinct targets/layers/modalities/baselines cannot collide.
    let identity = serde_json::to_vec(&(method, target_output, layer, modality, baseline))
        .context("failed to encode attribution identity")?;
    Ok(format!("i_{}", pid_runlog::sha256_hex(&identity)))
}

fn renderable_position(position: &[f64; 3]) -> Result<[f32; 3]> {
    for (axis, value) in position.iter().enumerate() {
        if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
            bail!("object position axis {axis} is not finite and f32-representable: {value}");
        }
    }
    Ok([position[0] as f32, position[1] as f32, position[2] as f32])
}

fn checked_flow_magnitude(vector: &[f64; 3]) -> Result<f64> {
    if vector.iter().any(|value| !value.is_finite()) {
        bail!("flow vector contains a nonfinite component");
    }
    let magnitude = vector[0].hypot(vector[1]).hypot(vector[2]);
    if !magnitude.is_finite() {
        bail!("flow-vector magnitude is not finite");
    }
    Ok(magnitude)
}

fn preflight_event_for_rerun(event: &RunLogEvent) -> Result<()> {
    if event.timestamp_ns() > i64::MAX as u64 {
        bail!(
            "run-log timestamp {} exceeds Rerun's signed-nanosecond timeline range",
            event.timestamp_ns()
        );
    }
    match event {
        RunLogEvent::ObjectPose { pose, .. } => {
            renderable_position(&pose.position)?;
        }
        RunLogEvent::SimSnapshot { objects, .. } => {
            for object in objects {
                renderable_position(&object.pose.position)?;
            }
        }
        RunLogEvent::FlowGt { flow, .. } | RunLogEvent::FlowPred { flow, .. } => {
            for vector in flow {
                checked_flow_magnitude(vector)?;
            }
        }
        RunLogEvent::PidMetric { value, .. }
        | RunLogEvent::GeometryMetric { value, .. }
        | RunLogEvent::EvaluationMetric { value, .. }
            if !value.is_finite() =>
        {
            bail!("Rerun scalar metric value must be finite");
        }
        _ => {}
    }
    Ok(())
}

fn projected_event_log_calls(event: &RunLogEvent, prepared: &PreparedEvent) -> usize {
    match event {
        RunLogEvent::FlowGt { flow, .. } | RunLogEvent::FlowPred { flow, .. } => flow.len(),
        RunLogEvent::FrameObserved { .. } => 0,
        RunLogEvent::AttributionLogged {
            faithfulness_check, ..
        } => {
            1 + usize::from(faithfulness_check.is_some())
                + usize::from(prepared.relevance.is_some())
        }
        _ => 1,
    }
}

fn projected_event_entity_path_bytes(
    event: &RunLogEvent,
    prepared: &PreparedEvent,
) -> Result<usize> {
    let path_bytes = match event {
        RunLogEvent::ObjectPose { object_id, .. } => "world/objects/"
            .len()
            .checked_add(entity_segment_len(object_id)?)
            .context("object-pose Rerun entity-path length overflow")?,
        RunLogEvent::FlowGt {
            object_id, flow, ..
        } => {
            let prefix_len = "flow/gt/"
                .len()
                .checked_add(entity_segment_len(object_id)?)
                .and_then(|value| value.checked_add(1))
                .context("ground-truth flow Rerun entity-path length overflow")?;
            indexed_entity_path_bytes(prefix_len, flow.len())?
        }
        RunLogEvent::FlowPred {
            source,
            object_id,
            flow,
            ..
        } => {
            let source_len = entity_segment_len(source)?;
            let object_id_len = entity_segment_len(object_id)?;
            let prefix_len = "flow/pred/"
                .len()
                .checked_add(source_len)
                .and_then(|value| value.checked_add(1))
                .and_then(|value| value.checked_add(object_id_len))
                .and_then(|value| value.checked_add(1))
                .context("predicted-flow Rerun entity-path length overflow")?;
            indexed_entity_path_bytes(prefix_len, flow.len())?
        }
        RunLogEvent::PidMetric { name, .. } => "pid/metrics/"
            .len()
            .checked_add(entity_segment_len(name)?)
            .context("PID metric Rerun entity-path length overflow")?,
        RunLogEvent::GeometryMetric { name, .. } => "pid/geometry/"
            .len()
            .checked_add(entity_segment_len(name)?)
            .context("geometry metric Rerun entity-path length overflow")?,
        RunLogEvent::EvaluationMetric { name, .. } => "evaluation/metrics/"
            .len()
            .checked_add(entity_segment_len(name)?)
            .context("evaluation metric Rerun entity-path length overflow")?,
        RunLogEvent::AttributionLogged {
            faithfulness_check, ..
        } => {
            let identity_len = prepared
                .attribution_identity
                .as_deref()
                .context("preflight omitted attribution identity")?
                .len();
            let mut total = "attributions/"
                .len()
                .checked_add(identity_len)
                .context("attribution Rerun entity-path length overflow")?;
            if faithfulness_check.is_some() {
                total = total
                    .checked_add("attributions/recorded_check/".len())
                    .and_then(|value| value.checked_add(identity_len))
                    .context("attribution recorded-check entity-path aggregate overflow")?;
            }
            if prepared.relevance.is_some() {
                total = total
                    .checked_add("attributions/relevance/".len())
                    .and_then(|value| value.checked_add(identity_len))
                    .context("attribution relevance entity-path aggregate overflow")?;
            }
            total
        }
        _ => projected_event_log_calls(event, prepared)
            .checked_mul(MAX_FIXED_ENTITY_PATH_BYTES)
            .context("fixed Rerun entity-path aggregate overflow")?,
    };
    Ok(path_bytes)
}

fn validate_manifest_artifacts(manifest: &RunManifest, events: &[RunLogEvent]) -> Result<()> {
    let mut supplied = manifest.artifacts.iter();
    for event in events {
        let RunLogEvent::ArtifactLogged {
            name,
            kind,
            uri,
            sha256,
            ..
        } = event
        else {
            continue;
        };
        let entry = supplied
            .next()
            .context("run-log manifest omits an event-derived artifact")?;
        let expected_content_hash = sha256
            .as_deref()
            .map(|digest| HashIdentity::sha256(HashRevision::FileBytesV1, digest))
            .transpose()
            .context("event-derived artifact hash is invalid")?;
        ensure!(
            entry.name.as_str() == name
                && entry.kind.as_str() == kind
                && entry.uri.as_str() == uri
                && entry.sha256.as_ref() == sha256.as_ref()
                && entry.content_hash.as_ref() == expected_content_hash.as_ref(),
            "run-log manifest artifact does not match the supplied event snapshot"
        );
    }
    ensure!(
        supplied.next().is_none(),
        "run-log manifest contains an artifact absent from the supplied event snapshot"
    );
    Ok(())
}

fn validate_manifest_matches_summary(
    manifest: &RunManifest,
    summary: &RunLogSummary,
    events: &[RunLogEvent],
) -> Result<()> {
    let expected_schema = summary
        .schema_version
        .unwrap_or(pid_runlog::RUN_LOG_SCHEMA_VERSION);
    ensure!(
        manifest.sidecar_schema_version == summary.sidecar_schema_version
            && manifest.schema_version == expected_schema
            && manifest.run_id == summary.run_id
            && manifest.config_hash == summary.config_hash
            && manifest.event_count == summary.event_count
            && manifest.trace_hash == summary.trace_hash
            && manifest.trace_hash_v2 == summary.trace_hash_v2
            && manifest.logical_trace_hash == summary.logical_trace_hash
            && manifest.logical_trace_hash_v3 == summary.logical_trace_hash_v3
            && manifest.hash_identities == summary.hash_identities
            && manifest.validation_errors == summary.validation_errors
            && manifest.validation_warnings == summary.validation_warnings,
        "run-log manifest does not match the supplied event snapshot"
    );
    validate_manifest_artifacts(manifest, events)?;

    // Event slices do not contain the source file's raw bytes, so this API
    // cannot independently prove `run_log_sha256`. It can and must still reject
    // an internally detached raw-file identity. The standalone converter builds
    // both fields from the exact byte snapshot it parsed.
    let raw_digest = manifest
        .run_log_sha256
        .as_deref()
        .context("run-log manifest omits its raw-file SHA-256 identity")?;
    let expected_run_log_hash = HashIdentity::sha256(HashRevision::FileBytesV1, raw_digest)
        .context("run-log manifest raw-file hash is invalid")?;
    ensure!(
        manifest.run_log_hash.as_ref() == Some(&expected_run_log_hash),
        "run-log manifest raw-file hash identity is internally inconsistent"
    );

    // Public fields allow a caller to deserialize or mutate anchors without
    // using `RunManifest::add_external_anchor_with_limits`. Replay the anchors
    // through that canonical validator before surfacing them as provenance.
    let mut validated_anchors = manifest.clone();
    validated_anchors.external_anchors.clear();
    for anchor in &manifest.external_anchors {
        validated_anchors
            .add_external_anchor_with_limits(anchor.clone(), RunLogLimits::default())
            .context("run-log manifest external anchor is invalid")?;
    }
    Ok(())
}

#[derive(Debug)]
struct PreparedRelevance {
    values: Vec<f64>,
    shape: Vec<usize>,
    sha256: String,
    uri: String,
}

impl PreparedRelevance {
    /// Heap bytes retained until the batch preflight has succeeded.
    ///
    /// Counting capacities, rather than only the scalar payload length, closes
    /// the high-rank/one-value case where a small NPY payload retains a large
    /// `shape` allocation for every attribution.
    fn retained_heap_bytes(&self) -> Result<usize> {
        let values = self
            .values
            .capacity()
            .checked_mul(size_of::<f64>())
            .context("prepared relevance value allocation overflow")?;
        let shape = self
            .shape
            .capacity()
            .checked_mul(size_of::<usize>())
            .context("prepared relevance shape allocation overflow")?;
        values
            .checked_add(shape)
            .and_then(|bytes| bytes.checked_add(self.sha256.capacity()))
            .and_then(|bytes| bytes.checked_add(self.uri.capacity()))
            .context("prepared relevance retained-byte count overflow")
    }
}

#[derive(Debug, Default)]
struct PreparedEvent {
    relevance: Option<PreparedRelevance>,
    attribution_identity: Option<String>,
}

pub struct RunLogRerunLogger<'a> {
    rec: &'a RecordingStream,
    /// A canonical run-log directory enables the otherwise-disabled external
    /// relevance-artifact capability. Relative paths may never escape it.
    artifact_base_dir: Option<PathBuf>,
}

impl<'a> RunLogRerunLogger<'a> {
    pub fn new(rec: &'a RecordingStream) -> Self {
        Self {
            rec,
            artifact_base_dir: None,
        }
    }

    /// Explicitly permit loading bounded `.npy` relevance artifacts confined to
    /// `run_log_dir`. Loading is disabled by default.
    pub fn with_external_artifact_loading(mut self, run_log_dir: impl AsRef<Path>) -> Result<Self> {
        let requested = run_log_dir.as_ref();
        let canonical = fs::canonicalize(requested).with_context(|| {
            format!(
                "failed to canonicalize run-log directory {}",
                requested.display()
            )
        })?;
        if !canonical.is_dir() {
            bail!(
                "external artifact base is not a directory: {}",
                canonical.display()
            );
        }
        self.artifact_base_dir = Some(canonical);
        Ok(self)
    }

    /// Do not dereference artifact paths embedded in run-log events.
    ///
    /// Provenance text and recorded compatibility flags are still emitted; only the
    /// optional loading of relevance arrays is disabled.
    pub fn without_external_artifact_loading(mut self) -> Self {
        self.artifact_base_dir = None;
        self
    }

    fn load_external_relevance(
        &self,
        artifact_uri: Option<&str>,
        metadata: &std::collections::BTreeMap<String, String>,
    ) -> Result<Option<PreparedRelevance>> {
        let Some(base) = &self.artifact_base_dir else {
            return Ok(None);
        };
        let Some(uri) = artifact_uri else {
            return Ok(None);
        };
        if Path::new(uri).extension().and_then(|value| value.to_str()) != Some("npy") {
            bail!("opted-in attribution artifact must be an .npy file");
        }

        let expected_sha256 = metadata
            .get("artifact_sha256")
            .context("opted-in attribution artifact is missing metadata.artifact_sha256")?;
        ensure!(
            is_lowercase_sha256(expected_sha256),
            "metadata.artifact_sha256 must be exactly 64 lowercase hexadecimal characters"
        );
        let expected_shape = metadata
            .get("relevance_shape")
            .context("opted-in attribution artifact is missing metadata.relevance_shape")?;

        let path = confined_regular_artifact(base, uri)?;
        let bytes = read_bounded_file(&path)?;
        let observed_sha256 = pid_runlog::sha256_hex(&bytes);
        ensure!(
            observed_sha256 == expected_sha256.as_str(),
            "attribution artifact SHA-256 mismatch for {uri}"
        );
        let (values, shape) = parse_npy_f64(&bytes)?;
        let observed_shape = relevance_shape_label(&shape);
        ensure!(
            observed_shape == expected_shape.as_str(),
            "attribution artifact shape mismatch for {uri}: expected {expected_shape:?}, observed {observed_shape:?}"
        );
        Ok(Some(PreparedRelevance {
            values,
            shape,
            sha256: observed_sha256,
            uri: uri.to_owned(),
        }))
    }

    fn prepare_event(&self, event: &RunLogEvent) -> Result<PreparedEvent> {
        preflight_event_for_rerun(event)?;
        match event {
            RunLogEvent::AttributionLogged {
                method,
                target_output,
                layer,
                modality,
                baseline,
                artifact_uri,
                metadata,
                ..
            } => Ok(PreparedEvent {
                relevance: self.load_external_relevance(artifact_uri.as_deref(), metadata)?,
                attribution_identity: Some(attribution_identity_segment(
                    method,
                    target_output,
                    layer.as_deref(),
                    modality.as_deref(),
                    baseline.as_deref(),
                )?),
            }),
            _ => Ok(PreparedEvent::default()),
        }
    }

    pub fn log_events(&self, events: &[RunLogEvent]) -> Result<()> {
        self.log_events_with_manifest(events, None).map(|_| ())
    }

    pub fn log_events_with_manifest(
        &self,
        events: &[RunLogEvent],
        manifest: Option<&RunManifest>,
    ) -> Result<RunLogSummary> {
        // Validate every numeric payload and load every explicitly authorized
        // relevance artifact before emitting the summary, manifest, or any event.
        // `--allow-invalid` may bypass run-log schema validation, but it may never
        // turn overflow/nonfinite data into viewer output or leave a partial
        // recording when a later external artifact is malformed.
        ensure!(
            events.len() <= MAX_RERUN_EVENTS,
            "Rerun event count {} exceeds the viewer limit {MAX_RERUN_EVENTS}",
            events.len()
        );
        let mut prepared = Vec::new();
        prepared
            .try_reserve_exact(events.len())
            .context("failed to reserve bounded Rerun event preflight")?;
        let mut prepared_relevance_bytes = 0usize;
        let mut projected_log_calls = 0usize;
        let mut projected_event_bytes = 0usize;
        let mut projected_entity_path_bytes = 0usize;
        for event in events {
            let prepared_event = self.prepare_event(event)?;
            if let Some(relevance) = &prepared_event.relevance {
                let bytes = relevance.retained_heap_bytes()?;
                prepared_relevance_bytes = prepared_relevance_bytes
                    .checked_add(bytes)
                    .context("prepared relevance aggregate overflow")?;
                if prepared_relevance_bytes > MAX_PREPARED_RELEVANCE_BYTES {
                    bail!(
                        "prepared relevance arrays exceed the {MAX_PREPARED_RELEVANCE_BYTES}-byte aggregate limit"
                    );
                }
            }
            projected_log_calls = projected_log_calls
                .checked_add(projected_event_log_calls(event, &prepared_event))
                .context("projected Rerun log-call count overflow")?;
            ensure!(
                projected_log_calls <= MAX_RERUN_PROJECTED_LOG_CALLS,
                "projected Rerun log calls exceed the viewer limit {MAX_RERUN_PROJECTED_LOG_CALLS}"
            );
            projected_entity_path_bytes = projected_entity_path_bytes
                .checked_add(projected_event_entity_path_bytes(event, &prepared_event)?)
                .context("projected Rerun entity-path byte count overflow")?;
            ensure!(
                projected_entity_path_bytes <= MAX_RERUN_PROJECTED_ENTITY_PATH_BYTES,
                "projected Rerun entity-path bytes exceed the viewer limit {MAX_RERUN_PROJECTED_ENTITY_PATH_BYTES}"
            );
            let event_bytes = serde_json::to_vec(event)
                .context("failed to size run-log event for Rerun conversion")?
                .len();
            projected_event_bytes = projected_event_bytes
                .checked_add(event_bytes)
                .context("projected Rerun event-byte count overflow")?;
            ensure!(
                projected_event_bytes <= MAX_RERUN_PROJECTED_EVENT_BYTES,
                "run-log event bytes exceed the viewer limit {MAX_RERUN_PROJECTED_EVENT_BYTES}"
            );
            prepared.push(prepared_event);
        }
        let summary = pid_runlog::summarize_events(events)?;
        if let Some(manifest) = manifest {
            let manifest_bytes = serde_json::to_vec(manifest)
                .context("failed to size run-log manifest for Rerun conversion")?
                .len();
            ensure!(
                manifest_bytes <= MAX_RERUN_MANIFEST_BYTES,
                "run-log manifest bytes exceed the viewer limit {MAX_RERUN_MANIFEST_BYTES}"
            );
            // Keep validation routines from cloning or traversing an oversized
            // caller-supplied manifest before the compact-size gate has passed.
            validate_manifest_matches_summary(manifest, &summary, events)?;
        }
        let fixed_and_diagnostic_calls = 15usize
            .checked_add(summary.validation_issues.len())
            .and_then(|value| {
                value.checked_add(
                    manifest
                        .map(|value| 2usize.saturating_add(value.artifacts.len()))
                        .unwrap_or(0),
                )
            })
            .context("projected Rerun summary log-call count overflow")?;
        projected_log_calls = projected_log_calls
            .checked_add(fixed_and_diagnostic_calls)
            .context("projected Rerun aggregate log-call count overflow")?;
        ensure!(
            projected_log_calls <= MAX_RERUN_PROJECTED_LOG_CALLS,
            "projected Rerun log calls exceed the viewer limit {MAX_RERUN_PROJECTED_LOG_CALLS}"
        );
        let fixed_entity_path_bytes = fixed_and_diagnostic_calls
            .checked_mul(MAX_FIXED_ENTITY_PATH_BYTES)
            .context("projected Rerun summary entity-path byte count overflow")?;
        projected_entity_path_bytes = projected_entity_path_bytes
            .checked_add(fixed_entity_path_bytes)
            .context("projected Rerun aggregate entity-path byte count overflow")?;
        ensure!(
            projected_entity_path_bytes <= MAX_RERUN_PROJECTED_ENTITY_PATH_BYTES,
            "projected Rerun entity-path bytes exceed the viewer limit {MAX_RERUN_PROJECTED_ENTITY_PATH_BYTES}"
        );
        self.log_summary(&summary)?;
        if let Some(manifest) = manifest {
            self.log_manifest(manifest)?;
        }
        for (event, prepared_event) in events.iter().zip(&prepared) {
            self.log_preflighted_event(event, prepared_event)?;
        }
        Ok(summary)
    }

    pub fn log_event(&self, event: &RunLogEvent) -> Result<()> {
        let prepared = self.prepare_event(event)?;
        ensure!(
            projected_event_log_calls(event, &prepared) <= MAX_RERUN_PROJECTED_LOG_CALLS,
            "projected Rerun log calls exceed the viewer limit {MAX_RERUN_PROJECTED_LOG_CALLS}"
        );
        ensure!(
            projected_event_entity_path_bytes(event, &prepared)?
                <= MAX_RERUN_PROJECTED_ENTITY_PATH_BYTES,
            "projected Rerun entity-path bytes exceed the viewer limit {MAX_RERUN_PROJECTED_ENTITY_PATH_BYTES}"
        );
        let event_bytes = serde_json::to_vec(event)
            .context("failed to size run-log event for Rerun conversion")?
            .len();
        ensure!(
            event_bytes <= MAX_RERUN_PROJECTED_EVENT_BYTES,
            "run-log event bytes exceed the viewer limit {MAX_RERUN_PROJECTED_EVENT_BYTES}"
        );
        self.log_preflighted_event(event, &prepared)
    }

    fn log_preflighted_event(&self, event: &RunLogEvent, prepared: &PreparedEvent) -> Result<()> {
        // Integer nanoseconds → Duration directly preserves every accepted tick.
        // Batch preflight rejects values beyond Rerun's signed-nanosecond range,
        // where the SDK would otherwise saturate distinct timestamps.
        self.rec
            .set_time("time", Duration::from_nanos(event.timestamp_ns()));
        match event {
            RunLogEvent::RunStarted { run_id, .. } => self.log_text("run/status", "INFO", run_id),
            RunLogEvent::RunEnded {
                status, message, ..
            } => self.log_text(
                "run/status",
                "INFO",
                &format!("{status:?}: {}", message.as_deref().unwrap_or("")),
            ),
            RunLogEvent::ObjectPose {
                object_id, pose, ..
            } => {
                let position = renderable_position(&pose.position)?;
                self.rec.log(
                    format!("world/objects/{}", entity_segment(object_id)),
                    &Points3D::new([position])
                        .with_colors([Color::from_rgb(230, 80, 60)])
                        .with_radii([0.025_f32]),
                )?;
                Ok(())
            }
            RunLogEvent::SimSnapshot { objects, .. } => self.log_snapshot(objects),
            RunLogEvent::FlowGt {
                object_id, flow, ..
            } => {
                let object_id = entity_segment(object_id);
                for (idx, vec) in flow.iter().enumerate() {
                    self.rec.log(
                        format!("flow/gt/{object_id}/{idx}"),
                        &Scalars::single(checked_flow_magnitude(vec)?),
                    )?;
                }
                Ok(())
            }
            RunLogEvent::FlowPred {
                source,
                object_id,
                flow,
                ..
            } => {
                let source = entity_segment(source);
                let object_id = entity_segment(object_id);
                for (idx, vec) in flow.iter().enumerate() {
                    self.rec.log(
                        format!("flow/pred/{source}/{object_id}/{idx}"),
                        &Scalars::single(checked_flow_magnitude(vec)?),
                    )?;
                }
                Ok(())
            }
            RunLogEvent::PidMetric { name, value, .. } => {
                self.rec.log(
                    format!("pid/metrics/{}", entity_segment(name)),
                    &Scalars::single(*value),
                )?;
                Ok(())
            }
            RunLogEvent::GeometryMetric { name, value, .. } => {
                self.rec.log(
                    format!("pid/geometry/{}", entity_segment(name)),
                    &Scalars::single(*value),
                )?;
                Ok(())
            }
            RunLogEvent::EvaluationMetric { name, value, .. } => {
                self.rec.log(
                    format!("evaluation/metrics/{}", entity_segment(name)),
                    &Scalars::single(*value),
                )?;
                Ok(())
            }
            RunLogEvent::LabelObserved { name, value, .. } => {
                self.log_text("labels/observed", "INFO", &format!("{name}: {value}"))
            }
            RunLogEvent::ActionApplied { action_type, .. } => {
                self.log_text("actions/applied", "INFO", action_type)
            }
            RunLogEvent::InterventionApplied {
                intervention_type, ..
            } => self.log_text("interventions/applied", "WARN", intervention_type),
            RunLogEvent::BridgeRequest {
                request_id, method, ..
            } => self.log_text(
                "bridge/requests",
                "INFO",
                &format!("{request_id}: {method}"),
            ),
            RunLogEvent::BridgeResponse {
                request_id,
                ok,
                message,
                ..
            } => self.log_text(
                "bridge/responses",
                if *ok { "INFO" } else { "ERROR" },
                &format!("{request_id}: {}", message.as_deref().unwrap_or("")),
            ),
            RunLogEvent::EmbeddingCaptured { name, dims, .. } => {
                self.log_text("vla/embeddings", "INFO", &format!("{name}: {dims:?}"))
            }
            RunLogEvent::EmbeddingContract {
                name, variables, ..
            } => self.log_text(
                "vla/embedding_contracts",
                "INFO",
                &format!(
                    "{name}: {}",
                    variables
                        .iter()
                        .map(|variable| format!(
                            "{}={} {:?}",
                            variable.variable, variable.source, variable.dims
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ),
            RunLogEvent::ArtifactLogged { name, uri, .. } => {
                self.log_text("artifacts", "INFO", &format!("{name}: {uri}"))
            }
            RunLogEvent::ConfigLogged { config_hash, .. } => {
                self.log_text("run/config", "INFO", config_hash)
            }
            // FrameObserved carries an opaque image/frame reference with no 3D
            // or scalar payload the run-log→Rerun adapter can plot, so it is
            // intentionally not surfaced here (the raw frame lives in its own
            // artifact). This is the one event type with no Rerun representation.
            RunLogEvent::FrameObserved { .. } => Ok(()),
            RunLogEvent::AttributionLogged {
                method,
                target_output,
                layer,
                modality,
                baseline,
                faithfulness_check,
                score_hash,
                artifact_uri,
                ..
            } => {
                let attribution_identity = prepared
                    .attribution_identity
                    .as_deref()
                    .context("preflight omitted attribution identity")?;
                // Plot the schema's legacy compatibility flag neutrally. Its
                // diagnostic/status/reason metadata defines the actual recorded
                // check; this Boolean is not a causal or mechanistic verdict.
                if let Some(passed) = faithfulness_check {
                    self.rec.log(
                        format!("attributions/recorded_check/{attribution_identity}"),
                        &Scalars::single(if *passed { 1.0 } else { 0.0 }),
                    )?;
                }
                if let Some(relevance) = &prepared.relevance {
                    self.rec.log(
                        format!("attributions/relevance/{attribution_identity}"),
                        &Scalars::new(relevance.values.iter().copied()),
                    )?;
                }
                let verdict = match faithfulness_check {
                    Some(true) => "PASS",
                    Some(false) => "FAIL",
                    None => "n/a",
                };
                let level = if matches!(faithfulness_check, Some(false)) {
                    "WARN"
                } else {
                    "INFO"
                };
                let artifact_provenance = if let Some(relevance) = &prepared.relevance {
                    format!(
                        "artifact={} exact_sha256={} shape={}",
                        relevance.uri,
                        relevance.sha256,
                        relevance_shape_label(&relevance.shape)
                    )
                } else {
                    format!(
                        "artifact={} external_loading=disabled",
                        artifact_uri.as_deref().unwrap_or("-")
                    )
                };
                self.log_text(
                    format!("attributions/{attribution_identity}"),
                    level,
                    &format!(
                        "{method} → {target_output} | recorded_check={verdict} layer={} modality={} baseline={} score={} {artifact_provenance}",
                        layer.as_deref().unwrap_or("-"),
                        modality.as_deref().unwrap_or("-"),
                        baseline.as_deref().unwrap_or("-"),
                        score_hash.as_deref().unwrap_or("-"),
                    ),
                )
            }
            RunLogEvent::ErrorLogged { message, .. } => self.log_text("errors", "ERROR", message),
            // `RunLogEvent` is `#[non_exhaustive]` upstream. A viewer may not silently drop an
            // event it cannot render: surface the variant tag so the run log stays auditable.
            other => self.log_text(
                "run/unhandled",
                "WARN",
                &format!("unrendered run-log event: {}", event_tag(other)),
            ),
        }
    }

    fn log_summary(&self, summary: &RunLogSummary) -> Result<()> {
        let trace_identity = replay_trace_v2_provenance(summary)?;
        self.rec.set_time("time", Duration::ZERO);
        self.log_text(
            "run/summary",
            if summary.validation_errors == 0 {
                "INFO"
            } else {
                "ERROR"
            },
            &format!(
                "run_id={} status={:?} events={} last_step={:?} {trace_identity} validation_errors={} validation_warnings={}",
                summary.run_id.as_deref().unwrap_or("<unknown>"),
                summary.status,
                summary.event_count,
                summary.last_step,
                summary.validation_errors,
                summary.validation_warnings
            ),
        )?;
        self.rec.log(
            "run/summary/event_count",
            &Scalars::single(summary.event_count as f64),
        )?;
        self.rec.log(
            "run/summary/actions",
            &Scalars::single(summary.actions as f64),
        )?;
        self.rec.log(
            "run/summary/bridge_records",
            &Scalars::single(summary.bridge_records as f64),
        )?;
        self.rec.log(
            "run/summary/flow_gt_records",
            &Scalars::single(summary.flow_gt_records as f64),
        )?;
        self.rec.log(
            "run/summary/flow_pred_records",
            &Scalars::single(summary.flow_pred_records as f64),
        )?;
        self.rec.log(
            "run/summary/evaluation_metrics",
            &Scalars::single(summary.evaluation_metrics as f64),
        )?;
        self.rec.log(
            "run/summary/pid_metric_events",
            &Scalars::single(summary.pid_metric_events as f64),
        )?;
        self.rec.log(
            "run/summary/geometry_metric_events",
            &Scalars::single(summary.geometry_metric_events as f64),
        )?;
        self.rec.log(
            "run/summary/evaluation_metric_events",
            &Scalars::single(summary.evaluation_metric_events as f64),
        )?;
        self.rec.log(
            "run/summary/labels",
            &Scalars::single(summary.labels as f64),
        )?;
        self.rec.log(
            "run/summary/embedding_contracts",
            &Scalars::single(summary.embedding_contracts as f64),
        )?;
        self.rec.log(
            "run/summary/validation_errors",
            &Scalars::single(summary.validation_errors as f64),
        )?;
        self.rec.log(
            "run/summary/validation_warnings",
            &Scalars::single(summary.validation_warnings as f64),
        )?;
        self.log_text("run/provenance/trace_hash_v2", "INFO", &trace_identity)?;
        for issue in &summary.validation_issues {
            self.log_text(
                "run/validation/issues",
                match issue.severity {
                    pid_runlog::ValidationSeverity::Error => "ERROR",
                    pid_runlog::ValidationSeverity::Warning => "WARN",
                    // `ValidationSeverity` is `#[non_exhaustive]` upstream; never hide an issue
                    // whose severity this build does not model.
                    _ => "WARN",
                },
                &format!("event={:?}: {}", issue.event_index, issue.message.as_str()),
            )?;
        }
        Ok(())
    }

    fn log_manifest(&self, manifest: &RunManifest) -> Result<()> {
        self.rec.set_time("time", Duration::ZERO);
        self.log_text(
            "run/provenance/run_log",
            "INFO",
            &format!(
                "{} sha256={}",
                manifest.run_log_uri,
                manifest.run_log_sha256.as_deref().unwrap_or("<unknown>")
            ),
        )?;
        self.log_text(
            "run/provenance/manifest",
            "INFO",
            &serde_json::to_string_pretty(manifest)?,
        )?;
        for artifact in &manifest.artifacts {
            self.log_text(
                "run/provenance/artifacts",
                "INFO",
                &format!(
                    "{} kind={} uri={} sha256={}",
                    artifact.name,
                    artifact.kind,
                    artifact.uri,
                    artifact.sha256.as_deref().unwrap_or("<unknown>")
                ),
            )?;
        }
        Ok(())
    }

    fn log_snapshot(&self, objects: &[SimObjectSnapshot]) -> Result<()> {
        let points = objects
            .iter()
            .map(|object| renderable_position(&object.pose.position))
            .collect::<Result<Vec<_>>>()?;
        self.rec.log(
            "world/objects",
            &Points3D::new(points)
                .with_colors([Color::from_rgb(230, 80, 60)])
                .with_radii([0.025_f32]),
        )?;
        Ok(())
    }

    fn log_text(&self, path: impl Into<String>, level: &str, message: &str) -> Result<()> {
        self.rec
            .log(path.into(), &TextLog::new(message).with_level(level))?;
        Ok(())
    }
}

/// The `type` tag of an event, as serialized in the canonical run log.
fn event_tag(event: &RunLogEvent) -> String {
    serde_json::to_value(event)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(|tag| tag.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pid_runlog::{Actor, ActorType, RunLogEvent, RunStatus, RUN_LOG_SCHEMA_VERSION};
    use rerun::RecordingStreamBuilder;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn actor() -> Actor {
        Actor {
            actor_type: ActorType::Script,
            actor_id: "rerun-test".to_string(),
            session_id: None,
        }
    }

    fn sample_events() -> Vec<RunLogEvent> {
        let config = json!({"dt": 0.1});
        let config_hash = pid_runlog::canonical_json_hash_v2(&config).unwrap();
        let payload = json!({ "dt": 0.1 });
        vec![
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "rerun-run".to_string(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            },
            RunLogEvent::ActionApplied {
                step: 0,
                timestamp_ns: 0,
                actor: actor(),
                action_type: "sim.step".to_string(),
                payload_hash: pid_runlog::canonical_json_hash_v2(&payload).unwrap(),
                payload,
            },
            RunLogEvent::PidMetric {
                step: 0,
                timestamp_ns: 1,
                name: "redundancy".to_string(),
                value: 0.1,
                metadata: BTreeMap::new(),
            },
            RunLogEvent::RunEnded {
                run_id: "rerun-run".to_string(),
                timestamp_ns: 2,
                status: RunStatus::Succeeded,
                message: None,
            },
        ]
    }

    #[test]
    fn logs_events_with_summary_diagnostics() -> Result<()> {
        let rec = RecordingStreamBuilder::new("runlog_summary_test").buffered()?;
        let summary =
            RunLogRerunLogger::new(&rec).log_events_with_manifest(&sample_events(), None)?;
        assert_eq!(summary.run_id.as_deref(), Some("rerun-run"));
        assert_eq!(summary.validation_errors, 0);
        assert_eq!(summary.actions, 1);
        Ok(())
    }

    #[test]
    fn summary_provenance_uses_explicit_lossless_replay_identity() -> Result<()> {
        let mut summary = pid_runlog::summarize_events(&sample_events())?;
        assert_eq!(
            replay_trace_v2_provenance(&summary)?,
            format!(
                "algorithm=sha256 revision=replay_trace_v2 digest={}",
                summary.trace_hash_v2
            )
        );

        summary
            .hash_identities
            .as_mut()
            .context("test summary omitted hash identities")?
            .replay_lossless
            .revision = HashRevision::ReplayTraceV1;
        assert!(replay_trace_v2_provenance(&summary).is_err());
        Ok(())
    }

    #[test]
    fn logs_attribution_with_recorded_check_flag() -> Result<()> {
        let rec = RecordingStreamBuilder::new("runlog_attribution_test").buffered()?;
        let config = json!({"k": 1});
        let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
        let events = vec![
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "attr-run".to_string(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            },
            RunLogEvent::AttributionLogged {
                timestamp_ns: 1,
                method: "lrp_epsilon".to_string(),
                target_output: "action_dim_0".to_string(),
                layer: Some("D_hidden_7".to_string()),
                modality: Some("vision".to_string()),
                baseline: Some("zero".to_string()),
                score_hash: Some(
                    "68deff9edee31c80d3a9252f3ac95e4935cd47c1bdb651dd8f1f2c6c0aa61fed".to_string(),
                ),
                faithfulness_check: Some(true),
                artifact_uri: None,
                metadata: BTreeMap::new(),
            },
            RunLogEvent::AttributionLogged {
                timestamp_ns: 2,
                method: "grad_x_input".to_string(),
                target_output: "action_dim_0".to_string(),
                layer: None,
                modality: None,
                baseline: Some("zero".to_string()),
                score_hash: Some(
                    "9bcc817d29aa98682819e1034eec7e8ccc3facf374520bddc87ee89491dd9668".to_string(),
                ),
                faithfulness_check: Some(false),
                artifact_uri: None,
                metadata: BTreeMap::new(),
            },
            RunLogEvent::RunEnded {
                run_id: "attr-run".to_string(),
                timestamp_ns: 3,
                status: RunStatus::Succeeded,
                message: None,
            },
        ];
        // The adapter must process both attributions (the failing one too) without
        // error, and the run-log summary must count them.
        let summary = RunLogRerunLogger::new(&rec).log_events_with_manifest(&events, None)?;
        assert_eq!(summary.validation_errors, 0);
        assert_eq!(summary.attributions, 2);
        Ok(())
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("pid-rerun-{label}-{}-{stamp}", std::process::id()));
        fs::create_dir(&path).unwrap();
        path
    }

    fn framed_npy(header_dict: &str, payload: &[u8]) -> Vec<u8> {
        let padding = (64 - ((NPY_V1_PREFIX_BYTES + header_dict.len() + 1) % 64)) % 64;
        let mut header = String::from(header_dict);
        header.extend(std::iter::repeat_n(' ', padding));
        header.push('\n');

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x93NUMPY");
        bytes.extend_from_slice(&[1, 0]);
        bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }

    /// Produce a `.npy` v1.0 little-endian f64 C-order array matching `numpy.save`.
    fn npy_f64_bytes(values: &[f64], shape: &[usize]) -> Vec<u8> {
        let shape_str = match shape.len() {
            0 => "()".to_owned(),
            1 => format!("({},)", shape[0]),
            _ => format!(
                "({})",
                shape
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        let header = format!("{{'descr': '<f8', 'fortran_order': False, 'shape': {shape_str}, }}");
        let mut payload = Vec::with_capacity(std::mem::size_of_val(values));
        for v in values {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        framed_npy(&header, &payload)
    }

    fn artifact_metadata(bytes: &[u8], shape: &[usize]) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("artifact_sha256".to_owned(), pid_runlog::sha256_hex(bytes)),
            ("relevance_shape".to_owned(), relevance_shape_label(shape)),
        ])
    }

    fn rejected_batch_without_writes(
        name: &str,
        events: &[RunLogEvent],
        artifact_base: Option<&Path>,
    ) -> Result<()> {
        let (recording, storage) = RecordingStreamBuilder::new(name).memory()?;
        recording.flush_blocking()?;
        let before = storage.num_msgs();
        let mut logger = RunLogRerunLogger::new(&recording);
        if let Some(base) = artifact_base {
            logger = logger.with_external_artifact_loading(base)?;
        }
        let result = logger.log_events_with_manifest(events, None);
        recording.flush_blocking()?;
        let after = storage.num_msgs();
        let _ = storage.take();
        assert!(result.is_err());
        assert_eq!(after, before);
        Ok(())
    }

    #[test]
    fn npy_reader_round_trips() {
        let values = vec![0.5, -1.25, 3.0, 0.0, 2.5, -0.75];
        let (got, shape) = parse_npy_f64(&npy_f64_bytes(&values, &[2, 3])).unwrap();
        assert_eq!(shape, vec![2, 3]);
        assert_eq!(got, values);
    }

    #[test]
    fn npy_reader_rejects_overflowing_shape_without_panicking() {
        let header =
            "{'descr': '<f8', 'fortran_order': False, 'shape': (2305843009213693952, 8), }";
        assert!(parse_npy_f64(&framed_npy(header, &[0; 8])).is_err());
    }

    #[test]
    fn npy_reader_rejects_structured_dtype_containing_f8_field() {
        let header =
            "{'descr': [('a', '<f8'), ('b', '<i4')], 'fortran_order': False, 'shape': (2,), }";
        assert!(
            parse_npy_f64(&framed_npy(header, &[0; 24])).is_err(),
            "structured dtype must be rejected, not decoded as flat f64"
        );
    }

    #[test]
    fn npy_reader_rejects_noncanonical_shape_spelling() {
        for shape in ["(01,)", "(1 ,)", "(1,2)", "(1,  2)"] {
            let header = format!("{{'descr': '<f8', 'fortran_order': False, 'shape': {shape}, }}");
            assert!(parse_npy_f64(&framed_npy(&header, &[0; 16])).is_err());
        }
    }

    #[test]
    fn npy_reader_rejects_nonzero_minor_version() {
        let mut bytes = npy_f64_bytes(&[1.0], &[1]);
        bytes[7] = 1;
        assert!(parse_npy_f64(&bytes).is_err());
    }

    #[test]
    fn npy_reader_rejects_trailing_bytes() {
        let mut bytes = npy_f64_bytes(&[1.0], &[1]);
        bytes.push(0);
        assert!(parse_npy_f64(&bytes).is_err());
    }

    #[test]
    fn npy_reader_rejects_nonfinite_values() {
        let bytes = npy_f64_bytes(&[f64::NAN], &[1]);
        assert!(parse_npy_f64(&bytes).is_err());
    }

    #[test]
    fn npy_reader_rejects_more_than_display_limit() {
        let values = vec![0.0; MAX_RELEVANCE_POINTS + 1];
        assert!(parse_npy_f64(&npy_f64_bytes(&values, &[values.len()])).is_err());
    }

    #[test]
    fn external_artifact_loading_is_disabled_by_default() -> Result<()> {
        let rec = RecordingStreamBuilder::new("runlog_no_external_reads").buffered()?;
        let logger = RunLogRerunLogger::new(&rec);
        assert!(logger
            .load_external_relevance(Some("/definitely/not/read/relevance.npy"), &BTreeMap::new())?
            .is_none());
        Ok(())
    }

    #[test]
    fn external_artifact_loading_rejects_traversal_and_absolute_paths() -> Result<()> {
        let root = unique_temp_dir("confinement");
        let base = root.join("run");
        fs::create_dir(&base)?;
        let bytes = npy_f64_bytes(&[1.0], &[1]);
        fs::write(root.join("outside.npy"), &bytes)?;
        let metadata = artifact_metadata(&bytes, &[1]);
        let rec = RecordingStreamBuilder::new("runlog_confinement").buffered()?;
        let logger = RunLogRerunLogger::new(&rec).with_external_artifact_loading(&base)?;

        assert!(logger
            .load_external_relevance(Some("../outside.npy"), &metadata)
            .is_err());
        assert!(logger
            .load_external_relevance(Some(root.join("outside.npy").to_str().unwrap()), &metadata)
            .is_err());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn external_artifact_loading_rejects_symlinks() -> Result<()> {
        use std::os::unix::fs::symlink;

        let base = unique_temp_dir("symlink");
        let bytes = npy_f64_bytes(&[1.0], &[1]);
        fs::write(base.join("target.npy"), &bytes)?;
        symlink("target.npy", base.join("link.npy"))?;
        let metadata = artifact_metadata(&bytes, &[1]);
        let rec = RecordingStreamBuilder::new("runlog_symlink").buffered()?;
        let logger = RunLogRerunLogger::new(&rec).with_external_artifact_loading(&base)?;

        assert!(logger
            .load_external_relevance(Some("link.npy"), &metadata)
            .is_err());
        fs::remove_dir_all(base)?;
        Ok(())
    }

    #[test]
    fn external_artifact_loading_rejects_nonregular_files() -> Result<()> {
        let base = unique_temp_dir("nonregular");
        fs::create_dir(base.join("directory.npy"))?;
        let rec = RecordingStreamBuilder::new("runlog_nonregular").buffered()?;
        let logger = RunLogRerunLogger::new(&rec).with_external_artifact_loading(&base)?;
        let metadata = artifact_metadata(&npy_f64_bytes(&[1.0], &[1]), &[1]);

        assert!(logger
            .load_external_relevance(Some("directory.npy"), &metadata)
            .is_err());
        fs::remove_dir_all(base)?;
        Ok(())
    }

    #[test]
    fn external_artifact_loading_rejects_missing_files() -> Result<()> {
        let base = unique_temp_dir("missing");
        let rec = RecordingStreamBuilder::new("runlog_missing").buffered()?;
        let logger = RunLogRerunLogger::new(&rec).with_external_artifact_loading(&base)?;
        let metadata = artifact_metadata(&npy_f64_bytes(&[1.0], &[1]), &[1]);

        assert!(logger
            .load_external_relevance(Some("missing.npy"), &metadata)
            .is_err());
        fs::remove_dir_all(base)?;
        Ok(())
    }

    #[test]
    fn external_artifact_loading_rejects_oversize_files() -> Result<()> {
        let base = unique_temp_dir("oversize");
        fs::write(
            base.join("oversize.npy"),
            vec![0_u8; MAX_RELEVANCE_FILE_BYTES + 1],
        )?;
        let rec = RecordingStreamBuilder::new("runlog_oversize").buffered()?;
        let logger = RunLogRerunLogger::new(&rec).with_external_artifact_loading(&base)?;
        let oversized = vec![0_u8; MAX_RELEVANCE_FILE_BYTES + 1];
        let metadata = artifact_metadata(&oversized, &[1]);

        assert!(logger
            .load_external_relevance(Some("oversize.npy"), &metadata)
            .is_err());
        fs::remove_dir_all(base)?;
        Ok(())
    }

    #[test]
    fn dynamic_entity_segments_are_injective_and_single_segment() {
        let nested = entity_segment("same/name");
        let escaped_looking = entity_segment("same%2Fname");
        assert_eq!(entity_segment("redundancy_1-a"), "s_redundancy_1-a");
        assert_eq!(nested, "s_same%2Fname");
        assert_eq!(escaped_looking, "s_same%252Fname");
        assert_ne!(nested, escaped_looking);
        assert!(!nested.contains('/'));
    }

    #[test]
    fn logs_attribution_relevance_heatmap_from_npy() -> Result<()> {
        let dir = unique_temp_dir("relevance");
        let npy = dir.join("relevance.npy");
        let relevance_bytes = npy_f64_bytes(&[0.1, 0.9, -0.4, 0.2], &[2, 2]);
        fs::write(&npy, &relevance_bytes)?;

        let rec = RecordingStreamBuilder::new("runlog_relevance_test").buffered()?;
        let config = json!({"k": 1});
        let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
        let events = vec![
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "rel-run".to_string(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            },
            RunLogEvent::AttributionLogged {
                timestamp_ns: 1,
                method: "lrp_epsilon".to_string(),
                target_output: "action_dim_0".to_string(),
                layer: None,
                modality: Some("vision".to_string()),
                baseline: Some("zero".to_string()),
                score_hash: Some(
                    "e1e1f202d19d2784d1bc5241db134ebe9bd68a78ba3ecf3f2c84f2f4a708ebeb".to_string(),
                ),
                faithfulness_check: Some(true),
                artifact_uri: Some("relevance.npy".to_string()),
                metadata: artifact_metadata(&relevance_bytes, &[2, 2]),
            },
            RunLogEvent::RunEnded {
                run_id: "rel-run".to_string(),
                timestamp_ns: 2,
                status: RunStatus::Succeeded,
                message: None,
            },
        ];
        let summary = RunLogRerunLogger::new(&rec)
            .with_external_artifact_loading(&dir)?
            .log_events_with_manifest(&events, None)?;
        assert_eq!(summary.validation_errors, 0);
        assert_eq!(summary.attributions, 1);

        // Default conversion never dereferences the absolute path, even when it
        // does not exist.
        let mut bad = events.clone();
        if let RunLogEvent::AttributionLogged { artifact_uri, .. } = &mut bad[2] {
            *artifact_uri = Some("/no/such/relevance.npy".to_string());
        }
        let rec2 = RecordingStreamBuilder::new("runlog_relevance_missing").buffered()?;
        let summary2 = RunLogRerunLogger::new(&rec2).log_events_with_manifest(&bad, None)?;
        assert_eq!(summary2.validation_errors, 0);

        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn batch_preflight_rejects_late_bad_artifact_before_logging() -> Result<()> {
        let dir = unique_temp_dir("late-artifact");
        let valid_bytes = npy_f64_bytes(&[0.25], &[1]);
        fs::write(dir.join("valid.npy"), &valid_bytes)?;
        let valid_metadata = artifact_metadata(&valid_bytes, &[1]);
        let missing_metadata = artifact_metadata(&npy_f64_bytes(&[0.5], &[1]), &[1]);
        let config = json!({"k": 1});
        let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
        let events = vec![
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "late-artifact-run".to_owned(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            },
            RunLogEvent::AttributionLogged {
                timestamp_ns: 1,
                method: "first".to_owned(),
                target_output: "action".to_owned(),
                layer: None,
                modality: None,
                baseline: None,
                score_hash: None,
                faithfulness_check: Some(true),
                artifact_uri: Some("valid.npy".to_owned()),
                metadata: valid_metadata,
            },
            RunLogEvent::AttributionLogged {
                timestamp_ns: 2,
                method: "second".to_owned(),
                target_output: "action".to_owned(),
                layer: None,
                modality: None,
                baseline: None,
                score_hash: None,
                faithfulness_check: Some(true),
                artifact_uri: Some("missing.npy".to_owned()),
                metadata: missing_metadata,
            },
            RunLogEvent::RunEnded {
                run_id: "late-artifact-run".to_owned(),
                timestamp_ns: 3,
                status: RunStatus::Succeeded,
                message: None,
            },
        ];

        rejected_batch_without_writes("runlog_late_artifact", &events, Some(&dir))?;
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn batch_preflight_counts_high_rank_relevance_shape_storage() -> Result<()> {
        let dir = unique_temp_dir("high-rank-relevance-budget");
        // The scalar payload is only eight bytes, while each retained shape has
        // enough dimensions to consume roughly ten KiB on a 64-bit target.
        // The old value-only accounting therefore admitted this entire batch.
        let shape = vec![1usize; 1_200];
        let relevance_bytes = npy_f64_bytes(&[0.25], &shape);
        assert!(relevance_bytes.len() <= MAX_RELEVANCE_FILE_BYTES);
        fs::write(dir.join("relevance.npy"), &relevance_bytes)?;
        let metadata = artifact_metadata(&relevance_bytes, &shape);

        let one = PreparedRelevance {
            values: vec![0.25],
            shape: shape.clone(),
            sha256: pid_runlog::sha256_hex(&relevance_bytes),
            uri: "relevance.npy".to_owned(),
        };
        let retained_per_event = one.retained_heap_bytes()?;
        assert!(retained_per_event > size_of::<f64>());
        let event_count = MAX_PREPARED_RELEVANCE_BYTES / retained_per_event + 1;
        assert!(event_count < MAX_RERUN_EVENTS);

        let events = (0..event_count)
            .map(|index| RunLogEvent::AttributionLogged {
                timestamp_ns: index as u64,
                method: "high_rank_fixture".to_owned(),
                target_output: "action".to_owned(),
                layer: None,
                modality: None,
                baseline: None,
                score_hash: None,
                faithfulness_check: None,
                artifact_uri: Some("relevance.npy".to_owned()),
                metadata: metadata.clone(),
            })
            .collect::<Vec<_>>();

        rejected_batch_without_writes("runlog_high_rank_relevance_budget", &events, Some(&dir))?;
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn opted_in_artifact_requires_exact_digest_and_shape_metadata() -> Result<()> {
        let dir = unique_temp_dir("artifact-binding-metadata");
        let bytes = npy_f64_bytes(&[1.0, 2.0], &[2]);
        fs::write(dir.join("relevance.npy"), &bytes)?;
        let rec = RecordingStreamBuilder::new("runlog_artifact_metadata").buffered()?;
        let logger = RunLogRerunLogger::new(&rec).with_external_artifact_loading(&dir)?;

        for metadata in [
            BTreeMap::new(),
            BTreeMap::from([
                ("artifact_sha256".to_owned(), "A".repeat(64)),
                ("relevance_shape".to_owned(), "2".to_owned()),
            ]),
            BTreeMap::from([("artifact_sha256".to_owned(), pid_runlog::sha256_hex(&bytes))]),
            BTreeMap::from([
                ("artifact_sha256".to_owned(), pid_runlog::sha256_hex(&bytes)),
                ("relevance_shape".to_owned(), "1x2".to_owned()),
            ]),
        ] {
            assert!(logger
                .load_external_relevance(Some("relevance.npy"), &metadata)
                .is_err());
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn artifact_substitution_is_rejected_before_any_rerun_write() -> Result<()> {
        let dir = unique_temp_dir("artifact-substitution");
        let expected = npy_f64_bytes(&[1.0, 2.0], &[2]);
        let substituted = npy_f64_bytes(&[3.0, 4.0], &[2]);
        assert_eq!(expected.len(), substituted.len());
        fs::write(dir.join("relevance.npy"), &substituted)?;
        let events = vec![RunLogEvent::AttributionLogged {
            timestamp_ns: 1,
            method: "lrp_epsilon".to_owned(),
            target_output: "action_dim_0".to_owned(),
            layer: Some("decoder".to_owned()),
            modality: Some("vision".to_owned()),
            baseline: Some("zero".to_owned()),
            score_hash: None,
            faithfulness_check: Some(true),
            artifact_uri: Some("relevance.npy".to_owned()),
            metadata: artifact_metadata(&expected, &[2]),
        }];

        rejected_batch_without_writes("runlog_artifact_substitution", &events, Some(&dir))?;
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn attribution_identity_separates_targets_layers_modalities_and_baselines() -> Result<()> {
        let base = attribution_identity_segment(
            "lrp",
            "action_0",
            Some("layer_1"),
            Some("vision"),
            Some("zero"),
        )?;
        for distinct in [
            attribution_identity_segment(
                "lrp",
                "action_1",
                Some("layer_1"),
                Some("vision"),
                Some("zero"),
            )?,
            attribution_identity_segment(
                "lrp",
                "action_0",
                Some("layer_2"),
                Some("vision"),
                Some("zero"),
            )?,
            attribution_identity_segment(
                "lrp",
                "action_0",
                Some("layer_1"),
                Some("language"),
                Some("zero"),
            )?,
            attribution_identity_segment(
                "lrp",
                "action_0",
                Some("layer_1"),
                Some("vision"),
                Some("mean"),
            )?,
        ] {
            assert_ne!(base, distinct);
        }
        Ok(())
    }

    #[test]
    fn preflight_rejects_unrenderable_object_and_snapshot_positions() -> Result<()> {
        let pose = pid_runlog::Pose {
            position: [f64::from(f32::MAX) * 2.0, 0.0, 0.0],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        };
        let object_pose = RunLogEvent::ObjectPose {
            step: 0,
            timestamp_ns: 0,
            object_id: "object".to_owned(),
            pose: pose.clone(),
        };
        let snapshot = RunLogEvent::SimSnapshot {
            step: 0,
            timestamp_ns: 0,
            objects: vec![SimObjectSnapshot {
                object_id: "object".to_owned(),
                pose,
                velocity: [0.0; 3],
            }],
            metadata: BTreeMap::new(),
        };

        rejected_batch_without_writes("runlog_object_position_preflight", &[object_pose], None)?;
        rejected_batch_without_writes("runlog_snapshot_position_preflight", &[snapshot], None)?;
        Ok(())
    }

    #[test]
    fn preflight_rejects_nonfinite_flow_magnitude() -> Result<()> {
        let event = RunLogEvent::FlowGt {
            step: 0,
            timestamp_ns: 0,
            object_id: "object".to_owned(),
            flow: vec![[f64::MAX, f64::MAX, 0.0]],
        };
        rejected_batch_without_writes("runlog_flow_preflight", &[event], None)
    }

    #[test]
    fn preflight_rejects_nonfinite_metric_before_batch_logging() -> Result<()> {
        let mut events = sample_events();
        events.insert(
            3,
            RunLogEvent::EvaluationMetric {
                step: 0,
                timestamp_ns: 1,
                name: "invalid".to_owned(),
                value: f64::INFINITY,
                metadata: BTreeMap::new(),
            },
        );
        rejected_batch_without_writes("runlog_metric_preflight", &events, None)
    }

    #[test]
    fn preflight_rejects_timestamp_saturation_before_batch_logging() -> Result<()> {
        let mut events = sample_events();
        if let RunLogEvent::PidMetric { timestamp_ns, .. } = &mut events[3] {
            *timestamp_ns = (i64::MAX as u64) + 1;
        }
        rejected_batch_without_writes("runlog_timestamp_preflight", &events, None)?;

        let rec = RecordingStreamBuilder::new("runlog_timestamp_boundary").buffered()?;
        let boundary = RunLogEvent::PidMetric {
            step: 0,
            timestamp_ns: i64::MAX as u64,
            name: "boundary".to_owned(),
            value: 1.0,
            metadata: BTreeMap::new(),
        };
        RunLogRerunLogger::new(&rec).log_event(&boundary)
    }

    #[test]
    fn projected_flow_amplification_is_rejected_without_writes() -> Result<()> {
        let events = vec![RunLogEvent::FlowGt {
            step: 0,
            timestamp_ns: 0,
            object_id: "object".to_owned(),
            flow: vec![[0.0; 3]; MAX_RERUN_PROJECTED_LOG_CALLS + 1],
        }];
        rejected_batch_without_writes("runlog_flow_budget", &events, None)
    }

    fn long_identifier_flow(predicted: bool) -> RunLogEvent {
        // One MiB is accepted by the canonical reader's per-string limit. Only
        // 22 vectors are needed for percent encoding of this identifier to
        // exceed the separate 64 MiB generated-path budget, while both the
        // serialized event and projected call count remain comfortably bounded.
        let identifier = "/".repeat(1024 * 1024);
        let flow = vec![[0.0; 3]; 22];
        if predicted {
            RunLogEvent::FlowPred {
                step: 0,
                timestamp_ns: 0,
                source: identifier.clone(),
                object_id: identifier,
                horizon_steps: 1,
                flow,
                metadata: BTreeMap::new(),
            }
        } else {
            RunLogEvent::FlowGt {
                step: 0,
                timestamp_ns: 0,
                object_id: identifier,
                flow,
            }
        }
    }

    #[test]
    fn long_identifier_flow_paths_are_rejected_below_other_budgets_without_writes() -> Result<()> {
        for (name, event) in [
            ("runlog_long_flow_gt", long_identifier_flow(false)),
            ("runlog_long_flow_pred", long_identifier_flow(true)),
        ] {
            let prepared = PreparedEvent::default();
            assert!(projected_event_log_calls(&event, &prepared) < MAX_RERUN_PROJECTED_LOG_CALLS);
            assert!(serde_json::to_vec(&event)?.len() < MAX_RERUN_PROJECTED_EVENT_BYTES);
            assert!(
                projected_event_entity_path_bytes(&event, &prepared)?
                    > MAX_RERUN_PROJECTED_ENTITY_PATH_BYTES
            );
            rejected_batch_without_writes(name, &[event], None)?;
        }
        Ok(())
    }

    #[test]
    fn direct_event_flow_amplification_is_rejected_without_writes() -> Result<()> {
        let (recording, storage) =
            RecordingStreamBuilder::new("runlog_direct_flow_budget").memory()?;
        recording.flush_blocking()?;
        let before = storage.num_msgs();
        let event = RunLogEvent::FlowGt {
            step: 0,
            timestamp_ns: 0,
            object_id: "object".to_owned(),
            flow: vec![[0.0; 3]; MAX_RERUN_PROJECTED_LOG_CALLS + 1],
        };
        assert!(RunLogRerunLogger::new(&recording)
            .log_event(&event)
            .is_err());
        recording.flush_blocking()?;
        assert_eq!(storage.num_msgs(), before);
        let _ = storage.take();
        Ok(())
    }

    #[test]
    fn direct_long_identifier_flow_is_rejected_without_writes() -> Result<()> {
        let (recording, storage) =
            RecordingStreamBuilder::new("runlog_direct_long_flow_budget").memory()?;
        recording.flush_blocking()?;
        let before = storage.num_msgs();
        let event = long_identifier_flow(false);
        assert!(RunLogRerunLogger::new(&recording)
            .log_event(&event)
            .is_err());
        recording.flush_blocking()?;
        assert_eq!(storage.num_msgs(), before);
        let _ = storage.take();
        Ok(())
    }

    #[test]
    fn logs_manifest_diagnostics() -> Result<()> {
        let rec = RecordingStreamBuilder::new("runlog_manifest_test").buffered()?;
        let events = sample_events();
        // `RunManifest` is `#[non_exhaustive]` in the current pid-runlog review surface, so build
        // it through the real
        // constructor against a real run log rather than hand-rolling the struct.
        let dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = dir.join(format!("pid-rerun-manifest-{stamp}.jsonl"));
        let mut writer = pid_runlog::RunLogWriter::create(&path)?;
        for event in &events {
            writer.append(event)?;
        }
        drop(writer);

        let manifest = pid_runlog::manifest_for_events(&path, &events)?;
        let logged =
            RunLogRerunLogger::new(&rec).log_events_with_manifest(&events, Some(&manifest))?;
        assert_eq!(logged.trace_hash, manifest.trace_hash);

        let _ = std::fs::remove_file(&path);
        Ok(())
    }

    #[test]
    fn mismatched_manifest_is_rejected_without_writes() -> Result<()> {
        let events = sample_events();
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("run.jsonl");
        let mut writer = pid_runlog::RunLogWriter::create(&path)?;
        for event in &events {
            writer.append(event)?;
        }
        drop(writer);
        let mut manifest = pid_runlog::manifest_for_events(&path, &events)?;
        manifest.event_count += 1;

        let (recording, storage) =
            RecordingStreamBuilder::new("runlog_manifest_mismatch").memory()?;
        recording.flush_blocking()?;
        let before = storage.num_msgs();
        assert!(RunLogRerunLogger::new(&recording)
            .log_events_with_manifest(&events, Some(&manifest))
            .is_err());
        recording.flush_blocking()?;
        assert_eq!(storage.num_msgs(), before);
        let _ = storage.take();
        Ok(())
    }

    fn events_with_artifact() -> Vec<RunLogEvent> {
        let mut events = sample_events();
        events.insert(
            events.len() - 1,
            RunLogEvent::ArtifactLogged {
                timestamp_ns: 1,
                name: "estimate".to_owned(),
                kind: "application/json".to_owned(),
                uri: "artifacts/estimate.json".to_owned(),
                sha256: Some("0".repeat(64)),
                metadata: BTreeMap::new(),
            },
        );
        events
    }

    fn manifest_for_test_events(
        dir: &tempfile::TempDir,
        events: &[RunLogEvent],
    ) -> Result<RunManifest> {
        let path = dir.path().join("run.jsonl");
        let mut writer = pid_runlog::RunLogWriter::create(&path)?;
        for event in events {
            writer.append(event)?;
        }
        drop(writer);
        pid_runlog::manifest_for_events(path, events)
    }

    fn rejected_manifest_without_writes(
        name: &str,
        events: &[RunLogEvent],
        manifest: &RunManifest,
    ) -> Result<()> {
        let (recording, storage) = RecordingStreamBuilder::new(name).memory()?;
        recording.flush_blocking()?;
        let before = storage.num_msgs();
        assert!(RunLogRerunLogger::new(&recording)
            .log_events_with_manifest(events, Some(manifest))
            .is_err());
        recording.flush_blocking()?;
        assert_eq!(storage.num_msgs(), before);
        let _ = storage.take();
        Ok(())
    }

    #[test]
    fn mismatched_manifest_sidecar_schema_is_rejected_without_writes() -> Result<()> {
        let events = sample_events();
        let dir = tempfile::tempdir()?;
        let mut manifest = manifest_for_test_events(&dir, &events)?;
        manifest.sidecar_schema_version += 1;
        rejected_manifest_without_writes("runlog_manifest_sidecar_mismatch", &events, &manifest)
    }

    #[test]
    fn mismatched_manifest_artifact_is_rejected_without_writes() -> Result<()> {
        let events = events_with_artifact();
        let dir = tempfile::tempdir()?;
        let base_manifest = manifest_for_test_events(&dir, &events)?;

        let mut manifest = base_manifest.clone();
        manifest.artifacts[0].uri = "artifacts/substituted.json".to_owned();
        rejected_manifest_without_writes("runlog_manifest_artifact_mismatch", &events, &manifest)?;

        let mut manifest = base_manifest;
        manifest.artifacts[0].content_hash = None;
        rejected_manifest_without_writes(
            "runlog_manifest_artifact_identity_mismatch",
            &events,
            &manifest,
        )
    }

    #[test]
    fn detached_manifest_raw_file_identity_is_rejected_without_writes() -> Result<()> {
        let events = sample_events();
        let dir = tempfile::tempdir()?;
        let mut manifest = manifest_for_test_events(&dir, &events)?;
        manifest.run_log_hash = None;
        rejected_manifest_without_writes(
            "runlog_manifest_raw_identity_mismatch",
            &events,
            &manifest,
        )?;

        let mut unbound_manifest = manifest_for_test_events(&dir, &events)?;
        unbound_manifest.run_log_sha256 = None;
        unbound_manifest.run_log_hash = None;
        rejected_manifest_without_writes(
            "runlog_manifest_raw_identity_missing",
            &events,
            &unbound_manifest,
        )
    }

    #[test]
    fn invalid_manifest_external_anchor_is_rejected_without_writes() -> Result<()> {
        let events = sample_events();
        let dir = tempfile::tempdir()?;
        let mut manifest = manifest_for_test_events(&dir, &events)?;
        manifest.external_anchors.push(pid_runlog::ExternalAnchor {
            provider: String::new(),
            uri: "https://example.invalid/receipt".to_owned(),
            anchored_hash: manifest
                .run_log_hash
                .clone()
                .context("test manifest omitted raw-file identity")?,
            signature: None,
        });

        rejected_manifest_without_writes(
            "runlog_manifest_invalid_external_anchor",
            &events,
            &manifest,
        )
    }
}
