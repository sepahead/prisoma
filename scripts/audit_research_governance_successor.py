#!/usr/bin/env python3
"""Validate the typed, still-unfrozen M0 successor governance draft.

The checked-in v2 artifact is a reviewed future-freeze contract, not a preregistration or
scientific result.  It binds the additional structure that a future freeze candidate
must provide while leaving every scientific value null.  The validator also supports
validating an explicitly materialized ``freeze_candidate_under_review`` or ``frozen``
document, so the cross-field rules are executable before real values exist.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import stat
import sys
import unicodedata
from datetime import date, datetime
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SUCCESSOR_PATH = Path("protocols/m0_preregistration_successor_draft_v2.json")
FREEZE_BLOCKED_EXIT = 3

MAX_SUCCESSOR_DOCUMENT_BYTES = 2 * 1024 * 1024
MAX_BOUND_CONTENT_BYTES = 64 * 1024 * 1024
MAX_BOUND_CONTENT_TOTAL_BYTES = 256 * 1024 * 1024
MAX_ARRAY_ITEMS = 4096
MAX_STRING_BYTES = 64 * 1024
MAX_REPOSITORY_PATH_BYTES = 4096

EXPECTED_SCOPE = (
    "reviewed typed successor draft for future M0 freeze review; it contains no "
    "frozen scientific value, does not promote any claim, and does not authorize "
    "confirmatory holdout access"
)
EXPECTED_BASE_V1_PATH = "protocols/m0_preregistration_skeleton_v1.json"
EXPECTED_FREEZE_REQUIREMENTS = [
    "every required typed slot must be non-null and type-valid before a freeze "
    "candidate can pass",
    "active scientific claims must be H1 and H2 plus exactly one of H3 or H4, "
    "never more than three",
    "H1 Protocol A calibration bins must be prespecified or train-defined without "
    "heldout outcomes",
    "H1 Protocol B must name exactly one primary effect-specific endpoint and one "
    "explicit testing hierarchy",
    "H2 must freeze exactly one censoring-aware proper-score endpoint and bind its "
    "minimum useful margin, censoring assumptions, calibration, actionability, "
    "subgroup, replication, multiplicity, and secondary-only decision-utility rules",
    "H3 primary scoring must retain the complete inherited target ledger with exact "
    "same-fold M1 fallback for every abstention; every warning must follow the frozen "
    "disposition map and every unlisted warning must abstain with exact M1 fallback",
    "H4 must bind the target and sampling design, transport assumptions when needed, "
    "one outcome and primary tuple, simultaneous strong error control, target-weight "
    "uncertainty, and joint design power",
    "EC1 acceptance must be finite, oracle-graded, comparator-benchmarked, "
    "uncertainty-aware, and require complete fault-detection, replay-fidelity, and "
    "valid-case false-positive endpoint coverage for the registered universe; every "
    "registered fault-adapter pair requires its own absolute sensitivity floor and "
    "mandatory pass, with no distribution-average substitution",
    "freeze metadata and content-bound review receipts must be present before the "
    "status can become frozen",
]

FREEZE_BLOCKERS = [
    "M0_SUCCESSOR_DRAFT_UNFROZEN",
    "M0_SUCCESSOR_CLAIM_SELECTION_UNFROZEN",
    "M0_SUCCESSOR_EC1_FINITE_ACCEPTANCE_PROTOCOL_UNFROZEN",
    "M0_SUCCESSOR_H1A_CALIBRATION_POLICY_UNFROZEN",
    "M0_SUCCESSOR_H1B_PRIMARY_EFFECT_ENDPOINT_HIERARCHY_UNFROZEN",
    "M0_SUCCESSOR_H2_PRIMARY_SCORE_CENSORING_SUCCESS_PROTOCOL_UNFROZEN",
    "M0_SUCCESSOR_H3_WARNING_DISPOSITION_UNFROZEN",
    "M0_SUCCESSOR_H4_TARGET_TRANSPORT_TUPLE_INFERENCE_POWER_UNFROZEN",
    "M0_SUCCESSOR_CONTENT_BOUND_FREEZE_RECEIPTS_MISSING",
]

SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
PLACEHOLDER_RE = re.compile(
    r"(?:\b(?:tbd|todo|tk|changeme|placeholder|fixme)\b|<[^>]+>|\?\?\?)",
    re.IGNORECASE,
)

TOP_LEVEL_KEYS = {
    "schema_version",
    "artifact_id",
    "as_of_date",
    "canonical_spec",
    "base_v1_intake_binding",
    "scope",
    "status",
    "freeze_receipt",
    "freeze_revision",
    "frozen_at",
    "global_freeze_slots",
    "claim_selection_contract",
    "typed_protocol_contracts",
    "freeze_requirements",
}
CANONICAL_SPEC_KEYS = {"path", "version"}
CONTENT_BINDING_KEYS = {"path", "sha256"}
SLOT_KEYS = {"value_type", "required_for_freeze", "value"}
ENUM_SLOT_KEYS = SLOT_KEYS | {"allowed_values"}
FREEZE_RECEIPT_KEYS = {
    "schema_version",
    "artifact_id",
    "candidate_artifact_id",
    "candidate_schema_version",
    "candidate_path",
    "candidate_status",
    "authorized_status",
    "canonicalization",
    "canonical_candidate_sha256",
    "reviewed_global_freeze_slot_bindings",
    "frozen_at",
}
FREEZE_CANONICALIZATION = (
    "utf8_json_sorted_keys_compact_separators_ensure_ascii_false_"
    "candidate_status_with_null_freeze_metadata_v1"
)

EXPECTED_GLOBAL_SLOTS = {
    "base_scientific_freeze_bundle_binding": ("content_binding", None),
    "holdout_access_governance_binding": ("content_binding", None),
    "environment_digest_bundle_binding": ("content_binding", None),
    "scientific_and_statistical_review_bundle_binding": ("content_binding", None),
}

EXPECTED_CLAIM_SELECTION_SLOTS = {
    "active_scientific_claims": ("finite_nonempty_string_array", None),
    "selected_h1_protocol": (
        "enum",
        ("h1_protocol_a", "h1_protocol_b"),
    ),
    "selected_h3_or_h4_branch": ("enum", ("H3", "H4")),
    "h3_h4_selection_timing": (
        "enum",
        (
            "before_any_h3_or_h4_confirmatory_outcome",
            "after_h3_with_fresh_holdout_and_sequential_error_control",
        ),
    ),
    "branch_selection_and_error_control_binding": ("content_binding", None),
}

EXPECTED_PROTOCOL_SLOTS = {
    "ec1": {
        "supported_adapter_ids": ("finite_nonempty_string_array", None),
        "declared_capability_matrix_binding": ("content_binding", None),
        "required_causal_temporal_variable_inventory_binding": (
            "content_binding",
            None,
        ),
        "fault_class_registry": ("ec1_fault_class_array", None),
        "valid_case_registry_binding": ("content_binding", None),
        "sampling_unit_and_dependence_binding": ("content_binding", None),
        "fault_sampling_distribution_binding": ("content_binding", None),
        "independent_oracle_binding": ("content_binding", None),
        "comparator_stack_binding": ("content_binding", None),
        "primary_endpoint_registry": ("ec1_endpoint_array", None),
        "fault_detection_endpoint_map": ("ec1_fault_endpoint_map", None),
        "replay_fidelity_endpoint_map": ("ec1_adapter_endpoint_map", None),
        "false_positive_endpoint_map": ("ec1_adapter_endpoint_map", None),
        "replay_tolerance_table_binding": ("content_binding", None),
        "false_positive_endpoint_id": ("string", None),
        "multiplicity_procedure_binding": ("content_binding", None),
        "uncertainty_and_design_analysis_binding": ("content_binding", None),
        "blind_challenge_protocol_binding": ("content_binding", None),
        "pass_fail_decision_rule_binding": ("content_binding", None),
        "external_reproduction_and_second_adapter_binding": (
            "content_binding",
            None,
        ),
    },
    "h1_protocol_a": {
        "calibration_bin_origin": (
            "enum",
            (
                "prespecified_before_holdout",
                "train_defined_from_predictions_only",
            ),
        ),
        "calibration_bin_definition_binding": ("content_binding", None),
        "calibration_role": (
            "enum",
            ("primary_gate", "secondary_gatekeeping", "secondary_descriptive"),
        ),
        "heldout_outcomes_used_to_define_bins": ("boolean", None),
        "response_scale_score_and_uncertainty_binding": ("content_binding", None),
    },
    "h1_protocol_b": {
        "primary_effect_endpoint_id": ("string", None),
        "effect_endpoint_registry": ("h1b_effect_endpoint_array", None),
        "decision_hierarchy": ("decision_hierarchy_array", None),
        "multiplicity_procedure_binding": ("content_binding", None),
        "success_rule_binding": ("content_binding", None),
    },
    "h2": {
        "target_population_and_intended_use_binding": ("content_binding", None),
        "landmark_time_zero_eligibility_update_schedule_binding": (
            "content_binding",
            None,
        ),
        "prediction_horizon_and_target_type_binding": ("content_binding", None),
        "episode_case_persistent_world_grouping_binding": (
            "content_binding",
            None,
        ),
        "feature_cutoff_deployment_availability_train_fit_binding": (
            "content_binding",
            None,
        ),
        "event_ontology_named_failure_competing_events_binding": (
            "content_binding",
            None,
        ),
        "censoring_missingness_assumptions_binding": ("content_binding", None),
        "censoring_model_strata_crossfit_positivity_sensitivity_binding": (
            "content_binding",
            None,
        ),
        "sampled_and_target_prevalence_binding": ("content_binding", None),
        "matched_access_comparator_registry_and_budgets_binding": (
            "content_binding",
            None,
        ),
        "primary_proper_score": ("h2_primary_proper_score", None),
        "calibration_intercept_slope_reliability_binding": (
            "content_binding",
            None,
        ),
        "conformal_plan_or_not_used_binding": ("content_binding", None),
        "alarm_policy_binding": ("content_binding", None),
        "nondetection_retaining_lead_time_binding": ("content_binding", None),
        "decision_utility_cost_capacity_latency_binding": (
            "content_binding",
            None,
        ),
        "uncertainty_cluster_and_independent_counts_binding": (
            "content_binding",
            None,
        ),
        "nested_outer_split_binding": ("content_binding", None),
        "external_or_later_time_holdout_binding": ("content_binding", None),
        "separate_recalibration_split_binding": ("content_binding", None),
        "shift_subgroups_and_degradation_bounds_binding": (
            "content_binding",
            None,
        ),
        "multiplicity_procedure_binding": ("content_binding", None),
        "replication_target_binding": ("content_binding", None),
        "success_rule": ("h2_success_rule", None),
        "permitted_interpretation_binding": ("content_binding", None),
    },
    "h3": {
        "warning_code_registry_binding": ("content_binding", None),
        "warning_dispositions": ("warning_disposition_array", None),
        "allowlisted_use_output_warning_codes": ("finite_string_array", None),
        "warning_disposition_receipt_binding": ("content_binding", None),
        "active_parent_claim": ("enum", ("H1", "H2")),
    },
    "h4": {
        "target_population_binding": ("content_binding", None),
        "confirmatory_sample_source": (
            "enum",
            (
                "probability_sample_from_target",
                "finite_benchmark_equals_target",
                "transported_randomized_sample",
            ),
        ),
        "sampling_design_binding": ("content_binding", None),
        "selection_indicator_and_sampling_overlap_binding": (
            "content_binding",
            None,
        ),
        "conditional_effect_transport_assumptions_binding": (
            "content_binding",
            None,
        ),
        "target_covariate_sample_binding": ("content_binding", None),
        "target_weight_estimand_and_source_binding": ("content_binding", None),
        "target_weight_uncertainty_binding": ("content_binding", None),
        "baseline_region_partition_binding": ("content_binding", None),
        "primary_tuple": ("h4_primary_tuple", None),
        "randomized_unit_and_assignment_binding": ("content_binding", None),
        "positivity_engagement_receipt_support_binding": (
            "content_binding",
            None,
        ),
        "simultaneous_inference_plan": (
            "h4_simultaneous_inference_plan",
            None,
        ),
        "joint_design_power_plan": ("h4_joint_design_power_plan", None),
        "positive_and_negative_controls_binding": ("content_binding", None),
        "replication_construction_binding": ("content_binding", None),
        "permitted_interpretation_binding": ("content_binding", None),
    },
}

PROTOCOL_KEYS = {
    "ec1": {
        "registered_claim_id",
        "claim_class",
        "unregistered_fault_policy",
        "slots",
    },
    "h1_protocol_a": {
        "registered_claim_id",
        "protocol_label",
        "heldout_outcome_defined_bins_policy",
        "slots",
    },
    "h1_protocol_b": {
        "registered_claim_id",
        "protocol_label",
        "one_primary_effect_endpoint_required",
        "slots",
    },
    "h2": {
        "registered_claim_id",
        "protocol_label",
        "one_primary_proper_score_required",
        "decision_utility_primary_role_forbidden",
        "slots",
    },
    "h3": {
        "registered_claim_id",
        "unlisted_warning_disposition",
        "allowed_warning_dispositions",
        "common_comparison_population_contract",
        "slots",
    },
    "h4": {
        "registered_claim_id",
        "protocol_label",
        "individual_effect_prevalence_claim_forbidden",
        "slots",
    },
}

H4_POWER_SCENARIOS = {
    "task_family_heterogeneity",
    "cluster_count_and_size",
    "cell_positivity",
    "availability_effect_dependence",
    "target_weight_estimation",
    "controls_and_support_failures",
    "abstention_and_missingness",
    "region_discovery_split",
    "margins_and_minimum_region_mass",
    "multiplicity_and_simultaneous_inference",
    "null_weak_boundary_and_alternative_regimes",
}

H1B_ENDPOINT_DIRECTIONS = {
    "causal_effect_prediction_loss": {"lower_is_better"},
    "causal_calibration": {
        "lower_is_better",
        "inside_equivalence_region",
    },
    "rank_or_prioritization_statistic": {"higher_is_better"},
    "policy_value": {"higher_is_better"},
    "policy_regret": {"lower_is_better"},
    "factual_outcome_proper_loss": {"lower_is_better"},
}

H3_COMMON_COMPARISON_KEYS = {
    "contract_version",
    "allowed_policies",
    "selected_policy",
    "population_binding",
    "candidate_id_ledger",
    "m1_coverage",
    "m2_coverage",
    "m2_status_timing",
    "m2_allowed_statuses",
    "fallback_rule",
    "paired_scoring_rule",
    "primary_denominator_rule",
    "eligible_only_analysis_role",
    "required_reporting",
    "fail_closed_conditions",
}
H3_ALLOWED_COMPARISON_POLICIES = [
    "identical_candidate_unit_denominator_for_m1_and_m2",
    "full_target_population_with_exact_m1_fallback_on_every_m2_abstention",
]
H3_SELECTED_COMPARISON_POLICY = (
    "full_target_population_with_exact_m1_fallback_on_every_m2_abstention"
)
EXPECTED_H3_COMMON_COMPARISON_FIELDS = {
    "population_binding": (
        "inherit_exact_target_population_unit_cluster_eligibility_time_zero_sampling_"
        "weights_and_outer_holdout_from_one_active_parent_h1_or_h2_estimand"
    ),
    "candidate_id_ledger": (
        "one_content_bound_canonical_ordered_ledger_of_unique_nonempty_candidate_ids_"
        "frozen_before_outer_holdout_outcome_access"
    ),
    "m1_coverage": (
        "exactly_one_heldout_prediction_or_decision_for_every_candidate_id"
    ),
    "m2_coverage": (
        "for_every_same_candidate_id_exactly_one_pid_augmented_output_or_one_typed_"
        "abstention"
    ),
    "m2_status_timing": (
        "derived_only_from_frozen_support_and_allowed_pre_outcome_inputs_without_"
        "outer_holdout_outcome_access"
    ),
    "fallback_rule": (
        "every_typed_m2_abstention_uses_the_exact_recorded_m1_output_for_the_same_"
        "candidate_id_and_outer_fold"
    ),
    "paired_scoring_rule": (
        "score_m1_and_the_deployed_m2_fallback_policy_on_the_same_ids_outcomes_"
        "weights_clusters_outer_folds_and_primary_endpoint"
    ),
    "primary_denominator_rule": (
        "all_ids_in_the_frozen_target_ledger_never_only_pid_eligible_or_"
        "successfully_computed_cases"
    ),
    "eligible_only_analysis_role": (
        "secondary_diagnostic_only_never_primary_or_confirmatory"
    ),
}
EXPECTED_H3_M2_STATUSES = ["produced", "produced_with_warning", "abstained"]
EXPECTED_H3_REQUIRED_REPORTING = [
    "frozen_target_candidate_count",
    "frozen_target_candidate_ledger_sha256",
    "content_bound_per_candidate_paired_scoring_receipt_sha256",
    "m2_produced_count",
    "m2_produced_with_warning_count",
    "m2_abstained_count_by_reason",
    "m1_fallback_count",
    "full_population_paired_incremental_endpoint",
]
EXPECTED_H3_FAIL_CLOSED_CONDITIONS = [
    "candidate_id_ledger_missing_changed_or_not_content_bound",
    "empty_duplicate_missing_extra_or_reordered_candidate_id",
    "m1_output_missing_for_any_candidate_id",
    "m2_has_both_output_and_abstention_or_has_neither_one",
    "m2_status_is_not_requested_or_unknown",
    "m2_status_or_abstention_reason_uses_outer_holdout_outcome_or_post_time_zero_data",
    "abstained_pid_estimate_contains_a_numeric_placeholder",
    "deployed_m2_fallback_differs_from_same_fold_m1_output",
    "fallback_is_applied_when_m2_status_is_not_abstained",
    "outcome_weight_cluster_fold_endpoint_or_population_binding_differs_between_m1_and_m2",
    "primary_analysis_restricts_to_pid_eligible_or_successfully_computed_cases",
    "target_ledger_or_eligibility_changes_after_outer_holdout_outcome_access",
]


class SuccessorGovernanceError(ValueError):
    """The successor draft or candidate is malformed or scientifically unsafe."""


def _reject_constant(token: str) -> None:
    raise SuccessorGovernanceError(f"non-finite JSON number {token!r} is forbidden")


def _object_without_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise SuccessorGovernanceError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def _parse_json_bytes(raw: bytes, *, context: str) -> Any:
    if len(raw) > MAX_SUCCESSOR_DOCUMENT_BYTES:
        raise SuccessorGovernanceError(
            f"{context} exceeds the {MAX_SUCCESSOR_DOCUMENT_BYTES}-byte limit"
        )
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise SuccessorGovernanceError(f"{context} is not UTF-8") from error
    try:
        value = json.loads(
            text,
            object_pairs_hook=_object_without_duplicate_keys,
            parse_constant=_reject_constant,
        )
    except SuccessorGovernanceError:
        raise
    except json.JSONDecodeError as error:
        raise SuccessorGovernanceError(
            f"{context} is invalid JSON at line {error.lineno}, column {error.colno}"
        ) from error
    except RecursionError as error:
        raise SuccessorGovernanceError(
            f"{context} exceeds the supported JSON nesting depth"
        ) from error
    try:
        _reject_placeholders_and_nonfinite(value, context=context)
    except RecursionError as error:
        raise SuccessorGovernanceError(
            f"{context} exceeds the supported JSON nesting depth"
        ) from error
    return value


def _reject_placeholders_and_nonfinite(value: Any, *, context: str) -> None:
    if isinstance(value, dict):
        for key, item in value.items():
            if not isinstance(key, str):
                raise SuccessorGovernanceError(
                    f"{context} contains a non-string object key"
                )
            _reject_surrogates(key, context=f"{context} key")
            _reject_placeholders_and_nonfinite(item, context=f"{context}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            _reject_placeholders_and_nonfinite(item, context=f"{context}[{index}]")
    elif isinstance(value, str):
        _reject_surrogates(value, context=context)
        if PLACEHOLDER_RE.search(value):
            raise SuccessorGovernanceError(f"{context} contains a placeholder token")
    elif isinstance(value, float) and not math.isfinite(value):
        raise SuccessorGovernanceError(f"{context} contains a non-finite number")


def _reject_surrogates(value: str, *, context: str) -> None:
    if any(0xD800 <= ord(character) <= 0xDFFF for character in value):
        raise SuccessorGovernanceError(
            f"{context} contains a Unicode surrogate code point"
        )


def _exact_keys(value: Any, expected: set[str], *, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise SuccessorGovernanceError(f"{context} must be an object")
    missing = sorted(expected - value.keys())
    unknown = sorted(value.keys() - expected)
    if missing or unknown:
        details: list[str] = []
        if missing:
            details.append(f"missing={missing}")
        if unknown:
            details.append(f"unknown={unknown}")
        raise SuccessorGovernanceError(
            f"{context} has invalid fields: {', '.join(details)}"
        )
    return value


def _string(value: Any, *, context: str) -> str:
    if not isinstance(value, str) or not value:
        raise SuccessorGovernanceError(f"{context} must be a non-empty string")
    if value != value.strip():
        raise SuccessorGovernanceError(
            f"{context} must not contain leading or trailing whitespace"
        )
    _reject_surrogates(value, context=context)
    if value != unicodedata.normalize("NFC", value):
        raise SuccessorGovernanceError(f"{context} must use canonical Unicode NFC form")
    if any(
        ord(character) < 32 or 0x7F <= ord(character) <= 0x9F for character in value
    ):
        raise SuccessorGovernanceError(
            f"{context} contains a forbidden control character"
        )
    if PLACEHOLDER_RE.search(value):
        raise SuccessorGovernanceError(f"{context} contains a placeholder token")
    encoded = value.encode("utf-8")
    if len(encoded) > MAX_STRING_BYTES:
        raise SuccessorGovernanceError(
            f"{context} exceeds the {MAX_STRING_BYTES}-byte string limit"
        )
    return value


def _boolean(value: Any, *, context: str) -> bool:
    if not isinstance(value, bool):
        raise SuccessorGovernanceError(f"{context} must be a boolean")
    return value


def _integer(value: Any, *, context: str, minimum: int | None = None) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise SuccessorGovernanceError(f"{context} must be an integer, not a boolean")
    if minimum is not None and value < minimum:
        raise SuccessorGovernanceError(f"{context} must be >= {minimum}")
    return value


def _number(value: Any, *, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise SuccessorGovernanceError(f"{context} must be a finite number")
    number = float(value)
    if not math.isfinite(number):
        raise SuccessorGovernanceError(f"{context} must be a finite number")
    return number


def _probability(value: Any, *, context: str, strictly_positive: bool = False) -> float:
    number = _number(value, context=context)
    lower_ok = number > 0.0 if strictly_positive else number >= 0.0
    if not lower_ok or number > 1.0:
        boundary = "(0, 1]" if strictly_positive else "[0, 1]"
        raise SuccessorGovernanceError(f"{context} must lie in {boundary}")
    return number


def _array(value: Any, *, context: str) -> list[Any]:
    if not isinstance(value, list):
        raise SuccessorGovernanceError(f"{context} must be an array")
    if len(value) > MAX_ARRAY_ITEMS:
        raise SuccessorGovernanceError(
            f"{context} exceeds the {MAX_ARRAY_ITEMS}-item array limit"
        )
    return value


def _string_array(
    value: Any,
    *,
    context: str,
    allow_empty: bool,
) -> list[str]:
    items = _array(value, context=context)
    if not allow_empty and not items:
        raise SuccessorGovernanceError(f"{context} must not be empty")
    strings = [
        _string(item, context=f"{context}[{index}]") for index, item in enumerate(items)
    ]
    if len(strings) != len(set(strings)):
        raise SuccessorGovernanceError(f"{context} must not contain duplicates")
    return strings


def _date(value: Any, *, context: str) -> date:
    text = _string(value, context=context)
    try:
        parsed = date.fromisoformat(text)
    except ValueError as error:
        raise SuccessorGovernanceError(f"{context} is not a valid ISO date") from error
    if parsed.isoformat() != text:
        raise SuccessorGovernanceError(f"{context} must use canonical YYYY-MM-DD form")
    return parsed


def _timestamp(value: Any, *, context: str) -> datetime:
    text = _string(value, context=context)
    if not text.endswith("Z"):
        raise SuccessorGovernanceError(
            f"{context} must be an RFC 3339 UTC timestamp ending in Z"
        )
    try:
        parsed = datetime.fromisoformat(text[:-1] + "+00:00")
    except ValueError as error:
        raise SuccessorGovernanceError(
            f"{context} is not a valid RFC 3339 timestamp"
        ) from error
    if parsed.isoformat().replace("+00:00", "Z") != text:
        raise SuccessorGovernanceError(
            f"{context} must use canonical RFC 3339 UTC form"
        )
    return parsed


def _validated_relative_parts(relative: Any, *, context: str) -> tuple[str, ...]:
    raw = _string(relative, context=context)
    if len(raw.encode("utf-8")) > MAX_REPOSITORY_PATH_BYTES:
        raise SuccessorGovernanceError(
            f"{context} exceeds the {MAX_REPOSITORY_PATH_BYTES}-byte path limit"
        )
    if "\\" in raw:
        raise SuccessorGovernanceError(f"{context} must use POSIX path separators")
    pure = PurePosixPath(raw)
    if (
        pure.is_absolute()
        or not pure.parts
        or ".." in pure.parts
        or "." in pure.parts
        or pure.as_posix() != raw
    ):
        raise SuccessorGovernanceError(
            f"{context} must be a canonical repository-relative path: {raw!r}"
        )
    return pure.parts


def _snapshot_identity(value: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _node_identity(value: os.stat_result) -> tuple[int, int, int]:
    return (value.st_dev, value.st_ino, value.st_mode)


def _read_bounded_repo_file(
    root: Path,
    relative: Any,
    *,
    max_bytes: int,
    context: str,
) -> bytes:
    """Read a stable repository-relative regular file through descriptor-safe traversal."""

    if max_bytes < 0:
        raise SuccessorGovernanceError(f"{context} has an invalid negative byte limit")
    parts = _validated_relative_parts(relative, context=context)
    root = root.resolve(strict=True)
    directory_flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_DIRECTORY", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    file_flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_NONBLOCK", 0)
    )
    descriptors: list[int] = []
    directory_identities: list[tuple[int, int, int]] = []
    try:
        descriptors.append(os.open(root, directory_flags))
        root_metadata = os.fstat(descriptors[-1])
        if not stat.S_ISDIR(root_metadata.st_mode):
            raise SuccessorGovernanceError(
                f"{context} repository root is not a directory"
            )
        directory_identities.append(_node_identity(root_metadata))
        for part in parts[:-1]:
            descriptor = os.open(part, directory_flags, dir_fd=descriptors[-1])
            descriptors.append(descriptor)
            directory_metadata = os.fstat(descriptor)
            if not stat.S_ISDIR(directory_metadata.st_mode):
                raise SuccessorGovernanceError(
                    f"{context} may traverse only real directories"
                )
            directory_identities.append(_node_identity(directory_metadata))
        descriptor = os.open(parts[-1], file_flags, dir_fd=descriptors[-1])
        descriptors.append(descriptor)
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode):
            raise SuccessorGovernanceError(
                f"{context} must identify a regular non-symlink file"
            )
        if opened.st_size < 0 or opened.st_size > max_bytes:
            raise SuccessorGovernanceError(
                f"{context} exceeds the {max_bytes}-byte limit"
            )

        raw = bytearray()
        while len(raw) <= max_bytes:
            chunk = os.read(
                descriptor,
                min(1024 * 1024, max_bytes + 1 - len(raw)),
            )
            if not chunk:
                break
            raw.extend(chunk)
        if len(raw) > max_bytes:
            raise SuccessorGovernanceError(
                f"{context} exceeds the {max_bytes}-byte limit"
            )
        opened_after = os.fstat(descriptor)
        named_after = os.stat(
            parts[-1],
            dir_fd=descriptors[-2],
            follow_symlinks=False,
        )
        if (
            not stat.S_ISREG(named_after.st_mode)
            or _snapshot_identity(opened) != _snapshot_identity(opened_after)
            or _snapshot_identity(opened_after) != _snapshot_identity(named_after)
            or len(raw) != opened_after.st_size
        ):
            raise SuccessorGovernanceError(f"{context} changed while it was read")

        verification_descriptors: list[int] = []
        try:
            verification_descriptors.append(os.open(root, directory_flags))
            if (
                _node_identity(os.fstat(verification_descriptors[-1]))
                != (directory_identities[0])
            ):
                raise SuccessorGovernanceError(
                    f"{context} repository path changed while it was read"
                )
            for index, part in enumerate(parts[:-1], start=1):
                directory = os.open(
                    part,
                    directory_flags,
                    dir_fd=verification_descriptors[-1],
                )
                verification_descriptors.append(directory)
                if _node_identity(os.fstat(directory)) != directory_identities[index]:
                    raise SuccessorGovernanceError(
                        f"{context} repository path changed while it was read"
                    )
            current_file = os.open(
                parts[-1],
                file_flags,
                dir_fd=verification_descriptors[-1],
            )
            verification_descriptors.append(current_file)
            if _snapshot_identity(os.fstat(current_file)) != _snapshot_identity(
                opened_after
            ):
                raise SuccessorGovernanceError(
                    f"{context} repository path changed while it was read"
                )
        finally:
            for verification_descriptor in reversed(verification_descriptors):
                try:
                    os.close(verification_descriptor)
                except OSError:
                    pass
        return bytes(raw)
    except SuccessorGovernanceError:
        raise
    except OSError as error:
        raise SuccessorGovernanceError(
            f"cannot read stable repository file for {context}: {error}"
        ) from error
    finally:
        for descriptor in reversed(descriptors):
            try:
                os.close(descriptor)
            except OSError:
                pass


class _ContentSnapshotReader:
    """Cache stable content snapshots and enforce one aggregate validation budget."""

    def __init__(self, root: Path) -> None:
        self.root = root.resolve(strict=True)
        self._cache: dict[str, bytes] = {}
        self._aggregate_bytes = 0

    def read(self, relative: Any, *, context: str) -> bytes:
        raw = _string(relative, context=context)
        cached = self._cache.get(raw)
        if cached is not None:
            return cached
        payload = _read_bounded_repo_file(
            self.root,
            raw,
            max_bytes=MAX_BOUND_CONTENT_BYTES,
            context=context,
        )
        if self._aggregate_bytes + len(payload) > MAX_BOUND_CONTENT_TOTAL_BYTES:
            raise SuccessorGovernanceError(
                "content bindings exceed the "
                f"{MAX_BOUND_CONTENT_TOTAL_BYTES}-byte aggregate limit"
            )
        self._aggregate_bytes += len(payload)
        self._cache[raw] = payload
        return payload

    def sha256(self, relative: Any, *, context: str) -> str:
        return hashlib.sha256(self.read(relative, context=context)).hexdigest()


def _content_binding(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> dict[str, str]:
    binding = _exact_keys(value, CONTENT_BINDING_KEYS, context=context)
    path = _string(binding["path"], context=f"{context}.path")
    digest = _string(binding["sha256"], context=f"{context}.sha256")
    if not SHA256_RE.fullmatch(digest):
        raise SuccessorGovernanceError(
            f"{context}.sha256 must be a lowercase SHA-256 digest"
        )
    observed = reader.sha256(path, context=f"{context}.path")
    if digest != observed:
        raise SuccessorGovernanceError(
            f"{context} SHA-256 mismatch: expected {digest}, observed {observed}"
        )
    return {"path": binding["path"], "sha256": digest}


def _canonical_freeze_candidate_bytes(document: dict[str, Any]) -> bytes:
    """Serialize the non-circular candidate payload reviewed before freeze.

    The receipt and terminal metadata cannot be included in their own digest. The
    canonical payload is therefore the complete schema-v2 document with status set
    to ``freeze_candidate_under_review`` and ``freeze_receipt``,
    ``freeze_revision``, and ``frozen_at`` set to null. Keys are sorted, JSON uses
    compact separators and UTF-8 without ASCII escaping, and no trailing newline is
    hashed.
    """

    candidate = dict(document)
    candidate["status"] = "freeze_candidate_under_review"
    candidate["freeze_receipt"] = None
    candidate["freeze_revision"] = None
    candidate["frozen_at"] = None
    try:
        canonical = json.dumps(
            candidate,
            ensure_ascii=False,
            allow_nan=False,
            sort_keys=True,
            separators=(",", ":"),
        )
    except (TypeError, ValueError) as error:
        raise SuccessorGovernanceError(
            f"cannot canonicalize freeze candidate: {error}"
        ) from error
    return canonical.encode("utf-8")


def _canonical_freeze_candidate_sha256(document: dict[str, Any]) -> str:
    return hashlib.sha256(_canonical_freeze_candidate_bytes(document)).hexdigest()


def _validate_freeze_receipt(
    value: Any,
    *,
    document: dict[str, Any],
    reader: _ContentSnapshotReader,
    candidate_digest: str,
    frozen_at: datetime,
) -> dict[str, Any]:
    binding = _content_binding(
        value,
        reader=reader,
        context=f"{SUCCESSOR_PATH}.freeze_receipt",
    )
    receipt_context = f"{SUCCESSOR_PATH}.freeze_receipt document"
    receipt = _exact_keys(
        _parse_json_bytes(
            reader.read(binding["path"], context=f"{receipt_context}.path"),
            context=receipt_context,
        ),
        FREEZE_RECEIPT_KEYS,
        context=receipt_context,
    )
    if (
        _integer(
            receipt["schema_version"],
            context=f"{receipt_context}.schema_version",
        )
        != 1
    ):
        raise SuccessorGovernanceError(f"{receipt_context}.schema_version must equal 1")
    if receipt["artifact_id"] != "prisoma_m0_freeze_receipt_v1":
        raise SuccessorGovernanceError(f"{receipt_context}.artifact_id drifted")
    if receipt["candidate_artifact_id"] != document["artifact_id"]:
        raise SuccessorGovernanceError(
            f"{receipt_context}.candidate_artifact_id does not bind the candidate"
        )
    receipt_candidate_schema = _integer(
        receipt["candidate_schema_version"],
        context=f"{receipt_context}.candidate_schema_version",
    )
    if receipt_candidate_schema != document["schema_version"]:
        raise SuccessorGovernanceError(
            f"{receipt_context}.candidate_schema_version does not bind the candidate"
        )
    if receipt["candidate_path"] != SUCCESSOR_PATH.as_posix():
        raise SuccessorGovernanceError(
            f"{receipt_context}.candidate_path must identify the successor artifact"
        )
    if receipt["candidate_status"] != "freeze_candidate_under_review":
        raise SuccessorGovernanceError(
            f"{receipt_context}.candidate_status must identify the reviewed state"
        )
    if receipt["authorized_status"] != "frozen":
        raise SuccessorGovernanceError(
            f"{receipt_context}.authorized_status must equal 'frozen'"
        )
    if receipt["canonicalization"] != FREEZE_CANONICALIZATION:
        raise SuccessorGovernanceError(
            f"{receipt_context}.canonicalization does not match the validator"
        )
    digest = _string(
        receipt["canonical_candidate_sha256"],
        context=f"{receipt_context}.canonical_candidate_sha256",
    )
    if digest != candidate_digest:
        raise SuccessorGovernanceError(
            f"{receipt_context} does not bind the canonical freeze candidate digest"
        )
    receipt_timestamp = _timestamp(
        receipt["frozen_at"],
        context=f"{receipt_context}.frozen_at",
    )
    if receipt_timestamp != frozen_at:
        raise SuccessorGovernanceError(
            f"{receipt_context}.frozen_at disagrees with the frozen document"
        )

    reviewed_bindings = _exact_keys(
        receipt["reviewed_global_freeze_slot_bindings"],
        set(EXPECTED_GLOBAL_SLOTS),
        context=f"{receipt_context}.reviewed_global_freeze_slot_bindings",
    )
    candidate_bindings = {
        field: slot["value"] for field, slot in document["global_freeze_slots"].items()
    }
    for field in EXPECTED_GLOBAL_SLOTS:
        validated = _content_binding(
            reviewed_bindings[field],
            reader=reader,
            context=(f"{receipt_context}.reviewed_global_freeze_slot_bindings.{field}"),
        )
        if validated != candidate_bindings[field]:
            raise SuccessorGovernanceError(
                f"{receipt_context} reviewed binding {field!r} does not match "
                "the frozen candidate"
            )
    return receipt


def _validate_endpoint_array(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> list[dict[str, Any]]:
    endpoints = _array(value, context=context)
    if not endpoints:
        raise SuccessorGovernanceError(f"{context} must not be empty")
    result: list[dict[str, Any]] = []
    identifiers: list[str] = []
    for index, raw_endpoint in enumerate(endpoints):
        endpoint_context = f"{context}[{index}]"
        endpoint = _exact_keys(
            raw_endpoint,
            {
                "endpoint_id",
                "endpoint_kind",
                "role",
                "direction",
                "minimum_useful_margin",
                "unit",
                "estimand_binding",
            },
            context=endpoint_context,
        )
        endpoint_id = _string(
            endpoint["endpoint_id"], context=f"{endpoint_context}.endpoint_id"
        )
        identifiers.append(endpoint_id)
        endpoint_kind = _string(
            endpoint["endpoint_kind"],
            context=f"{endpoint_context}.endpoint_kind",
        )
        allowed_directions = H1B_ENDPOINT_DIRECTIONS.get(endpoint_kind)
        if allowed_directions is None:
            raise SuccessorGovernanceError(
                f"{endpoint_context}.endpoint_kind has unknown value {endpoint_kind!r}"
            )
        role = _string(endpoint["role"], context=f"{endpoint_context}.role")
        if role not in {
            "primary",
            "secondary_gatekeeping",
            "secondary_descriptive",
        }:
            raise SuccessorGovernanceError(
                f"{endpoint_context}.role has unknown value {role!r}"
            )
        if (
            endpoint_kind == "factual_outcome_proper_loss"
            and role != "secondary_descriptive"
        ):
            raise SuccessorGovernanceError(
                f"{endpoint_context} factual-outcome proper loss is a secondary "
                "outcome-model diagnostic only and cannot be primary or gatekeeping"
            )
        direction = _string(
            endpoint["direction"], context=f"{endpoint_context}.direction"
        )
        if direction not in allowed_directions:
            raise SuccessorGovernanceError(
                f"{endpoint_context}.direction is incompatible with H1-B endpoint "
                f"kind {endpoint_kind!r}"
            )
        margin = _number(
            endpoint["minimum_useful_margin"],
            context=f"{endpoint_context}.minimum_useful_margin",
        )
        if margin < 0.0:
            raise SuccessorGovernanceError(
                f"{endpoint_context}.minimum_useful_margin must be nonnegative"
            )
        if role in {"primary", "secondary_gatekeeping"} and margin <= 0.0:
            raise SuccessorGovernanceError(
                f"{endpoint_context}.minimum_useful_margin must be positive for "
                "a confirmatory endpoint"
            )
        _string(endpoint["unit"], context=f"{endpoint_context}.unit")
        _content_binding(
            endpoint["estimand_binding"],
            reader=reader,
            context=f"{endpoint_context}.estimand_binding",
        )
        result.append(endpoint)
    if len(identifiers) != len(set(identifiers)):
        raise SuccessorGovernanceError(f"{context} endpoint_id values must be unique")
    return result


def _validate_ec1_endpoint_array(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> list[dict[str, Any]]:
    endpoints = _array(value, context=context)
    if not endpoints:
        raise SuccessorGovernanceError(f"{context} must not be empty")
    result: list[dict[str, Any]] = []
    identifiers: list[str] = []
    for index, raw_endpoint in enumerate(endpoints):
        endpoint_context = f"{context}[{index}]"
        endpoint = _exact_keys(
            raw_endpoint,
            {
                "endpoint_id",
                "endpoint_kind",
                "role",
                "direction",
                "minimum_useful_margin",
                "unit",
                "estimand_binding",
            },
            context=endpoint_context,
        )
        endpoint_id = _string(
            endpoint["endpoint_id"], context=f"{endpoint_context}.endpoint_id"
        )
        identifiers.append(endpoint_id)
        endpoint_kind = _string(
            endpoint["endpoint_kind"],
            context=f"{endpoint_context}.endpoint_kind",
        )
        allowed_directions = {
            "fault_detection_sensitivity": {"higher_is_better"},
            "replay_fidelity": {
                "lower_is_better",
                "inside_equivalence_region",
            },
            "valid_case_false_positive_rate": {"lower_is_better"},
        }
        if endpoint_kind not in allowed_directions:
            raise SuccessorGovernanceError(
                f"{endpoint_context}.endpoint_kind has unknown value {endpoint_kind!r}"
            )
        role = _string(endpoint["role"], context=f"{endpoint_context}.role")
        if role != "primary":
            raise SuccessorGovernanceError(
                f"{endpoint_context}.role must be primary for an EC1 acceptance "
                "endpoint"
            )
        direction = _string(
            endpoint["direction"], context=f"{endpoint_context}.direction"
        )
        if direction not in allowed_directions[endpoint_kind]:
            raise SuccessorGovernanceError(
                f"{endpoint_context}.direction is incompatible with EC1 endpoint "
                f"kind {endpoint_kind!r}"
            )
        margin = _number(
            endpoint["minimum_useful_margin"],
            context=f"{endpoint_context}.minimum_useful_margin",
        )
        if margin <= 0.0:
            raise SuccessorGovernanceError(
                f"{endpoint_context}.minimum_useful_margin must be positive"
            )
        _string(endpoint["unit"], context=f"{endpoint_context}.unit")
        _content_binding(
            endpoint["estimand_binding"],
            reader=reader,
            context=f"{endpoint_context}.estimand_binding",
        )
        result.append(endpoint)
    if len(identifiers) != len(set(identifiers)):
        raise SuccessorGovernanceError(f"{context} endpoint_id values must be unique")
    return result


def _validate_ec1_fault_endpoint_map(
    value: Any,
    *,
    context: str,
) -> list[dict[str, Any]]:
    mappings = _array(value, context=context)
    if not mappings:
        raise SuccessorGovernanceError(f"{context} must not be empty")
    result: list[dict[str, str]] = []
    observed_pairs: list[tuple[str, str]] = []
    for index, raw_mapping in enumerate(mappings):
        mapping_context = f"{context}[{index}]"
        mapping = _exact_keys(
            raw_mapping,
            {
                "fault_id",
                "adapter_id",
                "endpoint_id",
                "pairwise_estimate_required",
                "minimum_absolute_sensitivity",
                "mandatory_pass",
            },
            context=mapping_context,
        )
        fault_id = _string(mapping["fault_id"], context=f"{mapping_context}.fault_id")
        adapter_id = _string(
            mapping["adapter_id"], context=f"{mapping_context}.adapter_id"
        )
        endpoint_id = _string(
            mapping["endpoint_id"], context=f"{mapping_context}.endpoint_id"
        )
        if not _boolean(
            mapping["pairwise_estimate_required"],
            context=f"{mapping_context}.pairwise_estimate_required",
        ):
            raise SuccessorGovernanceError(
                f"{mapping_context}.pairwise_estimate_required must be true; "
                "a distribution-average estimate cannot substitute for this "
                "fault-adapter pair"
            )
        sensitivity_floor = _number(
            mapping["minimum_absolute_sensitivity"],
            context=f"{mapping_context}.minimum_absolute_sensitivity",
        )
        if not 0.0 < sensitivity_floor <= 1.0:
            raise SuccessorGovernanceError(
                f"{mapping_context}.minimum_absolute_sensitivity must be in (0, 1]"
            )
        if not _boolean(
            mapping["mandatory_pass"],
            context=f"{mapping_context}.mandatory_pass",
        ):
            raise SuccessorGovernanceError(
                f"{mapping_context}.mandatory_pass must be true; no registered "
                "fault-adapter failure may be rescued by an aggregate endpoint"
            )
        observed_pairs.append((fault_id, adapter_id))
        result.append(
            {
                "fault_id": fault_id,
                "adapter_id": adapter_id,
                "endpoint_id": endpoint_id,
                "pairwise_estimate_required": True,
                "minimum_absolute_sensitivity": sensitivity_floor,
                "mandatory_pass": True,
            }
        )
    if len(observed_pairs) != len(set(observed_pairs)):
        raise SuccessorGovernanceError(
            f"{context} must map each fault-adapter pair exactly once"
        )
    return result


def _validate_ec1_adapter_endpoint_map(
    value: Any,
    *,
    context: str,
) -> list[dict[str, str]]:
    mappings = _array(value, context=context)
    if not mappings:
        raise SuccessorGovernanceError(f"{context} must not be empty")
    result: list[dict[str, str]] = []
    observed_adapters: list[str] = []
    for index, raw_mapping in enumerate(mappings):
        mapping_context = f"{context}[{index}]"
        mapping = _exact_keys(
            raw_mapping,
            {"adapter_id", "endpoint_id"},
            context=mapping_context,
        )
        adapter_id = _string(
            mapping["adapter_id"], context=f"{mapping_context}.adapter_id"
        )
        endpoint_id = _string(
            mapping["endpoint_id"], context=f"{mapping_context}.endpoint_id"
        )
        observed_adapters.append(adapter_id)
        result.append({"adapter_id": adapter_id, "endpoint_id": endpoint_id})
    if len(observed_adapters) != len(set(observed_adapters)):
        raise SuccessorGovernanceError(f"{context} must map each adapter exactly once")
    return result


def _validate_fault_class_array(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> list[dict[str, Any]]:
    faults = _array(value, context=context)
    if not faults:
        raise SuccessorGovernanceError(f"{context} must not be empty")
    identifiers: list[str] = []
    result: list[dict[str, Any]] = []
    for index, raw_fault in enumerate(faults):
        fault_context = f"{context}[{index}]"
        fault = _exact_keys(
            raw_fault,
            {
                "fault_id",
                "severity",
                "in_scope_adapter_ids",
                "injection_and_oracle_binding",
            },
            context=fault_context,
        )
        identifiers.append(
            _string(fault["fault_id"], context=f"{fault_context}.fault_id")
        )
        severity = _string(fault["severity"], context=f"{fault_context}.severity")
        if severity not in {"minor", "major", "critical"}:
            raise SuccessorGovernanceError(
                f"{fault_context}.severity has unknown value {severity!r}"
            )
        _string_array(
            fault["in_scope_adapter_ids"],
            context=f"{fault_context}.in_scope_adapter_ids",
            allow_empty=False,
        )
        _content_binding(
            fault["injection_and_oracle_binding"],
            reader=reader,
            context=f"{fault_context}.injection_and_oracle_binding",
        )
        result.append(fault)
    if len(identifiers) != len(set(identifiers)):
        raise SuccessorGovernanceError(f"{context} fault_id values must be unique")
    return result


def _validate_decision_hierarchy(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> list[dict[str, Any]]:
    stages = _array(value, context=context)
    if not stages:
        raise SuccessorGovernanceError(f"{context} must not be empty")
    result: list[dict[str, Any]] = []
    observed_stage_numbers: list[int] = []
    for index, raw_stage in enumerate(stages):
        stage_context = f"{context}[{index}]"
        stage = _exact_keys(
            raw_stage,
            {"stage", "endpoint_id", "role", "pass_condition_binding"},
            context=stage_context,
        )
        observed_stage_numbers.append(
            _integer(stage["stage"], context=f"{stage_context}.stage", minimum=1)
        )
        _string(stage["endpoint_id"], context=f"{stage_context}.endpoint_id")
        role = _string(stage["role"], context=f"{stage_context}.role")
        if role not in {
            "primary_gate",
            "secondary_gatekeeping",
            "secondary_descriptive",
        }:
            raise SuccessorGovernanceError(
                f"{stage_context}.role has unknown value {role!r}"
            )
        _content_binding(
            stage["pass_condition_binding"],
            reader=reader,
            context=f"{stage_context}.pass_condition_binding",
        )
        result.append(stage)
    expected = list(range(1, len(stages) + 1))
    if observed_stage_numbers != expected:
        raise SuccessorGovernanceError(
            f"{context}.stage values must be ordered consecutively from 1"
        )
    return result


def _validate_warning_dispositions(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> list[dict[str, Any]]:
    dispositions = _array(value, context=context)
    identifiers: list[str] = []
    result: list[dict[str, Any]] = []
    for index, raw_disposition in enumerate(dispositions):
        disposition_context = f"{context}[{index}]"
        disposition = _exact_keys(
            raw_disposition,
            {"warning_code", "disposition", "rationale_and_evidence_binding"},
            context=disposition_context,
        )
        identifiers.append(
            _string(
                disposition["warning_code"],
                context=f"{disposition_context}.warning_code",
            )
        )
        action = _string(
            disposition["disposition"],
            context=f"{disposition_context}.disposition",
        )
        if action not in {
            "use_pid_output",
            "abstain_and_exact_m1_fallback",
            "block_primary_result",
        }:
            raise SuccessorGovernanceError(
                f"{disposition_context}.disposition has unknown value {action!r}"
            )
        _content_binding(
            disposition["rationale_and_evidence_binding"],
            reader=reader,
            context=f"{disposition_context}.rationale_and_evidence_binding",
        )
        result.append(disposition)
    if len(identifiers) != len(set(identifiers)):
        raise SuccessorGovernanceError(f"{context} warning_code values must be unique")
    return result


def _validate_h3_common_comparison_contract(
    value: Any,
    *,
    context: str,
) -> dict[str, Any]:
    contract = _exact_keys(value, H3_COMMON_COMPARISON_KEYS, context=context)
    if (
        _integer(
            contract["contract_version"],
            context=f"{context}.contract_version",
        )
        != 1
    ):
        raise SuccessorGovernanceError(f"{context}.contract_version must equal 1")
    policies = _string_array(
        contract["allowed_policies"],
        context=f"{context}.allowed_policies",
        allow_empty=False,
    )
    if policies != H3_ALLOWED_COMPARISON_POLICIES:
        raise SuccessorGovernanceError(
            f"{context}.allowed_policies must preserve the two-policy inventory"
        )
    selected_policy = _string(
        contract["selected_policy"],
        context=f"{context}.selected_policy",
    )
    if selected_policy not in H3_ALLOWED_COMPARISON_POLICIES:
        raise SuccessorGovernanceError(
            f"{context}.selected_policy has unknown value {selected_policy!r}"
        )
    if selected_policy != H3_SELECTED_COMPARISON_POLICY:
        raise SuccessorGovernanceError(
            f"{context}.selected_policy must equal the frozen full-target "
            "M1-fallback policy"
        )
    for field, expected in EXPECTED_H3_COMMON_COMPARISON_FIELDS.items():
        actual = _string(contract[field], context=f"{context}.{field}")
        if actual != expected:
            raise SuccessorGovernanceError(
                f"{context}.{field} must equal the frozen contract"
            )
    statuses = _string_array(
        contract["m2_allowed_statuses"],
        context=f"{context}.m2_allowed_statuses",
        allow_empty=False,
    )
    if statuses != EXPECTED_H3_M2_STATUSES:
        raise SuccessorGovernanceError(
            f"{context}.m2_allowed_statuses must exclude unrequested and unknown states"
        )
    reporting = _string_array(
        contract["required_reporting"],
        context=f"{context}.required_reporting",
        allow_empty=False,
    )
    if reporting != EXPECTED_H3_REQUIRED_REPORTING:
        raise SuccessorGovernanceError(
            f"{context}.required_reporting inventory drifted"
        )
    fail_closed = _string_array(
        contract["fail_closed_conditions"],
        context=f"{context}.fail_closed_conditions",
        allow_empty=False,
    )
    if fail_closed != EXPECTED_H3_FAIL_CLOSED_CONDITIONS:
        raise SuccessorGovernanceError(
            f"{context}.fail_closed_conditions inventory drifted"
        )
    return contract


def _validate_h2_primary_proper_score(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> dict[str, Any]:
    endpoint = _exact_keys(
        value,
        {
            "endpoint_id",
            "role",
            "score_family",
            "censoring_handling",
            "direction",
            "minimum_useful_margin",
            "unit",
            "estimand_binding",
            "score_definition_binding",
            "properness_and_identifiability_assumptions_binding",
            "fitted_only_within_outer_training",
            "forecast_dependent_censoring_weights",
        },
        context=context,
    )
    _string(endpoint["endpoint_id"], context=f"{context}.endpoint_id")
    if endpoint["role"] != "primary":
        raise SuccessorGovernanceError(f"{context}.role must equal 'primary'")
    score_family = _string(endpoint["score_family"], context=f"{context}.score_family")
    if score_family not in {"fixed_horizon_log_loss", "time_dependent_brier"}:
        raise SuccessorGovernanceError(
            f"{context}.score_family has unknown value {score_family!r}"
        )
    censoring_handling = _string(
        endpoint["censoring_handling"],
        context=f"{context}.censoring_handling",
    )
    allowed_pairs = {
        ("fixed_horizon_log_loss", "full_eligible_population_complete_followup"),
        ("fixed_horizon_log_loss", "marginalized_observed_data_score"),
        ("time_dependent_brier", "full_eligible_population_complete_followup"),
        ("time_dependent_brier", "cross_fitted_ipcw"),
        ("time_dependent_brier", "marginalized_observed_data_score"),
    }
    if (score_family, censoring_handling) not in allowed_pairs:
        raise SuccessorGovernanceError(
            f"{context} has an unsupported score-family/censoring-handling pair"
        )
    if endpoint["direction"] != "lower_is_better":
        raise SuccessorGovernanceError(
            f"{context}.direction must equal 'lower_is_better'"
        )
    margin = _number(
        endpoint["minimum_useful_margin"],
        context=f"{context}.minimum_useful_margin",
    )
    if margin <= 0.0:
        raise SuccessorGovernanceError(
            f"{context}.minimum_useful_margin must be positive"
        )
    _string(endpoint["unit"], context=f"{context}.unit")
    for field in (
        "estimand_binding",
        "score_definition_binding",
        "properness_and_identifiability_assumptions_binding",
    ):
        _content_binding(
            endpoint[field],
            reader=reader,
            context=f"{context}.{field}",
        )
    if not _boolean(
        endpoint["fitted_only_within_outer_training"],
        context=f"{context}.fitted_only_within_outer_training",
    ):
        raise SuccessorGovernanceError(
            f"{context}.fitted_only_within_outer_training must be true"
        )
    if _boolean(
        endpoint["forecast_dependent_censoring_weights"],
        context=f"{context}.forecast_dependent_censoring_weights",
    ):
        raise SuccessorGovernanceError(
            f"{context}.forecast_dependent_censoring_weights must be false"
        )
    return endpoint


def _validate_h2_success_rule(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> dict[str, Any]:
    rule = _exact_keys(
        value,
        {
            "primary_endpoint_id",
            "primary_improvement_direction",
            "strongest_matched_access_baseline_required",
            "minimum_useful_margin_required",
            "external_or_later_time_replication_required",
            "calibration_requirement",
            "actionability_requirement",
            "subgroup_requirement",
            "decision_utility_role",
            "secondary_endpoints_cannot_rescue_primary_failure",
            "success_decision_binding",
        },
        context=context,
    )
    _string(rule["primary_endpoint_id"], context=f"{context}.primary_endpoint_id")
    if rule["primary_improvement_direction"] != (
        "baseline_score_minus_diagnostic_score"
    ):
        raise SuccessorGovernanceError(
            f"{context}.primary_improvement_direction must encode lower-score "
            "improvement"
        )
    for field in (
        "strongest_matched_access_baseline_required",
        "minimum_useful_margin_required",
        "external_or_later_time_replication_required",
        "secondary_endpoints_cannot_rescue_primary_failure",
    ):
        if not _boolean(rule[field], context=f"{context}.{field}"):
            raise SuccessorGovernanceError(f"{context}.{field} must be true")
    if rule["calibration_requirement"] != (
        "within_frozen_tolerance_or_pass_prespecified_recalibration_on_separate_split"
    ):
        raise SuccessorGovernanceError(
            f"{context}.calibration_requirement does not bind the required "
            "calibration gate"
        )
    if rule["actionability_requirement"] != (
        "frozen_alarm_policy_meets_prespecified_false_alarm_and_warning_time_criteria"
    ):
        raise SuccessorGovernanceError(
            f"{context}.actionability_requirement does not bind the required "
            "alarm-policy gate"
        )
    if rule["subgroup_requirement"] != "prespecified_degradation_bounds_hold":
        raise SuccessorGovernanceError(
            f"{context}.subgroup_requirement does not bind degradation limits"
        )
    utility_role = _string(
        rule["decision_utility_role"],
        context=f"{context}.decision_utility_role",
    )
    if utility_role not in {"secondary_gatekeeping", "secondary_descriptive"}:
        raise SuccessorGovernanceError(
            f"{context}.decision_utility_role must be secondary, never primary"
        )
    _content_binding(
        rule["success_decision_binding"],
        reader=reader,
        context=f"{context}.success_decision_binding",
    )
    return rule


def _validate_h4_primary_tuple(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> dict[str, Any]:
    primary = _exact_keys(
        value,
        {
            "task_variable_q",
            "representation_site",
            "probe_binding",
            "availability_metric_id",
            "availability_reference_binding",
            "intervention_construction_id",
            "dose",
            "outcome_id",
            "region_rule_binding",
            "target_weight_binding",
            "availability_margin",
            "effect_equivalence_margin",
            "minimum_region_mass",
            "preprocessing_binding",
            "time_window_binding",
            "support_gate_binding",
            "availability_direction",
            "effect_contrast_direction",
        },
        context=context,
    )
    for field in (
        "task_variable_q",
        "representation_site",
        "availability_metric_id",
        "intervention_construction_id",
        "dose",
        "outcome_id",
    ):
        _string(primary[field], context=f"{context}.{field}")
    for field in (
        "probe_binding",
        "availability_reference_binding",
        "region_rule_binding",
        "target_weight_binding",
        "preprocessing_binding",
        "time_window_binding",
        "support_gate_binding",
    ):
        _content_binding(
            primary[field],
            reader=reader,
            context=f"{context}.{field}",
        )
    for field in ("availability_margin", "effect_equivalence_margin"):
        margin = _number(primary[field], context=f"{context}.{field}")
        if margin <= 0.0:
            raise SuccessorGovernanceError(f"{context}.{field} must be positive")
    _probability(
        primary["minimum_region_mass"],
        context=f"{context}.minimum_region_mass",
        strictly_positive=True,
    )
    if primary["availability_direction"] != "higher_is_more_available":
        raise SuccessorGovernanceError(
            f"{context}.availability_direction must equal 'higher_is_more_available'"
        )
    if primary["effect_contrast_direction"] != "treatment_minus_control":
        raise SuccessorGovernanceError(
            f"{context}.effect_contrast_direction must equal 'treatment_minus_control'"
        )
    return primary


def _validate_h4_inference_plan(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> dict[str, Any]:
    plan = _exact_keys(
        value,
        {
            "method_id",
            "family_alpha",
            "family_components",
            "strong_familywise_control",
            "intersection_union_per_cell",
            "availability_bound_direction",
            "effect_interval_requirement",
            "target_weight_uncertainty_included",
            "divergence_mass_lower_bound_binding",
        },
        context=context,
    )
    _string(plan["method_id"], context=f"{context}.method_id")
    alpha = _probability(
        plan["family_alpha"],
        context=f"{context}.family_alpha",
        strictly_positive=True,
    )
    if alpha >= 0.5:
        raise SuccessorGovernanceError(f"{context}.family_alpha must be < 0.5")
    components = _string_array(
        plan["family_components"],
        context=f"{context}.family_components",
        allow_empty=False,
    )
    if components != ["availability_superiority", "effect_equivalence"]:
        raise SuccessorGovernanceError(
            f"{context}.family_components must contain availability superiority "
            "and effect equivalence in canonical order"
        )
    if not _boolean(
        plan["strong_familywise_control"],
        context=f"{context}.strong_familywise_control",
    ):
        raise SuccessorGovernanceError(
            f"{context}.strong_familywise_control must be true"
        )
    if not _boolean(
        plan["intersection_union_per_cell"],
        context=f"{context}.intersection_union_per_cell",
    ):
        raise SuccessorGovernanceError(
            f"{context}.intersection_union_per_cell must be true"
        )
    if plan["availability_bound_direction"] != "simultaneous_lower_bound":
        raise SuccessorGovernanceError(
            f"{context}.availability_bound_direction must equal "
            "'simultaneous_lower_bound'"
        )
    if plan["effect_interval_requirement"] != "wholly_inside_equivalence_region":
        raise SuccessorGovernanceError(
            f"{context}.effect_interval_requirement must equal "
            "'wholly_inside_equivalence_region'"
        )
    if not _boolean(
        plan["target_weight_uncertainty_included"],
        context=f"{context}.target_weight_uncertainty_included",
    ):
        raise SuccessorGovernanceError(
            f"{context}.target_weight_uncertainty_included must be true"
        )
    _content_binding(
        plan["divergence_mass_lower_bound_binding"],
        reader=reader,
        context=f"{context}.divergence_mass_lower_bound_binding",
    )
    return plan


def _validate_h4_power_plan(
    value: Any,
    *,
    reader: _ContentSnapshotReader,
    context: str,
) -> dict[str, Any]:
    plan = _exact_keys(
        value,
        {
            "joint_success_event",
            "minimum_joint_power",
            "maximum_familywise_type_i_error",
            "required_scenarios",
            "simulation_design_binding",
            "operating_characteristics_binding",
        },
        context=context,
    )
    if plan["joint_success_event"] != "probability_all_required_h4_components_pass":
        raise SuccessorGovernanceError(
            f"{context}.joint_success_event must bind the complete H4 success event"
        )
    _probability(
        plan["minimum_joint_power"],
        context=f"{context}.minimum_joint_power",
        strictly_positive=True,
    )
    type_i = _probability(
        plan["maximum_familywise_type_i_error"],
        context=f"{context}.maximum_familywise_type_i_error",
        strictly_positive=True,
    )
    if type_i >= 0.5:
        raise SuccessorGovernanceError(
            f"{context}.maximum_familywise_type_i_error must be < 0.5"
        )
    scenarios = set(
        _string_array(
            plan["required_scenarios"],
            context=f"{context}.required_scenarios",
            allow_empty=False,
        )
    )
    if scenarios != H4_POWER_SCENARIOS:
        missing = sorted(H4_POWER_SCENARIOS - scenarios)
        unknown = sorted(scenarios - H4_POWER_SCENARIOS)
        raise SuccessorGovernanceError(
            f"{context}.required_scenarios has incomplete H4 joint-power coverage: "
            f"missing={missing}, unknown={unknown}"
        )
    for field in ("simulation_design_binding", "operating_characteristics_binding"):
        _content_binding(
            plan[field],
            reader=reader,
            context=f"{context}.{field}",
        )
    return plan


def _validate_slot_value(
    value: Any,
    *,
    value_type: str,
    allowed_values: tuple[str, ...] | None,
    reader: _ContentSnapshotReader,
    context: str,
) -> Any:
    if value_type == "string":
        return _string(value, context=context)
    if value_type == "boolean":
        return _boolean(value, context=context)
    if value_type == "enum":
        text = _string(value, context=context)
        if allowed_values is None:
            raise SuccessorGovernanceError(
                f"{context} enum schema has no allowed values"
            )
        if text not in allowed_values:
            raise SuccessorGovernanceError(f"{context} has unknown value {text!r}")
        return text
    if value_type == "content_binding":
        return _content_binding(value, reader=reader, context=context)
    if value_type == "finite_nonempty_string_array":
        return _string_array(value, context=context, allow_empty=False)
    if value_type == "finite_string_array":
        return _string_array(value, context=context, allow_empty=True)
    if value_type == "h1b_effect_endpoint_array":
        return _validate_endpoint_array(value, reader=reader, context=context)
    if value_type == "ec1_endpoint_array":
        return _validate_ec1_endpoint_array(value, reader=reader, context=context)
    if value_type == "ec1_fault_endpoint_map":
        return _validate_ec1_fault_endpoint_map(value, context=context)
    if value_type == "ec1_adapter_endpoint_map":
        return _validate_ec1_adapter_endpoint_map(value, context=context)
    if value_type == "ec1_fault_class_array":
        return _validate_fault_class_array(value, reader=reader, context=context)
    if value_type == "decision_hierarchy_array":
        return _validate_decision_hierarchy(value, reader=reader, context=context)
    if value_type == "warning_disposition_array":
        return _validate_warning_dispositions(value, reader=reader, context=context)
    if value_type == "h2_primary_proper_score":
        return _validate_h2_primary_proper_score(
            value,
            reader=reader,
            context=context,
        )
    if value_type == "h2_success_rule":
        return _validate_h2_success_rule(value, reader=reader, context=context)
    if value_type == "h4_primary_tuple":
        return _validate_h4_primary_tuple(value, reader=reader, context=context)
    if value_type == "h4_simultaneous_inference_plan":
        return _validate_h4_inference_plan(value, reader=reader, context=context)
    if value_type == "h4_joint_design_power_plan":
        return _validate_h4_power_plan(value, reader=reader, context=context)
    raise SuccessorGovernanceError(
        f"{context} has unsupported value_type {value_type!r}"
    )


def _validate_slots(
    raw_slots: Any,
    *,
    expected: dict[str, tuple[str, tuple[str, ...] | None]],
    context: str,
) -> dict[str, Any]:
    slots = _exact_keys(raw_slots, set(expected), context=context)
    values: dict[str, Any] = {}
    for field, (expected_type, expected_allowed) in expected.items():
        slot_context = f"{context}.{field}"
        raw_slot = slots[field]
        expected_keys = ENUM_SLOT_KEYS if expected_type == "enum" else SLOT_KEYS
        slot = _exact_keys(raw_slot, expected_keys, context=slot_context)
        observed_type = _string(
            slot["value_type"], context=f"{slot_context}.value_type"
        )
        if observed_type != expected_type:
            raise SuccessorGovernanceError(
                f"{slot_context}.value_type must equal {expected_type!r}"
            )
        if not _boolean(
            slot["required_for_freeze"],
            context=f"{slot_context}.required_for_freeze",
        ):
            raise SuccessorGovernanceError(
                f"{slot_context}.required_for_freeze must be true"
            )
        if expected_type == "enum":
            observed_allowed = tuple(
                _string_array(
                    slot["allowed_values"],
                    context=f"{slot_context}.allowed_values",
                    allow_empty=False,
                )
            )
            if observed_allowed != expected_allowed:
                raise SuccessorGovernanceError(f"{slot_context}.allowed_values drifted")
        values[field] = slot["value"]
    return values


def _all_slot_values(document: dict[str, Any]) -> list[tuple[str, Any]]:
    result: list[tuple[str, Any]] = []
    for field, slot in document["global_freeze_slots"].items():
        result.append((f"global_freeze_slots.{field}", slot["value"]))
    for field, slot in document["claim_selection_contract"]["slots"].items():
        result.append((f"claim_selection_contract.slots.{field}", slot["value"]))
    for protocol_id, protocol in document["typed_protocol_contracts"].items():
        for field, slot in protocol["slots"].items():
            result.append(
                (f"typed_protocol_contracts.{protocol_id}.slots.{field}", slot["value"])
            )
    return result


def validate_active_scientific_claims(
    active_claims: list[str],
    *,
    selected_branch: str,
) -> None:
    if len(active_claims) != len(set(active_claims)):
        raise SuccessorGovernanceError(
            "active_scientific_claims must not contain duplicates"
        )
    unknown = sorted(set(active_claims) - {"H1", "H2", "H3", "H4"})
    if unknown:
        raise SuccessorGovernanceError(
            f"active_scientific_claims contains unknown claims: {unknown}"
        )
    if len(active_claims) > 3:
        raise SuccessorGovernanceError(
            "no more than three scientific claims may be active"
        )
    if not {"H1", "H2"}.issubset(active_claims):
        raise SuccessorGovernanceError(
            "active_scientific_claims must include H1 and H2"
        )
    conditional = {"H3", "H4"}.intersection(active_claims)
    if len(conditional) != 1:
        raise SuccessorGovernanceError(
            "active_scientific_claims must include exactly one of H3 or H4"
        )
    if conditional != {selected_branch}:
        raise SuccessorGovernanceError(
            "selected_h3_or_h4_branch disagrees with active_scientific_claims"
        )


def _validate_candidate_semantics(
    document: dict[str, Any],
    *,
    reader: _ContentSnapshotReader,
) -> None:
    claim_slots = document["claim_selection_contract"]["slots"]
    active_claims = _validate_slot_value(
        claim_slots["active_scientific_claims"]["value"],
        value_type="finite_nonempty_string_array",
        allowed_values=None,
        reader=reader,
        context="claim_selection_contract.slots.active_scientific_claims.value",
    )
    _validate_slot_value(
        claim_slots["selected_h1_protocol"]["value"],
        value_type="enum",
        allowed_values=("h1_protocol_a", "h1_protocol_b"),
        reader=reader,
        context="claim_selection_contract.slots.selected_h1_protocol.value",
    )
    selected_branch = _validate_slot_value(
        claim_slots["selected_h3_or_h4_branch"]["value"],
        value_type="enum",
        allowed_values=("H3", "H4"),
        reader=reader,
        context="claim_selection_contract.slots.selected_h3_or_h4_branch.value",
    )
    timing = _validate_slot_value(
        claim_slots["h3_h4_selection_timing"]["value"],
        value_type="enum",
        allowed_values=(
            "before_any_h3_or_h4_confirmatory_outcome",
            "after_h3_with_fresh_holdout_and_sequential_error_control",
        ),
        reader=reader,
        context="claim_selection_contract.slots.h3_h4_selection_timing.value",
    )
    validate_active_scientific_claims(
        active_claims,
        selected_branch=selected_branch,
    )
    if active_claims != ["H1", "H2", selected_branch]:
        raise SuccessorGovernanceError(
            "active_scientific_claims must use canonical H1, H2, selected-branch order"
        )
    if (
        timing == "after_h3_with_fresh_holdout_and_sequential_error_control"
        and selected_branch != "H4"
    ):
        raise SuccessorGovernanceError(
            "post-H3 branch selection is valid only for a fresh-holdout H4 branch"
        )

    protocols = document["typed_protocol_contracts"]
    ec1 = protocols["ec1"]["slots"]
    adapters = ec1["supported_adapter_ids"]["value"]
    if len(adapters) < 2:
        raise SuccessorGovernanceError(
            "EC1 supported_adapter_ids must contain at least two adapters"
        )
    faults = ec1["fault_class_registry"]["value"]
    covered_adapters: set[str] = set()
    for index, fault in enumerate(faults):
        fault_adapters = set(fault["in_scope_adapter_ids"])
        unknown_adapters = sorted(fault_adapters - set(adapters))
        if unknown_adapters:
            raise SuccessorGovernanceError(
                f"EC1 fault_class_registry[{index}] references unknown adapters: "
                f"{unknown_adapters}"
            )
        covered_adapters.update(fault_adapters)
    uncovered_adapters = sorted(set(adapters) - covered_adapters)
    if uncovered_adapters:
        raise SuccessorGovernanceError(
            "EC1 every supported adapter must occur in the finite fault registry; "
            f"uncovered={uncovered_adapters}"
        )
    ec1_endpoints = ec1["primary_endpoint_registry"]["value"]
    endpoints_by_id = {endpoint["endpoint_id"]: endpoint for endpoint in ec1_endpoints}
    endpoint_kinds = {endpoint["endpoint_kind"] for endpoint in ec1_endpoints}
    required_endpoint_kinds = {
        "fault_detection_sensitivity",
        "replay_fidelity",
        "valid_case_false_positive_rate",
    }
    missing_endpoint_kinds = sorted(required_endpoint_kinds - endpoint_kinds)
    if missing_endpoint_kinds:
        raise SuccessorGovernanceError(
            "EC1 endpoint registry omits mandatory acceptance endpoint kinds: "
            f"{missing_endpoint_kinds}"
        )

    registered_fault_adapter_pairs = {
        (fault["fault_id"], adapter_id)
        for fault in faults
        for adapter_id in fault["in_scope_adapter_ids"]
    }
    detection_map = ec1["fault_detection_endpoint_map"]["value"]
    mapped_fault_adapter_pairs = {
        (mapping["fault_id"], mapping["adapter_id"]) for mapping in detection_map
    }
    if mapped_fault_adapter_pairs != registered_fault_adapter_pairs:
        missing = sorted(registered_fault_adapter_pairs - mapped_fault_adapter_pairs)
        unknown = sorted(mapped_fault_adapter_pairs - registered_fault_adapter_pairs)
        raise SuccessorGovernanceError(
            "EC1 fault-detection endpoint map must exactly cover every registered "
            f"fault-adapter pair: missing={missing}, unknown={unknown}"
        )
    used_endpoint_ids: set[str] = set()
    for mapping in detection_map:
        endpoint_id = mapping["endpoint_id"]
        endpoint = endpoints_by_id.get(endpoint_id)
        if endpoint is None:
            raise SuccessorGovernanceError(
                "EC1 fault-detection endpoint map references an unknown endpoint: "
                f"{endpoint_id!r}"
            )
        if endpoint["endpoint_kind"] != "fault_detection_sensitivity":
            raise SuccessorGovernanceError(
                "EC1 fault-detection endpoint map must reference only "
                "fault_detection_sensitivity endpoints"
            )
        used_endpoint_ids.add(endpoint_id)

    for map_name, expected_kind in (
        ("replay_fidelity_endpoint_map", "replay_fidelity"),
        ("false_positive_endpoint_map", "valid_case_false_positive_rate"),
    ):
        endpoint_map = ec1[map_name]["value"]
        mapped_adapters = {mapping["adapter_id"] for mapping in endpoint_map}
        if mapped_adapters != set(adapters):
            missing = sorted(set(adapters) - mapped_adapters)
            unknown = sorted(mapped_adapters - set(adapters))
            raise SuccessorGovernanceError(
                f"EC1 {map_name} must exactly cover every supported adapter: "
                f"missing={missing}, unknown={unknown}"
            )
        for mapping in endpoint_map:
            endpoint_id = mapping["endpoint_id"]
            endpoint = endpoints_by_id.get(endpoint_id)
            if endpoint is None:
                raise SuccessorGovernanceError(
                    f"EC1 {map_name} references an unknown endpoint: {endpoint_id!r}"
                )
            if endpoint["endpoint_kind"] != expected_kind:
                raise SuccessorGovernanceError(
                    f"EC1 {map_name} must reference only {expected_kind} endpoints"
                )
            used_endpoint_ids.add(endpoint_id)

    false_positive_id = ec1["false_positive_endpoint_id"]["value"]
    false_positive_endpoint = endpoints_by_id.get(false_positive_id)
    if (
        false_positive_endpoint is None
        or false_positive_endpoint["endpoint_kind"] != "valid_case_false_positive_rate"
    ):
        raise SuccessorGovernanceError(
            "EC1 false_positive_endpoint_id must identify one "
            "valid_case_false_positive_rate endpoint"
        )
    false_positive_map_ids = {
        mapping["endpoint_id"]
        for mapping in ec1["false_positive_endpoint_map"]["value"]
    }
    if false_positive_map_ids != {false_positive_id}:
        raise SuccessorGovernanceError(
            "EC1 false-positive adapter map must use exactly the designated "
            "false_positive_endpoint_id"
        )
    if used_endpoint_ids != set(endpoints_by_id):
        unused = sorted(set(endpoints_by_id) - used_endpoint_ids)
        raise SuccessorGovernanceError(
            f"EC1 endpoint registry contains unmapped acceptance endpoints: {unused}"
        )

    h1a = protocols["h1_protocol_a"]["slots"]
    if h1a["heldout_outcomes_used_to_define_bins"]["value"] is not False:
        raise SuccessorGovernanceError(
            "H1 Protocol A heldout outcomes may not define calibration bins"
        )

    h1b = protocols["h1_protocol_b"]["slots"]
    h1b_endpoints = h1b["effect_endpoint_registry"]["value"]
    primary_endpoints = [
        endpoint for endpoint in h1b_endpoints if endpoint["role"] == "primary"
    ]
    if len(primary_endpoints) != 1:
        raise SuccessorGovernanceError(
            "H1 Protocol B must have exactly one primary effect endpoint"
        )
    primary_id = h1b["primary_effect_endpoint_id"]["value"]
    if primary_endpoints[0]["endpoint_id"] != primary_id:
        raise SuccessorGovernanceError(
            "H1 Protocol B primary_effect_endpoint_id disagrees with the endpoint registry"
        )
    endpoint_ids = {endpoint["endpoint_id"] for endpoint in h1b_endpoints}
    hierarchy = h1b["decision_hierarchy"]["value"]
    hierarchy_id_list = [stage["endpoint_id"] for stage in hierarchy]
    if len(hierarchy_id_list) != len(set(hierarchy_id_list)):
        raise SuccessorGovernanceError(
            "H1 Protocol B decision hierarchy must not repeat an endpoint"
        )
    hierarchy_ids = set(hierarchy_id_list)
    if not hierarchy_ids.issubset(endpoint_ids):
        raise SuccessorGovernanceError(
            "H1 Protocol B decision hierarchy references an unknown endpoint"
        )
    confirmatory_ids = {
        endpoint["endpoint_id"]
        for endpoint in h1b_endpoints
        if endpoint["role"] in {"primary", "secondary_gatekeeping"}
    }
    if not confirmatory_ids.issubset(hierarchy_ids):
        missing = sorted(confirmatory_ids - hierarchy_ids)
        raise SuccessorGovernanceError(
            f"H1 Protocol B hierarchy omits confirmatory endpoints: {missing}"
        )
    endpoint_roles = {
        endpoint["endpoint_id"]: endpoint["role"] for endpoint in h1b_endpoints
    }
    hierarchy_role_for_endpoint = {
        "primary": "primary_gate",
        "secondary_gatekeeping": "secondary_gatekeeping",
        "secondary_descriptive": "secondary_descriptive",
    }
    for stage in hierarchy:
        expected_role = hierarchy_role_for_endpoint[
            endpoint_roles[stage["endpoint_id"]]
        ]
        if stage["role"] != expected_role:
            raise SuccessorGovernanceError(
                "H1 Protocol B hierarchy role disagrees with endpoint registry "
                f"for {stage['endpoint_id']!r}"
            )
    primary_gates = [stage for stage in hierarchy if stage["role"] == "primary_gate"]
    if len(primary_gates) != 1 or primary_gates[0]["endpoint_id"] != primary_id:
        raise SuccessorGovernanceError(
            "H1 Protocol B hierarchy must contain one primary gate for the primary endpoint"
        )
    if hierarchy[0]["role"] != "primary_gate":
        raise SuccessorGovernanceError(
            "H1 Protocol B primary gate must be the first decision stage"
        )

    h2 = protocols["h2"]["slots"]
    primary_score = h2["primary_proper_score"]["value"]
    success_rule = h2["success_rule"]["value"]
    if success_rule["primary_endpoint_id"] != primary_score["endpoint_id"]:
        raise SuccessorGovernanceError(
            "H2 success rule primary_endpoint_id disagrees with the one primary "
            "proper-score endpoint"
        )

    h3 = protocols["h3"]["slots"]
    warning_dispositions = h3["warning_dispositions"]["value"]
    use_output_codes = sorted(
        disposition["warning_code"]
        for disposition in warning_dispositions
        if disposition["disposition"] == "use_pid_output"
    )
    allowlist = sorted(h3["allowlisted_use_output_warning_codes"]["value"])
    if allowlist != use_output_codes:
        raise SuccessorGovernanceError(
            "H3 use-output warning allowlist must exactly match the disposition map"
        )

    h4 = protocols["h4"]["slots"]
    inference = h4["simultaneous_inference_plan"]["value"]
    power = h4["joint_design_power_plan"]["value"]
    if power["maximum_familywise_type_i_error"] > inference["family_alpha"] + 1e-15:
        raise SuccessorGovernanceError(
            "H4 joint design maximum familywise type-I error exceeds family alpha"
        )
    if (
        h4["confirmatory_sample_source"]["value"] == "transported_randomized_sample"
        and not h4["conditional_effect_transport_assumptions_binding"]["value"]
    ):
        raise SuccessorGovernanceError(
            "H4 transported randomized samples require effect-transport assumptions"
        )


def validate_successor_document(
    document: Any,
    *,
    root: Path = ROOT,
) -> list[str]:
    """Validate one v2 draft/candidate/frozen document and return readiness blockers."""

    root = root.resolve(strict=True)
    reader = _ContentSnapshotReader(root)
    artifact = _exact_keys(document, TOP_LEVEL_KEYS, context=SUCCESSOR_PATH.as_posix())
    if (
        _integer(
            artifact["schema_version"],
            context=f"{SUCCESSOR_PATH}.schema_version",
        )
        != 2
    ):
        raise SuccessorGovernanceError("successor schema_version must equal 2")
    if artifact["artifact_id"] != "prisoma_m0_preregistration_successor_draft_v2":
        raise SuccessorGovernanceError("successor artifact_id drifted")
    as_of_date = _date(artifact["as_of_date"], context=f"{SUCCESSOR_PATH}.as_of_date")

    canonical = _exact_keys(
        artifact["canonical_spec"],
        CANONICAL_SPEC_KEYS,
        context=f"{SUCCESSOR_PATH}.canonical_spec",
    )
    if canonical != {"path": "grandplan.md", "version": "12.5"}:
        raise SuccessorGovernanceError(
            "successor canonical_spec must identify grandplan.md v12.5"
        )
    reader.read(canonical["path"], context="canonical_spec.path")
    base_binding = _content_binding(
        artifact["base_v1_intake_binding"],
        reader=reader,
        context=f"{SUCCESSOR_PATH}.base_v1_intake_binding",
    )
    if base_binding["path"] != EXPECTED_BASE_V1_PATH:
        raise SuccessorGovernanceError(
            "successor base_v1_intake_binding must bind the checked v1 intake"
        )
    scope = _string(artifact["scope"], context=f"{SUCCESSOR_PATH}.scope")
    if scope != EXPECTED_SCOPE:
        raise SuccessorGovernanceError("successor scope boundary drifted")

    status = _string(artifact["status"], context=f"{SUCCESSOR_PATH}.status")
    if status not in {
        "reviewed_successor_draft_unfrozen",
        "freeze_candidate_under_review",
        "frozen",
    }:
        raise SuccessorGovernanceError(
            f"{SUCCESSOR_PATH}.status has unknown value {status!r}"
        )

    _validate_slots(
        artifact["global_freeze_slots"],
        expected=EXPECTED_GLOBAL_SLOTS,
        context=f"{SUCCESSOR_PATH}.global_freeze_slots",
    )

    selection = _exact_keys(
        artifact["claim_selection_contract"],
        {
            "maximum_active_scientific_claims",
            "mandatory_scientific_claims",
            "conditional_exactly_one_scientific_claims",
            "engineering_claims_excluded_from_scientific_count",
            "slots",
        },
        context=f"{SUCCESSOR_PATH}.claim_selection_contract",
    )
    if selection["maximum_active_scientific_claims"] != 3:
        raise SuccessorGovernanceError(
            "claim_selection_contract maximum must equal three"
        )
    if selection["mandatory_scientific_claims"] != ["H1", "H2"]:
        raise SuccessorGovernanceError(
            "claim_selection_contract mandatory claims must be H1 and H2"
        )
    if selection["conditional_exactly_one_scientific_claims"] != ["H3", "H4"]:
        raise SuccessorGovernanceError(
            "claim_selection_contract must require exactly one of H3 or H4"
        )
    if selection["engineering_claims_excluded_from_scientific_count"] != ["EC1"]:
        raise SuccessorGovernanceError(
            "EC1 must remain outside the scientific-claim count"
        )
    _validate_slots(
        selection["slots"],
        expected=EXPECTED_CLAIM_SELECTION_SLOTS,
        context=f"{SUCCESSOR_PATH}.claim_selection_contract.slots",
    )

    protocols = _exact_keys(
        artifact["typed_protocol_contracts"],
        set(EXPECTED_PROTOCOL_SLOTS),
        context=f"{SUCCESSOR_PATH}.typed_protocol_contracts",
    )
    for protocol_id, expected_slots in EXPECTED_PROTOCOL_SLOTS.items():
        protocol_context = f"{SUCCESSOR_PATH}.typed_protocol_contracts.{protocol_id}"
        protocol = _exact_keys(
            protocols[protocol_id],
            PROTOCOL_KEYS[protocol_id],
            context=protocol_context,
        )
        _validate_slots(
            protocol["slots"],
            expected=expected_slots,
            context=f"{protocol_context}.slots",
        )

    ec1 = protocols["ec1"]
    if (
        ec1["registered_claim_id"] != "EC1"
        or ec1["claim_class"] != "engineering_acceptance"
        or ec1["unregistered_fault_policy"] != "not_evaluated_never_detected"
    ):
        raise SuccessorGovernanceError("EC1 finite-acceptance boundary drifted")
    h1a = protocols["h1_protocol_a"]
    if (
        h1a["registered_claim_id"] != "H1"
        or h1a["protocol_label"] != "paired_frozen_snapshot_algorithmic_response"
        or h1a["heldout_outcome_defined_bins_policy"] != "forbidden"
    ):
        raise SuccessorGovernanceError("H1 Protocol A calibration boundary drifted")
    h1b = protocols["h1_protocol_b"]
    if (
        h1b["registered_claim_id"] != "H1"
        or h1b["protocol_label"] != "randomized_closed_loop_effect_modification"
        or h1b["one_primary_effect_endpoint_required"] is not True
    ):
        raise SuccessorGovernanceError("H1 Protocol B endpoint boundary drifted")
    h2 = protocols["h2"]
    if (
        h2["registered_claim_id"] != "H2"
        or h2["protocol_label"] != "prospective_censoring_aware_failure_prediction"
        or h2["one_primary_proper_score_required"] is not True
        or h2["decision_utility_primary_role_forbidden"] is not True
    ):
        raise SuccessorGovernanceError("H2 primary-score boundary drifted")
    h3 = protocols["h3"]
    if (
        h3["registered_claim_id"] != "H3"
        or h3["unlisted_warning_disposition"] != "abstain_and_exact_m1_fallback"
        or h3["allowed_warning_dispositions"]
        != [
            "use_pid_output",
            "abstain_and_exact_m1_fallback",
            "block_primary_result",
        ]
    ):
        raise SuccessorGovernanceError("H3 warning-disposition boundary drifted")
    _validate_h3_common_comparison_contract(
        h3["common_comparison_population_contract"],
        context=(
            f"{SUCCESSOR_PATH}.typed_protocol_contracts.h3."
            "common_comparison_population_contract"
        ),
    )
    h4 = protocols["h4"]
    if (
        h4["registered_claim_id"] != "H4"
        or h4["protocol_label"]
        != "representational_availability_versus_tested_intervention_effect"
        or h4["individual_effect_prevalence_claim_forbidden"] is not True
    ):
        raise SuccessorGovernanceError("H4 interpretation boundary drifted")

    requirements = _string_array(
        artifact["freeze_requirements"],
        context=f"{SUCCESSOR_PATH}.freeze_requirements",
        allow_empty=False,
    )
    if requirements != EXPECTED_FREEZE_REQUIREMENTS:
        raise SuccessorGovernanceError(
            "successor freeze_requirements inventory drifted"
        )

    slot_values = _all_slot_values(artifact)
    if status == "reviewed_successor_draft_unfrozen":
        non_null = [path for path, value in slot_values if value is not None]
        if non_null:
            raise SuccessorGovernanceError(
                "the checked successor draft must keep every freeze slot null: "
                f"{non_null}"
            )
        if any(
            artifact[field] is not None
            for field in ("freeze_receipt", "freeze_revision", "frozen_at")
        ):
            raise SuccessorGovernanceError(
                "the unfrozen successor draft cannot carry freeze metadata"
            )
        return list(FREEZE_BLOCKERS)

    missing = [path for path, value in slot_values if value is None]
    if missing:
        raise SuccessorGovernanceError(
            f"{status} has null required freeze slots: {missing}"
        )

    for field, slot in artifact["global_freeze_slots"].items():
        _validate_slot_value(
            slot["value"],
            value_type=slot["value_type"],
            allowed_values=None,
            reader=reader,
            context=f"global_freeze_slots.{field}.value",
        )
    for field, slot in selection["slots"].items():
        allowed = (
            tuple(slot["allowed_values"]) if slot["value_type"] == "enum" else None
        )
        _validate_slot_value(
            slot["value"],
            value_type=slot["value_type"],
            allowed_values=allowed,
            reader=reader,
            context=f"claim_selection_contract.slots.{field}.value",
        )
    for protocol_id, protocol in protocols.items():
        for field, slot in protocol["slots"].items():
            allowed = (
                tuple(slot["allowed_values"]) if slot["value_type"] == "enum" else None
            )
            _validate_slot_value(
                slot["value"],
                value_type=slot["value_type"],
                allowed_values=allowed,
                reader=reader,
                context=(f"typed_protocol_contracts.{protocol_id}.slots.{field}.value"),
            )
    _validate_candidate_semantics(artifact, reader=reader)

    if status == "freeze_candidate_under_review":
        if any(
            artifact[field] is not None
            for field in ("freeze_receipt", "freeze_revision", "frozen_at")
        ):
            raise SuccessorGovernanceError(
                "a freeze candidate under review cannot claim completed freeze metadata"
            )
        return ["M0_SUCCESSOR_FREEZE_CANDIDATE_REVIEW_PENDING"]

    revision = _string(
        artifact["freeze_revision"],
        context=f"{SUCCESSOR_PATH}.freeze_revision",
    )
    if not SHA256_RE.fullmatch(revision):
        raise SuccessorGovernanceError(
            f"{SUCCESSOR_PATH}.freeze_revision must be a lowercase SHA-256 digest"
        )
    candidate_digest = _canonical_freeze_candidate_sha256(artifact)
    if revision != candidate_digest:
        raise SuccessorGovernanceError(
            f"{SUCCESSOR_PATH}.freeze_revision does not equal the canonical "
            "freeze-candidate SHA-256"
        )
    frozen_at = _timestamp(
        artifact["frozen_at"],
        context=f"{SUCCESSOR_PATH}.frozen_at",
    )
    _validate_freeze_receipt(
        artifact["freeze_receipt"],
        document=artifact,
        reader=reader,
        candidate_digest=candidate_digest,
        frozen_at=frozen_at,
    )
    if frozen_at.date() < as_of_date:
        raise SuccessorGovernanceError(
            f"{SUCCESSOR_PATH}.frozen_at cannot predate as_of_date"
        )
    return []


def audit_successor(
    root: Path = ROOT,
    *,
    require_freeze_ready: bool = False,
) -> list[str]:
    """Read and validate the checked successor artifact."""

    root = root.resolve(strict=True)
    document = _parse_json_bytes(
        _read_bounded_repo_file(
            root,
            SUCCESSOR_PATH.as_posix(),
            max_bytes=MAX_SUCCESSOR_DOCUMENT_BYTES,
            context=SUCCESSOR_PATH.as_posix(),
        ),
        context=SUCCESSOR_PATH.as_posix(),
    )
    blockers = validate_successor_document(document, root=root)
    if require_freeze_ready:
        return blockers
    return blockers


def _parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help=argparse.SUPPRESS)
    parser.add_argument(
        "--require-freeze-ready",
        action="store_true",
        help="report the successor draft's stable freeze blockers",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = _parse_args(argv)
    try:
        blockers = audit_successor(
            args.root,
            require_freeze_ready=args.require_freeze_ready,
        )
    except (OSError, SuccessorGovernanceError) as error:
        print(f"Research-governance successor audit failed: {error}", file=sys.stderr)
        return 1

    if args.require_freeze_ready and blockers:
        print("Research-governance successor freeze blockers:", file=sys.stderr)
        for blocker in blockers:
            print(f"- {blocker}", file=sys.stderr)
        return FREEZE_BLOCKED_EXIT

    if blockers:
        print(
            "Research-governance successor OK: typed draft is structurally valid "
            "and remains honestly unfrozen"
        )
    else:
        print("Research-governance successor OK: frozen contract is structurally valid")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
