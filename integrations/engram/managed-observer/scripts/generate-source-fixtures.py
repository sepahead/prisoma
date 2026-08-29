#!/usr/bin/env python3
"""Regenerate Prisoma source vectors with Engram's current receipt builders."""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import os
import sys
import tempfile
from pathlib import Path
from typing import Any

from source_provenance import (
    capture_imported_source_roster,
    capture_repository_file,
    digest_regular_file,
    imported_source_roster_sha256,
    snapshot_regular_file,
    verify_imported_source_roster_unchanged,
    verify_repository_revision,
)


ROOT = Path(__file__).resolve().parents[4]
FIXTURES = ROOT / "integrations" / "engram" / "managed-observer" / "fixtures"
PROVENANCE = (
    ROOT
    / "integrations"
    / "engram"
    / "managed-observer"
    / "contracts"
    / "PROVENANCE.json"
)
IPC_CONTRACT_SOURCE = Path(
    "integrations/contracts/engram.managed-runtime-ipc.v1.schema.json"
)
FINITE_FLOAT_SOURCE = Path(
    "integrations/contracts/engram.managed-runtime-finite-float.v1.json"
)
IPC_CONTRACT_COPY = PROVENANCE.parent / "managed-runtime-ipc.schema.json"
FINITE_FLOAT_COPY = PROVENANCE.parent / "engram.managed-runtime-finite-float.v1.json"
MAX_COPIED_CONTRACT_BYTES = 1024 * 1024
MAX_LOCAL_JSON_BYTES = 16 * 1024 * 1024
MAX_PYTHON_SOURCE_BYTES = 16 * 1024 * 1024
FIXTURE_NAMES = (
    "engram-run-receipt.generated.json",
    "engram-runtime-finished-neural-cleanup-failed.generated.json",
    "engram-zero-step-run-receipt.generated.json",
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


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(
        snapshot_regular_file(path, MAX_LOCAL_JSON_BYTES),
        object_pairs_hook=closed_object,
        parse_constant=reject_constant,
    )
    if not isinstance(value, dict):
        raise ValueError(f"JSON root is not an object: {path}")
    return value


def encoded(value: dict[str, Any]) -> bytes:
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


def digest_file(path: Path, max_bytes: int = MAX_LOCAL_JSON_BYTES) -> str:
    return digest_regular_file(path, max_bytes)


def replace_exact(path: Path, payload: bytes) -> None:
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.",
        dir=path.parent,
    )
    temporary = Path(temporary_name)
    try:
        os.fchmod(descriptor, 0o644)
        with os.fdopen(descriptor, "wb", closefd=True) as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def parser() -> argparse.ArgumentParser:
    command = argparse.ArgumentParser(description=__doc__)
    command.add_argument("--engram-root", required=True, type=Path)
    command.add_argument("--expected-engram-revision", required=True)
    command.add_argument("--verify", action="store_true")
    return command


def refreshed_provenance(
    *,
    expected_revision: str,
    source_roster: list[dict[str, Any]],
    copied_contract_sources: dict[str, dict[str, Any]],
    outputs: dict[Path, bytes],
) -> dict[str, Any]:
    provenance = load_json(PROVENANCE)
    closed_loop_path = "backend/optimization/extension_closed_loop.py"
    matching_sources = [row for row in source_roster if row["path"] == closed_loop_path]
    if len(matching_sources) != 1:
        raise ValueError("Engram closed-loop source is absent from imported roster")
    closed_loop_source = provenance.get("closed_loop_source")
    if (
        not isinstance(closed_loop_source, dict)
        or closed_loop_source.get("repository")
        != "git@github.com:sepahead/Paper2Brain.git"
        or closed_loop_source.get("path") != closed_loop_path
    ):
        raise ValueError("closed-loop provenance identity differs")
    closed_loop_source.update(
        {
            "revision": expected_revision,
            "sha256": matching_sources[0]["sha256"],
            "git_blob": matching_sources[0]["git_blob"],
            "source_state": "contained-in-recorded-revision",
            "imported_source_roster_sha256": imported_source_roster_sha256(
                source_roster
            ),
            "imported_source_roster": source_roster,
            "generator_source_sha256": digest_file(
                Path(__file__).resolve(),
                MAX_PYTHON_SOURCE_BYTES,
            ),
        }
    )
    ipc_source = copied_contract_sources[IPC_CONTRACT_SOURCE.as_posix()]
    finite_float_source = copied_contract_sources[FINITE_FLOAT_SOURCE.as_posix()]
    copied_contract = provenance.get("copied_contract")
    ipc_provenance = provenance.get("ipc_source")
    finite_float_provenance = provenance.get("finite_float_corpus")
    finite_float = load_json(FINITE_FLOAT_COPY)
    randomized = finite_float.get("randomized")
    if (
        not isinstance(copied_contract, dict)
        or not isinstance(ipc_provenance, dict)
        or not isinstance(finite_float_provenance, dict)
        or not isinstance(randomized, dict)
        or digest_file(IPC_CONTRACT_COPY) != ipc_source["sha256"]
        or digest_file(FINITE_FLOAT_COPY) != finite_float_source["sha256"]
    ):
        raise ValueError("copied Engram contract bytes or provenance differ")
    copied_contract["sha256"] = ipc_source["sha256"]
    ipc_provenance.update(
        {
            "revision": expected_revision,
            "git_blob": ipc_source["git_blob"],
            "source_state": "contained-in-recorded-revision",
        }
    )
    finite_float_provenance.update(
        {
            "revision": expected_revision,
            "sha256": finite_float_source["sha256"],
            "source_git_blob": finite_float_source["git_blob"],
            "source_state": "contained-in-recorded-revision",
            "case_count": len(finite_float.get("cases", [])),
            "randomized_sample_count": randomized.get("sample_count"),
            "randomized_accepted_count": randomized.get("accepted_count"),
            "randomized_transcript_sha256": randomized.get("transcript_sha256"),
        }
    )
    generated = provenance.get("generated_receipts")
    if not isinstance(generated, list):
        raise ValueError("generated receipt provenance roster is absent")
    rows = {row.get("path"): row for row in generated if isinstance(row, dict)}
    if len(rows) != len(FIXTURE_NAMES):
        raise ValueError("generated receipt provenance roster differs")
    for path, payload in outputs.items():
        relative = path.relative_to(ROOT).as_posix()
        row = rows.get(relative)
        if row is None:
            raise ValueError(f"generated receipt provenance is absent: {relative}")
        row["sha256"] = hashlib.sha256(payload).hexdigest()
    return provenance


def main() -> int:
    args = parser().parse_args()
    engram_root = args.engram_root.resolve(strict=True)
    verify_repository_revision(engram_root, args.expected_engram_revision)
    copied_contract_sources = {
        relative.as_posix(): capture_repository_file(
            engram_root,
            args.expected_engram_revision,
            relative,
            MAX_COPIED_CONTRACT_BYTES,
        )
        for relative in (IPC_CONTRACT_SOURCE, FINITE_FLOAT_SOURCE)
    }
    source_path = (
        engram_root / "backend" / "optimization" / "extension_closed_loop.py"
    ).resolve(strict=True)
    source_sha256 = digest_file(source_path, MAX_PYTHON_SOURCE_BYTES)
    engram_root_text = os.fspath(engram_root)
    sys.dont_write_bytecode = True
    sys.path[:] = [entry for entry in sys.path if entry != engram_root_text]
    sys.path.insert(0, engram_root_text)
    closed_loop_module = importlib.import_module(
        "backend.optimization.extension_closed_loop"
    )
    module_file = getattr(closed_loop_module, "__file__", None)
    if (
        not isinstance(module_file, str)
        or Path(module_file).resolve(strict=True) != source_path
    ):
        raise ValueError("Engram closed-loop builder import differs from source")
    ClosedLoopRunReceiptV2 = closed_loop_module.ClosedLoopRunReceiptV2
    RuntimeLifecycleReceiptBindingV1 = (
        closed_loop_module.RuntimeLifecycleReceiptBindingV1
    )
    build_cleanup_receipt = closed_loop_module.build_cleanup_receipt
    build_neural_execution_binding = closed_loop_module.build_neural_execution_binding
    build_run_receipt = closed_loop_module.build_run_receipt
    build_step_receipt = closed_loop_module.build_step_receipt
    closed_loop_step_id = closed_loop_module.closed_loop_step_id
    source_roster = capture_imported_source_roster(
        engram_root,
        args.expected_engram_revision,
    )

    outputs: dict[Path, bytes] = {}
    for name in FIXTURE_NAMES:
        path = FIXTURES / name
        source = load_json(path)
        initial_snapshot = source["initial_snapshot_sha256"]
        timebase = source.get(
            "timebase",
            {
                "runtime_step_duration_tics": 1_000,
                "neural_step_duration_tics": 1_000,
            },
        )
        prior_output: str | None = None
        steps = []
        for index, raw_step in enumerate(source["steps"]):
            values = {
                key: value
                for key, value in raw_step.items()
                if key
                not in {
                    "schema_version",
                    "receipt_sha256",
                    "provider_execution_scope",
                    "provider_execution_sha256",
                }
            }
            values["input_snapshot_sha256"] = (
                initial_snapshot if index == 0 else prior_output
            )
            values["step_id"] = closed_loop_step_id(
                values["study_run_id"], values["step_index"]
            )
            values["provider_execution_scope"] = raw_step.get(
                "provider_execution_scope", "nest-exact-step-readback"
            )
            values["provider_execution_sha256"] = raw_step.get(
                "provider_execution_sha256",
                hashlib.sha256(f"{name}:provider:{index + 1}".encode()).hexdigest(),
            )
            values["fault_codes"] = tuple(values["fault_codes"])
            step = build_step_receipt(**values)
            steps.append(step)
            prior_output = step.output_snapshot_sha256
        neural_executions = tuple(
            build_neural_execution_binding(
                step_index=step.step_index,
                step_id=step.step_id,
                neural_request_sha256=step.neural_request_sha256,
                neural_result_sha256=step.neural_result_sha256,
                provider_execution_scope=step.provider_execution_scope,
                provider_execution_sha256=step.provider_execution_sha256,
            )
            for step in steps
        )
        cleanup_rows = []
        for row in source["cleanup"]:
            cleanup_values = {
                key: value
                for key, value in row.items()
                if key
                not in {
                    "schema_version",
                    "receipt_sha256",
                    "provider_terminal_receipt_sha256",
                    "provider_lifecycle_receipt_sha256",
                }
            }
            if cleanup_values["runtime_lifecycle"] is not None:
                cleanup_values["runtime_lifecycle"] = (
                    RuntimeLifecycleReceiptBindingV1.model_validate(
                        cleanup_values["runtime_lifecycle"]
                    )
                )
            if row["component"] == "neural" and name == FIXTURE_NAMES[0]:
                cleanup_values["provider_terminal_receipt_sha256"] = (
                    row.get("provider_terminal_receipt_sha256")
                    or hashlib.sha256(b"fixture:nest:terminal-tail").hexdigest()
                )
                cleanup_values["provider_lifecycle_receipt_sha256"] = (
                    row.get("provider_lifecycle_receipt_sha256")
                    or hashlib.sha256(b"fixture:nest:lifecycle-tail").hexdigest()
                )
            else:
                cleanup_values["provider_terminal_receipt_sha256"] = row.get(
                    "provider_terminal_receipt_sha256"
                )
                cleanup_values["provider_lifecycle_receipt_sha256"] = row.get(
                    "provider_lifecycle_receipt_sha256"
                )
            cleanup_rows.append(build_cleanup_receipt(**cleanup_values))
        cleanup = tuple(cleanup_rows)
        runtime_lifecycle = source["runtime_lifecycle"]
        if runtime_lifecycle is not None:
            runtime_lifecycle = RuntimeLifecycleReceiptBindingV1.model_validate(
                runtime_lifecycle
            )
        values = {
            key: value
            for key, value in source.items()
            if key
            not in {
                "schema_version",
                "digest_canonicalization",
                "timebase",
                "steps",
                "neural_executions",
                "cleanup",
                "runtime_lifecycle",
                "last_verified_simulation_time_tics",
                "runtime_progress_disposition",
                "transcript_sha256",
                "receipt_sha256",
            }
        }
        values["neural_durable_evidence_profile"] = source.get(
            "neural_durable_evidence_profile",
            "none",
        )
        last_verified_time = (
            len(steps) * timebase["runtime_step_duration_tics"]
            if initial_snapshot is not None
            else None
        )
        if source["runtime_finish_sha256"] is not None:
            runtime_progress_disposition = "finished-and-host-verified"
        elif initial_snapshot is None:
            runtime_progress_disposition = "not-started"
        else:
            runtime_progress_disposition = "last-host-verified"
        receipt = build_run_receipt(
            **values,
            timebase=timebase,
            last_verified_simulation_time_tics=last_verified_time,
            runtime_progress_disposition=runtime_progress_disposition,
            steps=tuple(steps),
            neural_executions=neural_executions,
            cleanup=cleanup,
            runtime_lifecycle=runtime_lifecycle,
        )
        validated = ClosedLoopRunReceiptV2.model_validate(
            receipt.model_dump(mode="json")
        )
        outputs[path] = encoded(validated.model_dump(mode="json"))

    if source_sha256 != digest_file(source_path, MAX_PYTHON_SOURCE_BYTES):
        raise ValueError("Engram closed-loop source changed during fixture generation")
    verify_imported_source_roster_unchanged(
        engram_root,
        args.expected_engram_revision,
        source_roster,
    )
    provenance_payload = encoded(
        refreshed_provenance(
            expected_revision=args.expected_engram_revision,
            source_roster=source_roster,
            copied_contract_sources=copied_contract_sources,
            outputs=outputs,
        )
    )

    drift = [
        path
        for path, payload in outputs.items()
        if snapshot_regular_file(path, MAX_LOCAL_JSON_BYTES) != payload
    ]
    provenance_drift = load_json(PROVENANCE) != json.loads(
        provenance_payload,
        object_pairs_hook=closed_object,
        parse_constant=reject_constant,
    )
    if args.verify:
        if drift or provenance_drift:
            names = ", ".join(path.name for path in drift) or "none"
            raise ValueError(
                "Engram-generated source fixture or provenance drift: "
                f"fixtures={names}, provenance={provenance_drift}"
            )
        print("OK: Engram-generated source fixtures and provenance are current")
        return 0
    for path in drift:
        replace_exact(path, outputs[path])
    if provenance_drift:
        replace_exact(PROVENANCE, provenance_payload)
    print(
        f"OK: checked {len(outputs)} Engram source receipts; "
        f"refreshed {len(drift)} fixtures and {int(provenance_drift)} provenance file"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
