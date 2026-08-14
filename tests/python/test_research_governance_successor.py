"""Adversarial tests for the typed, still-unfrozen M0 successor draft."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "audit_research_governance_successor.py"
SPEC = importlib.util.spec_from_file_location(
    "prisoma_research_governance_successor",
    SCRIPT,
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

SuccessorGovernanceError = MODULE.SuccessorGovernanceError
validate_successor_document = MODULE.validate_successor_document

SUCCESSOR = Path("protocols/m0_preregistration_successor_draft_v3.json")
V1 = Path("protocols/m0_preregistration_skeleton_v1.json")
GRANDPLAN = Path("grandplan.md")
FREEZE_RECEIPT = Path("protocols/test_m0_freeze_receipt_v1.json")
H3_LANDMARK_DEFINITION = GRANDPLAN
H3_SOURCE_TARGET_INVENTORY = Path("pidsplatspecs.md")
H3_PRODUCER_IMPLEMENTATION = Path("experiments/safe_adapter/contract.py")
H3_CONSUMER_VALIDATOR = Path("crates/pid-sim/src/offline_harness.rs")
H3_PER_ROW_RECEIPT_SCHEMA = Path("protocols/research_claim_registry_v1.json")
H3_ROLE_PATHS = (
    H3_LANDMARK_DEFINITION,
    H3_SOURCE_TARGET_INVENTORY,
    H3_PRODUCER_IMPLEMENTATION,
    H3_CONSUMER_VALIDATOR,
    H3_PER_ROW_RECEIPT_SCHEMA,
)

EXPECTED_BLOCKERS = [
    "M0_SUCCESSOR_REVISION_UNREVIEWED",
    "M0_SUCCESSOR_DRAFT_UNFROZEN",
    "M0_SUCCESSOR_CLAIM_SELECTION_UNFROZEN",
    "M0_SUCCESSOR_EC1_FINITE_ACCEPTANCE_PROTOCOL_UNFROZEN",
    "M0_SUCCESSOR_H1A_PRIMARY_RESPONSE_CALIBRATION_SUCCESS_CONTRACT_UNFROZEN",
    "M0_SUCCESSOR_H1B_PRIMARY_EFFECT_HIERARCHY_SUCCESS_CONTRACT_UNFROZEN",
    "M0_SUCCESSOR_H2_PRIMARY_SCORE_CENSORING_SUCCESS_PROTOCOL_UNFROZEN",
    "M0_SUCCESSOR_H3_INCREMENTAL_VALUE_AND_WARNING_CONTRACT_UNFROZEN",
    "M0_SUCCESSOR_H4_TARGET_TRANSPORT_TUPLE_INFERENCE_POWER_UNFROZEN",
    "M0_SUCCESSOR_CONTENT_BOUND_FREEZE_RECEIPTS_MISSING",
]


def _candidate_blockers(*, h3_active: bool = True) -> list[str]:
    blockers = ["M0_SUCCESSOR_FREEZE_CANDIDATE_REVIEW_PENDING"]
    if h3_active:
        blockers.insert(0, MODULE.H3_ANCESTRY_IMPLEMENTATION_BLOCKER)
    return blockers


def _load(root: Path = ROOT) -> dict:
    return json.loads((root / SUCCESSOR).read_text(encoding="utf-8"))


def _write(root: Path, document: dict) -> None:
    (root / SUCCESSOR).write_text(
        json.dumps(document, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def _copy_bundle(tmp_path: Path) -> Path:
    for relative in dict.fromkeys((SUCCESSOR, V1, GRANDPLAN, *H3_ROLE_PATHS)):
        destination = tmp_path / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / relative, destination)
    return tmp_path


def _binding(root: Path = ROOT, relative: Path = GRANDPLAN) -> dict[str, str]:
    path = root / relative
    return {
        "path": relative.as_posix(),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }


def _role_binding(
    role: str,
    relative: Path,
    *,
    root: Path = ROOT,
) -> dict[str, str]:
    return {"artifact_role": role, **_binding(root, relative)}


def _h3_source_target_ancestry_contract(root: Path = ROOT) -> dict:
    return {
        "contract_version": 1,
        "target_specific_prediction_landmark_definition_binding": _role_binding(
            "target_specific_prediction_landmark_definition",
            H3_LANDMARK_DEFINITION,
            root=root,
        ),
        "source_target_inventory_binding": _role_binding(
            "source_target_inventory", H3_SOURCE_TARGET_INVENTORY, root=root
        ),
        "producer_implementation_binding": _role_binding(
            "producer_implementation", H3_PRODUCER_IMPLEMENTATION, root=root
        ),
        "consumer_validation_implementation_binding": _role_binding(
            "consumer_validation_implementation", H3_CONSUMER_VALIDATOR, root=root
        ),
        "per_row_receipt_schema_binding": _role_binding(
            "per_row_receipt_schema", H3_PER_ROW_RECEIPT_SCHEMA, root=root
        ),
        "source_availability_rule": (
            "every_source_ancestor_must_be_observed_no_later_than_the_frozen_"
            "target_specific_prediction_landmark"
        ),
        "landmark_target_order_rule": (
            "the_frozen_target_specific_prediction_landmark_must_precede_target_"
            "realization_or_availability"
        ),
        "target_exclusion_rule": (
            "no_source_computation_graph_may_contain_the_pid_target_or_any_"
            "descendant_of_that_target"
        ),
        "future_supervision_rule": (
            "training_only_future_supervision_is_allowed_only_when_the_deployed_"
            "capture_path_reads_no_post_landmark_observation"
        ),
        "candidate_action_rule": (
            "a_candidate_action_conditioned_state_is_forbidden_as_a_source_when_"
            "the_pid_target_is_that_exact_candidate_action"
        ),
        "candidate_action_comparator_rule": (
            "when_a_source_is_conditioned_on_a_candidate_action_and_the_pid_target_is_"
            "any_downstream_command_or_later_outcome_the_matched_baseline_must_receive_"
            "that_exact_candidate_action"
        ),
        "downstream_action_interpretation_rule": (
            "incremental_value_for_a_controller_or_executed_action_target_cannot_"
            "establish_physical_forecast_validity"
        ),
        "fail_closed_on_missing_unknown_or_mismatch": True,
    }


def test_enum_slot_schema_fails_closed_without_allowed_values(
    tmp_path: Path,
) -> None:
    with pytest.raises(
        SuccessorGovernanceError, match="enum schema has no allowed values"
    ):
        MODULE._validate_slot_value(
            "value",
            value_type="enum",
            allowed_values=None,
            reader=MODULE._ContentSnapshotReader(tmp_path),
            context="fixture.enum",
        )


@pytest.mark.parametrize(
    ("value", "message"),
    [
        (" leading", "leading or trailing"),
        ("e\u0301", "Unicode NFC"),
        ("line\nbreak", "control character"),
        ("TODO", "placeholder token"),
        ("\ud800", "surrogate"),
    ],
)
def test_direct_string_validation_requires_canonical_bounded_text(
    value: str,
    message: str,
) -> None:
    with pytest.raises(SuccessorGovernanceError, match=message):
        MODULE._string(value, context="fixture.string")


def test_parser_and_arrays_enforce_explicit_resource_bounds(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setattr(MODULE, "MAX_SUCCESSOR_DOCUMENT_BYTES", 8)
    with pytest.raises(SuccessorGovernanceError, match="byte limit"):
        MODULE._parse_json_bytes(b'{"value":1}', context="fixture.json")

    monkeypatch.setattr(MODULE, "MAX_ARRAY_ITEMS", 2)
    with pytest.raises(SuccessorGovernanceError, match="item array limit"):
        MODULE._array([1, 2, 3], context="fixture.array")


def test_content_reader_rejects_symlinks_path_replacement_and_aggregate_overrun(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    target = tmp_path / "target.txt"
    target.write_bytes(b"stable")
    link = tmp_path / "link.txt"
    link.symlink_to(target.name)
    with pytest.raises(SuccessorGovernanceError, match="stable repository file"):
        MODULE._read_bounded_repo_file(
            tmp_path,
            link.name,
            max_bytes=64,
            context="fixture.link",
        )

    victim = tmp_path / "victim.txt"
    victim.write_bytes(b"original")
    replacement = tmp_path / "replacement.txt"
    replacement.write_bytes(b"replacement")
    original_read = os.read
    replaced = False

    def replace_after_read(descriptor: int, count: int) -> bytes:
        nonlocal replaced
        payload = original_read(descriptor, count)
        if payload and not replaced:
            replacement.replace(victim)
            replaced = True
        return payload

    monkeypatch.setattr(MODULE.os, "read", replace_after_read)
    with pytest.raises(SuccessorGovernanceError, match="changed while it was read"):
        MODULE._read_bounded_repo_file(
            tmp_path,
            victim.name,
            max_bytes=64,
            context="fixture.race",
        )
    monkeypatch.setattr(MODULE.os, "read", original_read)

    first = tmp_path / "first.txt"
    second = tmp_path / "second.txt"
    first.write_bytes(b"12")
    second.write_bytes(b"34")
    monkeypatch.setattr(MODULE, "MAX_BOUND_CONTENT_TOTAL_BYTES", 3)
    reader = MODULE._ContentSnapshotReader(tmp_path)
    assert reader.read(first.name, context="fixture.first") == b"12"
    with pytest.raises(SuccessorGovernanceError, match="aggregate limit"):
        reader.read(second.name, context="fixture.second")


def _endpoint(
    endpoint_id: str,
    role: str,
    *,
    endpoint_kind: str = "causal_effect_prediction_loss",
    direction: str = "lower_is_better",
    root: Path = ROOT,
) -> dict:
    return {
        "endpoint_id": endpoint_id,
        "endpoint_kind": endpoint_kind,
        "role": role,
        "direction": direction,
        "minimum_useful_margin": 0.01,
        "unit": "per_independent_cluster",
        "estimand_binding": _binding(root),
    }


def _ec1_endpoint(
    endpoint_id: str,
    endpoint_kind: str,
    *,
    direction: str,
    root: Path = ROOT,
) -> dict:
    return {
        "endpoint_id": endpoint_id,
        "endpoint_kind": endpoint_kind,
        "role": "primary",
        "direction": direction,
        "minimum_useful_margin": 0.01,
        "unit": "per_independent_cluster",
        "estimand_binding": _binding(root),
    }


def _materialized_candidate(
    root: Path = ROOT,
    *,
    selected_h1: str = "h1_protocol_a",
    selected_branch: str = "H3",
    selection_timing: str = "before_any_h3_or_h4_confirmatory_outcome",
) -> dict:
    document = _load(root)
    document["status"] = "freeze_candidate_under_review"

    for slot in document["global_freeze_slots"].values():
        slot["value"] = _binding(root)

    selection = document["claim_selection_contract"]["slots"]
    selection["active_scientific_claims"]["value"] = ["H1", "H2", selected_branch]
    selection["selected_h1_protocol"]["value"] = selected_h1
    selection["selected_h3_or_h4_branch"]["value"] = selected_branch
    selection["h3_h4_selection_timing"]["value"] = selection_timing
    selection["branch_selection_and_error_control_binding"]["value"] = _binding(root)

    protocols = document["typed_protocol_contracts"]
    for protocol in protocols.values():
        for slot in protocol["slots"].values():
            value_type = slot["value_type"]
            if value_type == "content_binding":
                slot["value"] = _binding(root)
            elif value_type == "enum":
                slot["value"] = slot["allowed_values"][0]
            elif value_type == "boolean":
                slot["value"] = False
            elif value_type == "string":
                slot["value"] = "registered_value"
            elif value_type == "finite_nonempty_string_array":
                slot["value"] = ["registered_value"]
            elif value_type == "finite_string_array":
                slot["value"] = []

    ec1 = protocols["ec1"]["slots"]
    ec1["supported_adapter_ids"]["value"] = ["safe", "external"]
    ec1["fault_class_registry"]["value"] = [
        {
            "fault_id": "dropped_intervention_receipt",
            "severity": "critical",
            "in_scope_adapter_ids": ["safe", "external"],
            "injection_and_oracle_binding": _binding(root),
        }
    ]
    ec1["primary_endpoint_registry"]["value"] = [
        _ec1_endpoint(
            "fault_detection_sensitivity",
            "fault_detection_sensitivity",
            direction="higher_is_better",
            root=root,
        ),
        _ec1_endpoint(
            "replay_fidelity",
            "replay_fidelity",
            direction="inside_equivalence_region",
            root=root,
        ),
        _ec1_endpoint(
            "valid_case_false_positive_rate",
            "valid_case_false_positive_rate",
            direction="lower_is_better",
            root=root,
        ),
    ]
    ec1["fault_detection_endpoint_map"]["value"] = [
        {
            "fault_id": "dropped_intervention_receipt",
            "adapter_id": adapter_id,
            "endpoint_id": "fault_detection_sensitivity",
            "pairwise_estimate_required": True,
            "minimum_absolute_sensitivity": 0.95,
            "mandatory_pass": True,
        }
        for adapter_id in ("safe", "external")
    ]
    ec1["replay_fidelity_endpoint_map"]["value"] = [
        {
            "adapter_id": adapter_id,
            "endpoint_id": "replay_fidelity",
        }
        for adapter_id in ("safe", "external")
    ]
    ec1["false_positive_endpoint_map"]["value"] = [
        {
            "adapter_id": adapter_id,
            "endpoint_id": "valid_case_false_positive_rate",
        }
        for adapter_id in ("safe", "external")
    ]
    ec1["false_positive_endpoint_id"]["value"] = "valid_case_false_positive_rate"

    h1a = protocols["h1_protocol_a"]["slots"]
    h1a["heldout_outcomes_used_to_define_bins"]["value"] = False
    h1a["primary_response_prediction_contract"]["value"] = {
        "endpoint_id": "heldout_algorithmic_response_prediction",
        "response_functional_binding": _binding(root),
        "primary_score_binding": _binding(root),
        "matched_access_comparator_binding": _binding(root),
        "minimum_useful_margin": 0.01,
        "improvement_direction": ("positive_values_favor_diagnostic_augmented_model"),
        "uncertainty_and_dependence_binding": _binding(root),
        "calibration_acceptance_and_failure_consequence_binding": _binding(root),
        "multiplicity_procedure_binding": _binding(root),
        "replication_or_finite_benchmark_scope_binding": _binding(root),
        "success_decision": (
            "one_sided_lower_confidence_bound_exceeds_minimum_useful_margin"
        ),
        "secondary_endpoints_cannot_rescue_primary_failure": True,
    }

    h1b = protocols["h1_protocol_b"]["slots"]
    h1b["primary_effect_endpoint_id"]["value"] = "cross_fitted_r_loss"
    h1b["effect_endpoint_registry"]["value"] = [
        _endpoint("cross_fitted_r_loss", "primary", root=root),
        _endpoint(
            "causal_calibration",
            "secondary_gatekeeping",
            endpoint_kind="causal_calibration",
            direction="inside_equivalence_region",
            root=root,
        ),
    ]
    h1b["decision_hierarchy"]["value"] = [
        {
            "stage": 1,
            "endpoint_id": "cross_fitted_r_loss",
            "role": "primary_gate",
            "pass_condition_binding": _binding(root),
        },
        {
            "stage": 2,
            "endpoint_id": "causal_calibration",
            "role": "secondary_gatekeeping",
            "pass_condition_binding": _binding(root),
        },
    ]
    h1b["primary_effect_success_contract"]["value"] = {
        "primary_endpoint_id": "cross_fitted_r_loss",
        "canonical_improvement_contrast_binding": _binding(root),
        "mandatory_effect_validation_stack_binding": _binding(root),
        "overall_itt_assignment_engagement_and_specificity_binding": _binding(root),
        "uncertainty_and_dependence_binding": _binding(root),
        "replication_target_binding": _binding(root),
        "success_decision": (
            "one_sided_lower_confidence_bound_exceeds_minimum_useful_margin"
        ),
        "secondary_endpoints_cannot_rescue_primary_failure": True,
        "factual_outcome_fit_cannot_establish_success": True,
        "directional_replication_required": True,
    }

    h2 = protocols["h2"]["slots"]
    h2["primary_scoring_contract"]["value"] = {
        "endpoint_id": "heldout_ipcw_brier",
        "role": "primary",
        "prediction_object": "fixed_horizon_event_probability",
        "score_family": "fixed_horizon_brier_loss",
        "censoring_handling": "cross_fitted_ipcw_complete_data_risk",
        "evaluation_object": "complete_data_risk_estimator_under_censoring",
        "direction": "lower_is_better",
        "minimum_useful_margin": 0.01,
        "unit": "episode_landmark_with_episode_clustered_inference",
        "estimand_binding": _binding(root),
        "score_definition_binding": _binding(root),
        "censoring_model_or_law_binding": _binding(root),
        "validity_and_identifiability_assumptions_binding": _binding(root),
        "fitted_only_within_outer_training": True,
        "forecast_dependent_censoring_weights": False,
    }
    h2["success_rule"]["value"] = {
        "primary_endpoint_id": "heldout_ipcw_brier",
        "primary_improvement_direction": "baseline_score_minus_diagnostic_score",
        "frozen_matched_access_comparator_rule_required": True,
        "minimum_useful_margin_required": True,
        "external_or_later_time_replication_required": True,
        "calibration_requirement": (
            "within_frozen_tolerance_or_pass_prespecified_recalibration_on_separate_split"
        ),
        "actionability_requirement": (
            "frozen_alarm_policy_meets_prespecified_false_alarm_and_warning_time_criteria"
        ),
        "subgroup_requirement": "prespecified_degradation_bounds_hold",
        "decision_utility_role": "secondary_gatekeeping",
        "secondary_endpoints_cannot_rescue_primary_failure": True,
        "success_decision": (
            "one_sided_lower_confidence_bound_exceeds_minimum_useful_margin"
        ),
        "success_decision_binding": _binding(root),
    }

    h3 = protocols["h3"]["slots"]
    h3["warning_dispositions"]["value"] = [
        {
            "warning_code": "finite_sample_stability_warning",
            "disposition": "abstain_and_exact_m1_fallback",
            "rationale_and_evidence_binding": _binding(root),
        }
    ]
    h3["allowlisted_use_output_warning_codes"]["value"] = []
    h3["source_target_ancestry_binding"]["value"] = _h3_source_target_ancestry_contract(
        root
    )
    h3["primary_incremental_value_contract"]["value"] = {
        "endpoint_id": "full_population_pid_incremental_value",
        "active_parent_primary_endpoint_binding": _binding(root),
        "pid_regime_and_feature_construction_binding": _binding(root),
        "canonical_improvement_contrast_binding": _binding(root),
        "improvement_direction": "positive_values_favor_deployed_m2_policy",
        "minimum_useful_margin": 0.01,
        "support_abstention_and_warning_acceptance_binding": _binding(root),
        "uncertainty_and_dependence_procedure_binding": _binding(root),
        "multiplicity_procedure_binding": _binding(root),
        "replication_target_binding": _binding(root),
        "success_decision": (
            "one_sided_lower_confidence_bound_exceeds_minimum_useful_margin"
        ),
        "eligible_only_analysis_cannot_establish_success": True,
        "secondary_endpoints_cannot_rescue_primary_failure": True,
        "independent_replication_required": True,
    }

    h4 = protocols["h4"]["slots"]
    h4["primary_tuple"]["value"] = {
        "task_variable_q": "declared_task_variable",
        "representation_site": "declared_site",
        "probe_binding": _binding(root),
        "availability_metric_id": "heldout_probe_score",
        "availability_reference_binding": _binding(root),
        "intervention_construction_id": "declared_intervention",
        "dose": "declared_dose",
        "outcome_id": "declared_policy_outcome",
        "region_rule_binding": _binding(root),
        "target_weight_binding": _binding(root),
        "availability_margin": 0.05,
        "effect_equivalence_margin": 0.05,
        "minimum_region_mass": 0.2,
        "preprocessing_binding": _binding(root),
        "time_window_binding": _binding(root),
        "support_gate_binding": _binding(root),
        "availability_direction": "higher_is_more_available",
        "effect_contrast_direction": "treatment_minus_control",
    }
    h4["simultaneous_inference_plan"]["value"] = {
        "method_id": "simultaneous_confidence_region",
        "family_alpha": 0.05,
        "family_components": [
            "availability_superiority",
            "effect_equivalence",
        ],
        "strong_familywise_control": True,
        "intersection_union_per_cell": True,
        "availability_bound_direction": "simultaneous_lower_bound",
        "effect_interval_requirement": "wholly_inside_equivalence_region",
        "target_weight_uncertainty_treatment": "included_in_simultaneous_inference",
        "divergence_mass_lower_bound_binding": _binding(root),
    }
    h4["joint_design_power_plan"]["value"] = {
        "joint_success_event": "probability_all_required_h4_components_pass",
        "minimum_joint_power": 0.8,
        "maximum_familywise_type_i_error": 0.05,
        "required_scenarios": sorted(MODULE.H4_POWER_SCENARIOS),
        "simulation_design_binding": _binding(root),
        "operating_characteristics_binding": _binding(root),
    }

    active_protocols = {"ec1", "h2", selected_h1, selected_branch.lower()}
    if selection_timing == "after_h3_with_fresh_holdout_and_sequential_error_control":
        active_protocols.add("h3")
    for protocol_id, protocol in protocols.items():
        if protocol_id not in active_protocols:
            for slot in protocol["slots"].values():
                slot["value"] = None
    return document


def _materialized_frozen(
    root: Path,
    *,
    candidate: dict | None = None,
    frozen_at: str = "2026-08-13T12:00:00Z",
) -> dict:
    reviewed_candidate = (
        _materialized_candidate(root) if candidate is None else copy.deepcopy(candidate)
    )
    candidate_digest = MODULE._canonical_freeze_candidate_sha256(reviewed_candidate)
    receipt = {
        "schema_version": 1,
        "artifact_id": "prisoma_m0_freeze_receipt_v1",
        "candidate_artifact_id": reviewed_candidate["artifact_id"],
        "candidate_schema_version": reviewed_candidate["schema_version"],
        "candidate_path": SUCCESSOR.as_posix(),
        "candidate_status": "freeze_candidate_under_review",
        "authorized_status": "frozen",
        "canonicalization": MODULE.FREEZE_CANONICALIZATION,
        "canonical_candidate_sha256": candidate_digest,
        "reviewed_global_freeze_slot_bindings": {
            field: copy.deepcopy(slot["value"])
            for field, slot in reviewed_candidate["global_freeze_slots"].items()
        },
        "frozen_at": frozen_at,
    }
    receipt_path = root / FREEZE_RECEIPT
    receipt_path.parent.mkdir(parents=True, exist_ok=True)
    receipt_path.write_text(
        json.dumps(receipt, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )

    frozen = copy.deepcopy(reviewed_candidate)
    frozen["status"] = "frozen"
    frozen["freeze_receipt"] = _binding(root, FREEZE_RECEIPT)
    frozen["freeze_revision"] = candidate_digest
    frozen["frozen_at"] = frozen_at
    return frozen


def test_checked_successor_is_valid_and_honestly_unfrozen() -> None:
    assert MODULE.FREEZE_BLOCKERS == EXPECTED_BLOCKERS
    assert MODULE.audit_successor(ROOT) == EXPECTED_BLOCKERS

    default = subprocess.run(
        [sys.executable, str(SCRIPT)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    assert default.returncode == 0, default.stderr
    assert "remains honestly unreviewed and unfrozen" in default.stdout

    strict = subprocess.run(
        [sys.executable, str(SCRIPT), "--require-freeze-ready"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    assert strict.returncode == MODULE.FREEZE_BLOCKED_EXIT
    assert strict.stdout == ""
    for blocker in EXPECTED_BLOCKERS:
        assert f"- {blocker}\n" in strict.stderr


def test_successor_exactly_binds_and_does_not_modify_v1() -> None:
    document = _load()
    binding = document["base_v1_intake_binding"]
    assert binding["path"] == V1.as_posix()
    assert binding["sha256"] == hashlib.sha256((ROOT / V1).read_bytes()).hexdigest()
    assert all(value is None for _, value in MODULE._all_slot_values(document))


def test_duplicate_keys_unknown_fields_and_nonnull_draft_values_fail(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path / "duplicate")
    path = root / SUCCESSOR
    raw = path.read_text(encoding="utf-8")
    path.write_text(
        raw.replace(
            '  "schema_version": 3,',
            '  "schema_version": 3,\n  "schema_version": 3,',
            1,
        ),
        encoding="utf-8",
    )
    with pytest.raises(SuccessorGovernanceError, match="duplicate JSON key"):
        MODULE.audit_successor(root)

    document = _load()
    document["unexpected"] = None
    with pytest.raises(SuccessorGovernanceError, match="unknown"):
        validate_successor_document(document, root=ROOT)

    document = _load()
    document["typed_protocol_contracts"]["h1_protocol_a"]["slots"][
        "calibration_bin_origin"
    ]["value"] = "prespecified_before_holdout"
    with pytest.raises(SuccessorGovernanceError, match="every freeze slot null"):
        validate_successor_document(document, root=ROOT)


def test_fully_materialized_candidate_and_typed_frozen_receipt_validate(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path)
    candidate = _materialized_candidate(root)
    assert validate_successor_document(candidate, root=root) == _candidate_blockers()

    frozen = _materialized_frozen(root, candidate=candidate)
    with pytest.raises(SuccessorGovernanceError, match="active H3 contract cannot"):
        validate_successor_document(frozen, root=root)

    for selected_h1 in ("h1_protocol_a", "h1_protocol_b"):
        for selected_branch in ("H3", "H4"):
            candidate = _materialized_candidate(
                root,
                selected_h1=selected_h1,
                selected_branch=selected_branch,
            )
            assert validate_successor_document(
                candidate, root=root
            ) == _candidate_blockers(h3_active=selected_branch == "H3")

    h4_candidate = _materialized_candidate(root, selected_branch="H4")
    h4_frozen = _materialized_frozen(root, candidate=h4_candidate)
    assert validate_successor_document(h4_frozen, root=root) == []


def test_candidate_populates_only_selected_protocol_contracts() -> None:
    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "primary_effect_endpoint_id"
    ]["value"] = "unused_but_populated"
    with pytest.raises(SuccessorGovernanceError, match="populated inactive protocol"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h4"]["slots"]["target_population_binding"][
        "value"
    ] = _binding()
    with pytest.raises(SuccessorGovernanceError, match="populated inactive protocol"):
        validate_successor_document(candidate, root=ROOT)

    switched = _materialized_candidate(
        selected_branch="H4",
        selection_timing="after_h3_with_fresh_holdout_and_sequential_error_control",
    )
    assert validate_successor_document(switched, root=ROOT) == _candidate_blockers()


def test_arbitrary_receipt_revision_or_post_review_candidate_drift_cannot_freeze(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path)
    candidate = _materialized_candidate(root, selected_branch="H4")

    arbitrary_receipt = copy.deepcopy(candidate)
    arbitrary_receipt["status"] = "frozen"
    arbitrary_receipt["freeze_receipt"] = _binding(root)
    arbitrary_receipt["freeze_revision"] = MODULE._canonical_freeze_candidate_sha256(
        arbitrary_receipt
    )
    arbitrary_receipt["frozen_at"] = "2026-08-12T12:00:00Z"
    with pytest.raises(
        SuccessorGovernanceError,
        match="freeze_receipt document",
    ):
        validate_successor_document(arbitrary_receipt, root=root)

    arbitrary_revision = _materialized_frozen(root, candidate=candidate)
    arbitrary_revision["freeze_revision"] = "a" * 64
    with pytest.raises(
        SuccessorGovernanceError,
        match="canonical freeze-candidate SHA-256",
    ):
        validate_successor_document(arbitrary_revision, root=root)

    post_review_drift = _materialized_frozen(root, candidate=candidate)
    post_review_drift["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]["minimum_useful_margin"] = 0.02
    with pytest.raises(
        SuccessorGovernanceError,
        match="canonical freeze-candidate SHA-256",
    ):
        validate_successor_document(post_review_drift, root=root)

    receipt_timestamp_drift = _materialized_frozen(root, candidate=candidate)
    receipt_timestamp_drift["frozen_at"] = "2026-08-12T12:00:01Z"
    with pytest.raises(
        SuccessorGovernanceError,
        match="frozen_at disagrees",
    ):
        validate_successor_document(receipt_timestamp_drift, root=root)

    boolean_schema = _materialized_frozen(root, candidate=candidate)
    receipt_path = root / FREEZE_RECEIPT
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    receipt["candidate_schema_version"] = True
    receipt_path.write_text(
        json.dumps(receipt, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    boolean_schema["freeze_receipt"] = _binding(root, FREEZE_RECEIPT)
    with pytest.raises(
        SuccessorGovernanceError,
        match="must be an integer, not a boolean",
    ):
        validate_successor_document(boolean_schema, root=root)


def test_claim_count_h3_h4_exclusivity_and_selection_timing_fail_closed() -> None:
    with pytest.raises(SuccessorGovernanceError, match="exactly one of H3 or H4"):
        MODULE.validate_active_scientific_claims(
            ["H1", "H2"],
            selected_branch="H3",
        )
    with pytest.raises(SuccessorGovernanceError, match="no more than three"):
        MODULE.validate_active_scientific_claims(
            ["H1", "H2", "H3", "H4"],
            selected_branch="H3",
        )

    candidate = _materialized_candidate()
    selection = candidate["claim_selection_contract"]["slots"]
    selection["selected_h3_or_h4_branch"]["value"] = "H4"
    with pytest.raises(SuccessorGovernanceError, match="disagrees"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    selection = candidate["claim_selection_contract"]["slots"]
    selection["h3_h4_selection_timing"]["value"] = (
        "after_h3_with_fresh_holdout_and_sequential_error_control"
    )
    with pytest.raises(SuccessorGovernanceError, match="fresh-holdout H4"):
        validate_successor_document(candidate, root=ROOT)


def test_ec1_requires_complete_typed_acceptance_endpoint_coverage() -> None:
    candidate = _materialized_candidate()
    ec1 = candidate["typed_protocol_contracts"]["ec1"]["slots"]
    ec1["supported_adapter_ids"]["value"] = ["safe"]
    with pytest.raises(SuccessorGovernanceError, match="at least two adapters"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    faults = candidate["typed_protocol_contracts"]["ec1"]["slots"][
        "fault_class_registry"
    ]["value"]
    faults[0]["in_scope_adapter_ids"] = ["unknown"]
    with pytest.raises(SuccessorGovernanceError, match="unknown adapters"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    faults = candidate["typed_protocol_contracts"]["ec1"]["slots"][
        "fault_class_registry"
    ]["value"]
    faults[0]["in_scope_adapter_ids"] = ["safe"]
    with pytest.raises(SuccessorGovernanceError, match="every supported adapter"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ec1 = candidate["typed_protocol_contracts"]["ec1"]["slots"]
    ec1["false_positive_endpoint_id"]["value"] = "missing"
    with pytest.raises(
        SuccessorGovernanceError,
        match="valid_case_false_positive_rate endpoint",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ec1 = candidate["typed_protocol_contracts"]["ec1"]["slots"]
    ec1["fault_detection_endpoint_map"]["value"].pop()
    with pytest.raises(
        SuccessorGovernanceError,
        match="exactly cover every registered fault-adapter pair",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ec1 = candidate["typed_protocol_contracts"]["ec1"]["slots"]
    ec1["fault_detection_endpoint_map"]["value"][0]["endpoint_id"] = "replay_fidelity"
    with pytest.raises(
        SuccessorGovernanceError,
        match="fault_detection_sensitivity endpoints",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ec1 = candidate["typed_protocol_contracts"]["ec1"]["slots"]
    ec1["replay_fidelity_endpoint_map"]["value"].pop()
    with pytest.raises(
        SuccessorGovernanceError,
        match="replay_fidelity_endpoint_map must exactly cover every supported adapter",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ec1 = candidate["typed_protocol_contracts"]["ec1"]["slots"]
    ec1["false_positive_endpoint_map"]["value"].pop()
    with pytest.raises(
        SuccessorGovernanceError,
        match="false_positive_endpoint_map must exactly cover every supported adapter",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ec1 = candidate["typed_protocol_contracts"]["ec1"]["slots"]
    ec1["false_positive_endpoint_map"]["value"][0]["endpoint_id"] = (
        "fault_detection_sensitivity"
    )
    with pytest.raises(
        SuccessorGovernanceError,
        match="valid_case_false_positive_rate endpoints",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ec1 = candidate["typed_protocol_contracts"]["ec1"]["slots"]
    ec1["primary_endpoint_registry"]["value"][1]["endpoint_kind"] = (
        "fault_detection_sensitivity"
    )
    ec1["primary_endpoint_registry"]["value"][1]["direction"] = "higher_is_better"
    with pytest.raises(
        SuccessorGovernanceError,
        match="omits mandatory acceptance endpoint kinds",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ec1 = candidate["typed_protocol_contracts"]["ec1"]["slots"]
    ec1["primary_endpoint_registry"]["value"][0]["direction"] = "lower_is_better"
    with pytest.raises(
        SuccessorGovernanceError,
        match="direction is incompatible",
    ):
        validate_successor_document(candidate, root=ROOT)


def test_ec1_pairwise_detection_floors_cannot_be_rescued_by_an_average() -> None:
    # A distribution dominated by one easy pair can pass a high aggregate floor
    # even when a critical registered pair has zero sensitivity.
    easy_pair_weight = 0.99
    easy_pair_sensitivity = 1.0
    critical_pair_sensitivity = 0.0
    aggregate_sensitivity = (
        easy_pair_weight * easy_pair_sensitivity
        + (1.0 - easy_pair_weight) * critical_pair_sensitivity
    )
    assert aggregate_sensitivity >= 0.95
    assert critical_pair_sensitivity < 0.95

    candidate = _materialized_candidate()
    mapping = candidate["typed_protocol_contracts"]["ec1"]["slots"][
        "fault_detection_endpoint_map"
    ]["value"][0]
    mapping["pairwise_estimate_required"] = False
    with pytest.raises(
        SuccessorGovernanceError,
        match="distribution-average estimate cannot substitute",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    mapping = candidate["typed_protocol_contracts"]["ec1"]["slots"][
        "fault_detection_endpoint_map"
    ]["value"][0]
    mapping["mandatory_pass"] = False
    with pytest.raises(
        SuccessorGovernanceError,
        match="no registered fault-adapter failure may be rescued",
    ):
        validate_successor_document(candidate, root=ROOT)

    for invalid_floor in (0.0, -0.01, 1.01):
        candidate = _materialized_candidate()
        mapping = candidate["typed_protocol_contracts"]["ec1"]["slots"][
            "fault_detection_endpoint_map"
        ]["value"][0]
        mapping["minimum_absolute_sensitivity"] = invalid_floor
        with pytest.raises(
            SuccessorGovernanceError,
            match=r"minimum_absolute_sensitivity must be in \(0, 1\]",
        ):
            validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    mapping = candidate["typed_protocol_contracts"]["ec1"]["slots"][
        "fault_detection_endpoint_map"
    ]["value"][0]
    del mapping["minimum_absolute_sensitivity"]
    with pytest.raises(
        SuccessorGovernanceError,
        match="missing=.*minimum_absolute_sensitivity",
    ):
        validate_successor_document(candidate, root=ROOT)


def test_h1a_bins_and_h1b_primary_hierarchy_are_enforced() -> None:
    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h1_protocol_a"]["slots"][
        "heldout_outcomes_used_to_define_bins"
    ]["value"] = True
    with pytest.raises(SuccessorGovernanceError, match="may not define"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    h1a_contract = candidate["typed_protocol_contracts"]["h1_protocol_a"]["slots"][
        "primary_response_prediction_contract"
    ]["value"]
    h1a_contract["minimum_useful_margin"] = 0.0
    with pytest.raises(SuccessorGovernanceError, match="margin must be positive"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    h1a_contract = candidate["typed_protocol_contracts"]["h1_protocol_a"]["slots"][
        "primary_response_prediction_contract"
    ]["value"]
    h1a_contract["success_decision"] = "noninferiority"
    with pytest.raises(SuccessorGovernanceError, match="one-sided superiority"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    h1a_contract = candidate["typed_protocol_contracts"]["h1_protocol_a"]["slots"][
        "primary_response_prediction_contract"
    ]["value"]
    h1a_contract["secondary_endpoints_cannot_rescue_primary_failure"] = False
    with pytest.raises(SuccessorGovernanceError, match="must be true"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    h1b_success = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "primary_effect_success_contract"
    ]["value"]
    h1b_success["primary_endpoint_id"] = "different_endpoint"
    with pytest.raises(
        SuccessorGovernanceError,
        match="disagrees with the one primary effect endpoint",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    h1b_success = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "primary_effect_success_contract"
    ]["value"]
    h1b_success["success_decision"] = "equivalence"
    with pytest.raises(SuccessorGovernanceError, match="one-sided superiority"):
        validate_successor_document(candidate, root=ROOT)

    for field in (
        "secondary_endpoints_cannot_rescue_primary_failure",
        "factual_outcome_fit_cannot_establish_success",
        "directional_replication_required",
    ):
        candidate = _materialized_candidate(selected_h1="h1_protocol_b")
        h1b_success = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
            "primary_effect_success_contract"
        ]["value"]
        h1b_success[field] = False
        with pytest.raises(SuccessorGovernanceError, match=f"{field} must be true"):
            validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    endpoints = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "effect_endpoint_registry"
    ]["value"]
    endpoints[1]["role"] = "primary"
    endpoints[1]["direction"] = "lower_is_better"
    with pytest.raises(SuccessorGovernanceError, match="exactly one primary"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    hierarchy = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "decision_hierarchy"
    ]["value"]
    hierarchy[0]["endpoint_id"] = "unknown"
    with pytest.raises(SuccessorGovernanceError, match="unknown endpoint"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    hierarchy = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "decision_hierarchy"
    ]["value"]
    hierarchy.pop()
    with pytest.raises(SuccessorGovernanceError, match="omits confirmatory"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    hierarchy = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "decision_hierarchy"
    ]["value"]
    hierarchy[1]["endpoint_id"] = hierarchy[0]["endpoint_id"]
    with pytest.raises(SuccessorGovernanceError, match="must not repeat"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    hierarchy = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "decision_hierarchy"
    ]["value"]
    hierarchy[1]["role"] = "secondary_descriptive"
    with pytest.raises(SuccessorGovernanceError, match="role disagrees"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    endpoints = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "effect_endpoint_registry"
    ]["value"]
    endpoints[0]["minimum_useful_margin"] = 0.0
    with pytest.raises(SuccessorGovernanceError, match="positive.*confirmatory"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    endpoints = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "effect_endpoint_registry"
    ]["value"]
    endpoints[0]["endpoint_kind"] = "factual_outcome_proper_loss"
    with pytest.raises(
        SuccessorGovernanceError,
        match="secondary outcome-model diagnostic only",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    endpoints = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "effect_endpoint_registry"
    ]["value"]
    endpoints[0]["endpoint_kind"] = "policy_value"
    with pytest.raises(
        SuccessorGovernanceError,
        match="direction is incompatible with H1-B endpoint kind",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_h1="h1_protocol_b")
    endpoints = candidate["typed_protocol_contracts"]["h1_protocol_b"]["slots"][
        "effect_endpoint_registry"
    ]["value"]
    endpoints[0]["endpoint_kind"] = "causal_calibration"
    endpoints[0]["direction"] = "inside_equivalence_region"
    with pytest.raises(SuccessorGovernanceError, match="one-sided improvement rule"):
        validate_successor_document(candidate, root=ROOT)


def test_h2_requires_one_scoring_contract_and_non_rescuable_success_hierarchy() -> None:
    document = _load()
    del document["typed_protocol_contracts"]["h2"]
    with pytest.raises(SuccessorGovernanceError, match="missing=.*h2"):
        validate_successor_document(document, root=ROOT)

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["role"] = "secondary_gatekeeping"
    with pytest.raises(SuccessorGovernanceError, match="role must equal 'primary'"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["direction"] = "higher_is_better"
    with pytest.raises(SuccessorGovernanceError, match="lower_is_better"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["minimum_useful_margin"] = 0.0
    with pytest.raises(SuccessorGovernanceError, match="must be positive"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["score_family"] = "right_censored_log_likelihood"
    score["censoring_handling"] = "cross_fitted_ipcw_complete_data_risk"
    score["evaluation_object"] = "complete_data_risk_estimator_under_censoring"
    with pytest.raises(
        SuccessorGovernanceError,
        match="unsupported prediction-object, score, censoring, and evaluation-object contract",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["score_family"] = "right_censored_log_likelihood"
    score["censoring_handling"] = "mapped_observed_data_law"
    score["evaluation_object"] = "proper_observed_data_score"
    with pytest.raises(
        SuccessorGovernanceError,
        match="unsupported prediction-object, score, censoring, and evaluation-object contract",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["score_family"] = "mapped_observed_data_score"
    score["censoring_handling"] = "mapped_observed_data_law"
    score["evaluation_object"] = "proper_observed_data_score"
    with pytest.raises(
        SuccessorGovernanceError,
        match="unsupported prediction-object, score, censoring, and evaluation-object contract",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["score_family"] = "fixed_horizon_log_loss"
    score["censoring_handling"] = "complete_followup_only"
    score["evaluation_object"] = "proper_complete_data_score"
    with pytest.raises(
        SuccessorGovernanceError,
        match="unsupported prediction-object, score, censoring, and evaluation-object contract",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["score_family"] = "fixed_horizon_log_loss"
    score["censoring_handling"] = "full_eligible_population_complete_followup"
    score["evaluation_object"] = "proper_complete_data_score"
    assert validate_successor_document(candidate, root=ROOT) == _candidate_blockers()

    for prediction_object, score_family, censoring_handling, evaluation_object in (
        (
            "fixed_horizon_event_probability",
            "fixed_horizon_brier_loss",
            "full_eligible_population_complete_followup",
            "proper_complete_data_score",
        ),
        (
            "fixed_horizon_event_probability",
            "fixed_horizon_log_loss",
            "cross_fitted_ipcw_complete_data_risk",
            "complete_data_risk_estimator_under_censoring",
        ),
        (
            "fixed_horizon_event_probability",
            "censoring_adjusted_fixed_horizon_brier",
            "forecast_independent_conditional_censoring_law",
            "proper_observed_data_score_on_identifiable_horizon_risk",
        ),
        (
            "full_event_time_and_type_distribution",
            "right_censored_log_likelihood",
            "mapped_observed_data_law",
            "proper_observed_data_score",
        ),
        (
            "event_time_and_type_distribution_through_horizon",
            "mapped_observed_data_score",
            "mapped_observed_data_law",
            "proper_observed_data_score",
        ),
    ):
        candidate = _materialized_candidate()
        score = candidate["typed_protocol_contracts"]["h2"]["slots"][
            "primary_scoring_contract"
        ]["value"]
        score["prediction_object"] = prediction_object
        score["score_family"] = score_family
        score["censoring_handling"] = censoring_handling
        score["evaluation_object"] = evaluation_object
        assert (
            validate_successor_document(candidate, root=ROOT) == _candidate_blockers()
        )

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["fitted_only_within_outer_training"] = False
    with pytest.raises(
        SuccessorGovernanceError,
        match="fitted_only_within_outer_training must be true",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    score = candidate["typed_protocol_contracts"]["h2"]["slots"][
        "primary_scoring_contract"
    ]["value"]
    score["forecast_dependent_censoring_weights"] = True
    with pytest.raises(
        SuccessorGovernanceError,
        match="forecast_dependent_censoring_weights must be false",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    success = candidate["typed_protocol_contracts"]["h2"]["slots"]["success_rule"][
        "value"
    ]
    success["primary_endpoint_id"] = "different_endpoint"
    with pytest.raises(
        SuccessorGovernanceError,
        match="disagrees with the one primary scoring contract",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    success = candidate["typed_protocol_contracts"]["h2"]["slots"]["success_rule"][
        "value"
    ]
    success["decision_utility_role"] = "primary"
    with pytest.raises(
        SuccessorGovernanceError,
        match="must be secondary, never primary",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    success = candidate["typed_protocol_contracts"]["h2"]["slots"]["success_rule"][
        "value"
    ]
    success["secondary_endpoints_cannot_rescue_primary_failure"] = False
    with pytest.raises(
        SuccessorGovernanceError,
        match="secondary_endpoints_cannot_rescue_primary_failure must be true",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    success = candidate["typed_protocol_contracts"]["h2"]["slots"]["success_rule"][
        "value"
    ]
    success["success_decision"] = "noninferiority"
    with pytest.raises(SuccessorGovernanceError, match="one-sided superiority"):
        validate_successor_document(candidate, root=ROOT)


def test_h3_common_population_policy_is_frozen_and_fail_closed() -> None:
    document = _load()
    contract = document["typed_protocol_contracts"]["h3"][
        "common_comparison_population_contract"
    ]
    contract["selected_policy"] = contract["allowed_policies"][0]
    with pytest.raises(
        SuccessorGovernanceError,
        match="frozen full-target M1-fallback policy",
    ):
        validate_successor_document(document, root=ROOT)

    document = _load()
    contract = document["typed_protocol_contracts"]["h3"][
        "common_comparison_population_contract"
    ]
    contract["allowed_policies"] = [contract["selected_policy"]]
    with pytest.raises(SuccessorGovernanceError, match="two-policy inventory"):
        validate_successor_document(document, root=ROOT)

    document = _load()
    contract = document["typed_protocol_contracts"]["h3"][
        "common_comparison_population_contract"
    ]
    contract["candidate_id_ledger"] = "separate_unbound_candidate_lists_for_each_model"
    with pytest.raises(
        SuccessorGovernanceError,
        match="candidate_id_ledger.*frozen contract",
    ):
        validate_successor_document(document, root=ROOT)

    document = _load()
    contract = document["typed_protocol_contracts"]["h3"][
        "common_comparison_population_contract"
    ]
    contract["primary_denominator_rule"] = (
        "pid_successful_cases_only_complete_case_analysis"
    )
    with pytest.raises(
        SuccessorGovernanceError,
        match="primary_denominator_rule.*frozen contract",
    ):
        validate_successor_document(document, root=ROOT)

    document = _load()
    contract = document["typed_protocol_contracts"]["h3"][
        "common_comparison_population_contract"
    ]
    contract["fallback_rule"] = "drop_every_m2_abstention_before_scoring"
    with pytest.raises(
        SuccessorGovernanceError,
        match="fallback_rule.*frozen contract",
    ):
        validate_successor_document(document, root=ROOT)

    document = _load()
    contract = document["typed_protocol_contracts"]["h3"][
        "common_comparison_population_contract"
    ]
    contract["m2_allowed_statuses"].insert(0, "not_requested")
    with pytest.raises(SuccessorGovernanceError, match="must exclude unrequested"):
        validate_successor_document(document, root=ROOT)

    document = _load()
    contract = document["typed_protocol_contracts"]["h3"][
        "common_comparison_population_contract"
    ]
    contract["m2_status_timing"] = (
        "derive_abstention_after_inspecting_outer_holdout_outcomes"
    )
    with pytest.raises(
        SuccessorGovernanceError,
        match="m2_status_timing.*frozen contract",
    ):
        validate_successor_document(document, root=ROOT)

    document = _load()
    contract = document["typed_protocol_contracts"]["h3"][
        "common_comparison_population_contract"
    ]
    contract["required_reporting"].remove(
        "content_bound_per_candidate_paired_scoring_receipt_sha256"
    )
    with pytest.raises(
        SuccessorGovernanceError,
        match="required_reporting inventory drifted",
    ):
        validate_successor_document(document, root=ROOT)

    document = _load()
    contract = document["typed_protocol_contracts"]["h3"][
        "common_comparison_population_contract"
    ]
    contract["fail_closed_conditions"].remove(
        "deployed_m2_fallback_differs_from_same_fold_m1_output"
    )
    with pytest.raises(
        SuccessorGovernanceError,
        match="fail_closed_conditions inventory drifted",
    ):
        validate_successor_document(document, root=ROOT)


def test_h3_warning_allowlist_and_unknown_warning_default_are_fail_closed() -> None:
    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h3"]["slots"][
        "allowlisted_use_output_warning_codes"
    ]["value"] = ["finite_sample_stability_warning"]
    with pytest.raises(SuccessorGovernanceError, match="exactly match"):
        validate_successor_document(candidate, root=ROOT)

    document = _load()
    document["typed_protocol_contracts"]["h3"]["unlisted_warning_disposition"] = (
        "use_pid_output"
    )
    with pytest.raises(SuccessorGovernanceError, match="boundary drifted"):
        validate_successor_document(document, root=ROOT)


def test_h3_incremental_value_requires_useful_superiority_and_full_population() -> None:
    candidate = _materialized_candidate()
    contract = candidate["typed_protocol_contracts"]["h3"]["slots"][
        "primary_incremental_value_contract"
    ]["value"]
    contract["minimum_useful_margin"] = 0.0
    with pytest.raises(SuccessorGovernanceError, match="margin must be positive"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    contract = candidate["typed_protocol_contracts"]["h3"]["slots"][
        "primary_incremental_value_contract"
    ]["value"]
    contract["success_decision"] = "noninferior_or_not_significantly_worse"
    with pytest.raises(SuccessorGovernanceError, match="one-sided superiority"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    contract = candidate["typed_protocol_contracts"]["h3"]["slots"][
        "primary_incremental_value_contract"
    ]["value"]
    contract["improvement_direction"] = "positive_values_favor_m1"
    with pytest.raises(SuccessorGovernanceError, match="complete deployed M2 policy"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    contract = candidate["typed_protocol_contracts"]["h3"]["slots"][
        "primary_incremental_value_contract"
    ]["value"]
    contract["eligible_only_analysis_cannot_establish_success"] = False
    with pytest.raises(
        SuccessorGovernanceError,
        match="eligible_only_analysis_cannot_establish_success must be true",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h3"]["slots"][
        "source_target_ancestry_binding"
    ]["value"] = None
    with pytest.raises(
        SuccessorGovernanceError, match="null active required freeze slots"
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h3"]["slots"][
        "source_target_ancestry_binding"
    ]["value"]["candidate_action_rule"] = (
        "a_candidate_action_conditioned_state_is_forbidden_as_a_source_when_the_pid_"
        "target_is_that_candidate_action_or_any_descendant_of_that_action"
    )
    with pytest.raises(
        SuccessorGovernanceError, match="must equal the fail-closed ancestry rule"
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h3"]["slots"][
        "source_target_ancestry_binding"
    ]["value"][
        "landmark_target_order_rule"
    ] = "the_prediction_landmark_may_follow_target_availability"
    with pytest.raises(
        SuccessorGovernanceError, match="must equal the fail-closed ancestry rule"
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h3"]["slots"][
        "source_target_ancestry_binding"
    ]["value"][
        "downstream_action_interpretation_rule"
    ] = "executed_action_prediction_proves_physical_forecast_validity"
    with pytest.raises(
        SuccessorGovernanceError, match="must equal the fail-closed ancestry rule"
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h3"]["slots"][
        "source_target_ancestry_binding"
    ]["value"][
        "candidate_action_comparator_rule"
    ] = "the_candidate_action_may_be_hidden_from_the_matched_baseline"
    with pytest.raises(
        SuccessorGovernanceError, match="must equal the fail-closed ancestry rule"
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    candidate["typed_protocol_contracts"]["h3"]["slots"][
        "source_target_ancestry_binding"
    ]["value"]["fail_closed_on_missing_unknown_or_mismatch"] = False
    with pytest.raises(
        SuccessorGovernanceError,
        match="fail_closed_on_missing_unknown_or_mismatch must be true",
    ):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ancestry = candidate["typed_protocol_contracts"]["h3"]["slots"][
        "source_target_ancestry_binding"
    ]["value"]
    ancestry["producer_implementation_binding"]["artifact_role"] = (
        "source_target_inventory"
    )
    with pytest.raises(SuccessorGovernanceError, match="artifact_role must equal"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    ancestry = candidate["typed_protocol_contracts"]["h3"]["slots"][
        "source_target_ancestry_binding"
    ]["value"]
    ancestry["source_target_inventory_binding"] = copy.deepcopy(
        ancestry["target_specific_prediction_landmark_definition_binding"]
    )
    ancestry["source_target_inventory_binding"]["artifact_role"] = (
        "source_target_inventory"
    )
    with pytest.raises(SuccessorGovernanceError, match="five distinct artifact paths"):
        validate_successor_document(candidate, root=ROOT)


def test_h4_tuple_simultaneous_inference_weight_uncertainty_and_power_are_joint() -> (
    None
):
    candidate = _materialized_candidate(selected_branch="H4")
    h4 = candidate["typed_protocol_contracts"]["h4"]["slots"]
    h4["primary_tuple"]["value"]["outcome_id"] = ["multiple", "outcomes"]
    with pytest.raises(SuccessorGovernanceError, match="non-empty string"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_branch="H4")
    h4 = candidate["typed_protocol_contracts"]["h4"]["slots"]
    h4["simultaneous_inference_plan"]["value"]["strong_familywise_control"] = False
    with pytest.raises(SuccessorGovernanceError, match="must be true"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_branch="H4")
    h4 = candidate["typed_protocol_contracts"]["h4"]["slots"]
    h4["simultaneous_inference_plan"]["value"][
        "target_weight_uncertainty_treatment"
    ] = "not_applicable_exact_finite_target_enumeration"
    with pytest.raises(SuccessorGovernanceError, match="must include target-weight"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate(selected_branch="H4")
    h4 = candidate["typed_protocol_contracts"]["h4"]["slots"]
    h4["joint_design_power_plan"]["value"]["required_scenarios"].pop()
    with pytest.raises(SuccessorGovernanceError, match="incomplete H4 joint-power"):
        validate_successor_document(candidate, root=ROOT)

    finite_target = _materialized_candidate(selected_branch="H4")
    h4 = finite_target["typed_protocol_contracts"]["h4"]["slots"]
    h4["confirmatory_sample_source"]["value"] = "finite_benchmark_equals_target"
    h4["simultaneous_inference_plan"]["value"][
        "target_weight_uncertainty_treatment"
    ] = "not_applicable_exact_finite_target_enumeration"
    assert validate_successor_document(finite_target, root=ROOT) == _candidate_blockers(
        h3_active=False
    )


def test_base_v1_binding_drift_and_false_freeze_metadata_are_rejected(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path)
    (root / V1).write_bytes((root / V1).read_bytes() + b"\n")
    with pytest.raises(SuccessorGovernanceError, match="SHA-256 mismatch"):
        MODULE.audit_successor(root)

    document = _load()
    document["base_v1_intake_binding"] = _binding(relative=GRANDPLAN)
    with pytest.raises(SuccessorGovernanceError, match="checked v1 intake"):
        validate_successor_document(document, root=ROOT)

    document = _load()
    document["freeze_requirements"][0] = "different but equally long inventory item"
    with pytest.raises(SuccessorGovernanceError, match="inventory drifted"):
        validate_successor_document(document, root=ROOT)

    candidate = _materialized_candidate()
    candidate["claim_selection_contract"]["slots"]["active_scientific_claims"][
        "value"
    ] = ["H2", "H1", "H3"]
    with pytest.raises(SuccessorGovernanceError, match="canonical H1, H2"):
        validate_successor_document(candidate, root=ROOT)

    candidate = _materialized_candidate()
    candidate["freeze_receipt"] = _binding()
    with pytest.raises(SuccessorGovernanceError, match="cannot claim"):
        validate_successor_document(candidate, root=ROOT)

    root = _copy_bundle(tmp_path / "predate")
    candidate = _materialized_candidate(root, selected_branch="H4")
    frozen = _materialized_frozen(
        root,
        candidate=candidate,
        frozen_at="2026-07-15T23:59:59Z",
    )
    with pytest.raises(SuccessorGovernanceError, match="cannot predate"):
        validate_successor_document(frozen, root=root)
