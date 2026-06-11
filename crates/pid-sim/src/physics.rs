//! Physics backend trait and adapter skeletons for rigid-body simulation.
//!
//! The existing `DeterministicObjectSim` uses constant-velocity Euler integration
//! with no collision handling. This module defines a `PhysicsBackend` trait that
//! abstracts over physics engines so that a Rapier3D adapter (or other backends)
//! can be swapped in behind a feature flag.
//!
//! # Feature flag
//! Enable `rapier` to compile the Rapier3D adapter stub. The stub initialises a
//! Rapier `PhysicsPipeline` but does **not** yet wire collision geometry from
//! mesh assets — that requires the mesh ingestion pipeline (milestone M5+).

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// A snapshot of a single rigid body from the physics engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RigidBodyState {
    pub object_id: String,
    pub position: [f64; 3],
    pub orientation_xyzw: [f64; 4],
    pub linear_velocity: [f64; 3],
    pub angular_velocity: [f64; 3],
}

/// Configuration for the physics world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicsWorldConfig {
    pub gravity: [f64; 3],
    pub fixed_dt_secs: f64,
    pub max_substeps: usize,
}

impl Default for PhysicsWorldConfig {
    fn default() -> Self {
        Self {
            gravity: [0.0, 0.0, -9.81],
            fixed_dt_secs: 1.0 / 240.0,
            max_substeps: 4,
        }
    }
}

/// Result of a single physics step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicsStepReport {
    pub step: u64,
    pub timestamp_ns: u64,
    pub substeps: usize,
    pub contact_count: usize,
    pub bodies: Vec<RigidBodyState>,
}

/// Trait abstracting over rigid-body physics engines.
///
/// Implementations must be deterministic given the same seed and inputs.
pub trait PhysicsBackend {
    /// Human-readable backend name (e.g. "rapier3d", "deterministic_object").
    fn name(&self) -> &str;

    /// Number of rigid bodies currently in the world.
    fn body_count(&self) -> usize;

    /// Insert a rigid body (box approximation) into the world.
    fn add_rigid_body(
        &mut self,
        object_id: &str,
        position: [f64; 3],
        orientation_xyzw: [f64; 4],
        half_extents: [f64; 3],
        mass_kg: f64,
    ) -> Result<()>;

    /// Remove a body by id.
    fn remove_body(&mut self, object_id: &str) -> Result<()>;

    /// Apply an impulse (linear) to a body.
    fn apply_impulse(&mut self, object_id: &str, impulse: [f64; 3]) -> Result<()>;

    /// Set the linear velocity of a body directly.
    fn set_linear_velocity(&mut self, object_id: &str, velocity: [f64; 3]) -> Result<()>;

    /// Advance the simulation by `dt_secs`, returning a step report.
    fn step(&mut self, dt_secs: f64) -> Result<PhysicsStepReport>;

    /// Read back all body states without advancing.
    fn snapshot(&self) -> Vec<RigidBodyState>;

    /// Reset the world to empty.
    fn reset(&mut self);
}

// ---------------------------------------------------------------------------
// Null backend (always available, used for testing the trait contract)
// ---------------------------------------------------------------------------

/// A no-op physics backend for testing the trait contract.
pub struct NullPhysicsBackend {
    bodies: Vec<RigidBodyState>,
    step_count: u64,
}

impl NullPhysicsBackend {
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            step_count: 0,
        }
    }
}

impl Default for NullPhysicsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsBackend for NullPhysicsBackend {
    fn name(&self) -> &str {
        "null"
    }

    fn body_count(&self) -> usize {
        self.bodies.len()
    }

    fn add_rigid_body(
        &mut self,
        object_id: &str,
        position: [f64; 3],
        orientation_xyzw: [f64; 4],
        _half_extents: [f64; 3],
        _mass_kg: f64,
    ) -> Result<()> {
        if object_id.is_empty() {
            bail!("object_id must not be empty");
        }
        if self.bodies.iter().any(|b| b.object_id == object_id) {
            bail!("duplicate object_id: {object_id}");
        }
        self.bodies.push(RigidBodyState {
            object_id: object_id.to_string(),
            position,
            orientation_xyzw,
            linear_velocity: [0.0; 3],
            angular_velocity: [0.0; 3],
        });
        Ok(())
    }

    fn remove_body(&mut self, object_id: &str) -> Result<()> {
        let before = self.bodies.len();
        self.bodies.retain(|b| b.object_id != object_id);
        if self.bodies.len() == before {
            bail!("unknown object_id: {object_id}");
        }
        Ok(())
    }

    fn apply_impulse(&mut self, object_id: &str, impulse: [f64; 3]) -> Result<()> {
        let body = self
            .bodies
            .iter_mut()
            .find(|b| b.object_id == object_id)
            .ok_or_else(|| anyhow::anyhow!("unknown object_id: {object_id}"))?;
        // Null backend: treat mass=1 so impulse = delta_v
        for (v, dv) in body.linear_velocity.iter_mut().zip(impulse) {
            *v += dv;
        }
        Ok(())
    }

    fn set_linear_velocity(&mut self, object_id: &str, velocity: [f64; 3]) -> Result<()> {
        let body = self
            .bodies
            .iter_mut()
            .find(|b| b.object_id == object_id)
            .ok_or_else(|| anyhow::anyhow!("unknown object_id: {object_id}"))?;
        body.linear_velocity = velocity;
        Ok(())
    }

    fn step(&mut self, dt_secs: f64) -> Result<PhysicsStepReport> {
        if !dt_secs.is_finite() || dt_secs <= 0.0 {
            bail!("dt_secs must be positive and finite");
        }
        // Constant-velocity Euler (no gravity, no collisions — same as DeterministicObjectSim)
        for body in &mut self.bodies {
            for (p, v) in body.position.iter_mut().zip(body.linear_velocity) {
                *p += v * dt_secs;
            }
        }
        self.step_count += 1;
        let dt_ns = (dt_secs * 1_000_000_000.0).round() as u64;
        Ok(PhysicsStepReport {
            step: self.step_count,
            timestamp_ns: self.step_count * dt_ns,
            substeps: 1,
            contact_count: 0,
            bodies: self.bodies.clone(),
        })
    }

    fn snapshot(&self) -> Vec<RigidBodyState> {
        self.bodies.clone()
    }

    fn reset(&mut self) {
        self.bodies.clear();
        self.step_count = 0;
    }
}

// ---------------------------------------------------------------------------
// Rapier3D adapter stub (behind `rapier` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "rapier")]
pub mod rapier_adapter {
    //! Rapier3D physics adapter stub.
    //!
    //! This requires `rapier3d` as an optional dependency in `Cargo.toml`:
    //! ```toml
    //! [features]
    //! rapier = ["dep:rapier3d"]
    //!
    //! [dependencies]
    //! rapier3d = { version = "0.22", optional = true, features = ["simd-stable"] }
    //! ```
    //!
    //! The adapter is a skeleton: it compiles but does not yet wire mesh
    //! collision geometry or joint constraints. Those require the mesh
    //! ingestion pipeline (milestone M5+).

    use super::*;

    /// Rapier3D physics backend.
    ///
    /// Wraps `rapier3d::pipeline::PhysicsPipeline` and related data structures.
    pub struct RapierBackend {
        _config: PhysicsWorldConfig,
        // NOTE: Actual Rapier fields would go here:
        // pipeline: rapier3d::pipeline::PhysicsPipeline,
        // gravity: rapier3d::math::Vector<f64>,
        // integration_params: rapier3d::dynamics::IntegrationParameters,
        // rigid_body_set: rapier3d::dynamics::RigidBodySet,
        // collider_set: rapier3d::geometry::ColliderSet,
        // ...
        bodies: Vec<RigidBodyState>,
        step_count: u64,
    }

    impl RapierBackend {
        pub fn new(config: PhysicsWorldConfig) -> Self {
            Self {
                _config: config,
                bodies: Vec::new(),
                step_count: 0,
            }
        }
    }

    impl PhysicsBackend for RapierBackend {
        fn name(&self) -> &str {
            "rapier3d"
        }

        fn body_count(&self) -> usize {
            self.bodies.len()
        }

        fn add_rigid_body(
            &mut self,
            object_id: &str,
            position: [f64; 3],
            orientation_xyzw: [f64; 4],
            _half_extents: [f64; 3],
            _mass_kg: f64,
        ) -> Result<()> {
            // TODO: create rapier3d RigidBody + Cuboid Collider
            self.bodies.push(RigidBodyState {
                object_id: object_id.to_string(),
                position,
                orientation_xyzw,
                linear_velocity: [0.0; 3],
                angular_velocity: [0.0; 3],
            });
            Ok(())
        }

        fn remove_body(&mut self, object_id: &str) -> Result<()> {
            let before = self.bodies.len();
            self.bodies.retain(|b| b.object_id != object_id);
            if self.bodies.len() == before {
                bail!("unknown object_id: {object_id}");
            }
            Ok(())
        }

        fn apply_impulse(&mut self, _object_id: &str, _impulse: [f64; 3]) -> Result<()> {
            // TODO: rapier3d rigid_body_set[obj_handle].apply_impulse(...)
            bail!("Rapier apply_impulse not yet wired")
        }

        fn set_linear_velocity(&mut self, _object_id: &str, _velocity: [f64; 3]) -> Result<()> {
            // TODO: rapier3d rigid_body_set[obj_handle].set_linvel(...)
            bail!("Rapier set_linear_velocity not yet wired")
        }

        fn step(&mut self, dt_secs: f64) -> Result<PhysicsStepReport> {
            if !dt_secs.is_finite() || dt_secs <= 0.0 {
                bail!("dt_secs must be positive and finite");
            }
            // TODO: self.pipeline.step(&self.gravity, &integration_params, ...)
            self.step_count += 1;
            let dt_ns = (dt_secs * 1_000_000_000.0).round() as u64;
            Ok(PhysicsStepReport {
                step: self.step_count,
                timestamp_ns: self.step_count * dt_ns,
                substeps: 0,
                contact_count: 0,
                bodies: self.bodies.clone(),
            })
        }

        fn snapshot(&self) -> Vec<RigidBodyState> {
            self.bodies.clone()
        }

        fn reset(&mut self) {
            self.bodies.clear();
            self.step_count = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_backend_add_step_snapshot() {
        let mut backend = NullPhysicsBackend::new();
        backend
            .add_rigid_body(
                "cube",
                [0.0, 0.0, 0.1],
                [0.0, 0.0, 0.0, 1.0],
                [0.025; 3],
                1.0,
            )
            .unwrap();
        assert_eq!(backend.body_count(), 1);

        backend
            .set_linear_velocity("cube", [1.0, 0.0, 0.0])
            .unwrap();
        let report = backend.step(0.1).unwrap();
        assert_eq!(report.step, 1);
        assert_eq!(report.contact_count, 0);
        assert!((report.bodies[0].position[0] - 0.1).abs() < 1e-12);

        let snap = backend.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].object_id, "cube");
    }

    #[test]
    fn null_backend_rejects_duplicate_id() {
        let mut backend = NullPhysicsBackend::new();
        backend
            .add_rigid_body("a", [0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3], 1.0)
            .unwrap();
        assert!(backend
            .add_rigid_body("a", [0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3], 1.0)
            .is_err());
    }

    #[test]
    fn null_backend_impulse_alters_velocity() {
        let mut backend = NullPhysicsBackend::new();
        backend
            .add_rigid_body("ball", [0.0; 3], [0.0, 0.0, 0.0, 1.0], [0.05; 3], 1.0)
            .unwrap();
        backend.apply_impulse("ball", [2.0, 0.0, 0.0]).unwrap();
        let snap = backend.snapshot();
        assert!((snap[0].linear_velocity[0] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn null_backend_reset_clears() {
        let mut backend = NullPhysicsBackend::new();
        backend
            .add_rigid_body("x", [0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3], 1.0)
            .unwrap();
        backend.reset();
        assert_eq!(backend.body_count(), 0);
    }

    #[test]
    fn null_backend_rejects_bad_dt() {
        let mut backend = NullPhysicsBackend::new();
        assert!(backend.step(-0.1).is_err());
        assert!(backend.step(f64::NAN).is_err());
    }

    #[test]
    fn null_backend_remove_unknown_errors() {
        let mut backend = NullPhysicsBackend::new();
        assert!(backend.remove_body("nope").is_err());
    }

    #[test]
    fn null_backend_deterministic() {
        let make = || {
            let mut b = NullPhysicsBackend::new();
            b.add_rigid_body("c", [1.0, 2.0, 3.0], [0.0, 0.0, 0.0, 1.0], [0.1; 3], 1.0)
                .unwrap();
            b.set_linear_velocity("c", [0.5, -0.3, 0.1]).unwrap();
            b.step(0.016).unwrap();
            b.snapshot()
        };
        assert_eq!(make(), make());
    }
}
