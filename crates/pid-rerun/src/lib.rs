//! `pid-rerun`: Rerun visualization adapters for prisoma diagnostics.
//!
//! This crate provides logging adapters to visualize:
//! - PID metrics (redundancy, synergy, unique information) over time
//! - VLA embeddings and their geometry
//! - 3D object flow trajectories
//! - Ghost splat overlays for predicted vs actual states
//!
//! # Architecture (Rerun-First, Phases 1-3)
//! Uses Rerun SDK as the primary visualization backend. Entities are logged to:
//! - `world/reality`: Captured scene data
//! - `world/ghost`: Predicted flow as secondary point cloud
//! - `pid/metrics`: PID atom time series
//! - `vla/embeddings`: Embedding geometry diagnostics

pub mod adapters;
pub mod data;
pub mod entities;
pub mod runlog;

pub use adapters::{PidLogger, VlaLogger};
pub use data::{VlaEpisode, VlaFrame};
pub use entities::EntityPaths;
pub use runlog::RunLogRerunLogger;

use anyhow::Result;
use rerun::{RecordingStream, RecordingStreamBuilder};

// Re-export rerun for convenience
pub use rerun;

/// Initialize a Rerun recording stream for prisoma visualization.
pub fn init_recording(app_id: &str, spawn_viewer: bool) -> Result<RecordingStream> {
    let builder = RecordingStreamBuilder::new(app_id);

    let rec = if spawn_viewer {
        builder.spawn()?
    } else {
        builder.buffered()?
    };

    Ok(rec)
}

/// Save recording to an RRD file.
pub fn save_recording(rec: &RecordingStream, path: &str) -> Result<()> {
    rec.save(path)?;
    Ok(())
}
