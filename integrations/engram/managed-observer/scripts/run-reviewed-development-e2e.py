#!/usr/bin/env python3
"""Run one temporary-store Engram Host API 2 observer interoperability proof."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import shutil
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

from source_provenance import (
    capture_imported_source_roster,
    digest_regular_file,
    imported_source_roster_sha256,
    snapshot_regular_file,
    verify_imported_source_roster_unchanged,
    verify_repository_revision,
)


ROOT = Path(__file__).resolve().parents[4]
INTEGRATION = ROOT / "integrations" / "engram" / "managed-observer"
TRANSCRIPT = INTEGRATION / "sample-transcript.json"
SOURCE_RECEIPT = INTEGRATION / "fixtures" / "engram-run-receipt.generated.json"
EVIDENCE_SCHEMA = (
    INTEGRATION / "evidence" / "engram-reviewed-development-e2e.schema.json"
)
PROVENANCE = INTEGRATION / "contracts" / "PROVENANCE.json"
SCHEMA_VERSION = "prisoma.observer.engram-reviewed-development-e2e.v2"
TEMPORARY_PARENT = Path("/private/tmp")
MAX_LOCAL_JSON_BYTES = 4 * 1024 * 1024
MAX_INPUT_BUNDLE_BYTES = 512 * 1024 * 1024


def closed_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON member: {key}")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise ValueError(f"non-finite JSON constant: {value}")


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(
        snapshot_regular_file(path, MAX_LOCAL_JSON_BYTES),
        object_pairs_hook=closed_object,
        parse_constant=reject_constant,
    )
    if not isinstance(value, dict):
        raise ValueError(f"JSON root is not an object: {path}")
    return value


def canonical(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def pretty(value: Any) -> bytes:
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            allow_nan=False,
            indent=2,
            sort_keys=True,
        )
        + "\n"
    ).encode("utf-8")


def digest_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def digest_file(path: Path) -> str:
    return digest_regular_file(path, MAX_LOCAL_JSON_BYTES)


def operation_controls(transcript: dict[str, Any]) -> list[dict[str, Any]]:
    controls: list[dict[str, Any]] = []
    frames = transcript.get("frames")
    if not isinstance(frames, list):
        raise ValueError("sample transcript has no frame roster")
    expected_responses: dict[int, tuple[str, dict[str, Any]]] = {}
    consumed_response_sequences: set[int] = set()
    for row in frames:
        if not isinstance(row, dict) or row.get("direction") != "runtime-to-host":
            continue
        envelope = row.get("envelope")
        if (
            not isinstance(envelope, dict)
            or envelope.get("kind") != "operation.response"
        ):
            continue
        sequence = envelope.get("sequence")
        body = envelope.get("body")
        if (
            isinstance(sequence, bool)
            or not isinstance(sequence, int)
            or not isinstance(body, dict)
            or not isinstance(body.get("status"), str)
            or not isinstance(body.get("control"), dict)
            or sequence in expected_responses
        ):
            raise ValueError("sample operation response roster differs")
        expected_responses[sequence] = (body["status"], body["control"])
    for row in frames:
        if not isinstance(row, dict) or row.get("direction") != "host-to-runtime":
            continue
        envelope = row.get("envelope")
        if (
            not isinstance(envelope, dict)
            or envelope.get("kind") != "operation.request"
        ):
            continue
        body = envelope.get("body")
        if not isinstance(body, dict):
            raise ValueError("sample operation request has no body")
        operation = body.get("operation")
        control = body.get("control")
        sequence = envelope.get("sequence")
        if (
            not isinstance(operation, dict)
            or not isinstance(control, dict)
            or isinstance(sequence, bool)
            or not isinstance(sequence, int)
            or sequence not in expected_responses
            or sequence in consumed_response_sequences
        ):
            raise ValueError("sample operation request is incomplete")
        consumed_response_sequences.add(sequence)
        controls.append(
            {
                "operation_id": operation.get("operation_id"),
                "operation_class": operation.get("class"),
                "effect": operation.get("effect"),
                "artifact_access": operation.get("artifact_access"),
                "control": control,
                "expected_status": expected_responses[sequence][0],
                "expected_response": expected_responses[sequence][1],
            }
        )
    expected = [
        "prisoma.observer.prepare.v1",
        "prisoma.observer.observe.v1",
        "prisoma.observer.observe.v1",
        "prisoma.observer.finish.v1",
    ]
    if [row["operation_id"] for row in controls] != expected:
        raise ValueError("sample operation roster differs")
    if consumed_response_sequences != set(expected_responses):
        raise ValueError("sample operation response roster is not one-to-one")
    return controls


def operation_summary(
    binding: Any, result: Any, expected_response: dict[str, Any]
) -> dict[str, Any]:
    control = result.control
    return {
        "operation_id": binding.operation_id,
        "operation_class": binding.operation_class,
        "compute_grant": binding.compute_grant,
        "max_cpu_time_ms": binding.max_cpu_time_ms,
        "status": result.status,
        "request_frame_sha256": result.request_frame_sha256,
        "response_frame_sha256": result.response_frame_sha256,
        "expected_response_control_sha256": digest_bytes(canonical(expected_response)),
        "live_response_control_sha256": digest_bytes(canonical(control)),
        "semantic_response_exact_match": True,
        "response": {
            "study_run_id": control["study_run_id"],
            "step_index": control["step_index"],
            "source_receipt_sha256": control["source_receipt_sha256"],
            "observer_receipt_sha256": control["observer_receipt_sha256"],
            "observer_transcript_sha256": control["observer_transcript_sha256"],
            "terminal": control["terminal"],
            "state_cleared": control["state_cleared"],
            "authority": control["authority"],
            "roster_authority": control["roster_authority"],
            "source_roster_authenticated": control["source_roster_authenticated"],
            "descriptive_only": control["descriptive_only"],
            "agent_bridge_command": control["agent_bridge_command"],
            "physical_actuation": control["physical_actuation"],
            "ncp_used": control["ncp_used"],
            "pid_result": control["pid_result"],
            "source_durable_evidence_verified": control[
                "source_durable_evidence_verified"
            ],
            "scientific_authority": control["scientific_authority"],
            "is_paper_local_evidence": control["is_paper_local_evidence"],
            "calibrated_posterior": control["calibrated_posterior"],
        },
    }


def scenario_summary(
    controls: list[dict[str, Any]],
    source_receipt: dict[str, Any],
) -> dict[str, Any]:
    """Rejoin the bounded scenario description to exact request and source bytes."""

    prepare = controls[0]["control"]
    subject_ids = prepare.get("subject_ids")
    channel_ids = prepare.get("channel_ids")
    planned_step_count = prepare.get("planned_step_count")
    source_steps = source_receipt.get("steps")
    observed_controls = [
        row for row in controls if row["operation_id"] == "prisoma.observer.observe.v1"
    ]
    finish_controls = [
        row for row in controls if row["operation_id"] == "prisoma.observer.finish.v1"
    ]
    identity_fields = (
        "study_run_id",
        "study_definition_sha256",
        "closed_loop_definition_sha256",
        "runtime_binding_sha256",
        "runtime_adapter_configuration_sha256",
        "neural_provider_identity_sha256",
        "planned_step_count",
    )
    step_fields = (
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
    )
    observed_steps_match = (
        isinstance(source_steps, list)
        and len(observed_controls) == len(source_steps)
        and all(
            isinstance(source_step, dict)
            and all(
                control["control"].get(field) == source_step.get(field)
                for field in step_fields
            )
            and control["control"].get("source_receipt_sha256")
            == source_step.get("receipt_sha256")
            for control, source_step in zip(
                observed_controls,
                source_steps,
                strict=True,
            )
        )
    )
    finish = finish_controls[0]["control"] if len(finish_controls) == 1 else {}
    if (
        not isinstance(subject_ids, list)
        or not subject_ids
        or not all(isinstance(value, str) for value in subject_ids)
        or not isinstance(channel_ids, list)
        or len(channel_ids) != len(subject_ids)
        or isinstance(planned_step_count, bool)
        or not isinstance(planned_step_count, int)
        or not isinstance(source_steps, list)
        or any(
            prepare.get(field) != source_receipt.get(field) for field in identity_fields
        )
        or len(source_steps) != len(observed_controls)
        or len(source_steps) != planned_step_count
        or [row["control"].get("step_index") for row in observed_controls]
        != list(range(1, len(observed_controls) + 1))
        or [row.get("step_index") for row in source_steps]
        != list(range(1, len(source_steps) + 1))
        or not observed_steps_match
        or finish.get("study_run_id") != source_receipt.get("study_run_id")
        or finish.get("planned_step_count") != planned_step_count
        or finish.get("step_count") != len(source_steps)
        or finish.get("source_run_receipt_sha256")
        != source_receipt.get("receipt_sha256")
        or finish.get("neural_durable_evidence_profile")
        != source_receipt.get("neural_durable_evidence_profile")
        or source_receipt.get("neural_durable_evidence_profile")
        != "engram.nest-closed-loop-evidence-bundle.v2"
    ):
        raise ValueError("sample scenario differs from its source receipt")
    return {
        "subject_count": len(subject_ids),
        "subject_ids": list(subject_ids),
        "observed_step_count": len(observed_controls),
        "planned_step_count": planned_step_count,
        "roster_authority": "host-declared-projection",
        "source_roster_authenticated": False,
    }


def validate_response_authority(control: dict[str, Any]) -> None:
    if (
        control.get("authority") != "read-only-observer"
        or control.get("roster_authority") != "host-declared-projection"
        or control.get("source_roster_authenticated") is not False
        or control.get("descriptive_only") is not True
        or any(
            control.get(field) is not False
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
        raise ValueError("live observer response exceeded its authority boundary")


def remove_private_tree(root: Path) -> None:
    """Remove only this script's exact owner-private temporary tree."""

    if (
        root.parent != TEMPORARY_PARENT
        or not root.name.startswith("prisoma-observer-engram-e2e-")
        or root.is_symlink()
    ):
        raise ValueError("temporary cleanup target differs")
    for current, directories, _files in os.walk(root, topdown=False):
        current_path = Path(current)
        for child in directories:
            child_path = current_path / child
            if child_path.is_symlink():
                raise ValueError("temporary cleanup contains a directory symlink")
            os.chmod(child_path, 0o700)
        os.chmod(current_path, 0o700)
    shutil.rmtree(root)


def write_new_receipt(path: Path, payload: bytes) -> None:
    flags = (
        os.O_WRONLY
        | os.O_CREAT
        | os.O_EXCL
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    descriptor = os.open(path, flags, 0o600)
    created = True
    try:
        view = memoryview(payload)
        while view:
            written = os.write(descriptor, view)
            if written <= 0:
                raise OSError("operational receipt write made no progress")
            view = view[written:]
        os.fsync(descriptor)
    except BaseException:
        os.close(descriptor)
        descriptor = -1
        if created:
            path.unlink(missing_ok=True)
        raise
    finally:
        if descriptor >= 0:
            os.close(descriptor)


def parser() -> argparse.ArgumentParser:
    command = argparse.ArgumentParser(description=__doc__)
    command.add_argument("--engram-root", required=True, type=Path)
    command.add_argument("--expected-engram-revision", required=True)
    command.add_argument("--bundle", required=True, type=Path)
    command.add_argument("--output", required=True, type=Path)
    command.add_argument("--provenance-output", required=True, type=Path)
    return command


def main() -> int:
    args = parser().parse_args()
    engram_root = args.engram_root.resolve(strict=True)
    bundle = args.bundle.resolve(strict=True)
    output = Path(os.path.abspath(args.output))
    provenance_output = Path(os.path.abspath(args.provenance_output))
    if (
        output == provenance_output
        or output.exists()
        or output.is_symlink()
        or provenance_output.exists()
        or provenance_output.is_symlink()
    ):
        raise ValueError("operational receipt output path differs or already exists")
    engram_revision = verify_repository_revision(
        engram_root,
        args.expected_engram_revision,
    )
    input_bundle_exact_sha256 = digest_regular_file(bundle, MAX_INPUT_BUNDLE_BYTES)
    source_paths = {
        "closed_loop_source_sha256": (
            engram_root / "backend" / "optimization" / "extension_closed_loop.py"
        ),
        "development_launcher_source_sha256": (
            engram_root
            / "backend"
            / "integrations"
            / "reviewed_native_development_session.py"
        ),
        "package_store_source_sha256": (
            engram_root / "backend" / "integrations" / "extension_package_store.py"
        ),
        "standard_simulator_source_sha256": (
            engram_root
            / "backend"
            / "integrations"
            / "standard_closed_loop_simulator.py"
        ),
    }
    local_input_hashes = {
        "sample_transcript_exact_sha256": digest_file(TRANSCRIPT),
        "source_fixture_exact_sha256": digest_file(SOURCE_RECEIPT),
    }
    evidence_schema_exact_sha256 = digest_file(EVIDENCE_SCHEMA)
    engram_root_text = os.fspath(engram_root)
    sys.dont_write_bytecode = True
    sys.path[:] = [entry for entry in sys.path if entry != engram_root_text]
    sys.path.insert(0, engram_root_text)
    package_store_module = importlib.import_module(
        "backend.integrations.extension_package_store"
    )
    launcher_module = importlib.import_module(
        "backend.integrations.reviewed_native_development_session"
    )
    closed_loop_module = importlib.import_module(
        "backend.optimization.extension_closed_loop"
    )
    standard_simulator_module = importlib.import_module(
        "backend.integrations.standard_closed_loop_simulator"
    )
    imported_sources = {
        "closed_loop_source_sha256": closed_loop_module,
        "development_launcher_source_sha256": launcher_module,
        "package_store_source_sha256": package_store_module,
        "standard_simulator_source_sha256": standard_simulator_module,
    }
    for source_name, module in imported_sources.items():
        module_file = getattr(module, "__file__", None)
        if (
            not isinstance(module_file, str)
            or Path(module_file).resolve(strict=True) != source_paths[source_name]
        ):
            raise ValueError(f"Engram import source differs: {source_name}")
    source_roster = capture_imported_source_roster(
        engram_root,
        args.expected_engram_revision,
    )
    source_roster_sha256 = imported_source_roster_sha256(source_roster)
    ExtensionPackageStore = package_store_module.ExtensionPackageStore
    ExtensionPackageStoreError = package_store_module.ExtensionPackageStoreError
    ReviewedNativeDevelopmentSession = launcher_module.ReviewedNativeDevelopmentSession
    build_runtime_lifecycle_binding = closed_loop_module.build_runtime_lifecycle_binding
    from jsonschema import Draft202012Validator  # noqa: PLC0415

    transcript = load_json(TRANSCRIPT)
    source_receipt = load_json(SOURCE_RECEIPT)
    evidence_schema = load_json(EVIDENCE_SCHEMA)
    controls = operation_controls(transcript)
    scenario = scenario_summary(controls, source_receipt)
    run_root = Path(
        tempfile.mkdtemp(prefix="prisoma-observer-engram-e2e-", dir=TEMPORARY_PARENT)
    )
    os.chmod(run_root, 0o700)
    store: ExtensionPackageStore | None = None
    installed: Any = None
    generation_id: str | None = None
    store_id: str | None = None
    session: ReviewedNativeDevelopmentSession | None = None
    handshake: Any = None
    termination: Any = None
    lifecycle: Any = None
    removal_rejection_reason: str | None = None
    removal: dict[str, Any] | None = None
    operation_results: list[dict[str, Any]] = []
    binding: Any = None
    try:
        store = ExtensionPackageStore(run_root / "store", repo_root=engram_root)
        installed = store.install(bundle)
        generation_id = str(installed.package["generation_id"])
        store_id = str(installed.installation_observation["store_id"])
        session = ReviewedNativeDevelopmentSession.launch_from_store(
            store,
            generation_id,
        )
        binding = session.binding
        handshake = session.handshake_receipt
        operations = {item.operation_id: item for item in binding.operations}
        if set(operations) != {
            "prisoma.observer.prepare.v1",
            "prisoma.observer.observe.v1",
            "prisoma.observer.finish.v1",
        }:
            raise ValueError("launched observer operation roster differs")
        if any(
            item.operation_class != "observation"
            or item.compute_grant != "none"
            or item.max_cpu_time_ms != 0
            for item in operations.values()
        ):
            raise ValueError("launched observer operation authority differs")
        try:
            store.remove(generation_id)
        except ExtensionPackageStoreError as error:
            removal_rejection_reason = error.reason
        else:
            raise ValueError("store removed a generation with a retained live lease")
        if removal_rejection_reason != "store.live-lease":
            raise ValueError("live-lease removal failed for an unexpected reason")

        for index, row in enumerate(controls, start=1):
            operation = operations[row["operation_id"]]
            if (
                row["operation_class"] != operation.operation_class
                or row["effect"] != "none"
                or row["artifact_access"] != {"read": "none", "write": "none"}
            ):
                raise ValueError("sample operation authority differs from live binding")
            result = session.operation_channel.invoke(
                operation,
                row["control"],
                idempotency_material=f"prisoma-observer-e2e-{index}",
                deadline_ns=time.monotonic_ns() + operation.timeout_ms * 1_000_000,
            )
            if result.status != row["expected_status"] or result.status != "succeeded":
                raise ValueError(
                    f"live operation {operation.operation_id} did not succeed"
                )
            if result.control != row["expected_response"]:
                raise ValueError(
                    f"live operation {operation.operation_id} response differs from transcript"
                )
            validate_response_authority(result.control)
            operation_results.append(
                operation_summary(operation, result, row["expected_response"])
            )

        if (
            not operation_results[-1]["response"]["terminal"]
            or not operation_results[-1]["response"]["state_cleared"]
        ):
            raise ValueError("live finish response did not clear observer state")
        session.operation_channel.close()
        if not session.confirm_generation_exit(time.monotonic_ns() + 5_000_000_000):
            raise ValueError("reviewed development child did not exit cleanly")
        termination = session.termination_receipt
        if termination is None:
            raise ValueError("reviewed development termination receipt is absent")
        lifecycle = build_runtime_lifecycle_binding(
            profile=handshake.profile,
            generation_id=handshake.generation_id,
            launch_source=handshake.launch_source,
            store_id=handshake.store_id,
            package_generation_id=handshake.package_generation_id,
            generation_directory_identity_sha256=(
                handshake.generation_directory_identity_sha256
            ),
            package_generation_lease_retained_at_launch=(
                handshake.package_generation_lease_retained
            ),
            package_generation_lease_released=(
                termination.package_generation_lease_released
            ),
            handshake_receipt_sha256=handshake.receipt_sha256,
            termination_receipt_sha256=termination.receipt_sha256,
            termination_disposition=termination.disposition,
            child_reaped=termination.child_reaped,
            containment_empty=termination.containment_empty,
            diagnostic_stream_complete=termination.diagnostic_stream_complete,
            private_work_directory_removed=termination.private_work_directory_removed,
            publisher_authenticated=False,
            durable_process_launch_authority=False,
            ncp_authority=False,
            physical_authority=False,
            scientific_authority=False,
        )
        removal = dict(store.remove(generation_id))
        if store.list():
            raise ValueError("temporary package store retained an active generation")
    finally:
        try:
            if session is not None:
                session.close()
        finally:
            remove_private_tree(run_root)
    if run_root.exists():
        raise ValueError("temporary package store cleanup is incomplete")
    verify_imported_source_roster_unchanged(
        engram_root,
        args.expected_engram_revision,
        source_roster,
    )
    verify_repository_revision(engram_root, args.expected_engram_revision)
    if input_bundle_exact_sha256 != digest_regular_file(
        bundle,
        MAX_INPUT_BUNDLE_BYTES,
    ):
        raise ValueError("input bundle changed during the interoperability run")
    if local_input_hashes != {
        "sample_transcript_exact_sha256": digest_file(TRANSCRIPT),
        "source_fixture_exact_sha256": digest_file(SOURCE_RECEIPT),
    }:
        raise ValueError("Prisoma interoperability input changed during the run")
    if evidence_schema_exact_sha256 != digest_file(EVIDENCE_SCHEMA):
        raise ValueError("operational evidence schema changed during the run")
    if None in (
        handshake,
        termination,
        lifecycle,
        removal,
        binding,
        installed,
        generation_id,
        store_id,
    ):
        raise ValueError("operational receipt inputs are incomplete")

    receipt: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "reviewed_development_only": True,
        "production_manager_execution": False,
        "real_child_process_executed": True,
        "source": {
            "engram_revision": engram_revision,
            "engram_imported_source_roster_sha256": source_roster_sha256,
            "engram_imported_source_roster": source_roster,
            "engram_loaded_source_bytes_attested": False,
            **local_input_hashes,
            "input_bundle_exact_sha256": input_bundle_exact_sha256,
            "source_run_receipt_sha256": source_receipt["receipt_sha256"],
        },
        "package": {
            "extension_id": binding.extension_id,
            "extension_version": binding.extension_version,
            "target_id": binding.target_id,
            "profile": binding.profile,
            "installation_id": binding.installation_id,
            "package_generation_id": generation_id,
            "bundle_receipt_sha256": installed.package["bundle_receipt_sha256"],
            "executable_sha256": binding.executable_sha256,
            "operation_roster_sha256": binding.operation_roster_sha256,
            "schema_registry_sha256": binding.schema_registry_sha256,
            "installation_disposition": installed.disposition,
            "publisher_authentication": "publisher-unattested",
        },
        "store": {
            "store_id": store_id,
            "installation_observation_sha256": digest_bytes(
                canonical(installed.installation_observation)
            ),
            "live_lease_removal_rejected": True,
            "live_lease_removal_reason": removal_rejection_reason,
            "post_reap_removal_disposition": removal["disposition"],
            "post_reap_removal_recoverable": removal["recoverable"],
            "active_generation_count_after_removal": 0,
            "temporary_store_retained": False,
            "temporary_store_generation_bytes_retained": False,
            "input_bundle_retained_by_harness": True,
        },
        "session": {
            "generation_id": binding.generation_id,
            "generation_ordinal": binding.generation_ordinal,
            "handshake": {
                "receipt_sha256": handshake.receipt_sha256,
                "launch_source": handshake.launch_source,
                "store_id": handshake.store_id,
                "package_generation_id": handshake.package_generation_id,
                "package_generation_lease_retained": (
                    handshake.package_generation_lease_retained
                ),
                "process_launch_performed": handshake.process_launch_performed,
                "explicit_absolute_path_spawn": (
                    handshake.explicit_absolute_path_spawn
                ),
                "path_lookup_at_spawn": handshake.path_lookup_at_spawn,
                "package_path_reopened_for_spawn": (
                    handshake.package_path_reopened_for_spawn
                ),
                "verified_executable_staged": handshake.verified_executable_staged,
                "staged_executable_owner_private": (
                    handshake.staged_executable_owner_private
                ),
                "staged_executable_user_immutable": (
                    handshake.staged_executable_user_immutable
                ),
                "process_group_containment": handshake.process_group_containment,
                "guardian_owner_loss_seal": handshake.guardian_owner_loss_seal,
                "guardian_generation_lease_retained": (
                    handshake.guardian_generation_lease_retained
                ),
                "guardian_uncertainty_record_prepared": (
                    handshake.guardian_uncertainty_record_prepared
                ),
                "descendant_creation_denied": handshake.descendant_creation_denied,
                "os_sandbox_enforced": handshake.os_sandbox_enforced,
                "network_isolation_enforced": handshake.network_isolation_enforced,
                "filesystem_isolation_enforced": handshake.filesystem_isolation_enforced,
                "external_dependency_closure_attested": (
                    handshake.external_dependency_closure_attested
                ),
                "automatic_restart": handshake.automatic_restart,
                "publisher_authenticated": handshake.publisher_authenticated,
                "durable_process_launch_authority": (
                    handshake.durable_process_launch_authority
                ),
                "replayable_live_launch_authority": (
                    handshake.replayable_live_launch_authority
                ),
                "ncp_authority": handshake.ncp_authority,
                "physical_authority": handshake.physical_authority,
                "scientific_authority": handshake.scientific_authority,
            },
            "operations": operation_results,
            "termination": {
                "receipt_sha256": termination.receipt_sha256,
                "handshake_receipt_sha256": termination.handshake_receipt_sha256,
                "disposition": termination.disposition,
                "reason_code": termination.reason_code,
                "exit_code": termination.exit_code,
                "termination_signal": termination.termination_signal,
                "child_reaped": termination.child_reaped,
                "guardian_reaped": termination.guardian_reaped,
                "group_signal_while_guardian_unreaped": (
                    termination.group_signal_while_guardian_unreaped
                ),
                "direct_child_signal_while_unreaped": (
                    termination.direct_child_signal_while_unreaped
                ),
                "containment_signal_scope": termination.containment_signal_scope,
                "containment_seal_signal": termination.containment_seal_signal,
                "containment_empty": termination.containment_empty,
                "diagnostic_stream_complete": termination.diagnostic_stream_complete,
                "private_work_directory_removed": (
                    termination.private_work_directory_removed
                ),
                "package_generation_lease_released": (
                    termination.package_generation_lease_released
                ),
                "guardian_generation_lease_held_until_containment": (
                    termination.guardian_generation_lease_held_until_containment
                ),
                "durable_process_launch_authority": (
                    termination.durable_process_launch_authority
                ),
                "ncp_authority": termination.ncp_authority,
                "physical_authority": termination.physical_authority,
                "scientific_authority": termination.scientific_authority,
            },
            "runtime_lifecycle_binding_sha256": lifecycle.binding_sha256,
            "clean_exit_confirmed": True,
        },
        "scenario": scenario,
        "authority": {
            "operation_class": "observation",
            "compute_grant": "none",
            "descriptive_only": True,
            "agent_bridge_command": False,
            "execution_authority": False,
            "store_installation_authority": False,
            "publisher_authenticated": False,
            "durable_process_launch_authority": False,
            "ncp_authority": False,
            "physical_authority": False,
            "source_durable_evidence_verified": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
    }
    receipt["receipt_sha256"] = digest_bytes(canonical(receipt))
    if receipt["receipt_sha256"] != digest_bytes(
        canonical(
            {key: value for key, value in receipt.items() if key != "receipt_sha256"}
        )
    ):
        raise ValueError("operational receipt self-digest differs")
    Draft202012Validator.check_schema(evidence_schema)
    validation_errors = sorted(
        Draft202012Validator(evidence_schema).iter_errors(receipt),
        key=lambda error: tuple(str(part) for part in error.absolute_path),
    )
    if validation_errors:
        first = validation_errors[0]
        location = "/".join(str(part) for part in first.absolute_path) or "<root>"
        raise ValueError(f"operational receipt fails schema at {location}")
    receipt_payload = canonical(receipt) + b"\n"
    provenance = load_json(PROVENANCE)
    provenance["current_operational_evidence"] = {
        "status": "observed-reviewed-development-v2",
        "schema_id": SCHEMA_VERSION,
        "path": (
            "integrations/engram/managed-observer/evidence/"
            "engram-reviewed-development-e2e.json"
        ),
        "schema_path": (
            "integrations/engram/managed-observer/evidence/"
            "engram-reviewed-development-e2e.schema.json"
        ),
        "sha256": digest_bytes(receipt_payload),
        "schema_sha256": evidence_schema_exact_sha256,
        "receipt_sha256": receipt["receipt_sha256"],
        "profile": "engram.reviewed-native-development.v1",
        "engram_revision": engram_revision,
        "engram_imported_source_roster_sha256": source_roster_sha256,
        "engram_loaded_source_bytes_attested": False,
        "input_bundle_exact_sha256": input_bundle_exact_sha256,
        "reviewed_development_only": True,
        "production_manager_execution": False,
        "publisher_authenticated": False,
        "ncp_authority": False,
        "physical_authority": False,
        "source_durable_evidence_verified": False,
        "scientific_authority": False,
    }
    provenance_payload = pretty(provenance)
    output.parent.mkdir(parents=True, exist_ok=True)
    provenance_output.parent.mkdir(parents=True, exist_ok=True)
    output_written = False
    provenance_output_written = False
    try:
        write_new_receipt(output, receipt_payload)
        output_written = True
        write_new_receipt(provenance_output, provenance_payload)
        provenance_output_written = True
    except BaseException:
        if output_written:
            output.unlink(missing_ok=True)
        if provenance_output_written:
            provenance_output.unlink(missing_ok=True)
        raise
    print(
        "OK: wrote reviewed development E2E receipt and provenance candidate "
        f"to {output} and {provenance_output}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
