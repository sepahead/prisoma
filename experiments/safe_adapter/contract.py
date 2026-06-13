"""The ``(V, L, D, A)`` + labels contract consumed by ``pid-offline-harness``.

This mirrors the Rust ``OfflineVldaSample`` / ``OfflineVldaDataset`` schema in
``crates/pid-sim/src/offline_harness.rs`` exactly, so a dataset emitted here can be
fed straight into the offline harness:

* the top-level object has optional ``run_id`` / ``source`` / ``model`` / ``task``
  strings and a required ``samples`` array;
* each sample carries a non-empty ``sample_id``, an optional ``episode_id``,
  numeric ``v`` / ``l`` / ``d`` / ``a`` vectors of *fixed per-variable length*
  across the dataset, an optional ``labels`` object (we always set a boolean
  ``success``), and an optional flat string ``metadata`` map (we set
  ``metadata.split`` to a recognised train/held-out token plus per-variable
  provenance).

The dataclasses here are intentionally dependency-light (stdlib + the float
coercion below) so the adapter runs without the compiled extension.
"""

from __future__ import annotations

import json
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path

# Recognised metadata.split tokens, matching the Rust harness's accepted values.
TRAIN_SPLIT_TOKENS = ("train", "training")
HELDOUT_SPLIT_TOKENS = (
    "test",
    "validation",
    "val",
    "eval",
    "evaluation",
    "heldout",
    "holdout",
    "held_out",
    "hold_out",
)


def _coerce_vector(name: str, values: Sequence[float]) -> list[float]:
    out: list[float] = []
    for v in values:
        fv = float(v)
        # The harness rejects non-finite embedding values; fail loudly here.
        if fv != fv or fv in (float("inf"), float("-inf")):
            raise ValueError(f"{name} contains a non-finite value: {v!r}")
        out.append(fv)
    if not out:
        raise ValueError(f"{name} must be non-empty")
    return out


@dataclass
class VldaSample:
    """One ``(V, L, D, A)`` sample with a success label and provenance."""

    sample_id: str
    v: list[float]
    l: list[float]
    d: list[float]
    a: list[float]
    success: bool
    episode_id: str | None = None
    metadata: dict[str, str] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not self.sample_id:
            raise ValueError("sample_id must be non-empty")
        self.v = _coerce_vector("v", self.v)
        self.l = _coerce_vector("l", self.l)
        self.d = _coerce_vector("d", self.d)
        self.a = _coerce_vector("a", self.a)
        self.success = bool(self.success)

    def to_json(self) -> dict:
        obj: dict = {
            "sample_id": self.sample_id,
            "v": self.v,
            "l": self.l,
            "d": self.d,
            "a": self.a,
            "labels": {"success": self.success},
        }
        if self.episode_id is not None:
            obj["episode_id"] = self.episode_id
        if self.metadata:
            # The harness metadata map is string -> string.
            obj["metadata"] = {str(k): str(val) for k, val in self.metadata.items()}
        return obj


@dataclass
class VldaDataset:
    """A full ``(V, L, D, A)`` dataset ready for the offline harness."""

    samples: list[VldaSample]
    run_id: str | None = None
    source: str | None = None
    model: str | None = None
    task: str | None = None

    def dims(self) -> dict[str, int]:
        if not self.samples:
            raise ValueError("dataset has no samples")
        first = self.samples[0]
        return {
            "v": len(first.v),
            "l": len(first.l),
            "d": len(first.d),
            "a": len(first.a),
        }

    def validate(self) -> list[str]:
        """Return a list of contract violations (empty == valid).

        Mirrors the structural checks the Rust harness performs: non-empty,
        unique sample ids, and *fixed per-variable dimensionality* across all
        samples (the harness builds matrices and would reject ragged rows).
        """
        issues: list[str] = []
        if not self.samples:
            issues.append("dataset must contain at least one sample")
            return issues
        dims = self.dims()
        seen_ids: set[str] = set()
        for idx, s in enumerate(self.samples):
            if not s.sample_id:
                issues.append(f"sample {idx}: empty sample_id")
            if s.sample_id in seen_ids:
                issues.append(f"sample {idx}: duplicate sample_id {s.sample_id!r}")
            seen_ids.add(s.sample_id)
            for key, vec in (("v", s.v), ("l", s.l), ("d", s.d), ("a", s.a)):
                if len(vec) != dims[key]:
                    issues.append(
                        f"sample {idx} ({s.sample_id!r}): {key} has length "
                        f"{len(vec)}, expected {dims[key]}"
                    )
        return issues

    def to_json(self) -> dict:
        obj: dict = {"samples": [s.to_json() for s in self.samples]}
        for key in ("run_id", "source", "model", "task"):
            value = getattr(self, key)
            if value is not None:
                obj[key] = value
        return obj

    def write_json(self, path: str | Path) -> Path:
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        issues = self.validate()
        if issues:
            raise ValueError(
                "refusing to write an invalid dataset:\n  " + "\n  ".join(issues)
            )
        path.write_text(json.dumps(self.to_json(), indent=2))
        return path


def split_token(seen: bool, *, train_if_seen: bool = True) -> str:
    """Map a SAFE seen/unseen flag to a recognised harness split token.

    SAFE evaluates zero-shot failure detection on *unseen* tasks, so the natural
    leakage-safe mapping is: seen tasks -> train, unseen tasks -> held-out.
    """
    is_train = seen if train_if_seen else not seen
    return TRAIN_SPLIT_TOKENS[0] if is_train else HELDOUT_SPLIT_TOKENS[0]


def load_dataset_json(path: str | Path) -> dict:
    """Load a previously written contract JSON (for round-trip checks)."""
    return json.loads(Path(path).read_text())


def is_mapping(obj: object) -> bool:
    return isinstance(obj, Mapping)
