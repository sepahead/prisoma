//! Rerun logging adapters for PID metrics and VLA data.

use crate::data::{VlaEpisode, VlaFrame};
use crate::entities::EntityPaths;
use anyhow::{bail, ensure, Context, Result};
use ndarray::Array2;
use rerun::RecordingStream;
use rerun_types::{
    archetypes::{Arrows3D, Points3D, Scalars, TextLog},
    components::Color,
};
use std::time::Duration;

const PID_UNIQUE_SOURCE_1: &str = "pid/metrics/unique_source_1";
const PID_UNIQUE_SOURCE_2: &str = "pid/metrics/unique_source_2";
const PID_SOURCE_1_LABEL: &str = "pid/provenance/source_1_label";
const PID_SOURCE_2_LABEL: &str = "pid/provenance/source_2_label";
const MAX_EPISODE_ID_BYTES: usize = 1024;
const MAX_INSTRUCTION_BYTES: usize = 16 * 1024;
const MAX_METADATA_TEXT_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy)]
struct ValidatedPidAtoms {
    redundancy: f64,
    synergy: f64,
    unique_source_1: f64,
    unique_source_2: f64,
    mi_total: f64,
}

struct PreparedFrame<'a> {
    frame: &'a VlaFrame,
    timestamp: Duration,
    object_points: Option<Vec<[f32; 3]>>,
}

struct PreparedEpisode<'a> {
    frames: Vec<PreparedFrame<'a>>,
    outcome: Option<(Duration, &'static str, &'static str)>,
}

fn validated_timestamp(timestamp_secs: f64, context: &str) -> Result<Duration> {
    ensure!(
        timestamp_secs.is_finite() && timestamp_secs >= 0.0,
        "{context} must be finite and non-negative"
    );
    let timestamp = Duration::try_from_secs_f64(timestamp_secs)
        .with_context(|| format!("{context} is outside the representable timeline range"))?;
    ensure!(
        timestamp.as_nanos() <= i64::MAX as u128,
        "{context} exceeds Rerun's signed-nanosecond timeline range"
    );
    Ok(timestamp)
}

fn validate_source_label(label: &str, context: &str) -> Result<()> {
    ensure!(!label.trim().is_empty(), "{context} must not be empty");
    ensure!(
        label.len() <= 1024,
        "{context} must be at most 1024 UTF-8 bytes"
    );
    ensure!(
        !label.chars().any(char::is_control),
        "{context} must not contain control characters"
    );
    Ok(())
}

fn validate_text(value: &str, context: &str, max_bytes: usize) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{context} must not be empty");
    ensure!(
        value.len() <= max_bytes,
        "{context} must be at most {max_bytes} UTF-8 bytes"
    );
    ensure!(
        !value.chars().any(char::is_control),
        "{context} must not contain control characters"
    );
    Ok(())
}

fn validate_pid_atoms(
    redundancy: f64,
    synergy: f64,
    unique_source_1: f64,
    unique_source_2: f64,
) -> Result<ValidatedPidAtoms> {
    for (name, value) in [
        ("redundancy", redundancy),
        ("synergy", synergy),
        ("unique_source_1", unique_source_1),
        ("unique_source_2", unique_source_2),
    ] {
        ensure!(value.is_finite(), "PID atom {name} must be finite");
    }
    let mi_total = redundancy + unique_source_1 + unique_source_2 + synergy;
    ensure!(
        mi_total.is_finite(),
        "PID atom sum overflows the finite MI range"
    );
    Ok(ValidatedPidAtoms {
        redundancy,
        synergy,
        unique_source_1,
        unique_source_2,
        mi_total,
    })
}

fn checked_f32(value: f64, context: &str) -> Result<f32> {
    ensure!(value.is_finite(), "{context} must be finite");
    let converted = value as f32;
    ensure!(
        converted.is_finite(),
        "{context} is outside the finite f32 range"
    );
    ensure!(
        value == 0.0 || converted != 0.0,
        "{context} underflows the nonzero f32 range"
    );
    Ok(converted)
}

fn prepare_points3(values: &Array2<f64>, context: &str) -> Result<Vec<[f32; 3]>> {
    ensure!(
        values.ncols() == 3,
        "{context} must have shape [n, 3], found [{}, {}]",
        values.nrows(),
        values.ncols()
    );
    values
        .rows()
        .into_iter()
        .enumerate()
        .map(|(row_index, row)| {
            Ok([
                checked_f32(row[0], &format!("{context}[{row_index},0]"))?,
                checked_f32(row[1], &format!("{context}[{row_index},1]"))?,
                checked_f32(row[2], &format!("{context}[{row_index},2]"))?,
            ])
        })
        .collect()
}

fn validate_finite_vector<'a>(
    values: impl IntoIterator<Item = &'a f64>,
    context: &str,
) -> Result<()> {
    for (index, value) in values.into_iter().enumerate() {
        ensure!(value.is_finite(), "{context}[{index}] must be finite");
    }
    Ok(())
}

fn validate_image(frame: &VlaFrame) -> Result<()> {
    match (&frame.image, frame.image_shape) {
        (None, None) => Ok(()),
        (Some(bytes), Some(shape)) => {
            ensure!(
                shape.into_iter().all(|extent| extent > 0),
                "image dimensions must all be positive"
            );
            let expected = shape
                .into_iter()
                .try_fold(1usize, usize::checked_mul)
                .context("image shape element count overflows usize")?;
            ensure!(
                bytes.len() == expected,
                "image byte count {} does not match declared shape element count {expected}",
                bytes.len()
            );
            Ok(())
        }
        _ => bail!("image and image_shape must either both be present or both be absent"),
    }
}

fn prepare_frame(frame: &VlaFrame) -> Result<PreparedFrame<'_>> {
    let timestamp = validated_timestamp(frame.timestamp, "frame timestamp")?;
    ensure!(
        !frame.vision_embedding.is_empty(),
        "vision_embedding must not be empty"
    );
    ensure!(!frame.action.is_empty(), "action must not be empty");
    validate_finite_vector(frame.vision_embedding.iter(), "vision_embedding")?;
    validate_finite_vector(frame.action.iter(), "action")?;
    if let Some(language) = &frame.language_embedding {
        ensure!(
            !language.is_empty(),
            "language_embedding must not be empty when present"
        );
        validate_finite_vector(language.iter(), "language_embedding")?;
    }
    if let Some(proprioception) = &frame.proprioception {
        validate_finite_vector(proprioception.iter(), "proprioception")?;
    }
    validate_image(frame)?;
    let object_points = frame
        .object_positions
        .as_ref()
        .map(|positions| prepare_points3(positions, "object_positions"))
        .transpose()?;
    Ok(PreparedFrame {
        frame,
        timestamp,
        object_points,
    })
}

fn prepare_episode(episode: &VlaEpisode) -> Result<PreparedEpisode<'_>> {
    validate_text(&episode.episode_id, "episode_id", MAX_EPISODE_ID_BYTES)?;
    validate_text(&episode.instruction, "instruction", MAX_INSTRUCTION_BYTES)?;
    for (name, value) in [
        ("robot_type", episode.metadata.robot_type.as_deref()),
        ("task_name", episode.metadata.task_name.as_deref()),
        ("scene_name", episode.metadata.scene_name.as_deref()),
        (
            "embedding_model",
            episode.metadata.embedding_model.as_deref(),
        ),
    ] {
        if let Some(value) = value {
            validate_text(value, name, MAX_METADATA_TEXT_BYTES)?;
        }
    }
    episode
        .validate_shapes()
        .context("episode shape validation failed")?;
    if let Some(frequency) = episode.metadata.control_frequency_hz {
        ensure!(
            frequency.is_finite() && frequency > 0.0,
            "control_frequency_hz must be finite and positive"
        );
    }
    if episode.failure_timestamp.is_some() && episode.success != Some(false) {
        bail!("failure_timestamp is valid only when success is explicitly false");
    }
    let failure_timestamp = episode
        .failure_timestamp
        .map(|timestamp| validated_timestamp(timestamp, "failure_timestamp"))
        .transpose()?;
    let frames = episode
        .frames
        .iter()
        .map(prepare_frame)
        .collect::<Result<Vec<_>>>()?;
    for (index, pair) in frames.windows(2).enumerate() {
        ensure!(
            pair[0].timestamp < pair[1].timestamp,
            "frame timestamps must be strictly increasing; frames {index} and {} are duplicate or out of order",
            index + 1
        );
    }
    if let Some(first_frame) = frames.first() {
        let expected_language_dim = first_frame
            .frame
            .language_embedding
            .as_ref()
            .map(|embedding| embedding.len());
        for (index, frame) in frames.iter().enumerate().skip(1) {
            let language_dim = frame
                .frame
                .language_embedding
                .as_ref()
                .map(|embedding| embedding.len());
            ensure!(
                language_dim == expected_language_dim,
                "frame {index}: language_embedding presence or dimension differs from frame 0"
            );
        }
    }
    if let Some(failure) = failure_timestamp {
        let Some((first, last)) = frames.first().zip(frames.last()) else {
            bail!("failure_timestamp requires a nonempty episode timeline");
        };
        ensure!(
            failure >= first.timestamp && failure <= last.timestamp,
            "failure_timestamp must lie within the episode timeline"
        );
    }
    let outcome = episode.success.map(|success| {
        let timestamp = failure_timestamp
            .or_else(|| frames.last().map(|frame| frame.timestamp))
            .unwrap_or(Duration::ZERO);
        if success {
            (timestamp, "INFO", "Episode succeeded")
        } else {
            (timestamp, "WARN", "Episode failed")
        }
    });
    Ok(PreparedEpisode { frames, outcome })
}

/// Logger for PID metrics to Rerun.
pub struct PidLogger<'a> {
    rec: &'a RecordingStream,
}

impl<'a> PidLogger<'a> {
    pub fn new(rec: &'a RecordingStream) -> Self {
        Self { rec }
    }

    /// Set the already-validated time for subsequent logs.
    fn set_time(&self, timestamp: Duration) {
        self.rec.set_time("time", timestamp);
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
        let timestamp = validated_timestamp(timestamp_secs, "PID timestamp")?;
        let atoms = validate_pid_atoms(redundancy, synergy, unique_v, unique_l)?;
        self.set_time(timestamp);

        self.rec.log(
            EntityPaths::PID_REDUNDANCY,
            &Scalars::single(atoms.redundancy),
        )?;
        self.rec
            .log(EntityPaths::PID_SYNERGY, &Scalars::single(atoms.synergy))?;
        self.rec.log(
            EntityPaths::PID_UNIQUE_V,
            &Scalars::single(atoms.unique_source_1),
        )?;
        self.rec.log(
            EntityPaths::PID_UNIQUE_L,
            &Scalars::single(atoms.unique_source_2),
        )?;
        self.rec
            .log(EntityPaths::PID_MI_TOTAL, &Scalars::single(atoms.mi_total))?;

        Ok(())
    }

    /// Log PID atoms for two caller-labeled sources without assigning modality semantics.
    ///
    /// The unique terms always use the static `unique_source_1` and `unique_source_2`
    /// entities. Source labels are logged separately at fixed provenance entities.
    pub fn log_source_labeled_pid_atoms(
        &self,
        timestamp_secs: f64,
        redundancy: f64,
        synergy: f64,
        unique_sources: [f64; 2],
        source_labels: [&str; 2],
    ) -> Result<()> {
        let timestamp = validated_timestamp(timestamp_secs, "PID timestamp")?;
        let atoms = validate_pid_atoms(redundancy, synergy, unique_sources[0], unique_sources[1])?;
        let [source_1_label, source_2_label] = source_labels;
        validate_source_label(source_1_label, "source_1_label")?;
        validate_source_label(source_2_label, "source_2_label")?;
        ensure!(
            source_1_label != source_2_label,
            "source labels must distinguish source 1 from source 2"
        );
        self.set_time(timestamp);

        self.rec.log(
            PID_SOURCE_1_LABEL,
            &TextLog::new(source_1_label).with_level("INFO"),
        )?;
        self.rec.log(
            PID_SOURCE_2_LABEL,
            &TextLog::new(source_2_label).with_level("INFO"),
        )?;
        self.rec.log(
            EntityPaths::PID_REDUNDANCY,
            &Scalars::single(atoms.redundancy),
        )?;
        self.rec
            .log(EntityPaths::PID_SYNERGY, &Scalars::single(atoms.synergy))?;
        self.rec
            .log(PID_UNIQUE_SOURCE_1, &Scalars::single(atoms.unique_source_1))?;
        self.rec
            .log(PID_UNIQUE_SOURCE_2, &Scalars::single(atoms.unique_source_2))?;
        self.rec
            .log(EntityPaths::PID_MI_TOTAL, &Scalars::single(atoms.mi_total))?;

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
        let timestamp = validated_timestamp(timestamp_secs, "geometry timestamp")?;
        ensure!(intrinsic_dim.is_finite(), "intrinsic_dim must be finite");
        ensure!(intrinsic_dim >= 0.0, "intrinsic_dim must be non-negative");
        ensure!(
            distance_concentration_cv.is_finite(),
            "distance_concentration_cv must be finite"
        );
        ensure!(
            distance_concentration_cv >= 0.0,
            "distance_concentration_cv must be non-negative"
        );
        if let Some(delta) = hyperbolicity {
            ensure!(delta.is_finite(), "hyperbolicity must be finite");
            ensure!(delta >= 0.0, "hyperbolicity must be non-negative");
        }
        self.set_time(timestamp);

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
        let timestamp = validated_timestamp(timestamp_secs, "event timestamp")?;
        self.set_time(timestamp);
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

    /// Set the already-validated time for subsequent logs.
    fn set_time(&self, timestamp: Duration) {
        self.rec.set_time("time", timestamp);
    }

    /// Log a complete VLA episode.
    pub fn log_episode(&self, episode: &VlaEpisode) -> Result<()> {
        let prepared = prepare_episode(episode)?;

        self.rec.log_static(
            "episode/instruction",
            &TextLog::new(episode.instruction.as_str()),
        )?;

        if let Some(ref robot) = episode.metadata.robot_type {
            self.rec
                .log_static("episode/robot", &TextLog::new(robot.as_str()))?;
        }

        for frame in &prepared.frames {
            self.log_prepared_frame(frame)?;
        }

        if let Some((timestamp, level, message)) = prepared.outcome {
            self.set_time(timestamp);
            self.rec
                .log("episode/outcome", &TextLog::new(message).with_level(level))?;
        }

        Ok(())
    }

    /// Log a single VLA frame.
    pub fn log_frame(&self, frame: &VlaFrame) -> Result<()> {
        let prepared = prepare_frame(frame)?;
        self.log_prepared_frame(&prepared)
    }

    fn log_prepared_frame(&self, prepared: &PreparedFrame<'_>) -> Result<()> {
        self.set_time(prepared.timestamp);

        for (index, value) in prepared.frame.action.iter().copied().enumerate() {
            self.rec.log(
                format!("{}/joint_{}", EntityPaths::VLA_ACTION, index),
                &Scalars::single(value),
            )?;
        }

        if let Some(points) = &prepared.object_points {
            let colors = (0..points.len())
                .map(|index| match index {
                    0 => Color::from_rgb(255, 0, 0),     // Red cube
                    1 => Color::from_rgb(0, 0, 255),     // Blue bowl
                    _ => Color::from_rgb(128, 128, 128), // Gray
                })
                .collect::<Vec<_>>();

            self.rec.log(
                EntityPaths::WORLD_OBJECTS,
                &Points3D::new(points.iter().copied())
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
        let timestamp = validated_timestamp(timestamp_secs, "flow timestamp")?;
        ensure!(
            predicted.nrows() > 0 && actual.nrows() > 0,
            "predicted and actual flow point sets must not be empty"
        );
        ensure!(
            predicted.nrows() == actual.nrows(),
            "predicted and actual flow must have equal point counts, found {} and {}",
            predicted.nrows(),
            actual.nrows()
        );
        let predicted_points = prepare_points3(predicted, "predicted")?;
        let actual_points = prepare_points3(actual, "actual")?;
        let vectors = (0..predicted.nrows())
            .map(|row| {
                Ok([
                    checked_f32(
                        actual[(row, 0)] - predicted[(row, 0)],
                        &format!("flow_vector[{row},0]"),
                    )?,
                    checked_f32(
                        actual[(row, 1)] - predicted[(row, 1)],
                        &format!("flow_vector[{row},1]"),
                    )?,
                    checked_f32(
                        actual[(row, 2)] - predicted[(row, 2)],
                        &format!("flow_vector[{row},2]"),
                    )?,
                ])
            })
            .collect::<Result<Vec<_>>>()?;
        self.set_time(timestamp);

        self.rec.log(
            EntityPaths::FLOW_PREDICTED,
            &Points3D::new(predicted_points.iter().copied())
                .with_colors([Color::from_rgb(100, 100, 255)])
                .with_radii([0.01_f32]),
        )?;

        self.rec.log(
            EntityPaths::FLOW_ACTUAL,
            &Points3D::new(actual_points.iter().copied())
                .with_colors([Color::from_rgb(100, 255, 100)])
                .with_radii([0.01_f32]),
        )?;

        if !vectors.is_empty() {
            self.rec.log(
                EntityPaths::FLOW_ERROR,
                &Arrows3D::from_vectors(vectors)
                    .with_origins(predicted_points)
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
        let timestamp = validated_timestamp(timestamp_secs, "ghost-splat timestamp")?;
        ensure!(
            positions.nrows() > 0,
            "ghost-splat positions must not be empty"
        );
        ensure!(
            positions.nrows() == pid_values.len(),
            "positions and pid_values must have equal counts, found {} and {}",
            positions.nrows(),
            pid_values.len()
        );
        let points = prepare_points3(positions, "positions")?;
        let colors = pid_values
            .iter()
            .copied()
            .enumerate()
            .map(|(index, value)| {
                checked_f32(value, &format!("pid_values[{index}]"))?;
                let t = (value.clamp(-1.0, 1.0) + 1.0) / 2.0;
                let r = (t * 255.0) as u8;
                let b = ((1.0 - t) * 255.0) as u8;
                Ok(Color::from_rgb(r, 50, b))
            })
            .collect::<Result<Vec<_>>>()?;
        self.set_time(timestamp);

        self.rec.log(
            EntityPaths::WORLD_GHOST,
            &Points3D::new(points.iter().copied())
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
    use ndarray::{array, Array1};
    use rerun::log::{Chunk, LogMsg};
    use rerun::sink::MemorySinkStorage;
    use rerun::{RecordingStream, RecordingStreamBuilder};

    fn rejected_without_writes(
        name: &str,
        call: impl FnOnce(&RecordingStream) -> Result<()>,
    ) -> Result<()> {
        let (recording, storage) = RecordingStreamBuilder::new(name).memory()?;
        recording.flush_blocking()?;
        let before = storage.num_msgs();
        let result = call(&recording);
        recording.flush_blocking()?;
        let after = storage.num_msgs();
        let _ = storage.take();
        assert_eq!((result.is_err(), after), (true, before));
        Ok(())
    }

    fn take_entity_paths(
        recording: &RecordingStream,
        storage: &MemorySinkStorage,
    ) -> Result<Vec<String>> {
        recording.flush_blocking()?;
        let mut paths = Vec::new();
        for message in storage.take() {
            if let LogMsg::ArrowMsg(_, arrow) = message {
                let path = Chunk::from_arrow_msg(&arrow)?.entity_path().to_string();
                paths.push(path.trim_start_matches('/').to_owned());
            }
        }
        Ok(paths)
    }

    fn base_frame() -> VlaFrame {
        generate_synthetic_episode(1, 3, 2, 7).frames.remove(0)
    }

    #[test]
    fn timestamp_validation_rejects_negative_nonfinite_and_unrepresentable_values() {
        let rejected = [
            -1.0,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NAN,
            f64::MAX,
            10_000_000_000.0,
        ]
        .into_iter()
        .all(|value| validated_timestamp(value, "test timestamp").is_err());

        assert!(rejected);
        assert!(validated_timestamp(9_000_000_000.0, "test timestamp").is_ok());
    }

    #[test]
    fn pid_atom_validation_rejects_every_nonfinite_coordinate_without_writing() -> Result<()> {
        for (index, values) in [
            [f64::NAN, 0.1, 0.2, 0.3],
            [0.1, f64::INFINITY, 0.2, 0.3],
            [0.1, 0.2, f64::NEG_INFINITY, 0.3],
            [0.1, 0.2, 0.3, f64::NAN],
        ]
        .into_iter()
        .enumerate()
        {
            rejected_without_writes(&format!("pid_nonfinite_{index}"), |recording| {
                PidLogger::new(recording)
                    .log_pid_atoms(0.0, values[0], values[1], values[2], values[3])
            })?;
        }
        Ok(())
    }

    #[test]
    fn pid_mi_overflow_is_rejected_before_any_entity_write() -> Result<()> {
        rejected_without_writes("pid_overflow", |recording| {
            PidLogger::new(recording).log_pid_atoms(0.0, f64::MAX, f64::MAX, f64::MAX, f64::MAX)
        })
    }

    #[test]
    fn pid_and_geometry_timestamps_are_rejected_without_writing() -> Result<()> {
        rejected_without_writes("pid_timestamp", |recording| {
            PidLogger::new(recording).log_pid_atoms(-1.0, 0.1, 0.2, 0.3, 0.4)
        })?;
        rejected_without_writes("pid_labeled_timestamp", |recording| {
            PidLogger::new(recording).log_source_labeled_pid_atoms(
                f64::NAN,
                0.1,
                0.2,
                [0.3, 0.4],
                ["source 1", "source 2"],
            )
        })?;
        rejected_without_writes("geometry_timestamp", |recording| {
            PidLogger::new(recording).log_geometry(f64::INFINITY, 15.0, 0.25, Some(0.1))
        })
    }

    #[test]
    fn source_labels_are_validated_before_any_entity_write() -> Result<()> {
        rejected_without_writes("pid_label", |recording| {
            PidLogger::new(recording).log_source_labeled_pid_atoms(
                0.0,
                0.1,
                0.2,
                [0.3, 0.4],
                ["vision", "\n"],
            )
        })
    }

    #[test]
    fn source_labeled_pid_uses_static_generic_entities_and_provenance_paths() -> Result<()> {
        let (recording, storage) = RecordingStreamBuilder::new("pid_labeled").memory()?;
        PidLogger::new(&recording).log_source_labeled_pid_atoms(
            0.0,
            0.1,
            0.2,
            [0.3, 0.4],
            ["vision_dims_0_1", "vision_dims_2_3"],
        )?;
        let paths = take_entity_paths(&recording, &storage)?;
        let observed = (
            paths.iter().any(|path| path == PID_UNIQUE_SOURCE_1),
            paths.iter().any(|path| path == PID_UNIQUE_SOURCE_2),
            paths.iter().any(|path| path == PID_SOURCE_1_LABEL),
            paths.iter().any(|path| path == PID_SOURCE_2_LABEL),
            paths.iter().any(|path| path == EntityPaths::PID_UNIQUE_V),
            paths.iter().any(|path| path == EntityPaths::PID_UNIQUE_L),
        );

        assert_eq!(observed, (true, true, true, true, false, false));
        Ok(())
    }

    #[test]
    fn finite_pid_and_geometry_payloads_are_accepted() -> Result<()> {
        let (recording, storage) = RecordingStreamBuilder::new("pid_positive").memory()?;
        let logger = PidLogger::new(&recording);
        logger.log_pid_atoms(0.0, 0.3, 0.1, 0.2, 0.15)?;
        logger.log_geometry(0.0, 15.0, 0.25, Some(0.1))?;
        let paths = take_entity_paths(&recording, &storage)?;

        assert!(!paths.is_empty());
        Ok(())
    }

    #[test]
    fn geometry_validates_optional_value_before_any_entity_write() -> Result<()> {
        rejected_without_writes("geometry_invalid", |recording| {
            PidLogger::new(recording).log_geometry(0.0, 15.0, 0.25, Some(f64::NAN))
        })
    }

    #[test]
    fn event_rejects_invalid_timestamp_without_writing() -> Result<()> {
        rejected_without_writes("event_timestamp", |recording| {
            PidLogger::new(recording).log_event(-1.0, "WARN", "invalid timestamp")
        })
    }

    #[test]
    fn frame_rejects_nonfinite_action_before_any_entity_write() -> Result<()> {
        let mut frame = base_frame();
        frame.action[0] = f64::NAN;

        rejected_without_writes("frame_action", |recording| {
            VlaLogger::new(recording).log_frame(&frame)
        })
    }

    #[test]
    fn frame_rejects_malformed_positions_before_any_entity_write() -> Result<()> {
        let mut frame = base_frame();
        frame.object_positions = Some(Array2::zeros((2, 2)));

        rejected_without_writes("frame_shape", |recording| {
            VlaLogger::new(recording).log_frame(&frame)
        })
    }

    #[test]
    fn frame_rejects_non_f32_representable_positions_before_any_entity_write() -> Result<()> {
        let mut frame = base_frame();
        frame.object_positions.as_mut().unwrap()[(0, 0)] = f64::MAX;

        rejected_without_writes("frame_position_range", |recording| {
            VlaLogger::new(recording).log_frame(&frame)
        })
    }

    #[test]
    fn frame_preflight_rejects_nonfinite_unlogged_arrays_and_bad_image_contracts() {
        let mut vision = base_frame();
        vision.vision_embedding[0] = f64::NAN;
        let mut language = base_frame();
        language.language_embedding = Some(array![f64::INFINITY]);
        let mut proprioception = base_frame();
        proprioception.proprioception = Some(array![f64::NEG_INFINITY]);
        let mut image_pair = base_frame();
        image_pair.image = Some(vec![0; 3]);
        let mut image_size = base_frame();
        image_size.image = Some(vec![0; 3]);
        image_size.image_shape = Some([1, 1, 4]);

        let rejected = [vision, language, proprioception, image_pair, image_size]
            .iter()
            .all(|frame| prepare_frame(frame).is_err());

        assert!(rejected);
    }

    #[test]
    fn episode_preflights_late_frames_before_static_metadata_is_written() -> Result<()> {
        let mut episode = generate_synthetic_episode(3, 3, 2, 7);
        episode.frames[2].action[0] = f64::NAN;

        rejected_without_writes("episode_atomic", |recording| {
            VlaLogger::new(recording).log_episode(&episode)
        })
    }

    #[test]
    fn episode_rejects_empty_vision_and_action_dimensions_without_writing() -> Result<()> {
        let empty_vision = generate_synthetic_episode(2, 0, 2, 7);
        let empty_action = generate_synthetic_episode(2, 3, 0, 7);

        rejected_without_writes("episode_empty_vision", |recording| {
            VlaLogger::new(recording).log_episode(&empty_vision)
        })?;
        rejected_without_writes("episode_empty_action", |recording| {
            VlaLogger::new(recording).log_episode(&empty_action)
        })
    }

    #[test]
    fn episode_rejects_duplicate_and_out_of_order_timestamps_without_writing() -> Result<()> {
        let mut duplicate = generate_synthetic_episode(3, 3, 2, 7);
        duplicate.frames[1].timestamp = duplicate.frames[0].timestamp;
        let mut out_of_order = generate_synthetic_episode(3, 3, 2, 7);
        out_of_order.frames[2].timestamp = out_of_order.frames[0].timestamp + 0.05;
        let mut same_timeline_tick = generate_synthetic_episode(2, 3, 2, 7);
        same_timeline_tick.frames[1].timestamp = f64::MIN_POSITIVE;

        rejected_without_writes("episode_duplicate_time", |recording| {
            VlaLogger::new(recording).log_episode(&duplicate)
        })?;
        rejected_without_writes("episode_reverse_time", |recording| {
            VlaLogger::new(recording).log_episode(&out_of_order)
        })?;
        rejected_without_writes("episode_same_timeline_tick", |recording| {
            VlaLogger::new(recording).log_episode(&same_timeline_tick)
        })
    }

    #[test]
    fn episode_rejects_inconsistent_language_presence_and_dimension_without_writing() -> Result<()>
    {
        let mut presence = generate_synthetic_episode(2, 3, 2, 7);
        presence.frames[1].language_embedding = Some(array![0.1, 0.2]);
        let mut dimension = generate_synthetic_episode(2, 3, 2, 7);
        dimension.frames[0].language_embedding = Some(array![0.1, 0.2]);
        dimension.frames[1].language_embedding = Some(array![0.1]);

        rejected_without_writes("episode_language_presence", |recording| {
            VlaLogger::new(recording).log_episode(&presence)
        })?;
        rejected_without_writes("episode_language_dimension", |recording| {
            VlaLogger::new(recording).log_episode(&dimension)
        })
    }

    #[test]
    fn episode_rejects_failure_timestamp_outside_timeline_without_writing() -> Result<()> {
        let mut before = generate_synthetic_episode(2, 3, 2, 7);
        for frame in &mut before.frames {
            frame.timestamp += 1.0;
        }
        before.success = Some(false);
        before.failure_timestamp = Some(0.5);
        let mut after = generate_synthetic_episode(2, 3, 2, 7);
        after.success = Some(false);
        after.failure_timestamp = Some(0.2);

        rejected_without_writes("episode_failure_before", |recording| {
            VlaLogger::new(recording).log_episode(&before)
        })?;
        rejected_without_writes("episode_failure_after", |recording| {
            VlaLogger::new(recording).log_episode(&after)
        })
    }

    #[test]
    fn episode_rejects_invalid_static_text_before_writing() -> Result<()> {
        let mut episode_id = generate_synthetic_episode(2, 3, 2, 7);
        episode_id.episode_id = "invalid\nepisode".to_owned();
        let mut instruction = generate_synthetic_episode(2, 3, 2, 7);
        instruction.instruction = "x".repeat(MAX_INSTRUCTION_BYTES + 1);
        let mut robot = generate_synthetic_episode(2, 3, 2, 7);
        robot.metadata.robot_type = Some("invalid\trobot".to_owned());
        let mut metadata = generate_synthetic_episode(2, 3, 2, 7);
        metadata.metadata.task_name = Some("x".repeat(MAX_METADATA_TEXT_BYTES + 1));

        rejected_without_writes("episode_id_text", |recording| {
            VlaLogger::new(recording).log_episode(&episode_id)
        })?;
        rejected_without_writes("episode_instruction_text", |recording| {
            VlaLogger::new(recording).log_episode(&instruction)
        })?;
        rejected_without_writes("episode_robot_text", |recording| {
            VlaLogger::new(recording).log_episode(&robot)
        })?;
        rejected_without_writes("episode_metadata_text", |recording| {
            VlaLogger::new(recording).log_episode(&metadata)
        })
    }

    #[test]
    fn episode_rejects_invalid_metadata_and_outcome_contracts() {
        let mut frequency = generate_synthetic_episode(1, 3, 2, 7);
        frequency.metadata.control_frequency_hz = Some(f64::NAN);
        let mut outcome = generate_synthetic_episode(1, 3, 2, 7);
        outcome.failure_timestamp = Some(0.5);

        assert!(prepare_episode(&frequency).is_err() && prepare_episode(&outcome).is_err());
    }

    #[test]
    fn finite_frame_and_episode_payloads_are_accepted() -> Result<()> {
        let (recording, storage) = RecordingStreamBuilder::new("vla_positive").memory()?;
        let logger = VlaLogger::new(&recording);
        logger.log_frame(&base_frame())?;
        logger.log_episode(&generate_synthetic_episode(3, 3, 2, 42))?;
        let paths = take_entity_paths(&recording, &storage)?;

        assert!(!paths.is_empty());
        Ok(())
    }

    #[test]
    fn flow_rejects_malformed_or_mismatched_shapes_without_writing() -> Result<()> {
        let malformed = Array2::zeros((1, 2));
        let one_point = Array2::zeros((1, 3));
        let two_points = Array2::zeros((2, 3));
        rejected_without_writes("flow_shape", |recording| {
            VlaLogger::new(recording).log_flow(0.0, &malformed, &one_point)
        })?;
        rejected_without_writes("flow_count", |recording| {
            VlaLogger::new(recording).log_flow(0.0, &one_point, &two_points)
        })
    }

    #[test]
    fn flow_rejects_empty_point_sets_without_writing() -> Result<()> {
        let empty = Array2::zeros((0, 3));

        rejected_without_writes("flow_empty", |recording| {
            VlaLogger::new(recording).log_flow(0.0, &empty, &empty)
        })
    }

    #[test]
    fn flow_rejects_nonfinite_and_out_of_range_points_without_writing() -> Result<()> {
        let mut nonfinite = Array2::zeros((1, 3));
        nonfinite[(0, 1)] = f64::NAN;
        let mut out_of_range = Array2::zeros((1, 3));
        out_of_range[(0, 2)] = f64::MAX;
        let valid = Array2::zeros((1, 3));
        rejected_without_writes("flow_nonfinite", |recording| {
            VlaLogger::new(recording).log_flow(0.0, &nonfinite, &valid)
        })?;
        rejected_without_writes("flow_range", |recording| {
            VlaLogger::new(recording).log_flow(0.0, &valid, &out_of_range)
        })
    }

    #[test]
    fn flow_rejects_derived_vector_overflow_without_writing() -> Result<()> {
        let limit = f32::MAX as f64;
        let predicted = array![[-limit, 0.0, 0.0]];
        let actual = array![[limit, 0.0, 0.0]];

        rejected_without_writes("flow_vector_range", |recording| {
            VlaLogger::new(recording).log_flow(0.0, &predicted, &actual)
        })
    }

    #[test]
    fn finite_flow_is_accepted() -> Result<()> {
        let predicted = array![[0.0, 1.0, 2.0], [3.0, 4.0, 5.0]];
        let actual = array![[0.5, 1.5, 2.5], [3.5, 4.5, 5.5]];
        let (recording, storage) = RecordingStreamBuilder::new("flow_positive").memory()?;
        VlaLogger::new(&recording).log_flow(0.0, &predicted, &actual)?;
        let paths = take_entity_paths(&recording, &storage)?;

        assert!(!paths.is_empty());
        Ok(())
    }

    #[test]
    fn ghost_splat_rejects_shape_and_count_mismatches_without_writing() -> Result<()> {
        let malformed = Array2::zeros((1, 2));
        let points = Array2::zeros((2, 3));
        rejected_without_writes("ghost_shape", |recording| {
            VlaLogger::new(recording).log_ghost_splat(0.0, &malformed, &[0.0])
        })?;
        rejected_without_writes("ghost_count", |recording| {
            VlaLogger::new(recording).log_ghost_splat(0.0, &points, &[0.0])
        })
    }

    #[test]
    fn ghost_splat_rejects_empty_point_set_without_writing() -> Result<()> {
        let empty = Array2::zeros((0, 3));

        rejected_without_writes("ghost_empty", |recording| {
            VlaLogger::new(recording).log_ghost_splat(0.0, &empty, &[])
        })
    }

    #[test]
    fn ghost_splat_rejects_nonfinite_or_out_of_range_inputs_without_writing() -> Result<()> {
        let mut nonfinite_position = Array2::zeros((1, 3));
        nonfinite_position[(0, 0)] = f64::INFINITY;
        let valid = Array2::zeros((1, 3));
        rejected_without_writes("ghost_position", |recording| {
            VlaLogger::new(recording).log_ghost_splat(0.0, &nonfinite_position, &[0.0])
        })?;
        rejected_without_writes("ghost_pid_nonfinite", |recording| {
            VlaLogger::new(recording).log_ghost_splat(0.0, &valid, &[f64::NAN])
        })?;
        rejected_without_writes("ghost_pid_range", |recording| {
            VlaLogger::new(recording).log_ghost_splat(0.0, &valid, &[f64::MAX])
        })
    }

    #[test]
    fn finite_ghost_splat_is_accepted() -> Result<()> {
        let positions = array![[0.0, 1.0, 2.0], [3.0, 4.0, 5.0]];
        let (recording, storage) = RecordingStreamBuilder::new("ghost_positive").memory()?;
        VlaLogger::new(&recording).log_ghost_splat(0.0, &positions, &[-0.5, 0.5])?;
        let paths = take_entity_paths(&recording, &storage)?;

        assert!(!paths.is_empty());
        Ok(())
    }

    #[test]
    fn checked_f32_rejects_overflow_and_nonzero_underflow() {
        let rejected = checked_f32(f64::MAX, "overflow").is_err()
            && checked_f32(f64::MIN_POSITIVE, "underflow").is_err();

        assert!(rejected);
    }

    #[test]
    fn image_validation_accepts_exact_positive_shape() {
        let mut frame = base_frame();
        frame.image = Some(vec![0; 6]);
        frame.image_shape = Some([1, 2, 3]);

        assert!(prepare_frame(&frame).is_ok());
    }

    #[test]
    fn direct_frame_timestamp_is_rejected_without_writing() -> Result<()> {
        let mut frame = base_frame();
        frame.timestamp = f64::INFINITY;

        rejected_without_writes("frame_timestamp", |recording| {
            VlaLogger::new(recording).log_frame(&frame)
        })
    }

    #[test]
    fn flow_and_ghost_timestamps_are_rejected_without_writing() -> Result<()> {
        let points = Array2::zeros((1, 3));
        rejected_without_writes("flow_timestamp", |recording| {
            VlaLogger::new(recording).log_flow(f64::NAN, &points, &points)
        })?;
        rejected_without_writes("ghost_timestamp", |recording| {
            VlaLogger::new(recording).log_ghost_splat(-1.0, &points, &[0.0])
        })
    }

    #[test]
    fn source_labels_must_be_distinct() -> Result<()> {
        rejected_without_writes("pid_duplicate_labels", |recording| {
            PidLogger::new(recording).log_source_labeled_pid_atoms(
                0.0,
                0.1,
                0.2,
                [0.3, 0.4],
                ["vision", "vision"],
            )
        })
    }

    #[test]
    fn geometry_rejects_each_nonfinite_numeric_field() -> Result<()> {
        for (index, (intrinsic_dim, distance_cv, hyperbolicity)) in [
            (f64::NAN, 0.25, Some(0.1)),
            (15.0, f64::INFINITY, Some(0.1)),
            (15.0, 0.25, Some(f64::NEG_INFINITY)),
        ]
        .into_iter()
        .enumerate()
        {
            rejected_without_writes(&format!("geometry_nonfinite_{index}"), |recording| {
                PidLogger::new(recording).log_geometry(
                    0.0,
                    intrinsic_dim,
                    distance_cv,
                    hyperbolicity,
                )
            })?;
        }
        Ok(())
    }

    #[test]
    fn geometry_rejects_each_negative_domain_value_without_writing() -> Result<()> {
        for (index, (intrinsic_dim, distance_cv, hyperbolicity)) in [
            (-0.1, 0.25, Some(0.1)),
            (15.0, -0.1, Some(0.1)),
            (15.0, 0.25, Some(-0.1)),
        ]
        .into_iter()
        .enumerate()
        {
            rejected_without_writes(&format!("geometry_domain_{index}"), |recording| {
                PidLogger::new(recording).log_geometry(
                    0.0,
                    intrinsic_dim,
                    distance_cv,
                    hyperbolicity,
                )
            })?;
        }
        Ok(())
    }

    #[test]
    fn frame_rejects_nonfinite_object_position_without_writing() -> Result<()> {
        let mut frame = base_frame();
        frame.object_positions.as_mut().unwrap()[(0, 2)] = f64::NAN;

        rejected_without_writes("frame_position_nonfinite", |recording| {
            VlaLogger::new(recording).log_frame(&frame)
        })
    }

    #[test]
    fn finite_zero_is_f32_representable() {
        assert_eq!(checked_f32(0.0, "zero").unwrap(), 0.0);
    }

    #[test]
    fn episode_failure_timestamp_is_accepted_for_failed_episode() {
        let mut episode = generate_synthetic_episode(2, 3, 2, 7);
        episode.success = Some(false);
        episode.failure_timestamp = Some(0.05);

        assert!(prepare_episode(&episode).is_ok());
    }

    #[test]
    fn frame_rejects_empty_language_embedding() {
        let mut frame = base_frame();
        frame.language_embedding = Some(Array1::zeros(0));

        assert!(prepare_frame(&frame).is_err());
    }
}
