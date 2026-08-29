#!/usr/bin/env python3
"""Check the closed Prisoma managed-observer authoring contract."""

from __future__ import annotations

import copy
import hashlib
import json
import math
import struct
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[4]
INTEGRATION = ROOT / "integrations" / "engram" / "managed-observer"
CONTRACTS = INTEGRATION / "contracts"
FIXTURES = INTEGRATION / "fixtures"
EVIDENCE = INTEGRATION / "evidence"
OPERATIONAL_EVIDENCE_SCHEMA_ID = "prisoma.observer.engram-reviewed-development-e2e.v2"
NEST_SUMMARY_SCHEMA_ID = "prisoma.observer.engram-nest-evidence-validation-summary.v1"
CREBAIN_MATRIX_SCHEMA_ID = "prisoma.observer.crebain-real-nest-matrix.v1"
BUILD_RECEIPT_SCHEMA_ID = "prisoma.observer.release-build-receipt.v1"
STAGE_RECEIPT_SCHEMA_ID = "prisoma.observer.package-stage-receipt.v1"
LEGACY_FILES = {
    ROOT / "integrations" / "engram" / "manifest.json": (
        "006a6cc5fe46041fcc180d1890a36f821e8901768161952b143bbfc3c3fd70f9"
    ),
    ROOT / "integrations" / "engram" / "manifest.lock.json": (
        "d49ae548231d214e7081ec610c91964c0edd5c00779339fced9d313fa4787e1e"
    ),
}
MAX_SCHEMA_STEPS = 16_384
PROJECT_SCHEMA_FILES = {
    "prisoma.observer.configuration.v1": "configuration.schema.json",
    "prisoma.observer.finish-request.v1": "finish-request.schema.json",
    "prisoma.observer.finish-response.v1": "finish-response.schema.json",
    "prisoma.observer.observe-request.v1": "observe-request.schema.json",
    "prisoma.observer.observe-response.v1": "observe-response.schema.json",
    "prisoma.observer.prepare-request.v1": "prepare-request.schema.json",
    "prisoma.observer.prepare-response.v1": "prepare-response.schema.json",
}
SOURCE_RUN_FIELDS = {
    "schema_version",
    "digest_canonicalization",
    "study_run_id",
    "study_definition_sha256",
    "closed_loop_definition_sha256",
    "runtime_binding_sha256",
    "runtime_adapter_configuration_sha256",
    "neural_provider_identity_sha256",
    "timebase",
    "planned_step_count",
    "runtime_deadline_enforcement",
    "neural_deadline_enforcement",
    "neural_preparation_sha256",
    "neural_session_receipt_sha256",
    "neural_durable_evidence_profile",
    "initial_snapshot_sha256",
    "last_verified_simulation_time_tics",
    "runtime_progress_disposition",
    "steps",
    "neural_executions",
    "runtime_finish_sha256",
    "runtime_lifecycle",
    "cleanup",
    "status",
    "primary_reason_code",
    "terminal_reason_code",
    "cleanup_complete",
    "transcript_sha256",
    "simulator_only",
    "physical_actuation",
    "ncp_qualified",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
    "receipt_sha256",
}
SOURCE_CLEANUP_FIELDS = {
    "schema_version",
    "component",
    "owner_identity_sha256",
    "mode",
    "attempted",
    "confirmed",
    "containment_empty",
    "reason_code",
    "runtime_lifecycle",
    "provider_terminal_receipt_sha256",
    "provider_lifecycle_receipt_sha256",
    "receipt_sha256",
}
SOURCE_STEP_FIELDS = {
    "schema_version",
    "study_run_id",
    "step_index",
    "step_id",
    "input_snapshot_sha256",
    "neural_request_sha256",
    "neural_result_sha256",
    "provider_execution_scope",
    "provider_execution_sha256",
    "admitted_action_sha256",
    "runtime_request_sha256",
    "output_snapshot_sha256",
    "fault_codes",
    "receipt_sha256",
}
SOURCE_EXECUTION_FIELDS = {
    "schema_version",
    "step_index",
    "step_id",
    "neural_request_sha256",
    "neural_result_sha256",
    "provider_execution_scope",
    "provider_execution_sha256",
    "binding_sha256",
}
SOURCE_TIMEBASE_FIELDS = {
    "schema_version",
    "tic_unit",
    "runtime_step_duration_tics",
    "neural_step_duration_tics",
    "clock_relation",
    "coupling",
    "causality_policy",
    "dispatch_order",
    "observation_sample_phase",
    "action_application",
}
RUNTIME_LIFECYCLE_FIELDS = {
    "schema_version",
    "profile",
    "generation_id",
    "launch_source",
    "store_id",
    "package_generation_id",
    "generation_directory_identity_sha256",
    "package_generation_lease_retained_at_launch",
    "package_generation_lease_released",
    "handshake_receipt_sha256",
    "termination_receipt_sha256",
    "termination_disposition",
    "child_reaped",
    "containment_empty",
    "diagnostic_stream_complete",
    "private_work_directory_removed",
    "publisher_authenticated",
    "durable_process_launch_authority",
    "ncp_authority",
    "physical_authority",
    "scientific_authority",
    "binding_sha256",
}


def fail(reason: str) -> None:
    raise SystemExit(reason)


def closed_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON member: {key}")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    fail(f"non-finite JSON constant: {value}")


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(
            path.read_bytes(),
            object_pairs_hook=closed_object,
            parse_constant=reject_constant,
        )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        fail(f"invalid JSON at {path}: {error}")
    if not isinstance(value, dict):
        fail(f"JSON root is not an object: {path}")
    return value


def canonical(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def digest_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def digest_file(path: Path) -> str:
    return digest_bytes(path.read_bytes())


def digest_without(value: dict[str, Any], field: str) -> str:
    return digest_bytes(
        canonical({key: item for key, item in value.items() if key != field})
    )


def valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def valid_git_oid(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 40
        and all(character in "0123456789abcdef" for character in value)
    )


def valid_prefixed_sha256(value: Any, prefix: str) -> bool:
    return (
        isinstance(value, str)
        and value.startswith(prefix)
        and valid_sha256(value[len(prefix) :])
    )


def valid_bounded_code(value: Any) -> bool:
    return (
        isinstance(value, str)
        and value
        and value == value.strip()
        and len(value.encode("utf-8")) <= 256
        and all(ord(character) >= 33 and ord(character) != 127 for character in value)
    )


def valid_fault_code(value: Any) -> bool:
    return valid_bounded_code(value) and len(value.encode("utf-8")) <= 128


def check_runtime_lifecycle(lifecycle: dict[str, Any]) -> bool:
    if set(lifecycle) != RUNTIME_LIFECYCLE_FIELDS:
        fail("Engram runtime lifecycle field roster differs")
    if (
        lifecycle["schema_version"] != "engram.closed-loop-runtime-lifecycle-binding.v1"
        or lifecycle["profile"] != "engram.reviewed-native-development.v1"
        or not valid_prefixed_sha256(lifecycle["generation_id"], "gen_")
        or not valid_prefixed_sha256(lifecycle["package_generation_id"], "pkggen_")
        or not valid_sha256(lifecycle["handshake_receipt_sha256"])
        or not valid_sha256(lifecycle["termination_receipt_sha256"])
        or lifecycle["termination_disposition"]
        not in {"clean-exit", "terminated", "killed", "unconfirmed"}
        or any(
            lifecycle[field]
            for field in (
                "publisher_authenticated",
                "durable_process_launch_authority",
                "ncp_authority",
                "physical_authority",
                "scientific_authority",
            )
        )
        or (lifecycle["containment_empty"] and not lifecycle["child_reaped"])
    ):
        fail("Engram runtime lifecycle semantic boundary differs")
    if lifecycle["launch_source"] == "package-store-lease":
        if (
            not valid_prefixed_sha256(lifecycle["store_id"], "extstore_")
            or not valid_sha256(lifecycle["generation_directory_identity_sha256"])
            or lifecycle["package_generation_lease_retained_at_launch"] is not True
        ):
            fail("Engram store lifecycle launch identity differs")
    elif lifecycle["launch_source"] == "packed-bundle-path":
        if (
            lifecycle["store_id"] is not None
            or lifecycle["generation_directory_identity_sha256"] is not None
            or lifecycle["package_generation_lease_retained_at_launch"] is not False
            or lifecycle["package_generation_lease_released"] is not False
        ):
            fail("Engram path lifecycle claims store authority")
    else:
        fail("Engram runtime lifecycle launch source differs")
    if lifecycle["binding_sha256"] != digest_without(lifecycle, "binding_sha256"):
        fail("Engram runtime lifecycle binding digest differs")
    return bool(
        lifecycle["termination_disposition"] != "unconfirmed"
        and lifecycle["child_reaped"]
        and lifecycle["containment_empty"]
        and lifecycle["diagnostic_stream_complete"]
        and lifecycle["private_work_directory_removed"]
        and (
            lifecycle["launch_source"] != "package-store-lease"
            or lifecycle["package_generation_lease_released"]
        )
    )


def operation_roster_digest(operations: list[dict[str, Any]]) -> str:
    return digest_bytes(b"engram-managed-operation-roster-v1\0" + canonical(operations))


def imported_source_roster_digest(roster: list[dict[str, Any]]) -> str:
    return digest_bytes(
        b"prisoma-engram-imported-source-roster-v1\0" + canonical(roster)
    )


def valid_imported_source_roster(roster: Any) -> bool:
    if not isinstance(roster, list) or not 1 <= len(roster) <= 256:
        return False
    paths: list[str] = []
    total_bytes = 0
    for row in roster:
        if not isinstance(row, dict) or set(row) != {
            "path",
            "sha256",
            "git_blob",
            "byte_count",
            "module_names",
        }:
            return False
        path = row["path"]
        modules = row["module_names"]
        if (
            not isinstance(path, str)
            or not 1 <= len(path) <= 512
            or path.startswith("/")
            or "\\" in path
            or "\0" in path
            or any(part in {"", ".", ".."} for part in path.split("/"))
            or not valid_sha256(row["sha256"])
            or not valid_git_oid(row["git_blob"])
            or isinstance(row["byte_count"], bool)
            or not isinstance(row["byte_count"], int)
            or not 0 <= row["byte_count"] <= 16 * 1024 * 1024
            or not isinstance(modules, list)
            or not 1 <= len(modules) <= 16
            or modules != sorted(set(modules))
            or any(
                not isinstance(module, str) or not 1 <= len(module) <= 256
                for module in modules
            )
        ):
            return False
        total_bytes += row["byte_count"]
        if total_bytes > 64 * 1024 * 1024:
            return False
        paths.append(path)
    return paths == sorted(set(paths))


def validate_safe_project_schema(schema_id: str, schema: dict[str, Any]) -> None:
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        fail(f"{schema_id}: schema dialect differs")
    if schema.get("$id") != (
        f"https://engram.local/extension-contracts/{schema_id}.json"
    ):
        fail(f"{schema_id}: document identifier differs")

    def walk(value: Any, *, root: bool = False) -> None:
        if isinstance(value, dict):
            if not root and ("$id" in value or "$schema" in value):
                fail(f"{schema_id}: nested identifier or dialect")
            for key, child in value.items():
                if key in {"pattern", "patternProperties", "format", "$dynamicRef"}:
                    fail(f"{schema_id}: unsafe schema keyword {key}")
                if key == "$ref" and (
                    not isinstance(child, str) or not child.startswith("#/$defs/")
                ):
                    fail(f"{schema_id}: non-local schema reference")
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)

    walk(schema, root=True)


def schema_accepts(instance: Any, schema: dict[str, Any]) -> bool:
    steps = 0
    definitions = schema.get("$defs", {})

    def evaluate(value: Any, rule: dict[str, Any]) -> bool:
        nonlocal steps
        steps += 1
        if steps > MAX_SCHEMA_STEPS:
            return False
        reference = rule.get("$ref")
        if reference is not None:
            token = reference.removeprefix("#/$defs/")
            target = definitions.get(token)
            return isinstance(target, dict) and evaluate(value, target)
        if "anyOf" in rule:
            options = rule["anyOf"]
            return isinstance(options, list) and any(
                isinstance(option, dict) and evaluate(value, option)
                for option in options
            )
        if "allOf" in rule:
            options = rule["allOf"]
            return isinstance(options, list) and all(
                isinstance(option, dict) and evaluate(value, option)
                for option in options
            )
        if "const" in rule and value != rule["const"]:
            return False
        if "enum" in rule and value not in rule["enum"]:
            return False
        expected = rule.get("type")
        if expected == "null" and value is not None:
            return False
        if expected == "boolean" and not isinstance(value, bool):
            return False
        if expected == "integer" and (
            isinstance(value, bool) or not isinstance(value, int)
        ):
            return False
        if expected == "string" and not isinstance(value, str):
            return False
        if expected == "array" and not isinstance(value, list):
            return False
        if expected == "object" and not isinstance(value, dict):
            return False
        if isinstance(value, int) and not isinstance(value, bool):
            if value < rule.get("minimum", value) or value > rule.get("maximum", value):
                return False
        if isinstance(value, str):
            if len(value) < rule.get("minLength", 0):
                return False
            if len(value) > rule.get("maxLength", len(value)):
                return False
        if isinstance(value, list):
            if len(value) < rule.get("minItems", 0):
                return False
            if len(value) > rule.get("maxItems", len(value)):
                return False
            if rule.get("uniqueItems") and len(
                {canonical(item) for item in value}
            ) != len(value):
                return False
            prefix_rules = rule.get("prefixItems", [])
            if not isinstance(prefix_rules, list) or not all(
                isinstance(prefix_rule, dict) and evaluate(value[index], prefix_rule)
                for index, prefix_rule in enumerate(prefix_rules)
                if index < len(value)
            ):
                return False
            item_rule = rule.get("items")
            remaining = value[len(prefix_rules) :]
            if item_rule is False and remaining:
                return False
            if isinstance(item_rule, dict) and not all(
                evaluate(item, item_rule) for item in remaining
            ):
                return False
        if isinstance(value, dict):
            if len(value) < rule.get("minProperties", 0):
                return False
            if len(value) > rule.get("maxProperties", len(value)):
                return False
            required = rule.get("required", [])
            properties = rule.get("properties", {})
            if not all(field in value for field in required):
                return False
            if rule.get("additionalProperties") is False and any(
                field not in properties for field in value
            ):
                return False
            if not all(
                field not in value or evaluate(value[field], child)
                for field, child in properties.items()
                if isinstance(child, dict)
            ):
                return False
        return True

    return evaluate(instance, schema)


def check_nest_summary_schema_controls(schema: dict[str, Any]) -> None:
    digest = "a" * 64
    repository = {
        "repository": "https://github.com/sepahead/example.git",
        "commit": "c" * 40,
        "tree": "d" * 40,
        "origin_main": "c" * 40,
        "object_format": "sha1",
        "clean": True,
    }
    validator_sources = [
        {
            "path": path,
            "sha256": digest,
            "git_blob": "e" * 40,
            "byte_count": 1,
        }
        for path in (
            "integrations/engram/managed-observer/evidence/engram-nest-evidence-validation-summary.schema.json",
            "integrations/engram/managed-observer/scripts/source_provenance.py",
            "integrations/engram/managed-observer/scripts/summarize-nest-evidence.py",
        )
    ]
    source = {
        "path": "backend/optimization/extension_closed_loop.py",
        "sha256": digest,
        "git_blob": "b" * 40,
        "byte_count": 1,
        "module_names": ["backend.optimization.extension_closed_loop"],
    }
    summary = {
        "schema_version": NEST_SUMMARY_SCHEMA_ID,
        "validation_scope": "engram-exact-validator-rejoin-only",
        "prisoma_repository": repository,
        "prisoma_validator_source_roster_sha256": digest,
        "prisoma_validator_source_roster": validator_sources,
        "engram_repository": repository,
        "engram_revision": "c" * 40,
        "engram_imported_source_roster_sha256": digest,
        "engram_imported_source_roster": [source],
        "inputs": {
            "summary_schema_exact_sha256": digest_file(
                EVIDENCE / "engram-nest-evidence-validation-summary.schema.json"
            ),
            "run_receipt_exact_sha256": digest,
            "evidence_bundle_exact_sha256": digest,
            "source_run_receipt_sha256": digest,
            "source_bundle_sha256": digest,
            "validation_input_sha256": digest,
        },
        "lineage": {
            "study_run_id": "study-run",
            "neural_durable_evidence_profile": (
                "engram.nest-closed-loop-evidence-bundle.v2"
            ),
            "neural_provider_identity_sha256": digest,
            "run_status": "completed",
            "preparation_phase": "provider-prepare",
            "preparation_outcome": "succeeded",
            "completed_step_count": 2,
            "terminal_neural_execution_count": 2,
            "provider_step_execution_count": 2,
            "provider_step_attempt_count": 2,
            "worker_termination_attempt_count": 1,
            "worker_terminal_disposition": "confirmed-lifecycle",
            "tail_disposition_receipt_sha256": digest,
            "worker_lifecycle_receipt_sha256": digest,
            "neural_cleanup_confirmed": True,
            "validated_against_run": True,
        },
        "authority": {
            "descriptive_only": True,
            "source_durable_evidence_verified": True,
            "engram_loaded_source_bytes_attested": False,
            "agent_bridge_command": False,
            "execution_authority": False,
            "pid_result": False,
            "ncp_authority": False,
            "physical_authority": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
        "receipt_sha256": digest,
    }
    if not schema_accepts(summary, schema):
        fail("external NEST validation summary positive control rejects")
    negative_controls: list[dict[str, Any]] = []
    changed = copy.deepcopy(summary)
    changed["authority"]["source_durable_evidence_verified"] = False
    negative_controls.append(changed)
    changed = copy.deepcopy(summary)
    changed["authority"]["ncp_authority"] = True
    negative_controls.append(changed)
    changed = copy.deepcopy(summary)
    changed["authority"]["engram_loaded_source_bytes_attested"] = True
    negative_controls.append(changed)
    changed = copy.deepcopy(summary)
    changed["lineage"]["neural_durable_evidence_profile"] = "none"
    negative_controls.append(changed)
    changed = copy.deepcopy(summary)
    changed["engram_imported_source_roster"] = []
    negative_controls.append(changed)
    changed = copy.deepcopy(summary)
    changed["unbounded_projection"] = {}
    negative_controls.append(changed)
    if any(schema_accepts(changed, schema) for changed in negative_controls):
        fail("external NEST validation summary negative control accepts")


def check_source_receipt(source: dict[str, Any], expected_steps: int) -> None:
    if set(source) != SOURCE_RUN_FIELDS:
        fail("Engram source run receipt field roster differs")
    if (
        source["schema_version"] != "engram.extension-closed-loop-run-receipt.v2"
        or source["digest_canonicalization"] != "engram.managed-runtime-json.v1"
        or not all(
            valid_sha256(source[field])
            for field in (
                "study_definition_sha256",
                "closed_loop_definition_sha256",
                "runtime_binding_sha256",
                "runtime_adapter_configuration_sha256",
                "neural_provider_identity_sha256",
                "transcript_sha256",
                "receipt_sha256",
            )
        )
        or any(
            source[field] is not None and not valid_sha256(source[field])
            for field in (
                "neural_preparation_sha256",
                "neural_session_receipt_sha256",
                "initial_snapshot_sha256",
                "runtime_finish_sha256",
            )
        )
        or source["runtime_deadline_enforcement"]
        not in {"host-generation-kill", "cooperative-observed", "deterministic-test"}
        or source["neural_deadline_enforcement"]
        not in {"host-generation-kill", "cooperative-observed", "deterministic-test"}
        or source["neural_durable_evidence_profile"]
        not in {"none", "engram.nest-closed-loop-evidence-bundle.v2"}
        or source["status"] not in {"completed", "cancelled", "overloaded", "failed"}
        or not valid_bounded_code(source["primary_reason_code"])
        or not valid_bounded_code(source["terminal_reason_code"])
    ):
        fail("Engram source run receipt version or canonicalization differs")
    timebase = source["timebase"]
    if not isinstance(timebase, dict) or set(timebase) != SOURCE_TIMEBASE_FIELDS:
        fail("Engram source timebase field roster differs")
    if (
        timebase["schema_version"] != "engram.extension-closed-loop-timebase.v1"
        or timebase["tic_unit"] != "microsecond"
        or isinstance(timebase["runtime_step_duration_tics"], bool)
        or not isinstance(timebase["runtime_step_duration_tics"], int)
        or not 1 <= timebase["runtime_step_duration_tics"] <= 10_000_000
        or isinstance(timebase["neural_step_duration_tics"], bool)
        or not isinstance(timebase["neural_step_duration_tics"], int)
        or not 1 <= timebase["neural_step_duration_tics"] <= 10_000_000
        or timebase["clock_relation"]
        != "independent-controller-and-runtime-logical-clocks"
        or timebase["coupling"] != "one-controller-epoch-per-runtime-interval"
        or timebase["causality_policy"] != "sample-runtime-run-controller-apply-zoh-v1"
        or timebase["dispatch_order"] != "observe-controller-action-runtime"
        or timebase["observation_sample_phase"] != "runtime-interval-start"
        or timebase["action_application"]
        != "after-controller-completion-zoh-over-runtime-interval"
    ):
        fail("Engram source timebase semantics differ")
    steps = source["steps"]
    executions = source["neural_executions"]
    cleanup = source["cleanup"]
    if not isinstance(steps, list) or len(steps) != expected_steps:
        fail("Engram source step count differs")
    if (
        not isinstance(executions, list)
        or not len(steps) <= len(executions) <= len(steps) + 1
        or len(executions) > 1024
    ):
        fail("Engram source neural execution count differs")
    planned_step_count = source["planned_step_count"]
    if (
        isinstance(planned_step_count, bool)
        or not isinstance(planned_step_count, int)
        or not 1 <= planned_step_count <= 1024
        or len(steps) > planned_step_count
    ):
        fail("Engram source immutable step plan differs")
    if not isinstance(cleanup, list) or len(cleanup) != 2:
        fail("Engram source cleanup roster differs")
    for index, step in enumerate(steps, start=1):
        if set(step) != SOURCE_STEP_FIELDS:
            fail("Engram source step receipt field roster differs")
        if step.get("step_index") != index:
            fail("Engram source steps are not contiguous")
        expected_step_id = (
            "step_"
            + digest_bytes(
                canonical(
                    {
                        "domain": "engram-extension-closed-loop-step-v2",
                        "run_id": source["study_run_id"],
                        "step_index": index,
                    }
                )
            )[:32]
        )
        if (
            step.get("schema_version") != "engram.extension-closed-loop-step-receipt.v2"
            or step.get("study_run_id") != source["study_run_id"]
            or step.get("step_id") != expected_step_id
            or step.get("provider_execution_scope")
            not in {"decoded-proposal-only", "nest-exact-step-readback"}
            or not all(
                valid_sha256(step.get(field))
                for field in (
                    "input_snapshot_sha256",
                    "neural_request_sha256",
                    "neural_result_sha256",
                    "provider_execution_sha256",
                    "admitted_action_sha256",
                    "runtime_request_sha256",
                    "output_snapshot_sha256",
                    "receipt_sha256",
                )
            )
            or not isinstance(step["fault_codes"], list)
            or not 1 <= len(step["fault_codes"]) <= 64
            or not all(valid_fault_code(code) for code in step["fault_codes"])
        ):
            fail("Engram source step identity differs")
        if step.get("receipt_sha256") != digest_without(step, "receipt_sha256"):
            fail("Engram source step digest differs")
    if steps and (
        steps[0].get("input_snapshot_sha256") != source["initial_snapshot_sha256"]
        or any(
            current.get("input_snapshot_sha256")
            != previous.get("output_snapshot_sha256")
            for previous, current in zip(steps, steps[1:], strict=False)
        )
    ):
        fail("Engram source snapshot chain differs")
    for index, execution in enumerate(executions, start=1):
        if set(execution) != SOURCE_EXECUTION_FIELDS:
            fail("Engram neural execution field roster differs")
        expected_step_id = (
            "step_"
            + digest_bytes(
                canonical(
                    {
                        "domain": "engram-extension-closed-loop-step-v2",
                        "run_id": source["study_run_id"],
                        "step_index": index,
                    }
                )
            )[:32]
        )
        if (
            execution["schema_version"]
            != "engram.closed-loop-neural-execution-binding.v1"
            or execution["step_index"] != index
            or execution["step_id"] != expected_step_id
            or execution["provider_execution_scope"]
            not in {"decoded-proposal-only", "nest-exact-step-readback"}
            or not all(
                valid_sha256(execution[field])
                for field in (
                    "neural_request_sha256",
                    "neural_result_sha256",
                    "provider_execution_sha256",
                )
            )
            or execution["binding_sha256"]
            != digest_without(execution, "binding_sha256")
        ):
            fail("Engram neural execution binding differs")
    for step, execution in zip(steps, executions, strict=False):
        if any(
            step[field] != execution[field]
            for field in (
                "step_index",
                "step_id",
                "neural_request_sha256",
                "neural_result_sha256",
                "provider_execution_scope",
                "provider_execution_sha256",
            )
        ):
            fail("Engram neural execution roster differs from step lineage")
    for item in cleanup:
        if set(item) != SOURCE_CLEANUP_FIELDS:
            fail("Engram cleanup receipt field roster differs")
        if (
            item["schema_version"] != "engram.closed-loop-cleanup.v2"
            or item["component"] not in {"runtime", "neural"}
            or item["mode"] not in {"finish", "generation-kill", "close"}
            or item["attempted"] is not True
            or not isinstance(item["confirmed"], bool)
            or not isinstance(item["containment_empty"], bool)
            or (item["confirmed"] and not item["containment_empty"])
            or not valid_sha256(item["owner_identity_sha256"])
            or not valid_sha256(item["receipt_sha256"])
            or not valid_bounded_code(item["reason_code"])
            or any(
                value is not None and not valid_sha256(value)
                for value in (
                    item["provider_terminal_receipt_sha256"],
                    item["provider_lifecycle_receipt_sha256"],
                )
            )
        ):
            fail("Engram cleanup receipt semantics differ")
        lifecycle = item["runtime_lifecycle"]
        if lifecycle is not None and item["confirmed"] is not check_runtime_lifecycle(
            lifecycle
        ):
            fail("Engram cleanup differs from runtime lifecycle")
        if lifecycle is not None and (
            (
                item["mode"] == "finish"
                and lifecycle["termination_disposition"] != "clean-exit"
            )
            or (
                item["mode"] == "generation-kill"
                and lifecycle["termination_disposition"] not in {"terminated", "killed"}
            )
        ):
            fail("Engram cleanup mode differs from runtime lifecycle")
        if item.get("receipt_sha256") != digest_without(item, "receipt_sha256"):
            fail("Engram cleanup digest differs")
    runtime_cleanup, neural_cleanup = cleanup
    if (
        runtime_cleanup["component"] != "runtime"
        or neural_cleanup["component"] != "neural"
        or runtime_cleanup["owner_identity_sha256"] != source["runtime_binding_sha256"]
        or neural_cleanup["owner_identity_sha256"]
        != source["neural_provider_identity_sha256"]
        or runtime_cleanup["owner_identity_sha256"]
        == neural_cleanup["owner_identity_sha256"]
        or neural_cleanup["mode"] != "close"
        or runtime_cleanup["runtime_lifecycle"] != source["runtime_lifecycle"]
        or neural_cleanup["runtime_lifecycle"] is not None
    ):
        fail("Engram cleanup owner roster differs")
    expected_runtime_mode = (
        "finish" if source["runtime_finish_sha256"] is not None else "generation-kill"
    )
    if runtime_cleanup["mode"] != expected_runtime_mode:
        fail("Engram runtime cleanup mode differs")
    if (source["neural_preparation_sha256"] is None) != (
        source["neural_session_receipt_sha256"] is None
    ):
        fail("Engram neural preparation and session lineage is unpaired")
    cleanup_complete = all(
        item["confirmed"] and item["containment_empty"] for item in cleanup
    )
    if source["cleanup_complete"] is not cleanup_complete:
        fail("Engram cleanup disposition differs")
    preparation_present = source["neural_preparation_sha256"] is not None
    if preparation_present and source["initial_snapshot_sha256"] is None:
        fail("Engram neural preparation lacks an initial snapshot")
    finish_present = source["runtime_finish_sha256"] is not None
    if finish_present != (source["primary_reason_code"] == "loop.completed"):
        fail("Engram runtime finish and completed reason are unpaired")
    if not cleanup_complete:
        expected_status = "failed"
    elif finish_present:
        expected_status = "completed"
    elif source["primary_reason_code"] == "loop.cancelled":
        expected_status = "cancelled"
    elif source["primary_reason_code"] == "runtime.overload":
        expected_status = "overloaded"
    else:
        expected_status = "failed"
    if source["status"] != expected_status:
        fail("Engram source status differs from terminal evidence")
    if finish_present and (
        source["initial_snapshot_sha256"] is None
        or source["neural_preparation_sha256"] is None
        or source["neural_session_receipt_sha256"] is None
        or len(steps) != planned_step_count
    ):
        fail("Engram runtime finish lineage is incomplete")
    if source["status"] == "completed" and (
        source["initial_snapshot_sha256"] is None
        or source["neural_preparation_sha256"] is None
        or source["neural_session_receipt_sha256"] is None
        or not steps
        or source["runtime_finish_sha256"] is None
        or not source["cleanup_complete"]
    ):
        fail("Engram completed source lineage is incomplete")
    if steps and (
        source["initial_snapshot_sha256"] is None
        or source["neural_preparation_sha256"] is None
        or source["neural_session_receipt_sha256"] is None
    ):
        fail("Engram step source lineage is incomplete")
    expected_verified_time = (
        len(steps) * timebase["runtime_step_duration_tics"]
        if source["initial_snapshot_sha256"] is not None
        else None
    )
    if source["last_verified_simulation_time_tics"] != expected_verified_time:
        fail("Engram last verified runtime time differs")
    progress = source["runtime_progress_disposition"]
    if finish_present:
        expected_progress = "finished-and-host-verified"
    elif progress in {"unknown-after-dispatch", "unknown-after-operation-attempt"}:
        expected_progress = progress
    elif source["initial_snapshot_sha256"] is None:
        expected_progress = "not-started"
    else:
        expected_progress = "last-host-verified"
    if progress != expected_progress:
        fail("Engram runtime progress disposition differs")
    expected_terminal_reason = (
        source["primary_reason_code"]
        if source["cleanup_complete"]
        else "cleanup.unconfirmed"
    )
    if source["terminal_reason_code"] != expected_terminal_reason:
        fail("Engram terminal reason differs")
    transcript = {
        "domain": "engram-extension-closed-loop-transcript-v5",
        "digest_canonicalization": source["digest_canonicalization"],
        "planned_step_count": planned_step_count,
        "timebase": timebase,
        "neural_preparation_sha256": source["neural_preparation_sha256"],
        "neural_session_receipt_sha256": source["neural_session_receipt_sha256"],
        "neural_durable_evidence_profile": source["neural_durable_evidence_profile"],
        "initial_snapshot_sha256": source["initial_snapshot_sha256"],
        "last_verified_simulation_time_tics": source[
            "last_verified_simulation_time_tics"
        ],
        "runtime_progress_disposition": progress,
        "step_receipts": [item["receipt_sha256"] for item in steps],
        "neural_execution_bindings": [item["binding_sha256"] for item in executions],
        "runtime_finish_sha256": source["runtime_finish_sha256"],
        "runtime_lifecycle_binding_sha256": (
            source["runtime_lifecycle"]["binding_sha256"]
            if source["runtime_lifecycle"] is not None
            else None
        ),
        "cleanup_receipts": [item["receipt_sha256"] for item in cleanup],
        "status": source["status"],
        "primary_reason_code": source["primary_reason_code"],
        "terminal_reason_code": source["terminal_reason_code"],
    }
    if source["transcript_sha256"] != digest_bytes(canonical(transcript)):
        fail("Engram source transcript digest differs")
    if source["receipt_sha256"] != digest_without(source, "receipt_sha256"):
        fail("Engram source run digest differs")
    if source["simulator_only"] is not True or any(
        source[field]
        for field in (
            "physical_actuation",
            "ncp_qualified",
            "scientific_authority",
            "is_paper_local_evidence",
            "calibrated_posterior",
        )
    ):
        fail("Engram source authority boundary differs")


def check_transcript(
    transcript: dict[str, Any],
    schemas: dict[str, dict[str, Any]],
    roster_digest: str,
) -> None:
    if (
        transcript.get("schema_version") != "prisoma.observer.sample-transcript.v1"
        or transcript.get("fixture_only") is not True
        or transcript.get("real_binary_executed") is not True
        or transcript.get("operation_roster_sha256") != roster_digest
    ):
        fail("sample transcript header differs")
    scenario = transcript.get("scenario")
    if scenario != {
        "subject_count": 3,
        "subject_ids": ["drone-01", "drone-02", "drone-03"],
        "observed_step_count": 2,
        "authority": "read-only-observer",
        "roster_authority": "host-declared-projection",
        "source_roster_authenticated": False,
        "agent_bridge_command": False,
        "ncp_mode": "none",
    }:
        fail("sample transcript scenario differs")
    frames = transcript.get("frames")
    if not isinstance(frames, list) or len(frames) != 10:
        fail("sample transcript frame roster differs")
    message_ids: set[str] = set()
    for row in frames:
        envelope = row.get("envelope")
        if not isinstance(envelope, dict):
            fail("sample transcript envelope is absent")
        payload = canonical(envelope)
        if (
            row.get("payload_length") != len(payload)
            or row.get("prefix_hex") != struct.pack(">I", len(payload)).hex()
            or row.get("payload_sha256") != digest_bytes(payload)
        ):
            fail("sample transcript frame record differs")
        message_id = envelope.get("message_id")
        if not isinstance(message_id, str) or message_id in message_ids:
            fail("sample transcript message identifier replays")
        message_ids.add(message_id)
        if (
            row.get("direction") == "host-to-runtime"
            and envelope.get("kind") == "operation.request"
        ):
            body = envelope["body"]
            operation = body["operation"]
            if (
                operation["class"] != "observation"
                or operation["effect"] != "none"
                or body["compute_grant"] != {"mode": "none"}
                or body["bulk"] != {"inline": False, "references": []}
            ):
                fail("sample transcript request grants authority")
            schema_id = body["request_schema"]["schema_id"]
            if not schema_accepts(body["control"], schemas[schema_id]):
                fail(f"sample request fails {schema_id}")
        if (
            row.get("direction") == "runtime-to-host"
            and envelope.get("kind") == "operation.response"
        ):
            body = envelope["body"]
            control = body["control"]
            schema_id = body["response_schema"]["schema_id"]
            if not schema_accepts(control, schemas[schema_id]):
                fail(f"sample response fails {schema_id}")
            if (
                control["authority"] != "read-only-observer"
                or control["roster_authority"] != "host-declared-projection"
                or control["source_roster_authenticated"]
                or not control["descriptive_only"]
                or any(
                    control[field]
                    for field in (
                        "agent_bridge_command",
                        "physical_actuation",
                        "ncp_used",
                        "pid_result",
                        "source_durable_evidence_verified",
                        "scientific_authority",
                        "is_paper_local_evidence",
                        "calibrated_posterior",
                    )
                )
            ):
                fail("sample response grants authority")
    if frames[-1]["envelope"]["body"]["control"]["state_cleared"] is not True:
        fail("sample terminal response did not clear state")


def check_operational_evidence_schema_controls(
    receipt: dict[str, Any], schema: dict[str, Any]
) -> None:
    if not schema_accepts(receipt, schema):
        fail("reviewed-development evidence schema positive control rejects")
    mutations: tuple[tuple[tuple[str, ...], Any], ...] = (
        (("schema_version",), "prisoma.observer.engram-reviewed-development-e2e.v1"),
        (("session", "handshake", "explicit_absolute_path_spawn"), False),
        (("session", "handshake", "path_lookup_at_spawn"), False),
        (("session", "handshake", "package_path_reopened_for_spawn"), True),
        (("session", "handshake", "verified_executable_staged"), False),
        (("session", "handshake", "staged_executable_owner_private"), False),
        (("session", "handshake", "staged_executable_user_immutable"), False),
        (("session", "handshake", "guardian_owner_loss_seal"), False),
        (("session", "handshake", "guardian_generation_lease_retained"), False),
        (
            ("session", "handshake", "guardian_uncertainty_record_prepared"),
            False,
        ),
        (("session", "handshake", "filesystem_isolation_enforced"), True),
        (
            ("session", "handshake", "external_dependency_closure_attested"),
            True,
        ),
        (("session", "handshake", "automatic_restart"), True),
        (("session", "handshake", "replayable_live_launch_authority"), True),
        (("session", "termination", "guardian_reaped"), False),
        (
            (
                "session",
                "termination",
                "group_signal_while_guardian_unreaped",
            ),
            False,
        ),
        (
            (
                "session",
                "termination",
                "direct_child_signal_while_unreaped",
            ),
            True,
        ),
        (("session", "termination", "containment_signal_scope"), "direct-child"),
        (("session", "termination", "containment_seal_signal"), 15),
        (
            (
                "session",
                "termination",
                "guardian_generation_lease_held_until_containment",
            ),
            False,
        ),
    )
    for path, value in mutations:
        changed = copy.deepcopy(receipt)
        target: dict[str, Any] = changed
        for field in path[:-1]:
            nested = target.get(field)
            if not isinstance(nested, dict):
                fail("reviewed-development evidence control path differs")
            target = nested
        target[path[-1]] = value
        if schema_accepts(changed, schema):
            fail(
                "reviewed-development evidence schema negative control accepts: "
                + "/".join(path)
            )
    for section in ("handshake", "termination"):
        for identifier in ("guardian_pid", "process_group_id"):
            changed = copy.deepcopy(receipt)
            changed["session"][section][identifier] = 1
            if schema_accepts(changed, schema):
                fail(
                    "reviewed-development evidence exposes numeric process identity: "
                    f"{section}/{identifier}"
                )


def check_historical_operational_evidence(
    receipt: dict[str, Any],
    provenance: dict[str, Any],
) -> None:
    evidence_path = EVIDENCE / "engram-reviewed-development-e2e.json"
    schema_path = EVIDENCE / "engram-reviewed-development-e2e.schema.json"
    historical = provenance.get("historical_operational_evidence")
    current = provenance.get("current_operational_evidence")
    if (
        not isinstance(historical, dict)
        or set(historical)
        != {
            "status",
            "path",
            "sha256",
            "schema_id",
            "historical_schema_sha256",
            "receipt_sha256",
            "profile",
            "engram_revision",
            "closed_loop_source_sha256",
            "development_launcher_source_sha256",
            "package_store_source_sha256",
            "sample_transcript_exact_sha256",
            "source_fixture_exact_sha256",
            "reviewed_development_only",
            "production_manager_execution",
            "publisher_authenticated",
            "ncp_authority",
            "physical_authority",
            "scientific_authority",
        }
        or historical["status"] != "historical-audit-only-v1"
        or historical["path"] != str(evidence_path.relative_to(ROOT))
        or historical["sha256"] != digest_file(evidence_path)
        or historical["schema_id"]
        != "prisoma.observer.engram-reviewed-development-e2e.v1"
        or not valid_sha256(historical["historical_schema_sha256"])
        or historical["receipt_sha256"] != receipt.get("receipt_sha256")
        or historical["profile"] != "engram.reviewed-native-development.v1"
        or historical["reviewed_development_only"] is not True
        or any(
            historical[field]
            for field in (
                "production_manager_execution",
                "publisher_authenticated",
                "ncp_authority",
                "physical_authority",
                "scientific_authority",
            )
        )
    ):
        fail("historical reviewed-development provenance differs")
    if (
        not isinstance(current, dict)
        or set(current)
        != {
            "status",
            "schema_id",
            "schema_path",
            "schema_sha256",
            "production_manager_execution",
            "publisher_authenticated",
            "source_durable_evidence_verified",
            "ncp_authority",
            "physical_authority",
            "scientific_authority",
        }
        or current["status"] != "NOT RUN"
        or current["schema_id"] != OPERATIONAL_EVIDENCE_SCHEMA_ID
        or current["schema_path"] != str(schema_path.relative_to(ROOT))
        or current["schema_sha256"] != digest_file(schema_path)
        or any(
            current[field]
            for field in (
                "production_manager_execution",
                "publisher_authenticated",
                "source_durable_evidence_verified",
                "ncp_authority",
                "physical_authority",
                "scientific_authority",
            )
        )
    ):
        fail("current reviewed-development operational gate is not closed")
    source = receipt.get("source")
    authority = receipt.get("authority")
    package = receipt.get("package")
    session = receipt.get("session")
    scenario = receipt.get("scenario")
    if not all(
        isinstance(value, dict)
        for value in (source, authority, package, session, scenario)
    ):
        fail("historical reviewed-development evidence shape differs")
    handshake = session.get("handshake")
    termination = session.get("termination")
    if not isinstance(handshake, dict) or not isinstance(termination, dict):
        fail("historical reviewed-development lifecycle is absent")
    expected_authority = {
        "agent_bridge_command": False,
        "calibrated_posterior": False,
        "compute_grant": "none",
        "descriptive_only": True,
        "durable_process_launch_authority": False,
        "execution_authority": False,
        "is_paper_local_evidence": False,
        "ncp_authority": False,
        "operation_class": "observation",
        "physical_authority": False,
        "publisher_authenticated": False,
        "scientific_authority": False,
        "store_installation_authority": False,
    }
    if (
        receipt.get("schema_version")
        != "prisoma.observer.engram-reviewed-development-e2e.v1"
        or receipt.get("reviewed_development_only") is not True
        or receipt.get("production_manager_execution") is not False
        or receipt.get("receipt_sha256") != digest_without(receipt, "receipt_sha256")
        or source.get("source_state")
        != "working-tree-candidate-not-contained-in-recorded-revision"
        or source.get("engram_revision") != historical["engram_revision"]
        or source.get("closed_loop_source_sha256")
        != historical["closed_loop_source_sha256"]
        or source.get("development_launcher_source_sha256")
        != historical["development_launcher_source_sha256"]
        or source.get("package_store_source_sha256")
        != historical["package_store_source_sha256"]
        or source.get("sample_transcript_exact_sha256")
        != historical["sample_transcript_exact_sha256"]
        or source.get("source_fixture_exact_sha256")
        != historical["source_fixture_exact_sha256"]
        or authority != expected_authority
        or package.get("target_id") != "macos-aarch64-darwin"
        or package.get("publisher_authentication") != "publisher-unattested"
        or scenario.get("source_roster_authenticated") is not False
        or handshake.get("filesystem_isolation_enforced") is not False
        or handshake.get("path_lookup_at_spawn") is not True
        or termination.get("handshake_receipt_sha256")
        != handshake.get("receipt_sha256")
        or termination.get("child_reaped") is not True
        or termination.get("containment_empty") is not True
        or any(
            row.get(field) is not False
            for row in (handshake, termination)
            for field in (
                "durable_process_launch_authority",
                "ncp_authority",
                "physical_authority",
                "scientific_authority",
            )
        )
    ):
        fail("historical reviewed-development evidence boundary differs")


def check_operational_evidence(
    receipt: dict[str, Any],
    schema: dict[str, Any],
    provenance: dict[str, Any],
    success: dict[str, Any],
    transcript: dict[str, Any],
    roster_digest: str,
) -> None:
    evidence_path = EVIDENCE / "engram-reviewed-development-e2e.json"
    schema_path = EVIDENCE / "engram-reviewed-development-e2e.schema.json"
    evidence_provenance = provenance["current_operational_evidence"]
    if (
        set(evidence_provenance)
        != {
            "status",
            "schema_id",
            "path",
            "schema_path",
            "sha256",
            "schema_sha256",
            "receipt_sha256",
            "profile",
            "engram_revision",
            "engram_imported_source_roster_sha256",
            "engram_loaded_source_bytes_attested",
            "input_bundle_exact_sha256",
            "reviewed_development_only",
            "production_manager_execution",
            "publisher_authenticated",
            "ncp_authority",
            "physical_authority",
            "source_durable_evidence_verified",
            "scientific_authority",
        }
        or evidence_provenance["status"] != "observed-reviewed-development-v2"
        or evidence_provenance["schema_id"] != OPERATIONAL_EVIDENCE_SCHEMA_ID
        or not schema_accepts(receipt, schema)
        or evidence_provenance["path"] != str(evidence_path.relative_to(ROOT))
        or evidence_provenance["schema_path"] != str(schema_path.relative_to(ROOT))
        or evidence_provenance["sha256"] != digest_file(evidence_path)
        or evidence_provenance["schema_sha256"] != digest_file(schema_path)
        or evidence_provenance["receipt_sha256"] != receipt.get("receipt_sha256")
        or receipt.get("receipt_sha256") != digest_without(receipt, "receipt_sha256")
    ):
        fail("reviewed-development evidence or provenance differs")
    check_operational_evidence_schema_controls(receipt, schema)
    source = receipt["source"]
    source_roster = source.get("engram_imported_source_roster")
    if not valid_imported_source_roster(source_roster):
        fail("reviewed-development imported source roster differs")
    source_roster_sha256 = imported_source_roster_digest(source_roster)
    source_by_path = {
        row["path"]: row
        for row in source_roster
        if isinstance(row, dict) and isinstance(row.get("path"), str)
    }
    required_paths = {
        "backend/integrations/extension_package_store.py",
        "backend/integrations/reviewed_native_development_session.py",
        "backend/integrations/standard_closed_loop_simulator.py",
        "backend/optimization/extension_closed_loop.py",
    }
    if (
        not required_paths.issubset(source_by_path)
        or source["engram_imported_source_roster_sha256"] != source_roster_sha256
        or source["engram_loaded_source_bytes_attested"] is not False
        or source_roster_sha256
        != evidence_provenance["engram_imported_source_roster_sha256"]
        or source["engram_revision"] != evidence_provenance["engram_revision"]
        or source_by_path["backend/optimization/extension_closed_loop.py"]["sha256"]
        != source["closed_loop_source_sha256"]
        or source["sample_transcript_exact_sha256"]
        != digest_file(INTEGRATION / "sample-transcript.json")
        or source["source_fixture_exact_sha256"]
        != digest_file(FIXTURES / "engram-run-receipt.generated.json")
        or source["input_bundle_exact_sha256"]
        != evidence_provenance["input_bundle_exact_sha256"]
        or source["source_run_receipt_sha256"] != success["receipt_sha256"]
    ):
        fail("reviewed-development source binding differs")
    if (
        evidence_provenance["profile"] != "engram.reviewed-native-development.v1"
        or evidence_provenance["reviewed_development_only"] is not True
        or any(
            evidence_provenance[field]
            for field in (
                "production_manager_execution",
                "publisher_authenticated",
                "ncp_authority",
                "physical_authority",
                "source_durable_evidence_verified",
                "engram_loaded_source_bytes_attested",
                "scientific_authority",
            )
        )
    ):
        fail("reviewed-development provenance grants authority")
    package = receipt["package"]
    session = receipt["session"]
    handshake = session["handshake"]
    termination = session["termination"]
    expected_handshake_observations = {
        "explicit_absolute_path_spawn": True,
        "path_lookup_at_spawn": True,
        "package_path_reopened_for_spawn": False,
        "verified_executable_staged": True,
        "staged_executable_owner_private": True,
        "staged_executable_user_immutable": True,
        "guardian_owner_loss_seal": True,
        "guardian_generation_lease_retained": True,
        "guardian_uncertainty_record_prepared": True,
        "filesystem_isolation_enforced": False,
        "external_dependency_closure_attested": False,
        "automatic_restart": False,
        "replayable_live_launch_authority": False,
    }
    expected_termination_observations = {
        "guardian_reaped": True,
        "group_signal_while_guardian_unreaped": True,
        "direct_child_signal_while_unreaped": False,
        "containment_signal_scope": "process-group",
        "containment_seal_signal": 9,
        "guardian_generation_lease_held_until_containment": True,
    }
    if (
        package["operation_roster_sha256"] != roster_digest
        or not valid_prefixed_sha256(package["installation_id"], "inst_")
        or not valid_prefixed_sha256(package["package_generation_id"], "pkggen_")
        or not valid_prefixed_sha256(session["generation_id"], "gen_")
        or handshake["package_generation_id"] != package["package_generation_id"]
        or handshake["store_id"] != receipt["store"]["store_id"]
        or termination["handshake_receipt_sha256"] != handshake["receipt_sha256"]
        or any(
            handshake.get(field) != value
            for field, value in expected_handshake_observations.items()
        )
        or any(
            termination.get(field) != value
            for field, value in expected_termination_observations.items()
        )
        or any(
            field in handshake or field in termination
            for field in ("guardian_pid", "process_group_id")
        )
        or any(
            not valid_sha256(value)
            for value in (
                package["bundle_receipt_sha256"],
                package["executable_sha256"],
                package["schema_registry_sha256"],
                session["runtime_lifecycle_binding_sha256"],
                handshake["receipt_sha256"],
                termination["receipt_sha256"],
            )
        )
    ):
        fail("reviewed-development package or lifecycle binding differs")
    operations = session["operations"]
    scenario = receipt["scenario"]
    transcript_scenario = transcript["scenario"]
    if scenario != {
        "subject_count": transcript_scenario["subject_count"],
        "subject_ids": transcript_scenario["subject_ids"],
        "observed_step_count": len(success["steps"]),
        "planned_step_count": success["planned_step_count"],
        "roster_authority": transcript_scenario["roster_authority"],
        "source_roster_authenticated": transcript_scenario[
            "source_roster_authenticated"
        ],
    }:
        fail("reviewed-development scenario lineage differs")
    expected_ids = [
        "prisoma.observer.prepare.v1",
        "prisoma.observer.observe.v1",
        "prisoma.observer.observe.v1",
        "prisoma.observer.finish.v1",
    ]
    expected_sources = [
        None,
        success["steps"][0]["receipt_sha256"],
        success["steps"][1]["receipt_sha256"],
        success["receipt_sha256"],
    ]
    expected_controls = [
        frame["envelope"]["body"]["control"]
        for frame in transcript["frames"]
        if frame["direction"] == "runtime-to-host"
        and frame["envelope"]["kind"] == "operation.response"
    ]
    if [row["operation_id"] for row in operations] != expected_ids:
        fail("reviewed-development operation order differs")
    if len(expected_controls) != len(operations):
        fail("reviewed-development expected response roster differs")
    for row, source_receipt_sha256, expected_control in zip(
        operations,
        expected_sources,
        expected_controls,
        strict=True,
    ):
        response = row["response"]
        expected_response_sha256 = digest_bytes(canonical(expected_control))
        if (
            row["expected_response_control_sha256"] != expected_response_sha256
            or row["live_response_control_sha256"] != expected_response_sha256
            or row["semantic_response_exact_match"] is not True
            or response != {key: expected_control[key] for key in response}
            or response["source_receipt_sha256"] != source_receipt_sha256
            or any(
                not valid_sha256(row[field])
                for field in (
                    "request_frame_sha256",
                    "response_frame_sha256",
                    "expected_response_control_sha256",
                    "live_response_control_sha256",
                )
            )
        ):
            fail("reviewed-development semantic operation evidence differs")
    if (
        operations[0]["response"]["step_index"] != 0
        or operations[1]["response"]["step_index"] != 1
        or operations[2]["response"]["step_index"] != 2
        or operations[3]["response"]["step_index"] != 2
        or operations[3]["response"]["terminal"] is not True
        or operations[3]["response"]["state_cleared"] is not True
        or any(row["response"]["terminal"] for row in operations[:-1])
        or any(row["response"]["state_cleared"] for row in operations[:-1])
    ):
        fail("reviewed-development observer lifecycle differs")


def check_crebain_matrix_evidence(
    receipt: dict[str, Any],
    schema: dict[str, Any],
    provenance: dict[str, Any],
) -> None:
    evidence_path = EVIDENCE / "crebain-real-nest-observer-matrix.json"
    schema_path = EVIDENCE / "crebain-real-nest-observer-matrix.schema.json"
    evidence_provenance = provenance.get("crebain_real_nest_observer_matrix")
    expected_provenance_keys = {
        "status",
        "path",
        "sha256",
        "schema_id",
        "schema_path",
        "schema_sha256",
        "receipt_sha256",
        "review_scope",
        "prisoma_revision",
        "crebain_source_revision",
        "crebain_publication_revision",
        "crebain_index_sha256",
        "engram_revision",
        "capture_count",
        "reviewed_development_only",
        "production_manager_execution",
        "publisher_authenticated",
        "observer_source_durable_evidence_verified",
        "external_validator_source_durable_evidence_verified",
        "filesystem_isolation_enforced",
        "agent_bridge_command",
        "music_authority",
        "ncp_authority",
        "physical_authority",
        "scientific_authority",
    }
    if (
        not isinstance(evidence_provenance, dict)
        or set(evidence_provenance) != expected_provenance_keys
        or evidence_provenance["status"] != "observed-read-only-review-v1"
        or evidence_provenance["path"] != str(evidence_path.relative_to(ROOT))
        or evidence_provenance["sha256"] != digest_file(evidence_path)
        or evidence_provenance["schema_id"] != CREBAIN_MATRIX_SCHEMA_ID
        or evidence_provenance["schema_path"] != str(schema_path.relative_to(ROOT))
        or evidence_provenance["schema_sha256"] != digest_file(schema_path)
        or evidence_provenance["receipt_sha256"] != receipt.get("receipt_sha256")
        or evidence_provenance["review_scope"] != receipt.get("review_scope")
        or evidence_provenance["capture_count"] != 3
        or evidence_provenance["reviewed_development_only"] is not True
        or evidence_provenance["external_validator_source_durable_evidence_verified"]
        is not True
        or any(
            evidence_provenance[field]
            for field in (
                "production_manager_execution",
                "publisher_authenticated",
                "observer_source_durable_evidence_verified",
                "filesystem_isolation_enforced",
                "agent_bridge_command",
                "music_authority",
                "ncp_authority",
                "physical_authority",
                "scientific_authority",
            )
        )
        or not schema_accepts(receipt, schema)
        or receipt.get("receipt_sha256") != digest_without(receipt, "receipt_sha256")
    ):
        fail("tracked CREBAIN observer matrix or provenance differs")

    sources = receipt.get("sources")
    captures = receipt.get("captures")
    assertions = receipt.get("assertions")
    authority = receipt.get("authority")
    if not all(
        isinstance(value, expected_type)
        for value, expected_type in (
            (sources, dict),
            (captures, list),
            (assertions, dict),
            (authority, dict),
        )
    ):
        fail("tracked CREBAIN observer matrix shape differs")

    prisoma_source = sources.get("prisoma_repository")
    crebain_source = sources.get("crebain_source_repository")
    publication = sources.get("crebain_evidence_publication")
    engram_source = sources.get("engram_repository")
    if not all(
        isinstance(value, dict)
        for value in (prisoma_source, crebain_source, publication, engram_source)
    ):
        fail("tracked CREBAIN observer matrix source closure differs")
    if (
        prisoma_source.get("commit") != evidence_provenance["prisoma_revision"]
        or crebain_source.get("commit")
        != evidence_provenance["crebain_source_revision"]
        or publication.get("commit")
        != evidence_provenance["crebain_publication_revision"]
        or publication.get("parent_commit")
        != evidence_provenance["crebain_source_revision"]
        or sources.get("index_exact_sha256")
        != evidence_provenance["crebain_index_sha256"]
        or engram_source.get("commit") != evidence_provenance["engram_revision"]
        or receipt.get("reviewed_development_only") is not True
        or receipt.get("production_manager_execution") is not False
        or not assertions
        or any(value is not True for value in assertions.values())
    ):
        fail("tracked CREBAIN observer matrix immutable pins differ")

    if len(captures) != 3 or [row.get("drone_count") for row in captures] != [1, 2, 3]:
        fail("tracked CREBAIN observer matrix drone roster differs")
    source_rosters = {row["source"]["engram_source_roster_sha256"] for row in captures}
    source_closures = {
        row["source"]["engram_source_closure_sha256"] for row in captures
    }
    receipt_stores = {row["capture"]["receipt_store_id"] for row in captures}
    terminal_receipts = {row["capture"]["terminal_receipt_sha256"] for row in captures}
    if (
        len(source_rosters) != 1
        or len(source_closures) != 3
        or len(receipt_stores) != 3
        or len(terminal_receipts) != 3
    ):
        fail("tracked CREBAIN observer matrix run closure differs")
    for drone_count, row in enumerate(captures, start=1):
        nest = row["nest"]
        observer = row["observer"]
        lifecycle = row["lifecycle"]
        row_authority = row["authority"]
        if (
            row["capture"]["receipt_store_file_count"] != 8
            or nest["population_count"] != drone_count * 6
            or nest["signed_population_count"] != nest["population_count"]
            or nest["source_durable_evidence_verified"] is not True
            or observer["source_durable_evidence_verified"] is not False
            or observer["state_cleared"] is not True
            or lifecycle["filesystem_isolation_enforced"] is not False
            or row_authority["descriptive_only"] is not True
            or row_authority["simulator_only"] is not True
            or any(
                row_authority[field]
                for field in (
                    "agent_bridge_command",
                    "calibrated_posterior",
                    "execution_authority",
                    "is_paper_local_evidence",
                    "music_used",
                    "ncp_qualified",
                    "ncp_used",
                    "physical_actuation",
                    "plant_control",
                    "scientific_authority",
                )
            )
        ):
            fail("tracked CREBAIN observer matrix authority boundary differs")
    if (
        authority["observer_role"] != "read-only-observer"
        or authority["descriptive_only"] is not True
        or any(
            authority[field]
            for field in (
                "agent_bridge_command",
                "calibrated_posterior",
                "durable_process_launch_authority",
                "execution_authority",
                "is_paper_local_evidence",
                "music_authority",
                "ncp_authority",
                "observer_source_durable_evidence_verified",
                "physical_authority",
                "plant_control",
                "publisher_authenticated",
                "replayable_live_launch_authority",
                "scientific_authority",
                "store_installation_authority",
            )
        )
    ):
        fail("tracked CREBAIN observer matrix top-level authority differs")


def main() -> None:
    for path, expected in LEGACY_FILES.items():
        if digest_file(path) != expected:
            fail(f"legacy Host API 1.1 bridge changed: {path}")
    manifest = load_json(INTEGRATION / "manifest.template.json")
    configuration = load_json(INTEGRATION / "configuration.json")
    provenance = load_json(CONTRACTS / "PROVENANCE.json")
    schemas: dict[str, dict[str, Any]] = {}
    for schema_id, filename in PROJECT_SCHEMA_FILES.items():
        schema = load_json(CONTRACTS / filename)
        validate_safe_project_schema(schema_id, schema)
        schemas[schema_id] = schema
    operational_schema = load_json(
        EVIDENCE / "engram-reviewed-development-e2e.schema.json"
    )
    validate_safe_project_schema(OPERATIONAL_EVIDENCE_SCHEMA_ID, operational_schema)
    nest_summary_schema = load_json(
        EVIDENCE / "engram-nest-evidence-validation-summary.schema.json"
    )
    validate_safe_project_schema(NEST_SUMMARY_SCHEMA_ID, nest_summary_schema)
    check_nest_summary_schema_controls(nest_summary_schema)
    matrix_schema = load_json(
        EVIDENCE / "crebain-real-nest-observer-matrix.schema.json"
    )
    validate_safe_project_schema(CREBAIN_MATRIX_SCHEMA_ID, matrix_schema)
    matrix_receipt = load_json(EVIDENCE / "crebain-real-nest-observer-matrix.json")
    check_crebain_matrix_evidence(matrix_receipt, matrix_schema, provenance)
    build_receipt_schema = load_json(
        EVIDENCE / "observer-release-build-receipt.schema.json"
    )
    validate_safe_project_schema(BUILD_RECEIPT_SCHEMA_ID, build_receipt_schema)
    stage_receipt_schema = load_json(
        EVIDENCE / "observer-package-stage-receipt.schema.json"
    )
    validate_safe_project_schema(STAGE_RECEIPT_SCHEMA_ID, stage_receipt_schema)
    if not schema_accepts(configuration, schemas["prisoma.observer.configuration.v1"]):
        fail("configuration fails its closed schema")
    if digest_file(CONTRACTS / "managed-runtime-ipc.schema.json") != (
        "e6950a2b3d1913ebacb82823afe648538ec789fe845ed3894b2122dd9864cfc1"
    ):
        fail("generic managed-runtime IPC copy differs")
    float_corpus_path = CONTRACTS / "engram.managed-runtime-finite-float.v1.json"
    float_corpus = load_json(float_corpus_path)
    corpus_provenance = provenance["finite_float_corpus"]
    randomized_corpus = float_corpus.get("randomized")
    if (
        corpus_provenance["sha256"] != digest_file(float_corpus_path)
        or corpus_provenance["case_count"] != 25
        or float_corpus.get("schema_version")
        != "engram.managed-runtime-finite-float.v1"
        or float_corpus.get("canonicalizer") != "engram.managed-runtime-json.v1"
        or not isinstance(float_corpus.get("cases"), list)
        or len(float_corpus["cases"]) != 25
        or len({row.get("id") for row in float_corpus["cases"]}) != 25
        or not isinstance(randomized_corpus, dict)
        or randomized_corpus
        != {
            "algorithm": "splitmix64-v1",
            "seed_hex": "656e6772616d7631",
            "sample_count": 4096,
            "accepted_count": 4030,
            "transcript": "lowercase-binary64-hex:canonical-json-or-rejected\\n",
            "transcript_sha256": (
                "4199b76e20a650518c52c7ead69bfc28c7333c73be0a607bc3a39d1a7994599c"
            ),
        }
        or corpus_provenance.get("randomized_sample_count") != 4096
        or corpus_provenance.get("randomized_accepted_count") != 4030
        or corpus_provenance.get("randomized_transcript_sha256")
        != randomized_corpus["transcript_sha256"]
    ):
        fail("generic managed-runtime finite-float corpus differs")
    operations = manifest["runtime"]["operations"]
    if [row["operation_id"] for row in operations] != sorted(
        row["operation_id"] for row in operations
    ):
        fail("manifest operation roster is not ordered")
    for operation in operations:
        if (
            operation["class"] != "observation"
            or operation["effect"] != "none"
            or operation["compute_grant"] != "none"
            or operation["max_cpu_time_ms"] != 0
            or operation["artifact_access"] != {"read": "none", "write": "none"}
        ):
            fail("manifest operation grants authority")
        for field in ("request_schema", "response_schema"):
            reference = operation[field]
            filename = PROJECT_SCHEMA_FILES[reference["schema_id"]]
            if reference["schema_sha256"] != digest_file(CONTRACTS / filename):
                fail("manifest schema digest differs")
    if (
        manifest["capabilities"]
        != ["operations.observation", "runtime.managed-headless"]
        or any(manifest["authority"].values())
        or manifest["ncp"]
        != {"mode": "none", "activation_enabled": False, "host_compatible": False}
    ):
        fail("manifest authority boundary differs")
    roster_digest = operation_roster_digest(operations)
    if provenance["operation_roster_sha256"] != roster_digest:
        fail("provenance operation roster digest differs")
    closed_loop_provenance = provenance["closed_loop_source"]
    fixture_source_roster = closed_loop_provenance.get("imported_source_roster")
    if not valid_imported_source_roster(fixture_source_roster):
        fail("fixture-generation imported source roster differs")
    fixture_source_by_path = {row["path"]: row for row in fixture_source_roster}
    closed_loop_path = "backend/optimization/extension_closed_loop.py"
    if (
        closed_loop_provenance.get("source_state") != "contained-in-recorded-revision"
        or not valid_git_oid(closed_loop_provenance.get("revision"))
        or closed_loop_provenance.get("imported_source_roster_sha256")
        != imported_source_roster_digest(fixture_source_roster)
        or closed_loop_path not in fixture_source_by_path
        or fixture_source_by_path[closed_loop_path]["sha256"]
        != closed_loop_provenance.get("sha256")
        or fixture_source_by_path[closed_loop_path]["git_blob"]
        != closed_loop_provenance.get("git_blob")
        or not valid_sha256(closed_loop_provenance.get("generator_source_sha256"))
    ):
        fail("fixture-generation source provenance differs")
    copied_contract = provenance["copied_contract"]
    ipc_source = provenance["ipc_source"]
    if (
        copied_contract.get("schema_id") != "engram.managed-runtime-ipc.v1"
        or copied_contract.get("sha256")
        != digest_file(CONTRACTS / "managed-runtime-ipc.schema.json")
        or ipc_source.get("revision") != closed_loop_provenance["revision"]
        or ipc_source.get("path")
        != "integrations/contracts/engram.managed-runtime-ipc.v1.schema.json"
        or ipc_source.get("source_state") != "contained-in-recorded-revision"
        or not valid_git_oid(ipc_source.get("git_blob"))
        or corpus_provenance.get("revision") != closed_loop_provenance["revision"]
        or corpus_provenance.get("source_path")
        != "integrations/contracts/engram.managed-runtime-finite-float.v1.json"
        or corpus_provenance.get("source_state") != "contained-in-recorded-revision"
        or not valid_git_oid(corpus_provenance.get("source_git_blob"))
    ):
        fail("copied Engram contract source provenance differs")
    fixture_rows = {row["path"]: row for row in provenance["generated_receipts"]}
    success_path = FIXTURES / "engram-run-receipt.generated.json"
    cleanup_failure_path = (
        FIXTURES / "engram-runtime-finished-neural-cleanup-failed.generated.json"
    )
    zero_path = FIXTURES / "engram-zero-step-run-receipt.generated.json"
    for path in (success_path, cleanup_failure_path, zero_path):
        relative = str(path.relative_to(ROOT))
        if fixture_rows[relative]["sha256"] != digest_file(path):
            fail(f"generated source receipt provenance differs: {relative}")
    success = load_json(success_path)
    cleanup_failure = load_json(cleanup_failure_path)
    zero = load_json(zero_path)
    check_source_receipt(success, 2)
    check_source_receipt(cleanup_failure, 2)
    check_source_receipt(zero, 0)
    if (
        success["neural_session_receipt_sha256"] is None
        or success["neural_durable_evidence_profile"]
        != "engram.nest-closed-loop-evidence-bundle.v2"
        or success["runtime_lifecycle"] is None
    ):
        fail("positive source receipt profile or runtime lineage differs")
    if (
        zero["status"] != "failed"
        or zero["steps"]
        or zero["runtime_lifecycle"] is not None
    ):
        fail("zero-step source receipt is not a failed terminal vector")
    if (
        cleanup_failure["status"] != "failed"
        or cleanup_failure["runtime_finish_sha256"] is None
        or cleanup_failure["cleanup_complete"] is not False
        or cleanup_failure["cleanup"][0]["mode"] != "finish"
        or cleanup_failure["cleanup"][1]["confirmed"] is not False
    ):
        fail("runtime-finished neural-cleanup-failure vector differs")
    transcript = load_json(INTEGRATION / "sample-transcript.json")
    if transcript["source_fixture_exact_sha256"] != digest_file(success_path):
        fail("sample transcript source fixture digest differs")
    check_transcript(transcript, schemas, roster_digest)
    finish_controls = [
        frame["envelope"]["body"]["control"]
        for frame in transcript["frames"]
        if frame["direction"] == "host-to-runtime"
        and frame["envelope"]["kind"] == "operation.request"
        and frame["envelope"]["body"]["operation"]["operation_id"]
        == "prisoma.observer.finish.v1"
    ]
    if (
        len(finish_controls) != 1
        or finish_controls[0]["neural_durable_evidence_profile"]
        != success["neural_durable_evidence_profile"]
        or finish_controls[0]["source_run_receipt_sha256"] != success["receipt_sha256"]
    ):
        fail("sample transcript terminal source profile lineage differs")
    operational_receipt = load_json(EVIDENCE / "engram-reviewed-development-e2e.json")
    current_operational = provenance.get("current_operational_evidence")
    if (
        isinstance(current_operational, dict)
        and current_operational.get("status") == "NOT RUN"
    ):
        check_historical_operational_evidence(
            operational_receipt,
            provenance,
        )
        operational_status = (
            "historical v1 audit is closed; reviewed-development v2 launch is NOT RUN"
        )
    else:
        check_operational_evidence(
            operational_receipt,
            operational_schema,
            provenance,
            success,
            transcript,
            roster_digest,
        )
        operational_status = "current reviewed-development v2 evidence is closed"
    if not math.isfinite(float(len(canonical(transcript)))):
        fail("sample transcript size is invalid")
    print(
        "OK: managed observer contracts, source receipts, and transcript are closed; "
        f"{operational_status}; tracked CREBAIN read-only matrix is closed"
    )


if __name__ == "__main__":
    main()
