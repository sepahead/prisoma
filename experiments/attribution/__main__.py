"""CLI: run the attribution probe on the reference model and write a run log.

Example::

    python -m experiments.attribution demo \\
        --runlog outputs/attribution_runlog.jsonl --artifacts outputs/attribution

The emitted JSONL validates with ``pid-runlog-replay --validate`` and carries one
``attribution_logged`` event per method with its faithfulness verdict.
"""

from __future__ import annotations

import argparse
import sys

import numpy as np

from .model import SmallTransformer
from .probe import run_attribution_probe
from .runlog import write_attribution_runlog


def cmd_demo(args: argparse.Namespace) -> int:
    # Same construction the test suite proves faithful (model seed s, input
    # rng seed s+100): the default demo then demonstrates the PASSING path for
    # a genuinely faithful map (LRP here) — reproducing the tested case, not
    # cherry-picking — while degenerate maps still FAIL honestly.
    rng = np.random.default_rng(args.seed + 100)
    model = SmallTransformer(d_in=args.d_in, d_model=args.d_model, seed=args.seed)
    x = rng.standard_normal((args.tokens, args.d_in))
    records = run_attribution_probe(
        model,
        x,
        target_output=args.target,
        modality=args.modality,
        seed=args.seed,
    )
    out = write_attribution_runlog(
        args.runlog,
        records,
        config={"model": "small_transformer", "target_output": args.target},
        artifact_dir=args.artifacts,
    )
    for rec in records:
        verdict = "PASS" if rec.faithfulness_passed else "FAIL"
        print(
            f"{rec.method}: faithfulness={verdict} "
            f"(aopc method={rec.metadata['method_aopc']} "
            f"random={rec.metadata['random_aopc']})"
        )
    print(f"wrote {out}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="experiments.attribution")
    sub = parser.add_subparsers(dest="command", required=True)
    demo = sub.add_parser("demo", help="run the probe on the reference model")
    demo.add_argument("--runlog", default="outputs/attribution_runlog.jsonl")
    demo.add_argument("--artifacts", default=None, help="dir to save .npy relevance artifacts")
    demo.add_argument("--tokens", type=int, default=6)
    demo.add_argument("--d-in", type=int, default=5)
    demo.add_argument("--d-model", type=int, default=8)
    demo.add_argument("--target", default="action_dim_0")
    demo.add_argument("--modality", default=None)
    demo.add_argument("--seed", type=int, default=3)
    demo.set_defaults(func=cmd_demo)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
