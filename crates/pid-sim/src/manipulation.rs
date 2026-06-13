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
    if params.object_id.is_empty() {
        bail!("object_id must not be empty");
    }
    if !(params.dt.is_finite() && params.dt > 0.0) {
        bail!("dt must be positive and finite");
    }
    if !(params.tolerance.is_finite() && params.tolerance > 0.0) {
        bail!("tolerance must be positive and finite");
    }

    let mut events = Vec::new();
    let dt_ns = (params.dt * 1_000_000_000.0).round() as u64;

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
    let config_hash = pid_runlog::canonical_json_hash(&config)?;

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

    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: "push-policy".to_string(),
        session_id: Some(params.run_id.clone()),
    };

    // Step 0 baseline snapshot (needed so the step-1 Flow_gt has a predecessor).
    let mut step: u64 = 0;
    let mut timestamp_ns: u64 = 0;
    let mut prev_positions = snapshot_positions(backend);
    events.push(snapshot_event(backend, step, timestamp_ns));

    let mut max_contact_count = 0usize;
    let push_step_index = params.settle_steps; // 0-based control step at which to push
    let total_steps = params.settle_steps + 1 + params.coast_steps;

    for control_step in 0..total_steps {
        step += 1;
        timestamp_ns += dt_ns;

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
            payload_hash: pid_runlog::canonical_json_hash(&payload)?,
            payload,
        });

        let report = backend.step(params.dt)?;
        max_contact_count = max_contact_count.max(report.contact_count);

        // Snapshot + per-object pose + real Flow_gt (= pose delta).
        events.push(snapshot_event(backend, step, timestamp_ns));
        let positions = snapshot_positions(backend);
        for state in backend.snapshot() {
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
                .unwrap_or(state.position);
            let displacement = [
                state.position[0] - prev[0],
                state.position[1] - prev[1],
                state.position[2] - prev[2],
            ];
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
    let final_state = backend
        .snapshot()
        .into_iter()
        .find(|s| s.object_id == params.object_id)
        .ok_or_else(|| anyhow::anyhow!("cube vanished from the backend"))?;
    let final_position = final_state.position;
    let on_ground_and_finite =
        final_position.iter().all(|c| c.is_finite()) && final_position[2] > -0.05; // did not tunnel through the ground
    let distance_to_goal = (final_position[0] - params.goal_x).abs();
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

fn snapshot_positions<B: PhysicsBackend>(backend: &B) -> BTreeMap<String, [f64; 3]> {
    backend
        .snapshot()
        .into_iter()
        .map(|s| (s.object_id, s.position))
        .collect()
}

fn snapshot_event<B: PhysicsBackend>(backend: &B, step: u64, timestamp_ns: u64) -> RunLogEvent {
    RunLogEvent::SimSnapshot {
        step,
        timestamp_ns,
        objects: backend
            .snapshot()
            .into_iter()
            .map(|s| SimObjectSnapshot {
                object_id: s.object_id,
                pose: Pose {
                    position: s.position,
                    orientation_xyzw: s.orientation_xyzw,
                },
                velocity: s.linear_velocity,
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
        let validation = pid_runlog::validate_events(&episode.events);
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

    #[cfg(feature = "rapier")]
    mod rapier {
        use super::*;
        use crate::physics::rapier_adapter::RapierBackend;

        fn run_with_impulse(push_impulse: f64) -> PushEpisode {
            let world = PhysicsWorldConfig::default();
            let mut backend = RapierBackend::new(world.clone());
            backend.add_ground_slab(5.0, 0.1, 0.5);
            let params = PushTaskParams {
                push_impulse,
                ..Default::default()
            };
            run_push_episode(&mut backend, &world, &params).unwrap()
        }

        #[test]
        fn rapier_push_episode_log_is_valid_and_flow_consistent() {
            let ep = run_with_impulse(0.2);
            let validation = pid_runlog::validate_events(&ep.events);
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
