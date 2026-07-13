"""Verify an emitted contract JSON against the offline-harness schema.

This is a fail-closed pre-flight check the user can run before invoking
``pid-offline-harness`` on adapter output: it re-loads the JSON and checks the same
structural invariants the Rust harness enforces, plus the leakage-relevant
properties (train/held-out class coverage and episode disjointness) that the
harness's ``--require-*`` strict modes gate on.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from pathlib import Path

from .contract import (
    HELDOUT_SPLIT_TOKENS,
    TRAIN_SPLIT_TOKENS,
    load_dataset_json,
)


@dataclass
class ContractReport:
    n_samples: int = 0
    dims: dict[str, int] = field(default_factory=dict)
    train_success: int = 0
    train_failure: int = 0
    heldout_success: int = 0
    heldout_failure: int = 0
    shared_episode_ids: list[str] = field(default_factory=list)
    issues: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.issues

    def heldout_class_coverage_ok(self) -> bool:
        return (
            self.train_success > 0
            and self.train_failure > 0
            and self.heldout_success > 0
            and self.heldout_failure > 0
        )

    def episode_disjoint_ok(self) -> bool:
        return not self.shared_episode_ids


def verify_contract_obj(obj: dict) -> ContractReport:
    """Verify an already-loaded contract object."""
    report = ContractReport()
    if not isinstance(obj, dict):
        report.issues.append("top-level contract must be an object")
        return report
    for field_name in ("run_id", "source", "model", "task"):
        if field_name in obj and not isinstance(obj[field_name], str):
            report.issues.append(f"top-level {field_name!r} must be a string")
    samples = obj.get("samples")
    if not isinstance(samples, list) or not samples:
        report.issues.append("'samples' must be a non-empty array")
        return report
    report.n_samples = len(samples)
    if len(samples) < 8:
        # Mirrors the Rust harness's hard minimum (offline_harness.rs: "must
        # contain at least 8 samples").
        report.issues.append(
            f"dataset has {len(samples)} samples; the harness requires >= 8"
        )

    expected_dims: dict[str, int] | None = None
    seen_ids: set[str] = set()
    train_eps: set[str] = set()
    heldout_eps: set[str] = set()
    split_eligibility_blocked = False
    split_eligibility_statuses: set[str] = set()
    split_eligibility_marked_samples = 0

    for idx, sample in enumerate(samples):
        if not isinstance(sample, dict):
            report.issues.append(f"sample {idx}: must be an object")
            continue
        sid = sample.get("sample_id")
        if not isinstance(sid, str) or not sid:
            report.issues.append(f"sample {idx}: missing/empty sample_id")
        elif sid in seen_ids:
            report.issues.append(f"sample {idx}: duplicate sample_id {sid!r}")
        else:
            seen_ids.add(sid)

        dims: dict[str, int] = {}
        for key in ("v", "l", "d", "a"):
            values = sample.get(key)
            if not isinstance(values, list):
                dims[key] = 0
                report.issues.append(f"sample {idx}: {key!r} must be an array")
                continue
            dims[key] = len(values)
            if not values:
                report.issues.append(f"sample {idx}: empty or missing {key!r}")
            elif any(
                isinstance(x, bool)
                or not isinstance(x, (int, float))
                or not math.isfinite(float(x))
                for x in values
            ):
                report.issues.append(
                    f"sample {idx}: non-numeric/non-finite value in {key!r}"
                )
        if expected_dims is None:
            expected_dims = dims
            report.dims = dims
        elif dims != expected_dims:
            report.issues.append(
                f"sample {idx}: dims {dims} differ from first sample {expected_dims}"
            )

        labels = sample.get("labels", {})
        if not isinstance(labels, dict):
            report.issues.append(f"sample {idx}: labels must be an object")
            labels = {}
        success = labels.get("success")
        if success is not None and not isinstance(success, bool):
            report.issues.append(f"sample {idx}: labels.success must be boolean")
            success = None
        metadata = sample.get("metadata", {})
        if not isinstance(metadata, dict) or any(
            not isinstance(key, str) or not isinstance(value, str)
            for key, value in metadata.items()
        ):
            report.issues.append(f"sample {idx}: metadata must map strings to strings")
            metadata = {}
        # Mirror the Rust harness's split normalization exactly
        # (offline_harness.rs: trim + ASCII-lowercase + '-' -> '_').
        split = str(metadata.get("split", "")).strip().lower().replace("-", "_")
        if split not in {*TRAIN_SPLIT_TOKENS, *HELDOUT_SPLIT_TOKENS}:
            report.issues.append(
                f"sample {idx}: metadata.split must be a recognized train or held-out token"
            )
        eligibility = metadata.get("split_scientific_eligibility")
        if eligibility is not None:
            split_eligibility_marked_samples += 1
            split_eligibility_statuses.add(eligibility)
            if eligibility == "blocked_unfrozen_or_unreviewed":
                split_eligibility_blocked = True
            elif eligibility != "structural_split_ready":
                report.issues.append(
                    f"sample {idx}: unknown split scientific eligibility status"
                )
        episode_id = sample.get("episode_id")
        if not isinstance(episode_id, str) or not episode_id.strip():
            report.issues.append(f"sample {idx}: episode_id must be a non-empty string")
            episode_id = None

        if split in TRAIN_SPLIT_TOKENS:
            if episode_id is not None:
                train_eps.add(episode_id)
            if success is not None:
                report.train_success += int(success)
                report.train_failure += int(not success)
        elif split in HELDOUT_SPLIT_TOKENS:
            if episode_id is not None:
                heldout_eps.add(episode_id)
            if success is not None:
                report.heldout_success += int(success)
                report.heldout_failure += int(not success)

    report.shared_episode_ids = sorted(train_eps & heldout_eps)
    if not report.heldout_class_coverage_ok():
        report.issues.append(
            "train and held-out splits must each contain success and failure classes"
        )
    if report.shared_episode_ids:
        report.issues.append(
            f"episodes occur in both train and held-out splits: {report.shared_episode_ids}"
        )
    if split_eligibility_blocked:
        report.issues.append(
            "split scientific eligibility is blocked: unfrozen or contamination-unreviewed"
        )
    if len(split_eligibility_statuses) > 1:
        report.issues.append(
            "split scientific eligibility statuses are inconsistent across samples"
        )
    if 0 < split_eligibility_marked_samples < report.n_samples:
        report.issues.append(
            "split scientific eligibility must be stamped consistently on every sample"
        )
    return report


def verify_contract_file(path: str | Path) -> ContractReport:
    return verify_contract_obj(load_dataset_json(path))


def summarize(report: ContractReport) -> str:
    lines = [
        f"samples: {report.n_samples}",
        f"dims: {report.dims}",
        (
            "class coverage: "
            f"train(success={report.train_success}, failure={report.train_failure}) "
            f"heldout(success={report.heldout_success}, failure={report.heldout_failure}) "
            f"-> {'OK' if report.heldout_class_coverage_ok() else 'INCOMPLETE'}"
        ),
        (
            "episode disjointness: "
            f"{'OK' if report.episode_disjoint_ok() else 'LEAK'} "
            f"(shared={len(report.shared_episode_ids)})"
        ),
        f"contract valid: {'yes' if report.ok else 'NO'}",
    ]
    if report.issues:
        lines.append("issues:")
        lines.extend(f"  - {issue}" for issue in report.issues)
    return "\n".join(lines)
