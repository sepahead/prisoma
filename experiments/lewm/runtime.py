"""Actual pinned CPU/MPS qualification and complete CEM trace capture.

Execution remains opt-in. This module exposes no raw-command or outcome API.
"""

from __future__ import annotations

import importlib
import json
import os
from pathlib import Path
import signal
import sys
import time
import traceback
import types

from .assets import PACKAGE, read_json, save_json, sha, verify_stage, verify_runtime
from .contracts import array_save, ObservationPair, StandardizedCandidates
from .model import PreparedLeWM, owners, cost_call, preprocess, source_identity


def run_arm(plan, arm, model, cem_type, pixels, goal, fixed, output):
    import numpy as np
    import torch
    from gymnasium.spaces import Box

    started = time.monotonic()
    frozen = plan["controls"]
    results = {}
    for device in plan["device_policy"]["devices"]:
        model.to(device)
        predictions = [
            cost_call(model, pixels, goal, fixed, device)
            for _ in range(frozen["fixed_candidate_repeats"])
        ]
        costs, rollout, goal_embedding = predictions[0]
        drift = max(
            float(np.max(np.abs(predictions[1][i] - predictions[0][i]))) for i in (0, 1)
        )
        sensitivity = float(np.linalg.norm(rollout[0, 1, -1] - rollout[0, 2, -1]))
        artifact = array_save(
            output / f"fixed-{device}.npz",
            costs=costs,
            forecast=rollout,
            goal_embedding=goal_embedding,
            repeated_costs=predictions[1][0],
            repeated_forecast=predictions[1][1],
        )
        results[device] = {
            "artifact": artifact,
            "costs": costs.tolist(),
            "order": np.argsort(costs[0], kind="stable").tolist(),
            "repeat_max_abs": drift,
            "action_l2": sensitivity,
            "repeat_pass": drift <= frozen["repeat_max_abs"],
            "action_pass": sensitivity
            > max(
                frozen["action_l2_min"], frozen["sensitivity_repeat_multiplier"] * drift
            ),
        }
    with (
        np.load(output / "fixed-cpu.npz", allow_pickle=False) as cpu,
        np.load(output / "fixed-mps.npz", allow_pickle=False) as mps,
    ):
        results["parity"] = {
            "pass": bool(
                np.allclose(
                    cpu["forecast"],
                    mps["forecast"],
                    atol=frozen["predictor_atol"],
                    rtol=frozen["predictor_rtol"],
                )
            ),
            "maximum_absolute_difference": float(
                np.max(np.abs(cpu["forecast"] - mps["forecast"]))
            ),
            "cost_pass": bool(
                np.allclose(
                    cpu["costs"],
                    mps["costs"],
                    atol=frozen["cost_atol"],
                    rtol=frozen["cost_rtol"],
                )
            ),
            "cost_maximum_absolute_difference": float(
                np.max(np.abs(cpu["costs"] - mps["costs"]))
            ),
            "candidate_order_equal": results["cpu"]["order"] == results["mps"]["order"],
        }
    save_json(output / "fixed-control-results.json", results)

    # Invoke the exact installed solver. Instrumentation reads its actual model
    # outputs and callback tensors; it does not change proposals or costs.
    class TraceModel(torch.nn.Module):
        def __init__(self, actual):
            super().__init__()
            self.actual = actual
            self.latest = None
            self.goal = None
            self.round_index = 0

        def get_cost(self, info, candidates):
            candidate_record = array_save(
                output / f"candidates-{self.round_index:02d}.npz",
                candidates=candidates.detach().cpu().numpy().copy(),
            )
            save_json(
                output / f"candidates-{self.round_index:02d}.json",
                {
                    "schema": "prisoma.lewm.cem-input.v1",
                    "artifact": candidate_record,
                    "input_commitment_sha256": sha(
                        output.parent / "input-commitment.json"
                    ),
                    "run_binding_sha256": sha(output.parent / "run-binding.json"),
                    "round": self.round_index,
                    "forecast_computed": False,
                },
            )
            self.round_index += 1
            costs = self.actual.get_cost(info, candidates)
            self.latest = info["predicted_emb"].detach().cpu().numpy().copy()
            self.goal = info["goal_emb"].detach().cpu().numpy().copy()
            return costs

    class Trace:
        output_key = "prisoma_trace"
        history = []

        def reset(self):
            self.history = []

        def start_batch(self):
            pass

        def end_solve(self):
            pass

        def __call__(self, **values):
            arrays = {
                k: v.detach().cpu().numpy()
                for k, v in values.items()
                if torch.is_tensor(v)
            }
            arrays["forecast"] = traced.latest
            arrays["goal_embedding"] = traced.goal
            record = array_save(output / f"round-{values['step']:02d}.npz", **arrays)
            self.history.append(record)
            if (
                sum(p.stat().st_size for p in output.glob("*.npz"))
                > plan["resource_bounds"]["max_trace_bytes_per_arm"]
            ):
                raise ValueError("Trace byte bound exceeded")
            print(
                json.dumps({"arm": arm, "round": values["step"], "artifact": record}),
                flush=True,
            )

    traced = TraceModel(model.to("mps"))
    trace = Trace()
    config = plan["cem"]
    solver = cem_type(
        model=traced,
        batch_size=config["batch_size"],
        num_samples=config["num_samples"],
        var_scale=config["var_scale"],
        n_steps=config["n_steps"],
        topk=config["topk"],
        device="mps",
        seed=plan["seed"],
        callbacks=[trace],
    )
    solver.configure(
        action_space=Box(-1.0, 1.0, shape=(1, 2), dtype=np.float32),
        n_envs=1,
        config=types.SimpleNamespace(
            horizon=config["horizon"], action_block=config["action_block"]
        ),
    )
    inputs = {
        "pixels": pixels[None, None],
        "goal": goal[None, None],
        "action": torch.zeros(1, 1, 10),
    }
    with torch.inference_mode():
        planned = solver.solve(inputs)
    final = planned["actions"]
    final_cost, final_forecast, final_goal = cost_call(
        model, pixels, goal, final[:, None], "mps"
    )
    final_record = array_save(
        output / "final-recommendation.npz",
        standardized_actions=final.detach().cpu().numpy(),
        costs=final_cost,
        forecast=final_forecast,
        goal_embedding=final_goal,
    )
    if len(trace.history) != config["n_steps"]:
        raise ValueError("Incomplete CEM round trace")
    results["cem"] = {
        "rounds": trace.history,
        "final_recommendation": final_record,
        "raw_support": "unknown_without_source_bound_scaler",
        "raw_actions_executed": False,
    }
    results["elapsed_seconds_observation_only"] = time.monotonic() - started
    results["input_commitment_sha256"] = sha(output.parent / "input-commitment.json")
    results["run_binding_sha256"] = sha(output.parent / "run-binding.json")
    results["control_pass"] = (
        all(
            results[d]["repeat_pass"] and results[d]["action_pass"]
            for d in ("cpu", "mps")
        )
        and results["parity"]["pass"]
        and results["parity"]["cost_pass"]
        and results["parity"]["candidate_order_equal"]
    )
    return results


def qualify(staged: Path, output: Path) -> dict:
    """Run the fixed engineering plan. No download, raw action, or label occurs."""
    if "torch" in sys.modules:
        raise ValueError("Qualification requires a fresh process before Torch import")
    if os.environ.get("PYTORCH_ENABLE_MPS_FALLBACK", "0") != "0":
        raise ValueError("MPS fallback must be disabled before process launch")
    os.environ.update(
        PYTHONDONTWRITEBYTECODE="1",
        PYTORCH_ENABLE_MPS_FALLBACK="0",
        HF_HUB_OFFLINE="1",
        TRANSFORMERS_OFFLINE="1",
        SDL_VIDEODRIVER="dummy",
        SDL_AUDIODRIVER="dummy",
    )
    sys.dont_write_bytecode = True
    runtime = verify_runtime()
    verify_stage(staged)
    plan = read_json(PACKAGE / "qualification-plan.json")
    if plan["execution_authorized_in_this_plan"] or plan["scaler"] is not None:
        raise ValueError("The engineering profile cannot authorize raw execution")
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    save_json(output / "plan.json", plan)
    save_json(
        output / "run-binding.json",
        {
            "schema": "prisoma.lewm.engineering-run.v1",
            "plan_sha256": sha(output / "plan.json"),
            "projection_sha256": sha(PACKAGE / "projection.json"),
            "source_manifest_sha256": sha(staged / "source-manifest.json"),
            "adapter_sources": {
                p.name: sha(p)
                for p in sorted(PACKAGE.iterdir())
                if p.suffix in (".py", ".json", ".lock")
            },
            "runtime": runtime,
            "fallback_environment": os.environ["PYTORCH_ENABLE_MPS_FALLBACK"],
            "scope": plan["scope"],
            "source_observation_not_attestation": True,
        },
    )
    import numpy as np
    import torch

    if not torch.backends.mps.is_available():
        raise RuntimeError("MPS unavailable; fallback is forbidden")
    torch.set_num_threads(plan["resource_bounds"]["threads"])
    torch.manual_seed(plan["seed"])
    owners(staged)
    cem = importlib.import_module("stable_worldmodel.solver.cem").CEMSolver
    pusht = importlib.import_module("stable_worldmodel.envs.pusht.env").PushT
    world = pusht(**plan["render"], render_mode="rgb_array")
    try:
        state, _ = world.reset(
            seed=plan["seed"],
            options={
                "state": np.array(plan["initial_state"]),
                "goal_state": np.array(plan["goal_state"]),
            },
        )
        observation = ObservationPair(world.render(), world._goal.copy())
    finally:
        world.close()
    from PIL import Image

    Image.fromarray(observation.current).save(output / "input-pixels.png")
    Image.fromarray(observation.goal).save(output / "input-goal.png")
    pixels, goal = preprocess(observation.current.copy(), observation.goal.copy())
    dense = np.array(plan["fixed_standardized_actions"], dtype=np.float32)
    values = (
        np.broadcast_to(dense[:, None, None, :], (4, 5, 5, 2))
        .reshape(1, 4, 5, 10)
        .copy()
    )
    candidates = StandardizedCandidates(
        tuple(f"candidate-{i}" for i in range(4)), values
    )
    fixed = torch.from_numpy(candidates.values.copy())
    record = array_save(
        output / "frozen-inputs.npz",
        pixels=observation.current,
        goal=observation.goal,
        actual_initial_state=state["state"],
        transformed_pixels=pixels.numpy(),
        transformed_goal=goal.numpy(),
        standardized_actions=candidates.values,
    )
    save_json(
        output / "input-commitment.json",
        {
            "schema": "prisoma.lewm.input-commitment.v1",
            "artifact": record,
            "candidate_ids": candidates.ids,
            "weights_loaded": False,
            "outcome_accessed": False,
            "coordinates": "standardized_5x5x2",
            "raw_support": "unknown",
            "meaning": "pre-model-input-commitment_local_observation_not_attestation",
        },
    )
    all_results = {}
    for arm in plan["arms"]:
        directory = output / arm
        directory.mkdir()
        try:
            signal.alarm(plan["resource_bounds"]["max_wall_seconds_per_arm"])
            engine = PreparedLeWM(staged, arm, "cpu")
            engine.forecast(observation, candidates, directory / "public-api")
            model = engine._model
            save_json(directory / "model-source.json", source_identity(staged, arm))
            result = run_arm(plan, arm, model, cem, pixels, goal, fixed, directory)
            save_json(directory / "result.json", result)
            all_results[arm] = {
                "status": "completed",
                "control_pass": result["control_pass"],
                "result_sha256": sha(directory / "result.json"),
            }
            del model
            torch.mps.empty_cache()
        except Exception as error:
            failure = {
                "status": "failed",
                "error": type(error).__name__,
                "message": str(error),
                "traceback": traceback.format_exc(),
            }
            save_json(directory / "failure.json", failure)
            all_results[arm] = failure
        finally:
            signal.alarm(0)
    terminal = {
        "schema": "prisoma.lewm.engineering-terminal.v1",
        "arms": all_results,
        "scientific_status": "unchanged_open",
        "scope": plan["scope"],
        "raw_actions_executed": False,
    }
    save_json(output / "terminal.json", terminal)
    return terminal
