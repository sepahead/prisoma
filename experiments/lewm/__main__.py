"""Offline stage, qualify, and verify commands for the optional LeWM adapter."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import signal
import sys
import traceback

from .assets import read_json, save_json, sha, stage, verify_stage


def verify_run(root: Path, *, legacy_construction: bool = False) -> dict:
    from .contracts import load_arrays
    from .verify import verify

    terminal = read_json(root / "terminal.json")
    arms = ("repository_jepa", "model_config_wheel_lewm")
    if (
        set(terminal["arms"]) != set(arms)
        or terminal["scientific_status"] != "unchanged_open"
    ):
        raise ValueError("Invalid terminal source-arm roster or scientific status")
    input_record = read_json(root / "input-commitment.json")["artifact"]
    if (
        input_record["path"] != "frozen-inputs.npz"
        or sha(root / "frozen-inputs.npz") != input_record["sha256"]
        or (root / "frozen-inputs.npz").stat().st_size != input_record["bytes"]
    ):
        raise ValueError("Changed frozen input artifact")
    arrays = load_arrays(root / "frozen-inputs.npz")
    expected = {
        "pixels": (224, 224, 3),
        "goal": (224, 224, 3),
        "actual_initial_state": (7,),
        "transformed_pixels": (3, 224, 224),
        "transformed_goal": (3, 224, 224),
        "standardized_actions": (1, 4, 5, 10),
    }
    if set(arrays) != set(expected) or any(
        arrays[key].shape != shape for key, shape in expected.items()
    ):
        raise ValueError("Malformed frozen input arrays")
    if not legacy_construction and terminal["raw_actions_executed"] is not False:
        raise ValueError("Raw execution exceeds this profile")
    results = {}
    for arm in arms:
        record = terminal["arms"][arm]
        if record["status"] != "completed" or record["control_pass"] is not True:
            raise ValueError("The model arm did not pass its controls")
        if sha(root / arm / "result.json") != record["result_sha256"]:
            raise ValueError("Changed terminal result binding")
        results[arm] = verify(root / arm, legacy_construction=legacy_construction)
    return {
        "status": "pass",
        "arms": results,
        "scope": "engineering_arithmetic_not_model_quality",
    }


def differential(root: Path, reference: Path) -> dict:
    """Compare the two maintained arms with the explicitly supplied construction run."""
    import numpy as np

    from .contracts import load_arrays

    verify_run(reference, legacy_construction=True)
    comparisons = {}
    for arm in ("repository_jepa", "model_config_wheel_lewm"):
        maxima = {}
        for filename in [
            "fixed-cpu.npz",
            "fixed-mps.npz",
            "final-recommendation.npz",
        ] + [f"round-{index:02d}.npz" for index in range(30)]:
            current = load_arrays(root / arm / filename)
            original = load_arrays(reference / arm / filename)
            if set(current) != set(original):
                raise ValueError("Differential array roster mismatch")
            for key in current:
                left, right = current[key], original[key]
                if left.shape != right.shape or left.dtype != right.dtype:
                    raise ValueError("Differential array shape/type mismatch")
                same = (
                    np.array_equal(left, right)
                    if left.dtype.kind in "iu"
                    else np.allclose(left, right, atol=0.0001, rtol=0.0001)
                )
                if not same:
                    raise ValueError(f"Differential mismatch: {arm}/{filename}/{key}")
                maxima[f"{filename}:{key}"] = float(np.max(np.abs(left - right)))
        comparisons[arm] = {
            "reference_result_sha256": sha(reference / arm / "result.json"),
            "current_result_sha256": sha(root / arm / "result.json"),
            "maximum_absolute_difference": max(maxima.values()),
            "array_comparisons": len(maxima),
        }
    return {
        "status": "pass",
        "arms": comparisons,
        "scope": "one_frozen_input_not_general_source_equivalence",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("stage", "qualify"):
        command = commands.add_parser(name)
        command.add_argument("--assets-root", type=Path, required=True)
        command.add_argument("--output", type=Path, required=True)
        if name == "qualify":
            command.add_argument("--reference-run", type=Path)
    command = commands.add_parser("verify")
    command.add_argument("--run", type=Path, required=True)
    args = parser.parse_args()
    sys.dont_write_bytecode = True
    if args.command == "verify":
        print(json.dumps(verify_run(args.run.resolve()), indent=2))
        return 0
    if args.command == "stage":
        print(
            json.dumps(
                stage(args.assets_root.resolve(), args.output.resolve()), indent=2
            )
        )
        return 0
    args.output.mkdir(mode=0o700, parents=False, exist_ok=False)
    root = args.output.resolve()
    try:
        staged = root / "source-stage"
        stage(args.assets_root.resolve(), staged)
        from .runtime import qualify

        def expired(*_):
            raise TimeoutError("Frozen wall-time bound exceeded")

        signal.signal(signal.SIGALRM, expired)
        qualify(staged, root / "run")
        verified = verify_run(root / "run")
        save_json(root / "verification.json", verified)
        if args.reference_run is not None:
            compared = differential(root / "run", args.reference_run.resolve())
            save_json(root / "differential.json", compared)
        verify_stage(staged)
        save_json(
            root / "receipt.json",
            {
                "schema": "prisoma.lewm.engineering-publication.v1",
                "status": "pass",
                "terminal_sha256": sha(root / "run/terminal.json"),
                "verification_sha256": sha(root / "verification.json"),
                "differential_sha256": sha(root / "differential.json")
                if args.reference_run
                else None,
                "source_manifest_sha256": sha(staged / "source-manifest.json"),
                "scientific_status": "unchanged_open",
                "raw_actions_executed": False,
            },
        )
        print(json.dumps(read_json(root / "receipt.json"), indent=2))
        return 0
    except Exception as error:
        save_json(
            root / "failure.json",
            {
                "status": "failed",
                "error": type(error).__name__,
                "message": str(error),
                "traceback": traceback.format_exc(),
            },
        )
        raise


if __name__ == "__main__":
    raise SystemExit(main())
