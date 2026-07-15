"""CLI for the deterministic multi-case attribution diagnostic demo.

Example::

    python -m experiments.attribution demo \
        --runlog outputs/attribution_runlog.jsonl --artifacts outputs/attribution

The emitted JSONL validates with ``pid-runlog-replay --validate``.  Its historical
``faithfulness_check`` field is a frozen group-level deletion ranking-sensitivity
verdict, not a causal or mechanistic claim.
"""

from __future__ import annotations

import argparse
import os
import sys

import numpy as np

from .faithfulness import (
    MAX_VALIDATION_CASES,
    RankingSensitivityGate,
    _canonical_identifier,
    _validate_exact_int,
    _validate_gate,
)
from .model import (
    MAX_INPUT_DIMENSION,
    MAX_INPUT_VALUES,
    MAX_MODEL_DIMENSION,
    MAX_SEED,
    SmallTransformer,
)
from .probe import ProbeValidationCase, run_attribution_probe
from .runlog import write_attribution_runlog


def cmd_demo(args: argparse.Namespace) -> int:
    tokens = _validate_exact_int(
        args.tokens, "tokens", minimum=1, maximum=MAX_INPUT_VALUES
    )
    d_in = _validate_exact_int(
        args.d_in, "d_in", minimum=1, maximum=MAX_INPUT_DIMENSION
    )
    d_model = _validate_exact_int(
        args.d_model, "d_model", minimum=1, maximum=MAX_MODEL_DIMENSION
    )
    validation_cases = _validate_exact_int(
        args.validation_cases,
        "validation_cases",
        minimum=1,
        maximum=MAX_VALIDATION_CASES,
    )
    if tokens * d_in > MAX_INPUT_VALUES:
        raise ValueError(
            f"tokens*d_in must be <= the {MAX_INPUT_VALUES}-value input budget"
        )
    _canonical_identifier(args.target, "target")
    if args.modality is not None:
        _canonical_identifier(args.modality, "modality")
    for field in ("runlog", "artifacts"):
        value = getattr(args, field)
        if value is not None and not isinstance(value, (str, os.PathLike)):
            raise ValueError(f"{field} must be a filesystem path")

    gate = RankingSensitivityGate(
        frozen_gate_id="reference-demo-ranking-sensitivity-v1",
        baseline_name="fixed_zero_tensor_demo",
        baseline_provenance=(
            "CLI-constructed shape-matched zero tensor; demo only; distributional "
            "support is not established"
        ),
        validation_split="deterministic-demo-validation",
        selection_split="deterministic-demo-selection",
        grouping_provenance="one synthetic RNG stream case per declared demo group",
        selection_group_ids=("demo-selection-group-000",),
        selection_unit_ids=("demo-selection-unit-000",),
        alpha=args.alpha,
        min_groups=args.min_groups,
        n_steps=args.steps,
        n_random_rankings=args.random_rankings,
        seed=args.seed,
    )
    _validate_gate(gate)
    if gate.n_steps > tokens * d_in:
        raise ValueError("steps must not exceed tokens*d_in")

    # Constructor validation occurs before any model arrays or case tensors are
    # allocated. The gate validation above likewise precedes the case batch.
    seed = _validate_exact_int(args.seed, "seed", minimum=0, maximum=MAX_SEED)
    rng = np.random.default_rng(seed + 100)
    model = SmallTransformer(d_in=d_in, d_model=d_model, seed=seed)
    cases = [
        ProbeValidationCase(
            case_id=f"demo-case-{index:03d}",
            group_id=f"demo-validation-group-{index:03d}",
            unit_ids=(f"demo-validation-unit-{index:03d}",),
            x=rng.standard_normal((tokens, d_in)),
            # Explicit demo intervention.  It can be OOD; the gate metadata and
            # README preserve that limitation rather than calling zero neutral.
            baseline=np.zeros((tokens, d_in), dtype=np.float64),
        )
        for index in range(validation_cases)
    ]
    records = run_attribution_probe(
        model,
        cases,
        gate=gate,
        target_output=args.target,
        modality=args.modality,
    )
    out = write_attribution_runlog(
        args.runlog,
        records,
        config={
            "model": "small_transformer",
            "target_output": args.target,
            "diagnostic": "deletion_ranking_sensitivity",
            "frozen_gate_id": gate.frozen_gate_id,
        },
        artifact_dir=args.artifacts,
    )
    for record in records:
        print(
            f"{record.method}: ranking_sensitivity={record.metadata['gate_status']} "
            f"reason={record.metadata['gate_reason']} "
            f"group_sign_test_p={record.metadata['group_sign_test_p']}"
        )
    print(f"wrote {out}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="experiments.attribution")
    sub = parser.add_subparsers(dest="command", required=True)
    demo = sub.add_parser(
        "demo", help="run the deterministic multi-case reference probe"
    )
    demo.add_argument("--runlog", default="outputs/attribution_runlog.jsonl")
    demo.add_argument(
        "--artifacts",
        default=None,
        help="dir inside the run-log directory for confined .npy relevance artifacts",
    )
    demo.add_argument("--tokens", type=int, default=6)
    demo.add_argument("--d-in", type=int, default=5)
    demo.add_argument("--d-model", type=int, default=8)
    demo.add_argument("--validation-cases", type=int, default=8)
    demo.add_argument("--steps", type=int, default=6)
    demo.add_argument("--random-rankings", type=int, default=1024)
    demo.add_argument("--alpha", type=float, default=0.05)
    demo.add_argument("--min-groups", type=int, default=5)
    demo.add_argument("--target", default="action_dim_0")
    demo.add_argument("--modality", default=None)
    demo.add_argument("--seed", type=int, default=3)
    demo.set_defaults(func=cmd_demo)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except ValueError as error:
        parser.error(str(error))


if __name__ == "__main__":
    sys.exit(main())
