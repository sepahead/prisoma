"""Provider-free controls. No learned model, Torch import, or download is required."""

import io
from types import SimpleNamespace
from zipfile import ZipFile

import numpy as np
import pytest

from experiments.lewm.contracts import (
    ObservationPair,
    StandardizedCandidates,
    array_save,
    load_arrays,
    pid_handoff,
    verify_forecast,
)
from experiments.lewm.assets import read_json, save_json, sha
from experiments.lewm import assets, contracts, model


def candidates():
    return StandardizedCandidates(
        ("left", "right"), np.zeros((1, 2, 5, 10), dtype=np.float32)
    )


def test_candidate_input_snapshot_is_immutable():
    source = np.zeros((1, 2, 5, 10), dtype=np.float32)
    value = StandardizedCandidates(("left", "right"), source)
    source[:] = 9
    assert np.all(value.values == 0)
    with pytest.raises(ValueError):
        value.values.setflags(write=True)


@pytest.mark.parametrize("changed", [0.5, np.nan])
def test_candidate_validates_bytes_captured_during_copy(monkeypatch, changed):
    source = np.zeros((1, 2, 5, 10), dtype=np.float32)
    original_view = memoryview

    class ConcurrentWriterView:
        def __init__(self, array):
            self.view = original_view(array)
            self.format, self.shape = self.view.format, self.view.shape

        def tobytes(self):
            source.flat[0] = changed
            return self.view.tobytes()

    monkeypatch.setattr(contracts, "memoryview", ConcurrentWriterView, raising=False)
    if np.isnan(changed):
        with pytest.raises(ValueError, match="numerical admission"):
            StandardizedCandidates(("left", "right"), source)
    else:
        retained = StandardizedCandidates(("left", "right"), source)
        assert retained.values.flat[0] == changed


@pytest.mark.parametrize("kind", ["observation", "candidates"])
def test_array_subclass_cannot_replace_copied_bytes(kind):
    calls = []

    class ReplacingArray(np.ndarray):
        def tobytes(self, *args, **kwargs):
            calls.append(True)
            return b"replacement bytes"

    if kind == "observation":
        value = np.zeros((224, 224, 3), dtype=np.uint8).view(ReplacingArray)
        with pytest.raises(ValueError, match="exact ndarray"):
            ObservationPair(value, value)
    else:
        value = np.zeros((1, 2, 5, 10), dtype=np.float32).view(ReplacingArray)
        with pytest.raises(ValueError, match="exact ndarray"):
            StandardizedCandidates(("left", "right"), value)
    assert calls == []


def test_strided_exact_arrays_preserve_observation_and_candidate_order():
    pixels = np.arange(224 * 224 * 3, dtype=np.uint8).reshape(224, 224, 3)[::-1]
    observation = ObservationPair(pixels, pixels)
    assert np.array_equal(observation.current, pixels)
    values = np.arange(100, dtype=np.float32).reshape(1, 2, 5, 10)[:, ::-1]
    retained = StandardizedCandidates(("right", "left"), values)
    assert np.array_equal(retained.values, values)


def test_prepared_owner_identity_cannot_change_through_inspection(
    monkeypatch, tmp_path
):
    identity = {"arm": "repository_jepa", "checkpoint_sha256": "a" * 64}
    monkeypatch.setenv("PYTORCH_ENABLE_MPS_FALLBACK", "0")
    monkeypatch.setattr(assets, "verify_runtime", lambda: {})
    monkeypatch.setattr(model, "source_identity", lambda *_: identity)
    monkeypatch.setattr(model, "owners", lambda *_: {})
    monkeypatch.setattr(
        model, "construct", lambda *_: SimpleNamespace(to=lambda _: None)
    )
    monkeypatch.setattr(contracts, "_forecast_candidates", lambda *args: args[3])
    prepared = model.PreparedLeWM(tmp_path, "repository_jepa", "cpu")
    expected = prepared.source
    identity["checkpoint_sha256"] = "b" * 64
    inspection = prepared.source
    inspection["checkpoint_sha256"] = "c" * 64
    with pytest.raises(AttributeError):
        prepared.source = inspection
    committed = prepared.forecast(None, None, tmp_path / "output")
    assert committed == expected
    committed["checkpoint_sha256"] = "d" * 64
    assert prepared.source == expected


@pytest.mark.parametrize(
    "ids",
    [
        ("one",),
        ("one", "one"),
        ("path/escape", "right"),
        tuple(str(i) for i in range(301)),
    ],
)
def test_candidate_roster_rejects(ids):
    with pytest.raises(ValueError):
        StandardizedCandidates(ids, np.zeros((1, len(ids), 5, 10), dtype=np.float32))


@pytest.mark.parametrize("value", [np.nan, np.inf, 1_000_001])
def test_candidate_numerics_reject(value):
    array = candidates().values.copy()
    array[0, 0, 0, 0] = value
    with pytest.raises(ValueError):
        StandardizedCandidates(("left", "right"), array)


def test_candidate_shape_and_type_reject():
    with pytest.raises(ValueError):
        StandardizedCandidates(("left", "right"), np.zeros((1, 2, 5, 10)))
    with pytest.raises(ValueError):
        StandardizedCandidates(
            ("left", "right"), np.zeros((1, 2, 10, 5), dtype=np.float32)
        )


def test_raw_execution_has_no_implicit_scaler():
    with pytest.raises(ValueError, match="source-bound scaler"):
        candidates().raw_commands()


def test_observation_has_an_immutable_pixel_snapshot():
    pixels = np.zeros((224, 224, 3), dtype=np.uint8)
    value = ObservationPair(pixels, pixels)
    pixels[:] = 255
    assert np.all(value.current == 0)
    with pytest.raises(ValueError):
        value.goal.setflags(write=True)


@pytest.mark.parametrize(
    "array", [np.zeros((224, 224, 3)), np.zeros((3, 224, 224), dtype=np.uint8)]
)
def test_observation_rejects_dtype_or_axis_substitution(array):
    with pytest.raises(ValueError):
        ObservationPair(array, array)


def test_bounded_array_roundtrip_and_no_clobber(tmp_path):
    path = tmp_path / "arrays.npz"
    values = np.arange(12, dtype=np.float32).reshape(3, 4)
    record = array_save(path, values=values)
    assert record["bytes"] == path.stat().st_size
    assert np.array_equal(load_arrays(path)["values"], values)
    with pytest.raises(FileExistsError):
        array_save(path, values=values)


@pytest.mark.parametrize(
    "dtype,shape", [("<f4", (10**12,)), ("|O", (1,)), ("<f4", (1,) * 7)]
)
def test_npy_header_rejects_before_allocation(tmp_path, dtype, shape):
    buffer = io.BytesIO()
    np.lib.format.write_array_header_1_0(
        buffer, {"descr": dtype, "fortran_order": False, "shape": shape}
    )
    path = tmp_path / "bomb.npz"
    with ZipFile(path, "w") as archive:
        archive.writestr("values.npy", buffer.getvalue())
    with pytest.raises(ValueError):
        load_arrays(path)


def handoff(**changes):
    values = dict(
        candidate_digest="a" * 64,
        target_family="reference_state_outcome",
        baseline_candidate_digest="a" * 64,
        maximum_ancestor_step=2,
        prediction_landmark_step=2,
        target_available_step=3,
    )
    values.update(changes)
    return pid_handoff(**values)


def test_downstream_handoff_is_structural_and_requests_no_estimate():
    result = handoff()
    assert result["status"] == "structural_check_only"
    assert result["estimator_requested"] is False
    assert result["language_source"] == "absent"
    assert "estimate" not in result


@pytest.mark.parametrize(
    "changes",
    [
        {"target_family": "same_candidate_action"},
        {"baseline_candidate_digest": "b" * 64},
        {"baseline_candidate_digest": None},
        {"maximum_ancestor_step": 3},
        {"target_available_step": 2},
        {"prediction_landmark_step": True},
        {"target_family": "undefined"},
    ],
)
def test_pid_target_injection_or_unmatched_ancestry_rejects(changes):
    with pytest.raises(ValueError):
        handoff(**changes)


@pytest.fixture
def forecast_bundle(tmp_path):
    """Independent synthetic encoder arithmetic, never a learned-model result."""
    pixels = np.zeros((224, 224, 3), dtype=np.uint8)
    choices = candidates()
    inputs = array_save(
        tmp_path / "inputs.npz",
        current=pixels,
        goal=pixels,
        standardized_actions=choices.values,
    )
    save_json(
        tmp_path / "input-commitment.json",
        {
            "schema": "prisoma.lewm.candidates.v1",
            "source": {"scope": "synthetic_reader_control"},
            "candidate_ids": choices.ids,
            "artifact": inputs,
            "coordinates": "standardized_5x5x2",
            "raw_support": "unknown",
            "reference_outcome_access": False,
            "execution_authority": False,
        },
    )
    forecast = np.ones((1, 2, 6, 192), dtype=np.float32)
    forecast[:, 1] *= 2
    arrays = array_save(
        tmp_path / "forecasts.npz",
        forecast=forecast,
        costs=np.array([[192, 768]], dtype=np.float32),
        goal_embedding=np.zeros((1, 1, 192), dtype=np.float32),
    )
    save_json(
        tmp_path / "forecast-commitment.json",
        {
            "schema": "prisoma.lewm.forecasts.v1",
            "input_commitment_sha256": sha(tmp_path / "input-commitment.json"),
            "artifact": arrays,
            "selected_candidate": "left",
            "selection": "minimum_latent_squared_error_first_index_on_tie",
            "meaning": "model_objective_not_reference_outcome",
            "raw_actions_executed": False,
        },
    )
    return tmp_path


def test_independent_forecast_bundle_is_verified(forecast_bundle):
    assert verify_forecast(forecast_bundle)["selected_candidate"] == "left"


@pytest.mark.parametrize(
    "attack",
    ["candidate_bytes", "candidate_ids", "selection", "authority", "forecast_bytes"],
)
def test_mutation_after_forecast_commitment_rejects(forecast_bundle, attack):
    root = forecast_bundle
    if attack in ("candidate_bytes", "forecast_bytes"):
        name = "inputs.npz" if attack == "candidate_bytes" else "forecasts.npz"
        data = load_arrays(root / name)
        key = "standardized_actions" if attack == "candidate_bytes" else "forecast"
        data[key].flat[0] += 1
        (root / name).unlink()
        array_save(root / name, **data)
    else:
        name = (
            "input-commitment.json"
            if attack == "candidate_ids"
            else "forecast-commitment.json"
        )
        data = read_json(root / name)
        if attack == "candidate_ids":
            data["candidate_ids"].reverse()
        elif attack == "selection":
            data["selected_candidate"] = "right"
        else:
            data["raw_actions_executed"] = True
        (root / name).unlink()
        save_json(root / name, data)
    with pytest.raises(ValueError):
        verify_forecast(root)


def test_reassigned_candidate_names_change_matched_baseline_identity(forecast_bundle):
    path = forecast_bundle / "input-commitment.json"
    original = sha(path)
    record = read_json(path)
    record["candidate_ids"].reverse()
    path.unlink()
    save_json(path, record)
    renamed = sha(path)
    assert renamed != original
    with pytest.raises(ValueError, match="exact same candidate"):
        handoff(candidate_digest=renamed, baseline_candidate_digest=original)
