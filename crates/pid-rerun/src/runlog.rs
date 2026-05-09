use anyhow::Result;
use pid_runlog::{RunLogEvent, SimObjectSnapshot};
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
        for event in events {
            self.log_event(event)?;
        }
        Ok(())
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
