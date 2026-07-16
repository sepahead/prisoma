"""Emit ``attribution_logged`` run-log events conformant to the Rust schema.

The Rust ``RunLogEvent::AttributionLogged`` variant
(``pid-rs/crates/pid-runlog/src/lib.rs``) already exists; this module writes a canonical
JSONL run log (``run_started`` / ``config_logged`` / one ``attribution_logged`` per
probe / ``run_ended``) plus the attribution arrays as artifact files with sha256
provenance, so the result passes ``pid-runlog-replay --validate``.

Hash compatibility: the validator recomputes ``sha256(serde_json::to_vec(config))``
and checks it equals the logged ``config_hash``. serde_json (no ``preserve_order``)
serializes objects with **sorted keys, compact separators**, which
``json.dumps(..., sort_keys=True, separators=(",", ":"))`` reproduces — *provided
the config is composed only of objects, lists, strings, integers, and booleans* (we
never put floats or nulls in the config, to avoid cross-language formatting ambiguity).
The same canonical hash is used for
``run_started`` and ``config_logged`` so they agree.
"""

from __future__ import annotations

import base64
import hashlib
import io
import json
import math
import os
import platform
import re
import stat
import tempfile
import unicodedata
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path

import numpy as np

SCHEMA_VERSION = 2
MAX_RERUN_RELEVANCE_VALUES = 1024
MAX_RERUN_RELEVANCE_FILE_BYTES = 12 * 1024
MAX_RERUN_PREPARED_ARTIFACT_BYTES = 8 * 1024 * 1024
MAX_RERUN_EVENTS = 100_000
MAX_RERUN_SERIALIZED_EVENT_BYTES = 64 * 1024 * 1024
MAX_EVIDENCE_RECOMPUTE_MULTIPLY_ADDS = 1_000_000_000

# Keep the producer inside pid-runlog 0.9's default bounded-reader contract so
# every emitted line can be consumed by the canonical validator. These are
# intentionally duplicated here because the Python producer cannot import Rust
# constants at runtime; focused tests pin the cross-language boundary behavior.
MAX_RUNLOG_FILE_BYTES = 256 * 1024 * 1024
MAX_RUNLOG_LINE_BYTES = 4 * 1024 * 1024
MAX_RUNLOG_STRING_BYTES = 1024 * 1024
MAX_RUNLOG_ARRAY_LEN = 1_000_000
MAX_RUNLOG_OBJECT_ENTRIES = 100_000
MAX_RUNLOG_NESTING_DEPTH = 64


def _canonical_run_id(run_id: object) -> str:
    """Return a portable, normalization-stable run identifier."""
    if type(run_id) is not str or not run_id:
        raise ValueError("run_id must be a non-empty string")
    if run_id != run_id.strip():
        raise ValueError("run_id must not have leading or trailing whitespace")
    if unicodedata.normalize("NFC", run_id) != run_id:
        raise ValueError("run_id must use canonical NFC Unicode normalization")
    if any(ord(character) < 0x20 or ord(character) == 0x7F for character in run_id):
        raise ValueError("run_id must not contain control characters")
    return run_id


def _validate_canonical_value(
    value: object,
    path: str = "$",
    *,
    _depth: int = 0,
    _active_containers: set[int] | None = None,
) -> None:
    """Restrict cross-language hashes to JSON forms with identical number text."""
    if type(value) in (dict, list):
        depth = _depth + 1
        if depth > MAX_RUNLOG_NESTING_DEPTH:
            raise ValueError(
                "canonical JSON nesting exceeds the run-log limit "
                f"{MAX_RUNLOG_NESTING_DEPTH} at {path}"
            )
        active = _active_containers if _active_containers is not None else set()
        identity = id(value)
        if identity in active:
            raise ValueError(f"canonical JSON value at {path} contains a cycle")
        active.add(identity)
    else:
        depth = _depth
        active = _active_containers

    if type(value) is dict:
        if len(value) > MAX_RUNLOG_OBJECT_ENTRIES:
            raise ValueError(
                "canonical JSON object exceeds the run-log entry limit "
                f"{MAX_RUNLOG_OBJECT_ENTRIES} at {path}"
            )
        try:
            for key, child in value.items():
                if type(key) is not str:
                    raise ValueError(
                        f"canonical JSON object key at {path} must be a string"
                    )
                _validate_canonical_value(
                    child,
                    f"{path}.{key}",
                    _depth=depth,
                    _active_containers=active,
                )
        finally:
            active.remove(id(value))
        return
    if type(value) is list:
        if len(value) > MAX_RUNLOG_ARRAY_LEN:
            raise ValueError(
                "canonical JSON array exceeds the run-log length limit "
                f"{MAX_RUNLOG_ARRAY_LEN} at {path}"
            )
        try:
            for index, child in enumerate(value):
                _validate_canonical_value(
                    child,
                    f"{path}[{index}]",
                    _depth=depth,
                    _active_containers=active,
                )
        finally:
            active.remove(id(value))
        return
    if type(value) is int:
        if value < -(2**63) or value > 2**64 - 1:
            raise ValueError(
                f"canonical JSON value integer at {path} is outside serde_json's exact range"
            )
        return
    if type(value) in (str, bool):
        return
    raise ValueError(
        f"canonical JSON value at {path} must be a string, integer, boolean, list, or object"
    )


def canonical_hash(value: object) -> str:
    """Reproduce the Rust canonical hash for JSON built from exact shared forms."""
    # ``ensure_ascii=False`` is REQUIRED: serde_json writes raw UTF-8, while
    # Python's default escapes non-ASCII to ``\uXXXX`` — so a config value such as
    # ``I^sx_∩`` (which the docset uses freely) would otherwise hash differently
    # in Python than in Rust and fail ``pid-runlog-replay --validate``.
    _validate_canonical_value(value)
    payload = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def sha256_file(path: str | Path) -> str:
    h = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for chunk in iter(lambda: handle.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


_PRODUCER_METADATA_KEYS = frozenset(
    {
        "artifact_sha256",
        "relevance_shape",
        "score_hash_encoding",
        "evidence_bundle_sha256",
        "evidence_bundle_uri",
    }
)
_SHA256_RE = re.compile(r"[0-9a-f]{64}")


def _validated_record_metadata(metadata: object, record_index: int) -> dict[str, str]:
    """Preserve metadata exactly; never coerce keys or values through ``str``."""
    if metadata is None:
        return {}
    if type(metadata) is not dict:
        raise ValueError(f"record {record_index} metadata must be a plain dictionary")

    normalized_keys: dict[str, str] = {}
    result: dict[str, str] = {}
    for key, value in metadata.items():
        if type(key) is not str or type(value) is not str:
            raise ValueError(
                f"record {record_index} metadata keys and values must be exact strings"
            )
        normalized = unicodedata.normalize("NFC", key)
        prior = normalized_keys.get(normalized)
        if prior is not None and prior != key:
            raise ValueError(
                f"record {record_index} metadata contains a Unicode normalization collision"
            )
        if normalized != key:
            raise ValueError(
                f"record {record_index} metadata keys must use canonical NFC normalization"
            )
        if key in _PRODUCER_METADATA_KEYS:
            raise ValueError(
                f"record {record_index} metadata key {key!r} is reserved by the producer"
            )
        normalized_keys[normalized] = key
        result[key] = value
    return result


def _require_sha256(value: object, context: str) -> str:
    if type(value) is not str or _SHA256_RE.fullmatch(value) is None:
        raise ValueError(f"{context} must be a lowercase SHA-256 digest")
    return value


def _decode_exact_f64_bundle(
    value: object, context: str, *, max_values: int
) -> tuple[tuple[int, ...], bytes]:
    if type(value) is not dict:
        raise ValueError(f"{context} must be an exact-array object")
    if set(value) != {"dtype", "shape", "data_base64"} or value.get("dtype") != "<f8":
        raise ValueError(f"{context} must use the exact <f8 array-bundle schema")
    shape = value.get("shape")
    if type(shape) is not list or not shape:
        raise ValueError(f"{context}.shape must be a nonempty integer list")
    dimensions: list[int] = []
    values = 1
    for index, dimension in enumerate(shape):
        if (
            isinstance(dimension, bool)
            or not isinstance(dimension, int)
            or dimension <= 0
        ):
            raise ValueError(f"{context}.shape[{index}] must be a positive integer")
        values *= dimension
        if values > max_values:
            raise ValueError(f"{context} exceeds the {max_values}-value evidence limit")
        dimensions.append(dimension)
    encoded = value.get("data_base64")
    if type(encoded) is not str:
        raise ValueError(f"{context}.data_base64 must be a string")
    try:
        payload = base64.b64decode(encoded, validate=True)
    except (ValueError, TypeError) as error:
        raise ValueError(f"{context}.data_base64 is not canonical base64") from error
    if len(payload) != values * 8:
        raise ValueError(f"{context} byte length does not match its declared shape")
    array = np.frombuffer(payload, dtype="<f8")
    if not np.isfinite(array).all():
        raise ValueError(f"{context} must contain only finite float64 values")
    return tuple(dimensions), payload


def _require_bounded_int(
    value: object, context: str, *, minimum: int, maximum: int
) -> int:
    if type(value) is not int or not minimum <= value <= maximum:
        raise ValueError(
            f"{context} must be an integer from {minimum} through {maximum}"
        )
    return value


def _require_evidence_text(value: object, context: str) -> str:
    if type(value) is not str or not value or value != value.strip():
        raise ValueError(
            f"{context} must be a nonempty string without outer whitespace"
        )
    if unicodedata.normalize("NFC", value) != value:
        raise ValueError(f"{context} must use canonical NFC normalization")
    if any(ord(character) < 0x20 or ord(character) == 0x7F for character in value):
        raise ValueError(f"{context} must not contain control characters")
    return value


def _require_canonical_f64_text(value: object, context: str) -> str:
    text = _require_evidence_text(value, context)
    try:
        parsed = float(text)
    except ValueError as error:
        raise ValueError(f"{context} must encode one finite float64") from error
    if not math.isfinite(parsed) or format(parsed, ".17g") != text:
        raise ValueError(f"{context} must use canonical finite float64 text")
    return text


def _require_optional_f64_text(value: object, context: str) -> str:
    if value == "not_computed":
        return "not_computed"
    return _require_canonical_f64_text(value, context)


def _evidence_case_diagnostic_vector(
    cases: list[object], field: str
) -> tuple[float, ...]:
    values: list[float] = []
    not_computed = 0
    for index, case in enumerate(cases):
        assert isinstance(case, dict)
        encoded = case.get(field)
        if encoded == "not_computed":
            not_computed += 1
            continue
        if type(encoded) is not str:
            raise ValueError(
                f"evidence bundle case {index}.{field} must be canonical float hex "
                "or not_computed"
            )
        try:
            value = float.fromhex(encoded)
        except ValueError as error:
            raise ValueError(
                f"evidence bundle case {index}.{field} is not float64 hex"
            ) from error
        if not math.isfinite(value) or value.hex() != encoded:
            raise ValueError(
                f"evidence bundle case {index}.{field} must use canonical finite "
                "float64 hex"
            )
        values.append(value)
    if not_computed not in {0, len(cases)}:
        raise ValueError(
            f"evidence bundle case {field} values must be all computed or all absent"
        )
    return tuple(values)


def _recompute_model_parameter_hash(model: object) -> str:
    if type(model) is not dict:
        raise ValueError("evidence bundle model must be an object")
    expected_keys = {"kind", "d_in", "d_model", "parameter_sha256", "parameters"}
    if set(model) != expected_keys or model.get("kind") != "small_transformer_v1":
        raise ValueError(
            "evidence bundle model does not use the exact reference schema"
        )
    d_in = _require_bounded_int(
        model.get("d_in"),
        "evidence bundle model.d_in",
        minimum=1,
        maximum=1024,
    )
    d_model = _require_bounded_int(
        model.get("d_model"),
        "evidence bundle model.d_model",
        minimum=1,
        maximum=512,
    )
    parameter_shapes = {
        "w_embed": (d_in, d_model),
        "w_q": (d_model, d_model),
        "w_k": (d_model, d_model),
        "w_v": (d_model, d_model),
        "w_o": (d_model, d_model),
        "w_head": (d_model, 1),
    }
    if sum(np.prod(shape) for shape in parameter_shapes.values()) > 2_000_000:
        raise ValueError("evidence bundle model exceeds the reference parameter budget")
    parameters = model.get("parameters")
    if type(parameters) is not dict or set(parameters) != set(parameter_shapes):
        raise ValueError(
            "evidence bundle model parameters are incomplete or unexpected"
        )

    digest = hashlib.sha256(b"prisoma-small-transformer-parameters-v1\0")
    digest.update(d_in.to_bytes(8, "little"))
    digest.update(d_model.to_bytes(8, "little"))
    for name, expected_shape in parameter_shapes.items():
        shape, payload = _decode_exact_f64_bundle(
            parameters[name],
            f"evidence bundle model.parameters.{name}",
            max_values=2_000_000,
        )
        if shape != expected_shape:
            raise ValueError(
                f"evidence bundle model parameter {name} has the wrong shape"
            )
        encoded_name = name.encode("ascii")
        digest.update(len(encoded_name).to_bytes(8, "little"))
        digest.update(encoded_name)
        digest.update(len(shape).to_bytes(8, "little"))
        for dimension in shape:
            digest.update(dimension.to_bytes(8, "little"))
        digest.update(payload)
    return digest.hexdigest()


def _recompute_case_commitments(
    cases: object, gate: dict[str, object]
) -> tuple[
    str,
    str,
    str,
    tuple[int, ...],
    bytes,
    tuple[tuple[int, ...], ...],
]:
    if type(cases) is not list or not cases:
        raise ValueError("evidence bundle cases must be a nonempty list")
    if len(cases) > 1024:
        raise ValueError("evidence bundle cases exceed the 1024-case limit")

    case_digest = hashlib.sha256()
    relevance_digest = hashlib.sha256()
    case_ids: set[str] = set()
    group_ids: set[str] = set()
    unit_ids_seen: set[str] = set()
    first_case_id = ""
    first_relevance_shape: tuple[int, ...] = ()
    first_relevance_payload = b""
    case_shapes: list[tuple[int, ...]] = []

    def update_length_prefixed(digest: object, value: bytes) -> None:
        digest.update(len(value).to_bytes(8, "little"))
        digest.update(value)

    for index, case in enumerate(cases):
        if type(case) is not dict:
            raise ValueError("evidence bundle cases must contain objects")
        required = {
            "case_id",
            "group_id",
            "unit_ids",
            "x",
            "baseline",
            "relevance",
            "group_contrast_f64",
            "group_randomization_p_f64",
        }
        if set(case) != required:
            raise ValueError(
                f"evidence bundle case {index} has incomplete or unexpected fields"
            )
        case_id = _require_evidence_text(
            case.get("case_id"), f"evidence bundle case {index}.case_id"
        )
        group_id = _require_evidence_text(
            case.get("group_id"), f"evidence bundle case {index}.group_id"
        )
        if case_id in case_ids or group_id in group_ids:
            raise ValueError("evidence bundle case and group identities must be unique")
        case_ids.add(case_id)
        group_ids.add(group_id)
        units = case.get("unit_ids")
        if type(units) is not list or not units or len(units) > 1024:
            raise ValueError(
                f"evidence bundle case {index}.unit_ids must be a bounded nonempty list"
            )
        validated_units = [
            _require_evidence_text(
                unit, f"evidence bundle case {index}.unit_ids[{unit_index}]"
            )
            for unit_index, unit in enumerate(units)
        ]
        if len(set(validated_units)) != len(validated_units):
            raise ValueError("evidence bundle case unit identities must be unique")
        if unit_ids_seen.intersection(validated_units):
            raise ValueError("evidence bundle units must be disjoint across cases")
        unit_ids_seen.update(validated_units)

        x_shape, x_payload = _decode_exact_f64_bundle(
            case.get("x"),
            f"evidence bundle case {index}.x",
            max_values=MAX_RERUN_RELEVANCE_VALUES,
        )
        baseline_shape, baseline_payload = _decode_exact_f64_bundle(
            case.get("baseline"),
            f"evidence bundle case {index}.baseline",
            max_values=MAX_RERUN_RELEVANCE_VALUES,
        )
        relevance_shape, relevance_payload = _decode_exact_f64_bundle(
            case.get("relevance"),
            f"evidence bundle case {index}.relevance",
            max_values=MAX_RERUN_RELEVANCE_VALUES,
        )
        if x_shape != baseline_shape or x_shape != relevance_shape:
            raise ValueError(
                f"evidence bundle case {index} arrays must have identical shapes"
            )
        case_shapes.append(x_shape)

        encoded_case_id = case_id.encode("utf-8")
        update_length_prefixed(case_digest, encoded_case_id)
        update_length_prefixed(case_digest, group_id.encode("utf-8"))
        case_digest.update(len(validated_units).to_bytes(8, "little"))
        for unit in validated_units:
            update_length_prefixed(case_digest, unit.encode("utf-8"))
        for shape, payload in (
            (x_shape, x_payload),
            (baseline_shape, baseline_payload),
        ):
            case_digest.update(len(shape).to_bytes(8, "little"))
            for dimension in shape:
                case_digest.update(dimension.to_bytes(8, "little"))
            update_length_prefixed(case_digest, payload)

        update_length_prefixed(relevance_digest, encoded_case_id)
        relevance_digest.update(len(relevance_shape).to_bytes(8, "little"))
        for dimension in relevance_shape:
            relevance_digest.update(dimension.to_bytes(8, "little"))
        relevance_digest.update(relevance_payload)

        if index == 0:
            first_case_id = case_id
            first_relevance_shape = relevance_shape
            first_relevance_payload = relevance_payload

    selection_groups = gate.get("selection_group_ids")
    selection_units = gate.get("selection_unit_ids")
    if type(selection_groups) is not list or type(selection_units) is not list:
        raise ValueError("evidence bundle gate selection identities must be lists")
    if group_ids.intersection(selection_groups) or unit_ids_seen.intersection(
        selection_units
    ):
        raise ValueError("evidence bundle selection and validation identities overlap")
    if gate.get("validation_split") == gate.get("selection_split"):
        raise ValueError("evidence bundle selection and validation splits must differ")

    return (
        case_digest.hexdigest(),
        relevance_digest.hexdigest(),
        first_case_id,
        first_relevance_shape,
        first_relevance_payload,
        tuple(case_shapes),
    )


def _recompute_record_work_estimate(
    *,
    method: object,
    model: dict[str, object],
    gate: dict[str, object],
    case_shapes: tuple[tuple[int, ...], ...],
) -> int:
    """Recompute the complete per-record attribution and validation work bound."""

    if method not in {"lrp_epsilon", "grad_x_input"}:
        raise ValueError(
            "evidence bundle work estimate names an unsupported attribution method"
        )
    d_in = _require_bounded_int(
        model.get("d_in"),
        "evidence bundle model.d_in",
        minimum=1,
        maximum=1024,
    )
    d_model = _require_bounded_int(
        model.get("d_model"),
        "evidence bundle model.d_model",
        minimum=1,
        maximum=512,
    )
    n_steps = _require_bounded_int(
        gate.get("n_steps"), "evidence bundle gate.n_steps", minimum=2, maximum=1024
    )
    n_random_rankings = _require_bounded_int(
        gate.get("n_random_rankings"),
        "evidence bundle gate.n_random_rankings",
        minimum=2,
        maximum=100_000,
    )
    gate_forward_calls = 3 + (1 + n_random_rankings) * n_steps
    total = 0
    for index, shape in enumerate(case_shapes):
        if len(shape) != 2 or shape[1] != d_in:
            raise ValueError(
                f"evidence bundle case {index}.x must have shape (tokens, {d_in})"
            )
        token_count = shape[0]
        feature_count = token_count * d_in
        if feature_count < n_steps:
            raise ValueError(
                f"evidence bundle case {index}.x has fewer values than gate.n_steps"
            )
        forward_work = (
            token_count * d_in * d_model
            + 4 * token_count * d_model * d_model
            + 2 * token_count * token_count * d_model
            + d_model
        )
        if method == "lrp_epsilon":
            reverse_work = (
                2 * d_model
                + 4 * token_count * d_model * d_model
                + token_count * token_count * d_model
                + 2 * token_count * d_in * d_model
            )
            attribution_work = forward_work + reverse_work
        else:
            attribution_work = 2 * feature_count * forward_work
        total += attribution_work + gate_forward_calls * forward_work
        if total > MAX_EVIDENCE_RECOMPUTE_MULTIPLY_ADDS:
            raise ValueError(
                "attribution evidence exceeds the "
                f"{MAX_EVIDENCE_RECOMPUTE_MULTIPLY_ADDS}-multiply-add "
                "publication-recomputation resource budget"
            )
    return total


def _expected_software_manifest() -> dict[str, object]:
    directory = Path(__file__).resolve().parent
    source_sha256 = {
        name: hashlib.sha256((directory / name).read_bytes()).hexdigest()
        for name in (
            "attribute.py",
            "faithfulness.py",
            "model.py",
            "probe.py",
            "runlog.py",
        )
    }
    return {
        "python": platform.python_version(),
        "numpy": np.__version__,
        "source_sha256": source_sha256,
    }


def _verify_positive_evidence_decision(
    bundle: dict[str, object],
    *,
    gate: dict[str, object],
    decision: dict[str, object],
) -> None:
    """Recompute a positive check from the exact model, cases, and frozen gate."""

    from .attribute import grad_times_input, lrp_epsilon
    from .faithfulness import (
        AttributionValidationCase,
        RankingSensitivityGate,
        ranking_sensitivity_check,
    )
    from .model import SmallTransformer
    from .probe import METHOD_IMPLEMENTATIONS

    method = bundle["method"]
    methods = {
        "lrp_epsilon": lrp_epsilon,
        "grad_x_input": grad_times_input,
    }
    if method not in methods:
        raise ValueError("positive attribution evidence names an unsupported method")
    if bundle.get("method_implementation") != METHOD_IMPLEMENTATIONS[method]:
        raise ValueError(
            "positive attribution evidence does not name the current method implementation"
        )

    expected_gate_keys = {
        "schema",
        "baseline_name",
        "baseline_provenance",
        "validation_split",
        "selection_split",
        "grouping_provenance",
        "predictor_determinism_provenance",
        "selection_group_ids",
        "selection_unit_ids",
        "alpha_f64",
        "min_groups",
        "n_steps",
        "n_random_rankings",
        "seed",
        "ranking_transform",
        "tie_policy",
        "group_win_rule",
        "group_aggregation",
    }
    if set(gate) != expected_gate_keys:
        raise ValueError("positive attribution evidence gate has an unexpected schema")
    fixed_gate_values = {
        "schema": "prisoma-ranking-sensitivity-gate-v2",
        "ranking_transform": "descending_absolute_magnitude",
        "tie_policy": "abstain_on_any_exact_magnitude_tie",
        "group_win_rule": (
            "method_mean_absolute_deletion_sensitivity_gt_random_mean_and_"
            "plus_one_randomization_tail_lt_half"
        ),
        "group_aggregation": "one_sided_binomial_tail_p0_half",
    }
    if any(gate.get(key) != value for key, value in fixed_gate_values.items()):
        raise ValueError("positive attribution evidence gate changes frozen semantics")
    try:
        alpha = float(_require_evidence_text(gate.get("alpha_f64"), "gate.alpha_f64"))
    except ValueError as error:
        raise ValueError(
            "positive attribution evidence gate alpha is invalid"
        ) from error
    selection_group_ids = gate.get("selection_group_ids")
    selection_unit_ids = gate.get("selection_unit_ids")
    if type(selection_group_ids) is not list or type(selection_unit_ids) is not list:
        raise ValueError("positive attribution evidence gate selections must be lists")
    gate_object = RankingSensitivityGate(
        frozen_gate_id=_require_evidence_text(
            bundle.get("frozen_gate_id"), "evidence bundle frozen_gate_id"
        ),
        baseline_name=_require_evidence_text(
            gate.get("baseline_name"), "gate.baseline_name"
        ),
        baseline_provenance=_require_evidence_text(
            gate.get("baseline_provenance"), "gate.baseline_provenance"
        ),
        validation_split=_require_evidence_text(
            gate.get("validation_split"), "gate.validation_split"
        ),
        selection_split=_require_evidence_text(
            gate.get("selection_split"), "gate.selection_split"
        ),
        grouping_provenance=_require_evidence_text(
            gate.get("grouping_provenance"), "gate.grouping_provenance"
        ),
        predictor_determinism_provenance=_require_evidence_text(
            gate.get("predictor_determinism_provenance"),
            "gate.predictor_determinism_provenance",
        ),
        selection_group_ids=tuple(
            _require_evidence_text(value, "gate.selection_group_ids")
            for value in selection_group_ids
        ),
        selection_unit_ids=tuple(
            _require_evidence_text(value, "gate.selection_unit_ids")
            for value in selection_unit_ids
        ),
        alpha=alpha,
        min_groups=_require_bounded_int(
            gate.get("min_groups"), "gate.min_groups", minimum=1, maximum=1024
        ),
        n_steps=_require_bounded_int(
            gate.get("n_steps"), "gate.n_steps", minimum=1, maximum=1024
        ),
        n_random_rankings=_require_bounded_int(
            gate.get("n_random_rankings"),
            "gate.n_random_rankings",
            minimum=1,
            maximum=100_000,
        ),
        seed=_require_bounded_int(
            gate.get("seed"), "gate.seed", minimum=0, maximum=2**64 - 1
        ),
    )

    model_bundle = bundle["model"]
    assert isinstance(model_bundle, dict)
    d_in = int(model_bundle["d_in"])
    d_model = int(model_bundle["d_model"])
    model = SmallTransformer(d_in=d_in, d_model=d_model, seed=0)
    parameters = model_bundle["parameters"]
    assert isinstance(parameters, dict)
    for name in ("w_embed", "w_q", "w_k", "w_v", "w_o", "w_head"):
        shape, payload = _decode_exact_f64_bundle(
            parameters[name],
            f"evidence bundle model.parameters.{name}",
            max_values=2_000_000,
        )
        setattr(
            model,
            name,
            np.frombuffer(payload, dtype="<f8").copy().reshape(shape),
        )
    model.validate_parameters()

    evidence_cases = bundle["cases"]
    assert isinstance(evidence_cases, list)
    validation_cases = []
    case_rows: list[dict[str, object]] = []
    for index, case in enumerate(evidence_cases):
        assert isinstance(case, dict)

        def decode_array(field: str) -> np.ndarray:
            shape, payload = _decode_exact_f64_bundle(
                case[field],
                f"evidence bundle case {index}.{field}",
                max_values=MAX_RERUN_RELEVANCE_VALUES,
            )
            return np.frombuffer(payload, dtype="<f8").copy().reshape(shape)

        x = decode_array("x")
        baseline = decode_array("baseline")
        relevance = decode_array("relevance")
        recomputed_relevance = np.ascontiguousarray(
            methods[method](model, x), dtype="<f8"
        )
        if (
            recomputed_relevance.shape != relevance.shape
            or recomputed_relevance.tobytes(order="C") != relevance.tobytes(order="C")
        ):
            raise ValueError(
                "positive attribution evidence relevance does not match its method"
            )
        validation_cases.append(
            AttributionValidationCase(
                case_id=str(case["case_id"]),
                group_id=str(case["group_id"]),
                unit_ids=tuple(str(value) for value in case["unit_ids"]),
                x=x,
                attribution=relevance,
                baseline=baseline,
            )
        )
        case_rows.append(case)

    result = ranking_sensitivity_check(
        model.forward, validation_cases, gate=gate_object
    )

    def optional_float(value: float | None) -> str:
        return "not_computed" if value is None else format(value, ".17g")

    expected_decision = {
        "diagnostic": result.diagnostic,
        "status": result.status,
        "reason": result.reason,
        "passed": bool(result.passed),
        "method_sensitivity_f64": optional_float(result.method_sensitivity),
        "random_sensitivity_f64": optional_float(result.random_sensitivity),
        "random_sensitivity_std_f64": optional_float(result.random_sensitivity_std),
        "group_win_binomial_p_f64": optional_float(result.group_win_binomial_p_value),
        "winning_groups": result.winning_groups,
        "independent_groups": result.n_groups,
    }
    if decision != expected_decision:
        raise ValueError(
            "positive attribution evidence decision does not reproduce from its bundle"
        )
    for index, case in enumerate(case_rows):
        expected_contrast = (
            "not_computed"
            if index >= len(result.group_contrasts)
            else float(result.group_contrasts[index]).hex()
        )
        expected_randomization = (
            "not_computed"
            if index >= len(result.group_randomization_p_values)
            else float(result.group_randomization_p_values[index]).hex()
        )
        if (
            case["group_contrast_f64"] != expected_contrast
            or case["group_randomization_p_f64"] != expected_randomization
        ):
            raise ValueError(
                "positive attribution evidence case diagnostics do not reproduce"
            )


def _validate_evidence_metadata(
    *,
    metadata: dict[str, str],
    record: "AttributionRecord",
    bundle: dict[str, object],
    gate: dict[str, object],
    decision: dict[str, object],
    cases: list[object],
    model_hash: str,
    gate_hash: str,
    case_set_hash: str,
    relevance_set_hash: str,
    first_case_id: str,
    declared_work: int,
) -> None:
    """Bind every producer-generated metadata value to exact evidence content."""

    expected_decision_keys = {
        "diagnostic",
        "status",
        "reason",
        "passed",
        "method_sensitivity_f64",
        "random_sensitivity_f64",
        "random_sensitivity_std_f64",
        "group_win_binomial_p_f64",
        "winning_groups",
        "independent_groups",
    }
    if set(decision) != expected_decision_keys:
        raise ValueError("evidence bundle decision has an unexpected schema")
    if decision.get("diagnostic") != "deletion_ranking_sensitivity":
        raise ValueError("evidence bundle decision names an unsupported diagnostic")
    status = decision.get("status")
    if status not in {"passed", "failed", "abstained"}:
        raise ValueError("evidence bundle decision status is invalid")
    reason = _require_evidence_text(
        decision.get("reason"), "evidence bundle decision.reason"
    )
    for key in (
        "method_sensitivity_f64",
        "random_sensitivity_f64",
        "random_sensitivity_std_f64",
        "group_win_binomial_p_f64",
    ):
        _require_optional_f64_text(decision.get(key), f"evidence bundle decision.{key}")
    independent_groups = _require_bounded_int(
        decision.get("independent_groups"),
        "evidence bundle decision.independent_groups",
        minimum=1,
        maximum=len(cases),
    )
    if independent_groups != len(cases):
        raise ValueError(
            "evidence bundle decision independent-group count does not match cases"
        )
    winning_groups = _require_bounded_int(
        decision.get("winning_groups"),
        "evidence bundle decision.winning_groups",
        minimum=0,
        maximum=independent_groups,
    )

    alpha = _require_canonical_f64_text(
        gate.get("alpha_f64"), "evidence bundle gate.alpha_f64"
    )
    n_steps = _require_bounded_int(
        gate.get("n_steps"),
        "evidence bundle gate.n_steps",
        minimum=2,
        maximum=1024,
    )
    n_random_rankings = _require_bounded_int(
        gate.get("n_random_rankings"),
        "evidence bundle gate.n_random_rankings",
        minimum=2,
        maximum=100_000,
    )
    group_contrasts = _evidence_case_diagnostic_vector(cases, "group_contrast_f64")
    group_randomization_p_values = _evidence_case_diagnostic_vector(
        cases, "group_randomization_p_f64"
    )
    group_ids = [
        _require_evidence_text(
            case.get("group_id"), f"evidence bundle case {index}.group_id"
        )
        for index, case in enumerate(cases)
        if isinstance(case, dict)
    ]
    if len(group_ids) != len(cases):
        raise ValueError("evidence bundle cases must contain objects")

    primary_method = bundle["primary_method"]
    assert isinstance(primary_method, str)
    expected_role = "primary" if record.method == primary_method else "secondary"
    expected_metadata = {
        "diagnostic": "deletion_ranking_sensitivity",
        "gate_status": str(status),
        "gate_reason": reason,
        "frozen_gate_id": str(bundle["frozen_gate_id"]),
        "validation_split": _require_evidence_text(
            gate.get("validation_split"), "evidence bundle gate.validation_split"
        ),
        "selection_split": _require_evidence_text(
            gate.get("selection_split"), "evidence bundle gate.selection_split"
        ),
        "grouping_provenance": _require_evidence_text(
            gate.get("grouping_provenance"),
            "evidence bundle gate.grouping_provenance",
        ),
        "baseline_provenance": _require_evidence_text(
            gate.get("baseline_provenance"),
            "evidence bundle gate.baseline_provenance",
        ),
        "method_mean_absolute_deletion_sensitivity": str(
            decision["method_sensitivity_f64"]
        ),
        "random_mean_absolute_deletion_sensitivity": str(
            decision["random_sensitivity_f64"]
        ),
        "random_deletion_sensitivity_std": str(decision["random_sensitivity_std_f64"]),
        "group_win_binomial_p": str(decision["group_win_binomial_p_f64"]),
        "alpha": alpha,
        "validation_cases": str(len(cases)),
        "independent_groups": str(independent_groups),
        "winning_groups": str(winning_groups),
        "deletion_steps": str(n_steps),
        "random_rankings_per_case": str(n_random_rankings),
        "random_reference_se_bound": format(0.5 / math.sqrt(n_random_rankings), ".17g"),
        "group_contrasts": json.dumps(group_contrasts, separators=(",", ":")),
        "group_randomization_p_values": json.dumps(
            group_randomization_p_values, separators=(",", ":")
        ),
        "ordered_group_ids": json.dumps(group_ids, separators=(",", ":")),
        "representative_case_id": first_case_id,
        "validation_relevance_set_sha256": relevance_set_hash,
        "validation_input_baseline_set_sha256": case_set_hash,
        "baseline_may_be_out_of_distribution": "true",
        "feature_dependence_unresolved": "true",
        "causal_or_mechanistic_faithfulness_established": "false",
        "method_implementation": str(bundle["method_implementation"]),
        "model_parameter_sha256": model_hash,
        "gate_content_sha256": gate_hash,
        "probe_work_estimate_multiply_adds": str(declared_work),
        "confirmatory_role": expected_role,
        "multiplicity_policy": "one_predeclared_primary_method",
    }
    for key, expected in expected_metadata.items():
        if metadata.get(key) != expected:
            raise ValueError(
                f"attribution evidence metadata field {key!r} does not match "
                "the exact evidence bundle"
            )


def _validate_evidence_config(
    config: dict[str, object],
    *,
    records_count: int,
    primary_method: str,
    method_order: list[str],
    method_implementations: dict[str, str],
    first_bundle: dict[str, object],
    first_work: int,
    batch_identity: tuple[str, str, str, str, str, str, str],
) -> None:
    """Reject supplied configuration values that contradict exact evidence."""

    model = first_bundle["model"]
    gate = first_bundle["gate"]
    cases = first_bundle["cases"]
    assert isinstance(model, dict)
    assert isinstance(gate, dict)
    assert isinstance(cases, list)
    token_counts: set[int] = set()
    for index, case in enumerate(cases):
        assert isinstance(case, dict)
        x = case.get("x")
        if not isinstance(x, dict):
            raise ValueError(f"evidence bundle case {index}.x must be an object")
        shape = x.get("shape")
        if (
            not isinstance(shape, list)
            or len(shape) != 2
            or any(type(dimension) is not int for dimension in shape)
        ):
            raise ValueError(
                f"evidence bundle case {index}.x must have a two-dimensional shape"
            )
        token_counts.add(shape[0])

    target, modality, layer, baseline, model_hash, gate_hash, case_set_hash = (
        batch_identity
    )
    expected_if_supplied: dict[str, object] = {
        "model": "small_transformer",
        "model_parameter_sha256": model_hash,
        "d_in": model["d_in"],
        "d_model": model["d_model"],
        "validation_cases": len(cases),
        "target_output": target,
        "modality": modality,
        "layer": layer,
        "baseline": baseline,
        "baseline_name": baseline,
        "diagnostic": "deletion_ranking_sensitivity",
        "frozen_gate_id": first_bundle["frozen_gate_id"],
        "gate_content_sha256": gate_hash,
        "gate_manifest": gate,
        "methods": method_order,
        "primary_method": primary_method,
        "method_implementations": method_implementations,
        "validation_input_baseline_set_sha256": case_set_hash,
        "case_set_sha256": case_set_hash,
        "probe_work_estimate_multiply_adds": first_work,
        "seed": gate["seed"],
    }
    if len(token_counts) == 1:
        expected_if_supplied["tokens"] = next(iter(token_counts))
    elif "tokens" in config:
        raise ValueError(
            "attribution config tokens cannot summarize a variable-token evidence batch"
        )
    for key, expected in expected_if_supplied.items():
        if key in config and config[key] != expected:
            raise ValueError(
                f"attribution config field {key!r} contradicts the exact evidence batch"
            )
    if config.get("experiment") != "attribution_probe":
        raise ValueError("attribution config experiment must be attribution_probe")
    if config.get("n_records") != str(records_count):
        raise ValueError(
            "attribution config n_records must match the publication batch"
        )


def _validate_evidence_bundle(
    bundle: object,
    *,
    record: "AttributionRecord",
    metadata: dict[str, str],
    representative_relevance: np.ndarray,
    record_index: int,
) -> tuple[dict[str, object], int, bool]:
    """Require the probe's reconstructable v2 evidence shape before publication."""

    if type(bundle) is not dict:
        raise ValueError(
            f"record {record_index} evidence_bundle must be a plain dictionary"
        )
    required = {
        "schema",
        "software",
        "method",
        "method_implementation",
        "primary_method",
        "target_output",
        "modality",
        "layer",
        "work_estimate_multiply_adds",
        "model",
        "gate",
        "gate_content_sha256",
        "frozen_gate_id",
        "case_set_sha256",
        "cases",
        "decision",
    }
    missing = required.difference(bundle)
    if missing:
        raise ValueError(
            f"record {record_index} evidence_bundle omits required fields: "
            + ", ".join(sorted(missing))
        )
    if bundle.get("schema") != "prisoma-attribution-evidence-v2":
        raise ValueError(
            "evidence bundle schema must be prisoma-attribution-evidence-v2"
        )
    if bundle.get("method") != record.method:
        raise ValueError("evidence bundle method does not match the record")
    if bundle.get("target_output") != record.target_output:
        raise ValueError("evidence bundle target_output does not match the record")
    if bundle.get("method_implementation") != metadata.get("method_implementation"):
        raise ValueError(
            "evidence bundle method implementation does not match metadata"
        )
    if bundle.get("software") != _expected_software_manifest():
        raise ValueError(
            "evidence bundle software provenance does not match the publishing runtime"
        )
    expected_modality = (
        record.modality if record.modality is not None else "not_declared"
    )
    if bundle.get("modality") != expected_modality:
        raise ValueError("evidence bundle modality does not match the record")
    expected_layer = record.layer if record.layer is not None else "not_declared"
    if bundle.get("layer") != expected_layer:
        raise ValueError("evidence bundle layer does not match the record")

    gate = bundle.get("gate")
    if type(gate) is not dict:
        raise ValueError("evidence bundle gate must be an object")
    gate_payload = json.dumps(
        gate,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    gate_hash = hashlib.sha256(gate_payload).hexdigest()
    if (
        _require_sha256(
            bundle.get("gate_content_sha256"), "evidence bundle gate_content_sha256"
        )
        != gate_hash
    ):
        raise ValueError("evidence bundle gate hash does not match its gate manifest")
    if metadata.get("gate_content_sha256") != gate_hash:
        raise ValueError("evidence bundle gate hash does not match record metadata")
    if record.baseline != gate.get("baseline_name"):
        raise ValueError(
            "evidence bundle gate baseline does not match the attribution record"
        )
    frozen_gate_id = bundle.get("frozen_gate_id")
    if (
        frozen_gate_id != f"sha256:{gate_hash}"
        or metadata.get("frozen_gate_id") != frozen_gate_id
    ):
        raise ValueError("evidence bundle frozen gate identity is inconsistent")

    model = bundle.get("model")
    recomputed_model_hash = _recompute_model_parameter_hash(model)
    assert isinstance(model, dict)
    model_hash = _require_sha256(
        model.get("parameter_sha256"), "evidence bundle model.parameter_sha256"
    )
    if model_hash != recomputed_model_hash:
        raise ValueError("evidence bundle model hash does not match its parameters")
    if metadata.get("model_parameter_sha256") != model_hash:
        raise ValueError("evidence bundle model hash does not match record metadata")

    case_set_hash = _require_sha256(
        bundle.get("case_set_sha256"), "evidence bundle case_set_sha256"
    )
    (
        recomputed_case_set_hash,
        recomputed_relevance_set_hash,
        first_case_id,
        shape,
        payload,
        case_shapes,
    ) = _recompute_case_commitments(bundle.get("cases"), gate)
    if case_set_hash != recomputed_case_set_hash:
        raise ValueError("evidence bundle case-set hash does not match its cases")
    if metadata.get("validation_input_baseline_set_sha256") != case_set_hash:
        raise ValueError("evidence bundle case-set hash does not match record metadata")
    if metadata.get("validation_relevance_set_sha256") != recomputed_relevance_set_hash:
        raise ValueError(
            "evidence bundle relevance-set hash does not match record metadata"
        )
    if metadata.get("representative_case_id") != first_case_id:
        raise ValueError("evidence bundle representative case does not match metadata")
    representative = np.ascontiguousarray(representative_relevance, dtype="<f8")
    if shape != representative.shape or payload != representative.tobytes(order="C"):
        raise ValueError(
            "evidence bundle first-case relevance does not match the logged representative"
        )
    declared_work = _require_bounded_int(
        bundle.get("work_estimate_multiply_adds"),
        "evidence bundle work_estimate_multiply_adds",
        minimum=1,
        maximum=MAX_EVIDENCE_RECOMPUTE_MULTIPLY_ADDS,
    )
    metadata_work = metadata.get("probe_work_estimate_multiply_adds")
    if metadata_work != str(declared_work):
        raise ValueError("evidence bundle work estimate does not match record metadata")
    recomputed_work = _recompute_record_work_estimate(
        method=bundle.get("method"),
        model=model,
        gate=gate,
        case_shapes=case_shapes,
    )
    if declared_work != recomputed_work:
        raise ValueError(
            "evidence bundle work estimate does not match the recomputed "
            "per-record work"
        )

    decision = bundle.get("decision")
    if type(decision) is not dict:
        raise ValueError("evidence bundle must contain a decision object")
    if decision.get("diagnostic") != metadata.get("diagnostic"):
        raise ValueError("evidence bundle diagnostic does not match record metadata")
    if decision.get("status") != metadata.get("gate_status") or decision.get(
        "reason"
    ) != metadata.get("gate_reason"):
        raise ValueError("evidence bundle decision does not match record metadata")
    if type(decision.get("passed")) is not bool:
        raise ValueError("evidence bundle decision.passed must be a boolean")
    if decision.get("passed") != (decision.get("status") == "passed"):
        raise ValueError("evidence bundle decision status and passed flag disagree")

    primary_method = bundle.get("primary_method")
    if primary_method not in {"lrp_epsilon", "grad_x_input"}:
        raise ValueError(
            "evidence bundle primary_method must name one supported attribution method"
        )
    role = metadata.get("confirmatory_role")
    if role not in {"primary", "secondary"}:
        raise ValueError(
            "attribution evidence confirmatory_role must be primary or secondary"
        )
    if metadata.get("multiplicity_policy") != "one_predeclared_primary_method":
        raise ValueError(
            "attribution evidence must declare the one-predeclared-primary-method "
            "multiplicity policy"
        )
    if role == "primary" and primary_method != record.method:
        raise ValueError(
            "primary evidence record does not match the predeclared method"
        )
    if role == "secondary" and primary_method == record.method:
        raise ValueError("secondary evidence record incorrectly names itself primary")
    expected_recorded_check = bool(
        decision.get("passed") and role == "primary" and primary_method == record.method
    )
    if record.faithfulness_passed != expected_recorded_check:
        raise ValueError(
            "recorded check does not match the evidence decision and predeclared role"
        )
    evidence_cases = bundle.get("cases")
    assert isinstance(evidence_cases, list)
    _validate_evidence_metadata(
        metadata=metadata,
        record=record,
        bundle=bundle,
        gate=gate,
        decision=decision,
        cases=evidence_cases,
        model_hash=model_hash,
        gate_hash=gate_hash,
        case_set_hash=case_set_hash,
        relevance_set_hash=recomputed_relevance_set_hash,
        first_case_id=first_case_id,
        declared_work=declared_work,
    )
    return bundle, declared_work, bool(decision.get("passed"))


def _validate_json_resource_bytes(payload: bytes, line_number: int) -> None:
    """Mirror pid-runlog's bounded JSON scanner for one serialized event."""
    if len(payload) > MAX_RUNLOG_LINE_BYTES:
        raise ValueError(
            f"run-log line {line_number} exceeds the {MAX_RUNLOG_LINE_BYTES}-byte limit"
        )

    # Each frame is [container byte, comma count, has content]. The payload was
    # produced by json.dumps, so syntax is already valid; this scan enforces the
    # canonical reader's encoded-string, nesting, and container budgets exactly.
    stack: list[list[int | bool]] = []
    in_string = False
    escaped = False
    string_bytes = 0
    for byte in payload:
        if in_string:
            if escaped:
                escaped = False
                string_bytes += 1
            elif byte == ord("\\"):
                escaped = True
                string_bytes += 1
            elif byte == ord('"'):
                in_string = False
            else:
                string_bytes += 1
            if string_bytes > MAX_RUNLOG_STRING_BYTES:
                raise ValueError(
                    f"run-log line {line_number} contains a JSON string exceeding "
                    f"the {MAX_RUNLOG_STRING_BYTES}-byte limit"
                )
            continue

        if byte == ord('"'):
            if stack:
                stack[-1][2] = True
            in_string = True
            string_bytes = 0
        elif byte in (ord("{"), ord("[")):
            if stack:
                stack[-1][2] = True
            depth = len(stack) + 1
            if depth > MAX_RUNLOG_NESTING_DEPTH:
                raise ValueError(
                    f"run-log line {line_number} exceeds the JSON nesting limit "
                    f"{MAX_RUNLOG_NESTING_DEPTH}"
                )
            stack.append([byte, 0, False])
        elif byte == ord(","):
            if stack:
                stack[-1][1] = int(stack[-1][1]) + 1
        elif byte in (ord("}"), ord("]")):
            frame = stack.pop()
            entries = int(frame[1]) + 1 if frame[2] else 0
            if frame[0] == ord("["):
                limit = MAX_RUNLOG_ARRAY_LEN
                kind = "array length"
            else:
                limit = MAX_RUNLOG_OBJECT_ENTRIES
                kind = "object entries"
            if entries > limit:
                raise ValueError(
                    f"run-log line {line_number} exceeds the JSON {kind} limit {limit}"
                )
        elif not chr(byte).isspace() and stack:
            stack[-1][2] = True


def _serialize_bounded_runlog(events: Sequence[dict]) -> bytes:
    if len(events) > MAX_RERUN_EVENTS:
        raise ValueError(
            f"run-log event count exceeds the {MAX_RERUN_EVENTS}-event viewer limit"
        )

    output = bytearray()
    serialized_event_bytes = 0
    for line_number, event in enumerate(events, start=1):
        payload = json.dumps(event, ensure_ascii=False, allow_nan=False).encode("utf-8")
        _validate_json_resource_bytes(payload, line_number)
        serialized_event_bytes += len(payload)
        if serialized_event_bytes > MAX_RERUN_SERIALIZED_EVENT_BYTES:
            raise ValueError(
                "run-log serialized-event aggregate exceeds the "
                f"{MAX_RERUN_SERIALIZED_EVENT_BYTES}-byte viewer limit"
            )
        projected_total = len(output) + len(payload) + 1
        if projected_total > MAX_RUNLOG_FILE_BYTES:
            raise ValueError(
                f"run-log aggregate bytes exceed the {MAX_RUNLOG_FILE_BYTES}-byte limit"
            )
        output.extend(payload)
        output.append(ord("\n"))
    return bytes(output)


def _artifact_filename(content_hash: str) -> str:
    return f"{content_hash}.npy"


def _evidence_filename(content_hash: str) -> str:
    return f"{content_hash}.json"


def _lexical_absolute(path: str | Path) -> Path:
    """Return an absolute, normalized path without following symlinks."""
    return Path(os.path.abspath(os.fspath(path)))


def _validate_path_components(path: Path, *, leaf_kind: str) -> None:
    """Reject symlinked/non-directory ancestors and unsafe existing leaves."""
    current = Path(path.anchor)
    for part in path.parts[1:]:
        current /= part
        is_leaf = current == path
        try:
            metadata = current.lstat()
        except FileNotFoundError:
            return

        if stat.S_ISLNK(metadata.st_mode):
            raise ValueError(f"publication path must not contain symlinks: {current}")
        if not is_leaf:
            if not stat.S_ISDIR(metadata.st_mode):
                raise ValueError(
                    f"publication path ancestor must be a directory: {current}"
                )
            continue

        if leaf_kind == "directory":
            if not stat.S_ISDIR(metadata.st_mode):
                raise ValueError(f"artifact_dir must be a directory: {current}")
        elif leaf_kind == "file":
            if not stat.S_ISREG(metadata.st_mode):
                raise ValueError(f"runlog_path must be a regular file: {current}")
            if metadata.st_nlink != 1:
                raise ValueError(
                    "runlog_path must not have hard-link aliases before replacement"
                )
        else:  # pragma: no cover - internal programming error
            raise AssertionError(f"unknown leaf kind: {leaf_kind}")


def _validate_publication_topology(
    runlog_path: Path, artifact_dir: Path | None
) -> None:
    """Validate publication paths without creating or resolving through aliases."""
    if runlog_path == Path(runlog_path.anchor):
        raise ValueError("runlog_path must name a file, not a filesystem root")
    _validate_path_components(runlog_path, leaf_kind="file")

    if artifact_dir is None:
        return
    runlog_dir = runlog_path.parent
    try:
        relative_artifact_dir = artifact_dir.relative_to(runlog_dir)
    except ValueError as error:
        raise ValueError(
            "artifact_dir must be a strict descendant of the run log directory"
        ) from error
    if not relative_artifact_dir.parts:
        raise ValueError(
            "artifact_dir must be a strict descendant of the run log directory"
        )
    if artifact_dir == runlog_path or runlog_path in artifact_dir.parents:
        raise ValueError(
            "runlog_path and artifact_dir must not alias or contain each other"
        )
    _validate_path_components(artifact_dir, leaf_kind="directory")

    if runlog_dir.exists() and artifact_dir.exists():
        try:
            aliases_runlog_dir = os.path.samefile(runlog_dir, artifact_dir)
        except OSError as error:
            raise ValueError("unable to validate artifact_dir topology") from error
        if aliases_runlog_dir:
            raise ValueError("artifact_dir must not alias the run log directory")


def _stage_synced_bytes(directory: Path, payload: bytes) -> Path:
    """Write and file-sync one temporary file in its destination directory."""
    descriptor, raw_path = tempfile.mkstemp(
        prefix=".attribution-stage-", suffix=".tmp", dir=directory
    )
    staged_path = Path(raw_path)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
    except BaseException:
        staged_path.unlink(missing_ok=True)
        raise
    return staged_path


def _verify_existing_artifact(path: Path, expected_bytes: bytes) -> None:
    try:
        metadata = path.lstat()
    except FileNotFoundError as error:
        raise OSError(
            f"installed artifact disappeared before verification: {path}"
        ) from error
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError(
            f"artifact destination must be a regular non-symlink file: {path}"
        )
    if metadata.st_nlink != 1:
        raise ValueError(
            f"artifact destination must not have hard-link aliases: {path}"
        )
    if metadata.st_size != len(expected_bytes) or path.read_bytes() != expected_bytes:
        raise FileExistsError(
            f"content-addressed artifact exists with different bytes: {path}"
        )


def _preflight_artifact_destination(path: Path, expected_bytes: bytes) -> None:
    try:
        path.lstat()
    except FileNotFoundError:
        return
    _verify_existing_artifact(path, expected_bytes)


def _install_staged_artifact(
    staged_path: Path, artifact_path: Path, artifact_bytes: bytes
) -> None:
    """Install one staged artifact by a no-clobber hard-link operation."""
    try:
        os.link(staged_path, artifact_path, follow_symlinks=False)
    except FileExistsError:
        _verify_existing_artifact(artifact_path, artifact_bytes)
        return

    staged_path.unlink()
    _verify_existing_artifact(artifact_path, artifact_bytes)


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
    evidence_bundle: dict[str, object] | None = None

    def __post_init__(self) -> None:
        if type(self.method) is not str or not self.method:
            raise ValueError(
                "method must be a non-empty string (the harness validator rejects it)"
            )
        if type(self.target_output) is not str or not self.target_output:
            raise ValueError("target_output must be a non-empty string")
        for field_name in ("layer", "modality", "baseline"):
            value = getattr(self, field_name)
            if value is not None and type(value) is not str:
                raise ValueError(f"{field_name} must be a string or None")
        if type(self.faithfulness_passed) is not bool:
            raise ValueError("faithfulness_passed must be a boolean")
        if self.evidence_bundle is not None and type(self.evidence_bundle) is not dict:
            raise ValueError("evidence_bundle must be a plain dictionary or None")


def write_attribution_runlog(
    runlog_path: str | Path,
    records: Sequence[AttributionRecord],
    *,
    run_id: str = "attribution-probe",
    config: dict | None = None,
    artifact_dir: str | Path | None = None,
) -> Path:
    """Write a canonical run log for a batch of attribution records.

    Each record's ``relevance`` array is saved as an exact NumPy v1.0 little-endian
    ``f64`` C-order artifact (when ``artifact_dir`` is given). ``artifact_dir`` must
    resolve inside the run log's directory, and ``artifact_uri`` is emitted relative
    to that directory for the standalone converter's confined, explicit loader. A
    versioned evidence bundle, when present, is written as canonical JSON and bound
    to the record. Both files receive companion ``artifact_logged`` events so the
    canonical manifest can enumerate and verify them. ``score_hash`` commits to the
    exact shape and little-endian f64 bytes; no rounding is applied. Artifacts are
    content-addressed and installed without replacing an existing name. Their file
    contents and the staged run log are synced before the run-log name is atomically
    replaced last.
    """
    run_id = _canonical_run_id(run_id)
    if isinstance(records, (str, bytes, np.ndarray)) or not isinstance(
        records, Sequence
    ):
        raise ValueError("records must be a sequence of AttributionRecord values")

    requested_runlog_path = Path(runlog_path)
    publication_runlog_path = _lexical_absolute(requested_runlog_path)
    runlog_dir = publication_runlog_path.parent
    resolved_artifact_dir = None
    if artifact_dir is not None:
        resolved_artifact_dir = _lexical_absolute(artifact_dir)
    _validate_publication_topology(publication_runlog_path, resolved_artifact_dir)

    projected_event_count = 3 + len(records)
    if resolved_artifact_dir is not None:
        projected_event_count += len(records)
    projected_event_count += sum(
        1
        for record in records
        if isinstance(record, AttributionRecord) and record.evidence_bundle is not None
    )
    evidence_batch = any(
        isinstance(record, AttributionRecord) and record.evidence_bundle is not None
        for record in records
    )
    if projected_event_count > MAX_RERUN_EVENTS:
        raise ValueError(
            f"run-log event count exceeds the {MAX_RERUN_EVENTS}-event viewer limit"
        )

    if config is not None and type(config) is not dict:
        raise ValueError("config must be a plain dictionary when provided")
    config = dict(config or {})
    config.setdefault("experiment", "attribution_probe")
    config.setdefault("n_records", str(len(records)))
    config_hash = canonical_hash(config)

    events: list[dict] = []
    prepared_artifacts: dict[tuple[str, str], tuple[Path, bytes]] = {}
    prepared_artifact_bytes = 0
    positive_evidence: list[
        tuple[dict[str, object], dict[str, object], dict[str, object]]
    ] = []
    positive_evidence_work = 0
    evidence_methods: set[str] = set()
    evidence_method_order: list[str] = []
    evidence_method_implementations: dict[str, str] = {}
    evidence_primary_method: str | None = None
    evidence_batch_identity: tuple[str, str, str, str, str, str, str] | None = None
    evidence_first_bundle: dict[str, object] | None = None
    evidence_first_work: int | None = None
    primary_evidence_records = 0
    legacy_positive_records = 0
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
        if not isinstance(rec, AttributionRecord):
            raise ValueError(f"records[{i}] must be an AttributionRecord")
        if evidence_batch and rec.evidence_bundle is None:
            raise ValueError(
                "an evidence-bearing attribution publication batch requires "
                "reconstructable evidence for every record"
            )
        if type(rec.method) is not str or not rec.method:
            raise ValueError(
                "method must be a non-empty string (the harness validator rejects it)"
            )
        if type(rec.target_output) is not str or not rec.target_output:
            raise ValueError("target_output must be a non-empty string")
        for field_name in ("layer", "modality", "baseline"):
            value = getattr(rec, field_name)
            if value is not None and type(value) is not str:
                raise ValueError(f"{field_name} must be a string or None")
        if type(rec.faithfulness_passed) is not bool:
            raise ValueError("faithfulness_passed must be a boolean")
        if type(rec.relevance) is not np.ndarray:
            raise ValueError("relevance must be a NumPy ndarray")
        if rec.relevance.size > MAX_RERUN_RELEVANCE_VALUES:
            raise ValueError(
                "relevance arrays must contain at most "
                f"{MAX_RERUN_RELEVANCE_VALUES} values"
            )
        if np.issubdtype(rec.relevance.dtype, np.bool_) or not np.issubdtype(
            rec.relevance.dtype, np.number
        ):
            raise ValueError("relevance arrays must contain real numeric values")
        if np.issubdtype(rec.relevance.dtype, np.complexfloating):
            raise ValueError("relevance arrays must contain real numeric values")
        try:
            rel = np.ascontiguousarray(rec.relevance, dtype=np.dtype("<f8"))
        except (TypeError, ValueError, OverflowError) as error:
            raise ValueError(
                "relevance arrays must be representable as float64"
            ) from error
        if rel.size == 0:
            raise ValueError("relevance arrays must be non-empty")
        if not np.isfinite(rel).all():
            raise ValueError("relevance arrays must contain only finite values")
        score_digest = hashlib.sha256(b"prisoma-attribution-score-f64-le-v1\0")
        score_digest.update(len(rel.shape).to_bytes(8, "little"))
        for dimension in rel.shape:
            score_digest.update(int(dimension).to_bytes(8, "little"))
        score_digest.update(rel.tobytes(order="C"))
        score_hash = score_digest.hexdigest()
        metadata = _validated_record_metadata(rec.metadata, i)
        metadata["relevance_shape"] = "x".join(str(n) for n in rel.shape)
        metadata["score_hash_encoding"] = "shape_plus_exact_f64_le_bytes_v1"

        if rec.faithfulness_passed:
            required_positive_metadata = {
                "diagnostic": "deletion_ranking_sensitivity",
                "gate_status": "passed",
                "gate_reason": "ranking_sensitivity_gate_passed",
                "confirmatory_role": "primary",
                "causal_or_mechanistic_faithfulness_established": "false",
            }
            for key, expected_value in required_positive_metadata.items():
                if metadata.get(key) != expected_value:
                    raise ValueError(
                        "a positive recorded check requires internally consistent "
                        f"evidence metadata {key}={expected_value!r}"
                    )
            if rec.evidence_bundle is None:
                raise ValueError(
                    "a positive recorded check requires a reconstructable evidence bundle"
                )
        if metadata.get("gate_status") == "passed" and rec.evidence_bundle is None:
            raise ValueError(
                "a typed passed attribution decision requires a reconstructable "
                "evidence bundle"
            )

        artifact_uri = None
        if resolved_artifact_dir is not None:
            artifact_buffer = io.BytesIO()
            np.lib.format.write_array(
                artifact_buffer, rel, version=(1, 0), allow_pickle=False
            )
            artifact_bytes = artifact_buffer.getvalue()
            if len(artifact_bytes) > MAX_RERUN_RELEVANCE_FILE_BYTES:
                raise ValueError(
                    "relevance artifact exceeds the Rerun converter's "
                    f"{MAX_RERUN_RELEVANCE_FILE_BYTES}-byte limit"
                )
            artifact_hash = hashlib.sha256(artifact_bytes).hexdigest()
            artifact_path = resolved_artifact_dir / _artifact_filename(artifact_hash)
            artifact_uri = artifact_path.relative_to(runlog_dir).as_posix()
            metadata["artifact_sha256"] = artifact_hash
            artifact_key = (artifact_hash, ".npy")
            prior_artifact = prepared_artifacts.get(artifact_key)
            if prior_artifact is not None and prior_artifact[1] != artifact_bytes:
                raise RuntimeError("sha256 collision across attribution artifacts")
            if prior_artifact is None:
                prepared_artifact_bytes += len(artifact_bytes)
                if prepared_artifact_bytes > MAX_RERUN_PREPARED_ARTIFACT_BYTES:
                    raise ValueError(
                        "unique prepared relevance artifacts exceed the "
                        f"{MAX_RERUN_PREPARED_ARTIFACT_BYTES}-byte aggregate limit"
                    )
                prepared_artifacts[artifact_key] = (artifact_path, artifact_bytes)
            events.append(
                {
                    "type": "artifact_logged",
                    "timestamp_ns": ts,
                    "name": f"attribution_relevance_{i}",
                    "kind": "attribution_relevance_npy_v1",
                    "uri": artifact_uri,
                    "sha256": artifact_hash,
                    "metadata": {"method": rec.method, "score_hash": score_hash},
                }
            )
            ts += 1

        if rec.evidence_bundle is not None:
            if resolved_artifact_dir is None:
                raise ValueError(
                    "records with evidence bundles require a confined artifact_dir"
                )
            _validate_canonical_value(
                rec.evidence_bundle, f"records[{i}].evidence_bundle"
            )
            (
                evidence_bundle,
                evidence_work,
                evidence_decision_passed,
            ) = _validate_evidence_bundle(
                rec.evidence_bundle,
                record=rec,
                metadata=metadata,
                representative_relevance=rel,
                record_index=i,
            )
            primary_method = evidence_bundle["primary_method"]
            assert isinstance(primary_method, str)
            evidence_model = evidence_bundle["model"]
            assert isinstance(evidence_model, dict)
            batch_identity = (
                rec.target_output,
                rec.modality if rec.modality is not None else "not_declared",
                rec.layer if rec.layer is not None else "not_declared",
                rec.baseline if rec.baseline is not None else "not_declared",
                str(evidence_model["parameter_sha256"]),
                str(evidence_bundle["gate_content_sha256"]),
                str(evidence_bundle["case_set_sha256"]),
            )
            if evidence_batch_identity is None:
                evidence_batch_identity = batch_identity
            elif evidence_batch_identity != batch_identity:
                raise ValueError(
                    "all attribution evidence records in one publication batch "
                    "must share one target, modality, layer, baseline, model, gate, "
                    "and case-set identity"
                )
            if rec.method in evidence_methods:
                raise ValueError(
                    "attribution evidence records must name unique methods within "
                    "one publication batch"
                )
            evidence_methods.add(rec.method)
            evidence_method_order.append(rec.method)
            evidence_method_implementations[rec.method] = str(
                evidence_bundle["method_implementation"]
            )
            if evidence_first_bundle is None:
                evidence_first_bundle = evidence_bundle
                evidence_first_work = evidence_work
            if evidence_primary_method is None:
                evidence_primary_method = primary_method
            elif evidence_primary_method != primary_method:
                raise ValueError(
                    "all attribution evidence records in one publication batch "
                    "must name the same predeclared primary method"
                )
            if metadata["confirmatory_role"] == "primary":
                primary_evidence_records += 1
            if rec.faithfulness_passed:
                legacy_positive_records += 1
            if evidence_decision_passed:
                positive_evidence_work += evidence_work
                if positive_evidence_work > MAX_EVIDENCE_RECOMPUTE_MULTIPLY_ADDS:
                    raise ValueError(
                        "positive attribution evidence batch exceeds the "
                        f"{MAX_EVIDENCE_RECOMPUTE_MULTIPLY_ADDS}-multiply-add "
                        "publication-recomputation resource budget"
                    )
                evidence_gate = evidence_bundle["gate"]
                evidence_decision = evidence_bundle["decision"]
                assert isinstance(evidence_gate, dict)
                assert isinstance(evidence_decision, dict)
                positive_evidence.append(
                    (evidence_bundle, evidence_gate, evidence_decision)
                )
            evidence_bytes = json.dumps(
                evidence_bundle,
                sort_keys=True,
                separators=(",", ":"),
                ensure_ascii=False,
                allow_nan=False,
            ).encode("utf-8")
            evidence_hash = hashlib.sha256(evidence_bytes).hexdigest()
            evidence_path = resolved_artifact_dir / _evidence_filename(evidence_hash)
            evidence_uri = evidence_path.relative_to(runlog_dir).as_posix()
            metadata["evidence_bundle_sha256"] = evidence_hash
            metadata["evidence_bundle_uri"] = evidence_uri
            evidence_key = (evidence_hash, ".json")
            prior_evidence = prepared_artifacts.get(evidence_key)
            if prior_evidence is not None and prior_evidence[1] != evidence_bytes:
                raise RuntimeError(
                    "sha256 collision across attribution evidence bundles"
                )
            if prior_evidence is None:
                prepared_artifact_bytes += len(evidence_bytes)
                if prepared_artifact_bytes > MAX_RERUN_PREPARED_ARTIFACT_BYTES:
                    raise ValueError(
                        "unique prepared attribution artifacts exceed the "
                        f"{MAX_RERUN_PREPARED_ARTIFACT_BYTES}-byte aggregate limit"
                    )
                prepared_artifacts[evidence_key] = (evidence_path, evidence_bytes)
            events.append(
                {
                    "type": "artifact_logged",
                    "timestamp_ns": ts,
                    "name": f"attribution_evidence_{i}",
                    "kind": "attribution_evidence_json_v2",
                    "uri": evidence_uri,
                    "sha256": evidence_hash,
                    "metadata": {
                        "method": rec.method,
                        "diagnostic": metadata.get("diagnostic", "not_declared"),
                    },
                }
            )
            ts += 1

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

    if evidence_methods:
        if primary_evidence_records != 1:
            raise ValueError(
                "an attribution evidence publication batch must contain exactly "
                "one primary evidence record"
            )
        if legacy_positive_records > 1:
            raise ValueError(
                "an attribution evidence publication batch may contain at most "
                "one positive legacy compatibility flag"
            )
        assert evidence_primary_method is not None
        assert evidence_batch_identity is not None
        assert evidence_first_bundle is not None
        assert evidence_first_work is not None
        _validate_evidence_config(
            config,
            records_count=len(records),
            primary_method=evidence_primary_method,
            method_order=evidence_method_order,
            method_implementations=evidence_method_implementations,
            first_bundle=evidence_first_bundle,
            first_work=evidence_first_work,
            batch_identity=evidence_batch_identity,
        )

    # Enforce the complete batch budget before any model, method, or gate
    # reconstruction. Structural validation above already recomputed each exact
    # per-record work commitment; only bounded positive decisions reach this loop.
    for evidence_bundle, evidence_gate, evidence_decision in positive_evidence:
        _verify_positive_evidence_decision(
            evidence_bundle,
            gate=evidence_gate,
            decision=evidence_decision,
        )

    events.append(
        {
            "type": "run_ended",
            "run_id": run_id,
            "timestamp_ns": ts,
            "status": "succeeded",
            "message": f"logged {len(records)} attribution record(s)",
        }
    )

    # Serialize the complete log with strict finite-number handling before touching
    # the filesystem. Together with the in-memory artifact framing above, this
    # prevents invalid content from leaving publication files behind.
    runlog_bytes = _serialize_bounded_runlog(events)

    # Existing content-addressed names are checked before any directory or staging
    # file is created, then checked again during the no-clobber installation.
    for artifact_path, artifact_bytes in prepared_artifacts.values():
        _preflight_artifact_destination(artifact_path, artifact_bytes)

    publication_runlog_path.parent.mkdir(parents=True, exist_ok=True)
    if resolved_artifact_dir is not None:
        resolved_artifact_dir.mkdir(parents=True, exist_ok=True)
    _validate_publication_topology(publication_runlog_path, resolved_artifact_dir)

    staged_artifacts: list[tuple[Path, Path, bytes]] = []
    staged_runlog: Path | None = None
    try:
        if resolved_artifact_dir is not None:
            for artifact_path, artifact_bytes in prepared_artifacts.values():
                staged_path = _stage_synced_bytes(resolved_artifact_dir, artifact_bytes)
                staged_artifacts.append((staged_path, artifact_path, artifact_bytes))
        staged_runlog = _stage_synced_bytes(runlog_dir, runlog_bytes)

        for staged_path, artifact_path, artifact_bytes in staged_artifacts:
            _install_staged_artifact(staged_path, artifact_path, artifact_bytes)

        # Recheck the named topology immediately before publishing the new log. The
        # run log is the final visible name changed by this function.
        _validate_publication_topology(publication_runlog_path, resolved_artifact_dir)
        os.replace(staged_runlog, publication_runlog_path)
        staged_runlog = None
    finally:
        for staged_path, _, _ in staged_artifacts:
            staged_path.unlink(missing_ok=True)
        if staged_runlog is not None:
            staged_runlog.unlink(missing_ok=True)

    return requested_runlog_path
