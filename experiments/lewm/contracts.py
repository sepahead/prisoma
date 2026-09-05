"""Reusable observation, standardized-candidate, forecast, and PID handoff contracts."""

from __future__ import annotations

from dataclasses import dataclass
from io import BytesIO
import math
from pathlib import Path
import re
from zipfile import ZipFile

import numpy as np

from .assets import read_bytes, read_json, save_json, sha


def load_arrays(path: Path) -> dict[str, np.ndarray]:
    """Bound ZIP contents and NPY header shapes before NumPy array allocation."""
    data = read_bytes(path, 4 * 1024 * 1024)
    with ZipFile(BytesIO(data)) as archive:
        records = archive.infolist()
        if not 1 <= len(records) <= 16 or len({r.filename for r in records}) != len(
            records
        ):
            raise ValueError("Invalid NPZ member roster")
        if sum(r.file_size for r in records) > 4 * 1024 * 1024:
            raise ValueError("Decoded NPZ budget exceeded")
        for record in records:
            if re.fullmatch(r"[a-z_]+\.npy", record.filename) is None:
                raise ValueError("Invalid NPZ member name")
            with archive.open(record) as member:
                version = np.lib.format.read_magic(member)
                readers = {
                    (1, 0): np.lib.format.read_array_header_1_0,
                    (2, 0): np.lib.format.read_array_header_2_0,
                }
                if version not in readers:
                    raise ValueError("Unsupported NPY header version")
                shape, _, dtype = readers[version](member, max_header_size=4096)
                if dtype.hasobject or dtype.kind not in "iuf" or len(shape) > 6:
                    raise ValueError("Invalid NPY type or rank")
                if math.prod(shape) * dtype.itemsize > 4 * 1024 * 1024:
                    raise ValueError("NPY declared allocation exceeds budget")
    with np.load(BytesIO(data), allow_pickle=False) as arrays:
        return dict(arrays)


def array_save(path: Path, **arrays) -> dict:
    import os

    total = 0
    for array in arrays.values():
        total += array.nbytes
        if array.dtype.hasobject or not np.isfinite(array).all():
            raise ValueError("Nonfinite or object array")
    if total > 4 * 1024 * 1024:
        raise ValueError("Array output budget exceeded")
    with path.open("xb") as stream:
        np.savez_compressed(stream, **arrays)
        stream.flush()
        os.fsync(stream.fileno())
    return {"path": path.name, "sha256": sha(path), "bytes": path.stat().st_size}


@dataclass(frozen=True)
class ObservationPair:
    """Two immutable RGB images; neither value is an environment checkpoint."""

    current: np.ndarray
    goal: np.ndarray

    def __post_init__(self):
        for name in ("current", "goal"):
            value = getattr(self, name)
            if type(value) is not np.ndarray:
                raise ValueError("Observation requires an exact ndarray")
            view = memoryview(value)
            if view.format != "B" or view.shape != (224, 224, 3):
                raise ValueError("Observation requires uint8 RGB 224×224")
            immutable = np.frombuffer(view.tobytes(), dtype=np.uint8).reshape(
                (224, 224, 3)
            )
            object.__setattr__(self, name, immutable)


@dataclass(frozen=True)
class StandardizedCandidates:
    """At least two model-coordinate candidates; no raw-command support is implied."""

    ids: tuple[str, ...]
    values: np.ndarray

    def __post_init__(self):
        ids = tuple(self.ids)
        value = self.values
        if not 2 <= len(ids) <= 300 or len(set(ids)) != len(ids):
            raise ValueError("Candidate roster requires 2–300 unique identifiers")
        if any(
            not isinstance(name, str)
            or re.fullmatch(r"[a-zA-Z0-9_-]{1,64}", name) is None
            for name in ids
        ):
            raise ValueError("Invalid candidate identifier")
        if type(value) is not np.ndarray:
            raise ValueError("Candidates require an exact ndarray")
        shape = (1, len(ids), 5, 10)
        view = memoryview(value)
        if view.format != "f" or view.shape != shape:
            raise ValueError("Candidate shape requires float32 [1,N,5,10]")
        immutable = np.frombuffer(view.tobytes(), dtype=np.float32).reshape(shape)
        if not np.isfinite(immutable).all() or np.abs(immutable).max() > 1_000_000:
            raise ValueError("Candidate numerical admission failed")
        object.__setattr__(self, "ids", ids)
        object.__setattr__(
            self,
            "values",
            immutable,
        )

    def raw_commands(self):
        raise ValueError(
            "Raw execution is unsupported without a source-bound scaler and action-support profile"
        )


@dataclass(frozen=True)
class ForecastCommit:
    directory: Path
    commitment_sha256: str
    candidate_commitment_sha256: str
    selected_candidate: str


def _forecast_candidates(
    model,
    observation: ObservationPair,
    candidates: StandardizedCandidates,
    source: dict,
    device: str,
    output: Path,
) -> ForecastCommit:
    """Commit exact inputs, query the actual model, then commit every forecast.

    This helper owns no simulator, Agent Bridge, reference label, or PID request.
    Source fields identify caller-prepared code; the qualified CLI supplies them.
    """
    if device not in ("cpu", "mps"):
        raise ValueError("Unsupported device")
    from .model import cost_call, preprocess
    import torch

    output.mkdir(parents=False, exist_ok=False)
    inputs = array_save(
        output / "inputs.npz",
        current=observation.current,
        goal=observation.goal,
        standardized_actions=candidates.values,
    )
    save_json(
        output / "input-commitment.json",
        {
            "schema": "prisoma.lewm.candidates.v1",
            "source": source,
            "candidate_ids": candidates.ids,
            "artifact": inputs,
            "coordinates": "standardized_5x5x2",
            "raw_support": "unknown",
            "reference_outcome_access": False,
            "execution_authority": False,
        },
    )
    pixels, goal = preprocess(observation.current.copy(), observation.goal.copy())
    costs, forecast, goal_embedding = cost_call(
        model, pixels, goal, torch.from_numpy(candidates.values.copy()), device
    )
    if (
        costs.shape != (1, len(candidates.ids))
        or forecast.shape != (1, len(candidates.ids), 6, 192)
        or goal_embedding.shape != (1, 1, 192)
        or any(value.dtype != np.float32 for value in (costs, forecast, goal_embedding))
    ):
        raise ValueError("Malformed model forecast")
    arrays = array_save(
        output / "forecasts.npz",
        costs=costs,
        forecast=forecast,
        goal_embedding=goal_embedding,
    )
    selected = candidates.ids[int(np.argmin(costs[0]))]
    save_json(
        output / "forecast-commitment.json",
        {
            "schema": "prisoma.lewm.forecasts.v1",
            "input_commitment_sha256": sha(output / "input-commitment.json"),
            "artifact": arrays,
            "selected_candidate": selected,
            "selection": "minimum_latent_squared_error_first_index_on_tie",
            "meaning": "model_objective_not_reference_outcome",
            "raw_actions_executed": False,
        },
    )
    return ForecastCommit(
        output,
        sha(output / "forecast-commitment.json"),
        sha(output / "input-commitment.json"),
        selected,
    )


def verify_forecast(directory: Path) -> dict:
    """Verify an exported forecast against its retained candidate commitment."""
    inputs = read_json(directory / "input-commitment.json")
    result = read_json(directory / "forecast-commitment.json")
    if set(inputs) != {
        "schema",
        "source",
        "candidate_ids",
        "artifact",
        "coordinates",
        "raw_support",
        "reference_outcome_access",
        "execution_authority",
    }:
        raise ValueError("Input commitment roster mismatch")
    if set(result) != {
        "schema",
        "input_commitment_sha256",
        "artifact",
        "selected_candidate",
        "selection",
        "meaning",
        "raw_actions_executed",
    }:
        raise ValueError("Forecast commitment roster mismatch")
    if (
        inputs["schema"] != "prisoma.lewm.candidates.v1"
        or result["schema"] != "prisoma.lewm.forecasts.v1"
    ):
        raise ValueError("Unsupported forecast schema")
    if (
        inputs["coordinates"] != "standardized_5x5x2"
        or inputs["raw_support"] != "unknown"
        or inputs["reference_outcome_access"] is not False
        or inputs["execution_authority"] is not False
    ):
        raise ValueError("Unsupported input evidence authority")
    if (
        result["raw_actions_executed"] is not False
        or result["meaning"] != "model_objective_not_reference_outcome"
        or result["selection"] != "minimum_latent_squared_error_first_index_on_tie"
    ):
        raise ValueError("Unsupported forecast authority or selector")
    if result["input_commitment_sha256"] != sha(directory / "input-commitment.json"):
        raise ValueError("Changed candidate commitment after forecast")
    for record, name in (
        (inputs["artifact"], "inputs.npz"),
        (result["artifact"], "forecasts.npz"),
    ):
        if (
            set(record) != {"path", "sha256", "bytes"}
            or record["path"] != name
            or record["sha256"] != sha(directory / name)
            or record["bytes"] != (directory / name).stat().st_size
        ):
            raise ValueError("Changed committed forecast artifact")
    payload = load_arrays(directory / "inputs.npz")
    if set(payload) != {"current", "goal", "standardized_actions"}:
        raise ValueError("Forecast input array roster mismatch")
    ObservationPair(payload["current"], payload["goal"])
    candidates = StandardizedCandidates(
        tuple(inputs["candidate_ids"]), payload["standardized_actions"]
    )
    arrays = load_arrays(directory / "forecasts.npz")
    expected = {
        "costs": (1, len(candidates.ids)),
        "forecast": (1, len(candidates.ids), 6, 192),
        "goal_embedding": (1, 1, 192),
    }
    if set(arrays) != set(expected) or any(
        arrays[key].shape != shape
        or arrays[key].dtype != np.float32
        or not np.isfinite(arrays[key]).all()
        for key, shape in expected.items()
    ):
        raise ValueError("Malformed committed forecast")
    costs = (
        (arrays["forecast"][..., -1, :] - arrays["goal_embedding"][:, None, -1, :]) ** 2
    ).sum(axis=-1)
    if not np.allclose(costs, arrays["costs"], atol=0.0001, rtol=0.0001):
        raise ValueError("Forecast cost does not match the captured objective")
    selected = candidates.ids[int(np.argmin(arrays["costs"][0]))]
    if result["selected_candidate"] != selected:
        raise ValueError("Selection does not follow the committed score")
    return {
        "status": "pass",
        "candidate_count": len(candidates.ids),
        "selected_candidate": selected,
        "scope": "captured_forecast_arithmetic_not_model_quality",
    }


def pid_handoff(
    *,
    candidate_digest: str,
    target_family: str,
    baseline_candidate_digest: str | None,
    maximum_ancestor_step: int,
    prediction_landmark_step: int,
    target_available_step: int,
) -> dict:
    """Check declared ancestry structure, without claiming an H3 producer or estimate.

    candidate_digest identifies the complete input commitment, including ordered
    candidate IDs, source identity, coordinate convention, and exact tensor bytes.
    It is not merely the NPZ digest or the digest of an unnamed action vector.
    """
    if re.fullmatch(r"[0-9a-f]{64}", candidate_digest) is None:
        raise ValueError("Invalid candidate commitment")
    if target_family == "same_candidate_action":
        raise ValueError(
            "Candidate-target injection: conditioned forecast cannot source its own proposal target"
        )
    if target_family not in (
        "downstream_command",
        "reference_state_outcome",
        "physical_outcome",
    ):
        raise ValueError("Unknown target family")
    if baseline_candidate_digest != candidate_digest:
        raise ValueError("Matched baseline must receive the exact same candidate")
    if any(
        type(value) is not int or value < 0
        for value in (
            maximum_ancestor_step,
            prediction_landmark_step,
            target_available_step,
        )
    ):
        raise ValueError("Invalid prediction landmark")
    if not maximum_ancestor_step <= prediction_landmark_step < target_available_step:
        raise ValueError("Future or target ancestry crosses the prediction landmark")
    return {
        "schema": "prisoma.lewm.pid-structural-handoff.v1",
        "target_family": target_family,
        "candidate_sha256": candidate_digest,
        "status": "structural_check_only",
        "ancestry_authority": "caller_declared_not_attested",
        "estimator_requested": False,
        "language_source": "absent",
        "population_gate": "open",
        "measure_gate": "not_adjudicated",
        "estimator_gate": "blocked",
        "application_gate": "blocked",
    }
