use anyhow::Result;
use pid_runlog::{RunLogEvent, RunLogSummary, RunManifest, SimObjectSnapshot};
use rerun::{Color, Points3D, RecordingStream, Scalars, TextLog};
use std::time::Duration;

/// Cap on the number of relevance values surfaced to Rerun per attribution, to keep
/// the recording bounded for large `(T, d)` relevance tensors.
const MAX_RELEVANCE_POINTS: usize = 1024;

/// Minimal, dependency-free reader for NumPy `.npy` v1.0 arrays of little-endian
/// `float64` in C order — the exact format `numpy.save` writes for the attribution
/// probe's relevance arrays. Returns `(flattened_values, shape)` in C order, or
/// `None` on any deviation (other dtype, Fortran order, version, or I/O error); the
/// caller treats `None` as "no heatmap available" and falls back gracefully.
fn read_npy_f64(path: impl AsRef<std::path::Path>) -> Option<(Vec<f64>, Vec<usize>)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 10 || &bytes[0..6] != b"\x93NUMPY" {
        return None;
    }
    // Only v1.0 (2-byte little-endian header length).
    if bytes[6] != 1 {
        return None;
    }
    let header_len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    let header_start = 10;
    let data_start = header_start + header_len;
    if bytes.len() < data_start {
        return None;
    }
    let header = std::str::from_utf8(&bytes[header_start..data_start]).ok()?;
    // Require a plain little-endian f64 descr, C order. A substring match on
    // "'<f8'" would also accept a STRUCTURED dtype that merely contains an
    // "<f8" field (e.g. `[('a','<f8'),('b','<i4')]`), whose bytes are not a
    // flat f64 array and would decode to garbage — so match the exact
    // `'descr': '<f8'` (single- or double-quoted) form instead.
    let descr_ok = [
        "'descr': '<f8'",
        "'descr':'<f8'",
        "\"descr\": \"<f8\"",
        "\"descr\":\"<f8\"",
    ]
    .iter()
    .any(|needle| header.contains(needle));
    if !descr_ok {
        return None;
    }
    if header.contains("'fortran_order': True") || header.contains("\"fortran_order\": True") {
        return None;
    }
    let shape = parse_npy_shape(header)?;
    // Element count and byte size are computed with CHECKED arithmetic: a crafted
    // (or merely absurd) shape header must yield `None`, never a wrapped count that
    // bypasses the length guard and then aborts in `Vec::with_capacity`. `try_fold`
    // over an empty shape yields the initial `1`, preserving scalar-array behavior.
    let count: usize = shape
        .iter()
        .try_fold(1usize, |acc, &d| acc.checked_mul(d))?;
    let n_bytes = count.checked_mul(8)?;
    let data = &bytes[data_start..];
    if data.len() < n_bytes {
        return None;
    }
    // The caller keeps only the first `MAX_RELEVANCE_POINTS` values, so cap the
    // initial RESERVATION at that — not the final length: the equality check below
    // still reads all `count` in-bounds values, so the vec may grow past the hint.
    // (`count` is already bounded by the `data.len() < n_bytes` guard above, i.e.
    // by the actual file size, so this cannot over-allocate for a truncated file.)
    let mut values = Vec::with_capacity(count.min(MAX_RELEVANCE_POINTS));
    for chunk in data.chunks_exact(8).take(count) {
        values.push(f64::from_le_bytes(chunk.try_into().ok()?));
    }
    if values.len() != count {
        return None;
    }
    Some((values, shape))
}

/// Parse the `shape` tuple out of an `.npy` header dict string.
fn parse_npy_shape(header: &str) -> Option<Vec<usize>> {
    let key = header
        .find("'shape'")
        .or_else(|| header.find("\"shape\""))?;
    let open = header[key..].find('(')? + key;
    let close = header[open..].find(')')? + open;
    let inner = &header[open + 1..close];
    let mut dims = Vec::new();
    for part in inner.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        dims.push(p.parse::<usize>().ok()?);
    }
    Some(dims)
}

pub struct RunLogRerunLogger<'a> {
    rec: &'a RecordingStream,
    /// Directory that relative `artifact_uri`s resolve against — normally the
    /// run log's own directory. Without it a relative uri would resolve against
    /// the converter's current working directory, so an attribution relevance
    /// artifact written next to the run log would be silently skipped when the
    /// converter runs from elsewhere.
    artifact_base_dir: Option<std::path::PathBuf>,
}

impl<'a> RunLogRerunLogger<'a> {
    pub fn new(rec: &'a RecordingStream) -> Self {
        Self {
            rec,
            artifact_base_dir: None,
        }
    }

    /// Resolve relative `artifact_uri`s against `dir` (typically the run log's
    /// parent directory).
    pub fn with_artifact_base_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.artifact_base_dir = Some(dir.into());
        self
    }

    /// Resolve an artifact uri: absolute paths as-is, relative paths against the
    /// configured base dir (falling back to the raw uri if none is set).
    fn resolve_artifact_uri(&self, uri: &str) -> std::path::PathBuf {
        let path = std::path::Path::new(uri);
        match &self.artifact_base_dir {
            Some(base) if path.is_relative() => base.join(path),
            _ => path.to_path_buf(),
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
        let summary = pid_runlog::summarize_events(events)?;
        self.log_summary(&summary)?;
        if let Some(manifest) = manifest {
            self.log_manifest(manifest)?;
        }
        for event in events {
            self.log_event(event)?;
        }
        Ok(summary)
    }

    pub fn log_event(&self, event: &RunLogEvent) -> Result<()> {
        // Integer nanoseconds → Duration directly; going through f64 seconds
        // loses precision at epoch-scale timestamps (f64 has ~15–16 significant
        // digits, and ns-since-epoch already needs ~19), collapsing distinct
        // ticks onto the same Rerun time.
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
                self.rec.log(
                    format!("world/objects/{object_id}"),
                    &Points3D::new([[
                        pose.position[0] as f32,
                        pose.position[1] as f32,
                        pose.position[2] as f32,
                    ]])
                    .with_colors([Color::from_rgb(230, 80, 60)])
                    .with_radii([0.025_f32]),
                )?;
                Ok(())
            }
            RunLogEvent::SimSnapshot { objects, .. } => self.log_snapshot(objects),
            RunLogEvent::FlowGt {
                object_id, flow, ..
            } => {
                for (idx, vec) in flow.iter().enumerate() {
                    self.rec.log(
                        format!("flow/gt/{object_id}/{idx}"),
                        &Scalars::single(
                            (vec[0] * vec[0] + vec[1] * vec[1] + vec[2] * vec[2]).sqrt(),
                        ),
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
                for (idx, vec) in flow.iter().enumerate() {
                    self.rec.log(
                        format!("flow/pred/{source}/{object_id}/{idx}"),
                        &Scalars::single(
                            (vec[0] * vec[0] + vec[1] * vec[1] + vec[2] * vec[2]).sqrt(),
                        ),
                    )?;
                }
                Ok(())
            }
            RunLogEvent::PidMetric { name, value, .. } => {
                self.rec
                    .log(format!("pid/metrics/{name}"), &Scalars::single(*value))?;
                Ok(())
            }
            RunLogEvent::GeometryMetric { name, value, .. } => {
                self.rec
                    .log(format!("pid/geometry/{name}"), &Scalars::single(*value))?;
                Ok(())
            }
            RunLogEvent::EvaluationMetric { name, value, .. } => {
                self.rec.log(
                    format!("evaluation/metrics/{name}"),
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
                // Plottable faithfulness verdict (1.0 pass / 0.0 fail) so a viewer
                // can chart which attributions earned trust over the run; an
                // attribution that fails its faithfulness check (§6.10) cannot
                // corroborate or falsify a PID claim, so surface it prominently.
                if let Some(passed) = faithfulness_check {
                    self.rec.log(
                        format!("attributions/faithfulness/{method}"),
                        &Scalars::single(if *passed { 1.0 } else { 0.0 }),
                    )?;
                }
                // Best-effort: if the relevance was saved as a `.npy` artifact, surface
                // the actual per-element relevance values (capped) as a multi-value
                // Scalars series so the heatmap is inspectable in the viewer — not just
                // the pass/fail verdict. Any read/parse failure is silently skipped.
                if let Some(uri) = artifact_uri {
                    if uri.ends_with(".npy") {
                        let resolved = self.resolve_artifact_uri(uri);
                        match read_npy_f64(&resolved) {
                            Some((values, _shape)) => {
                                let capped: Vec<f64> =
                                    values.into_iter().take(MAX_RELEVANCE_POINTS).collect();
                                self.rec.log(
                                    format!("attributions/relevance/{method}"),
                                    &Scalars::new(capped),
                                )?;
                            }
                            None => {
                                // Surface the miss rather than dropping it: a
                                // relevance artifact that exists but cannot be
                                // read (moved, wrong dtype, truncated) should be
                                // visible in the viewer, not silently absent.
                                self.log_text(
                                    format!("attributions/relevance/{method}"),
                                    "WARN",
                                    &format!(
                                        "relevance artifact unreadable: {}",
                                        resolved.display()
                                    ),
                                )?;
                            }
                        }
                    }
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
                self.log_text(
                    format!("attributions/{method}"),
                    level,
                    &format!(
                        "{method} → {target_output} | faithfulness={verdict} layer={} modality={} baseline={} score={}",
                        layer.as_deref().unwrap_or("-"),
                        modality.as_deref().unwrap_or("-"),
                        baseline.as_deref().unwrap_or("-"),
                        score_hash.as_deref().unwrap_or("-"),
                    ),
                )
            }
            RunLogEvent::ErrorLogged { message, .. } => self.log_text("errors", "ERROR", message),
        }
    }

    fn log_summary(&self, summary: &RunLogSummary) -> Result<()> {
        self.rec.set_time("time", Duration::ZERO);
        self.log_text(
            "run/summary",
            if summary.validation_errors == 0 {
                "INFO"
            } else {
                "ERROR"
            },
            &format!(
                "run_id={} status={:?} events={} last_step={:?} trace_hash={} validation_errors={} validation_warnings={}",
                summary.run_id.as_deref().unwrap_or("<unknown>"),
                summary.status,
                summary.event_count,
                summary.last_step,
                summary.trace_hash,
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
        self.log_text("run/provenance/trace_hash", "INFO", &summary.trace_hash)?;
        for issue in &summary.validation_issues {
            self.log_text(
                "run/validation/issues",
                match issue.severity {
                    pid_runlog::ValidationSeverity::Error => "ERROR",
                    pid_runlog::ValidationSeverity::Warning => "WARN",
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
        let points = objects.iter().map(|object| {
            [
                object.pose.position[0] as f32,
                object.pose.position[1] as f32,
                object.pose.position[2] as f32,
            ]
        });
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
        let payload = json!({ "dt": 0.1 });
        vec![
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "rerun-run".to_string(),
                timestamp_ns: 0,
                config_hash: pid_runlog::canonical_json_hash(&json!({"dt": 0.1})).unwrap(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::ActionApplied {
                step: 0,
                timestamp_ns: 0,
                actor: actor(),
                action_type: "sim.step".to_string(),
                payload_hash: pid_runlog::canonical_json_hash(&payload).unwrap(),
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
    fn logs_attribution_with_faithfulness_verdict() -> Result<()> {
        let rec = RecordingStreamBuilder::new("runlog_attribution_test").buffered()?;
        let events = vec![
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "attr-run".to_string(),
                timestamp_ns: 0,
                config_hash: pid_runlog::canonical_json_hash(&json!({"k": 1})).unwrap(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::AttributionLogged {
                timestamp_ns: 1,
                method: "lrp_epsilon".to_string(),
                target_output: "action_dim_0".to_string(),
                layer: Some("D_hidden_7".to_string()),
                modality: Some("vision".to_string()),
                baseline: Some("zero".to_string()),
                score_hash: Some("deadbeef".to_string()),
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
                score_hash: Some("cafef00d".to_string()),
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

    /// Write a `.npy` v1.0 little-endian f64 C-order array (matching `numpy.save`).
    fn write_npy_f64(path: &std::path::Path, values: &[f64], shape: &[usize]) {
        let shape_str = match shape.len() {
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
        let header =
            format!("{{'descr': '<f8', 'fortran_order': False, 'shape': {shape_str}, }}\n");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x93NUMPY");
        bytes.push(1);
        bytes.push(0);
        bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
        bytes.extend_from_slice(header.as_bytes());
        for v in values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn npy_reader_round_trips() {
        let dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = dir.join(format!("pid-rerun-npy-{stamp}.npy"));
        let values = vec![0.5, -1.25, 3.0, 0.0, 2.5, -0.75];
        write_npy_f64(&path, &values, &[2, 3]);
        let (got, shape) = read_npy_f64(path.to_str().unwrap()).expect("npy parses");
        assert_eq!(shape, vec![2, 3]);
        assert_eq!(got, values);
        // 1-D shape too.
        write_npy_f64(&path, &values, &[6]);
        let (got1, shape1) = read_npy_f64(path.to_str().unwrap()).unwrap();
        assert_eq!(shape1, vec![6]);
        assert_eq!(got1, values);
        // A non-existent / non-npy path returns None, never panics.
        assert!(read_npy_f64("/no/such/file.npy").is_none());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn npy_reader_rejects_overflowing_shape_without_panicking() {
        // A header that is a valid v1.0 little-endian f8 C-order array but whose
        // shape product overflows `usize` must return None — never abort via
        // `Vec::with_capacity` (the documented "never panics / falls back" contract).
        let dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = dir.join(format!("pid-rerun-npy-huge-{stamp}.npy"));
        // 2^61 elements: count*8 wraps to 0 with naive arithmetic; checked_mul -> None.
        let header =
            "{'descr': '<f8', 'fortran_order': False, 'shape': (2305843009213693952,), }\n";
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x93NUMPY");
        bytes.push(1);
        bytes.push(0);
        bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(&[0u8; 64]); // a little real data, far less than claimed
        std::fs::write(&path, bytes).unwrap();
        assert!(read_npy_f64(path.to_str().unwrap()).is_none());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn npy_reader_rejects_structured_dtype_containing_f8_field() {
        // A STRUCTURED dtype descr that merely contains an "<f8" field is not a
        // flat f64 array; a substring match would accept it and decode garbage.
        let dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = dir.join(format!("pid-rerun-npy-struct-{stamp}.npy"));
        let header =
            "{'descr': [('a', '<f8'), ('b', '<i4')], 'fortran_order': False, 'shape': (2,), }\n";
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x93NUMPY");
        bytes.push(1);
        bytes.push(0);
        bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(&[0u8; 24]);
        std::fs::write(&path, bytes).unwrap();
        assert!(
            read_npy_f64(path.to_str().unwrap()).is_none(),
            "structured dtype must be rejected, not decoded as flat f64"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn logs_attribution_relevance_heatmap_from_npy() -> Result<()> {
        let dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let npy = dir.join(format!("pid-rerun-relevance-{stamp}.npy"));
        write_npy_f64(&npy, &[0.1, 0.9, -0.4, 0.2], &[2, 2]);

        let rec = RecordingStreamBuilder::new("runlog_relevance_test").buffered()?;
        let events = vec![
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "rel-run".to_string(),
                timestamp_ns: 0,
                config_hash: pid_runlog::canonical_json_hash(&json!({"k": 1})).unwrap(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::AttributionLogged {
                timestamp_ns: 1,
                method: "lrp_epsilon".to_string(),
                target_output: "action_dim_0".to_string(),
                layer: None,
                modality: Some("vision".to_string()),
                baseline: Some("zero".to_string()),
                score_hash: Some("abc".to_string()),
                faithfulness_check: Some(true),
                artifact_uri: Some(npy.to_str().unwrap().to_string()),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::RunEnded {
                run_id: "rel-run".to_string(),
                timestamp_ns: 2,
                status: RunStatus::Succeeded,
                message: None,
            },
        ];
        // The adapter loads the relevance .npy and logs it without error; a missing
        // artifact must also be handled gracefully (best-effort), so re-run with a
        // bogus uri and confirm it still succeeds.
        let summary = RunLogRerunLogger::new(&rec).log_events_with_manifest(&events, None)?;
        assert_eq!(summary.validation_errors, 0);
        assert_eq!(summary.attributions, 1);

        let mut bad = events.clone();
        if let RunLogEvent::AttributionLogged { artifact_uri, .. } = &mut bad[1] {
            *artifact_uri = Some("/no/such/relevance.npy".to_string());
        }
        let rec2 = RecordingStreamBuilder::new("runlog_relevance_missing").buffered()?;
        let summary2 = RunLogRerunLogger::new(&rec2).log_events_with_manifest(&bad, None)?;
        assert_eq!(summary2.validation_errors, 0);

        let _ = std::fs::remove_file(npy);
        Ok(())
    }

    #[test]
    fn logs_manifest_diagnostics() -> Result<()> {
        let rec = RecordingStreamBuilder::new("runlog_manifest_test").buffered()?;
        let events = sample_events();
        let summary = pid_runlog::summarize_events(&events)?;
        let manifest = RunManifest {
            schema_version: RUN_LOG_SCHEMA_VERSION,
            run_id: summary.run_id.clone(),
            config_hash: summary.config_hash.clone(),
            run_log_uri: "memory://rerun-run.jsonl".to_string(),
            run_log_sha256: Some("abc".to_string()),
            trace_hash: summary.trace_hash.clone(),
            logical_trace_hash: summary.logical_trace_hash.clone(),
            event_count: summary.event_count,
            validation_errors: summary.validation_errors,
            validation_warnings: summary.validation_warnings,
            artifacts: Vec::new(),
        };
        let logged =
            RunLogRerunLogger::new(&rec).log_events_with_manifest(&events, Some(&manifest))?;
        assert_eq!(logged.trace_hash, manifest.trace_hash);
        Ok(())
    }
}
