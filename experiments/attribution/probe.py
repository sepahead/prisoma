"""Attribute a held-out case set, validate ranking sensitivity, and log it.

The canonical run-log schema retains a historical ``faithfulness_check`` boolean.
This orchestrator sets it to true only when the frozen, group-disjoint deletion
ranking-sensitivity gate passes.  The narrower diagnostic name, decision reason,
randomization evidence, baseline provenance, and non-causal limitations are included
in metadata.
"""

from __future__ import annotations

import hashlib
import json
from collections.abc import Sequence
from dataclasses import dataclass
from types import MappingProxyType

import numpy as np

from .attribute import grad_times_input, lrp_epsilon
from .faithfulness import (
    MAX_ABLATION_EVALUATIONS,
    MAX_UNITS_PER_CASE,
    MAX_VALIDATION_CASES,
    AttributionValidationCase,
    RankingSensitivityGate,
    _canonical_identifier,
    _numeric_array,
    _validate_gate,
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

    # Attribution can contain ties, so enforce the worst-case method-tie plus random
    # deletion plan before running even one attribution forward pass.
    worst_case_evaluations = len(cases) + (
        2 * len(cases) * gate.n_random_rankings * gate.n_steps
    )
    if worst_case_evaluations > MAX_ABLATION_EVALUATIONS:
        raise ValueError(
            "frozen probe design exceeds the ablation-evaluation resource budget"
        )

    prepared: list[ProbeValidationCase] = []
    case_ids: set[str] = set()
    for index, case in enumerate(cases):
        if not isinstance(case, ProbeValidationCase):
            raise ValueError(f"cases[{index}] must be a ProbeValidationCase")
        case_id = _canonical_identifier(case.case_id, f"cases[{index}].case_id")
        if case_id in case_ids:
            raise ValueError("case_id values must be unique")
        case_ids.add(case_id)
        group_id = _canonical_identifier(case.group_id, f"cases[{index}].group_id")
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
    return tuple(prepared)


def run_attribution_probe(
    model: SmallTransformer,
    cases: Sequence[ProbeValidationCase],
    *,
    gate: RankingSensitivityGate,
    target_output: str = "scalar_target",
    methods: Sequence[str] = ("lrp_epsilon", "grad_x_input"),
    modality: str | None = None,
) -> list[AttributionRecord]:
    """Run each method against the same frozen validation cases.

    A single case is intentionally unsupported.  The low-level gate returns a typed
    abstention when the well-formed case set has fewer than ``gate.min_groups``.
    """

    if not isinstance(model, SmallTransformer):
        raise ValueError("model must be a SmallTransformer")
    _validate_gate(gate)
    validated_methods = _validated_methods(methods)
    target_output = _canonical_identifier(target_output, "target_output")
    if modality is not None:
        modality = _canonical_identifier(modality, "modality")
    model.validate_parameters()
    prepared_cases = _validated_probe_cases(model, cases, gate)

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
            "method_aopc": _optional_float(result.method_aopc),
            "random_aopc": _optional_float(result.random_aopc),
            "random_std": _optional_float(result.random_std),
            "group_sign_test_p": _optional_float(result.p_value),
            "alpha": format(result.alpha, ".17g"),
            "validation_cases": str(result.n_cases),
            "independent_groups": str(result.n_groups),
            "positive_groups": str(result.positive_groups),
            "faithfulness_steps": str(result.n_steps),
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
        }
        records.append(
            AttributionRecord(
                method=name,
                target_output=target_output,
                # The schema stores one array per event.  Log the declared
                # representative map and bind the complete validation set above.
                relevance=relevance_arrays[0],
                faithfulness_passed=result.passed,
                modality=modality,
                baseline=gate.baseline_name,
                metadata=metadata,
            )
        )
    return records
