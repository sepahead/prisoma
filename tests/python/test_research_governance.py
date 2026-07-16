"""Adversarial tests for the honest, unfinished M0 governance bundle."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "audit_research_governance.py"
SPEC = importlib.util.spec_from_file_location("prisoma_research_governance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)

GovernanceError = MODULE.GovernanceError
audit_bundle = MODULE.audit_bundle

PREREGISTRATION = Path("protocols/m0_preregistration_skeleton_v1.json")
SUCCESSOR = Path("protocols/m0_preregistration_successor_draft_v2.json")
HOLDOUT_REGISTRY = Path("protocols/holdout_registry_v1.json")
HOLDOUT_LEDGER = Path("protocols/holdout_access_ledger_v1.jsonl")
TRANSPORT = Path("protocols/transport_contamination_ledger_v1.json")
LITERATURE = Path("protocols/literature_screening_ledger_v1.json")
CLAIM_REGISTRY = Path("protocols/research_claim_registry_v1.json")
INVENTORY = Path(
    "docs/reviews/2026-07-12-grandplan-v12.5/grandplan_v12_5_reference_audit.csv"
)

COPIED_PATHS = [
    PREREGISTRATION,
    SUCCESSOR,
    HOLDOUT_REGISTRY,
    HOLDOUT_LEDGER,
    TRANSPORT,
    LITERATURE,
    CLAIM_REGISTRY,
    Path("grandplan.md"),
    Path("crates/pid-sim/fixtures/h1_preflight_valid.json"),
    Path("crates/pid-sim/fixtures/h1_protocol_a_valid.json"),
    Path("crates/pid-sim/fixtures/h2_reference/analysis_plan.json"),
    INVENTORY,
]

EXPECTED_FREEZE_BLOCKERS = [
    "M0_PREREGISTRATION_UNFROZEN",
    "M0_PRIMARY_H1_PROTOCOL_UNSELECTED",
    "M0_H1_PROTOCOL_AND_ESTIMAND_UNFROZEN",
    "M0_H2_ESTIMAND_ONTOLOGY_AND_COMPARATOR_UNFROZEN",
    "M0_H3_FOUR_PID_GATES_BLOCKED",
    "M0_H4_ESTIMAND_AND_INTERVENTION_PROTOCOL_UNFROZEN",
    "M0_MINIMUM_USEFUL_EFFECTS_UNFROZEN_PENDING_DOMAIN_AND_DECISION_JUSTIFICATION",
    "M0_ECOSYSTEM_FIREBREAK_OPTIONAL_MAP_BINDINGS_UNFROZEN",
    "M0_HOLDOUT_NOT_REGISTERED",
    "M0_TRANSPORT_CONTAMINATION_RIGHTS_UNASSESSED",
    "M0_H2_COMPARATOR_REGISTRY_UNRESOLVED",
    "M0_FRESH_REPRODUCIBLE_LITERATURE_SEARCH_REQUIRED",
    "M0_REVIEW_RECEIPTS_AND_ENVIRONMENT_DIGESTS_MISSING",
]


def _copy_bundle(tmp_path: Path) -> Path:
    for relative in COPIED_PATHS:
        destination = tmp_path / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / relative, destination)
    return tmp_path


def _load_json(root: Path, relative: Path) -> dict:
    return json.loads((root / relative).read_text(encoding="utf-8"))


def _write_json(root: Path, relative: Path, value: dict) -> None:
    (root / relative).write_text(
        json.dumps(value, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def _load_ledger(root: Path) -> list[dict]:
    return [
        json.loads(line)
        for line in (root / HOLDOUT_LEDGER).read_text(encoding="utf-8").splitlines()
    ]


def _rechain(events: list[dict]) -> list[dict]:
    result = copy.deepcopy(events)
    previous: str | None = None
    for index, event in enumerate(result):
        event["event_index"] = index
        event["previous_event_sha256"] = previous
        event["event_sha256"] = MODULE._holdout_event_digest(event)
        previous = event["event_sha256"]
    return result


def _write_ledger_and_bind(root: Path, events: list[dict]) -> None:
    ledger_path = root / HOLDOUT_LEDGER
    ledger_path.write_text(
        "".join(
            json.dumps(event, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
            + "\n"
            for event in events
        ),
        encoding="utf-8",
    )
    ledger_bytes = ledger_path.read_bytes()
    ledger_sha = hashlib.sha256(ledger_bytes).hexdigest()

    registry = _load_json(root, HOLDOUT_REGISTRY)
    metadata = registry["access_ledger"]
    metadata["file_sha256"] = ledger_sha
    metadata["file_byte_count"] = len(ledger_bytes)
    metadata["event_count"] = len(events)
    metadata["head_event_sha256"] = events[-1]["event_sha256"]
    _write_json(root, HOLDOUT_REGISTRY, registry)

    transport = _load_json(root, TRANSPORT)
    binding = transport["holdout_governance_binding"]
    binding["ledger_file_sha256"] = ledger_sha
    binding["ledger_head_event_sha256"] = events[-1]["event_sha256"]
    _write_json(root, TRANSPORT, transport)


def test_real_bundle_passes_but_freeze_ready_has_stable_closed_gate() -> None:
    assert MODULE.FREEZE_BLOCKERS == EXPECTED_FREEZE_BLOCKERS
    assert audit_bundle(ROOT) == EXPECTED_FREEZE_BLOCKERS
    assert audit_bundle(ROOT, require_freeze_ready=True) == EXPECTED_FREEZE_BLOCKERS

    default = subprocess.run(
        [sys.executable, str(SCRIPT)],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    assert default.returncode == 0, default.stderr
    assert "honest unfinished scaffold" in default.stdout

    strict = subprocess.run(
        [sys.executable, str(SCRIPT), "--require-freeze-ready"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    assert strict.returncode == MODULE.FREEZE_BLOCKED_EXIT
    assert strict.stdout == ""
    assert "Research-governance freeze blockers:" in strict.stderr
    for blocker in EXPECTED_FREEZE_BLOCKERS:
        assert f"- {blocker}\n" in strict.stderr


def test_v1_cannot_be_promoted_by_filling_arbitrary_scientific_strings(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path / "global")
    prereg = _load_json(root, PREREGISTRATION)
    prereg["global_freeze_fields"]["causal_graph"] = "arbitrary non-null string"
    _write_json(root, PREREGISTRATION, prereg)
    with pytest.raises(GovernanceError, match="must remain null while unfrozen"):
        audit_bundle(root, require_freeze_ready=True)

    root = _copy_bundle(tmp_path / "branch")
    prereg = _load_json(root, PREREGISTRATION)
    prereg["protocols"]["h1_protocol_a"]["freeze_fields"][
        "primary_protocol_selection"
    ] = "arbitrary non-null string"
    _write_json(root, PREREGISTRATION, prereg)
    with pytest.raises(GovernanceError, match="must remain null while unfrozen"):
        audit_bundle(root, require_freeze_ready=True)


def test_duplicate_keys_are_rejected(tmp_path: Path) -> None:
    root = _copy_bundle(tmp_path)
    path = root / PREREGISTRATION
    raw = path.read_text(encoding="utf-8")
    path.write_text(
        raw.replace('  "scope":', '  "scope": "duplicate",\n  "scope":', 1),
        encoding="utf-8",
    )
    with pytest.raises(GovernanceError, match="duplicate JSON key 'scope'"):
        audit_bundle(root)


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        (lambda value: value.update(unexpected_field=None), "unknown"),
        (lambda value: value.update(schema_version=True), "not a boolean"),
        (lambda value: value.update(scope="TODO"), "placeholder"),
    ],
)
def test_unknown_fields_boolean_numbers_and_placeholders_are_rejected(
    tmp_path: Path, mutation, message: str
) -> None:
    root = _copy_bundle(tmp_path)
    value = _load_json(root, PREREGISTRATION)
    mutation(value)
    _write_json(root, PREREGISTRATION, value)
    with pytest.raises(GovernanceError, match=message):
        audit_bundle(root)


def test_nonfinite_json_numbers_are_rejected(tmp_path: Path) -> None:
    root = _copy_bundle(tmp_path)
    path = root / TRANSPORT
    raw = path.read_text(encoding="utf-8")
    path.write_text(raw.replace('"schema_version": 1', '"schema_version": NaN', 1))
    with pytest.raises(GovernanceError, match="non-finite JSON number"):
        audit_bundle(root)


def test_traversal_symlink_and_content_hash_drift_are_rejected(tmp_path: Path) -> None:
    root = _copy_bundle(tmp_path / "traversal")
    literature = _load_json(root, LITERATURE)
    literature["inventory_artifact"]["path"] = "../outside.csv"
    _write_json(root, LITERATURE, literature)
    with pytest.raises(GovernanceError, match="escapes the repository"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "symlink")
    inventory_path = root / INVENTORY
    target = root / "inventory-target.csv"
    target.write_bytes(inventory_path.read_bytes())
    inventory_path.unlink()
    inventory_path.symlink_to(target)
    with pytest.raises(GovernanceError, match="symlink"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "historical-identity")
    preregistration = root / PREREGISTRATION
    preregistration.write_bytes(preregistration.read_bytes() + b"\n")
    with pytest.raises(
        GovernanceError,
        match="historical identity SHA-256 mismatch",
    ):
        audit_bundle(root)


def test_branch_blending_false_freeze_and_missing_estimand_row_are_rejected(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path / "blend")
    prereg = _load_json(root, PREREGISTRATION)
    prereg["protocols"]["h1_protocol_a"]["activation_status"] = "active_primary"
    prereg["protocols"]["h1_protocol_b"]["activation_status"] = "active_primary"
    _write_json(root, PREREGISTRATION, prereg)
    with pytest.raises(GovernanceError, match="blends H1-A and H1-B"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "freeze")
    prereg = _load_json(root, PREREGISTRATION)
    prereg["freeze_status"] = "frozen"
    prereg["freeze_receipt"] = {
        "path": "grandplan.md",
        "sha256": hashlib.sha256((root / "grandplan.md").read_bytes()).hexdigest(),
    }
    prereg["freeze_revision"] = "a" * 64
    prereg["frozen_at"] = "2026-07-13T00:00:00Z"
    prereg["m0_completion_status"] = "complete"
    _write_json(root, PREREGISTRATION, prereg)
    with pytest.raises(GovernanceError, match="falsely claims a freeze"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "row")
    prereg = _load_json(root, PREREGISTRATION)
    prereg["protocols"]["h2"]["estimand_rows"] = []
    _write_json(root, PREREGISTRATION, prereg)
    with pytest.raises(GovernanceError, match="must contain one primary row"):
        audit_bundle(root)


def test_h3_activation_and_gate_regime_mismatch_are_rejected(tmp_path: Path) -> None:
    root = _copy_bundle(tmp_path / "activation")
    prereg = _load_json(root, PREREGISTRATION)
    prereg["protocols"]["h3"]["activation_status"] = "active"
    _write_json(root, PREREGISTRATION, prereg)
    with pytest.raises(GovernanceError, match="falsely activated"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "regime")
    prereg = _load_json(root, PREREGISTRATION)
    evidence_sha = hashlib.sha256((root / "grandplan.md").read_bytes()).hexdigest()
    gates = prereg["protocols"]["h3"]["pid_gates"]
    for index, gate_name in enumerate(MODULE.PID_GATE_ORDER):
        gates[gate_name] = {
            "status": "passed",
            "regime_hash": ("a" if index < 2 else "b") * 64,
            "evidence_bindings": [{"path": "grandplan.md", "sha256": evidence_sha}],
        }
    _write_json(root, PREREGISTRATION, prereg)
    with pytest.raises(GovernanceError, match="does not share one exact regime hash"):
        audit_bundle(root)


def test_holdout_event_edit_scope_smuggling_and_false_access_are_rejected(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path / "edit")
    events = _load_ledger(root)
    events[0]["purpose"] = "edited without updating the event digest"
    _write_ledger_and_bind(root, events)
    with pytest.raises(GovernanceError, match="event_sha256 does not bind"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "scope")
    events = _load_ledger(root)
    events[0]["event_scope"] = "raw-label=secret-sample-id"
    events = _rechain(events)
    _write_ledger_and_bind(root, events)
    with pytest.raises(GovernanceError, match="safe genesis literal"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "access")
    events = _load_ledger(root)
    events[0]["exposure_occurred"] = True
    events = _rechain(events)
    _write_ledger_and_bind(root, events)
    with pytest.raises(GovernanceError, match="cannot record holdout exposure"):
        audit_bundle(root)


def test_holdout_sequence_reorder_truncation_head_count_and_date_drift_are_rejected(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path / "reorder")
    first = _load_ledger(root)[0]
    second = copy.deepcopy(first)
    two_events = _rechain([first, second])
    _write_ledger_and_bind(root, [two_events[1], two_events[0]])
    with pytest.raises(GovernanceError, match="event_index breaks the exact sequence"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "truncation")
    first = _load_ledger(root)[0]
    two_events = _rechain([first, copy.deepcopy(first)])
    _write_ledger_and_bind(root, two_events)
    (root / HOLDOUT_LEDGER).write_text(
        json.dumps(
            two_events[0], ensure_ascii=False, sort_keys=True, separators=(",", ":")
        )
        + "\n",
        encoding="utf-8",
    )
    with pytest.raises(GovernanceError, match="file_sha256 does not bind ledger bytes"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "head-count")
    registry = _load_json(root, HOLDOUT_REGISTRY)
    registry["access_ledger"]["event_count"] = 2
    _write_json(root, HOLDOUT_REGISTRY, registry)
    with pytest.raises(GovernanceError, match="event_count does not match"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "date")
    events = _load_ledger(root)
    events[0]["recorded_on"] = "2026-07-12"
    events = _rechain(events)
    _write_ledger_and_bind(root, events)
    with pytest.raises(GovernanceError, match="must match the registry as_of_date"):
        audit_bundle(root)


@pytest.mark.parametrize(
    ("relative", "field", "promoted", "message"),
    [
        (
            TRANSPORT,
            "status",
            "completed_for_named_source_and_target",
            "unknown value",
        ),
        (
            LITERATURE,
            "status",
            "completed_reproducible_search",
            "unknown value",
        ),
        (
            HOLDOUT_REGISTRY,
            "registry_status",
            "confirmatory_holdout_registered",
            "unknown value",
        ),
    ],
)
def test_false_completed_state_promotions_are_rejected(
    tmp_path: Path,
    relative: Path,
    field: str,
    promoted: str,
    message: str,
) -> None:
    root = _copy_bundle(tmp_path)
    artifact = _load_json(root, relative)
    artifact[field] = promoted
    _write_json(root, relative, artifact)
    with pytest.raises(GovernanceError, match=message):
        audit_bundle(root)


def test_transport_required_coverage_inventories_and_empty_records_are_exact(
    tmp_path: Path,
) -> None:
    for index, field in enumerate(
        (
            "required_shift_variable_ids",
            "required_contamination_subtypes",
            "required_rights_artifact_classes",
        )
    ):
        root = _copy_bundle(tmp_path / f"coverage-{index}")
        transport = _load_json(root, TRANSPORT)
        transport[field] = transport[field][:-1]
        _write_json(root, TRANSPORT, transport)
        with pytest.raises(GovernanceError, match=f"{field} is incomplete"):
            audit_bundle(root)

    root = _copy_bundle(tmp_path / "records")
    transport = _load_json(root, TRANSPORT)
    transport["contamination_assessments"]["status"] = "complete"
    transport["contamination_assessments"]["records"] = [{}]
    _write_json(root, TRANSPORT, transport)
    with pytest.raises(GovernanceError, match="status must equal 'not_assessed'"):
        audit_bundle(root)


def test_literature_comparator_inventory_and_tide_source_state_are_exact(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path / "missing")
    literature = _load_json(root, LITERATURE)
    literature["unresolved_h2_comparator_registry"]["families"].pop()
    _write_json(root, LITERATURE, literature)
    with pytest.raises(GovernanceError, match="incomplete or reordered"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "tide")
    literature = _load_json(root, LITERATURE)
    families = literature["unresolved_h2_comparator_registry"]["families"]
    tide = next(
        item for item in families if item["family_id"] == "tide_inter_chunk_discrepancy"
    )
    tide["screening_status"] = "not_screened_in_reproducible_search"
    _write_json(root, LITERATURE, literature)
    with pytest.raises(GovernanceError, match="loses its exact unresolved state"):
        audit_bundle(root)


def test_cross_file_dates_and_holdout_bindings_must_agree(tmp_path: Path) -> None:
    root = _copy_bundle(tmp_path / "date")
    literature = _load_json(root, LITERATURE)
    literature["as_of_date"] = "2026-07-12"
    _write_json(root, LITERATURE, literature)
    with pytest.raises(
        GovernanceError,
        match="historical governance as_of_date values disagree",
    ):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "successor-date")
    registry = _load_json(root, CLAIM_REGISTRY)
    registry["as_of_date"] = "2026-07-15"
    _write_json(root, CLAIM_REGISTRY, registry)
    with pytest.raises(GovernanceError, match="date/status does not match"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "successor-hash")
    registry = _load_json(root, CLAIM_REGISTRY)
    registry["m0_successor_binding"]["sha256"] = "0" * 64
    _write_json(root, CLAIM_REGISTRY, registry)
    with pytest.raises(GovernanceError, match="SHA-256 mismatch"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "binding")
    transport = _load_json(root, TRANSPORT)
    transport["holdout_governance_binding"]["ledger_head_event_sha256"] = "0" * 64
    _write_json(root, TRANSPORT, transport)
    with pytest.raises(GovernanceError, match="ledger_head_event_sha256 drifted"):
        audit_bundle(root)


def test_exact_semantic_snapshots_reject_overclaiming_prose(tmp_path: Path) -> None:
    root = _copy_bundle(tmp_path / "prereg")
    prereg = _load_json(root, PREREGISTRATION)
    prereg["protocols"]["h1_protocol_a"]["interpretation_boundary"] = (
        "proves a physical individual treatment effect"
    )
    _write_json(root, PREREGISTRATION, prereg)
    with pytest.raises(GovernanceError, match="exact reviewed semantic snapshot"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "transport")
    transport = _load_json(root, TRANSPORT)
    transport["contamination_assessments"]["completion_language_boundary"] = (
        "completed means no contamination"
    )
    _write_json(root, TRANSPORT, transport)
    with pytest.raises(GovernanceError, match="exact reviewed semantic snapshot"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "literature")
    literature = _load_json(root, LITERATURE)
    literature["scope"] = "complete systematic review with exhaustive search"
    _write_json(root, LITERATURE, literature)
    with pytest.raises(GovernanceError, match="exact reviewed semantic snapshot"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "claims")
    registry = _load_json(root, CLAIM_REGISTRY)
    h1 = next(claim for claim in registry["claims"] if claim["claim_id"] == "H1")
    h1["permitted_language"] = "H1 passed with a causal physical effect"
    _write_json(root, CLAIM_REGISTRY, registry)
    with pytest.raises(GovernanceError, match="exact reviewed semantic snapshot"):
        audit_bundle(root)


def test_literature_false_review_and_comparator_disposition_are_rejected(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path / "review")
    literature = _load_json(root, LITERATURE)
    literature["systematic_review_claimed"] = True
    _write_json(root, LITERATURE, literature)
    with pytest.raises(GovernanceError, match="cannot claim a systematic review"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "disposition")
    literature = _load_json(root, LITERATURE)
    family = literature["unresolved_h2_comparator_registry"]["families"][0]
    family["disposition"] = "included"
    _write_json(root, LITERATURE, literature)
    with pytest.raises(GovernanceError, match="must remain null while unresolved"):
        audit_bundle(root)


def test_claim_registry_m0_status_mutation_is_rejected(tmp_path: Path) -> None:
    root = _copy_bundle(tmp_path)
    registry = _load_json(root, CLAIM_REGISTRY)
    registry["m0_freeze_status"]["overall"] = "freeze_ready"
    _write_json(root, CLAIM_REGISTRY, registry)
    with pytest.raises(GovernanceError, match="must equal 'not_freeze_ready'"):
        audit_bundle(root)


def test_h4_reference_and_confirmatory_contract_cannot_overclaim(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path / "reference")
    registry = _load_json(root, CLAIM_REGISTRY)
    h4 = next(claim for claim in registry["claims"] if claim["claim_id"] == "H4")
    h4["reference_artifact_semantics"][
        "establishes_causal_or_mechanistic_faithfulness"
    ] = True
    with pytest.raises(GovernanceError, match="must equal False"):
        MODULE._validate_claim_registry(root, registry)

    root = _copy_bundle(tmp_path / "decision")
    registry = _load_json(root, CLAIM_REGISTRY)
    h4 = next(claim for claim in registry["claims"] if claim["claim_id"] == "H4")
    h4["confirmatory_design_contract"]["decision_rule"] = (
        "availability_significant_and_effect_not_significant"
    )
    with pytest.raises(GovernanceError, match="decision_rule must equal"):
        MODULE._validate_claim_registry(root, registry)


def test_all_h3_gates_passing_cannot_override_not_eligible_claim_state(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path)
    prereg = _load_json(root, PREREGISTRATION)
    evidence_sha = hashlib.sha256((root / "grandplan.md").read_bytes()).hexdigest()
    for gate_name in MODULE.PID_GATE_ORDER:
        prereg["protocols"]["h3"]["pid_gates"][gate_name] = {
            "status": "passed",
            "regime_hash": "a" * 64,
            "evidence_bindings": [{"path": "grandplan.md", "sha256": evidence_sha}],
        }
    _write_json(root, PREREGISTRATION, prereg)
    with pytest.raises(GovernanceError, match="claim registry remains not_eligible"):
        audit_bundle(root)


def test_surrogates_and_ascii_control_characters_have_controlled_errors(
    tmp_path: Path,
) -> None:
    root = _copy_bundle(tmp_path / "surrogate")
    path = root / PREREGISTRATION
    raw = path.read_text(encoding="utf-8")
    path.write_text(
        raw.replace(
            '"scope": "unfrozen machine-readable M0 scaffold only; not a preregistration, '
            "registration receipt, scientific result, or authorization to access a "
            'confirmatory holdout"',
            '"scope": "\\ud800"',
            1,
        ),
        encoding="utf-8",
    )
    with pytest.raises(GovernanceError, match="Unicode surrogate code point"):
        audit_bundle(root)

    root = _copy_bundle(tmp_path / "nul")
    literature = _load_json(root, LITERATURE)
    literature["inventory_artifact"]["path"] = "docs/\0.csv"
    _write_json(root, LITERATURE, literature)
    with pytest.raises(GovernanceError, match="forbidden ASCII control character"):
        audit_bundle(root)
