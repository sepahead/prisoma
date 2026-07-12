//! Physics backend trait and adapters for rigid-body simulation.
//!
//! The existing `DeterministicObjectSim` uses constant-velocity Euler integration
//! with no collision handling. This module defines a `PhysicsBackend` trait that
//! abstracts over physics engines, with two implementations:
//! - [`NullPhysicsBackend`]: constant-velocity kinematics (no gravity/contacts),
//!   always available; used to test the trait contract and as a cross-backend
//!   robustness baseline.
//! - [`rapier_adapter::RapierBackend`] (behind the `rapier` feature): a real
//!   single-threaded Rapier3D-f64 pipeline with gravity, contacts, and friction.
//!
//! # Feature flag
//! Enable `rapier` to compile the real Rapier3D backend. Collision geometry is
//! **box-approximation** (cuboid colliders derived from `half_extents`); arbitrary
//! mesh colliders still require a mesh-ingestion pipeline (a later, optional milestone).

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
/// Implementations must be deterministic given the same inputs. The trait
/// carries no RNG seed; a stochastic backend must own a seeded RNG and
/// document it.
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

/// A minimal kinematic backend for testing the trait contract: no forces,
/// collisions, or gravity — it Euler-integrates each body's constant velocity.
pub struct NullPhysicsBackend {
    bodies: Vec<RigidBodyState>,
    step_count: u64,
    /// Monotonic simulated time, accumulated from each step's `dt` so the report
    /// timestamp stays correct even when `dt` varies between steps.
    elapsed_ns: u64,
}

impl NullPhysicsBackend {
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            step_count: 0,
            elapsed_ns: 0,
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
        self.elapsed_ns = self.elapsed_ns.saturating_add(dt_ns);
        Ok(PhysicsStepReport {
            step: self.step_count,
            timestamp_ns: self.elapsed_ns,
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
        self.elapsed_ns = 0;
    }
}

// ---------------------------------------------------------------------------
// Rapier3D adapter (behind `rapier` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "rapier")]
pub mod rapier_adapter {
    //! Real Rapier3D (f64) physics adapter.
    //!
    //! Wraps a single-threaded `rapier3d_f64` pipeline behind the
    //! [`PhysicsBackend`](super::PhysicsBackend) trait: dynamic cuboid rigid
    //! bodies, gravity, contacts, and friction are all real. Box geometry is
    //! derived from the `half_extents`/`mass_kg` passed to
    //! [`PhysicsBackend::add_rigid_body`] (collider density is back-solved from
    //! the requested mass), so the deterministic object harness can be re-run on
    //! genuine contact dynamics without a mesh-ingestion pipeline.
    //!
    //! # Determinism
    //!
    //! Stepping is single-threaded with a fixed substep `dt` and fixed solver
    //! iteration counts, so re-running an identical command sequence on the same
    //! binary/platform reproduces the trajectory bit-for-bit (exercised by
    //! `rapier_backend_is_deterministic`). Cross-platform bit-determinism is
    //! **not** claimed: that needs Rapier's `enhanced-determinism` feature, which
    //! is intentionally not enabled here.

    use super::*;
    use rapier3d_f64::na::{Isometry3, Quaternion, Translation3, UnitQuaternion, Vector3};
    use rapier3d_f64::prelude::*;
    use std::collections::BTreeMap;

    /// Real Rapier3D (f64) physics backend.
    pub struct RapierBackend {
        config: PhysicsWorldConfig,
        gravity: Vector3<f64>,
        integration_parameters: IntegrationParameters,
        physics_pipeline: PhysicsPipeline,
        islands: IslandManager,
        broad_phase: DefaultBroadPhase,
        narrow_phase: NarrowPhase,
        bodies: RigidBodySet,
        colliders: ColliderSet,
        impulse_joints: ImpulseJointSet,
        multibody_joints: MultibodyJointSet,
        ccd_solver: CCDSolver,
        query_pipeline: QueryPipeline,
        /// Dynamic body handles by object id (ground is tracked separately).
        handles: BTreeMap<String, RigidBodyHandle>,
        /// Insertion order, so snapshots are deterministic regardless of map order.
        order: Vec<String>,
        last_contact_count: usize,
        step_count: u64,
        /// Monotonic simulated time (sum of per-step `dt`), correct under variable dt.
        elapsed_ns: u64,
    }

    impl RapierBackend {
        pub fn new(config: PhysicsWorldConfig) -> Self {
            let gravity = Vector3::new(config.gravity[0], config.gravity[1], config.gravity[2]);
            let integration_parameters = IntegrationParameters {
                dt: config.fixed_dt_secs,
                ..Default::default()
            };
            Self {
                config,
                gravity,
                integration_parameters,
                physics_pipeline: PhysicsPipeline::new(),
                islands: IslandManager::new(),
                broad_phase: DefaultBroadPhase::new(),
                narrow_phase: NarrowPhase::new(),
                bodies: RigidBodySet::new(),
                colliders: ColliderSet::new(),
                impulse_joints: ImpulseJointSet::new(),
                multibody_joints: MultibodyJointSet::new(),
                ccd_solver: CCDSolver::new(),
                query_pipeline: QueryPipeline::new(),
                handles: BTreeMap::new(),
                order: Vec::new(),
                last_contact_count: 0,
                step_count: 0,
                elapsed_ns: 0,
            }
        }

        /// Number of contact pairs with at least one active contact after the
        /// most recent [`step`](PhysicsBackend::step).
        pub fn last_contact_count(&self) -> usize {
            self.last_contact_count
        }

        /// Add a static ground slab whose top face sits at `z = 0`.
        ///
        /// Not part of the [`PhysicsBackend`] trait: kinematic backends (e.g.
        /// [`NullPhysicsBackend`](super::NullPhysicsBackend)) have no contacts, so a
        /// ground plane is meaningful only for a real dynamics backend.
        pub fn add_ground_slab(&mut self, half_extent_xy: f64, thickness: f64, friction: f64) {
            let body = RigidBodyBuilder::fixed()
                .translation(Vector3::new(0.0, 0.0, -thickness))
                .build();
            let handle = self.bodies.insert(body);
            let collider = ColliderBuilder::cuboid(half_extent_xy, half_extent_xy, thickness)
                .friction(friction)
                .build();
            self.colliders
                .insert_with_parent(collider, handle, &mut self.bodies);
        }

        /// Set the contact friction of the cuboid collider attached to `object_id`.
        pub fn set_friction(&mut self, object_id: &str, friction: f64) -> Result<()> {
            let handle = *self
                .handles
                .get(object_id)
                .ok_or_else(|| anyhow::anyhow!("unknown object_id: {object_id}"))?;
            let body = self
                .bodies
                .get(handle)
                .ok_or_else(|| anyhow::anyhow!("missing body for {object_id}"))?;
            let colliders: Vec<ColliderHandle> = body.colliders().to_vec();
            for ch in colliders {
                if let Some(c) = self.colliders.get_mut(ch) {
                    c.set_friction(friction);
                }
            }
            Ok(())
        }

        fn quat_from_xyzw(q: [f64; 4]) -> UnitQuaternion<f64> {
            // nalgebra Quaternion::new(w, i, j, k); the input layout is [x, y, z, w].
            UnitQuaternion::from_quaternion(Quaternion::new(q[3], q[0], q[1], q[2]))
        }

        fn state_for(&self, object_id: &str, handle: RigidBodyHandle) -> Option<RigidBodyState> {
            let rb = self.bodies.get(handle)?;
            let t = rb.translation();
            let q = rb.rotation().quaternion();
            let lv = rb.linvel();
            let av = rb.angvel();
            Some(RigidBodyState {
                object_id: object_id.to_string(),
                position: [t.x, t.y, t.z],
                orientation_xyzw: [q.i, q.j, q.k, q.w],
                linear_velocity: [lv.x, lv.y, lv.z],
                angular_velocity: [av.x, av.y, av.z],
            })
        }
    }

    impl PhysicsBackend for RapierBackend {
        fn name(&self) -> &str {
            "rapier3d"
        }

        fn body_count(&self) -> usize {
            self.order.len()
        }

        fn add_rigid_body(
            &mut self,
            object_id: &str,
            position: [f64; 3],
            orientation_xyzw: [f64; 4],
            half_extents: [f64; 3],
            mass_kg: f64,
        ) -> Result<()> {
            if object_id.is_empty() {
                bail!("object_id must not be empty");
            }
            if self.handles.contains_key(object_id) {
                bail!("duplicate object_id: {object_id}");
            }
            if !half_extents.iter().all(|h| h.is_finite() && *h > 0.0) {
                bail!("half_extents must be finite and positive");
            }
            if !mass_kg.is_finite() || mass_kg <= 0.0 {
                bail!("mass_kg must be finite and positive");
            }
            if !position.iter().all(|p| p.is_finite())
                || !orientation_xyzw.iter().all(|q| q.is_finite())
            {
                bail!("position and orientation must be finite");
            }
            let iso = Isometry3::from_parts(
                Translation3::new(position[0], position[1], position[2]),
                Self::quat_from_xyzw(orientation_xyzw),
            );
            let body = RigidBodyBuilder::dynamic().position(iso).build();
            let handle = self.bodies.insert(body);
            // Back-solve density so the collider mass matches the requested mass.
            let volume = 8.0 * half_extents[0] * half_extents[1] * half_extents[2];
            let density = mass_kg / volume;
            let collider =
                ColliderBuilder::cuboid(half_extents[0], half_extents[1], half_extents[2])
                    .density(density)
                    .friction(0.5)
                    .restitution(0.0)
                    .build();
            self.colliders
                .insert_with_parent(collider, handle, &mut self.bodies);
            self.handles.insert(object_id.to_string(), handle);
            self.order.push(object_id.to_string());
            Ok(())
        }

        fn remove_body(&mut self, object_id: &str) -> Result<()> {
            let handle = self
                .handles
                .remove(object_id)
                .ok_or_else(|| anyhow::anyhow!("unknown object_id: {object_id}"))?;
            self.bodies.remove(
                handle,
                &mut self.islands,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                true,
            );
            self.order.retain(|id| id != object_id);
            Ok(())
        }

        fn apply_impulse(&mut self, object_id: &str, impulse: [f64; 3]) -> Result<()> {
            if !impulse.iter().all(|v| v.is_finite()) {
                bail!("impulse must be finite");
            }
            let handle = *self
                .handles
                .get(object_id)
                .ok_or_else(|| anyhow::anyhow!("unknown object_id: {object_id}"))?;
            let body = self
                .bodies
                .get_mut(handle)
                .ok_or_else(|| anyhow::anyhow!("missing body for {object_id}"))?;
            body.apply_impulse(Vector3::new(impulse[0], impulse[1], impulse[2]), true);
            Ok(())
        }

        fn set_linear_velocity(&mut self, object_id: &str, velocity: [f64; 3]) -> Result<()> {
            if !velocity.iter().all(|v| v.is_finite()) {
                bail!("velocity must be finite");
            }
            let handle = *self
                .handles
                .get(object_id)
                .ok_or_else(|| anyhow::anyhow!("unknown object_id: {object_id}"))?;
            let body = self
                .bodies
                .get_mut(handle)
                .ok_or_else(|| anyhow::anyhow!("missing body for {object_id}"))?;
            body.set_linvel(Vector3::new(velocity[0], velocity[1], velocity[2]), true);
            Ok(())
        }

        fn step(&mut self, dt_secs: f64) -> Result<PhysicsStepReport> {
            if !dt_secs.is_finite() || dt_secs <= 0.0 {
                bail!("dt_secs must be positive and finite");
            }
            // Substep toward the configured fixed dt for stable, deterministic
            // integration while honouring the trait's "advance by dt_secs" contract.
            let max_substeps = self.config.max_substeps.max(1);
            let n_sub =
                ((dt_secs / self.config.fixed_dt_secs).round() as usize).clamp(1, max_substeps);
            let sub_dt = dt_secs / n_sub as f64;
            self.integration_parameters.dt = sub_dt;
            let hooks = ();
            let events = ();
            for _ in 0..n_sub {
                self.physics_pipeline.step(
                    &self.gravity,
                    &self.integration_parameters,
                    &mut self.islands,
                    &mut self.broad_phase,
                    &mut self.narrow_phase,
                    &mut self.bodies,
                    &mut self.colliders,
                    &mut self.impulse_joints,
                    &mut self.multibody_joints,
                    &mut self.ccd_solver,
                    Some(&mut self.query_pipeline),
                    &hooks,
                    &events,
                );
            }
            self.step_count += 1;
            self.last_contact_count = self
                .narrow_phase
                .contact_pairs()
                .filter(|p| p.has_any_active_contact)
                .count();
            let dt_ns = (dt_secs * 1_000_000_000.0).round() as u64;
            self.elapsed_ns = self.elapsed_ns.saturating_add(dt_ns);
            Ok(PhysicsStepReport {
                step: self.step_count,
                timestamp_ns: self.elapsed_ns,
                substeps: n_sub,
                contact_count: self.last_contact_count,
                bodies: self.snapshot(),
            })
        }

        fn snapshot(&self) -> Vec<RigidBodyState> {
            self.order
                .iter()
                .filter_map(|id| {
                    let handle = *self.handles.get(id)?;
                    self.state_for(id, handle)
                })
                .collect()
        }

        fn reset(&mut self) {
            *self = Self::new(self.config.clone());
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

    #[cfg(feature = "rapier")]
    mod rapier {
        use super::super::rapier_adapter::RapierBackend;
        use super::super::{PhysicsBackend, PhysicsWorldConfig};

        /// 5 cm cube resting on the ground slab (top at z=0): center settles near
        /// z = half_extent, and the body never tunnels below the ground.
        fn cube_on_ground() -> RapierBackend {
            let mut b = RapierBackend::new(PhysicsWorldConfig::default());
            b.add_ground_slab(5.0, 0.1, 0.5);
            // Start slightly above resting height so it settles under gravity.
            b.add_rigid_body(
                "cube",
                [0.0, 0.0, 0.05],
                [0.0, 0.0, 0.0, 1.0],
                [0.025; 3],
                0.1,
            )
            .unwrap();
            b
        }

        #[test]
        fn rapier_cube_rests_on_ground_under_gravity() {
            let mut b = cube_on_ground();
            for _ in 0..200 {
                b.step(0.01).unwrap();
            }
            let snap = b.snapshot();
            assert_eq!(snap.len(), 1);
            let z = snap[0].position[2];
            // Rests with center near half-extent (0.025) above the ground top (z=0).
            assert!(z > 0.015 && z < 0.035, "cube settled at z={z}");
            // It is in contact with the ground.
            assert!(b.last_contact_count() >= 1, "no contact detected");
            // Vertical velocity has damped out (resting).
            assert!(snap[0].linear_velocity[2].abs() < 0.05);
        }

        #[test]
        fn rapier_impulse_pushes_cube_then_friction_stops_it() {
            let mut b = cube_on_ground();
            // Let it settle first.
            for _ in 0..50 {
                b.step(0.01).unwrap();
            }
            let x_before = b.snapshot()[0].position[0];
            // Horizontal push.
            b.apply_impulse("cube", [0.2, 0.0, 0.0]).unwrap();
            for _ in 0..300 {
                b.step(0.01).unwrap();
            }
            let snap = b.snapshot();
            let x_after = snap[0].position[0];
            // The cube moved forward...
            assert!(
                x_after - x_before > 0.02,
                "cube barely moved: {x_before}->{x_after}"
            );
            // ...and friction brought it (nearly) to rest.
            assert!(
                snap[0].linear_velocity[0].abs() < 0.05,
                "cube still sliding: vx={}",
                snap[0].linear_velocity[0]
            );
        }

        #[test]
        fn rapier_backend_is_deterministic() {
            let run = || {
                let mut b = cube_on_ground();
                for _ in 0..30 {
                    b.step(0.01).unwrap();
                }
                b.apply_impulse("cube", [0.15, 0.05, 0.0]).unwrap();
                for _ in 0..30 {
                    b.step(0.01).unwrap();
                }
                b.snapshot()
            };
            let a = run();
            let b = run();
            assert_eq!(a.len(), b.len());
            for (sa, sb) in a.iter().zip(b.iter()) {
                assert_eq!(sa.position, sb.position, "nondeterministic position");
                assert_eq!(sa.linear_velocity, sb.linear_velocity);
            }
        }

        #[test]
        fn rapier_rejects_bad_inputs() {
            let mut b = RapierBackend::new(PhysicsWorldConfig::default());
            b.add_rigid_body("a", [0.0; 3], [0.0, 0.0, 0.0, 1.0], [0.1; 3], 1.0)
                .unwrap();
            // Duplicate id.
            assert!(b
                .add_rigid_body("a", [0.0; 3], [0.0, 0.0, 0.0, 1.0], [0.1; 3], 1.0)
                .is_err());
            // Bad mass / extents.
            assert!(b
                .add_rigid_body("z", [0.0; 3], [0.0, 0.0, 0.0, 1.0], [0.1; 3], 0.0)
                .is_err());
            assert!(b
                .add_rigid_body("z", [0.0; 3], [0.0, 0.0, 0.0, 1.0], [0.0; 3], 1.0)
                .is_err());
            // Unknown id operations.
            assert!(b.apply_impulse("nope", [1.0, 0.0, 0.0]).is_err());
            assert!(b.set_linear_velocity("nope", [1.0, 0.0, 0.0]).is_err());
            assert!(b.remove_body("nope").is_err());
            // Bad dt.
            assert!(b.step(-0.1).is_err());
            assert!(b.step(f64::NAN).is_err());
        }

        #[test]
        fn rapier_remove_and_reset() {
            let mut b = cube_on_ground();
            assert_eq!(b.body_count(), 1);
            b.remove_body("cube").unwrap();
            assert_eq!(b.body_count(), 0);
            b.add_rigid_body("c2", [0.0, 0.0, 0.1], [0.0, 0.0, 0.0, 1.0], [0.025; 3], 0.1)
                .unwrap();
            assert_eq!(b.body_count(), 1);
            b.reset();
            assert_eq!(b.body_count(), 0);
        }
    }
}
