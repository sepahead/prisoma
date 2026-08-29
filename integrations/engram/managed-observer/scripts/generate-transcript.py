#!/usr/bin/env python3
"""Generate the normalized Prisoma managed-observer transcript fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
import stat
import struct
import subprocess
from pathlib import Path
from typing import Any

from source_provenance import snapshot_regular_file


ROOT = Path(__file__).resolve().parents[4]
INTEGRATION = ROOT / "integrations" / "engram" / "managed-observer"
CONTRACTS = INTEGRATION / "contracts"
SOURCE_RECEIPT = INTEGRATION / "fixtures" / "engram-run-receipt.generated.json"
DEFAULT_BINARY = (
    ROOT
    / "crates"
    / "engram-managed-observer"
    / "target"
    / "debug"
    / "prisoma-engram-managed-observer"
)
IPC = "engram.managed-runtime-ipc.v1"
MAX_RUNTIME_OUTPUT_BYTES = 64 * 1024
MAX_LOCAL_INPUT_BYTES = 4 * 1024 * 1024
GENERATION = {
    "installation_id": "inst_" + "a" * 64,
    "generation_id": "gen_" + "b" * 64,
    "ordinal": 1,
}
SCHEMAS = {
    "configuration": (
        "prisoma.observer.configuration.v1",
        "configuration.schema.json",
    ),
    "finish-request": (
        "prisoma.observer.finish-request.v1",
        "finish-request.schema.json",
    ),
    "finish-response": (
        "prisoma.observer.finish-response.v1",
        "finish-response.schema.json",
    ),
    "observe-request": (
        "prisoma.observer.observe-request.v1",
        "observe-request.schema.json",
    ),
    "observe-response": (
        "prisoma.observer.observe-response.v1",
        "observe-response.schema.json",
    ),
    "prepare-request": (
        "prisoma.observer.prepare-request.v1",
        "prepare-request.schema.json",
    ),
    "prepare-response": (
        "prisoma.observer.prepare-response.v1",
        "prepare-response.schema.json",
    ),
}


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
        snapshot_regular_file(path, MAX_LOCAL_INPUT_BYTES),
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


def digest(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def schema_reference(key: str) -> dict[str, str]:
    schema_id, filename = SCHEMAS[key]
    return {
        "schema_id": schema_id,
        "schema_sha256": digest(
            snapshot_regular_file(CONTRACTS / filename, MAX_LOCAL_INPUT_BYTES)
        ),
    }


def operation_roster() -> list[dict[str, Any]]:
    bounds = (
        ("finish", 32768, 8192),
        ("observe", 49152, 8192),
        ("prepare", 32768, 8192),
    )
    return [
        {
            "operation_id": f"prisoma.observer.{name}.v1",
            "class": "observation",
            "effect": "none",
            "artifact_access": {"read": "none", "write": "none"},
            "request_schema": schema_reference(f"{name}-request"),
            "response_schema": schema_reference(f"{name}-response"),
            "compute_grant": "none",
            "timeout_ms": 1000,
            "max_cpu_time_ms": 0,
            "max_request_bytes": request_bytes,
            "max_response_bytes": response_bytes,
        }
        for name, request_bytes, response_bytes in bounds
    ]


def host_handshake(roster_digest: str) -> dict[str, Any]:
    configuration_bytes = snapshot_regular_file(
        INTEGRATION / "configuration.json",
        MAX_LOCAL_INPUT_BYTES,
    )
    configuration = load_json(INTEGRATION / "configuration.json")
    canonical_digest = digest(canonical(configuration))
    identity = {
        "manifest_exact_sha256": "1" * 64,
        "manifest_canonical_sha256": "2" * 64,
        "package_lock_exact_sha256": "3" * 64,
        "package_lock_canonical_sha256": "4" * 64,
        "package_sha256": "5" * 64,
        "executable_sha256": "6" * 64,
        "configuration_exact_sha256": digest(configuration_bytes),
        "configuration_canonical_sha256": canonical_digest,
        "target_id": "macos-aarch64-darwin",
        "profile": "engram.reviewed-native-development.v1",
        "launch_abi": "engram.managed-runtime-stdio.v1",
        "operation_roster_sha256": roster_digest,
        "schema_registry_sha256": "7" * 64,
        "installation_id": GENERATION["installation_id"],
    }
    return {
        "schema_version": "1.0",
        "protocol": IPC,
        "kind": "host.handshake",
        "sender": "host",
        "generation": GENERATION,
        "sequence": 0,
        "message_id": "msg_" + "1" * 32,
        "body": {
            "challenge": "chal_" + "c" * 64,
            "identity": identity,
            "configuration": {
                "schema": schema_reference("configuration"),
                "canonical_sha256": canonical_digest,
                "document": configuration,
            },
            "max_frame_bytes": 65536,
        },
    }


def operation_request(
    operation: dict[str, Any], sequence: int, marker: str, control: dict[str, Any]
) -> dict[str, Any]:
    identity_fields = ("operation_id", "class", "effect", "artifact_access")
    return {
        "schema_version": "1.0",
        "protocol": IPC,
        "kind": "operation.request",
        "sender": "host",
        "generation": GENERATION,
        "sequence": sequence,
        "message_id": "msg_" + marker * 32,
        "body": {
            "idempotency_key": "idem_" + marker * 64,
            "operation": {field: operation[field] for field in identity_fields},
            "request_schema": operation["request_schema"],
            "response_schema": operation["response_schema"],
            "compute_grant": {"mode": "none"},
            "timeout_ms": 1000,
            "control": control,
            "bulk": {"inline": False, "references": []},
        },
    }


def prepare_control(source: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": "prisoma.observer.prepare-request.v1",
        "study_run_id": source["study_run_id"],
        "study_definition_sha256": source["study_definition_sha256"],
        "closed_loop_definition_sha256": source["closed_loop_definition_sha256"],
        "runtime_binding_sha256": source["runtime_binding_sha256"],
        "runtime_adapter_configuration_sha256": source[
            "runtime_adapter_configuration_sha256"
        ],
        "neural_provider_identity_sha256": source["neural_provider_identity_sha256"],
        "channel_ids": ["channel-01", "channel-02", "channel-03"],
        "subject_ids": ["drone-01", "drone-02", "drone-03"],
        "planned_step_count": source["planned_step_count"],
        "max_steps": 8,
    }


def observe_control(step: dict[str, Any]) -> dict[str, Any]:
    fields = (
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
    control = {field: step[field] for field in fields}
    control["schema_version"] = "prisoma.observer.observe-request.v1"
    control["source_receipt_sha256"] = step["receipt_sha256"]
    return control


def runtime_lifecycle_values(source: dict[str, Any]) -> list[Any]:
    lifecycle = source["runtime_lifecycle"]
    if lifecycle is None:
        return [None] * 22
    fields = (
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
    )
    if set(lifecycle) != set(fields):
        raise ValueError("source runtime lifecycle field roster differs")
    return [lifecycle[field] for field in fields]


def finish_control(source: dict[str, Any]) -> dict[str, Any]:
    cleanup = source["cleanup"]
    timebase = source["timebase"]
    tail = (
        source["neural_executions"][len(source["steps"])]
        if len(source["neural_executions"]) > len(source["steps"])
        else None
    )
    return {
        "schema_version": "prisoma.observer.finish-request.v1",
        "digest_canonicalization": source["digest_canonicalization"],
        "study_run_id": source["study_run_id"],
        "timebase_values": [
            timebase["schema_version"],
            timebase["tic_unit"],
            timebase["runtime_step_duration_tics"],
            timebase["neural_step_duration_tics"],
            timebase["clock_relation"],
            timebase["coupling"],
            timebase["causality_policy"],
            timebase["dispatch_order"],
            timebase["observation_sample_phase"],
            timebase["action_application"],
        ],
        "runtime_deadline_enforcement": source["runtime_deadline_enforcement"],
        "neural_deadline_enforcement": source["neural_deadline_enforcement"],
        "neural_preparation_sha256": source["neural_preparation_sha256"],
        "neural_session_receipt_sha256": source["neural_session_receipt_sha256"],
        "neural_durable_evidence_profile": source["neural_durable_evidence_profile"],
        "initial_snapshot_sha256": source["initial_snapshot_sha256"],
        "last_verified_simulation_time_tics": source[
            "last_verified_simulation_time_tics"
        ],
        "runtime_progress_disposition": source["runtime_progress_disposition"],
        "planned_step_count": source["planned_step_count"],
        "step_count": len(source["steps"]),
        "neural_tail_values": (
            [
                tail["step_index"],
                tail["step_id"],
                tail["neural_request_sha256"],
                tail["neural_result_sha256"],
                tail["provider_execution_scope"],
                tail["provider_execution_sha256"],
            ]
            if tail
            else [None] * 6
        ),
        "runtime_finish_sha256": source["runtime_finish_sha256"],
        "runtime_lifecycle_values": runtime_lifecycle_values(source),
        "runtime_cleanup_values": cleanup_values(cleanup[0]),
        "neural_cleanup_values": cleanup_values(cleanup[1]),
        "source_status": source["status"],
        "primary_reason_code": source["primary_reason_code"],
        "terminal_reason_code": source["terminal_reason_code"],
        "cleanup_complete": source["cleanup_complete"],
        "source_transcript_sha256": source["transcript_sha256"],
        "source_run_receipt_sha256": source["receipt_sha256"],
        "simulator_only": source["simulator_only"],
        "physical_actuation": source["physical_actuation"],
        "ncp_qualified": source["ncp_qualified"],
        "scientific_authority": source["scientific_authority"],
        "is_paper_local_evidence": source["is_paper_local_evidence"],
        "calibrated_posterior": source["calibrated_posterior"],
    }


def cleanup_values(item: dict[str, Any]) -> list[Any]:
    return [
        item["schema_version"],
        item["component"],
        item["owner_identity_sha256"],
        item["mode"],
        item["attempted"],
        item["confirmed"],
        item["containment_empty"],
        item["reason_code"],
        (
            item["runtime_lifecycle"]["binding_sha256"]
            if item["runtime_lifecycle"] is not None
            else None
        ),
        item["provider_terminal_receipt_sha256"],
        item["provider_lifecycle_receipt_sha256"],
        item["receipt_sha256"],
    ]


def decode_frames(payload: bytes) -> list[dict[str, Any]]:
    frames: list[dict[str, Any]] = []
    offset = 0
    while offset < len(payload):
        if len(payload) - offset < 4:
            raise ValueError("runtime emitted a truncated frame prefix")
        length = struct.unpack(">I", payload[offset : offset + 4])[0]
        start = offset + 4
        end = start + length
        if not 0 < length <= 65536 or end > len(payload):
            raise ValueError("runtime emitted an invalid frame length")
        envelope = json.loads(payload[start:end], object_pairs_hook=closed_object)
        if not isinstance(envelope, dict):
            raise ValueError("runtime frame root is not an object")
        frames.append(envelope)
        offset = end
    return frames


def frame_record(direction: str, envelope: dict[str, Any]) -> dict[str, Any]:
    payload = canonical(envelope)
    return {
        "direction": direction,
        "payload_length": len(payload),
        "prefix_hex": struct.pack(">I", len(payload)).hex(),
        "payload_sha256": digest(payload),
        "envelope": envelope,
    }


def checked_binary(path: Path) -> Path:
    binary = path.resolve(strict=True)
    metadata = binary.stat()
    if not stat.S_ISREG(metadata.st_mode) or not metadata.st_mode & stat.S_IXUSR:
        raise ValueError("managed observer binary is not an executable regular file")
    return binary


def generate(binary_path: Path) -> dict[str, Any]:
    binary = checked_binary(binary_path)
    source = load_json(SOURCE_RECEIPT)
    roster = operation_roster()
    roster_digest = digest(b"engram-managed-operation-roster-v1\0" + canonical(roster))
    by_name = {row["operation_id"].split(".")[-2]: row for row in roster}
    host_frames = [host_handshake(roster_digest)]
    controls = [
        ("prepare", "2", prepare_control(source)),
        ("observe", "3", observe_control(source["steps"][0])),
        ("observe", "4", observe_control(source["steps"][1])),
        ("finish", "5", finish_control(source)),
    ]
    for sequence, (name, marker, control) in enumerate(controls, start=1):
        host_frames.append(operation_request(by_name[name], sequence, marker, control))
    wire = b"".join(
        struct.pack(">I", len(canonical(frame))) + canonical(frame)
        for frame in host_frames
    )
    completed = subprocess.run(
        [str(binary)],
        input=wire,
        capture_output=True,
        check=False,
        timeout=5,
        close_fds=True,
    )
    if completed.returncode != 0 or completed.stderr:
        raise RuntimeError("managed observer rejected its sample transcript")
    if len(completed.stdout) > MAX_RUNTIME_OUTPUT_BYTES:
        raise ValueError("managed observer output exceeded the fixture bound")
    runtime_frames = decode_frames(completed.stdout)
    if len(runtime_frames) != len(host_frames):
        raise ValueError("managed observer emitted an unexpected frame count")
    fixed_markers = ("8", "9", "a", "b", "c")
    for frame, marker in zip(runtime_frames, fixed_markers, strict=True):
        frame["message_id"] = "msg_" + marker * 32
    runtime_frames[0]["body"]["runtime_nonce"] = "nonce_" + "d" * 64
    for frame in runtime_frames[1:]:
        control = frame["body"]["control"]
        if (
            frame["body"]["status"] != "succeeded"
            or control["authority"] != "read-only-observer"
            or control["roster_authority"] != "host-declared-projection"
            or control["source_roster_authenticated"]
            or not control["descriptive_only"]
            or control["agent_bridge_command"]
            or control["physical_actuation"]
            or control["ncp_used"]
            or control["pid_result"]
            or control["source_durable_evidence_verified"]
            or control["scientific_authority"]
            or control["is_paper_local_evidence"]
            or control["calibrated_posterior"]
        ):
            raise ValueError("managed observer semantic boundary changed")
    if not runtime_frames[-1]["body"]["control"]["state_cleared"]:
        raise ValueError("managed observer did not clear terminal state")
    frames: list[dict[str, Any]] = []
    for host, runtime in zip(host_frames, runtime_frames, strict=True):
        frames.append(frame_record("host-to-runtime", host))
        frames.append(frame_record("runtime-to-host", runtime))
    return {
        "schema_version": "prisoma.observer.sample-transcript.v1",
        "fixture_only": True,
        "real_binary_executed": True,
        "framing": "uint32-be-length-prefixed-canonical-json",
        "normalization": {
            "fields": [
                "runtime envelope message_id",
                "runtime handshake runtime_nonce",
            ],
            "semantic_receipts_recomputed": False,
        },
        "operation_roster_sha256": roster_digest,
        "source_run_receipt_sha256": source["receipt_sha256"],
        "source_fixture_exact_sha256": digest(
            snapshot_regular_file(SOURCE_RECEIPT, MAX_LOCAL_INPUT_BYTES)
        ),
        "scenario": {
            "subject_count": 3,
            "subject_ids": ["drone-01", "drone-02", "drone-03"],
            "observed_step_count": 2,
            "authority": "read-only-observer",
            "roster_authority": "host-declared-projection",
            "source_roster_authenticated": False,
            "agent_bridge_command": False,
            "ncp_mode": "none",
        },
        "frames": frames,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    parser.add_argument("--compact", action="store_true")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--verify", type=Path)
    arguments = parser.parse_args()
    if arguments.output is not None and arguments.verify is not None:
        raise SystemExit("--output and --verify are mutually exclusive")
    document = generate(arguments.binary)
    if arguments.verify is not None:
        expected = load_json(arguments.verify.resolve(strict=True))
        if document != expected:
            raise SystemExit("managed observer transcript fixture is stale")
        print("OK: managed observer transcript replays exactly")
        return
    separators = (",", ":") if arguments.compact else None
    rendered = json.dumps(
        document,
        indent=None if arguments.compact else 2,
        separators=separators,
    )
    if arguments.output is not None:
        arguments.output.write_text(f"{rendered}\n", encoding="utf-8")
        print(f"OK: wrote managed observer transcript to {arguments.output}")
        return
    print(rendered)


if __name__ == "__main__":
    main()
