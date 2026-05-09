use anyhow::{bail, Result};
use pid_runlog::{Pose, RunLogEvent, SimObjectSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimObject {
    pub object_id: String,
    pub pose: Pose,
    pub velocity: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlowGtRecord {
    pub object_id: String,
    pub displacement: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimStepResult {
    pub step: u64,
    pub timestamp_ns: u64,
    pub flow_gt: Vec<FlowGtRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeterministicObjectSim {
    step: u64,
    timestamp_ns: u64,
    objects: BTreeMap<String, SimObject>,
}

impl Default for DeterministicObjectSim {
    fn default() -> Self {
        Self::new()
    }
}

impl DeterministicObjectSim {
    pub fn new() -> Self {
        Self {
            step: 0,
            timestamp_ns: 0,
            objects: BTreeMap::new(),
        }
    }

    pub fn step(&self) -> u64 {
        self.step
    }

    pub fn timestamp_ns(&self) -> u64 {
        self.timestamp_ns
    }

    pub fn objects(&self) -> impl Iterator<Item = &SimObject> {
        self.objects.values()
    }

    pub fn upsert_object(&mut self, object: SimObject) {
        self.objects.insert(object.object_id.clone(), object);
    }

    pub fn reset(&mut self) {
        self.step = 0;
        self.timestamp_ns = 0;
        self.objects.clear();
    }

    pub fn step_fixed(&mut self, dt_secs: f64) -> Result<SimStepResult> {
        if !dt_secs.is_finite() || dt_secs <= 0.0 {
            bail!("dt_secs must be positive and finite");
        }

        let dt_ns = (dt_secs * 1_000_000_000.0).round() as u64;
        let mut flow_gt = Vec::with_capacity(self.objects.len());
        for object in self.objects.values_mut() {
            let displacement = [
                object.velocity[0] * dt_secs,
                object.velocity[1] * dt_secs,
                object.velocity[2] * dt_secs,
            ];
            object.pose.position[0] += displacement[0];
            object.pose.position[1] += displacement[1];
            object.pose.position[2] += displacement[2];
            flow_gt.push(FlowGtRecord {
                object_id: object.object_id.clone(),
                displacement,
            });
        }
        self.step += 1;
        self.timestamp_ns += dt_ns;
        Ok(SimStepResult {
            step: self.step,
            timestamp_ns: self.timestamp_ns,
            flow_gt,
        })
    }

    pub fn snapshot_event(&self) -> RunLogEvent {
        RunLogEvent::SimSnapshot {
            step: self.step,
            timestamp_ns: self.timestamp_ns,
            objects: self
                .objects
                .values()
                .map(|object| SimObjectSnapshot {
                    object_id: object.object_id.clone(),
                    pose: object.pose.clone(),
                    velocity: object.velocity,
                })
                .collect(),
            metadata: BTreeMap::new(),
        }
    }

    pub fn pose_events(&self) -> Vec<RunLogEvent> {
        self.objects
            .values()
            .map(|object| RunLogEvent::ObjectPose {
                step: self.step,
                timestamp_ns: self.timestamp_ns,
                object_id: object.object_id.clone(),
                pose: object.pose.clone(),
            })
            .collect()
    }
}

impl SimStepResult {
    pub fn flow_events(&self) -> Vec<RunLogEvent> {
        self.flow_gt
            .iter()
            .map(|record| RunLogEvent::FlowGt {
                step: self.step,
                timestamp_ns: self.timestamp_ns,
                object_id: record.object_id.clone(),
                flow: vec![record.displacement],
            })
            .collect()
    }
}

pub fn demo_sim() -> DeterministicObjectSim {
    let mut sim = DeterministicObjectSim::new();
    sim.upsert_object(SimObject {
        object_id: "red_cube".to_string(),
        pose: Pose {
            position: [0.0, 0.0, 0.025],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        velocity: [0.1, 0.0, 0.0],
    });
    sim.upsert_object(SimObject {
        object_id: "blue_cube".to_string(),
        pose: Pose {
            position: [0.2, 0.0, 0.025],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        },
        velocity: [0.0, 0.05, 0.0],
    });
    sim
}

#[cfg(test)]
mod tests {
    use super::*;
    use pid_runlog::replay_events;

    #[test]
    fn fixed_step_is_deterministic() {
        let mut a = demo_sim();
        let mut b = demo_sim();
        for _ in 0..3 {
            a.step_fixed(0.1).unwrap();
            b.step_fixed(0.1).unwrap();
        }
        assert_eq!(a, b);
        assert_eq!(a.step(), 3);
    }

    #[test]
    fn flow_gt_matches_velocity_times_dt() {
        let mut sim = demo_sim();
        let result = sim.step_fixed(0.5).unwrap();
        let red = result
            .flow_gt
            .iter()
            .find(|record| record.object_id == "red_cube")
            .unwrap();
        assert_eq!(red.displacement, [0.05, 0.0, 0.0]);
    }

    #[test]
    fn sim_events_replay_into_runlog_state() {
        let mut sim = demo_sim();
        let step = sim.step_fixed(0.1).unwrap();
        let mut events = vec![sim.snapshot_event()];
        events.extend(sim.pose_events());
        events.extend(step.flow_events());
        let state = replay_events(&events);
        assert_eq!(state.sim_snapshots, 1);
        assert_eq!(state.object_poses.len(), 2);
        assert_eq!(state.flow_gt_records, 2);
    }
}
