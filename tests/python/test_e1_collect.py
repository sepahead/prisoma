"""Collector controls use synthetic family bytes, never a native scientific result."""

from __future__ import annotations

import base64
from dataclasses import replace
import hashlib
import json
from pathlib import Path
import struct
import uuid
from unittest.mock import patch

import pytest

pytest.importorskip("crebain_ncp_sensors.family_contract")
pytest.importorskip("prisoma_agent_bridge.crebain_family")
from crebain_ncp_sensors import (
    codec as c,
    family_contract as fc,
    family_types as f,
    types as t,
)
from prisoma_agent_bridge import crebain_family as family
from experiments import e1_reference as ref
from experiments import e1_collect as collect


ROOT = Path(ref.__file__).resolve().parents[1]


def case(episode_id="e1-train-000"):
    seed = next(
        row for row in ref.seed_roster()["episodes"] if row["episode_id"] == episode_id
    )
    raw = json.loads(
        (
            ROOT / "integrations/agent-bridge/tests/fixtures/m1.workload.v1.json"
        ).read_bytes()
    )["specification"]
    raw["seed"] = seed["seeds"]["runtime"]
    raw["acoustic"]["seed"] = seed["seeds"]["acoustic"]
    raw["drones"][0]["position"] = seed["explicit_position_m"]
    camera = raw["scene"]["rgbCameras"][0]
    camera.update(width=160, height=120, periodTicks=3)
    raw["scene"]["rgbCameras"] = [
        camera,
        {**camera, "id": "rgb-b", "position": [5, 9, 0]},
    ]
    raw["scene"]["thermalCameras"] = []
    targets = tuple(
        t.SetTarget("set_target", True, roll, pitch, 0.0, 8.0)
        for roll, pitch in ref.ANGLES
    )
    plan = f.FamilyPlan(
        str(uuid.uuid4()),
        fc.new_binding(),
        c.decode(
            "Prepare",
            {
                "specification": raw,
                "planned_ticks": 36,
                "composition_digest": fc.COMPOSITION_DIGEST,
            },
        ),
        12,
        tuple(
            f.BranchPlan(i + 1, name, "label", fc.new_binding(), target)
            for i, (name, target) in enumerate(zip(ref.ACTIONS, targets, strict=True))
        ),
        f.PressureWindow(
            "scaled_compensated_pressure_rms400_v1",
            "pressure:mic-a",
            34,
            36,
            400,
            "pascal",
            fc.TARGET_DIGEST,
        ),
        f.FamilyLimits(120, 6, 2, 1, 1, 3200),
    )
    return collect.StudyCase(episode_id, plan)


def test_exact_study_and_seed_admission():
    selected = case()
    admitted, row, cameras, microphone = collect.validate_case(selected)
    assert admitted == selected and row["split"] == "train"
    assert cameras == ("rgb:rgb-a", "rgb:rgb-b") and microphone == "pressure:mic-a"
    for bad in (
        replace(selected, episode_id="e1-qualification-000"),
        replace(selected, plan=replace(selected.plan, landmark_tick=9)),
        replace(
            selected,
            plan=replace(
                selected.plan,
                body=replace(
                    selected.plan.body,
                    specification=replace(selected.plan.body.specification, seed=4),
                ),
            ),
        ),
        replace(
            selected,
            plan=replace(
                selected.plan,
                branches=tuple(
                    replace(branch, target=selected.plan.branches[0].target)
                    for branch in selected.plan.branches
                ),
            ),
        ),
    ):
        with pytest.raises((ValueError, AssertionError)):
            collect.validate_case(bad)


def test_pre_effect_stage_and_scene_rejections(tmp_path):
    selected = case()
    with patch.object(family, "owned_family_experiment") as owner:
        with pytest.raises(collect.CollectionError, match="training_order"):
            collect.collect_episode(
                None, selected, tmp_path / "bad", references=(object(),)
            )
        assert not (tmp_path / "bad").exists()
        bad_camera = replace(
            selected.plan.body.specification.scene.rgbCameras[0], periodTicks=2
        )
        scene = replace(
            selected.plan.body.specification.scene,
            rgbCameras=(
                bad_camera,
                selected.plan.body.specification.scene.rgbCameras[1],
            ),
        )
        bad = replace(
            selected,
            plan=replace(
                selected.plan,
                body=replace(
                    selected.plan.body,
                    specification=replace(
                        selected.plan.body.specification, scene=scene
                    ),
                ),
            ),
        )
        with pytest.raises(collect.CollectionError, match="study_camera"):
            collect.collect_episode(None, bad, tmp_path / "bad")
        owner.assert_not_called()
        assert not (tmp_path / "bad").exists()


def test_full_roster_admission_keeps_qualification_separate(tmp_path):
    with patch.object(family, "owned_family_experiment") as owner:
        for selected in ((case(),), (case("e1-qualification-000"),)):
            with pytest.raises(collect.CollectionError, match="study_roster"):
                collect.collect_study(
                    None,
                    selected,
                    tmp_path / "study",
                    source_sha256="a" * 64,
                    runtime_inventory_sha256="b" * 64,
                )
        owner.assert_not_called()
    assert not (tmp_path / "study").exists()


def _synthetic_episode(stage, number):
    observation = ref.Landmark(
        (number / 100, 0.2, 0.3, 0.4, 0.5, 0.6, 0.1, 2.0),
        hashlib.sha256(str((stage, number)).encode()).hexdigest(),
    )
    return ref.Episode(
        f"e1-{stage}-{number:03d}",
        observation,
        tuple(5 + number / 100 + 30 * roll + 20 * pitch for roll, pitch in ref.ANGLES),
        "b" * 64,
    )


@pytest.fixture(scope="module")
def grid():
    return ref.fit_reference(
        tuple(_synthetic_episode("train", i) for i in range(64)),
        source_sha256="a" * 64,
        runtime_inventory_sha256="b" * 64,
    )


@pytest.fixture(scope="module")
def selection(grid):
    episodes = tuple(_synthetic_episode("development", i) for i in range(16))
    evaluations = tuple(
        ref.evaluate_episode(
            ref.commit_collection("development", ep.episode_id, ep.observation, grid),
            ep,
        )
        for ep in episodes
    )
    return ref.choose_lambda(grid, evaluations)


def test_stage_forecasts_and_neutral_collection(grid, selection):
    landmark = _synthetic_episode("train", 0).observation
    for stage, models, chosen in (
        ("train", (), None),
        ("development", grid, None),
        ("heldout", (), selection),
    ):
        row = collect._row(f"e1-{stage}-000")
        collect._stage_inputs(row, models, chosen)
        committed = collect._commit(row, landmark, models, chosen)
        assert (
            ref.load_commitment(
                ref.canonical(committed.record()), expected_sha256=committed.sha256
            )
            == committed
        )
        assert (
            len(committed.forecasts)
            == {"train": 1, "development": 6, "heldout": 3}[stage]
        )
        assert committed.selected_action == (
            "neutral" if stage != "heldout" else committed.forecasts[0].recommendation
        )
    with pytest.raises(collect.CollectionError, match="development_order"):
        collect._stage_inputs(collect._row("e1-development-000"), grid, selection)
    with pytest.raises(collect.CollectionError, match="holdout_lock"):
        collect._stage_inputs(collect._row("e1-heldout-000"), grid, selection)


def _training_result(selected):
    episode = _synthetic_episode("train", int(selected.episode_id[-3:]))
    committed = ref.commit_collection("train", selected.episode_id, episode.observation)
    return collect.CollectedEpisode(ref.evaluate_episode(committed, episode), {})


def test_study_stage_publication_order_and_failure_denominator(tmp_path):
    cases = tuple(
        case(row["episode_id"])
        for row in ref.seed_roster()["episodes"]
        if row["split"] != "qualification"
    )
    out = tmp_path / "study"
    failure = RuntimeError("controlled first development failure")
    seen = []

    def episode(runtime, selected, output, *, references, selection):
        seen.append(selected.episode_id)
        if len(seen) <= 64:
            assert references == () and selection is None
            assert not (out / "reference-0.json").exists()
            return _training_result(selected)
        assert len(references) == 4 and selection is None
        assert all(
            (out / f"reference-{index}.json").read_bytes()
            == ref.canonical(model.record())
            for index, model in enumerate(references)
        )
        raise failure

    with (
        patch.object(collect, "collect_episode", episode),
        pytest.raises(RuntimeError) as caught,
    ):
        collect.collect_study(
            None, cases, out, source_sha256="a" * 64, runtime_inventory_sha256="b" * 64
        )
    assert caught.value is failure
    record = json.loads((out / "failure.json").read_bytes())
    assert len(record["completed"]) == 64 and len(record["uncompleted"]) == 48
    assert record["uncompleted"][0] == "e1-development-000"
    assert not (out / "forecast-report.json").exists()
    assert not (out / "selection.json").exists()


@pytest.fixture
def synthetic_family(monkeypatch):
    """Use the maintained adapter's actual SDK/socket synthetic producer control."""
    import importlib.util
    import os

    support_path = ROOT / "integrations/agent-bridge/tests/family_support.py"
    if not all(
        os.environ.get(name)
        for name in (
            "CREBAIN_FAMILY_PRODUCER",
            "CREBAIN_SENSOR_BUN",
            "CREBAIN_SENSOR_NODE",
            "CREBAIN_SENSOR_BRIDGE",
        )
    ):
        pytest.skip("selected synthetic family producer is not configured")
    spec = importlib.util.spec_from_file_location(
        "e1_collector_family_support", support_path
    )
    support = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(support)
    monkeypatch.setattr(family, "family_session", support.synthetic_owner)
    return support


def test_actual_synthetic_adapter_capture_and_late_terminal_rejection(
    tmp_path, synthetic_family
):
    selected = case()
    output = tmp_path / "episode"
    result = collect.collect_episode(None, selected, output)
    assert result.evaluation.episode.episode_id == selected.episode_id
    receipt = result.receipt
    assert receipt["verification"]["family_replayed"] is True
    assert (
        receipt["policy_comparison"]["selected_restored_original_bytes_equal"] is True
    )
    assert receipt["policy_comparison"]["neutral_minus_selected_pa"] == 0
    assert receipt["authority"]["scientific_validation"] is False
    assert receipt["finish"]["process_exit"]["cleanup_confirmed"] is True
    original = base64.b64decode(receipt["original_forecast_base64"])
    assert (
        hashlib.sha256(original).hexdigest()
        == receipt["decision"]["forecast_commitment_digest"]
    )
    assert collect.readback_episode(selected, output / "run.jsonl") == result
    log = output / "run.jsonl"
    data = log.read_bytes()
    log.write_bytes(b"".join(data.splitlines(keepends=True)[:-1]))
    with pytest.raises(Exception):
        collect.readback_episode(selected, log)
    log.write_bytes(data)
    assert collect.readback_episode(selected, log) == result


def test_original_pressure_window_and_selected_bytes_reject_rehashed_foreign_labels(
    tmp_path, synthetic_family
):
    selected = case()
    output = tmp_path / "episode"
    result = collect.collect_episode(None, selected, output)
    admitted, row, cameras, microphone = collect.validate_case(selected)
    state = collect._Readback(admitted, row, cameras, microphone, (), None)
    verification = family.inspect_family_run(output / "run.jsonl", state.visit)
    assert state.accepted(verification) == result
    label = state.labels[0]
    with pytest.raises(collect.CollectionError, match="original_label_window"):
        state._label(1, replace(label, window_payload_sha256="a" * 64))
    with pytest.raises(collect.CollectionError, match="original_label_window"):
        state._label(1, replace(label, value_pa=label.value_pa + 1))
    foreign = replace(
        state.windows[1].readings[0], payload=struct.pack("<133d", *([123.0] * 133))
    )
    state.windows[1].readings[0] = foreign
    with pytest.raises(collect.CollectionError, match="selected_original_bytes"):
        state.accepted(verification)


def test_no_derived_publication_when_complete_readback_fails(
    tmp_path, synthetic_family
):
    output = tmp_path / "episode"
    failure = RuntimeError("late original terminal failure")
    original = family.inspect_family_run

    def fail_after_visits(path, visitor):
        original(path, visitor)
        raise failure

    with (
        patch.object(family, "inspect_family_run", fail_after_visits),
        pytest.raises(RuntimeError) as caught,
    ):
        collect.collect_episode(None, case(), output)
    assert caught.value is failure
    assert (output / "run.jsonl").exists()
    assert not (output / "episode.json").exists()


def test_complete_study_freezes_selection_before_heldout(tmp_path):
    cases = tuple(
        case(row["episode_id"])
        for row in ref.seed_roster()["episodes"]
        if row["split"] != "qualification"
    )
    out = tmp_path / "study"
    seen = []

    def episode(runtime, selected, output, *, references, selection):
        stage = collect._row(selected.episode_id)["split"]
        item = _synthetic_episode(stage, int(selected.episode_id[-3:]))
        if stage == "train":
            assert references == () and selection is None
            commitment = ref.commit_collection(stage, item.episode_id, item.observation)
        elif stage == "development":
            assert selection is None and len(references) == 4
            assert not (out / "selection.json").exists()
            commitment = ref.commit_collection(
                stage, item.episode_id, item.observation, references
            )
            assert all(
                (out / f"reference-{i}.json").read_bytes()
                == ref.canonical(model.record())
                for i, model in enumerate(references)
            )
        else:
            assert references == () and selection is not None
            assert (out / "selection.json").read_bytes() == ref.canonical(
                selection.record()
            )
            commitment = ref.commit_selected(
                selection, item.episode_id, item.observation
            )
        seen.append((stage, commitment.selected_action))
        return collect.CollectedEpisode(ref.evaluate_episode(commitment, item), {})

    with patch.object(collect, "collect_episode", episode):
        report = collect.collect_study(
            None, cases, out, source_sha256="a" * 64, runtime_inventory_sha256="b" * 64
        )
    assert len(seen) == 112
    assert all(action == "neutral" for _, action in seen[:80])
    assert any(action != "neutral" for _, action in seen[80:])
    assert report["episodes"] == 32 and report["action_rows"] == 160
    assert report["bootstrap_indices_sha256"] == ref.BOOTSTRAP_INDICES_SHA256
    assert not (out / "failure.json").exists()
    assert report["authority"]["scientific_validation"] is False


def test_derived_write_preserves_primary_and_close_failure(tmp_path):
    primary, secondary = OSError("write"), OSError("close")

    class Broken:
        def write(self, payload):
            raise primary

        def close(self):
            raise secondary

    with (
        patch.object(Path, "open", return_value=Broken()),
        pytest.raises(BaseExceptionGroup) as caught,
    ):
        collect._publish(tmp_path / "result.json", {"key": "value"})
    assert caught.value.exceptions == (primary, secondary)


@pytest.mark.parametrize("stage", ["development", "heldout"])
def test_actual_stage_commitments_and_selected_continuation(
    tmp_path, synthetic_family, grid, selection, stage
):
    selected = case(f"e1-{stage}-000")
    references = grid if stage == "development" else ()
    chosen = selection if stage == "heldout" else None
    output = tmp_path / "episode"
    result = collect.collect_episode(
        None, selected, output, references=references, selection=chosen
    )
    commitment = result.evaluation.commitment
    assert len(commitment.forecasts) == (6 if stage == "development" else 3)
    assert commitment.selected_action == (
        "neutral" if stage == "development" else commitment.forecasts[0].recommendation
    )
    assert (
        result.receipt["decision"]["selected_case_id"]
        == selected.plan.branches[ref.ACTIONS.index(commitment.selected_action)].case_id
    )
    assert (
        result.receipt["policy_comparison"]["selected_restored_original_bytes_equal"]
        is True
    )
    original = family.inspect_family_run

    def changed_forecast(path, visitor):
        def visit(event):
            if event.method.endswith(".commit_decision"):
                payload = dict(event.payload)
                payload["forecast_base64"] = base64.b64encode(
                    base64.b64decode(payload["forecast_base64"]) + b"\n"
                ).decode()
                event = replace(event, payload=payload)
            visitor(event)

        return original(path, visit)

    with (
        patch.object(family, "inspect_family_run", changed_forecast),
        pytest.raises(collect.CollectionError, match="original_forecast"),
    ):
        collect.readback_episode(
            selected, output / "run.jsonl", references=references, selection=chosen
        )
    assert (
        collect.readback_episode(
            selected, output / "run.jsonl", references=references, selection=chosen
        )
        == result
    )


def test_exact_qualification_roster_has_no_study_authority(tmp_path):
    for index in range(8):
        selected = case(f"e1-qualification-{index:03d}")
        admitted, row, _, _ = collect.validate_case(selected, qualification=True)
        assert admitted == selected and row["split"] == "qualification"
        with pytest.raises(collect.CollectionError, match="study_split"):
            collect.validate_case(selected)
    with patch.object(family, "owned_family_experiment") as owner:
        with pytest.raises(collect.CollectionError, match="qualification_split"):
            collect.collect_qualification_episode(None, case(), tmp_path / "bad")
        owner.assert_not_called()
    assert not (tmp_path / "bad").exists()


def test_actual_qualification_is_neutral_and_never_model_scored(
    tmp_path, synthetic_family
):
    selected = case("e1-qualification-000")
    output = tmp_path / "qualification"
    with (
        patch.object(ref, "evaluate_episode") as score,
        patch.object(ref, "fit_reference") as fit,
    ):
        result = collect.collect_qualification_episode(None, selected, output)
        assert type(result) is collect.CollectedQualification
        assert (
            collect.readback_qualification_episode(selected, output / "run.jsonl")
            == result
        )
        score.assert_not_called()
        fit.assert_not_called()
    commitment = json.loads(
        base64.b64decode(result.receipt["original_forecast_base64"])
    )
    assert commitment["schema"] == "prisoma.e1-qualification-commitment.v1"
    assert (
        commitment["stage"] == "qualification"
        and commitment["selected_action"] == "neutral"
    )
    assert commitment["forecasts"][0]["predictor_id"] == "persistence"
    assert result.receipt["policy_comparison"]["neutral_minus_selected_pa"] == 0
    assert len(result.receipt["labels"]) == 5
    assert "evaluation" not in json.loads((output / "episode.json").read_bytes())
    with pytest.raises(collect.CollectionError, match="study_split"):
        collect.readback_episode(selected, output / "run.jsonl")
