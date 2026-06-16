//! VLA data structures for loading and processing.
//!
//! Supports common VLA dataset formats:
//! - LIBERO (HDF5)
//! - Open X-Embodiment (RLDS/TFRecord)
//! - Simple JSON/numpy for testing

use anyhow::{bail, Result};
use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};

/// A single frame from a VLA episode.
#[derive(Debug, Clone)]
pub struct VlaFrame {
    /// Timestamp in seconds
    pub timestamp: f64,
    /// Vision embedding (e.g., DINO, SigLIP) - shape [embed_dim]
    pub vision_embedding: Array1<f64>,
    /// Language embedding - shape [embed_dim]
    pub language_embedding: Option<Array1<f64>>,
    /// Action output - shape [action_dim] (e.g., 7 for robot arm)
    pub action: Array1<f64>,
    /// Optional: RGB image as flattened bytes
    pub image: Option<Vec<u8>>,
    /// Optional: Image dimensions [height, width, channels]
    pub image_shape: Option<[usize; 3]>,
    /// Optional: 3D object positions - shape [n_objects, 3]
    pub object_positions: Option<Array2<f64>>,
    /// Optional: Robot proprioception - shape [proprio_dim]
    pub proprioception: Option<Array1<f64>>,
}

/// A complete VLA episode (trajectory).
#[derive(Debug, Clone)]
pub struct VlaEpisode {
    /// Episode identifier
    pub episode_id: String,
    /// Natural language instruction
    pub instruction: String,
    /// Sequence of frames
    pub frames: Vec<VlaFrame>,
    /// Success label (if available)
    pub success: Option<bool>,
    /// Failure timestamp (if failure occurred)
    pub failure_timestamp: Option<f64>,
    /// Metadata
    pub metadata: EpisodeMetadata,
}

/// Episode metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpisodeMetadata {
    pub robot_type: Option<String>,
    pub task_name: Option<String>,
    pub scene_name: Option<String>,
    pub embedding_model: Option<String>,
    pub control_frequency_hz: Option<f64>,
}

impl VlaEpisode {
    /// Create a new empty episode.
    pub fn new(episode_id: impl Into<String>, instruction: impl Into<String>) -> Self {
        Self {
            episode_id: episode_id.into(),
            instruction: instruction.into(),
            frames: Vec::new(),
            success: None,
            failure_timestamp: None,
            metadata: EpisodeMetadata::default(),
        }
    }

    /// Add a frame to the episode.
    pub fn push_frame(&mut self, frame: VlaFrame) {
        self.frames.push(frame);
    }

    pub fn validate_shapes(&self) -> Result<()> {
        if self.frames.is_empty() {
            return Ok(());
        }
        let vision_dim = self.frames[0].vision_embedding.len();
        let action_dim = self.frames[0].action.len();
        for (idx, frame) in self.frames.iter().enumerate() {
            if !frame.timestamp.is_finite() {
                bail!("frame {idx}: timestamp must be finite");
            }
            if frame.vision_embedding.len() != vision_dim {
                bail!("frame {idx}: inconsistent vision embedding dimension");
            }
            if frame.action.len() != action_dim {
                bail!("frame {idx}: inconsistent action dimension");
            }
            if let Some(language) = &frame.language_embedding {
                if language.is_empty() {
                    bail!("frame {idx}: language embedding must not be empty when present");
                }
            }
            if let Some(positions) = &frame.object_positions {
                if positions.ncols() != 3 {
                    bail!("frame {idx}: object positions must have shape [n_objects, 3]");
                }
            }
        }
        Ok(())
    }

    /// Get the duration of the episode in seconds.
    pub fn duration(&self) -> f64 {
        if self.frames.is_empty() {
            return 0.0;
        }
        let first = self.frames.first().unwrap().timestamp;
        let last = self.frames.last().unwrap().timestamp;
        last - first
    }

    /// Get vision embeddings as a 2D array [n_frames, embed_dim].
    pub fn vision_embeddings(&self) -> Array2<f64> {
        if self.frames.is_empty() {
            return Array2::zeros((0, 0));
        }
        let n = self.frames.len();
        let d = self.frames[0].vision_embedding.len();
        let mut arr = Array2::zeros((n, d));
        for (i, frame) in self.frames.iter().enumerate() {
            let width = d.min(frame.vision_embedding.len());
            for j in 0..width {
                arr[(i, j)] = frame.vision_embedding[j];
            }
        }
        arr
    }

    /// Get actions as a 2D array [n_frames, action_dim].
    pub fn actions(&self) -> Array2<f64> {
        if self.frames.is_empty() {
            return Array2::zeros((0, 0));
        }
        let n = self.frames.len();
        let d = self.frames[0].action.len();
        let mut arr = Array2::zeros((n, d));
        for (i, frame) in self.frames.iter().enumerate() {
            let width = d.min(frame.action.len());
            for j in 0..width {
                arr[(i, j)] = frame.action[j];
            }
        }
        arr
    }

    /// Get timestamps as a 1D array.
    pub fn timestamps(&self) -> Array1<f64> {
        Array1::from_vec(self.frames.iter().map(|f| f.timestamp).collect())
    }
}

/// Generate a synthetic VLA episode for testing.
/// This creates realistic-looking data without requiring actual VLA inference.
pub fn generate_synthetic_episode(
    n_frames: usize,
    vision_dim: usize,
    action_dim: usize,
    seed: u64,
) -> VlaEpisode {
    use std::f64::consts::PI;

    // Simple LCG for reproducibility. Take the top 32 bits (>>32) so the value maps
    // onto the `u32::MAX` divisor: ratio ∈ [0, 1], rescaled to a symmetric [-1, 1).
    // (A >>33 shift would keep only 31 bits, capping the ratio at 0.5 and yielding
    // exclusively negative samples.)
    let mut state = seed;
    let mut rand = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((state >> 32) as f64) / (u32::MAX as f64) * 2.0 - 1.0
    };

    let mut episode = VlaEpisode::new(
        "synthetic_001",
        "pick up the red cube and place it in the bowl",
    );
    episode.metadata.robot_type = Some("franka_panda".into());
    episode.metadata.task_name = Some("pick_and_place".into());
    episode.metadata.embedding_model = Some("dinov2_vitb14".into());
    episode.metadata.control_frequency_hz = Some(10.0);

    // Generate a smooth trajectory
    for i in 0..n_frames {
        let t = i as f64 / 10.0; // 10 Hz
        let phase = t * PI / 5.0; // Full cycle over ~10 seconds

        // Vision embedding: mostly random with some temporal structure
        let mut vision = Array1::zeros(vision_dim);
        for j in 0..vision_dim {
            // Mix of random noise + temporal signal in first few dimensions
            let signal = if j < 10 {
                0.3 * (phase + j as f64 * 0.1).sin()
            } else {
                0.0
            };
            vision[j] = signal + 0.7 * rand();
        }

        // Action: smooth trajectory (7-DoF robot arm)
        let mut action = Array1::zeros(action_dim);
        for j in 0..action_dim.min(7) {
            // Sinusoidal motion with different frequencies per joint
            action[j] = 0.1 * (phase * (1.0 + j as f64 * 0.2)).sin() + 0.02 * rand();
        }

        // Object positions: cube moving toward bowl
        let cube_x = 0.5 - 0.3 * (phase / PI).min(1.0);
        let cube_y = 0.0;
        let cube_z = 0.1 + 0.2 * (phase * 2.0).sin().max(0.0);
        let bowl_pos = [0.2, 0.3, 0.05];

        let object_positions = Array2::from_shape_vec(
            (2, 3),
            vec![
                cube_x,
                cube_y,
                cube_z,
                bowl_pos[0],
                bowl_pos[1],
                bowl_pos[2],
            ],
        )
        .unwrap();

        let frame = VlaFrame {
            timestamp: t,
            vision_embedding: vision,
            language_embedding: None,
            action,
            image: None,
            image_shape: None,
            object_positions: Some(object_positions),
            proprioception: None,
        };

        episode.push_frame(frame);
    }

    episode.success = Some(true);
    episode
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthetic_episode() {
        let episode = generate_synthetic_episode(100, 768, 7, 42);
        assert_eq!(episode.frames.len(), 100);
        assert_eq!(episode.frames[0].vision_embedding.len(), 768);
        assert_eq!(episode.frames[0].action.len(), 7);
        assert!(episode.duration() > 9.0 && episode.duration() < 11.0);
    }
}
