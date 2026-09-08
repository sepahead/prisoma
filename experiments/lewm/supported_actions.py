"""Prepare bounded PushT action pools with an owned sklearn scaler.

This component does not execute actions or query a model. Explicit controls and
the private verified dataset transaction have separate row provenance. Existing
standardized-only and CEM contracts remain unchanged.
"""

from __future__ import annotations

import hashlib
import json
import re
import weakref

import numpy as np

from .assets import verify_runtime
from .contracts import StandardizedCandidates

MAX_FIT_ROWS = 1_000_000
EVAL_SOURCE_SHA256 = "9eb68eedc5dd5c22a61c5e17db49b0bbe799f98dae1e629e1f35d5191f2b9226"
POLICY_SOURCE_SHA256 = (
    "4967e7e3d5b20eb7a1d0b00e5d60fd701cce1c750ae7ec4a9b02529b9366db22"
)
_SCALERS: weakref.WeakSet = weakref.WeakSet()


def _json(value: dict) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def _sha(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _record(array: np.ndarray) -> dict:
    return {
        "dtype": array.dtype.str,
        "shape": list(array.shape),
        "sha256": _sha(array.tobytes()),
    }


def _snapshot(value: np.ndarray, shapes: tuple, formats: tuple) -> np.ndarray:
    """Check extent before copying; validate the retained immutable bytes later."""
    if type(value) is not np.ndarray:
        raise ValueError("An exact ndarray is required")
    view = memoryview(value)
    if view.shape not in shapes or view.format not in formats:
        raise ValueError("Array shape or native floating-point dtype is unsupported")
    dtype = {"f": np.float32, "d": np.float64}[view.format]
    return np.frombuffer(view.tobytes(), dtype=dtype).reshape(view.shape)


def _sklearn():
    # The default dependency graph imports neither sklearn nor Torch.
    runtime = verify_runtime()
    from sklearn.preprocessing import StandardScaler

    return StandardScaler, runtime


class FittedActionScaler:
    """Immutable local issuer handle, not a receipt-based authority constructor.

    Private Python attributes are not malicious-code isolation. Inspection returns
    immutable arrays or independent dictionaries; no fitted sklearn object escapes.
    """

    __slots__ = ("_state", "_receipt", "_mask", "_rows", "__weakref__")

    def __init__(self):
        raise TypeError("A fitted scaler must come from an owned row issuer")

    def __setattr__(self, name, value):
        raise AttributeError("Fitted scaler state is immutable")

    @property
    def statistics(self) -> dict[str, np.ndarray]:
        return {
            name: np.frombuffer(data, dtype=np.float64)
            for name, data in zip(("mean", "variance", "scale"), self._state)
        }

    @property
    def retained_row_mask(self) -> np.ndarray:
        return np.frombuffer(self._mask, dtype=np.bool_)

    @property
    def fit_rows(self) -> np.ndarray:
        """Exact original rows, including NaNs, in their retained input order."""
        data, dtype, shape = self._rows
        return np.frombuffer(data, dtype=dtype).reshape(shape)

    @property
    def sha256(self) -> str:
        return _sha(self._receipt.encode())

    def receipt(self) -> dict:
        return json.loads(self._receipt)


def fit_control_scaler(rows: np.ndarray, *, source_id: str) -> FittedActionScaler:
    """Fit actual pinned sklearn on explicit control rows; grant no dataset authority.

    Complete-row NaN exclusion follows pinned eval.py. Infinity anywhere rejects,
    including an otherwise excluded row. This stricter rule is recorded explicitly.
    """
    if (
        not isinstance(source_id, str)
        or re.fullmatch(r"[a-zA-Z0-9_-]{1,64}", source_id) is None
    ):
        raise ValueError("A bounded control source identifier is required")
    return _fit_owned_rows(
        rows,
        {
            "schema": "prisoma.lewm.action-scaler-control.v1",
            "scope": "synthetic_control_only",
            "source_id": source_id,
            "row_order": "input_order",
        },
    )


def _fit_owned_rows(rows: np.ndarray, provenance: dict) -> FittedActionScaler:
    """Private trusted issuer seam; public receipt or array inputs grant no dataset scope.

    Only the dataset transaction supplies dataset provenance. This helper does not
    isolate malicious Python code or turn asserted hashes into verified inputs.
    """
    if type(rows) is not np.ndarray:
        raise ValueError("An exact ndarray is required")
    shape = memoryview(rows).shape
    if len(shape) != 2 or shape[1] != 2 or not 2 <= shape[0] <= MAX_FIT_ROWS:
        raise ValueError("Fit rows must have bounded shape [N,2] with N >= 2")
    data = _snapshot(rows, (shape,), ("f", "d"))
    if np.isinf(data).any():
        raise ValueError("Infinite fit values are not admissible")
    retained = ~np.isnan(data).any(axis=1)
    if int(retained.sum()) < 2:
        raise ValueError("At least two complete finite fit rows are required")
    scaler_type, runtime = _sklearn()
    fitted = scaler_type().fit(data[retained])
    arrays = (fitted.mean_, fitted.var_, fitted.scale_)
    if any(a.shape != (2,) or not np.isfinite(a).all() for a in arrays):
        raise ValueError("Fitted scaler statistics are not finite two-axis values")
    if (fitted.var_ < 0).any() or (fitted.scale_ <= 0).any():
        raise ValueError("Fitted variance or scale is invalid")
    if int(fitted.n_samples_seen_) != int(retained.sum()):
        raise ValueError("Fitted row count does not match retained rows")
    receipt = {
        **provenance,
        "rows": _record(data),
        "retained_mask": _record(retained),
        "retained_rows": int(retained.sum()),
        "excluded_nan_rows": int((~retained).sum()),
        "infinity_policy": "reject_any_including_nan_excluded_rows",
        "recipe": "sklearn-standard-scaler-default-fit-population-variance",
        "eval_source_sha256": EVAL_SOURCE_SHA256,
        "runtime_observation": runtime,
        "statistics": {
            name: _record(a) for name, a in zip(("mean", "variance", "scale"), arrays)
        },
        "n_samples_seen": int(fitted.n_samples_seen_),
        "n_features_in": int(fitted.n_features_in_),
    }
    owner = object.__new__(FittedActionScaler)
    object.__setattr__(
        owner, "_state", tuple(a.astype(np.float64).tobytes() for a in arrays)
    )
    object.__setattr__(owner, "_mask", retained.tobytes())
    object.__setattr__(owner, "_rows", (data.tobytes(), data.dtype.str, data.shape))
    object.__setattr__(owner, "_receipt", _json(receipt))
    _SCALERS.add(owner)
    return owner


class SupportedPushTCandidates:
    """Immutable legal conversion data; no model, dispatch, or dataset authority."""

    __slots__ = ("_arrays", "_ids", "_receipt")

    def __init__(self):
        raise TypeError("Use prepare_candidates with an owned fitted scaler")

    def __setattr__(self, name, value):
        raise AttributeError("Prepared candidates are immutable")

    @property
    def proposed(self) -> np.ndarray:
        return np.frombuffer(self._arrays[0], dtype=np.float32).reshape(
            len(self._ids), 25, 2
        )

    @property
    def executable(self) -> np.ndarray:
        return np.frombuffer(self._arrays[2], dtype=np.float32).reshape(
            len(self._ids), 25, 2
        )

    @property
    def standardized(self) -> StandardizedCandidates:
        values = np.frombuffer(self._arrays[1], dtype=np.float32).reshape(
            1, len(self._ids), 5, 10
        )
        return StandardizedCandidates(self._ids, values)

    @property
    def sha256(self) -> str:
        return _sha(self._receipt.encode())

    def receipt(self) -> dict:
        return json.loads(self._receipt)


def prepare_candidates(
    scaler: FittedActionScaler, ids: tuple[str, ...], proposed: np.ndarray
) -> SupportedPushTCandidates:
    """Use sklearn transform/inverse_transform, retaining all three representations.

    Twenty-five dense two-axis commands map to five ten-coordinate model blocks.
    The exact inverse-transformed float32 commands must remain inside [-1,1]^2.
    No clipping or tolerance expands that support. Round-trip differences remain
    recorded; the original proposal is never relabeled as the execution array.
    """
    if type(scaler) is not FittedActionScaler or scaler not in _SCALERS:
        raise ValueError("An owner-issued fitted scaler is required")
    if type(ids) is not tuple or not 2 <= len(ids) <= 300:
        raise ValueError("Candidate IDs require a bounded tuple")
    if any(
        not isinstance(i, str) or re.fullmatch(r"[a-zA-Z0-9_-]{1,64}", i) is None
        for i in ids
    ):
        raise ValueError("Candidate identifiers are invalid")
    if len(set(ids)) != len(ids):
        raise ValueError("Candidate identifiers must be unique")
    raw = _snapshot(proposed, ((len(ids), 25, 2),), ("f",))
    if not np.isfinite(raw).all() or (np.abs(raw) > 1).any():
        raise ValueError("Raw proposals must be finite and inside [-1,1]")
    scaler_type, runtime = _sklearn()
    receipt = scaler.receipt()
    if receipt["runtime_observation"] != runtime:
        raise ValueError("Scaler runtime identity changed")
    # Restore exact fitted attributes into the actual sklearn implementation.
    # No NumPy normalization substitute or caller-supplied scaler is admitted.
    fitted = scaler_type()
    statistics = scaler.statistics
    fitted.mean_ = statistics["mean"]
    fitted.var_ = statistics["variance"]
    fitted.scale_ = statistics["scale"]
    fitted.n_features_in_ = receipt["n_features_in"]
    fitted.n_samples_seen_ = np.int64(receipt["n_samples_seen"])
    transformed = fitted.transform(raw.reshape(-1, 2))
    standardized = StandardizedCandidates(ids, transformed.reshape(1, len(ids), 5, 10))
    execution = fitted.inverse_transform(standardized.values.reshape(-1, 2)).reshape(
        raw.shape
    )
    execution = _snapshot(execution, (raw.shape,), ("f",))
    if not np.isfinite(execution).all() or (np.abs(execution) > 1).any():
        raise ValueError("Inverse-transformed commands exceed finite raw support")
    record = {
        "schema": "prisoma.lewm.supported-action-preparation.v1",
        "scope": "conversion_control_only_no_execution_authority",
        "scaler_sha256": scaler.sha256,
        "scaler_scope": receipt["scope"],
        "candidate_ids": list(ids),
        "policy_source_sha256": POLICY_SOURCE_SHA256,
        "proposed": _record(raw),
        "standardized": _record(standardized.values),
        "executable": _record(execution),
        "primitive_order": "j -> block=j//5, slots=2*(j%5)+coordinate",
        "control_period_seconds": 0.1,
        "primitive_count": 25,
        "raw_units": "dimensionless_relative_offset_times_100_workspace_units",
        "roundtrip_max_abs": float(np.max(np.abs(execution.astype(np.float64) - raw))),
    }
    owner = object.__new__(SupportedPushTCandidates)
    for key, value in (
        ("_arrays", tuple(a.tobytes() for a in (raw, standardized.values, execution))),
        ("_ids", ids),
        ("_receipt", _json(record)),
    ):
        object.__setattr__(owner, key, value)
    return owner
