//! Scripted push-to-goal manipulation task over a [`PhysicsBackend`].
//!
//! This is the labeled-task half of milestone M3: a deterministic "push the cube
//! toward a goal x-position" episode that records canonical run-log events
//! (actions, snapshots, poses, real `Flow_gt`, a success label) plus full
//! backend/solver/task provenance. With the Rapier backend the dynamics are real
//! (gravity, contacts, friction), so the cube accelerates under the push impulse
//! and decelerates under friction; whether it lands within tolerance of the goal
//! is an externally meaningful success/failure label, not a hand-set flag.
//!
//! The runner is generic over the backend so the *same* scripted commands can be
//! replayed on the kinematic [`NullPhysicsBackend`](crate::physics::NullPhysicsBackend)
//! as a cross-backend robustness/confound check. The two backends will NOT agree
//! on the trajectory or the label — Null has no gravity/friction — and that
//! divergence is the point: it shows the run-log/label contract is backend-agnostic
//! while the physics is not a claim of ground truth.
//!
//! ## Verifiability
//!
//! Every emitted log is flow-consistent: each `Flow_gt` displacement equals the
//! difference of consecutive `SimSnapshot` poses, so [`crate::verify_flow_gt`]
//! passes. It is also deterministically self-reproducible (re-running the same
//! params yields an identical log). It is deliberately **not** constant-velocity
//! replayable via [`crate::verify_sim_replay`]: that model is for the kinematic
//! `DeterministicObjectSim`, and a real contact trajectory will not match it.

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use pid_runlog::{
    Actor, ActorType, Pose, RunLogEvent, RunStatus, SimObjectSnapshot, RUN_LOG_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::physics::{PhysicsBackend, PhysicsWorldConfig};

const NANOS_PER_SECOND: f64 = 1_000_000_000.0;
const MAX_PUSH_CONTROL_STEPS: usize = 10_000;
const MAX_PUSH_ID_BYTES: usize = 256;

fn duration_ns(dt_secs: f64) -> Result<u64> {
    let rounded = (dt_secs * NANOS_PER_SECOND).round();
    if !rounded.is_finite() || rounded < 1.0 || rounded >= u64::MAX as f64 {
        bail!("dt must round to a representable positive nanosecond interval");
    }
    Ok(rounded as u64)
}

fn validate_state(state: &crate::physics::RigidBodyState, object_id: &str) -> Result<()> {
    if state.object_id != object_id {
        bail!(
            "physics backend returned unexpected object {}, expected {object_id}",
            state.object_id
        );
    }
    if !state
        .position
        .iter()
        .chain(state.orientation_xyzw.iter())
        .chain(state.linear_velocity.iter())
        .chain(state.angular_velocity.iter())
        .all(|value| value.is_finite())
    {
        bail!("physics backend returned a non-finite state for {object_id}");
    }
    let orientation_scale = state
        .orientation_xyzw
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if orientation_scale == 0.0 {
        bail!("physics backend returned a zero-norm orientation for {object_id}");
    }
    Ok(())
}

fn validate_params(
    world_config: &PhysicsWorldConfig,
    params: &PushTaskParams,
) -> Result<(usize, u64)> {
    if params.run_id.is_empty() || params.run_id.len() > MAX_PUSH_ID_BYTES {
        bail!("run_id must be a non-empty bounded string");
    }
    if params.object_id.is_empty() || params.object_id.len() > MAX_PUSH_ID_BYTES {
        bail!("object_id must be a non-empty bounded string");
    }
    if !params.start_position.iter().all(|value| value.is_finite()) {
        bail!("start_position must be finite");
    }
    if !params.half_extent.is_finite() || params.half_extent <= 0.0 {
        bail!("half_extent must be positive and finite");
    }
    if !params.mass.is_finite() || params.mass <= 0.0 {
        bail!("mass must be positive and finite");
    }
    if !params.push_impulse.is_finite() {
        bail!("push_impulse must be finite");
    }
    if !params.goal_x.is_finite() {
        bail!("goal_x must be finite");
    }
    if !(params.start_position[0] - params.goal_x).is_finite() {
        bail!("start_position and goal_x must have a representable finite separation");
    }
    if !(params.dt.is_finite() && params.dt > 0.0) {
        bail!("dt must be positive and finite");
    }
    if !(params.tolerance.is_finite() && params.tolerance > 0.0) {
        bail!("tolerance must be positive and finite");
    }
    if !world_config.gravity.iter().all(|value| value.is_finite())
        || !world_config.fixed_dt_secs.is_finite()
        || world_config.fixed_dt_secs <= 0.0
        || world_config.max_substeps == 0
        || world_config.max_substeps > MAX_PUSH_CONTROL_STEPS
    {
        bail!("world_config must contain finite gravity, positive fixed dt, and bounded substeps");
    }
    let total_steps = params
        .settle_steps
        .checked_add(1)
        .and_then(|value| value.checked_add(params.coast_steps))
        .ok_or_else(|| anyhow::anyhow!("push-task control-step count overflow"))?;
    if total_steps > MAX_PUSH_CONTROL_STEPS {
        bail!("push-task control steps exceed the {MAX_PUSH_CONTROL_STEPS}-step limit");
    }
    let dt_ns = duration_ns(params.dt)?;
    dt_ns
        .checked_mul(
            u64::try_from(total_steps)
                .map_err(|_| anyhow::anyhow!("push-task step count is not representable"))?,
        )
        .ok_or_else(|| anyhow::anyhow!("push-task timestamp range overflow"))?;
    Ok((total_steps, dt_ns))
}

/// Parameters for a single scripted push-to-goal episode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PushTaskParams {
    /// Run id recorded in the run log.
    pub run_id: String,
    /// Object id for the pushed cube.
    pub object_id: String,
    /// Initial cube center (m). Choose `z` slightly above the resting height so
    /// the cube settles onto the ground during the settle phase.
    pub start_position: [f64; 3],
    /// Cube half-extent (m); the cube is a `2*half_extent` box.
    pub half_extent: f64,
    /// Cube mass (kg).
    pub mass: f64,
    /// Linear impulse applied in +x at the push step (N·s).
    pub push_impulse: f64,
    /// Steps to let the cube settle under gravity before the push.
    pub settle_steps: usize,
    /// Steps to coast (friction-only) after the push.
    pub coast_steps: usize,
    /// Control timestep (s).
    pub dt: f64,
    /// Target x-position (m) the cube center should reach.
    pub goal_x: f64,
    /// Success tolerance on `|final_x - goal_x|` (m).
    pub tolerance: f64,
}

impl Default for PushTaskParams {
    fn default() -> Self {
        Self {
            run_id: "push-task".to_string(),
            object_id: "cube".to_string(),
            start_position: [0.0, 0.0, 0.05],
            half_extent: 0.025,
            mass: 0.1,
            push_impulse: 0.2,
            settle_steps: 40,
            coast_steps: 260,
            dt: 0.01,
            goal_x: 0.3,
            tolerance: 0.05,
        }
    }
}

/// Outcome of a [`run_push_episode`] call.
#[derive(Debug, Clone)]
pub struct PushEpisode {
    /// Canonical run-log events, ready to append to a [`pid_runlog::RunLogWriter`].
    pub events: Vec<RunLogEvent>,
    /// Whether the cube finished within `tolerance` of `goal_x` (and stayed on the
    /// ground / finite).
    pub success: bool,
    /// Final cube center position (m).
    pub final_position: [f64; 3],
    /// `|final_x - goal_x|` (m).
    pub distance_to_goal: f64,
    /// Maximum contact-pair count seen across the episode (0 for backends without
    /// contacts, e.g. the Null backend).
    pub max_contact_count: usize,
    /// Total control steps executed (settle + 1 push + coast).
    pub total_steps: usize,
}

/// Run a scripted push-to-goal episode and return the canonical run-log events
/// plus the externally meaningful success label.
///
/// The caller is responsible for any backend-specific scene setup (e.g. calling
/// `add_ground_slab` on the Rapier backend) *before* this function adds the cube.
/// `world_config` is recorded for provenance and is not re-applied to the backend.
pub fn run_push_episode<B: PhysicsBackend>(
    backend: &mut B,
    world_config: &PhysicsWorldConfig,
    params: &PushTaskParams,
) -> Result<PushEpisode> {
    let (total_steps, dt_ns) = validate_params(world_config, params)?;
    if backend.body_count() != 0 {
        bail!("push-task backend must not contain pre-existing dynamic bodies");
    }

    let mut events = Vec::new();
    let event_capacity = total_steps
        .checked_mul(4)
        .and_then(|value| value.checked_add(6))
        .ok_or_else(|| anyhow::anyhow!("push-task event-count overflow"))?;
    events
        .try_reserve_exact(event_capacity)
        .map_err(|error| anyhow::anyhow!("failed to reserve bounded push-task events: {error}"))?;

    // Provenance: backend + solver + task parameters.
    let config = json!({
        "task": "push_to_goal",
        "backend": backend.name(),
        "solver": {
            "gravity": world_config.gravity,
            "fixed_dt_secs": world_config.fixed_dt_secs,
            "max_substeps": world_config.max_substeps,
        },
        "params": params,
    });
    let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;

    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "pid-rapier-harness".to_string());
    metadata.insert("backend".to_string(), backend.name().to_string());
    events.push(RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: params.run_id.clone(),
        timestamp_ns: 0,
        config_hash: config_hash.clone(),
        metadata,
    });
    events.push(RunLogEvent::ConfigLogged {
        timestamp_ns: 0,
        config_hash,
        config,
    });

    // Spawn the cube (at rest).
    backend.add_rigid_body(
        &params.object_id,
        params.start_position,
        [0.0, 0.0, 0.0, 1.0],
        [params.half_extent; 3],
        params.mass,
    )?;
    if backend.body_count() != 1 {
        bail!("push-task backend did not register exactly one dynamic body");
    }

    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: "push-policy".to_string(),
        session_id: Some(params.run_id.clone()),
    };

    // Step 0 baseline snapshot (needed so the step-1 Flow_gt has a predecessor).
    let mut step: u64 = 0;
    let mut timestamp_ns: u64 = 0;
    let baseline = backend.snapshot();
    if baseline.len() != 1 {
        bail!("push-task backend baseline must contain exactly one dynamic body");
    }
    validate_state(&baseline[0], &params.object_id)?;
    let mut prev_positions = snapshot_positions(&baseline);
    let mut last_state = baseline[0].clone();
    events.push(snapshot_event(&baseline, step, timestamp_ns));

    let mut max_contact_count = 0usize;
    let push_step_index = params.settle_steps; // 0-based control step at which to push

    for control_step in 0..total_steps {
        step = step
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("push-task step counter overflow"))?;
        timestamp_ns = timestamp_ns
            .checked_add(dt_ns)
            .ok_or_else(|| anyhow::anyhow!("push-task timestamp overflow"))?;

        // Scripted action for this step.
        let (action_type, payload) = if control_step == push_step_index {
            backend.apply_impulse(&params.object_id, [params.push_impulse, 0.0, 0.0])?;
            (
                "push",
                json!({ "impulse": [params.push_impulse, 0.0, 0.0] }),
            )
        } else {
            ("coast", json!({}))
        };
        events.push(RunLogEvent::ActionApplied {
            step,
            timestamp_ns,
            actor: actor.clone(),
            action_type: action_type.to_string(),
            payload_hash: pid_runlog::canonical_json_hash_v2(&payload)?,
            payload,
        });

        let report = backend.step(params.dt)?;
        if report.step != step || report.timestamp_ns != timestamp_ns {
            bail!("physics backend step/time diverged from the recorded control timeline");
        }
        if report.substeps == 0 || report.substeps > world_config.max_substeps {
            bail!("physics backend reported an invalid substep count");
        }
        if report.bodies.len() != 1 {
            bail!("physics backend report must contain exactly one dynamic body");
        }
        validate_state(&report.bodies[0], &params.object_id)?;
        last_state = report.bodies[0].clone();
        max_contact_count = max_contact_count.max(report.contact_count);

        // Snapshot + per-object pose + real Flow_gt (= pose delta).
        events.push(snapshot_event(&report.bodies, step, timestamp_ns));
        let positions = snapshot_positions(&report.bodies);
        for state in &report.bodies {
            events.push(RunLogEvent::ObjectPose {
                step,
                timestamp_ns,
                object_id: state.object_id.clone(),
                pose: Pose {
                    position: state.position,
                    orientation_xyzw: state.orientation_xyzw,
                },
            });
            let prev = prev_positions
                .get(&state.object_id)
                .copied()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "missing predecessor position for physics object {}",
                        state.object_id
                    )
                })?;
            let displacement = [
                state.position[0] - prev[0],
                state.position[1] - prev[1],
                state.position[2] - prev[2],
            ];
            if !displacement.iter().all(|value| value.is_finite()) {
                bail!(
                    "physics backend produced an unrepresentable displacement for {}",
                    state.object_id
                );
            }
            events.push(RunLogEvent::FlowGt {
                step,
                timestamp_ns,
                object_id: state.object_id.clone(),
                flow: vec![displacement],
            });
        }
        prev_positions = positions;
    }

    // Determine the success label from the final physical state.
    let final_position = last_state.position;
    let on_ground_and_finite =
        final_position.iter().all(|c| c.is_finite()) && final_position[2] > -0.05; // did not tunnel through the ground
    let distance_to_goal = (final_position[0] - params.goal_x).abs();
    if !distance_to_goal.is_finite() {
        bail!("final distance to goal is not representable as a finite value");
    }
    let success = on_ground_and_finite && distance_to_goal <= params.tolerance;

    events.push(RunLogEvent::LabelObserved {
        step,
        timestamp_ns,
        name: "success".to_string(),
        value: json!(success),
        metadata: [
            ("goal_x".to_string(), params.goal_x.to_string()),
            ("final_x".to_string(), final_position[0].to_string()),
            ("distance_to_goal".to_string(), distance_to_goal.to_string()),
            ("tolerance".to_string(), params.tolerance.to_string()),
        ]
        .into_iter()
        .collect(),
    });

    events.push(RunLogEvent::RunEnded {
        run_id: params.run_id.clone(),
        timestamp_ns,
        status: RunStatus::Succeeded,
        message: Some(format!(
            "push_to_goal: success={success}, final_x={:.4}, goal_x={:.4}",
            final_position[0], params.goal_x
        )),
    });

    Ok(PushEpisode {
        events,
        success,
        final_position,
        distance_to_goal,
        max_contact_count,
        total_steps,
    })
}

fn snapshot_positions(states: &[crate::physics::RigidBodyState]) -> BTreeMap<String, [f64; 3]> {
    states
        .iter()
        .map(|state| (state.object_id.clone(), state.position))
        .collect()
}

fn snapshot_event(
    states: &[crate::physics::RigidBodyState],
    step: u64,
    timestamp_ns: u64,
) -> RunLogEvent {
    RunLogEvent::SimSnapshot {
        step,
        timestamp_ns,
        objects: states
            .iter()
            .map(|state| SimObjectSnapshot {
                object_id: state.object_id.clone(),
                pose: Pose {
                    position: state.position,
                    orientation_xyzw: state.orientation_xyzw,
                },
                velocity: state.linear_velocity,
            })
            .collect(),
        metadata: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::NullPhysicsBackend;

    #[test]
    fn null_backend_push_episode_emits_valid_flow_consistent_log() {
        // The Null backend has no gravity/contacts, but the event/label/flow
        // contract must still be valid and flow-consistent.
        let mut backend = NullPhysicsBackend::new();
        let params = PushTaskParams {
            settle_steps: 2,
            coast_steps: 3,
            ..Default::default()
        };
        let episode =
            run_push_episode(&mut backend, &PhysicsWorldConfig::default(), &params).unwrap();
        let validation = pid_runlog::validate_events(&episode.events).unwrap();
        assert!(validation.is_valid(), "{:?}", validation.issues);
        // Flow_gt equals consecutive pose deltas.
        let flow = crate::verify_flow_gt(&episode.events, 1e-9);
        assert!(flow.is_valid(), "{:?}", flow.issues);
        assert!(flow.checked_flows >= 1);
        // Null has no contacts.
        assert_eq!(episode.max_contact_count, 0);
        assert_eq!(episode.total_steps, 6);
    }

    #[test]
    fn null_backend_push_episode_is_deterministic() {
        let params = PushTaskParams {
            settle_steps: 1,
            coast_steps: 2,
            ..Default::default()
        };
        let run = || {
            let mut b = NullPhysicsBackend::new();
            run_push_episode(&mut b, &PhysicsWorldConfig::default(), &params)
                .unwrap()
                .events
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn push_episode_rejects_invalid_or_unbounded_parameters_before_mutation() {
        let world = PhysicsWorldConfig::default();
        let invalid = [
            PushTaskParams {
                push_impulse: f64::NAN,
                ..Default::default()
            },
            PushTaskParams {
                start_position: [f64::INFINITY, 0.0, 0.0],
                ..Default::default()
            },
            PushTaskParams {
                half_extent: 0.0,
                ..Default::default()
            },
            PushTaskParams {
                mass: -1.0,
                ..Default::default()
            },
            PushTaskParams {
                goal_x: f64::NAN,
                ..Default::default()
            },
            PushTaskParams {
                start_position: [f64::MAX, 0.0, 0.0],
                goal_x: -f64::MAX,
                ..Default::default()
            },
            PushTaskParams {
                dt: 0.1e-9,
                ..Default::default()
            },
            PushTaskParams {
                settle_steps: MAX_PUSH_CONTROL_STEPS,
                coast_steps: 0,
                ..Default::default()
            },
        ];
        for params in invalid {
            let mut backend = NullPhysicsBackend::new();
            assert!(run_push_episode(&mut backend, &world, &params).is_err());
            assert_eq!(backend.body_count(), 0);
        }
    }

    #[test]
    fn push_episode_rejects_finite_states_with_overflowing_flow() {
        struct OverflowDisplacementBackend {
            object_id: Option<String>,
        }

        impl PhysicsBackend for OverflowDisplacementBackend {
            fn name(&self) -> &str {
                "overflow-displacement-test"
            }

            fn body_count(&self) -> usize {
                usize::from(self.object_id.is_some())
            }

            fn add_rigid_body(
                &mut self,
                object_id: &str,
                _position: [f64; 3],
                _orientation_xyzw: [f64; 4],
                _half_extents: [f64; 3],
                _mass_kg: f64,
            ) -> Result<()> {
                self.object_id = Some(object_id.to_string());
                Ok(())
            }

            fn remove_body(&mut self, _object_id: &str) -> Result<()> {
                bail!("not used")
            }

            fn apply_impulse(&mut self, _object_id: &str, _impulse: [f64; 3]) -> Result<()> {
                Ok(())
            }

            fn set_linear_velocity(&mut self, _object_id: &str, _velocity: [f64; 3]) -> Result<()> {
                bail!("not used")
            }

            fn step(&mut self, _dt_secs: f64) -> Result<crate::physics::PhysicsStepReport> {
                Ok(crate::physics::PhysicsStepReport {
                    step: 1,
                    timestamp_ns: 10_000_000,
                    substeps: 1,
                    contact_count: 0,
                    bodies: vec![crate::physics::RigidBodyState {
                        object_id: self.object_id.clone().expect("body inserted"),
                        position: [f64::MAX, 0.0, 0.0],
                        orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                        linear_velocity: [0.0; 3],
                        angular_velocity: [0.0; 3],
                    }],
                })
            }

            fn snapshot(&self) -> Vec<crate::physics::RigidBodyState> {
                self.object_id
                    .iter()
                    .map(|object_id| crate::physics::RigidBodyState {
                        object_id: object_id.clone(),
                        position: [-f64::MAX, 0.0, 0.0],
                        orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                        linear_velocity: [0.0; 3],
                        angular_velocity: [0.0; 3],
                    })
                    .collect()
            }

            fn reset(&mut self) {
                self.object_id = None;
            }
        }

        let mut backend = OverflowDisplacementBackend { object_id: None };
        let params = PushTaskParams {
            settle_steps: 0,
            coast_steps: 0,
            ..Default::default()
        };
        let error = run_push_episode(&mut backend, &PhysicsWorldConfig::default(), &params)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unrepresentable displacement"));
    }

    #[test]
    fn push_episode_requires_a_fresh_empty_backend_timeline() {
        let world = PhysicsWorldConfig::default();
        let params = PushTaskParams {
            settle_steps: 0,
            coast_steps: 0,
            ..Default::default()
        };

        let mut occupied = NullPhysicsBackend::new();
        occupied
            .add_rigid_body("existing", [0.0; 3], [0.0, 0.0, 0.0, 1.0], [0.1; 3], 1.0)
            .unwrap();
        assert!(run_push_episode(&mut occupied, &world, &params)
            .unwrap_err()
            .to_string()
            .contains("pre-existing"));

        let mut advanced = NullPhysicsBackend::new();
        advanced.step(0.01).unwrap();
        assert!(run_push_episode(&mut advanced, &world, &params)
            .unwrap_err()
            .to_string()
            .contains("diverged"));
    }

    #[cfg(feature = "rapier")]
    mod rapier {
        use super::*;
        use crate::physics::rapier_adapter::RapierBackend;

        fn run_with_impulse(push_impulse: f64) -> PushEpisode {
            let world = PhysicsWorldConfig::default();
            let mut backend = RapierBackend::new(world.clone());
            backend.add_ground_slab(5.0, 0.1, 0.5).unwrap();
            let params = PushTaskParams {
                push_impulse,
                ..Default::default()
            };
            run_push_episode(&mut backend, &world, &params).unwrap()
        }

        #[test]
        fn rapier_push_episode_log_is_valid_and_flow_consistent() {
            let ep = run_with_impulse(0.2);
            let validation = pid_runlog::validate_events(&ep.events).unwrap();
            assert!(validation.is_valid(), "{:?}", validation.issues);
            // Real contacts occurred (cube on ground).
            assert!(ep.max_contact_count >= 1);
            // Flow_gt is consistent with pose deltas under real dynamics.
            let flow = crate::verify_flow_gt(&ep.events, 1e-9);
            assert!(flow.is_valid(), "{:?}", flow.issues);
        }

        #[test]
        fn rapier_push_task_produces_both_success_and_failure_labels() {
            // A weak push undershoots the goal (failure); a tuned push lands within
            // tolerance (success). Same task, different impulse -> different label,
            // derived from real physics rather than a hand-set flag.
            let weak = run_with_impulse(0.02);
            assert!(!weak.success, "weak push unexpectedly succeeded");
            assert!(weak.final_position[0] < 0.3 - 0.05 + 1e-9);

            // Search a deterministic impulse grid for a success (the dynamics are
            // monotone-ish in impulse, so a hit exists between undershoot/overshoot).
            let mut found_success = false;
            for i in 0..40 {
                let impulse = 0.05 + 0.01 * i as f64;
                let ep = run_with_impulse(impulse);
                if ep.success {
                    found_success = true;
                    assert!(ep.distance_to_goal <= 0.05 + 1e-9);
                    break;
                }
            }
            assert!(found_success, "no impulse in the grid reached the goal");
        }

        #[test]
        fn rapier_push_episode_is_deterministic() {
            let a = run_with_impulse(0.18);
            let b = run_with_impulse(0.18);
            assert_eq!(a.success, b.success);
            assert_eq!(a.final_position, b.final_position);
            assert_eq!(a.events, b.events);
        }
    }
}
