"""Attribute a held-out case set, validate ranking sensitivity, and log it.

The canonical run-log schema retains a historical ``faithfulness_check`` boolean.
This orchestrator sets it to true only when the frozen, group-disjoint deletion
ranking-sensitivity gate passes.  The narrower diagnostic name, decision reason,
randomization evidence, baseline provenance, and non-causal limitations are included
in metadata.
"""

from __future__ import annotations

import base64
import hashlib
import json
import platform
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType

import numpy as np

from .attribute import grad_times_input, lrp_epsilon
from .faithfulness import (
    MAX_UNITS_PER_CASE,
    MAX_VALIDATION_CASES,
    AttributionValidationCase,
    RankingSensitivityGate,
    _canonical_identifier,
    _numeric_array,
    _validate_gate,
    ranking_gate_content_sha256,
    ranking_gate_manifest,
    ranking_sensitivity_check,
)
from .model import SmallTransformer
from .runlog import AttributionRecord

METHODS = MappingProxyType(
    {
        "lrp_epsilon": lrp_epsilon,
        "grad_x_input": grad_times_input,
    }
)
METHOD_IMPLEMENTATIONS = MappingProxyType(
    {
        "lrp_epsilon": (
            "detached_attention_value_path_epsilon_lrp_v2_not_attnlrp;eps=1e-6"
        ),
        "grad_x_input": (
            "central_finite_difference_gradient_times_input_v2;h=1e-5;"
            "adaptive_relative_step=cbrt_f64_epsilon"
        ),
    }
)
MAX_PROBE_MULTIPLY_ADDS = 1_000_000_000


@dataclass(frozen=True)
class ProbeValidationCase:
    """One group-disjoint held-out input and its explicit replacement baseline."""

    case_id: str
    group_id: str
    unit_ids: tuple[str, ...]
    x: np.ndarray
    baseline: np.ndarray


def _optional_float(value: float | None) -> str:
    return "not_computed" if value is None else format(value, ".17g")


def _relevance_set_hash(
    case_ids: Sequence[str], relevance_arrays: Sequence[np.ndarray]
) -> str:
    """Bind every evaluated relevance array while logging one representative map."""

    digest = hashlib.sha256()
    for case_id, relevance in zip(case_ids, relevance_arrays, strict=True):
        encoded_id = case_id.encode()
        canonical = np.ascontiguousarray(relevance, dtype="<f8")
        digest.update(len(encoded_id).to_bytes(8, "little"))
        digest.update(encoded_id)
        digest.update(len(canonical.shape).to_bytes(8, "little"))
        for dimension in canonical.shape:
            digest.update(int(dimension).to_bytes(8, "little"))
        digest.update(canonical.tobytes(order="C"))
    return digest.hexdigest()


def _validation_input_baseline_set_hash(
    cases: Sequence[ProbeValidationCase],
) -> str:
    """Bind declared case/group/unit identities and exact input/baseline tensors."""

    digest = hashlib.sha256()

    def update_bytes(value: bytes) -> None:
        digest.update(len(value).to_bytes(8, "little"))
        digest.update(value)

    for case in cases:
        update_bytes(case.case_id.encode())
        update_bytes(case.group_id.encode())
        digest.update(len(case.unit_ids).to_bytes(8, "little"))
        for unit_id in case.unit_ids:
            update_bytes(unit_id.encode())
        for array in (case.x, case.baseline):
            canonical = np.ascontiguousarray(array, dtype="<f8")
            digest.update(len(canonical.shape).to_bytes(8, "little"))
            for dimension in canonical.shape:
                digest.update(int(dimension).to_bytes(8, "little"))
            update_bytes(canonical.tobytes(order="C"))
    return digest.hexdigest()


def _validated_methods(methods: object) -> tuple[str, ...]:
    if isinstance(methods, (str, bytes, np.ndarray)) or not isinstance(
        methods, Sequence
    ):
        raise ValueError("methods must be a sequence of attribution method names")
    if not methods:
        raise ValueError("methods must be nonempty")
    if len(methods) > len(METHODS):
        raise ValueError("methods contains more entries than the supported method set")
    validated: list[str] = []
    for index, name in enumerate(methods):
        name = _canonical_identifier(name, f"methods[{index}]")
        if name not in METHODS:
            raise ValueError(f"unknown attribution method: {name!r}")
        validated.append(name)
    if len(set(validated)) != len(validated):
        raise ValueError("methods must not contain duplicates")
    return tuple(validated)


def _validated_probe_cases(
    model: SmallTransformer,
    cases: object,
    gate: RankingSensitivityGate,
) -> tuple[ProbeValidationCase, ...]:
    if isinstance(cases, (str, bytes, np.ndarray)) or not isinstance(cases, Sequence):
        raise ValueError("cases must be a sequence of ProbeValidationCase values")
    if not cases:
        raise ValueError("cases must be nonempty")
    if len(cases) > MAX_VALIDATION_CASES:
        raise ValueError(f"cases must contain at most {MAX_VALIDATION_CASES} entries")

    prepared: list[ProbeValidationCase] = []
    case_ids: set[str] = set()
    group_ids: set[str] = set()
    validation_units: set[str] = set()
    for index, case in enumerate(cases):
        if not isinstance(case, ProbeValidationCase):
            raise ValueError(f"cases[{index}] must be a ProbeValidationCase")
        case_id = _canonical_identifier(case.case_id, f"cases[{index}].case_id")
        if case_id in case_ids:
            raise ValueError("case_id values must be unique")
        case_ids.add(case_id)
        group_id = _canonical_identifier(case.group_id, f"cases[{index}].group_id")
        if group_id in group_ids:
            raise ValueError("validation groups must be disjoint")
        group_ids.add(group_id)
        if type(case.unit_ids) is not tuple or not case.unit_ids:
            raise ValueError(f"case {case_id!r} unit_ids must be a nonempty tuple")
        if len(case.unit_ids) > MAX_UNITS_PER_CASE:
            raise ValueError(
                f"case {case_id!r} unit_ids must contain at most "
                f"{MAX_UNITS_PER_CASE} values"
            )
        unit_ids = tuple(
            _canonical_identifier(unit, f"case {case_id!r} unit_ids")
            for unit in case.unit_ids
        )
        if len(set(unit_ids)) != len(unit_ids):
            raise ValueError(f"case {case_id!r} unit_ids must not contain duplicates")
        if validation_units.intersection(unit_ids):
            raise ValueError("validation units must be disjoint across cases")
        validation_units.update(unit_ids)
        x = model.validate_input(case.x)
        baseline = model.validate_input(case.baseline)
        if baseline.shape != x.shape:
            raise ValueError(f"case {case_id!r} baseline shape must match x")
        if x.size < gate.n_steps:
            raise ValueError(
                f"case {case_id!r} has fewer features than the frozen n_steps"
            )
        prepared.append(
            ProbeValidationCase(
                case_id=case_id,
                group_id=group_id,
                unit_ids=unit_ids,
                x=x,
                baseline=baseline,
            )
        )
    if gate.validation_split == gate.selection_split:
        raise ValueError("selection and validation splits must be disjoint")
    if group_ids.intersection(
        gate.selection_group_ids
    ) or validation_units.intersection(gate.selection_unit_ids):
        raise ValueError("selection and validation identifiers must be disjoint")
    if len(group_ids) < gate.min_groups:
        raise ValueError(
            "probe requires at least the frozen number of independent validation groups"
        )
    return tuple(prepared)


def _probe_work_estimate(
    model: SmallTransformer,
    cases: Sequence[ProbeValidationCase],
    gate: RankingSensitivityGate,
    methods: Sequence[str],
) -> int:
    total = 0
    gate_forward_calls = 3 + (1 + gate.n_random_rankings) * gate.n_steps
    for case in cases:
        token_count = case.x.shape[0]
        forward_work = model.estimated_forward_multiply_adds(token_count)
        for method in methods:
            if method == "lrp_epsilon":
                # One ordinary forward plus every dense reverse-pass product in
                # attribute.lrp_epsilon. A fixed number of forward equivalents is
                # not conservative when d_in exceeds the token count.
                reverse_work = (
                    2 * model.d_model
                    + 4 * token_count * model.d_model * model.d_model
                    + token_count * token_count * model.d_model
                    + 2 * token_count * model.d_in * model.d_model
                )
                attribution_work = forward_work + reverse_work
            elif method == "grad_x_input":
                attribution_work = 2 * case.x.size * forward_work
            else:  # pragma: no cover - protected by _validated_methods
                raise AssertionError(f"unplanned method {method}")
            total += attribution_work + gate_forward_calls * forward_work
            if total > MAX_PROBE_MULTIPLY_ADDS:
                raise ValueError(
                    "complete attribution probe exceeds the "
                    f"{MAX_PROBE_MULTIPLY_ADDS}-multiply-add resource budget"
                )
    return total


def _exact_array_bundle(array: np.ndarray) -> dict[str, object]:
    canonical = np.ascontiguousarray(array, dtype="<f8")
    return {
        "dtype": "<f8",
        "shape": [int(value) for value in canonical.shape],
        "data_base64": base64.b64encode(canonical.tobytes(order="C")).decode("ascii"),
    }


def _source_sha256() -> dict[str, str]:
    directory = Path(__file__).resolve().parent
    result: dict[str, str] = {}
    for name in (
        "attribute.py",
        "faithfulness.py",
        "model.py",
        "probe.py",
        "runlog.py",
    ):
        result[name] = hashlib.sha256((directory / name).read_bytes()).hexdigest()
    return result


def _model_bundle(model: SmallTransformer) -> dict[str, object]:
    return {
        "kind": "small_transformer_v1",
        "d_in": model.d_in,
        "d_model": model.d_model,
        "parameter_sha256": model.parameter_sha256(),
        "parameters": {
            name: _exact_array_bundle(getattr(model, name))
            for name in ("w_embed", "w_q", "w_k", "w_v", "w_o", "w_head")
        },
    }


def _evidence_bundle(
    *,
    model: SmallTransformer,
    method: str,
    primary_method: str,
    gate: RankingSensitivityGate,
    cases: Sequence[ProbeValidationCase],
    relevance_arrays: Sequence[np.ndarray],
    result: object,
    target_output: str,
    modality: str | None,
    layer: str | None,
    work_estimate: int,
) -> dict[str, object]:
    return {
        "schema": "prisoma-attribution-evidence-v2",
        "software": {
            "python": platform.python_version(),
            "numpy": np.__version__,
            "source_sha256": _source_sha256(),
        },
        "method": method,
        "method_implementation": METHOD_IMPLEMENTATIONS[method],
        "primary_method": primary_method,
        "target_output": target_output,
        "modality": modality or "not_declared",
        "layer": layer or "not_declared",
        "work_estimate_multiply_adds": work_estimate,
        "model": _model_bundle(model),
        "gate": ranking_gate_manifest(gate),
        "gate_content_sha256": ranking_gate_content_sha256(gate),
        "frozen_gate_id": gate.frozen_gate_id,
        "case_set_sha256": _validation_input_baseline_set_hash(cases),
        "cases": [
            {
                "case_id": case.case_id,
                "group_id": case.group_id,
                "unit_ids": list(case.unit_ids),
                "x": _exact_array_bundle(case.x),
                "baseline": _exact_array_bundle(case.baseline),
                "relevance": _exact_array_bundle(relevance),
                "group_contrast_f64": (
                    "not_computed"
                    if index >= len(result.group_contrasts)
                    else float(result.group_contrasts[index]).hex()
                ),
                "group_randomization_p_f64": (
                    "not_computed"
                    if index >= len(result.group_randomization_p_values)
                    else float(result.group_randomization_p_values[index]).hex()
                ),
            }
            for index, (case, relevance) in enumerate(
                zip(cases, relevance_arrays, strict=True)
            )
        ],
        "decision": {
            "diagnostic": result.diagnostic,
            "status": result.status,
            "reason": result.reason,
            "passed": bool(result.passed),
            "method_sensitivity_f64": _optional_float(result.method_sensitivity),
            "random_sensitivity_f64": _optional_float(result.random_sensitivity),
            "random_sensitivity_std_f64": _optional_float(
                result.random_sensitivity_std
            ),
            "group_win_binomial_p_f64": _optional_float(
                result.group_win_binomial_p_value
            ),
            "winning_groups": result.winning_groups,
            "independent_groups": result.n_groups,
        },
    }


def run_attribution_probe(
    model: SmallTransformer,
    cases: Sequence[ProbeValidationCase],
    *,
    gate: RankingSensitivityGate,
    target_output: str = "scalar_target",
    methods: Sequence[str] = ("lrp_epsilon", "grad_x_input"),
    primary_method: str = "lrp_epsilon",
    modality: str | None = None,
    layer: str | None = None,
) -> list[AttributionRecord]:
    """Run each method against the same frozen validation cases.

    A single case is intentionally unsupported.  The low-level gate returns a typed
    abstention when the well-formed case set has fewer than ``gate.min_groups``.
    """

    if not isinstance(model, SmallTransformer):
        raise ValueError("model must be a SmallTransformer")
    _validate_gate(gate)
    validated_methods = _validated_methods(methods)
    primary_method = _canonical_identifier(primary_method, "primary_method")
    if primary_method not in validated_methods:
        raise ValueError("primary_method must name exactly one requested method")
    target_output = _canonical_identifier(target_output, "target_output")
    if modality is not None:
        modality = _canonical_identifier(modality, "modality")
    if layer is not None:
        layer = _canonical_identifier(layer, "layer")
    model.validate_parameters()
    prepared_cases = _validated_probe_cases(model, cases, gate)
    _probe_work_estimate(model, prepared_cases, gate, validated_methods)
    method_work_estimates = {
        name: _probe_work_estimate(model, prepared_cases, gate, (name,))
        for name in validated_methods
    }

    records: list[AttributionRecord] = []
    for name in validated_methods:
        relevance_arrays = []
        for case in prepared_cases:
            relevance = _numeric_array(
                METHODS[name](model, case.x),
                f"method {name!r} case {case.case_id!r} relevance",
            )
            if relevance.shape != case.x.shape:
                raise ValueError(
                    f"method {name!r} case {case.case_id!r} relevance shape "
                    "must match x"
                )
            relevance_arrays.append(relevance)
        validation_cases = [
            AttributionValidationCase(
                case_id=case.case_id,
                group_id=case.group_id,
                unit_ids=case.unit_ids,
                x=case.x,
                attribution=relevance,
                baseline=case.baseline,
            )
            for case, relevance in zip(prepared_cases, relevance_arrays, strict=True)
        ]
        result = ranking_sensitivity_check(model.forward, validation_cases, gate=gate)
        metadata = {
            "diagnostic": result.diagnostic,
            "gate_status": result.status,
            "gate_reason": result.reason,
            "frozen_gate_id": result.frozen_gate_id,
            "validation_split": gate.validation_split,
            "selection_split": gate.selection_split,
            "grouping_provenance": gate.grouping_provenance,
            "baseline_provenance": result.baseline_provenance,
            "method_mean_absolute_deletion_sensitivity": _optional_float(
                result.method_sensitivity
            ),
            "random_mean_absolute_deletion_sensitivity": _optional_float(
                result.random_sensitivity
            ),
            "random_deletion_sensitivity_std": _optional_float(
                result.random_sensitivity_std
            ),
            "group_win_binomial_p": _optional_float(result.group_win_binomial_p_value),
            "alpha": format(result.alpha, ".17g"),
            "validation_cases": str(result.n_cases),
            "independent_groups": str(result.n_groups),
            "winning_groups": str(result.winning_groups),
            "deletion_steps": str(result.n_steps),
            "random_rankings_per_case": str(result.n_random_rankings),
            "random_reference_se_bound": format(
                result.random_reference_se_bound, ".17g"
            ),
            "group_contrasts": json.dumps(
                result.group_contrasts, separators=(",", ":")
            ),
            "group_randomization_p_values": json.dumps(
                result.group_randomization_p_values, separators=(",", ":")
            ),
            "ordered_group_ids": json.dumps(
                [case.group_id for case in prepared_cases], separators=(",", ":")
            ),
            "representative_case_id": prepared_cases[0].case_id,
            "validation_relevance_set_sha256": _relevance_set_hash(
                [case.case_id for case in prepared_cases], relevance_arrays
            ),
            "validation_input_baseline_set_sha256": (
                _validation_input_baseline_set_hash(prepared_cases)
            ),
            "baseline_may_be_out_of_distribution": "true",
            "feature_dependence_unresolved": "true",
            "causal_or_mechanistic_faithfulness_established": "false",
            "method_implementation": METHOD_IMPLEMENTATIONS[name],
            "model_parameter_sha256": model.parameter_sha256(),
            "gate_content_sha256": ranking_gate_content_sha256(gate),
            "probe_work_estimate_multiply_adds": str(method_work_estimates[name]),
            "confirmatory_role": "primary" if name == primary_method else "secondary",
            "multiplicity_policy": "one_predeclared_primary_method",
        }
        evidence_bundle = _evidence_bundle(
            model=model,
            method=name,
            primary_method=primary_method,
            gate=gate,
            cases=prepared_cases,
            relevance_arrays=relevance_arrays,
            result=result,
            target_output=target_output,
            modality=modality,
            layer=layer,
            work_estimate=method_work_estimates[name],
        )
        records.append(
            AttributionRecord(
                method=name,
                target_output=target_output,
                # The schema stores one array per event.  Log the declared
                # representative map and bind the complete validation set above.
                relevance=relevance_arrays[0],
                faithfulness_passed=bool(result.passed and name == primary_method),
                layer=layer,
                modality=modality,
                baseline=gate.baseline_name,
                metadata=metadata,
                evidence_bundle=evidence_bundle,
            )
        )
    return records
