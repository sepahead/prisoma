"""Verify an emitted contract JSON against the offline-harness schema.

This is a fail-closed pre-flight check the user can run before invoking
``pid-offline-harness`` on adapter output: it re-loads the JSON and checks the same
structural invariants the Rust harness enforces, plus the leakage-relevant
properties (train/held-out class coverage and episode disjointness) that the
harness's ``--require-*`` strict modes gate on.
"""

from __future__ import annotations

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
    samples = obj.get("samples")
    if not isinstance(samples, list) or not samples:
        report.issues.append("'samples' must be a non-empty array")
        return report
    report.n_samples = len(samples)

    expected_dims: dict[str, int] | None = None
    seen_ids: set[str] = set()
    train_eps: set[str] = set()
    heldout_eps: set[str] = set()

    for idx, sample in enumerate(samples):
        sid = sample.get("sample_id")
        if not sid:
            report.issues.append(f"sample {idx}: missing/empty sample_id")
        elif sid in seen_ids:
            report.issues.append(f"sample {idx}: duplicate sample_id {sid!r}")
        else:
            seen_ids.add(sid)

        dims = {key: len(sample.get(key, [])) for key in ("v", "l", "d", "a")}
        for key, n in dims.items():
            if n == 0:
                report.issues.append(f"sample {idx}: empty or missing {key!r}")
        if expected_dims is None:
            expected_dims = dims
            report.dims = dims
        elif dims != expected_dims:
            report.issues.append(
                f"sample {idx}: dims {dims} differ from first sample {expected_dims}"
            )

        labels = sample.get("labels") or {}
        success = labels.get("success")
        metadata = sample.get("metadata") or {}
        split = str(metadata.get("split", "")).lower()
        episode_id = sample.get("episode_id")

        if success is None:
            continue
        if split in TRAIN_SPLIT_TOKENS:
            report.train_success += int(bool(success))
            report.train_failure += int(not success)
            if episode_id is not None:
                train_eps.add(episode_id)
        elif split in HELDOUT_SPLIT_TOKENS:
            report.heldout_success += int(bool(success))
            report.heldout_failure += int(not success)
            if episode_id is not None:
                heldout_eps.add(episode_id)

    report.shared_episode_ids = sorted(train_eps & heldout_eps)
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
