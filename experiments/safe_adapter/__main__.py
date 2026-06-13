"""CLI for the SAFE -> (V,L,D,A) adapter.

Examples
--------
Generate a synthetic SAFE rollout directory (for testing the pipeline without the
multi-GB downloads), convert it, and verify the contract::

    python -m experiments.safe_adapter synth --out /tmp/safe_synth
    python -m experiments.safe_adapter convert --rollouts /tmp/safe_synth \\
        --out outputs/safe_vlda.json --seen-tasks 0,1
    python -m experiments.safe_adapter verify --input outputs/safe_vlda.json

The converted JSON is consumable directly by ``pid-offline-harness --input``.
"""

from __future__ import annotations

import argparse
import sys

from .convert import MappingConfig, rollouts_to_dataset
from .extract import VariableSpec
from .rollouts import load_safe_rollout_dir, write_synthetic_safe_dir
from .verify import summarize, verify_contract_file


def _parse_seen(value: str | None) -> set[int] | None:
    if value is None:
        return None
    return {int(tok) for tok in value.split(",") if tok.strip()}


def _mapping_from_args(args: argparse.Namespace) -> MappingConfig:
    if args.pooled:
        # Only pooled hidden states available: V, L, D all pool the same state
        # (degenerate decomposition); L can instead be a text proxy.
        return MappingConfig(
            v=VariableSpec("hidden_pool"),
            l=(
                VariableSpec("text_hash", dim=args.text_dim)
                if args.text_proxy_l
                else VariableSpec("hidden_pool")
            ),
            d=VariableSpec("hidden_pool"),
            train_if_seen=not args.train_if_unseen,
        )
    return MappingConfig(train_if_seen=not args.train_if_unseen)


def cmd_synth(args: argparse.Namespace) -> int:
    path = write_synthetic_safe_dir(
        args.out,
        n_tasks=args.n_tasks,
        episodes_per_task=args.episodes_per_task,
        n_steps=args.n_steps,
        seed=args.seed,
        raw_token_states=not args.pooled_states,
    )
    print(f"wrote synthetic SAFE rollouts to {path}")
    return 0


def cmd_convert(args: argparse.Namespace) -> int:
    rollouts = load_safe_rollout_dir(args.rollouts, seen_task_ids=_parse_seen(args.seen_tasks))
    dataset = rollouts_to_dataset(
        rollouts,
        _mapping_from_args(args),
        run_id=args.run_id,
        model=args.model,
        task=args.task,
    )
    out = dataset.write_json(args.out)
    dims = dataset.dims()
    print(
        f"wrote {len(dataset.samples)} samples from {len(rollouts)} rollouts to {out} "
        f"(dims v={dims['v']} l={dims['l']} d={dims['d']} a={dims['a']})"
    )
    return 0


def cmd_verify(args: argparse.Namespace) -> int:
    report = verify_contract_file(args.input)
    print(summarize(report))
    return 0 if report.ok else 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="experiments.safe_adapter")
    sub = parser.add_subparsers(dest="command", required=True)

    synth = sub.add_parser("synth", help="write a synthetic SAFE rollout directory")
    synth.add_argument("--out", required=True)
    synth.add_argument("--n-tasks", type=int, default=4)
    synth.add_argument("--episodes-per-task", type=int, default=4)
    synth.add_argument("--n-steps", type=int, default=12)
    synth.add_argument("--seed", type=int, default=0)
    synth.add_argument(
        "--pooled-states",
        action="store_true",
        help="store pooled (T,d) hidden states instead of raw (T,n_token,d)",
    )
    synth.set_defaults(func=cmd_synth)

    convert = sub.add_parser("convert", help="convert SAFE rollouts to the VLDA contract")
    convert.add_argument("--rollouts", required=True)
    convert.add_argument("--out", required=True)
    convert.add_argument("--seen-tasks", default=None, help="comma-separated seen task ids")
    convert.add_argument("--run-id", default="safe_adapter")
    convert.add_argument("--model", default=None)
    convert.add_argument("--task", default=None)
    convert.add_argument(
        "--pooled",
        action="store_true",
        help="rollouts only expose pooled hidden states (V==D; see --text-proxy-l)",
    )
    convert.add_argument(
        "--text-proxy-l",
        action="store_true",
        help="with --pooled, derive L from a text hashing proxy of the instruction",
    )
    convert.add_argument("--text-dim", type=int, default=16)
    convert.add_argument(
        "--train-if-unseen",
        action="store_true",
        help="invert the default seen=train mapping (use unseen tasks as train)",
    )
    convert.set_defaults(func=cmd_convert)

    verify = sub.add_parser("verify", help="verify a contract JSON file")
    verify.add_argument("--input", required=True)
    verify.set_defaults(func=cmd_verify)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
