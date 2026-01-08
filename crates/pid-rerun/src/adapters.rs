//! Rerun logging adapters for PID metrics and VLA data.

use crate::data::{VlaEpisode, VlaFrame};
use crate::entities::EntityPaths;
use anyhow::Result;
use ndarray::Array2;
use rerun::{Arrows3D, Color, Points3D, RecordingStream, Scalars, TextLog};
use std::time::Duration;

/// Logger for PID metrics to Rerun.
pub struct PidLogger<'a> {
    rec: &'a RecordingStream,
}

impl<'a> PidLogger<'a> {
    pub fn new(rec: &'a RecordingStream) -> Self {
        Self { rec }
    }

    /// Set the time for subsequent logs.
    fn set_time(&self, timestamp_secs: f64) {
        self.rec
            .set_time("time", Duration::from_secs_f64(timestamp_secs));
    }

    /// Log PID atoms at a given timestamp.
    pub fn log_pid_atoms(
        &self,
        timestamp_secs: f64,
        redundancy: f64,
        synergy: f64,
        unique_v: f64,
        unique_l: f64,
    ) -> Result<()> {
        self.set_time(timestamp_secs);

        self.rec
            .log(EntityPaths::PID_REDUNDANCY, &Scalars::single(redundancy))?;
        self.rec
            .log(EntityPaths::PID_SYNERGY, &Scalars::single(synergy))?;
        self.rec
            .log(EntityPaths::PID_UNIQUE_V, &Scalars::single(unique_v))?;
        self.rec
            .log(EntityPaths::PID_UNIQUE_L, &Scalars::single(unique_l))?;

        // Total MI = Red + Unq_V + Unq_L + Syn
        let mi_total = redundancy + unique_v + unique_l + synergy;
        self.rec
            .log(EntityPaths::PID_MI_TOTAL, &Scalars::single(mi_total))?;

        Ok(())
    }

    /// Log geometry diagnostics.
    pub fn log_geometry(
        &self,
        timestamp_secs: f64,
        intrinsic_dim: f64,
        distance_concentration_cv: f64,
        hyperbolicity: Option<f64>,
    ) -> Result<()> {
        self.set_time(timestamp_secs);

        self.rec
            .log(EntityPaths::GEOMETRY_ID, &Scalars::single(intrinsic_dim))?;
        self.rec.log(
            EntityPaths::GEOMETRY_DCCV,
            &Scalars::single(distance_concentration_cv),
        )?;

        if let Some(delta) = hyperbolicity {
            self.rec
                .log(EntityPaths::GEOMETRY_HYPERBOLICITY, &Scalars::single(delta))?;
        }

        Ok(())
    }

    /// Log a text annotation (e.g., failure detected).
    pub fn log_event(&self, timestamp_secs: f64, level: &str, message: &str) -> Result<()> {
        self.set_time(timestamp_secs);
        self.rec
            .log("pid/events", &TextLog::new(message).with_level(level))?;
        Ok(())
    }
}

/// Logger for VLA data to Rerun.
pub struct VlaLogger<'a> {
    rec: &'a RecordingStream,
}

impl<'a> VlaLogger<'a> {
    pub fn new(rec: &'a RecordingStream) -> Self {
        Self { rec }
    }

    /// Set the time for subsequent logs.
    fn set_time(&self, timestamp_secs: f64) {
        self.rec
            .set_time("time", Duration::from_secs_f64(timestamp_secs));
    }

    /// Log a complete VLA episode.
    pub fn log_episode(&self, episode: &VlaEpisode) -> Result<()> {
        // Log episode metadata
        self.rec.log_static(
            "episode/instruction",
            &TextLog::new(episode.instruction.as_str()),
        )?;

        if let Some(ref robot) = episode.metadata.robot_type {
            self.rec
                .log_static("episode/robot", &TextLog::new(robot.as_str()))?;
        }

        // Log each frame
        for frame in &episode.frames {
            self.log_frame(frame)?;
        }

        // Log success/failure annotation
        if let Some(success) = episode.success {
            let level = if success { "INFO" } else { "WARN" };
            let msg = if success {
                "Episode succeeded"
            } else {
                "Episode failed"
            };
            let t = episode.failure_timestamp.unwrap_or(episode.duration());
            self.set_time(t);
            self.rec
                .log("episode/outcome", &TextLog::new(msg).with_level(level))?;
        }

        Ok(())
    }

    /// Log a single VLA frame.
    pub fn log_frame(&self, frame: &VlaFrame) -> Result<()> {
        self.set_time(frame.timestamp);

        // Log action as a scalar series (one per dimension)
        for (i, &val) in frame.action.iter().enumerate() {
            self.rec.log(
                format!("{}/joint_{}", EntityPaths::VLA_ACTION, i),
                &Scalars::single(val),
            )?;
        }

        // Log object positions as 3D points
        if let Some(ref positions) = frame.object_positions {
            let points: Vec<[f32; 3]> = positions
                .rows()
                .into_iter()
                .map(|row| [row[0] as f32, row[1] as f32, row[2] as f32])
                .collect();

            // Color by object index
            let colors: Vec<Color> = (0..points.len())
                .map(|i| match i {
                    0 => Color::from_rgb(255, 0, 0),     // Red cube
                    1 => Color::from_rgb(0, 0, 255),     // Blue bowl
                    _ => Color::from_rgb(128, 128, 128), // Gray
                })
                .collect();

            self.rec.log(
                EntityPaths::WORLD_OBJECTS,
                &Points3D::new(points)
                    .with_colors(colors)
                    .with_radii([0.02_f32]),
            )?;
        }

        Ok(())
    }

    /// Log 3D flow predictions vs actuals.
    pub fn log_flow(
        &self,
        timestamp_secs: f64,
        predicted: &Array2<f64>, // [n_points, 3]
        actual: &Array2<f64>,    // [n_points, 3]
    ) -> Result<()> {
        self.set_time(timestamp_secs);

        // Predicted flow as blue points
        let pred_points: Vec<[f32; 3]> = predicted
            .rows()
            .into_iter()
            .map(|row| [row[0] as f32, row[1] as f32, row[2] as f32])
            .collect();

        self.rec.log(
            EntityPaths::FLOW_PREDICTED,
            &Points3D::new(pred_points.clone())
                .with_colors([Color::from_rgb(100, 100, 255)])
                .with_radii([0.01_f32]),
        )?;

        // Actual flow as green points
        let actual_points: Vec<[f32; 3]> = actual
            .rows()
            .into_iter()
            .map(|row| [row[0] as f32, row[1] as f32, row[2] as f32])
            .collect();

        self.rec.log(
            EntityPaths::FLOW_ACTUAL,
            &Points3D::new(actual_points)
                .with_colors([Color::from_rgb(100, 255, 100)])
                .with_radii([0.01_f32]),
        )?;

        // Flow error as arrows from predicted to actual
        if predicted.nrows() == actual.nrows() && predicted.nrows() > 0 {
            let origins: Vec<[f32; 3]> = pred_points.clone();
            let vectors: Vec<[f32; 3]> = predicted
                .rows()
                .into_iter()
                .zip(actual.rows())
                .map(|(p, a)| {
                    [
                        (a[0] - p[0]) as f32,
                        (a[1] - p[1]) as f32,
                        (a[2] - p[2]) as f32,
                    ]
                })
                .collect();

            self.rec.log(
                EntityPaths::FLOW_ERROR,
                &Arrows3D::from_vectors(vectors)
                    .with_origins(origins)
                    .with_colors([Color::from_rgb(255, 165, 0)]), // Orange
            )?;
        }

        Ok(())
    }

    /// Log ghost splat overlay (predicted object positions colored by PID).
    pub fn log_ghost_splat(
        &self,
        timestamp_secs: f64,
        positions: &Array2<f64>, // [n_points, 3]
        pid_values: &[f64],      // Per-point PID value (e.g., synergy)
    ) -> Result<()> {
        self.set_time(timestamp_secs);

        let points: Vec<[f32; 3]> = positions
            .rows()
            .into_iter()
            .map(|row| [row[0] as f32, row[1] as f32, row[2] as f32])
            .collect();

        // Color by PID value: blue (low) -> red (high)
        let colors: Vec<Color> = pid_values
            .iter()
            .map(|&v| {
                let t = (v.clamp(-1.0, 1.0) + 1.0) / 2.0; // Normalize to [0, 1]
                let r = (t * 255.0) as u8;
                let b = ((1.0 - t) * 255.0) as u8;
                Color::from_rgb(r, 50, b)
            })
            .collect();

        self.rec.log(
            EntityPaths::WORLD_GHOST,
            &Points3D::new(points)
                .with_colors(colors)
                .with_radii([0.015_f32]),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::generate_synthetic_episode;
    use rerun::RecordingStreamBuilder;

    #[test]
    fn test_pid_logger() -> Result<()> {
        let rec = RecordingStreamBuilder::new("test_pid").buffered()?;
        let logger = PidLogger::new(&rec);
        logger.log_pid_atoms(0.0, 0.3, 0.1, 0.2, 0.15)?;
        logger.log_geometry(0.0, 15.0, 0.25, Some(0.1))?;
        Ok(())
    }

    #[test]
    fn test_vla_logger() -> Result<()> {
        let rec = RecordingStreamBuilder::new("test_vla").buffered()?;
        let logger = VlaLogger::new(&rec);
        let episode = generate_synthetic_episode(10, 768, 7, 42);
        logger.log_episode(&episode)?;
        Ok(())
    }
}
