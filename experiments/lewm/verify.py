"""Independent NumPy reconstruction of captured CEM arithmetic.

No model, simulator, or PyTorch import is used. Passing this check does not
validate the model's forecasts or reproduce the physics environment.
"""

import hashlib

import numpy as np

from .assets import PACKAGE, read_json, sha
from .contracts import load_arrays, verify_forecast


def require(condition, message):
    if not condition:
        raise ValueError(message)


def same(left, right, message):
    require(np.allclose(left, right, atol=0.0001, rtol=0.0001), message)


def objective(goal, reference):
    require(goal.shape == (1, 1, 192), "Goal embedding shape")
    require(
        goal.dtype == np.float32 and np.isfinite(goal).all(),
        "Goal embedding type/finite",
    )
    require(np.array_equal(goal, reference), "Changed goal objective")


def verify_round(arrays):
    expected = {
        "candidates": (1, 300, 5, 10),
        "costs": (1, 300),
        "forecast": (1, 300, 6, 192),
        "goal_embedding": (1, 1, 192),
        "topk_inds": (1, 30),
        "topk_candidates": (1, 30, 5, 10),
        "topk_vals": (1, 30),
        "mean": (1, 5, 10),
        "var": (1, 5, 10),
        "prev_mean": (1, 5, 10),
        "prev_var": (1, 5, 10),
    }
    require(set(arrays) == set(expected), "Round array roster")
    for name, value in arrays.items():
        require(
            not value.dtype.hasobject and np.isfinite(value).all(),
            f"Invalid array: {name}",
        )
        require(value.shape == expected[name], f"Invalid shape: {name}")
        if name != "topk_inds":
            require(value.dtype == np.float32, f"Invalid type: {name}")
    candidates, costs = arrays["candidates"], arrays["costs"]
    require(candidates.shape == (1, 300, 5, 10), "Candidate shape")
    require(costs.shape == (1, 300), "Cost shape")
    require(arrays["forecast"].shape == (1, 300, 6, 192), "Forecast shape")
    indexes = arrays["topk_inds"]
    require(indexes.shape == (1, 30) and indexes.dtype.kind in "iu", "Elite shape/type")
    require(
        len(np.unique(indexes)) == 30 and np.all((indexes >= 0) & (indexes < 300)),
        "Elite membership",
    )
    elite = candidates[0, indexes[0]][None]
    same(elite, arrays["topk_candidates"], "Elite payload mismatch")
    same(costs[0, indexes[0]][None], arrays["topk_vals"], "Elite cost mismatch")
    same(
        np.sort(costs, axis=1)[:, :30],
        np.sort(arrays["topk_vals"], axis=1),
        "Incorrect elite selection",
    )
    same(candidates[:, 0], arrays["prev_mean"], "First proposal must equal prior mean")
    same(elite.mean(axis=1), arrays["mean"], "Incorrect CEM mean")
    same(
        elite.std(axis=1, ddof=1),
        arrays["var"],
        "Incorrect CEM sample standard deviation",
    )
    goal = arrays["goal_embedding"][..., -1, :]
    predicted = arrays["forecast"][..., -1, :]
    calculated = ((predicted - goal[:, None, :]) ** 2).sum(axis=-1)
    same(calculated, costs, "Forecast/goal/cost mismatch")


def verify(directory, *, legacy_construction=False):
    result = read_json(directory / "result.json")
    for name, key in (
        ("input-commitment.json", "input_commitment_sha256"),
        ("run-binding.json", "run_binding_sha256"),
    ):
        require(
            hashlib.sha256((directory.parent / name).read_bytes()).hexdigest()
            == result[key],
            "Changed run/input binding",
        )
    fixed_record = result["mps"]["artifact"]
    require(
        fixed_record["path"] == "fixed-mps.npz", "Unexpected fixed objective artifact"
    )
    fixed_path = directory / fixed_record["path"]
    require(
        hashlib.sha256(fixed_path.read_bytes()).hexdigest() == fixed_record["sha256"],
        "Fixed objective digest mismatch",
    )
    fixed = load_arrays(fixed_path)
    reference_goal = fixed["goal_embedding"].copy()
    objective(reference_goal, reference_goal)
    records = result["cem"]["rounds"]
    require(len(records) == 30, "Incomplete CEM trace")
    previous = None
    for index, record in enumerate(records):
        require(record["path"] == f"round-{index:02d}.npz", "Wrong ordered round name")
        path = directory / record["path"]
        require(
            hashlib.sha256(path.read_bytes()).hexdigest() == record["sha256"],
            "Artifact digest mismatch",
        )
        arrays = load_arrays(path)
        verify_round(arrays)
        objective(arrays["goal_embedding"], reference_goal)
        if not legacy_construction:
            commitment = read_json(directory / f"candidates-{index:02d}.json")
            require(
                set(commitment)
                == {
                    "schema",
                    "artifact",
                    "input_commitment_sha256",
                    "run_binding_sha256",
                    "round",
                    "forecast_computed",
                },
                "Candidate commitment roster",
            )
            require(
                commitment["schema"] == "prisoma.lewm.cem-input.v1"
                and commitment["round"] == index
                and commitment["forecast_computed"] is False,
                "Candidate commitment identity",
            )
            for key in ("input_commitment_sha256", "run_binding_sha256"):
                require(
                    commitment[key] == result[key],
                    "Candidate input/run binding mismatch",
                )
            candidate_path = directory / f"candidates-{index:02d}.npz"
            require(
                commitment["artifact"]["path"] == candidate_path.name
                and sha(candidate_path) == commitment["artifact"]["sha256"],
                "Candidate artifact mismatch",
            )
            candidate = load_arrays(candidate_path)
            require(
                set(candidate) == {"candidates"}
                and np.array_equal(candidate["candidates"], arrays["candidates"]),
                "Changed candidate after commitment",
            )
        if previous is not None:
            same(previous["mean"], arrays["prev_mean"], "Broken mean join")
            same(previous["var"], arrays["prev_var"], "Broken scale join")
        else:
            same(arrays["prev_mean"], np.zeros((1, 5, 10)), "Wrong initial mean")
            same(arrays["prev_var"], np.ones((1, 5, 10)), "Wrong initial scale")
        previous = arrays
    record = result["cem"]["final_recommendation"]
    path = directory / record["path"]
    require(path.name == "final-recommendation.npz", "Unexpected final artifact")
    require(
        hashlib.sha256(path.read_bytes()).hexdigest() == record["sha256"],
        "Final digest mismatch",
    )
    final = load_arrays(path)
    if final:
        expected = {
            "standardized_actions": (1, 5, 10),
            "costs": (1, 1),
            "forecast": (1, 1, 6, 192),
            "goal_embedding": (1, 1, 192),
        }
        require(set(final) == set(expected), "Final array roster")
        for name, shape in expected.items():
            require(
                final[name].shape == shape
                and final[name].dtype == np.float32
                and np.isfinite(final[name]).all(),
                f"Invalid final array: {name}",
            )
        objective(final["goal_embedding"], reference_goal)
        same(
            final["standardized_actions"],
            previous["mean"],
            "Final recommendation differs from final mean",
        )
        require(np.isfinite(final["costs"]).all(), "Non-finite final score")
        goal = final["goal_embedding"][..., -1, :]
        predicted = final["forecast"][..., -1, :]
        same(
            ((predicted - goal[:, None, :]) ** 2).sum(axis=-1),
            final["costs"],
            "Final forecast/goal/cost mismatch",
        )
    verify_fixed_controls(directory, result)
    if not legacy_construction:
        verify_forecast(directory / "public-api")
        public_inputs = load_arrays(directory / "public-api/inputs.npz")
        frozen_inputs = load_arrays(directory.parent / "frozen-inputs.npz")
        for public, frozen in (
            ("current", "pixels"),
            ("goal", "goal"),
            ("standardized_actions", "standardized_actions"),
        ):
            require(
                np.array_equal(public_inputs[public], frozen_inputs[frozen]),
                "Public API input differs from the frozen qualification input",
            )
        public_forecasts = load_arrays(directory / "public-api/forecasts.npz")
        fixed_cpu = load_arrays(directory / "fixed-cpu.npz")
        for name in public_forecasts:
            same(
                public_forecasts[name],
                fixed_cpu[name],
                "Public API forecast differs from the fixed-candidate control",
            )
    return {
        "rounds": 30,
        "proposals": 9000,
        "status": "pass",
        "scope": "CEM arithmetic and captured forecast-cost joins only",
    }


def verify_fixed_controls(directory, result):
    plan = read_json(PACKAGE / "qualification-plan.json")
    control = plan["controls"]
    outputs = {}
    for device in ("cpu", "mps"):
        path = directory / f"fixed-{device}.npz"
        require(
            result[device]["artifact"]["path"] == path.name
            and sha(path) == result[device]["artifact"]["sha256"],
            "Fixed control digest mismatch",
        )
        arrays = load_arrays(path)
        expected = {
            "costs": (1, 4),
            "forecast": (1, 4, 6, 192),
            "goal_embedding": (1, 1, 192),
            "repeated_costs": (1, 4),
            "repeated_forecast": (1, 4, 6, 192),
        }
        require(set(arrays) == set(expected), "Fixed array roster")
        for key, shape in expected.items():
            require(
                arrays[key].shape == shape
                and arrays[key].dtype == np.float32
                and np.isfinite(arrays[key]).all(),
                "Invalid fixed array",
            )
        costs = (
            (arrays["forecast"][..., -1, :] - arrays["goal_embedding"][:, None, -1, :])
            ** 2
        ).sum(axis=-1)
        same(costs, arrays["costs"], "Fixed forecast-cost mismatch")
        drift = max(
            float(np.max(np.abs(arrays[key] - arrays[f"repeated_{key}"])))
            for key in ("costs", "forecast")
        )
        require(drift <= control["repeat_max_abs"], "Repeat control failed")
        sensitivity = float(
            np.linalg.norm(arrays["forecast"][0, 1, -1] - arrays["forecast"][0, 2, -1])
        )
        require(
            sensitivity
            > max(
                control["action_l2_min"],
                control["sensitivity_repeat_multiplier"] * drift,
            ),
            "Action sensitivity control failed",
        )
        outputs[device] = arrays
    for key, prefix in (("forecast", "predictor"), ("costs", "cost")):
        require(
            np.allclose(
                outputs["cpu"][key],
                outputs["mps"][key],
                atol=control[f"{prefix}_atol"],
                rtol=control[f"{prefix}_rtol"],
            ),
            "CPU/MPS parity failed",
        )
    require(
        np.array_equal(
            np.argsort(outputs["cpu"]["costs"]), np.argsort(outputs["mps"]["costs"])
        ),
        "Candidate ordering changed",
    )
