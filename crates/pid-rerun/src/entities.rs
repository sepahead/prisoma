//! Entity path definitions for Rerun logging.
//!
//! Following the PID-Splat architecture:
//! - `world/reality`: Captured scene splats
//! - `world/ghost`: Predicted flow as point cloud (colored by PID values)
//! - `pid/metrics/{name}`: PID atom time series
//! - `vla/embeddings`: Embedding geometry
//! - `vla/action`: VLA control outputs

/// Standard entity paths for PID-VLA visualization.
pub struct EntityPaths;

impl EntityPaths {
    // World entities
    pub const WORLD_REALITY: &'static str = "world/reality";
    pub const WORLD_GHOST: &'static str = "world/ghost";
    pub const WORLD_OBJECTS: &'static str = "world/objects";

    // PID metric entities
    pub const PID_REDUNDANCY: &'static str = "pid/metrics/redundancy";
    pub const PID_SYNERGY: &'static str = "pid/metrics/synergy";
    pub const PID_UNIQUE_V: &'static str = "pid/metrics/unique_v";
    pub const PID_UNIQUE_L: &'static str = "pid/metrics/unique_l";
    pub const PID_MI_TOTAL: &'static str = "pid/metrics/mi_total";

    // Geometry diagnostic entities
    pub const GEOMETRY_ID: &'static str = "pid/geometry/intrinsic_dim";
    pub const GEOMETRY_DCCV: &'static str = "pid/geometry/distance_concentration";
    pub const GEOMETRY_HYPERBOLICITY: &'static str = "pid/geometry/hyperbolicity";

    // VLA entities
    pub const VLA_VISION: &'static str = "vla/vision";
    pub const VLA_LANGUAGE: &'static str = "vla/language";
    pub const VLA_ACTION: &'static str = "vla/action";
    pub const VLA_EMBEDDINGS: &'static str = "vla/embeddings";

    // Flow entities
    pub const FLOW_PREDICTED: &'static str = "flow/predicted";
    pub const FLOW_ACTUAL: &'static str = "flow/actual";
    pub const FLOW_ERROR: &'static str = "flow/error";
}
