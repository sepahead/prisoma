use ncp_local::local::{
    local_digest, LocalBinding, LocalCode, LocalError, LocalOperation, LocalOutcome, LocalRole,
};
use ncp_local::local_data::{
    BodyResult, BodyStep, NeuralProposal, NeuralStep, PrepareData, RunPlan, Snapshot,
};
use serde_json::{json, Value};

use crate::contract::*;

pub(crate) fn invalid() -> LocalError {
    LocalError(LocalCode::InvalidInput)
}
pub(crate) fn parse<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, LocalError> {
    serde_json::from_value(value.clone()).map_err(|_| invalid())
}
fn equal<T: PartialEq>(actual: T, expected: T) -> Result<(), LocalError> {
    if actual == expected {
        Ok(())
    } else {
        Err(LocalError(LocalCode::Binding))
    }
}
fn hash(value: &Value) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn research() -> Value {
    json!({"class":"exploratory_research","profile":"subset_magnitude_v0_9"})
}

fn exchange(
    exchange: &Exchange,
    binding: &LocalBinding,
    operation: LocalOperation,
    sequence: u64,
) -> Result<(), LocalError> {
    binding.validate()?;
    exchange.response.verify(binding, &exchange.request)?;
    if exchange.request.schema != "ncp.local.request.v1"
        || exchange.request.operation != operation
        || exchange.request.sequence != sequence
        || exchange.response.outcome != LocalOutcome::Committed
        || exchange.response.code != LocalCode::Ok
    {
        return Err(invalid());
    }
    Ok(())
}

/// Pure state for the accepted journal prefix. No file or protocol execution occurs here.
#[derive(Clone)]
pub(crate) struct CausalState {
    pub plan: RunPlan,
    pub plan_digest: String,
    pub quota: u64,
    pub snapshot: Snapshot,
    pub reserved: Option<u64>,
    pub completed: u64,
    pub terminal: Option<bool>,
    body_binding: LocalBinding,
    neural_binding: LocalBinding,
    monitor_binding: LocalBinding,
    monitor_configuration_digest: Value,
}

impl CausalState {
    pub fn prepare(header: &Header) -> Result<Self, LocalError> {
        header.binding.validate()?;
        if header.binding.role != LocalRole::Capture
            || header.preparation.application_profile != APPLICATION_PROFILE
        {
            return Err(invalid());
        }
        let plan = header.preparation.plan.clone();
        plan.validate()?;
        let plan_digest = plan.digest()?;
        let config: CaptureConfiguration = parse(&header.preparation.configuration)?;
        if config.max_capture_bytes > MAX_CAPTURE_BYTES {
            return Err(LocalError(LocalCode::Capacity));
        }
        let mut generations = vec![header.binding.generation.clone()];
        for (item, role, profile) in [
            (
                &config.body_preparation,
                LocalRole::Body,
                "crebain.local-kinematic-kalman.v1",
            ),
            (
                &config.neural_preparation,
                LocalRole::Neural,
                "engram.local-nest-rate-controller.v1",
            ),
            (
                &config.monitor_preparation,
                LocalRole::Monitor,
                "galadriel.scalar-nis-record-only.v1",
            ),
        ] {
            let binding = &item.response.binding;
            if binding.role != role
                || binding.run_id != header.binding.run_id
                || binding.profile_digest != header.binding.profile_digest
                || generations.contains(&binding.generation)
            {
                return Err(invalid());
            }
            generations.push(binding.generation.clone());
            exchange(item, binding, LocalOperation::Prepare, 1)?;
            let prepared: PrepareData = parse(&item.request.body)?;
            equal(&prepared.plan, &plan)?;
            if prepared.application_profile != profile
                || item.response.body["application_profile"] != profile
                || item.response.body["plan_digest"] != plan_digest
            {
                return Err(invalid());
            }
        }
        let body_binding = config.body_preparation.response.binding.clone();
        let neural_binding = config.neural_preparation.response.binding.clone();
        let monitor_binding = config.monitor_preparation.response.binding.clone();
        equal(
            &config.body_preparation.request.body["configuration"]["expected_neural_generation"],
            &json!(neural_binding.generation),
        )?;
        equal(
            &config.monitor_preparation.request.body["configuration"]["body_generation"],
            &json!(body_binding.generation),
        )?;
        let monitor = &config.monitor_preparation.response.body;
        equal(
            &monitor["body_binding"],
            &serde_json::to_value(&body_binding).map_err(|_| invalid())?,
        )?;
        if monitor["classification"] != research()
            || monitor["calibrated_posterior"] != false
            || !hash(&monitor["configuration_digest"])
        {
            return Err(invalid());
        }
        let neural = &config.neural_preparation.response.body;
        if neural["neural_time_us"] != 0
            || neural["calibrated_posterior"] != false
            || neural["network_digest"]
                != local_digest("ncp.local.plan.v1", &neural["network_descriptor"])?
        {
            return Err(invalid());
        }
        let snapshot: Snapshot = parse(&config.body_preparation.response.body["snapshot"])?;
        snapshot.validate(&plan)?;
        if snapshot.step != 0 {
            return Err(invalid());
        }
        Ok(Self {
            plan,
            plan_digest,
            quota: config.max_capture_bytes,
            snapshot,
            reserved: None,
            completed: 0,
            terminal: None,
            body_binding,
            neural_binding,
            monitor_binding,
            monitor_configuration_digest: monitor["configuration_digest"].clone(),
        })
    }
    pub fn reserve(&mut self, data: &ReserveData) -> Result<(), LocalError> {
        if self.terminal.is_some()
            || self.reserved.is_some()
            || data.plan_digest != self.plan_digest
            || data.step != self.completed + 1
            || data.step > self.plan.planned_steps
        {
            return Err(invalid());
        }
        self.reserved = Some(data.step);
        Ok(())
    }
    pub fn capture(&mut self, data: &CaptureData) -> Result<(), LocalError> {
        if self.terminal.is_some()
            || self.reserved != Some(data.step)
            || data.step != self.completed + 1
            || data.plan_digest != self.plan_digest
            || data.source_snapshot != self.snapshot
        {
            return Err(invalid());
        }
        let k = data.step;
        exchange(
            &data.neural,
            &self.neural_binding,
            LocalOperation::Step,
            k + 1,
        )?;
        equal(
            &data.neural.request.body,
            &serde_json::to_value(NeuralStep {
                source_snapshot: self.snapshot.clone(),
            })
            .map_err(|_| invalid())?,
        )?;
        let neural: NeuralProposal = parse(&data.neural.response.body)?;
        neural.validate(&self.plan, &self.snapshot)?;
        exchange(&data.body, &self.body_binding, LocalOperation::Step, k + 1)?;
        equal(
            &data.body.request.body,
            &serde_json::to_value(BodyStep {
                neural_response: data.neural.response.clone(),
            })
            .map_err(|_| invalid())?,
        )?;
        let body: BodyResult = parse(&data.body.response.body)?;
        body.validate(&self.plan)?;
        if body.step != k
            || body.source_snapshot_digest != self.snapshot.snapshot_digest
            || body.neural_result_digest != data.neural.response.result_digest
            || body.proposed_values != neural.values
            || body.selected_modes != neural.selected_modes
        {
            return Err(invalid());
        }
        exchange(
            &data.monitor,
            &self.monitor_binding,
            LocalOperation::Assess,
            k + 1,
        )?;
        equal(
            &data.monitor.request.body,
            &json!({"body_response":data.body.response}),
        )?;
        let monitor = &data.monitor.response.body;
        let expected = [
            "schema",
            "application_profile",
            "plan_digest",
            "step",
            "time_us",
            "body_result_digest",
            "snapshot_digest",
            "classification",
            "configuration_digest",
            "calibrated_posterior",
            "tracks",
        ];
        if !monitor.as_object().is_some_and(|object| {
            object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
        }) || monitor["schema"] != "galadriel.local.assessment.v1"
            || monitor["application_profile"] != "galadriel.scalar-nis-record-only.v1"
            || monitor["plan_digest"] != self.plan_digest
            || monitor["step"] != k
            || monitor["time_us"] != body.snapshot.time_us
            || monitor["body_result_digest"] != data.body.response.result_digest
            || monitor["snapshot_digest"] != body.snapshot.snapshot_digest
            || monitor["classification"] != research()
            || monitor["configuration_digest"] != self.monitor_configuration_digest
            || monitor["calibrated_posterior"] != false
        {
            return Err(invalid());
        }
        let tracks = monitor["tracks"].as_array().ok_or_else(invalid)?;
        if tracks.len() != self.plan.entity_ids.len() {
            return Err(invalid());
        }
        for (track, innovation) in tracks.iter().zip(&body.snapshot.innovations) {
            if track["entity_id"] != innovation.entity_id
                || track["source"]
                    != serde_json::to_value(&innovation.source).map_err(|_| invalid())?
                || track["modality"] != innovation.modality
                || track["dof"] != innovation.dof
            {
                return Err(invalid());
            }
        }
        self.snapshot = body.snapshot;
        self.completed = k;
        self.reserved = None;
        Ok(())
    }
    pub fn finish(&mut self, data: &FinishData) -> Result<(), LocalError> {
        if self.terminal.is_some()
            || self.reserved.is_some()
            || self.completed != self.plan.planned_steps
            || data.completed_steps != self.completed
            || data.plan_digest != self.plan_digest
        {
            return Err(invalid());
        }
        let expected = json!({"plan_digest":self.plan_digest,"completed_steps":self.completed});
        for (item, binding) in [
            (&data.neural_finish, &self.neural_binding),
            (&data.body_finish, &self.body_binding),
            (&data.monitor_finish, &self.monitor_binding),
        ] {
            exchange(item, binding, LocalOperation::Finish, self.completed + 2)?;
            equal(&item.request.body, &expected)?;
            if item.response.body["plan_digest"] != self.plan_digest
                || item.response.body["completed_steps"] != self.completed
            {
                return Err(invalid());
            }
        }
        if data.body_finish.response.body["snapshot_digest"] != self.snapshot.snapshot_digest
            || data.body_finish.response.body["planned_steps"] != self.plan.planned_steps
            || data.body_finish.response.body["cleaned_up"] != true
            || data.neural_finish.response.body["terminal_status"] != "completed"
            || data.neural_finish.response.body["calibrated_posterior"] != false
            || data.monitor_finish.response.body["calibrated_posterior"] != false
        {
            return Err(invalid());
        }
        self.terminal = Some(true);
        Ok(())
    }
    pub fn abort(&mut self, data: &AbortData) -> Result<(), LocalError> {
        if self.terminal.is_some()
            || data.plan_digest != self.plan_digest
            || data.completed_steps != self.completed
            || data.reserved_step != self.reserved
            || data.remaining_suffix != "unresolved"
        {
            return Err(invalid());
        }
        self.terminal = Some(false);
        Ok(())
    }
    pub fn apply(&mut self, event: &Event) -> Result<(), LocalError> {
        match event {
            Event::Header(_) => Err(invalid()),
            Event::Reserve(data) => self.reserve(data),
            Event::Capture(data) => self.capture(data),
            Event::Finish(data) => self.finish(data),
            Event::Abort(data) => self.abort(data),
        }
    }
}
