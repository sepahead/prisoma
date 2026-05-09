use anyhow::Result;
use pid_runlog::{RunLogEvent, RunLogSummary, RunManifest, SimObjectSnapshot};
use rerun::{Color, Points3D, RecordingStream, Scalars, TextLog};
use std::time::Duration;

pub struct RunLogRerunLogger<'a> {
    rec: &'a RecordingStream,
}

impl<'a> RunLogRerunLogger<'a> {
    pub fn new(rec: &'a RecordingStream) -> Self {
        Self { rec }
    }

    pub fn log_events(&self, events: &[RunLogEvent]) -> Result<()> {
        let summary = pid_runlog::summarize_events(events)?;
        self.log_summary(&summary)?;
        for event in events {
            self.log_event(event)?;
        }
        Ok(())
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
        self.rec.set_time(
            "time",
            Duration::from_secs_f64(event.timestamp_ns() as f64 / 1_000_000_000.0),
        );
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
            RunLogEvent::FrameObserved { .. } => Ok(()),
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
