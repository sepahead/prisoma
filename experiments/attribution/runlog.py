"""Emit ``attribution_logged`` run-log events conformant to the Rust schema.

The Rust ``RunLogEvent::AttributionLogged`` variant
(``crates/pid-runlog/src/lib.rs``) already exists; this module writes a canonical
JSONL run log (``run_started`` / ``config_logged`` / one ``attribution_logged`` per
probe / ``run_ended``) plus the attribution arrays as artifact files with sha256
provenance, so the result passes ``pid-runlog-replay --validate``.

Hash compatibility: the validator recomputes ``sha256(serde_json::to_vec(config))``
and checks it equals the logged ``config_hash``. serde_json (no ``preserve_order``)
serializes objects with **sorted keys, compact separators**, which
``json.dumps(..., sort_keys=True, separators=(",", ":"))`` reproduces — *provided
the config contains only strings / ints / bools* (we never put floats in the config,
to avoid float-formatting divergence). The same canonical hash is used for
``run_started`` and ``config_logged`` so they agree.
"""

from __future__ import annotations

import hashlib
import json
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

import numpy as np

SCHEMA_VERSION = 1


def canonical_hash(value: object) -> str:
    """Reproduce ``pid_runlog::canonical_json_hash`` for str/int/bool-only values."""
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: str | Path) -> str:
    h = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for chunk in iter(lambda: handle.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


@dataclass
class AttributionRecord:
    """One attribution result to log (mirrors the Rust event fields)."""

    method: str
    target_output: str
    relevance: np.ndarray
    faithfulness_passed: bool
    layer: str | None = None
    modality: str | None = None
    baseline: str | None = None
    metadata: dict[str, str] | None = None

    def __post_init__(self) -> None:
        if not self.method:
            raise ValueError("method must be non-empty (the harness validator rejects it)")
        if not self.target_output:
            raise ValueError("target_output must be non-empty")


def write_attribution_runlog(
    runlog_path: str | Path,
    records: Sequence[AttributionRecord],
    *,
    run_id: str = "attribution-probe",
    config: dict | None = None,
    artifact_dir: str | Path | None = None,
) -> Path:
    """Write a canonical run log for a batch of attribution records.

    Each record's ``relevance`` array is saved as a ``.npy`` artifact (when
    ``artifact_dir`` is given) and referenced by ``artifact_uri`` with an
    ``artifact_sha256`` in metadata; a ``score_hash`` over the rounded relevance is
    always recorded so identical attributions are detectable without the file.
    """
    runlog_path = Path(runlog_path)
    runlog_path.parent.mkdir(parents=True, exist_ok=True)
    if artifact_dir is not None:
        artifact_dir = Path(artifact_dir)
        artifact_dir.mkdir(parents=True, exist_ok=True)

    config = dict(config or {})
    config.setdefault("experiment", "attribution_probe")
    config.setdefault("n_records", str(len(records)))
    config_hash = canonical_hash(config)

    events: list[dict] = []
    events.append(
        {
            "type": "run_started",
            "schema_version": SCHEMA_VERSION,
            "run_id": run_id,
            "timestamp_ns": 0,
            "config_hash": config_hash,
            "metadata": {"source": "attribution-probe"},
        }
    )
    events.append(
        {
            "type": "config_logged",
            "timestamp_ns": 0,
            "config_hash": config_hash,
            "config": config,
        }
    )

    ts = 1
    for i, rec in enumerate(records):
        rel = np.asarray(rec.relevance, dtype=np.float64)
        # Stable score hash over the rounded relevance (order/precision-stable).
        score_hash = hashlib.sha256(
            np.round(rel, 8).tobytes() + rel.shape.__repr__().encode()
        ).hexdigest()
        metadata = {k: str(v) for k, v in (rec.metadata or {}).items()}
        metadata["relevance_shape"] = "x".join(str(n) for n in rel.shape)

        artifact_uri = None
        if artifact_dir is not None:
            artifact_path = artifact_dir / f"{run_id}_{rec.method}_{i}.npy"
            np.save(artifact_path, rel)
            artifact_uri = str(artifact_path)
            metadata["artifact_sha256"] = sha256_file(artifact_path)

        events.append(
            {
                "type": "attribution_logged",
                "timestamp_ns": ts,
                "method": rec.method,
                "target_output": rec.target_output,
                "layer": rec.layer,
                "modality": rec.modality,
                "baseline": rec.baseline,
                "score_hash": score_hash,
                "faithfulness_check": bool(rec.faithfulness_passed),
                "artifact_uri": artifact_uri,
                "metadata": metadata,
            }
        )
        ts += 1

    events.append(
        {
            "type": "run_ended",
            "run_id": run_id,
            "timestamp_ns": ts,
            "status": "succeeded",
            "message": f"logged {len(records)} attribution record(s)",
        }
    )

    with runlog_path.open("w") as handle:
        for event in events:
            handle.write(json.dumps(event))
            handle.write("\n")
    return runlog_path
