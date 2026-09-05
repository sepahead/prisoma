//! Synthetic producer exchanges test capture integrity, not simulation validity.
use ncp_local::local::{
    local_digest, local_profile_digest, read_local_frame, LocalBackend, LocalBinding, LocalCode,
    LocalOperation, LocalOutcome, LocalOwner, LocalRequest, LocalResponse, LocalRole,
};
use ncp_local::local_data::{
    action_layout, observation_layout, ActionMode, BodyResult, BodyStep, InnovationStatus,
    NeuralProposal, NeuralStep, PrepareData, RunPlan, ScalarInnovation, Snapshot,
};
use prisoma_ncp_local_capture::*;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "prisoma-native-capture-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn binding(role: LocalRole) -> LocalBinding {
    let suffix = match role {
        LocalRole::Body => 1,
        LocalRole::Neural => 2,
        LocalRole::Monitor => 3,
        LocalRole::Capture => 4,
    };
    LocalBinding {
        profile_digest: local_profile_digest().unwrap(),
        run_id: "00000000-0000-4000-8000-000000000000".into(),
        generation: format!("00000000-0000-4000-8000-{suffix:012}"),
        role,
    }
}
fn plan(n: usize, steps: u64) -> RunPlan {
    RunPlan {
        schema: "ncp.local.plan.v1".into(),
        entity_ids: (0..n).map(|i| format!("entity-{i}")).collect(),
        planned_steps: steps,
        step_us: 10_000,
        resolution_us: 100,
        readout_delay_us: 100,
        seed: 42,
        execution_mode: "direct_simulation".into(),
        capture_mode: "lossless_bounded".into(),
        monitor_mode: "record_only".into(),
        calibrated_posterior: false,
        observation_layout: observation_layout(),
        action_layout: action_layout(),
    }
}
fn source(plan: &RunPlan, step: u64) -> Snapshot {
    let mut value = Snapshot {
        schema: "ncp.local.snapshot.v1".into(),
        plan_digest: plan.digest().unwrap(),
        step,
        time_us: plan.time_us(step).unwrap(),
        entity_ids: plan.entity_ids.clone(),
        available: vec![true; plan.entity_ids.len()],
        values: vec![0.0; plan.entity_ids.len() * 6],
        innovations: plan
            .entity_ids
            .iter()
            .map(|id| ScalarInnovation {
                entity_id: id.clone(),
                modality: "visual".into(),
                dof: 3,
                status: if step == 0 {
                    InnovationStatus::Birth
                } else {
                    InnovationStatus::Unavailable
                },
                nis: None,
                source: None,
            })
            .collect(),
        snapshot_digest: String::new(),
    };
    value.seal(plan).unwrap();
    value
}
fn request(role: LocalRole, sequence: u64, operation: LocalOperation, body: Value) -> LocalRequest {
    let binding = binding(role);
    let mut req = LocalRequest {
        schema: "ncp.local.request.v1".into(),
        profile_digest: binding.profile_digest,
        run_id: binding.run_id,
        generation: binding.generation,
        sequence,
        operation,
        body,
        request_digest: String::new(),
    };
    req.seal().unwrap();
    req
}
fn seal(response: &mut LocalResponse) {
    let mut value = serde_json::to_value(&*response).unwrap();
    value.as_object_mut().unwrap().remove("result_digest");
    response.result_digest = local_digest("ncp.local.response.v1", &value).unwrap();
}
fn exchange(
    role: LocalRole,
    sequence: u64,
    operation: LocalOperation,
    input: Value,
    output: Value,
) -> Exchange {
    let req = request(role, sequence, operation, input);
    let mut response = LocalResponse {
        schema: "ncp.local.response.v1".into(),
        binding: binding(role),
        sequence,
        operation,
        request_digest: req.request_digest.clone(),
        outcome: LocalOutcome::Committed,
        code: LocalCode::Ok,
        body: output,
        result_digest: String::new(),
    };
    seal(&mut response);
    Exchange {
        request: req,
        response,
    }
}
fn prepared(plan: &RunPlan) -> PrepareData {
    let digest = plan.digest().unwrap();
    let neural_generation = binding(LocalRole::Neural).generation;
    let body_binding = binding(LocalRole::Body);
    let classification = json!({"class":"exploratory_research","profile":"subset_magnitude_v0_9"});
    let body = exchange(
        LocalRole::Body,
        1,
        LocalOperation::Prepare,
        json!({"plan":plan,"application_profile":"crebain.local-kinematic-kalman.v1","configuration":{"expected_neural_generation":neural_generation}}),
        json!({"application_profile":"crebain.local-kinematic-kalman.v1","plan_digest":digest,"snapshot":source(plan,0)}),
    );
    let descriptor = json!({"test_input":"synthetic contract control"});
    let neural = exchange(
        LocalRole::Neural,
        1,
        LocalOperation::Prepare,
        json!({"plan":plan,"application_profile":"engram.local-nest-rate-controller.v1","configuration":{}}),
        json!({"application_profile":"engram.local-nest-rate-controller.v1","plan_digest":digest,"neural_time_us":0,"calibrated_posterior":false,"network_descriptor":descriptor,"network_digest":local_digest("ncp.local.plan.v1",&descriptor).unwrap()}),
    );
    let monitor = exchange(
        LocalRole::Monitor,
        1,
        LocalOperation::Prepare,
        json!({"plan":plan,"application_profile":"galadriel.scalar-nis-record-only.v1","configuration":{"body_generation":body_binding.generation}}),
        json!({"application_profile":"galadriel.scalar-nis-record-only.v1","plan_digest":digest,"body_binding":body_binding,"classification":classification,"configuration_digest":"a".repeat(64),"calibrated_posterior":false}),
    );
    PrepareData {
        plan: plan.clone(),
        application_profile: APPLICATION_PROFILE.into(),
        configuration: serde_json::to_value(CaptureConfiguration {
            body_preparation: body,
            neural_preparation: neural,
            monitor_preparation: monitor,
            max_capture_bytes: 1_048_576,
        })
        .unwrap(),
    }
}
fn capture(plan: &RunPlan, k: u64) -> CaptureData {
    let previous = source(plan, k - 1);
    let next = source(plan, k);
    let n = plan.entity_ids.len();
    let proposal = NeuralProposal {
        schema: "ncp.local.neural-result.v1".into(),
        plan_digest: plan.digest().unwrap(),
        step: k,
        source_snapshot_digest: previous.snapshot_digest.clone(),
        selected_modes: vec![ActionMode::ZeroAcceleration; n],
        values: vec![0.0; n * 3],
        neural_time_us: plan.time_us(k).unwrap(),
        completed_end_us: plan.time_us(k).unwrap() - plan.readout_delay_us,
        window_start_us: previous.time_us.saturating_sub(plan.readout_delay_us),
        spike_counts: vec![0; n * 6],
        neural_model: "controlled-test-input".into(),
    };
    let neural = exchange(
        LocalRole::Neural,
        k + 1,
        LocalOperation::Step,
        serde_json::to_value(NeuralStep {
            source_snapshot: previous.clone(),
        })
        .unwrap(),
        serde_json::to_value(proposal).unwrap(),
    );
    let result = BodyResult {
        schema: "ncp.local.body-result.v1".into(),
        plan_digest: plan.digest().unwrap(),
        step: k,
        source_snapshot_digest: previous.snapshot_digest.clone(),
        neural_result_digest: neural.response.result_digest.clone(),
        selected_modes: vec![ActionMode::ZeroAcceleration; n],
        proposed_values: vec![0.0; n * 3],
        applied_values: vec![0.0; n * 3],
        saturated: vec![false; n],
        snapshot: next.clone(),
    };
    let body = exchange(
        LocalRole::Body,
        k + 1,
        LocalOperation::Step,
        serde_json::to_value(BodyStep {
            neural_response: neural.response.clone(),
        })
        .unwrap(),
        serde_json::to_value(result).unwrap(),
    );
    let monitor = exchange(
        LocalRole::Monitor,
        k + 1,
        LocalOperation::Assess,
        json!({"body_response":body.response}),
        json!({"schema":"galadriel.local.assessment.v1","application_profile":"galadriel.scalar-nis-record-only.v1","plan_digest":plan.digest().unwrap(),"step":k,"time_us":next.time_us,"body_result_digest":body.response.result_digest,"snapshot_digest":next.snapshot_digest,"classification":{"class":"exploratory_research","profile":"subset_magnitude_v0_9"},"configuration_digest":"a".repeat(64),"calibrated_posterior":false,"tracks":plan.entity_ids.iter().map(|id|json!({"entity_id":id,"source":null,"modality":"visual","dof":3,"status":"not_ready","reason":"unavailable_before_activation","adapter_digest":null,"report":null})).collect::<Vec<_>>()}),
    );
    CaptureData {
        plan_digest: plan.digest().unwrap(),
        step: k,
        source_snapshot: previous,
        neural,
        body,
        monitor,
    }
}
fn finish(plan: &RunPlan) -> FinishData {
    let n = plan.planned_steps;
    let digest = plan.digest().unwrap();
    let input = json!({"plan_digest":digest,"completed_steps":n});
    FinishData {
        plan_digest: digest.clone(),
        completed_steps: n,
        neural_finish: exchange(
            LocalRole::Neural,
            n + 2,
            LocalOperation::Finish,
            input.clone(),
            json!({"plan_digest":digest,"completed_steps":n,"terminal_status":"completed","calibrated_posterior":false}),
        ),
        body_finish: exchange(
            LocalRole::Body,
            n + 2,
            LocalOperation::Finish,
            input.clone(),
            json!({"plan_digest":digest,"planned_steps":n,"completed_steps":n,"snapshot_digest":source(plan,n).snapshot_digest,"cleaned_up":true}),
        ),
        monitor_finish: exchange(
            LocalRole::Monitor,
            n + 2,
            LocalOperation::Finish,
            input,
            json!({"plan_digest":digest,"completed_steps":n,"calibrated_posterior":false}),
        ),
    }
}
fn value<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap()
}
fn reserve(plan: &RunPlan, k: u64) -> Value {
    value(ReserveData {
        plan_digest: plan.digest().unwrap(),
        step: k,
    })
}
fn complete(path: &Path, plan: &RunPlan) {
    let mut backend = CaptureBackend::new(binding(LocalRole::Capture), path).unwrap();
    backend
        .execute(LocalOperation::Prepare, &value(prepared(plan)))
        .unwrap();
    for k in 1..=plan.planned_steps {
        backend
            .execute(LocalOperation::Reserve, &reserve(plan, k))
            .unwrap();
        backend
            .execute(LocalOperation::Capture, &value(capture(plan, k)))
            .unwrap();
    }
    backend
        .execute(LocalOperation::Finish, &value(finish(plan)))
        .unwrap();
}

#[test]
fn one_to_three_entity_complete_journals_reopen_with_exact_terminal() {
    for n in 1..=3 {
        let dir = Dir::new();
        let path = dir.file("journal.jsonl");
        let plan = plan(n, 3);
        complete(&path, &plan);
        let verified = verify_journal(&path).unwrap();
        assert_eq!(verified.captured_steps, 3);
        assert_eq!(verified.store_completion, "complete");
        assert!(verified.execution_plan_complete);
        assert!(!verified.scientific_validation);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let output = Command::new(env!("CARGO_BIN_EXE_prisoma-ncp-local-capture"))
            .arg("--verify")
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            serde_json::from_slice::<Verification>(&output.stdout).unwrap(),
            verified
        );
    }
}

#[test]
fn reservation_quota_and_causal_rejection_preserve_prefix() {
    let dir = Dir::new();
    let path = dir.file("journal.jsonl");
    let plan = plan(2, 2);
    let mut backend = CaptureBackend::new(binding(LocalRole::Capture), &path).unwrap();
    let good = value(prepared(&plan));
    let mut bad = good.clone();
    bad["configuration"]["max_capture_bytes"] = json!(STEP_BYTES);
    assert!(backend.execute(LocalOperation::Prepare, &bad).is_err());
    assert_eq!(fs::metadata(&path).unwrap().len(), 0);
    backend.execute(LocalOperation::Prepare, &good).unwrap();
    let initial = fs::read(&path).unwrap();
    let good = value(capture(&plan, 1));
    assert!(backend.execute(LocalOperation::Capture, &good).is_err());
    assert_eq!(fs::read(&path).unwrap(), initial);
    backend
        .execute(LocalOperation::Reserve, &reserve(&plan, 1))
        .unwrap();
    let reserved = fs::read(&path).unwrap();
    assert!(backend
        .execute(LocalOperation::Reserve, &reserve(&plan, 1))
        .is_err());
    for i in 0..9 {
        let mut bad: CaptureData = serde_json::from_value(good.clone()).unwrap();
        match i {
            0 => bad.step = 2,
            1 => bad.source_snapshot.values[0] = 1.0,
            2 => bad.neural.response.binding.generation = binding(LocalRole::Body).generation,
            3 => {
                bad.neural.response.body["source_snapshot_digest"] = json!("0".repeat(64));
                seal(&mut bad.neural.response);
            }
            4 => {
                bad.body.response.body["neural_result_digest"] = json!("0".repeat(64));
                seal(&mut bad.body.response);
            }
            5 => {
                bad.monitor.response.body["calibrated_posterior"] = json!(true);
                seal(&mut bad.monitor.response);
            }
            6 => {
                bad.monitor.response.body["classification"] = json!({"class":"named_release"});
                seal(&mut bad.monitor.response);
            }
            7 => {
                bad.monitor.response.body["tracks"][0]["source"] = json!({});
                seal(&mut bad.monitor.response);
            }
            _ => {
                bad.monitor.response.body["tracks"][0]["report"] = json!("x".repeat(65_000));
                seal(&mut bad.monitor.response);
            }
        }
        assert!(backend
            .execute(LocalOperation::Capture, &value(bad))
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), reserved);
        assert!(backend.validate(LocalOperation::Capture, &good).is_ok());
    }
    backend.execute(LocalOperation::Capture, &good).unwrap();
    assert!(backend
        .execute(LocalOperation::Finish, &value(finish(&plan)))
        .is_err());
    backend.execute(LocalOperation::Abort, &json!({})).unwrap();
    let report = verify_journal(&path).unwrap();
    assert_eq!(report.store_completion, "aborted");
    assert_eq!(report.captured_steps, 1);
    assert!(!report.execution_plan_complete);
    assert_eq!(report.remaining_suffix, Some("unresolved".into()));
}

fn rechain(records: &mut [JournalRecord]) -> Vec<u8> {
    let mut previous = None;
    let mut output = vec![];
    for (ordinal, row) in records.iter_mut().enumerate() {
        row.ordinal = ordinal as u64;
        row.previous_digest = previous.clone();
        let mut v = value(&*row);
        v.as_object_mut().unwrap().remove("record_digest");
        row.record_digest = local_digest("ncp.local.capture.v1", &v).unwrap();
        previous = Some(row.record_digest.clone());
        output.extend(serde_json::to_vec(row).unwrap());
        output.push(b'\n');
    }
    output
}
#[test]
fn missing_terminal_whole_step_reordered_rows_and_duplicate_keys_fail() {
    let dir = Dir::new();
    let path = dir.file("journal.jsonl");
    let plan = plan(1, 3);
    complete(&path, &plan);
    let original = fs::read(&path).unwrap();
    let records: Vec<JournalRecord> = original
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    let mut mutations = vec![];
    let mut changed = records.clone();
    changed.pop();
    mutations.push(rechain(&mut changed));
    let mut changed = records.clone();
    changed.drain(3..5);
    mutations.push(rechain(&mut changed));
    let mut changed = records.clone();
    changed.swap(1, 2);
    mutations.push(rechain(&mut changed));
    let mut changed = records.clone();
    if let Event::Finish(data) = &mut changed.last_mut().unwrap().event {
        data.completed_steps = 2;
    }
    mutations.push(rechain(&mut changed));
    let changed = String::from_utf8(original.clone()).unwrap().replacen(
        "\"ordinal\":0",
        "\"ordinal\":0,\"ordinal\":0",
        1,
    );
    mutations.push(changed.into_bytes());
    let changed =
        String::from_utf8(original.clone())
            .unwrap()
            .replacen("\"previous_digest\":null,", "", 1);
    mutations.push(changed.into_bytes());
    let mut changed = original.clone();
    changed.pop();
    mutations.push(changed);
    for changed in mutations {
        fs::write(&path, changed).unwrap();
        assert!(verify_journal(&path).is_err());
        fs::write(&path, &original).unwrap();
        assert!(verify_journal(&path).is_ok());
    }
}

#[test]
fn no_replace_no_follow_and_unterminated_eof_remain_explicit() {
    let dir = Dir::new();
    let path = dir.file("journal.jsonl");
    let plan = plan(1, 1);
    let mut backend = CaptureBackend::new(binding(LocalRole::Capture), &path).unwrap();
    assert!(CaptureBackend::new(binding(LocalRole::Capture), &path).is_err());
    let alias = dir.file("alias");
    symlink(&path, &alias).unwrap();
    assert!(CaptureBackend::new(binding(LocalRole::Capture), &alias).is_err());
    backend
        .execute(LocalOperation::Prepare, &value(prepared(&plan)))
        .unwrap();
    backend
        .execute(LocalOperation::Reserve, &reserve(&plan, 1))
        .unwrap();
    backend.retire();
    assert!(verify_journal(&path).is_err());
    assert!(verify_journal(&alias).is_err());
}

#[test]
fn exact_local_replay_and_result_lookup_do_not_append_again() {
    let dir = Dir::new();
    let path = dir.file("journal.jsonl");
    let plan = plan(1, 1);
    let backend = CaptureBackend::new(binding(LocalRole::Capture), &path).unwrap();
    let mut owner = LocalOwner::new(binding(LocalRole::Capture), backend).unwrap();
    let req = request(
        LocalRole::Capture,
        1,
        LocalOperation::Prepare,
        value(prepared(&plan)),
    );
    let raw = serde_json::to_vec(&req).unwrap();
    let result = owner.handle(&raw).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(owner.handle(&raw).unwrap(), result);
    let lookup = request(
        LocalRole::Capture,
        1,
        LocalOperation::Result,
        json!({"request_digest":req.request_digest}),
    );
    assert_eq!(
        owner.handle(&serde_json::to_vec(&lookup).unwrap()).unwrap(),
        result
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let response: LocalResponse = serde_json::from_slice(&result).unwrap();
    let ack = request(
        LocalRole::Capture,
        1,
        LocalOperation::Ack,
        json!({"result_digest":response.result_digest}),
    );
    owner.handle(&serde_json::to_vec(&ack).unwrap()).unwrap();
    let replay: LocalResponse = serde_json::from_slice(&owner.handle(&raw).unwrap()).unwrap();
    assert_eq!(replay.outcome, LocalOutcome::Unavailable);
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn private_stdio_process_commits_header_then_eof_stays_incomplete() {
    let dir = Dir::new();
    let path = dir.file("journal.jsonl");
    let installed = binding(LocalRole::Capture);
    let plan = plan(1, 1);
    let req = request(
        LocalRole::Capture,
        1,
        LocalOperation::Prepare,
        value(prepared(&plan)),
    );
    let bytes = serde_json::to_vec(&req).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_prisoma-ncp-local-capture"))
        .args([
            "--run-id",
            &installed.run_id,
            "--generation",
            &installed.generation,
            "--output-file",
            path.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let input = child.stdin.as_mut().unwrap();
        input
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .unwrap();
        input.write_all(&bytes).unwrap();
        input.flush().unwrap();
    }
    let response = read_local_frame(child.stdout.as_mut().unwrap())
        .unwrap()
        .unwrap();
    let response: LocalResponse = serde_json::from_slice(&response).unwrap();
    response.verify(&installed, &req).unwrap();
    assert_eq!(response.outcome, LocalOutcome::Committed);
    assert!(response.body["durable_record_committed"].as_bool().unwrap());
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert!(verify_journal(&path).is_err());
}

#[test]
fn prepared_peer_and_application_drift_reject_before_header_append() {
    let dir = Dir::new();
    let path = dir.file("journal.jsonl");
    let plan = plan(1, 1);
    let good = prepared(&plan);
    let mut backend = CaptureBackend::new(binding(LocalRole::Capture), &path).unwrap();
    for mutation in 0..7 {
        let mut bad = good.clone();
        let mut config: CaptureConfiguration =
            serde_json::from_value(bad.configuration.clone()).unwrap();
        match mutation {
            0 => {
                config.body_preparation.response.binding.generation =
                    binding(LocalRole::Capture).generation
            }
            1 => config.neural_preparation.response.binding.role = LocalRole::Body,
            2 => {
                config.monitor_preparation.request.body["configuration"]["body_generation"] =
                    json!(binding(LocalRole::Neural).generation);
                config.monitor_preparation.request.seal().unwrap();
                config.monitor_preparation.response.request_digest =
                    config.monitor_preparation.request.request_digest.clone();
                seal(&mut config.monitor_preparation.response);
            }
            3 => {
                config.monitor_preparation.response.body["calibrated_posterior"] = json!(true);
                seal(&mut config.monitor_preparation.response);
            }
            4 => {
                config.neural_preparation.response.body["network_digest"] = json!("0".repeat(64));
                seal(&mut config.neural_preparation.response);
            }
            5 => {
                config.body_preparation.request.body["plan"]["seed"] = json!(43);
                config.body_preparation.request.seal().unwrap();
                config.body_preparation.response.request_digest =
                    config.body_preparation.request.request_digest.clone();
                seal(&mut config.body_preparation.response);
            }
            _ => bad.application_profile = "legacy-wire-observer".into(),
        }
        bad.configuration = value(config);
        assert!(backend
            .execute(LocalOperation::Prepare, &value(bad))
            .is_err());
        assert_eq!(fs::metadata(&path).unwrap().len(), 0);
        assert!(backend
            .validate(LocalOperation::Prepare, &value(&good))
            .is_ok());
    }
    backend
        .execute(LocalOperation::Prepare, &value(good))
        .unwrap();
    backend.execute(LocalOperation::Abort, &json!({})).unwrap();
    assert_eq!(verify_journal(&path).unwrap().captured_steps, 0);
}

#[test]
fn terminal_cleanup_and_ownership_fail_without_losing_complete_prefix() {
    let dir = Dir::new();
    let path = dir.file("journal.jsonl");
    let plan = plan(1, 1);
    let mut backend = CaptureBackend::new(binding(LocalRole::Capture), &path).unwrap();
    backend
        .execute(LocalOperation::Prepare, &value(prepared(&plan)))
        .unwrap();
    backend
        .execute(LocalOperation::Reserve, &reserve(&plan, 1))
        .unwrap();
    backend
        .execute(LocalOperation::Capture, &value(capture(&plan, 1)))
        .unwrap();
    let prefix = fs::read(&path).unwrap();
    let good = finish(&plan);
    for mutation in 0..4 {
        let mut bad = good.clone();
        match mutation {
            0 => {
                bad.body_finish.response.body["cleaned_up"] = json!(false);
                seal(&mut bad.body_finish.response);
            }
            1 => {
                bad.body_finish.response.body["snapshot_digest"] = json!("0".repeat(64));
                seal(&mut bad.body_finish.response);
            }
            2 => {
                bad.neural_finish.response.body["terminal_status"] = json!("aborted");
                seal(&mut bad.neural_finish.response);
            }
            _ => {
                bad.monitor_finish.response.binding.generation =
                    binding(LocalRole::Neural).generation;
                seal(&mut bad.monitor_finish.response);
            }
        }
        assert!(backend
            .execute(LocalOperation::Finish, &value(bad))
            .is_err());
        assert_eq!(fs::read(&path).unwrap(), prefix);
        assert!(backend
            .validate(LocalOperation::Finish, &value(&good))
            .is_ok());
    }
    backend
        .execute(LocalOperation::Finish, &value(good))
        .unwrap();
    assert!(verify_journal(&path).unwrap().execution_plan_complete);
}

#[test]
fn reserved_zero_step_abort_keeps_suffix_unresolved() {
    let dir = Dir::new();
    let path = dir.file("journal.jsonl");
    let plan = plan(1, 1);
    let mut backend = CaptureBackend::new(binding(LocalRole::Capture), &path).unwrap();
    backend
        .execute(LocalOperation::Prepare, &value(prepared(&plan)))
        .unwrap();
    backend
        .execute(LocalOperation::Reserve, &reserve(&plan, 1))
        .unwrap();
    let result = backend.execute(LocalOperation::Abort, &json!({})).unwrap();
    assert_eq!(result["captured_steps"], 0);
    assert_eq!(result["remaining_suffix"], "unresolved");
    let report = verify_journal(&path).unwrap();
    assert_eq!(report.store_completion, "aborted");
    assert!(!report.execution_plan_complete);
}
