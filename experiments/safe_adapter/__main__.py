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
from pathlib import Path

from .convert import MappingConfig, rollouts_to_dataset
from .extract import VariableSpec
from .rollouts import (
    load_safe_rollout_dir,
    write_safe_bundle_manifest,
    write_synthetic_safe_dir,
)
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
        )
    return MappingConfig()


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
    rollout_root = Path(args.rollouts).resolve()
    output_path = Path(args.out).resolve()
    if output_path.is_relative_to(rollout_root):
        raise ValueError(
            "conversion output must be outside the immutable ingress bundle"
        )
    if args.allow_legacy_pickle:
        print(
            "WARNING: legacy pickle import is an explicit trust boundary; only "
            "manifest-hashed NumPy containers are admitted, and pickle is not a sandbox",
            file=sys.stderr,
        )
    if args.allow_unverified_rights:
        print(
            "WARNING: proceeding with a manifest whose rights review may be unverified; "
            "this does not establish permission to use or redistribute the data",
            file=sys.stderr,
        )
    if args.allow_unfrozen_split:
        print(
            "WARNING: proceeding with an unfrozen or unreviewed split for audit only; "
            "this cannot support a held-out scientific claim",
            file=sys.stderr,
        )
    rollouts = load_safe_rollout_dir(
        args.rollouts,
        seen_task_ids=_parse_seen(args.seen_tasks),
        allow_legacy_pickle=args.allow_legacy_pickle,
        allow_unverified_rights=args.allow_unverified_rights,
        allow_unfrozen_split=args.allow_unfrozen_split,
    )
    dataset = rollouts_to_dataset(
        rollouts,
        _mapping_from_args(args),
        run_id=args.run_id,
        model=args.model,
        task=args.task,
    )
    out = dataset.write_json(args.out, overwrite=args.overwrite)
    dims = dataset.dims()
    print(
        f"wrote {len(dataset.samples)} samples from {len(rollouts)} rollouts to {out} "
        f"(dims v={dims['v']} l={dims['l']} d={dims['d']} a={dims['a']})"
    )
    return 0


def cmd_manifest(args: argparse.Namespace) -> int:
    seen_task_ids = _parse_seen(args.seen_tasks)
    if seen_task_ids is None:
        raise ValueError(
            "--seen-tasks is required for a leakage-auditable split receipt"
        )
    path = write_safe_bundle_manifest(
        args.rollouts,
        source_name=args.source_name,
        source_revision=args.source_revision,
        rights_status=args.rights_status,
        rights_reference=args.rights_reference,
        seen_task_ids=seen_task_ids,
        overwrite=args.overwrite,
        split_origin=args.split_origin,
        split_frozen_before_outcomes=args.split_frozen_before_outcomes,
        contamination_review=args.contamination_review,
        model_id=args.model_id,
        checkpoint_revision=args.checkpoint_revision,
        hook_id=args.hook_id,
        tensor_contract_sha256=args.tensor_contract_sha256,
        semantic_validation_status=args.semantic_validation_status,
    )
    print(f"wrote content-addressed SAFE ingress manifest to {path}")
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

    convert = sub.add_parser(
        "convert", help="convert SAFE rollouts to the VLDA contract"
    )
    convert.add_argument("--rollouts", required=True)
    convert.add_argument("--out", required=True)
    convert.add_argument(
        "--seen-tasks", default=None, help="comma-separated seen task ids"
    )
    convert.add_argument("--run-id", default="safe_adapter")
    convert.add_argument("--model", default=None)
    convert.add_argument("--task", default=None)
    convert.add_argument(
        "--overwrite",
        action="store_true",
        help="atomically replace an existing converted dataset output",
    )
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
        "--allow-legacy-pickle",
        action="store_true",
        help=(
            "explicitly admit manifest-hashed legacy NumPy pickles through the restricted "
            "unpickler; arbitrary globals and Torch pickles remain rejected"
        ),
    )
    convert.add_argument(
        "--allow-unverified-rights",
        action="store_true",
        help=(
            "explicitly proceed when the manifest rights status is unverified; this records "
            "the caveat and does not grant use or redistribution rights"
        ),
    )
    convert.add_argument(
        "--allow-unfrozen-split",
        action="store_true",
        help="audit-only override for a split not frozen/reviewed before outcomes",
    )
    convert.set_defaults(func=cmd_convert)

    manifest = sub.add_parser(
        "manifest",
        help="hash and bind a prepared SAFE bundle without deserializing its payloads",
    )
    manifest.add_argument("--rollouts", required=True)
    manifest.add_argument("--source-name", default="vla-safe/SAFE")
    manifest.add_argument("--source-revision", required=True)
    manifest.add_argument(
        "--rights-status",
        required=True,
        choices=("verified", "restricted", "unverified", "synthetic_generated"),
    )
    manifest.add_argument("--rights-reference", required=True)
    manifest.add_argument("--model-id", required=True)
    manifest.add_argument("--checkpoint-revision", required=True)
    manifest.add_argument("--hook-id", required=True)
    manifest.add_argument(
        "--tensor-contract-sha256",
        required=True,
        help="SHA-256 of the archived tensor contract for the captured hook",
    )
    manifest.add_argument(
        "--semantic-validation-status",
        choices=("unvalidated", "validated"),
        default="unvalidated",
        help="architecture/probe status; unvalidated is the honest default",
    )
    manifest.add_argument(
        "--seen-tasks",
        required=True,
        help="comma-separated task ids assigned to the seen/training side",
    )
    manifest.add_argument(
        "--split-origin",
        required=True,
        help="versioned rule/artifact that produced the task-level split",
    )
    manifest.add_argument(
        "--split-frozen-before-outcomes",
        action="store_true",
        help="assert that the split was frozen before outcome inspection",
    )
    manifest.add_argument(
        "--contamination-review",
        required=True,
        help="bounded receipt for cross-split lineage/contamination review",
    )
    manifest.add_argument(
        "--overwrite",
        action="store_true",
        help="atomically replace an existing manifest after re-hashing every payload",
    )
    manifest.set_defaults(func=cmd_manifest)

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
