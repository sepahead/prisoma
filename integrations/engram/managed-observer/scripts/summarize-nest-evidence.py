#!/usr/bin/env python3
"""Validate a full Engram NEST bundle externally and emit a bounded summary."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import sys
from pathlib import Path
from typing import Any

from source_provenance import (
    canonical,
    capture_repository_files,
    capture_repository_identity,
    capture_imported_source_roster,
    digest_regular_file,
    imported_source_roster_sha256,
    snapshot_regular_file,
    verify_imported_source_roster_unchanged,
    verify_repository_revision,
)


ROOT = Path(__file__).resolve().parents[4]
INTEGRATION = ROOT / "integrations" / "engram" / "managed-observer"
SUMMARY_SCHEMA = (
    INTEGRATION / "evidence" / "engram-nest-evidence-validation-summary.schema.json"
)
SCHEMA_VERSION = "prisoma.observer.engram-nest-evidence-validation-summary.v1"
MAX_RUN_RECEIPT_BYTES = 16 * 1024 * 1024
MAX_EVIDENCE_BUNDLE_BYTES = 240 * 1024 * 1024
MAX_SCHEMA_BYTES = 1024 * 1024
MAX_SUMMARY_BYTES = 2 * 1024 * 1024
VALIDATOR_SOURCE_PATHS = tuple(
    sorted(
        (
            Path("integrations/engram/managed-observer/scripts/source_provenance.py"),
            Path(
                "integrations/engram/managed-observer/scripts/"
                "summarize-nest-evidence.py"
            ),
            Path(
                "integrations/engram/managed-observer/evidence/"
                "engram-nest-evidence-validation-summary.schema.json"
            ),
        ),
        key=lambda path: path.as_posix(),
    )
)


def closed_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON member: {key}")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise ValueError(f"non-finite JSON constant: {value}")


def load_json_payload(payload: bytes, label: str) -> dict[str, Any]:
    value = json.loads(
        payload,
        object_pairs_hook=closed_object,
        parse_constant=reject_constant,
    )
    if not isinstance(value, dict):
        raise ValueError(f"{label} JSON root is not an object")
    return value


def digest_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def write_new_receipt(path: Path, payload: bytes) -> None:
    flags = (
        os.O_WRONLY
        | os.O_CREAT
        | os.O_EXCL
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
    )
    descriptor = os.open(path, flags, 0o600)
    try:
        view = memoryview(payload)
        while view:
            written = os.write(descriptor, view)
            if written <= 0:
                raise OSError("validation summary write made no progress")
            view = view[written:]
        os.fsync(descriptor)
    except BaseException:
        os.close(descriptor)
        descriptor = -1
        path.unlink(missing_ok=True)
        raise
    finally:
        if descriptor >= 0:
            os.close(descriptor)


def parser() -> argparse.ArgumentParser:
    command = argparse.ArgumentParser(description=__doc__)
    command.add_argument("--expected-prisoma-revision", required=True)
    command.add_argument("--engram-root", required=True, type=Path)
    command.add_argument("--expected-engram-revision", required=True)
    command.add_argument("--run-receipt", required=True, type=Path)
    command.add_argument("--evidence-bundle", required=True, type=Path)
    command.add_argument("--output", required=True, type=Path)
    command.add_argument("--verify", action="store_true")
    return command


def main() -> int:
    args = parser().parse_args()
    prisoma_root = ROOT.resolve(strict=True)
    if ROOT.absolute() != prisoma_root:
        raise ValueError("Prisoma repository path traverses a link")
    prisoma_identity = capture_repository_identity(
        prisoma_root,
        args.expected_prisoma_revision,
    )
    validator_sources = capture_repository_files(
        prisoma_root,
        args.expected_prisoma_revision,
        VALIDATOR_SOURCE_PATHS,
        MAX_SCHEMA_BYTES,
    )
    validator_source_roster_sha256 = digest_bytes(
        b"prisoma-nest-summary-validator-source-roster-v1\0"
        + canonical(validator_sources)
    )
    engram_root = Path(os.path.abspath(args.engram_root))
    if engram_root.resolve(strict=True) != engram_root:
        raise ValueError("Engram repository path traverses a link")
    run_receipt_path = Path(os.path.abspath(args.run_receipt))
    evidence_bundle_path = Path(os.path.abspath(args.evidence_bundle))
    if (
        run_receipt_path.parent.resolve(strict=True) != run_receipt_path.parent
        or evidence_bundle_path.parent.resolve(strict=True)
        != evidence_bundle_path.parent
    ):
        raise ValueError("NEST validation input parent traverses a link")
    output = Path(os.path.abspath(args.output))
    if output.parent.exists() and output.parent.resolve(strict=True) != output.parent:
        raise ValueError("validation summary output parent traverses a link")
    if not args.verify and (output.exists() or output.is_symlink()):
        raise ValueError("validation summary output already exists")
    engram_identity = capture_repository_identity(
        engram_root,
        args.expected_engram_revision,
    )
    engram_revision = engram_identity["commit"]
    summary_schema_payload = snapshot_regular_file(
        SUMMARY_SCHEMA,
        MAX_SCHEMA_BYTES,
    )
    summary_schema_exact_sha256 = digest_bytes(summary_schema_payload)
    run_payload = snapshot_regular_file(
        run_receipt_path,
        MAX_RUN_RECEIPT_BYTES,
    )
    bundle_payload = snapshot_regular_file(
        evidence_bundle_path,
        MAX_EVIDENCE_BUNDLE_BYTES,
    )
    run_exact_sha256 = digest_bytes(run_payload)
    bundle_exact_sha256 = digest_bytes(bundle_payload)
    run_value = load_json_payload(run_payload, "run receipt")
    bundle_value = load_json_payload(bundle_payload, "NEST evidence bundle")

    engram_root_text = os.fspath(engram_root)
    sys.dont_write_bytecode = True
    sys.path[:] = [entry for entry in sys.path if entry != engram_root_text]
    sys.path.insert(0, engram_root_text)
    closed_loop_module = importlib.import_module(
        "backend.optimization.extension_closed_loop"
    )
    evidence_module = importlib.import_module(
        "backend.optimization.extension_closed_loop_nest_evidence"
    )
    expected_modules = {
        engram_root / "backend" / "optimization" / "extension_closed_loop.py": (
            closed_loop_module
        ),
        engram_root
        / "backend"
        / "optimization"
        / "extension_closed_loop_nest_evidence.py": evidence_module,
    }
    for expected_path, module in expected_modules.items():
        module_file = getattr(module, "__file__", None)
        if not isinstance(module_file, str) or Path(module_file).resolve(
            strict=True
        ) != expected_path.resolve(strict=True):
            raise ValueError("Engram exact validator import source differs")
    if (
        getattr(evidence_module, "NEST_EVIDENCE_ADMISSION_MAX_BYTES", None)
        != MAX_EVIDENCE_BUNDLE_BYTES
    ):
        raise ValueError("Engram NEST evidence admission bound differs")
    source_roster = capture_imported_source_roster(
        engram_root,
        args.expected_engram_revision,
    )
    ClosedLoopRunReceiptV2 = closed_loop_module.ClosedLoopRunReceiptV2
    NestClosedLoopEvidenceBundleV2 = evidence_module.NestClosedLoopEvidenceBundleV2
    validate_nest_evidence_against_run = (
        evidence_module.validate_nest_evidence_against_run
    )
    run = ClosedLoopRunReceiptV2.model_validate(run_value)
    bundle = NestClosedLoopEvidenceBundleV2.model_validate(bundle_value)
    validate_nest_evidence_against_run(bundle, run)

    verify_imported_source_roster_unchanged(
        engram_root,
        args.expected_engram_revision,
        source_roster,
    )
    verify_repository_revision(engram_root, args.expected_engram_revision)
    if run_exact_sha256 != digest_regular_file(
        run_receipt_path,
        MAX_RUN_RECEIPT_BYTES,
    ) or bundle_exact_sha256 != digest_regular_file(
        evidence_bundle_path,
        MAX_EVIDENCE_BUNDLE_BYTES,
    ):
        raise ValueError("NEST validation input changed during exact rejoin")
    if summary_schema_exact_sha256 != digest_regular_file(
        SUMMARY_SCHEMA,
        MAX_SCHEMA_BYTES,
    ):
        raise ValueError("validation summary schema changed during exact rejoin")

    tail = bundle.tail_disposition_receipt
    lifecycle = bundle.worker_lifecycle_receipt
    neural_cleanups = [item for item in run.cleanup if item.component == "neural"]
    if len(neural_cleanups) != 1:
        raise ValueError("validated run has no unique neural cleanup receipt")
    neural_cleanup = neural_cleanups[0]
    validation_input = {
        "prisoma_repository": prisoma_identity,
        "prisoma_validator_source_roster_sha256": validator_source_roster_sha256,
        "engram_repository": engram_identity,
        "engram_imported_source_roster_sha256": imported_source_roster_sha256(
            source_roster
        ),
        "summary_schema_exact_sha256": summary_schema_exact_sha256,
        "run_receipt_exact_sha256": run_exact_sha256,
        "evidence_bundle_exact_sha256": bundle_exact_sha256,
        "source_run_receipt_sha256": run.receipt_sha256,
        "source_bundle_sha256": bundle.bundle_sha256,
    }
    validation_input_sha256 = digest_bytes(
        b"prisoma-engram-nest-validation-input-v1\0" + canonical(validation_input)
    )
    summary: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "validation_scope": "engram-exact-validator-rejoin-only",
        "prisoma_repository": prisoma_identity,
        "prisoma_validator_source_roster_sha256": validator_source_roster_sha256,
        "prisoma_validator_source_roster": validator_sources,
        "engram_repository": engram_identity,
        "engram_revision": engram_revision,
        "engram_imported_source_roster_sha256": imported_source_roster_sha256(
            source_roster
        ),
        "engram_imported_source_roster": source_roster,
        "inputs": {
            "summary_schema_exact_sha256": summary_schema_exact_sha256,
            "run_receipt_exact_sha256": run_exact_sha256,
            "evidence_bundle_exact_sha256": bundle_exact_sha256,
            "source_run_receipt_sha256": run.receipt_sha256,
            "source_bundle_sha256": bundle.bundle_sha256,
            "validation_input_sha256": validation_input_sha256,
        },
        "lineage": {
            "study_run_id": run.study_run_id,
            "neural_durable_evidence_profile": run.neural_durable_evidence_profile,
            "neural_provider_identity_sha256": run.neural_provider_identity_sha256,
            "run_status": run.status,
            "preparation_phase": bundle.preparation_attempt.phase,
            "preparation_outcome": bundle.preparation_attempt.outcome,
            "completed_step_count": len(run.steps),
            "terminal_neural_execution_count": len(run.neural_executions),
            "provider_step_execution_count": len(bundle.step_execution_receipts),
            "provider_step_attempt_count": len(bundle.step_attempt_receipts),
            "worker_termination_attempt_count": len(
                bundle.worker_termination_attempt_receipts
            ),
            "worker_terminal_disposition": bundle.worker_terminal_disposition,
            "tail_disposition_receipt_sha256": (
                tail.receipt_sha256 if tail is not None else None
            ),
            "worker_lifecycle_receipt_sha256": (
                lifecycle.receipt_sha256 if lifecycle is not None else None
            ),
            "neural_cleanup_confirmed": neural_cleanup.confirmed,
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
    }
    summary["receipt_sha256"] = digest_bytes(canonical(summary))

    from jsonschema import Draft202012Validator  # noqa: PLC0415

    schema = load_json_payload(
        summary_schema_payload,
        "validation summary schema",
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(summary),
        key=lambda error: tuple(str(part) for part in error.absolute_path),
    )
    if errors:
        location = "/".join(str(part) for part in errors[0].absolute_path) or "<root>"
        raise ValueError(f"validation summary fails schema at {location}")
    if summary["receipt_sha256"] != digest_bytes(
        canonical(
            {key: value for key, value in summary.items() if key != "receipt_sha256"}
        )
    ):
        raise ValueError("validation summary self-digest differs")
    summary_payload = canonical(summary) + b"\n"
    if len(summary_payload) > MAX_SUMMARY_BYTES:
        raise ValueError("external Engram NEST validation summary exceeds its bound")
    if (
        capture_repository_identity(prisoma_root, args.expected_prisoma_revision)
        != prisoma_identity
        or capture_repository_files(
            prisoma_root,
            args.expected_prisoma_revision,
            VALIDATOR_SOURCE_PATHS,
            MAX_SCHEMA_BYTES,
        )
        != validator_sources
        or capture_repository_identity(engram_root, args.expected_engram_revision)
        != engram_identity
    ):
        raise ValueError("validation source provenance changed before publication")
    if args.verify:
        if snapshot_regular_file(output, MAX_SUMMARY_BYTES) != summary_payload:
            raise ValueError("external Engram NEST validation summary differs")
        print(f"OK: verified external Engram NEST validation summary at {output}")
        return 0
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.parent.resolve(strict=True) != output.parent:
        raise ValueError("validation summary output parent traverses a link")
    write_new_receipt(output, summary_payload)
    print(f"OK: wrote external Engram NEST validation summary to {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
