#!/usr/bin/env python3
"""Review the immutable CREBAIN 1/2/3-drone real-NEST capture matrix."""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
import math
import os
import re
import stat
import struct
import subprocess
import sys
import tempfile
from collections.abc import Mapping, Sequence
from pathlib import Path, PurePosixPath
from types import ModuleType
from typing import Any

import observed_build
from source_provenance import (
    EVIDENCE_PUBLICATION_POLICY,
    EVIDENCE_PUBLICATION_ROSTER_DOMAIN,
    capture_evidence_publication,
    capture_repository_file,
    capture_repository_files,
    capture_repository_identity,
    snapshot_regular_file,
    valid_git_object,
    verify_committed_source_roster,
)


ROOT = Path(__file__).resolve().parents[4]
INTEGRATION = ROOT / "integrations" / "engram" / "managed-observer"
MATRIX_SCHEMA = (
    INTEGRATION / "evidence" / "crebain-real-nest-observer-matrix.schema.json"
)
BUILD_RECEIPT_SCHEMA = (
    INTEGRATION / "evidence" / "observer-release-build-receipt.schema.json"
)
NEST_SUMMARY_SCHEMA = (
    INTEGRATION / "evidence" / "engram-nest-evidence-validation-summary.schema.json"
)
TRANSCRIPT_GENERATOR = INTEGRATION / "scripts" / "generate-transcript.py"
CONTRACT_CHECKER = INTEGRATION / "scripts" / "check-contract.py"
NEST_SUMMARIZER = INTEGRATION / "scripts" / "summarize-nest-evidence.py"
MANAGED_RUNTIME_FLOAT_CONTRACT = (
    INTEGRATION / "contracts" / "engram.managed-runtime-finite-float.v1.json"
)
NEST_VALIDATOR_SOURCE_PATHS = tuple(
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
SOURCE_RECEIPT = INTEGRATION / "fixtures" / "engram-run-receipt.generated.json"
ZERO_STEP_SOURCE_RECEIPT = (
    INTEGRATION / "fixtures" / "engram-zero-step-run-receipt.generated.json"
)
RELEASE_BINARY = (
    ROOT
    / "crates"
    / "engram-managed-observer"
    / "target"
    / "release"
    / "prisoma-engram-managed-observer"
)
CRATE_MANIFEST = ROOT / "crates" / "engram-managed-observer" / "Cargo.toml"
CRATE_LOCK = ROOT / "crates" / "engram-managed-observer" / "Cargo.lock"
INDEX_RELATIVE = PurePosixPath(
    "integrations/engram/managed-simulation/operational-evidence/"
    "real-nest-3.9-v2/INDEX.json"
)
EVIDENCE_DIRECTORY_RELATIVE = INDEX_RELATIVE.parent
INPUT_SUITE_RELATIVE = PurePosixPath(
    "integrations/engram/managed-simulation/operational-inputs/"
    "real-nest-3.9-v1/SUITE.json"
)
CAPTURE_PATHS = {
    1: "capture-1-drone.json",
    2: "capture-2-drones.json",
    3: "capture-3-drones.json",
}
EVIDENCE_PUBLICATION_PATHS = tuple(
    Path(path)
    for path in sorted(
        (
            INDEX_RELATIVE.as_posix(),
            *(
                f"{EVIDENCE_DIRECTORY_RELATIVE.as_posix()}/{name}"
                for name in CAPTURE_PATHS.values()
            ),
        )
    )
)
SCHEMA_VERSION = "prisoma.observer.crebain-real-nest-matrix.v1"
CAPTURE_SCHEMA_VERSION = "crebain.real-nest-closed-loop-capture.v2"
INDEX_SCHEMA_VERSION = "crebain.real-nest-closed-loop-evidence-index.v2"
AUDIT_ONLY_INDEX_SCHEMA_VERSION = "crebain.real-nest-closed-loop-evidence-index.v1"
MAX_INDEX_BYTES = 1024 * 1024
MAX_CAPTURE_BYTES = 16 * 1024 * 1024
MAX_SCHEMA_BYTES = 1024 * 1024
MAX_BINARY_BYTES = 64 * 1024 * 1024
MAX_SOURCE_BYTES = 16 * 1024 * 1024
MAX_SOURCE_TOTAL_BYTES = 256 * 1024 * 1024
MAX_RECEIPT_STORE_BYTES = 64 * 1024 * 1024
RECEIPT_STORE_FILE_COUNT = 8
RECEIPT_STORE_SCHEMA = "engram.extension-closed-loop-receipt-store.v5"
RECEIPT_STORE_POLICY = "engram.extension-closed-loop-receipt-store-policy.v5"
RECEIPT_STORE_CANONICALIZATION = "engram.managed-runtime-json.v1"
RECEIPT_STORE_LOCK_BYTES = b"engram-extension-closed-loop-receipt-store-lock-v1\n"
RECEIPT_STORE_SIDECARS_SCHEMA = "crebain.closed-loop-receipt-store-sidecars.v1"
MANAGED_RUNTIME_MAX_SAFE_INTEGER = 9_007_199_254_740_991
MANAGED_RUNTIME_MAX_FLOAT_ABS = 1.0e300
MANAGED_RUNTIME_MAX_DEPTH = 64
MANAGED_RUNTIME_MAX_NODES = 131_072
MAX_JSON_DEPTH = 96
MAX_JSON_NODES = 2_000_000
MAX_OBSERVER_OUTPUT_BYTES = 16 * 1024 * 1024
MAX_MATRIX_OUTPUT_BYTES = 4 * 1024 * 1024
MAX_GIT_OUTPUT_BYTES = 32 * 1024 * 1024
MAX_TOOL_DIAGNOSTIC_BYTES = 64 * 1024
HEX = frozenset("0123456789abcdef")
EXPECTED_CREBAIN_OPERATION_IDS = [
    "crebain.simulation.finish.v1",
    "crebain.simulation.finish.v3",
    "crebain.simulation.prepare.v1",
    "crebain.simulation.prepare.v3",
    "crebain.simulation.step.v1",
    "crebain.simulation.step.v3",
]
EXPECTED_STANDARD_SCHEMA_IDS = {
    "engram.closed-loop-simulator.finish-request.v3",
    "engram.closed-loop-simulator.finish-response.v3",
    "engram.closed-loop-simulator.prepare-request.v3",
    "engram.closed-loop-simulator.prepare-response.v3",
    "engram.closed-loop-simulator.step-request.v3",
    "engram.closed-loop-simulator.step-response.v3",
}
EXPECTED_CREBAIN_EXTENSION_ID = "sepahead.crebain.simulation"
EXPECTED_CREBAIN_EXTENSION_VERSION = "0.1.0"
EXPECTED_CREBAIN_TARGET_ID = "macos-aarch64-darwin"
EXPECTED_REVIEWED_PROFILE = "engram.reviewed-native-development.v1"
EXPECTED_ENGRAM_PACK_TOOL_PATH = "scripts/engram_extension.py"
EXPECTED_ENGRAM_EXEC_GATE_PATH = "backend/integrations/contained_exec_gate.py"
EXPECTED_EXEC_GATE_ARGUMENT_SHAPE = [
    "python",
    "-I",
    "-S",
    "-c",
    "frozen-exec-gate-source",
    "--gate-fd",
    "descriptor",
    "--ready-fd",
    "descriptor",
    "--expected-session-id",
    "supervisor-session-id",
    "target-command",
]
EXPECTED_CREBAIN_TARGET = {
    "target_id": "macos-aarch64-darwin",
    "operating_system": "macos",
    "architecture": "aarch64",
    "abi": "darwin",
    "rust_target_triple": "aarch64-apple-darwin",
}
CREBAIN_NO_AUTHORITY = {
    "execution": False,
    "installation": False,
    "ncp": False,
    "physical": False,
    "plant": False,
    "scientific": False,
}
CREBAIN_BUILD_CONTRACT_PATHS = {
    f"integrations/engram/managed-simulation/contracts/{name}.schema.json"
    for name in (
        "configuration",
        "finish-request",
        "finish-response",
        "managed-runtime-ipc",
        "prepare-request",
        "prepare-response",
        "standard-v3-finish-request",
        "standard-v3-finish-response",
        "standard-v3-prepare-request",
        "standard-v3-prepare-response",
        "standard-v3-step-request",
        "standard-v3-step-response",
        "step-request",
        "step-response",
    )
}
EXPECTED_CREBAIN_BUILD_GENERATORS = [
    "scripts/build-managed-simulation-bootstrap.py",
    "scripts/managed_simulation_authoring_files.py",
    "scripts/managed_simulation_build_provenance.py",
]
EXPECTED_CREBAIN_TOOL_SOURCES = {
    "scripts/managed_simulation_authoring_files.py": "atomic-authoring-io",
    "scripts/managed_simulation_build_provenance.py": "receipt-validator",
    "scripts/run-managed-simulation-real-nest-proof.py": "capture-runner",
    "scripts/run-managed-simulation-real-nest-suite.py": "suite-runner",
}
EXPECTED_CREBAIN_CARGO_ARGV = [
    "rustup",
    "run",
    "1.91.1",
    "cargo",
    "build",
    "--locked",
    "--release",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "-p",
    "crebain-managed-simulation",
    "--target",
    "aarch64-apple-darwin",
    "--target-dir",
    "src-tauri/target/managed-simulation-bootstrap/observed-build-target",
]

EXTERNAL_LINEAGE_FIELDS = {
    "study_run_id",
    "neural_durable_evidence_profile",
    "neural_provider_identity_sha256",
    "run_status",
    "preparation_phase",
    "preparation_outcome",
    "completed_step_count",
    "terminal_neural_execution_count",
    "provider_step_execution_count",
    "provider_step_attempt_count",
    "worker_termination_attempt_count",
    "worker_terminal_disposition",
    "tail_disposition_receipt_sha256",
    "worker_lifecycle_receipt_sha256",
    "neural_cleanup_confirmed",
    "validated_against_run",
}
EXTERNAL_AUTHORITY_FIELDS = {
    "descriptive_only",
    "source_durable_evidence_verified",
    "engram_loaded_source_bytes_attested",
    "agent_bridge_command",
    "execution_authority",
    "pid_result",
    "ncp_authority",
    "physical_authority",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
}

INDEX_FIELDS = {
    "schema_version",
    "profile",
    "input_suite",
    "tool_source_closure",
    "crebain_source_repository",
    "engram",
    "package",
    "installed_package_proof_exact_sha256",
    "captures",
    "assertions",
    "authority",
    "disclosure",
}
CAPTURE_FIELDS = {
    "schema_version",
    "engram_source_sha256",
    "engram_source_closure",
    "package_generation_id",
    "installed_package_proof_exact_sha256",
    "installed_package_proof",
    "plan_exact_sha256",
    "nest_config_exact_sha256",
    "receipt_lock_timeout_ms",
    "run_plan",
    "nest_config",
    "summary",
    "terminal_receipt",
    "reviewed_native_runtime",
    "nest_worker_guardian_closure",
    "receipt_store_closure",
    "receipt_store_sidecars",
    "population_topology",
    "nest_evidence_bundle",
    "neural_steps",
    "assertions",
    "authority",
    "disclosure",
}
INPUT_SUITE_FIELDS = {
    "schema_version",
    "profile",
    "capture_schema_version",
    "nest_config",
    "runs",
    "constraints",
    "authority",
    "suite_definition_sha256",
}
INPUT_SUITE_RUN_FIELDS = {
    "drone_count",
    "plan_path",
    "plan_exact_sha256",
    "expected_channel_ids",
    "expected_population_count",
    "expected_population_neuron_count",
    "expected_device_node_count",
    "expected_connection_count",
    "expected_step_count",
}
INDEX_CAPTURE_FIELDS = {
    "drone_count",
    "path",
    "capture_sha256",
    "plan_exact_sha256",
    "receipt_sha256",
    "evidence_bundle_sha256",
    "receipt_store_id",
    "receipt_store_closure_sha256",
    "engram_source_closure_sha256",
    "engram_source_roster_sha256",
    "observed_build_receipt_exact_sha256",
    "population_count",
    "population_neuron_count",
    "device_node_count",
    "connection_count",
    "session_count",
}
INDEX_PACKAGE_FIELDS = {
    "store_id",
    "package_generation_id",
    "installation_id",
    "generation_core_sha256",
    "bundle_receipt_exact_sha256",
    "seal_receipt_exact_sha256",
    "install_observation_exact_sha256",
    "package_sha256",
    "executable_sha256",
    "configuration_canonical_sha256",
    "operation_roster_sha256",
    "receipt_sha256",
    "observed_build_receipt_exact_sha256",
    "observed_build_receipt_sha256",
    "package_stage_receipt_exact_sha256",
    "package_stage_receipt_sha256",
    "engram_pack_receipt_exact_sha256",
    "engram_pack_receipt_sha256",
    "crebain_commit",
    "crebain_tree",
    "crebain_origin_main",
    "engram_commit",
    "engram_tree",
    "engram_origin_main",
    "engram_extension_tool_sha256",
    "engram_extension_tool_git_blob",
    "build_source_roster_sha256",
    "build_input_identity_sha256",
    "executable_format",
    "executable_architecture",
    "build_stage_seal_pack_install_lineage_verified",
}
INDEX_ASSERTION_FIELDS = {
    "tracked_inputs_exact",
    "one_session_per_run",
    "exact_6n_population_topology",
    "one_two_three_drone_roster",
    "distinct_receipt_and_evidence_identities",
    "distinct_closed_receipt_stores",
    "common_clean_engram_source_roster",
    "distinct_engram_runtime_source_closures",
    "installed_package_lineage_common",
    "observed_build_stage_seal_install_lineage_common",
    "engram_pack_source_lineage_common",
    "crebain_source_lineage_common",
    "observed_build_stage_seal_pack_install_lineage_common",
}
INSTALLED_PACKAGE_PROOF_FIELDS = {
    "schema_version",
    "observed_build_receipt_exact_sha256",
    "observed_build_receipt_sha256",
    "observed_build_receipt",
    "package_stage_receipt_exact_sha256",
    "package_stage_receipt_sha256",
    "package_stage_receipt",
    "engram_pack_receipt_exact_sha256",
    "engram_pack_receipt_sha256",
    "engram_pack_receipt",
    "crebain_commit",
    "crebain_tree",
    "crebain_origin_main",
    "engram_commit",
    "engram_tree",
    "engram_origin_main",
    "engram_extension_tool_sha256",
    "engram_extension_tool_git_blob",
    "build_source_roster_sha256",
    "build_input_identity_sha256",
    "executable_format",
    "executable_architecture",
    "store_id",
    "package_generation_id",
    "installation_id",
    "generation_core_sha256",
    "bundle_receipt_exact_sha256",
    "seal_receipt_exact_sha256",
    "install_observation_exact_sha256",
    "manifest_exact_sha256",
    "package_lock_exact_sha256",
    "configuration_exact_sha256",
    "package_sha256",
    "executable_sha256",
    "configuration_canonical_sha256",
    "operation_roster_sha256",
    "operation_ids",
    "standard_schema_sha256",
    "drone_counts",
    "step_count",
    "fault_step",
    "fault",
    "host_policy",
    "recovery_controls_sha256",
    "baseline_three_controls_sha256",
    "replay_exact",
    "unaffected_lane_observations_exact",
    "negative_clock_gate",
    "signal_cancellation_gate",
    "installed_artifacts_reverified_after_execution",
    "generation_seal_package_bundle_store_lineage_verified",
    "build_stage_seal_install_lineage_verified",
    "build_stage_seal_pack_install_lineage_verified",
    "authority",
    "disclosure",
    "receipt_sha256",
}
CREBAIN_BUILD_RECEIPT_FIELDS = {
    "schema_version",
    "repository",
    "source",
    "generator",
    "cargo",
    "output",
    "input_identity_sha256",
    "claims",
    "authority",
    "disclosure",
    "receipt_sha256",
}
CREBAIN_BUILD_REPOSITORY_FIELDS = {
    "origin",
    "commit",
    "tree",
    "origin_main",
    "object_format",
    "clean",
}
CREBAIN_SOURCE_FIELDS = {"policy", "files", "roster_sha256"}
CREBAIN_SOURCE_ROW_FIELDS = {
    "relative_path",
    "size_bytes",
    "sha256",
    "git_mode",
    "git_blob",
}
CREBAIN_GENERATOR_FIELDS = {"files", "roster_sha256"}
CREBAIN_CARGO_FIELDS = {
    "workspace_manifest_path",
    "workspace_manifest_exact_sha256",
    "package_manifest_path",
    "package_manifest_exact_sha256",
    "lock_path",
    "lock_exact_sha256",
    "toolchain_path",
    "toolchain_exact_sha256",
    "rust_toolchain",
    "rustc_version",
    "cargo_version",
    "argv",
    "profile",
    "target",
    "target_directory_policy",
    "environment_policy",
}
CREBAIN_BUILD_OUTPUT_FIELDS = {
    "file_name",
    "byte_length",
    "sha256",
    "source_mode",
    "format",
    "architecture",
    "file_type",
}
CREBAIN_BUILD_CLAIM_FIELDS = {
    "observed_local_build",
    "reproducible_build",
    "signature",
    "external_dependency_bytes_attested",
    "complete_environment_attested",
}
CREBAIN_STAGE_RECEIPT_FIELDS = {
    "schema_version",
    "observed_build_receipt_exact_sha256",
    "observed_build_receipt_sha256",
    "crebain_commit",
    "crebain_tree",
    "origin_main",
    "target",
    "recipe_exact_sha256",
    "configuration_exact_sha256",
    "source_executable",
    "staged_executable",
    "package_inventory",
    "package_inventory_sha256",
    "authority",
    "disclosure",
    "receipt_sha256",
}
CREBAIN_EXECUTABLE_FIELDS = {
    "byte_length",
    "sha256",
    "mode",
    "format",
    "architecture",
    "file_type",
}
CREBAIN_STAGE_INVENTORY_FIELDS = {
    "relative_path",
    "byte_length",
    "sha256",
    "mode",
    "role",
}
ENGRAM_PACK_RECEIPT_FIELDS = {
    "schema_version",
    "engram_repository",
    "engram_tool",
    "verification_policy",
    "operations",
    "observed_build_receipt_exact_sha256",
    "observed_build_receipt_sha256",
    "package_stage_receipt_exact_sha256",
    "package_stage_receipt_sha256",
    "seal_receipt_exact_sha256",
    "bundle_receipt_exact_sha256",
    "package_generation_id",
    "claims",
    "authority",
    "disclosure",
    "receipt_sha256",
}
ENGRAM_PACK_REPOSITORY_FIELDS = {
    "origin",
    "commit",
    "tree",
    "origin_main",
    "object_format",
    "clean",
}
ENGRAM_PACK_TOOL_FIELDS = {
    "relative_path",
    "size_bytes",
    "sha256",
    "git_mode",
    "git_blob",
}
ENGRAM_PACK_OPERATIONS = [
    {"operation": "pack", "exit_code": 0, "source_reverified": True},
    {"operation": "check", "exit_code": 0, "source_reverified": True},
]
ENGRAM_PACK_CLAIMS = {
    "local_pack_observed": True,
    "local_check_observed": True,
    "publisher_authenticated": False,
    "signature": False,
    "reproducible": False,
    "executed_tool_loaded_bytes_attested": False,
    "complete_python_environment_attested": False,
}
RECEIPT_STORE_CLOSURE_FIELDS = {
    "schema_version",
    "store_id",
    "receipt_sha256",
    "receipt_artifact_path",
    "evidence_bundle_sha256",
    "evidence_artifact_path",
    "file_count",
    "total_bytes",
    "files",
    "closure_sha256",
}
RECEIPT_STORE_SIDECAR_FIELDS = {
    "schema_version",
    "store_metadata",
    "finalized_reservation",
    "observation",
    "publication_admission_anchor",
    "publication_authority",
    "closure_sha256",
}
RECEIPT_STORE_METADATA_FIELDS = {
    "schema_version",
    "store_id",
    "policy",
    "digest_canonicalization",
    "execution_authority",
    "ncp_control",
    "physical_actuation",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
}
RECEIPT_STORE_FINALIZATION_FIELDS = {
    "schema_version",
    "store_id",
    "reservation",
    "pre_spawn_sha256",
    "extension_dispatch_sha256",
    "simulation_dispatch_sha256",
    "terminal_receipt_sha256",
    "evidence_bundle_sha256",
    "nest_work_admission_rejoined",
    "execution_authority",
    "ncp_control",
    "physical_actuation",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
    "finalization_sha256",
}
RECEIPT_STORE_RESERVATION_FIELDS = {
    "schema_version",
    "store_id",
    "reservation_id",
    "study_run_id",
    "closed_loop_definition_sha256",
    "receipt_profile",
    "evidence_profile",
    "nest_work_admission_sha256",
    "pre_spawn_sha256",
    "run_plan_sha256",
    "nest_configuration_sha256",
    "expected_runtime_binding_sha256",
    "reviewed_native_handshake_receipt_sha256",
    "reviewed_native_handshake",
    "package_generation_id",
    "runtime_generation_id",
    "reserved_record_count",
    "reserved_artifact_bytes",
    "reserved_evidence_bytes",
    "reserved_record_bytes",
    "execution_authority",
    "ncp_control",
    "physical_actuation",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
    "reservation_sha256",
}
RECEIPT_STORE_OBSERVATION_FIELDS = {
    "schema_version",
    "store_id",
    "artifact",
    "study_run_id",
    "run_status",
    "terminal_reason_code",
    "relative_artifact_path",
    "artifact_byte_length",
    "evidence_profile",
    "evidence_bundle_sha256",
    "relative_evidence_path",
    "evidence_byte_length",
    "admission_mode",
    "publication_authority_sha256",
    "reservation_id",
    "reservation_sha256",
    "reservation_finalization_sha256",
    "nest_work_admission_sha256",
    "nest_work_admission_rejoined",
    "digest_canonicalization",
    "execution_authority",
    "ncp_control",
    "physical_actuation",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
    "record_sha256",
}
RECEIPT_STORE_ADMISSION_ANCHOR_FIELDS = {
    "schema_version",
    "store_id",
    "study_run_key_sha256",
    "study_run_id",
    "terminal_receipt_sha256",
    "admission_mode",
    "publication_wal_sha256",
    "evidence_bundle_sha256",
    "reservation_id",
    "reservation_sha256",
    "pre_spawn_sha256",
    "extension_dispatch_sha256",
    "simulation_dispatch_sha256",
    "reservation_finalization_sha256",
    "execution_authority",
    "ncp_control",
    "physical_actuation",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
    "anchor_sha256",
}
RECEIPT_STORE_PUBLICATION_AUTHORITY_FIELDS = {
    "schema_version",
    "store_id",
    "terminal_receipt_sha256",
    "study_run_id",
    "admission_mode",
    "publication_admission_anchor_sha256",
    "publication_wal_sha256",
    "evidence_bundle_sha256",
    "reservation_id",
    "reservation_sha256",
    "reservation_finalization_sha256",
    "nest_work_admission_sha256",
    "execution_authority",
    "ncp_control",
    "physical_actuation",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
    "authority_sha256",
}
NON_AUTHORITY_FALSE_FIELDS = frozenset(
    {
        "agent_action_authority",
        "calibrated_posterior",
        "durable_process_launch_authority",
        "execution_authority",
        "is_paper_local_evidence",
        "music_transport_used",
        "ncp_authority",
        "ncp_control",
        "ncp_qualified",
        "ncp_transport",
        "ncp_transport_used",
        "physical_actuation",
        "physical_authority",
        "plant_control",
        "replayable_live_launch_authority",
        "scientific_authority",
    }
)
POPULATION_TOPOLOGY_FIELDS = {
    "session_count",
    "drone_count",
    "action_axis_count",
    "population_count",
    "population_neuron_count",
    "device_node_count",
    "connection_count",
    "population_names",
    "derived_population_roster_sha256",
}
SESSION_POPULATION_ROSTER_FIELDS = {"channel_id", "population_names"}
SESSION_CONTROL_BINDING_FIELDS = {
    "channel_id",
    "neural_codec_sha256",
    "axis_binding_sha256s",
}
SESSION_CONNECTION_READBACK_FIELDS = {
    "population_name",
    "direction",
    "connection_count",
    "synapse_model",
    "requested_weight",
    "effective_weight",
    "requested_delay_tics",
    "delay_api_argument_ms",
    "effective_delay_ms",
    "effective_delay_tics",
    "requested_receptor",
    "effective_receptor",
}
WORKER_GUARDIAN_CLOSURE_FIELDS = {
    "worker_session_binding_receipt_sha256",
    "worker_runtime_identity_receipt_sha256",
    "worker_lifecycle_receipt_sha256",
    "termination_attempt_count",
    "termination_attempt_roster_sha256",
    "worker_pid",
    "worker_source_sha256",
    "worker_command_sha256",
    "child_reaped",
    "containment_empty",
    "diagnostic_stream_complete",
}
REVIEWED_NATIVE_RUNTIME_FIELDS = {
    "handshake_receipt",
    "termination_receipt",
    "exec_gate_command_binding",
    "lifecycle_binding_sha256",
    "guardian_closure_verified",
    "package_store_lineage_verified",
}
REVIEWED_EXEC_GATE_COMMAND_FIELDS = {
    "schema_version",
    "python_executable_sha256",
    "exec_gate_source_sha256",
    "argument_shape",
    "target_command_sha256",
    "exec_gate_command_sha256",
}
SOURCE_CLOSURE_FIELDS = {
    "schema_version",
    "discovery_policy",
    "git",
    "host_modules",
    "worker_project_modules",
    "worker_project_source_roster_sha256",
    "reviewed_runtime_handshake_receipt_sha256",
    "reviewed_runtime_guardian_source_sha256",
    "reviewed_runtime_exec_gate_source_sha256",
    "reviewed_runtime_exec_gate_command_sha256",
    "exercised_entrypoints",
    "sources",
    "source_roster_sha256",
    "closure_sha256",
}
RUN_PLAN_FIELDS = {
    "schema_version",
    "study_run_id",
    "study_definition_sha256",
    "channels",
    "step_count",
    "timebase",
    "neural_step_timeout_ms",
    "cleanup_timeout_ms",
    "total_deadline_ms",
    "max_transcript_bytes",
    "advisory_proposal_sha256",
    "agent_action_authority",
    "simulator_only",
    "physical_actuation",
    "ncp_transport_used",
    "music_transport_used",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
}
CHANNEL_FIELDS = {
    "channel_id",
    "subject_id",
    "subject_kind",
    "observation_space_id",
    "observation_width",
    "observation_components",
    "action_space_id",
    "action_width",
    "action_components",
    "action_min",
    "action_max",
    "safe_action",
    "neural_population_prefix",
    "neural_control_axes",
}
COMPONENT_FIELDS = {"component_id", "unit_id"}
NEURAL_AXIS_FIELDS = {
    "action_index",
    "decoded_action_gain",
    "encoder",
    "terms",
}
NEURAL_TERM_FIELDS = {
    "observation_index",
    "gain_per_observation_unit",
    "reference_value",
}
SUMMARY_FIELDS = {
    "authority",
    "status",
    "study_run_id",
    "run_status",
    "terminal_reason_code",
    "planned_step_count",
    "completed_step_count",
    "channel_count",
    "store_id",
    "receipt_sha256",
    "reservation_id",
    "evidence_bundle_sha256",
    "simulator_only",
    "physical_actuation",
    "ncp_qualified",
    "scientific_authority",
    "calibrated_posterior",
}
CAPTURE_ASSERTION_FIELDS = {
    "fault_then_next_step_hold",
    "nest_hold_washout_and_reset_verified",
    "nest_recovery_washout_and_reset_verified",
    "resumed_nest_proposal_nonzero",
    "other_channels_never_entered_safety_mode",
    "terminal_receipt_and_neural_result_lineage_verified",
    "engram_host_and_worker_source_closure_verified",
    "reviewed_runtime_guardian_lineage_verified",
    "engram_commit_equals_local_origin_main",
    "private_frozen_run_inputs_used",
    "one_nest_session_exact_6n_population_topology_verified",
    "nest_worker_guardian_terminal_closure_verified",
    "receipt_store_artifact_closure_verified",
    "installed_generation_seal_package_bundle_store_lineage_verified",
}
CAPTURE_AUTHORITY_FIELDS = {
    "simulator_only",
    "ncp_qualified",
    "physical_actuation",
    "plant_control",
    "scientific_authority",
}
REVIEWED_HANDSHAKE_FIELDS = {
    "schema_version",
    "installation_id",
    "generation_id",
    "generation_ordinal",
    "extension_id",
    "extension_version",
    "target_id",
    "profile",
    "executable_sha256",
    "validator_set_sha256",
    "launch_source",
    "store_id",
    "package_generation_id",
    "package_generation_lease_retained",
    "generation_directory_identity_sha256",
    "host_handshake_frame_sha256",
    "runtime_handshake_frame_sha256",
    "handshake_transcript_accepted",
    "child_ready_claim",
    "host_local_admission",
    "process_launch_performed",
    "explicit_absolute_path_spawn",
    "exec_gate_command_sha256",
    "exec_gate_source_sha256",
    "path_lookup_at_spawn",
    "package_path_reopened_for_spawn",
    "verified_executable_staged",
    "staged_executable_owner_private",
    "staged_executable_user_immutable",
    "process_group_containment",
    "guardian_source_sha256",
    "guardian_command_sha256",
    "guardian_pid",
    "process_pid",
    "process_group_id",
    "session_id",
    "runtime_process_group_leader",
    "guardian_group_member",
    "guardian_ready_frame_sha256",
    "guardian_owner_loss_seal",
    "guardian_generation_lease_retained",
    "guardian_uncertainty_record_prepared",
    "descendant_creation_denied",
    "os_sandbox_enforced",
    "network_isolation_enforced",
    "filesystem_isolation_enforced",
    "sandbox_profile_sha256",
    "sandbox_launcher_sha256",
    "external_dependency_closure_attested",
    "automatic_restart",
    "publisher_authenticated",
    "durable_process_launch_authority",
    "replayable_live_launch_authority",
    "ncp_authority",
    "physical_authority",
    "scientific_authority",
    "receipt_sha256",
}
REVIEWED_TERMINATION_FIELDS = {
    "schema_version",
    "handshake_receipt_sha256",
    "generation_id",
    "disposition",
    "reason_code",
    "exit_code",
    "termination_signal",
    "child_reaped",
    "guardian_pid",
    "process_group_id",
    "guardian_reaped",
    "group_signal_while_guardian_unreaped",
    "direct_child_signal_while_unreaped",
    "containment_signal_scope",
    "containment_seal_signal",
    "containment_empty",
    "stderr_sha256",
    "stderr_retained_bytes",
    "stderr_truncated",
    "diagnostic_stream_complete",
    "private_work_directory_removed",
    "package_generation_lease_released",
    "guardian_generation_lease_held_until_containment",
    "durable_process_launch_authority",
    "ncp_authority",
    "physical_authority",
    "scientific_authority",
    "receipt_sha256",
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
CLEANUP_FIELDS = {
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
TERMINAL_FIELDS = {
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
NEST_BUNDLE_FIELDS = {
    "schema_version",
    "digest_canonicalization",
    "profile",
    "study_run_id",
    "run_receipt_sha256",
    "neural_provider_identity_sha256",
    "neural_preparation_sha256",
    "runtime_launch_expectation",
    "worker_launch_attempt",
    "preparation_attempt",
    "child_capabilities",
    "child_preparation_receipt",
    "provider_preparation_receipt",
    "nest_session_readback",
    "step_attempt_receipts",
    "step_execution_receipts",
    "worker_termination_attempt_receipts",
    "tail_disposition_receipt",
    "worker_lifecycle_receipt",
    "worker_terminal_disposition",
    "worker_runtime_identity",
    "worker_session_binding",
    "execution_authority",
    "ncp_control",
    "physical_actuation",
    "scientific_authority",
    "is_paper_local_evidence",
    "calibrated_posterior",
    "bundle_sha256",
}
RUNTIME_LAUNCH_EXPECTATION_FIELDS = {
    "adapter_source_sha256",
    "address_space_bytes",
    "address_space_limit_enforced",
    "child_provider_test_failure_phase",
    "controller_configuration",
    "core_file_bytes",
    "cpu_time_seconds",
    "darwin_sandbox_launcher_sha256",
    "darwin_sandbox_profile_sha256",
    "descendant_creation_denied",
    "environment",
    "exec_gate_source_file",
    "exec_gate_source_sha256",
    "expected_child_provider_identity_sha256",
    "external_dependency_closure_attested",
    "file_size_bytes",
    "guardian_command",
    "guardian_group_member",
    "guardian_source_file",
    "guardian_source_sha256",
    "loaded_bytes_attested",
    "network_namespace_isolation",
    "open_file_count",
    "platform",
    "production_isolation",
    "project_source_discovery_policy",
    "python_executable_sha256",
    "receipt_sha256",
    "required_project_source_roster_sha256",
    "required_runtime_file_roster_sha256",
    "required_runtime_files",
    "resource_limit_profile",
    "runtime_process_group_leader",
    "schema_version",
    "session_escape_prevention_profile",
    "sys_path",
    "syscall_filter",
    "worker_command",
    "worker_command_sha256",
    "worker_dispatch_command",
    "worker_source_sha256",
}
RUNTIME_FILE_FIELDS = {"role", "absolute_path", "sha256", "size_bytes"}
WORKER_LAUNCH_ATTEMPT_FIELDS = {
    "anchored_group_kill_delivered",
    "bounded_cleanup_observation_complete",
    "containment_empty",
    "containment_seal_signal",
    "group_signal_attempted",
    "group_signal_basis",
    "guardian_pid",
    "guardian_ready_observed",
    "guardian_reaped",
    "guardian_started",
    "launch_expectation_sha256",
    "outcome",
    "phase",
    "posix_process_group_portability_scope",
    "process_group_id",
    "production_isolation",
    "reason_code",
    "receipt_sha256",
    "schema_version",
    "scientific_authority",
    "session_id",
    "stderr_drain_started",
    "worker_pid",
    "worker_reaped",
    "worker_started",
}
PREPARATION_ATTEMPT_FIELDS = {
    "definition_sha256",
    "outcome",
    "phase",
    "provider_preparation_receipt_sha256",
    "reason_code",
    "receipt_sha256",
    "runtime_identity_receipt_sha256",
    "runtime_launch_expectation_sha256",
    "schema_version",
    "scientific_authority",
    "session_binding_receipt_sha256",
    "study_run_id",
    "worker_launch_attempt_sha256",
    "worker_request_dispatched",
    "worker_response_observed",
}
CHILD_CAPABILITIES_FIELDS = {
    "automatic_restart",
    "deadline_enforcement",
    "declared_step_duration_tics",
    "durable_evidence_profile",
    "loaded_bytes_attested",
    "max_channels",
    "ncp_transport",
    "physical_actuation",
    "provider",
    "provider_identity_sha256",
    "schema_version",
    "session_model",
}
PREPARATION_RECEIPT_FIELDS = {
    "definition_sha256",
    "populations",
    "provider_identity_sha256",
    "provider_session_receipt_sha256",
    "receipt_sha256",
    "schema_version",
    "single_session",
    "step_duration_tics",
    "study_run_id",
}
STEP_ATTEMPT_FIELDS = {
    "attempt_index",
    "before_biological_time_tics",
    "decoded_proposal_produced",
    "execution_receipt_sha256",
    "observation_scope",
    "observed_after_biological_time_tics",
    "outcome",
    "partial_readback_sha256",
    "reason_code",
    "receipt_sha256",
    "request_sha256",
    "requested_run_tics",
    "schema_version",
    "scientific_authority",
    "simulation_dispatched",
    "simulation_returned",
    "step_index",
}
NEURAL_STEP_FIELDS = {"request", "result"}
NEURAL_REQUEST_FIELDS = {
    "schema_version",
    "study_run_id",
    "step_index",
    "step_id",
    "observation_runtime_time_tics",
    "runtime_interval_tics",
    "runtime_interval_end_time_tics",
    "controller_start_time_tics",
    "controller_interval_tics",
    "controller_end_time_tics",
    "source_snapshot_sha256",
    "neural_preparation_sha256",
    "channels",
    "request_sha256",
}
NEURAL_RESULT_FIELDS = {
    "schema_version",
    "study_run_id",
    "step_index",
    "step_id",
    "controller_start_time_tics",
    "controller_end_time_tics",
    "request_sha256",
    "proposals",
    "provider_execution_scope",
    "provider_execution_sha256",
    "result_sha256",
}
NEURAL_CHANNEL_FIELDS = {
    "channel_id",
    "subject_id",
    "observation_values",
    "fault_code",
    "hold_required",
}
NEURAL_PROPOSAL_FIELDS = {"channel_id", "values", "source_populations"}
NEST_CONFIG_FIELDS = {
    "schema_version",
    "resolution_ms",
    "step_duration_ms",
    "population_size",
    "baseline_rate_hz",
    "input_span_hz",
    "output_rate_scale_hz",
    "input_weight_mv",
    "rng_seed",
}


class MatrixError(ValueError):
    """A fail-closed matrix review error."""


def closed_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise MatrixError(f"duplicate JSON member: {key}")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise MatrixError(f"non-finite JSON constant: {value}")


def canonical(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def invalid_managed_runtime_unicode(character: str) -> bool:
    codepoint = ord(character)
    if 0xD800 <= codepoint <= 0xDFFF or codepoint == 0xFFFD:
        return True
    if 0xFDD0 <= codepoint <= 0xFDEF or (codepoint & 0xFFFF) in {0xFFFE, 0xFFFF}:
        return True
    if codepoint < 0x20 and character not in {"\t", "\n", "\r"}:
        return True
    return 0x7F <= codepoint <= 0x9F


def validate_managed_runtime_domain(value: Any) -> None:
    active: set[int] = set()
    nodes = 0

    def visit(current: Any, depth: int) -> None:
        nonlocal nodes
        nodes += 1
        if nodes > MANAGED_RUNTIME_MAX_NODES or depth > MANAGED_RUNTIME_MAX_DEPTH:
            raise MatrixError("managed-runtime JSON exceeds its bounds")
        if current is None or type(current) is bool:
            return
        if type(current) is str:
            if any(invalid_managed_runtime_unicode(item) for item in current):
                raise MatrixError("managed-runtime JSON contains nonportable Unicode")
            return
        if type(current) is int:
            if abs(current) > MANAGED_RUNTIME_MAX_SAFE_INTEGER:
                raise MatrixError(
                    "managed-runtime JSON integer exceeds the exact range"
                )
            return
        if type(current) is float:
            if (
                not math.isfinite(current)
                or abs(current) > MANAGED_RUNTIME_MAX_FLOAT_ABS
                or (current == 0.0 and math.copysign(1.0, current) < 0.0)
            ):
                raise MatrixError(
                    "managed-runtime JSON number exceeds the portable finite range"
                )
            return
        is_mapping = isinstance(current, Mapping)
        is_sequence = isinstance(current, Sequence) and not isinstance(
            current,
            (str, bytes, bytearray),
        )
        if not is_mapping and not is_sequence:
            raise MatrixError(
                f"unsupported managed-runtime JSON value: {type(current).__name__}"
            )
        identity = id(current)
        if identity in active:
            raise MatrixError("managed-runtime JSON contains a cycle")
        active.add(identity)
        try:
            if is_mapping:
                for key, child in current.items():
                    if type(key) is not str:
                        raise MatrixError(
                            "managed-runtime JSON object key is not a string"
                        )
                    visit(key, depth + 1)
                    visit(child, depth + 1)
            else:
                for child in current:
                    visit(child, depth + 1)
        finally:
            active.remove(identity)

    visit(value, 1)


def managed_runtime_float_text(value: float) -> str:
    if type(value) is not float:
        raise TypeError("managed-runtime float rendering requires a float")
    if (
        not math.isfinite(value)
        or abs(value) > MANAGED_RUNTIME_MAX_FLOAT_ABS
        or (value == 0.0 and math.copysign(1.0, value) < 0.0)
    ):
        raise MatrixError("managed-runtime float exceeds the portable finite range")
    negative = value < 0.0
    source = repr(abs(value)).lower()
    if "e" in source:
        mantissa, exponent_text = source.split("e", 1)
        exponent = int(exponent_text)
        digits = mantissa.replace(".", "").lstrip("0").rstrip("0") or "0"
        decimal_point = exponent + 1
    else:
        integer, dot, fraction = source.partition(".")
        combined = integer + (fraction if dot else "")
        first = next(
            (index for index, character in enumerate(combined) if character != "0"),
            None,
        )
        if first is None:
            return "0.0"
        decimal_point = len(integer) - first
        digits = combined[first:].rstrip("0")
    length = len(digits)
    trailing_zero_count = decimal_point - length
    if 0 <= trailing_zero_count and decimal_point <= 16:
        rendered = digits + ("0" * trailing_zero_count) + ".0"
    elif 0 < decimal_point <= 16:
        rendered = digits[:decimal_point] + "." + digits[decimal_point:]
    elif -5 < decimal_point <= 0:
        rendered = "0." + ("0" * (-decimal_point)) + digits
    else:
        exponent = decimal_point - 1
        exponent_text = f"+{exponent}" if exponent >= 0 else str(exponent)
        if length == 1:
            rendered = f"{digits}e{exponent_text}"
        else:
            rendered = f"{digits[0]}.{digits[1:]}e{exponent_text}"
    return f"-{rendered}" if negative else rendered


def managed_runtime_canonical(value: Any) -> bytes:
    """Encode one value under Engram's Host API 2 canonical JSON profile."""

    validate_managed_runtime_domain(value)

    def encode(current: Any) -> bytes:
        if current is None:
            return b"null"
        if current is True:
            return b"true"
        if current is False:
            return b"false"
        if type(current) is int:
            return str(current).encode("ascii")
        if type(current) is float:
            return managed_runtime_float_text(current).encode("ascii")
        if type(current) is str:
            return json.dumps(
                current,
                ensure_ascii=False,
                allow_nan=False,
                separators=(",", ":"),
            ).encode("utf-8")
        if isinstance(current, Mapping):
            members = (
                encode(key) + b":" + encode(current[key]) for key in sorted(current)
            )
            return b"{" + b",".join(members) + b"}"
        return b"[" + b",".join(encode(child) for child in current) + b"]"

    return encode(value)


def digest_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def digest_without(value: dict[str, Any], field: str) -> str:
    return digest_bytes(
        canonical({key: item for key, item in value.items() if key != field})
    )


def managed_runtime_digest(value: Any) -> str:
    return digest_bytes(managed_runtime_canonical(value))


def managed_runtime_digest_without(value: dict[str, Any], field: str) -> str:
    return managed_runtime_digest(
        {key: item for key, item in value.items() if key != field}
    )


def valid_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in HEX for character in value)
    )


def valid_git_oid(value: Any, object_format: str | None = None) -> bool:
    return valid_git_object(value, object_format)


def valid_prefixed_sha256(value: Any, prefix: str) -> bool:
    return (
        isinstance(value, str)
        and value.startswith(prefix)
        and valid_sha256(value[len(prefix) :])
    )


def require_keys(value: Any, fields: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != fields:
        observed = sorted(value) if isinstance(value, dict) else type(value).__name__
        raise MatrixError(f"{label} field roster differs: {observed}")
    return value


def require_canonical_digest(
    value: dict[str, Any],
    field: str,
    label: str,
) -> str:
    reported = value.get(field)
    if not valid_sha256(reported) or reported != digest_without(value, field):
        raise MatrixError(f"{label} canonical digest differs")
    return reported


def require_managed_runtime_digest(
    value: dict[str, Any],
    field: str,
    label: str,
) -> str:
    reported = value.get(field)
    if not valid_sha256(reported) or reported != managed_runtime_digest_without(
        value, field
    ):
        raise MatrixError(f"{label} managed-runtime digest differs")
    return reported


def reject_recursive_authority(value: Any, label: str) -> None:
    """Reject any authority-bearing boolean in retained sidecar material."""

    pending = [value]
    observed_nodes = 0
    while pending:
        current = pending.pop()
        observed_nodes += 1
        if observed_nodes > 1_000_000:
            raise MatrixError(f"{label} exceeds the authority-audit node bound")
        if isinstance(current, dict):
            for key, child in current.items():
                if key in NON_AUTHORITY_FALSE_FIELDS and child is not False:
                    raise MatrixError(f"{label} grants or implies authority")
                if key == "simulator_only" and child is not True:
                    raise MatrixError(f"{label} contradicts simulator-only scope")
                if (
                    key == "authority"
                    and isinstance(child, bool)
                    and child is not False
                ):
                    raise MatrixError(f"{label} grants generic authority")
                pending.append(child)
        elif isinstance(current, list):
            pending.extend(current)


def require_list(value: Any, label: str, *, minimum: int, maximum: int) -> list[Any]:
    if not isinstance(value, list) or not minimum <= len(value) <= maximum:
        raise MatrixError(f"{label} length differs")
    return value


def measure_json(value: Any) -> None:
    nodes = 0
    stack: list[tuple[Any, int]] = [(value, 1)]
    while stack:
        item, depth = stack.pop()
        nodes += 1
        if depth > MAX_JSON_DEPTH or nodes > MAX_JSON_NODES:
            raise MatrixError("JSON structure exceeds the matrix review bound")
        if isinstance(item, dict):
            stack.extend((child, depth + 1) for child in item.values())
        elif isinstance(item, list):
            stack.extend((child, depth + 1) for child in item)


def load_json_payload(payload: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(
            payload,
            object_pairs_hook=closed_object,
            parse_constant=reject_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise MatrixError(f"{label} is not strict JSON: {error}") from error
    if not isinstance(value, dict):
        raise MatrixError(f"{label} root is not an object")
    measure_json(value)
    return value


def validate_managed_runtime_canonicalizer_contract() -> None:
    contract = load_json_payload(
        snapshot_regular_file(MANAGED_RUNTIME_FLOAT_CONTRACT, MAX_SCHEMA_BYTES),
        "managed-runtime finite-float contract",
    )
    require_keys(
        contract,
        {"schema_version", "canonicalizer", "cases", "randomized"},
        "managed-runtime finite-float contract",
    )
    cases = require_list(
        contract["cases"],
        "managed-runtime finite-float cases",
        minimum=25,
        maximum=25,
    )
    case_ids: list[str] = []
    for item in cases:
        case = require_keys(
            item,
            {"id", "binary64_be_hex", "portable", "canonical_json"},
            "managed-runtime finite-float case",
        )
        case_id = case["id"]
        binary64 = case["binary64_be_hex"]
        portable = case["portable"]
        expected = case["canonical_json"]
        if (
            not isinstance(case_id, str)
            or not case_id
            or not isinstance(binary64, str)
            or len(binary64) != 16
            or any(character not in HEX for character in binary64)
            or not isinstance(portable, bool)
            or (
                expected is not None and (not isinstance(expected, str) or not expected)
            )
        ):
            raise MatrixError("managed-runtime finite-float case differs")
        value = struct.unpack(">d", bytes.fromhex(binary64))[0]
        if portable:
            if (
                expected is None
                or managed_runtime_float_text(value) != expected
                or managed_runtime_canonical(value) != expected.encode("ascii")
            ):
                raise MatrixError(
                    f"managed-runtime finite-float spelling differs: {case_id}"
                )
        else:
            try:
                managed_runtime_canonical(value)
            except MatrixError:
                pass
            else:
                raise MatrixError(
                    f"managed-runtime finite-float case was accepted: {case_id}"
                )
        case_ids.append(case_id)
    randomized = require_keys(
        contract["randomized"],
        {
            "algorithm",
            "seed_hex",
            "sample_count",
            "accepted_count",
            "transcript",
            "transcript_sha256",
        },
        "managed-runtime randomized corpus identity",
    )
    if (
        contract["schema_version"] != "engram.managed-runtime-finite-float.v1"
        or contract["canonicalizer"] != RECEIPT_STORE_CANONICALIZATION
        or case_ids != list(dict.fromkeys(case_ids))
        or randomized["algorithm"] != "splitmix64-v1"
        or randomized["seed_hex"] != "656e6772616d7631"
        or randomized["sample_count"] != 4096
        or randomized["accepted_count"] != 4030
        or not valid_sha256(randomized["transcript_sha256"])
    ):
        raise MatrixError("managed-runtime finite-float contract identity differs")
    threshold = {"threshold": 1.0e-6}
    if managed_runtime_canonical(threshold) != b'{"threshold":1e-6}' or canonical(
        threshold
    ) == managed_runtime_canonical(threshold):
        raise MatrixError("managed-runtime and ledger JSON domains were collapsed")


def safe_relative(
    value: Any, label: str, *, suffix: str | None = None
) -> PurePosixPath:
    if not isinstance(value, str) or not value or "\\" in value or "\0" in value:
        raise MatrixError(f"{label} is not a canonical relative path")
    relative = PurePosixPath(value)
    if (
        relative.is_absolute()
        or relative.as_posix() != value
        or any(part in {"", ".", ".."} for part in relative.parts)
        or (suffix is not None and relative.suffix != suffix)
    ):
        raise MatrixError(f"{label} is not a canonical relative path")
    return relative


def repository_path(root: Path, relative: PurePosixPath, label: str) -> Path:
    target = root.joinpath(*relative.parts)
    if target.parent.resolve(strict=True) != target.parent:
        raise MatrixError(f"{label} parent traverses a link")
    return target


def git_output(root: Path, *arguments: str) -> bytes:
    environment = {
        key: value for key, value in os.environ.items() if not key.startswith("GIT_")
    }
    environment.update(
        {
            "GIT_NO_REPLACE_OBJECTS": "1",
            "GIT_OPTIONAL_LOCKS": "0",
            "GIT_TERMINAL_PROMPT": "0",
            "LC_ALL": "C",
        }
    )
    completed = subprocess.run(
        [
            "git",
            "--no-replace-objects",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            *arguments,
        ],
        cwd=root,
        env=environment,
        stdin=subprocess.DEVNULL,
        capture_output=True,
        check=False,
        timeout=30,
    )
    if (
        completed.returncode != 0
        or completed.stderr
        or len(completed.stdout) > MAX_GIT_OUTPUT_BYTES
    ):
        raise MatrixError("Git identity query failed")
    return completed.stdout


def verify_repository(root: Path, expected_revision: str, label: str) -> dict[str, Any]:
    try:
        resolved = root.resolve(strict=True)
        if root.absolute() != resolved:
            raise ValueError("repository path traverses a link")
        identity = capture_repository_identity(resolved, expected_revision)
    except (OSError, ValueError) as error:
        raise MatrixError(
            f"{label} is not clean at the required pushed main revision"
        ) from error
    return {"root": resolved, **identity}


def check_false_authority(authority: Any, fields: set[str], label: str) -> None:
    value = require_keys(authority, fields, label)
    if value.get("simulator_only") is not True:
        raise MatrixError(f"{label} is not simulator-only")
    if any(item is not False for key, item in value.items() if key != "simulator_only"):
        raise MatrixError(f"{label} grants authority")


def require_index_schema_version(index: dict[str, Any]) -> None:
    observed = index.get("schema_version")
    if observed == AUDIT_ONLY_INDEX_SCHEMA_VERSION:
        raise MatrixError("CREBAIN evidence-index v1 is historical and audit-only")
    if observed != INDEX_SCHEMA_VERSION:
        raise MatrixError(f"unsupported CREBAIN evidence-index schema: {observed}")


def validate_index_shape(index: dict[str, Any]) -> None:
    require_index_schema_version(index)
    require_keys(index, INDEX_FIELDS, "CREBAIN evidence index")
    require_keys(
        index["input_suite"],
        {
            "schema_version",
            "exact_sha256",
            "suite_definition_sha256",
            "nest_config_exact_sha256",
        },
        "CREBAIN input-suite index",
    )
    tool_closure = require_keys(
        index["tool_source_closure"],
        {"schema_version", "files", "roster_sha256"},
        "CREBAIN tool-source closure",
    )
    for row in require_list(
        tool_closure["files"],
        "CREBAIN tool-source roster",
        minimum=len(EXPECTED_CREBAIN_TOOL_SOURCES),
        maximum=len(EXPECTED_CREBAIN_TOOL_SOURCES),
    ):
        require_keys(row, {"role", "path", "exact_sha256"}, "CREBAIN tool source")
    require_keys(
        index["crebain_source_repository"],
        {
            "repository",
            "commit",
            "tree",
            "origin_main_at_capture",
            "object_format",
            "clean_at_capture",
        },
        "CREBAIN evidence-index source repository",
    )
    require_keys(
        index["engram"],
        {"repository", "commit", "tree", "origin_main", "object_format", "clean"},
        "CREBAIN evidence-index Engram identity",
    )
    require_keys(index["package"], INDEX_PACKAGE_FIELDS, "CREBAIN package index")
    require_keys(
        index["assertions"],
        INDEX_ASSERTION_FIELDS,
        "CREBAIN evidence-index assertions",
    )
    require_keys(
        index["authority"],
        CAPTURE_AUTHORITY_FIELDS,
        "CREBAIN evidence-index authority",
    )
    rows = require_list(
        index["captures"],
        "CREBAIN capture index",
        minimum=3,
        maximum=3,
    )
    for row in rows:
        require_keys(row, INDEX_CAPTURE_FIELDS, "CREBAIN capture index row")


def validate_nest_config(config: Any) -> dict[str, Any]:
    value = require_keys(
        config,
        NEST_CONFIG_FIELDS,
        "CREBAIN NEST controller configuration",
    )
    positive_numeric_fields = (
        "resolution_ms",
        "step_duration_ms",
        "population_size",
        "baseline_rate_hz",
        "input_span_hz",
        "output_rate_scale_hz",
        "input_weight_mv",
    )
    if (
        value["schema_version"] != "engram.nest-population-controller-config.v2"
        or any(
            isinstance(value[field], bool)
            or not isinstance(value[field], (int, float))
            or not math.isfinite(float(value[field]))
            or value[field] <= 0
            for field in positive_numeric_fields
        )
        or not isinstance(value["population_size"], int)
        or isinstance(value["rng_seed"], bool)
        or not isinstance(value["rng_seed"], int)
        or not 0 <= value["rng_seed"] <= 2**63 - 1
    ):
        raise MatrixError("CREBAIN NEST controller identity differs")
    return value


def validate_tool_source_closure(
    value: Any,
    expected_crebain: dict[str, Any],
) -> dict[str, Any]:
    """Reopen every indexed CREBAIN tool from the bound Git revision."""

    closure = require_keys(
        value,
        {"schema_version", "files", "roster_sha256"},
        "CREBAIN tool-source closure",
    )
    rows = require_list(
        closure["files"],
        "CREBAIN tool-source roster",
        minimum=len(EXPECTED_CREBAIN_TOOL_SOURCES),
        maximum=len(EXPECTED_CREBAIN_TOOL_SOURCES),
    )
    expected_rows: list[dict[str, Any]] = []
    ordered_tools = sorted(EXPECTED_CREBAIN_TOOL_SOURCES.items())
    committed_sources = capture_repository_files(
        expected_crebain["root"],
        expected_crebain["commit"],
        tuple(Path(path) for path, _role in ordered_tools),
        MAX_SOURCE_BYTES,
        checkout_revision=expected_crebain["checkout_revision"],
    )
    for (expected_path, expected_role), source in zip(
        ordered_tools,
        committed_sources,
        strict=True,
    ):
        expected_rows.append(
            {
                "role": expected_role,
                "path": expected_path,
                "exact_sha256": source["sha256"],
            }
        )
    if (
        closure["schema_version"] != "crebain.real-nest-tool-source-closure.v1"
        or rows != expected_rows
        or closure["roster_sha256"] != digest_bytes(canonical(rows))
    ):
        raise MatrixError("CREBAIN tool-source closure differs")
    return {
        "document": closure,
        "files": rows,
        "committed_sources": committed_sources,
        "roster_sha256": closure["roster_sha256"],
    }


def validate_input_suite(
    row: Any,
    crebain_root: Path,
) -> dict[str, Any]:
    index_suite = require_keys(
        row,
        {
            "schema_version",
            "exact_sha256",
            "suite_definition_sha256",
            "nest_config_exact_sha256",
        },
        "CREBAIN input-suite index",
    )
    suite_path = repository_path(
        crebain_root,
        INPUT_SUITE_RELATIVE,
        "CREBAIN input suite",
    )
    suite_payload = snapshot_regular_file(suite_path, MAX_INDEX_BYTES)
    suite = load_json_payload(suite_payload, "CREBAIN input suite")
    require_keys(suite, INPUT_SUITE_FIELDS, "CREBAIN input suite")
    definition = {
        key: value for key, value in suite.items() if key != "suite_definition_sha256"
    }
    if (
        suite["schema_version"] != "crebain.real-nest-operational-input-suite.v1"
        or suite["profile"] != "installed-crebain-standard-v3-real-nest-3.9"
        or suite["capture_schema_version"] != CAPTURE_SCHEMA_VERSION
        or not valid_sha256(suite["suite_definition_sha256"])
        or suite["suite_definition_sha256"] != digest_bytes(canonical(definition))
        or index_suite["schema_version"] != suite["schema_version"]
        or index_suite["exact_sha256"] != digest_bytes(suite_payload)
        or index_suite["suite_definition_sha256"] != suite["suite_definition_sha256"]
        or not valid_sha256(index_suite["nest_config_exact_sha256"])
    ):
        raise MatrixError("CREBAIN input-suite identity differs")
    check_false_authority(
        suite["authority"],
        CAPTURE_AUTHORITY_FIELDS,
        "CREBAIN input-suite authority",
    )
    constraints = require_keys(
        suite["constraints"],
        {
            "session_count_per_run",
            "action_axes_per_drone",
            "signed_populations_per_action_axis",
            "population_size",
            "ncp_transport_used",
            "music_transport_used",
            "simulator_only",
        },
        "CREBAIN input-suite constraints",
    )
    population_size = constraints["population_size"]
    if (
        constraints.get("session_count_per_run") != 1
        or constraints.get("action_axes_per_drone") != 3
        or constraints.get("signed_populations_per_action_axis") != 2
        or isinstance(population_size, bool)
        or not isinstance(population_size, int)
        or population_size < 1
        or constraints.get("ncp_transport_used") is not False
        or constraints.get("music_transport_used") is not False
        or constraints.get("simulator_only") is not True
    ):
        raise MatrixError("CREBAIN input-suite constraints differ")
    config_row = require_keys(
        suite["nest_config"],
        {"path", "exact_sha256"},
        "CREBAIN input-suite NEST configuration",
    )
    config_relative = safe_relative(
        config_row["path"],
        "CREBAIN input-suite NEST configuration path",
        suffix=".json",
    )
    if len(config_relative.parts) != 1 or config_relative.name != "nest-config.json":
        raise MatrixError("CREBAIN input-suite NEST configuration path differs")
    config_path = repository_path(
        suite_path.parent,
        config_relative,
        "CREBAIN input-suite NEST configuration",
    )
    config_payload = snapshot_regular_file(config_path, MAX_INDEX_BYTES)
    config = validate_nest_config(
        load_json_payload(config_payload, "CREBAIN input-suite NEST configuration")
    )
    config_sha256 = digest_bytes(config_payload)
    if (
        config_row["exact_sha256"] != config_sha256
        or index_suite["nest_config_exact_sha256"] != config_sha256
        or config["population_size"] != population_size
    ):
        raise MatrixError("CREBAIN input-suite NEST configuration differs")
    runs = require_list(suite["runs"], "CREBAIN input-suite runs", minimum=3, maximum=3)
    by_count: dict[int, dict[str, Any]] = {}
    for item in runs:
        run = require_keys(item, INPUT_SUITE_RUN_FIELDS, "CREBAIN input-suite run")
        drone_count = run["drone_count"]
        if isinstance(drone_count, bool) or drone_count not in {1, 2, 3}:
            raise MatrixError("CREBAIN input-suite drone count differs")
        plan_relative = safe_relative(
            run["plan_path"],
            "CREBAIN input-suite run-plan path",
            suffix=".json",
        )
        expected_name = (
            f"run-plan-{drone_count}-drone{'s' if drone_count > 1 else ''}.json"
        )
        if len(plan_relative.parts) != 1 or plan_relative.name != expected_name:
            raise MatrixError("CREBAIN input-suite run-plan path differs")
        plan_path = repository_path(
            suite_path.parent,
            plan_relative,
            "CREBAIN input-suite run plan",
        )
        plan_payload = snapshot_regular_file(plan_path, MAX_INDEX_BYTES)
        plan = load_json_payload(plan_payload, "CREBAIN input-suite run plan")
        roster = channel_roster(plan, drone_count)
        population_count = drone_count * 6
        if (
            drone_count in by_count
            or run["plan_exact_sha256"] != digest_bytes(plan_payload)
            or run["expected_channel_ids"] != roster["channel_ids"]
            or roster["action_dimension_count"] != drone_count * 3
            or run["expected_population_count"] != population_count
            or run["expected_population_neuron_count"]
            != population_count * population_size
            or run["expected_device_node_count"] != population_count * 2
            or run["expected_connection_count"]
            != population_count * population_size * 2
            or run["expected_step_count"] != 6
            or plan["step_count"] != 6
        ):
            raise MatrixError("CREBAIN input-suite run topology differs")
        by_count[drone_count] = {
            "row": run,
            "plan": plan,
            "plan_exact_sha256": digest_bytes(plan_payload),
        }
    if list(by_count) != [1, 2, 3]:
        raise MatrixError("CREBAIN input suite does not contain ordered 1/2/3 drones")
    return {
        "document": suite,
        "exact_sha256": digest_bytes(suite_payload),
        "config": config,
        "config_exact_sha256": config_sha256,
        "runs": by_count,
    }


def validate_index(
    index: dict[str, Any],
    expected_crebain: dict[str, Any],
    expected_engram: dict[str, Any],
) -> dict[str, Any]:
    validate_index_shape(index)
    if index["profile"] != "installed-crebain-standard-v3-real-nest-3.9":
        raise MatrixError("CREBAIN evidence-index profile differs")
    input_suite = validate_input_suite(index["input_suite"], expected_crebain["root"])
    if input_suite["document"]["profile"] != index["profile"]:
        raise MatrixError("CREBAIN evidence index and input suite profiles differ")
    tool_sources = validate_tool_source_closure(
        index["tool_source_closure"],
        expected_crebain,
    )
    index_crebain = require_keys(
        index["crebain_source_repository"],
        {
            "repository",
            "commit",
            "tree",
            "origin_main_at_capture",
            "object_format",
            "clean_at_capture",
        },
        "CREBAIN evidence-index source repository",
    )
    if index_crebain != {
        "repository": expected_crebain["repository"],
        "commit": expected_crebain["commit"],
        "tree": expected_crebain["tree"],
        "origin_main_at_capture": expected_crebain["commit"],
        "object_format": expected_crebain["object_format"],
        "clean_at_capture": True,
    }:
        raise MatrixError("CREBAIN evidence index binds a different source revision")
    index_engram = require_keys(
        index["engram"],
        {"repository", "commit", "tree", "origin_main", "object_format", "clean"},
        "CREBAIN evidence-index Engram identity",
    )
    if index_engram != {
        "repository": expected_engram["repository"],
        "commit": expected_engram["commit"],
        "tree": expected_engram["tree"],
        "origin_main": expected_engram["origin_main"],
        "object_format": expected_engram["object_format"],
        "clean": True,
    }:
        raise MatrixError("CREBAIN evidence index binds a different Engram revision")
    package = require_keys(
        index["package"], INDEX_PACKAGE_FIELDS, "CREBAIN package index"
    )
    package_non_digest_fields = {
        "store_id",
        "package_generation_id",
        "installation_id",
        "crebain_commit",
        "crebain_tree",
        "crebain_origin_main",
        "engram_commit",
        "engram_tree",
        "engram_origin_main",
        "engram_extension_tool_git_blob",
        "executable_format",
        "executable_architecture",
        "build_stage_seal_pack_install_lineage_verified",
    }
    if (
        not valid_prefixed_sha256(package["store_id"], "extstore_")
        or not valid_prefixed_sha256(package["package_generation_id"], "pkggen_")
        or not valid_prefixed_sha256(package["installation_id"], "inst_")
        or any(
            not valid_sha256(package[field])
            for field in INDEX_PACKAGE_FIELDS - package_non_digest_fields
        )
        or package["crebain_commit"] != expected_crebain["commit"]
        or package["crebain_tree"] != expected_crebain["tree"]
        or package["crebain_origin_main"] != expected_crebain["origin_main"]
        or package["engram_commit"] != expected_engram["commit"]
        or package["engram_tree"] != expected_engram["tree"]
        or package["engram_origin_main"] != expected_engram["origin_main"]
        or not valid_git_oid(
            package["engram_extension_tool_git_blob"],
            expected_engram["object_format"],
        )
        or package["executable_format"] != "mach-o-64"
        or package["executable_architecture"] != "arm64"
        or package["build_stage_seal_pack_install_lineage_verified"] is not True
        or not valid_sha256(index["installed_package_proof_exact_sha256"])
    ):
        raise MatrixError("CREBAIN package index identity differs")
    required_boolean_fields(
        index["assertions"],
        INDEX_ASSERTION_FIELDS,
        "CREBAIN evidence-index assertions",
    )
    check_false_authority(
        index["authority"],
        CAPTURE_AUTHORITY_FIELDS,
        "CREBAIN evidence-index authority",
    )
    if (
        not isinstance(index["disclosure"], str)
        or not index["disclosure"]
        or len(index["disclosure"].encode("utf-8")) > 512
    ):
        raise MatrixError("CREBAIN evidence-index disclosure differs")
    rows = require_list(
        index["captures"], "CREBAIN capture index", minimum=3, maximum=3
    )
    by_count: dict[int, dict[str, Any]] = {}
    for item in rows:
        row = require_keys(item, INDEX_CAPTURE_FIELDS, "CREBAIN capture index row")
        drone_count = row["drone_count"]
        suite_run = input_suite["runs"].get(drone_count)
        if (
            isinstance(drone_count, bool)
            or drone_count not in {1, 2, 3}
            or drone_count in by_count
            or row["path"] != CAPTURE_PATHS[drone_count]
            or suite_run is None
            or row["plan_exact_sha256"] != suite_run["plan_exact_sha256"]
            or not valid_prefixed_sha256(row["receipt_store_id"], "clrs_")
            or row["population_count"] != drone_count * 6
            or row["population_neuron_count"]
            != suite_run["row"]["expected_population_neuron_count"]
            or row["device_node_count"]
            != suite_run["row"]["expected_device_node_count"]
            or row["connection_count"] != suite_run["row"]["expected_connection_count"]
            or row["session_count"] != 1
            or any(
                not valid_sha256(row[field])
                for field in (
                    "capture_sha256",
                    "plan_exact_sha256",
                    "receipt_sha256",
                    "evidence_bundle_sha256",
                    "receipt_store_closure_sha256",
                    "engram_source_closure_sha256",
                    "engram_source_roster_sha256",
                    "observed_build_receipt_exact_sha256",
                )
            )
        ):
            raise MatrixError("CREBAIN capture index row differs")
        by_count[drone_count] = row
    if list(by_count) != [1, 2, 3]:
        raise MatrixError("CREBAIN capture index does not contain ordered 1/2/3 drones")
    ordered = [by_count[count] for count in (1, 2, 3)]
    for fields, expected_count, label in (
        (("capture_sha256",), 3, "capture"),
        (("receipt_sha256",), 3, "receipt"),
        (("evidence_bundle_sha256",), 3, "evidence"),
        (("receipt_store_id", "receipt_store_closure_sha256"), 3, "receipt-store"),
        (("engram_source_closure_sha256",), 3, "Engram runtime source-closure"),
        (("engram_source_roster_sha256",), 1, "Engram source-roster"),
        (("observed_build_receipt_exact_sha256",), 1, "observed-build receipt"),
    ):
        identities = {tuple(row[field] for field in fields) for row in ordered}
        if len(identities) != expected_count:
            raise MatrixError(f"CREBAIN indexed {label} identity cardinality differs")
    if (
        ordered[0]["observed_build_receipt_exact_sha256"]
        != package["observed_build_receipt_exact_sha256"]
    ):
        raise MatrixError("CREBAIN indexed observed-build receipt differs")
    return {
        "rows": ordered,
        "input_suite": input_suite,
        "tool_sources": tool_sources,
        "crebain_source_repository": index_crebain,
    }


def validate_source_file(
    engram_root: Path,
    expected_revision: str,
    record: dict[str, Any],
) -> None:
    relative = safe_relative(
        record["relative_path"], "Engram source path", suffix=".py"
    )
    target = repository_path(engram_root, relative, "Engram source")
    payload = snapshot_regular_file(target, MAX_SOURCE_BYTES, allow_empty=True)
    if (
        len(payload) != record["size_bytes"]
        or digest_bytes(payload) != record["sha256"]
    ):
        raise MatrixError(f"Engram source bytes differ: {relative}")
    tree_row = git_output(
        engram_root,
        "ls-tree",
        expected_revision,
        "--",
        relative.as_posix(),
    ).decode()
    expected = (
        f"{record['git_mode']} blob {record['git_blob']}\t{relative.as_posix()}\n"
    )
    if tree_row != expected:
        raise MatrixError(f"Engram source Git identity differs: {relative}")
    observed_blob = (
        git_output(
            engram_root,
            "hash-object",
            "--no-filters",
            "--",
            relative.as_posix(),
        )
        .decode()
        .strip()
    )
    if observed_blob != record["git_blob"]:
        raise MatrixError(f"Engram source blob differs: {relative}")


def validate_reviewed_exec_gate_binding(
    binding_value: Any,
    handshake_value: Any,
    *,
    closure_source_sha256: Any,
    closure_command_sha256: Any,
    source_row_sha256: Any,
    runtime_files_value: Any,
) -> dict[str, Any]:
    """Close one reviewed contained-command preimage across its source evidence."""

    binding = require_keys(
        binding_value,
        REVIEWED_EXEC_GATE_COMMAND_FIELDS,
        "CREBAIN reviewed contained-exec command",
    )
    if not isinstance(handshake_value, dict):
        raise MatrixError("CREBAIN reviewed runtime handshake differs")
    runtime_files = require_list(
        runtime_files_value,
        "CREBAIN reviewed runtime file roster",
        minimum=1,
        maximum=256,
    )
    python_rows = [
        row
        for row in runtime_files
        if isinstance(row, dict) and row.get("role") == "python-executable"
    ]
    if len(python_rows) != 1:
        raise MatrixError("CREBAIN reviewed Python executable identity differs")
    python_row = require_keys(
        python_rows[0],
        {"role", "absolute_path", "sha256", "size_bytes"},
        "CREBAIN reviewed Python executable",
    )
    if (
        binding["schema_version"] != "engram.contained-exec-command.v1"
        or binding["argument_shape"] != EXPECTED_EXEC_GATE_ARGUMENT_SHAPE
        or any(
            not valid_sha256(binding[field])
            for field in (
                "python_executable_sha256",
                "exec_gate_source_sha256",
                "target_command_sha256",
                "exec_gate_command_sha256",
            )
        )
        or binding["exec_gate_command_sha256"]
        != digest_without(binding, "exec_gate_command_sha256")
        or not valid_sha256(closure_source_sha256)
        or not valid_sha256(closure_command_sha256)
        or binding["exec_gate_source_sha256"] != closure_source_sha256
        or binding["exec_gate_source_sha256"] != source_row_sha256
        or binding["exec_gate_command_sha256"] != closure_command_sha256
        or handshake_value.get("exec_gate_source_sha256") != closure_source_sha256
        or handshake_value.get("exec_gate_command_sha256") != closure_command_sha256
        or python_row["sha256"] != binding["python_executable_sha256"]
    ):
        raise MatrixError("CREBAIN reviewed contained-exec command closure differs")
    return binding


def validate_source_closure(
    capture: dict[str, Any],
    expected_engram: dict[str, Any],
    engram_pack_receipt: dict[str, Any],
    *,
    verify_source_bytes: bool,
) -> dict[str, Any]:
    closure = require_keys(
        capture["engram_source_closure"],
        SOURCE_CLOSURE_FIELDS,
        "CREBAIN Engram source closure",
    )
    if (
        closure["schema_version"] != "crebain.engram-python-source-closure.v1"
        or closure["discovery_policy"]
        != "loaded-host-modules-plus-worker-runtime-identity-and-entrypoints.v1"
        or not valid_sha256(closure["closure_sha256"])
        or not valid_sha256(closure["source_roster_sha256"])
        or closure["closure_sha256"] != digest_without(closure, "closure_sha256")
    ):
        raise MatrixError("CREBAIN Engram source-closure identity differs")
    git = require_keys(
        closure["git"],
        {"repository", "commit", "tree", "origin_main", "object_format", "clean"},
        "CREBAIN Engram Git closure",
    )
    if git != {
        "repository": expected_engram["repository"],
        "commit": expected_engram["commit"],
        "tree": expected_engram["tree"],
        "origin_main": expected_engram["origin_main"],
        "object_format": expected_engram["object_format"],
        "clean": True,
    }:
        raise MatrixError("CREBAIN capture binds a different Engram revision")
    sources = require_list(
        closure["sources"],
        "CREBAIN Engram source roster",
        minimum=1,
        maximum=512,
    )
    source_by_path: dict[str, dict[str, Any]] = {}
    total_bytes = 0
    for source in sources:
        record = require_keys(
            source,
            {"relative_path", "size_bytes", "sha256", "git_mode", "git_blob"},
            "CREBAIN Engram source row",
        )
        relative = safe_relative(
            record["relative_path"], "Engram source path", suffix=".py"
        )
        size_bytes = record["size_bytes"]
        if (
            isinstance(size_bytes, bool)
            or not isinstance(size_bytes, int)
            or not 0 <= size_bytes <= MAX_SOURCE_BYTES
            or not valid_sha256(record["sha256"])
            or record["git_mode"] not in {"100644", "100755"}
            or not valid_git_oid(record["git_blob"])
            or relative.as_posix() in source_by_path
        ):
            raise MatrixError("CREBAIN Engram source row differs")
        source_by_path[relative.as_posix()] = record
        total_bytes += size_bytes
        if verify_source_bytes:
            validate_source_file(
                expected_engram["root"],
                expected_engram["commit"],
                record,
            )
    if total_bytes > MAX_SOURCE_TOTAL_BYTES or list(source_by_path) != sorted(
        source_by_path
    ):
        raise MatrixError("CREBAIN Engram source roster is not bounded and ordered")
    source_roster_sha256 = digest_bytes(
        b"crebain.engram-source-roster.v1\0" + canonical(sources)
    )
    if closure["source_roster_sha256"] != source_roster_sha256:
        raise MatrixError("CREBAIN Engram source-roster digest differs")
    source_hashes = capture["engram_source_sha256"]
    if not isinstance(source_hashes, dict) or source_hashes != {
        path: row["sha256"] for path, row in source_by_path.items()
    }:
        raise MatrixError("CREBAIN Engram source hash projection differs")
    host_modules = require_list(
        closure["host_modules"],
        "CREBAIN Engram host-module roster",
        minimum=1,
        maximum=512,
    )
    worker_modules = require_list(
        closure["worker_project_modules"],
        "CREBAIN Engram worker-module roster",
        minimum=1,
        maximum=512,
    )
    declared_source_paths: set[str] = set()
    for label, roster in (
        ("host module", host_modules),
        ("worker module", worker_modules),
    ):
        seen_names: set[str] = set()
        seen_paths: set[str] = set()
        observed_order: list[tuple[str, str]] = []
        for item in roster:
            row = require_keys(item, {"module_name", "relative_path"}, label)
            relative = safe_relative(
                row["relative_path"], f"{label} path", suffix=".py"
            )
            module_path = (
                relative.parent
                if relative.name == "__init__.py"
                else relative.with_suffix("")
            )
            expected_module_name = ".".join(module_path.parts)
            if (
                not isinstance(row["module_name"], str)
                or not row["module_name"]
                or row["module_name"] != expected_module_name
                or any(
                    not part.isidentifier() for part in row["module_name"].split(".")
                )
                or row["module_name"] in seen_names
                or relative.as_posix() in seen_paths
                or relative.as_posix() not in source_by_path
            ):
                raise MatrixError(f"CREBAIN Engram {label} roster differs")
            seen_names.add(row["module_name"])
            seen_paths.add(relative.as_posix())
            declared_source_paths.add(relative.as_posix())
            observed_order.append((row["module_name"], relative.as_posix()))
        if observed_order != sorted(observed_order):
            raise MatrixError(f"CREBAIN Engram {label} roster is not ordered")
    pack_repository = require_keys(
        engram_pack_receipt["engram_repository"],
        ENGRAM_PACK_REPOSITORY_FIELDS,
        "CREBAIN Engram pack repository",
    )
    pack_tool = require_keys(
        engram_pack_receipt["engram_tool"],
        ENGRAM_PACK_TOOL_FIELDS,
        "CREBAIN Engram pack tool",
    )
    expected_pack_repository = {
        "origin": git["repository"],
        "commit": git["commit"],
        "tree": git["tree"],
        "origin_main": git["origin_main"],
        "object_format": git["object_format"],
        "clean": git["clean"],
    }
    expected_pack_module = {
        "module_name": "scripts.engram_extension",
        "relative_path": EXPECTED_ENGRAM_PACK_TOOL_PATH,
    }
    if (
        pack_repository != expected_pack_repository
        or source_by_path.get(EXPECTED_ENGRAM_PACK_TOOL_PATH) != pack_tool
        or expected_pack_module not in host_modules
    ):
        raise MatrixError(
            "CREBAIN Engram pack tool differs from the loaded source closure"
        )
    exercised = require_list(
        closure["exercised_entrypoints"],
        "CREBAIN Engram entrypoint roster",
        minimum=1,
        maximum=64,
    )
    guardian_path: str | None = None
    seen_roles: set[str] = set()
    exercised_pairs: list[tuple[str, str]] = []
    for item in exercised:
        row = require_keys(item, {"role", "relative_path"}, "Engram entrypoint row")
        relative = safe_relative(
            row["relative_path"], "Engram entrypoint path", suffix=".py"
        )
        if (
            not isinstance(row["role"], str)
            or not row["role"]
            or row["role"] in seen_roles
            or relative.as_posix() not in source_by_path
        ):
            raise MatrixError("CREBAIN Engram entrypoint roster differs")
        seen_roles.add(row["role"])
        declared_source_paths.add(relative.as_posix())
        exercised_pairs.append((row["role"], relative.as_posix()))
        if row["role"] == "reviewed-runtime-guardian":
            guardian_path = relative.as_posix()
    expected_exercised_pairs = [
        (
            "nest-guardian",
            "backend/optimization/extension_closed_loop_nest_guardian.py",
        ),
        (
            "nest-worker",
            "backend/optimization/extension_closed_loop_nest_worker.py",
        ),
        (
            "reviewed-runtime-guardian",
            "backend/integrations/reviewed_native_process_guardian.py",
        ),
    ]
    exec_gate_source = source_by_path.get(EXPECTED_ENGRAM_EXEC_GATE_PATH)
    if (
        guardian_path is None
        or exercised_pairs != expected_exercised_pairs
        or closure["reviewed_runtime_guardian_source_sha256"]
        != (source_by_path[guardian_path]["sha256"])
        or exec_gate_source is None
        or closure["reviewed_runtime_exec_gate_source_sha256"]
        != exec_gate_source["sha256"]
        or not valid_sha256(closure["reviewed_runtime_exec_gate_command_sha256"])
        or set(source_by_path) != declared_source_paths
    ):
        raise MatrixError("CREBAIN declared source lineage differs")
    evidence = capture.get("nest_evidence_bundle")
    worker_identity = (
        evidence.get("worker_runtime_identity") if isinstance(evidence, dict) else None
    )
    worker_binding = (
        evidence.get("worker_session_binding") if isinstance(evidence, dict) else None
    )
    worker_roster_sha256 = closure["worker_project_source_roster_sha256"]
    if (
        not valid_sha256(worker_roster_sha256)
        or not isinstance(worker_identity, dict)
        or not isinstance(worker_binding, dict)
        or worker_identity.get("project_source_closure_verified") is not True
        or worker_identity.get("project_source_roster_sha256") != worker_roster_sha256
        or worker_binding.get("worker_project_source_roster_sha256")
        != worker_roster_sha256
    ):
        raise MatrixError("CREBAIN NEST worker source roster lineage differs")
    identity_files = require_list(
        worker_identity.get("files"),
        "CREBAIN NEST worker runtime files",
        minimum=1,
        maximum=256,
    )
    if worker_identity.get("file_roster_sha256") != digest_bytes(
        canonical(identity_files)
    ):
        raise MatrixError("CREBAIN NEST worker file roster digest differs")
    project_files = [
        row
        for row in identity_files
        if isinstance(row, dict)
        and isinstance(row.get("role"), str)
        and row["role"].startswith("project-module:")
    ]
    if worker_roster_sha256 != digest_bytes(canonical(project_files)):
        raise MatrixError("CREBAIN NEST worker project roster digest differs")
    derived_worker_modules: list[dict[str, str]] = []
    for row in project_files:
        record = require_keys(
            row,
            {"role", "absolute_path", "sha256", "size_bytes"},
            "CREBAIN NEST worker project file",
        )
        module_name = record["role"].removeprefix("project-module:")
        if not module_name or any(
            not part.isidentifier() for part in module_name.split(".")
        ):
            raise MatrixError("CREBAIN NEST worker module name differs")
        absolute_path = record["absolute_path"]
        if (
            not isinstance(absolute_path, str)
            or not absolute_path.startswith("/")
            or "\\" in absolute_path
            or "\0" in absolute_path
        ):
            raise MatrixError("CREBAIN NEST worker source path differs")
        reported = PurePosixPath(absolute_path)
        module_path = PurePosixPath(*module_name.split("."))
        relative = (
            module_path / "__init__.py"
            if reported.name == "__init__.py"
            else module_path.with_suffix(".py")
        )
        source = source_by_path.get(relative.as_posix())
        if (
            source is None
            or record["sha256"] != source["sha256"]
            or record["size_bytes"] != source["size_bytes"]
            or tuple(reported.parts[-len(relative.parts) :]) != relative.parts
        ):
            raise MatrixError("CREBAIN NEST worker source bytes differ")
        derived_worker_modules.append(
            {"module_name": module_name, "relative_path": relative.as_posix()}
        )
    if worker_modules != sorted(
        derived_worker_modules,
        key=lambda row: (row["module_name"], row["relative_path"]),
    ):
        raise MatrixError("CREBAIN source closure omits a NEST worker module")
    reviewed = require_keys(
        capture["reviewed_native_runtime"],
        REVIEWED_NATIVE_RUNTIME_FIELDS,
        "CREBAIN reviewed runtime",
    )
    handshake = reviewed["handshake_receipt"]
    if not isinstance(handshake, dict) or closure[
        "reviewed_runtime_handshake_receipt_sha256"
    ] != handshake.get("receipt_sha256"):
        raise MatrixError("CREBAIN source closure and reviewed handshake differ")
    validate_reviewed_exec_gate_binding(
        reviewed["exec_gate_command_binding"],
        handshake,
        closure_source_sha256=closure["reviewed_runtime_exec_gate_source_sha256"],
        closure_command_sha256=closure["reviewed_runtime_exec_gate_command_sha256"],
        source_row_sha256=exec_gate_source["sha256"],
        runtime_files_value=identity_files,
    )
    return {
        "engram_revision": git["commit"],
        "engram_tree": git["tree"],
        "engram_source_closure_sha256": closure["closure_sha256"],
        "engram_source_file_count": len(sources),
        "engram_source_roster_sha256": closure["source_roster_sha256"],
    }


def required_boolean_fields(
    value: dict[str, Any], fields: set[str], label: str
) -> None:
    require_keys(value, fields, label)
    if any(item is not True for item in value.values()):
        raise MatrixError(f"{label} contains an unverified assertion")


def validate_recorded_summary_authority(summary: dict[str, Any]) -> None:
    """Keep a recorded simulator summary outside execution authority."""

    if (
        summary.get("authority") is not False
        or summary.get("simulator_only") is not True
    ):
        raise MatrixError("CREBAIN recorded summary authority differs")


def validate_crebain_source_rows(
    rows_value: Any,
    *,
    label: str,
    expected_crebain: dict[str, Any],
    require_build_inputs: bool,
    verify_source_bytes: bool,
) -> list[dict[str, Any]]:
    """Validate one bounded, committed CREBAIN source roster."""

    rows = require_list(rows_value, label, minimum=1, maximum=128)
    object_format = expected_crebain["object_format"]
    paths: list[str] = []
    total_bytes = 0
    for item in rows:
        row = require_keys(item, CREBAIN_SOURCE_ROW_FIELDS, f"{label} row")
        relative = safe_relative(row["relative_path"], f"{label} path")
        size = row["size_bytes"]
        if (
            row["git_mode"] not in {"100644", "100755"}
            or not valid_git_oid(row["git_blob"], object_format)
            or not valid_sha256(row["sha256"])
            or isinstance(size, bool)
            or not isinstance(size, int)
            or not 1 <= size <= MAX_SOURCE_BYTES
        ):
            raise MatrixError(f"{label} row identity differs")
        paths.append(relative.as_posix())
        total_bytes += size
    if paths != sorted(set(paths)) or total_bytes > 128 * 1024 * 1024:
        raise MatrixError(f"{label} is not one bounded sorted source roster")
    if require_build_inputs:
        required = {
            "rust-toolchain.toml",
            "src-tauri/Cargo.lock",
            "src-tauri/Cargo.toml",
            "src-tauri/crates/managed-simulation/Cargo.toml",
            "src-tauri/crates/managed-simulation/src/lib.rs",
            "src-tauri/crates/managed-simulation/src/main.rs",
            "src-tauri/src/pid_observation.rs",
            "src-tauri/src/sensor_fusion.rs",
        } | CREBAIN_BUILD_CONTRACT_PATHS
        allowed = {
            "rust-toolchain.toml",
            "src-tauri/Cargo.lock",
            "src-tauri/Cargo.toml",
            "src-tauri/crates/managed-simulation/Cargo.toml",
            "src-tauri/src/pid_observation.rs",
            "src-tauri/src/sensor_fusion.rs",
        } | CREBAIN_BUILD_CONTRACT_PATHS
        if not required.issubset(paths) or any(
            path not in allowed
            and not (
                path.startswith("src-tauri/crates/managed-simulation/src/")
                and path.endswith(".rs")
            )
            for path in paths
        ):
            raise MatrixError(f"{label} differs from the managed build closure")
    if verify_source_bytes:
        try:
            verify_committed_source_roster(
                expected_crebain["root"],
                expected_crebain["commit"],
                rows,
                max_files=128,
                max_file_bytes=MAX_SOURCE_BYTES,
                max_total_bytes=128 * 1024 * 1024,
                checkout_revision=expected_crebain["checkout_revision"],
            )
        except (OSError, ValueError) as error:
            raise MatrixError(
                f"{label} does not reopen from committed Git bytes"
            ) from error
    return rows


def validate_crebain_build_receipt(
    value: Any,
    *,
    expected_crebain: dict[str, Any],
    verify_source_bytes: bool,
) -> dict[str, Any]:
    """Validate the embedded observed build without trusting its producer."""

    receipt = require_keys(
        value,
        CREBAIN_BUILD_RECEIPT_FIELDS,
        "CREBAIN observed-build receipt",
    )
    repository = require_keys(
        receipt["repository"],
        CREBAIN_BUILD_REPOSITORY_FIELDS,
        "CREBAIN observed-build repository",
    )
    expected_repository = {
        "origin": expected_crebain["repository"],
        "commit": expected_crebain["commit"],
        "tree": expected_crebain["tree"],
        "origin_main": expected_crebain["origin_main"],
        "object_format": expected_crebain["object_format"],
        "clean": True,
    }
    if (
        receipt["schema_version"]
        != "crebain.managed-simulation-observed-build-receipt.v1"
        or repository != expected_repository
    ):
        raise MatrixError("CREBAIN observed-build repository identity differs")
    source = require_keys(
        receipt["source"],
        CREBAIN_SOURCE_FIELDS,
        "CREBAIN build source closure",
    )
    source_rows = validate_crebain_source_rows(
        source["files"],
        label="CREBAIN build source roster",
        expected_crebain=expected_crebain,
        require_build_inputs=True,
        verify_source_bytes=verify_source_bytes,
    )
    if source[
        "policy"
    ] != "clean-origin-main-git-blob-and-rustc-dep-info-build-inputs.v1" or source[
        "roster_sha256"
    ] != digest_bytes(canonical(source_rows)):
        raise MatrixError("CREBAIN observed-build source closure differs")
    generator = require_keys(
        receipt["generator"],
        CREBAIN_GENERATOR_FIELDS,
        "CREBAIN build generator closure",
    )
    generator_rows = validate_crebain_source_rows(
        generator["files"],
        label="CREBAIN build generator roster",
        expected_crebain=expected_crebain,
        require_build_inputs=False,
        verify_source_bytes=verify_source_bytes,
    )
    if [
        row["relative_path"] for row in generator_rows
    ] != EXPECTED_CREBAIN_BUILD_GENERATORS or generator[
        "roster_sha256"
    ] != digest_bytes(canonical(generator_rows)):
        raise MatrixError("CREBAIN observed-build generator closure differs")
    cargo = require_keys(
        receipt["cargo"],
        CREBAIN_CARGO_FIELDS,
        "CREBAIN observed Cargo build",
    )
    exact_paths = {
        "workspace_manifest_path": "src-tauri/Cargo.toml",
        "package_manifest_path": "src-tauri/crates/managed-simulation/Cargo.toml",
        "lock_path": "src-tauri/Cargo.lock",
        "toolchain_path": "rust-toolchain.toml",
    }
    if any(cargo[field] != path for field, path in exact_paths.items()) or any(
        not valid_sha256(cargo[field])
        for field in (
            "workspace_manifest_exact_sha256",
            "package_manifest_exact_sha256",
            "lock_exact_sha256",
            "toolchain_exact_sha256",
        )
    ):
        raise MatrixError("CREBAIN observed-build Cargo source identity differs")
    if (
        cargo["rust_toolchain"] != "1.91.1"
        or cargo["rustc_version"] != "rustc 1.91.1 (ed61e7d7e 2025-11-07)"
        or cargo["cargo_version"] != "cargo 1.91.1 (ea2d97820 2025-10-10)"
        or cargo["argv"] != EXPECTED_CREBAIN_CARGO_ARGV
        or cargo["profile"] != "release"
        or cargo["target"] != EXPECTED_CREBAIN_TARGET
        or cargo["target_directory_policy"]
        != "fresh-fixed-owner-private-removed-after-copy.v1"
        or cargo["environment_policy"]
        != "reject-build-override-environment-and-record-output-bytes.v1"
    ):
        raise MatrixError("CREBAIN observed-build Cargo command or policy differs")
    source_by_path = {row["relative_path"]: row for row in source_rows}
    digest_joins = {
        "workspace_manifest_exact_sha256": "src-tauri/Cargo.toml",
        "package_manifest_exact_sha256": (
            "src-tauri/crates/managed-simulation/Cargo.toml"
        ),
        "lock_exact_sha256": "src-tauri/Cargo.lock",
        "toolchain_exact_sha256": "rust-toolchain.toml",
    }
    if any(
        cargo[field] != source_by_path[path]["sha256"]
        for field, path in digest_joins.items()
    ):
        raise MatrixError("CREBAIN Cargo inputs do not join committed source bytes")
    output = require_keys(
        receipt["output"],
        CREBAIN_BUILD_OUTPUT_FIELDS,
        "CREBAIN observed-build output",
    )
    if (
        output["file_name"] != "crebain-managed-simulation"
        or isinstance(output["byte_length"], bool)
        or not isinstance(output["byte_length"], int)
        or not 1 <= output["byte_length"] <= MAX_BINARY_BYTES
        or not valid_sha256(output["sha256"])
        or output["source_mode"] not in {0o700, 0o500, 0o755, 0o555}
        or output["format"] != "mach-o-64"
        or output["architecture"] != "arm64"
        or output["file_type"] != "executable"
    ):
        raise MatrixError("CREBAIN observed-build output identity differs")
    input_identity = {
        "repository": repository,
        "source_roster_sha256": source["roster_sha256"],
        "generator_roster_sha256": generator["roster_sha256"],
        "cargo": cargo,
    }
    if (
        receipt["input_identity_sha256"] != digest_bytes(canonical(input_identity))
        or receipt["claims"]
        != {
            "observed_local_build": True,
            "reproducible_build": False,
            "signature": False,
            "external_dependency_bytes_attested": False,
            "complete_environment_attested": False,
        }
        or receipt["authority"] != CREBAIN_NO_AUTHORITY
        or not isinstance(receipt["disclosure"], str)
        or not receipt["disclosure"]
        or len(receipt["disclosure"].encode("utf-8")) > 4096
        or not valid_sha256(receipt["receipt_sha256"])
        or receipt["receipt_sha256"] != digest_without(receipt, "receipt_sha256")
    ):
        raise MatrixError("CREBAIN observed-build receipt closure differs")
    return receipt


def validate_crebain_stage_inventory(
    value: Any,
    *,
    expected_crebain: dict[str, Any],
    stage_receipt: dict[str, Any],
    verify_source_bytes: bool,
) -> list[dict[str, Any]]:
    """Validate staged package identities and reopen committed contract bytes."""

    rows = require_list(value, "CREBAIN package inventory", minimum=2, maximum=128)
    paths: list[str] = []
    executable_rows: list[dict[str, Any]] = []
    contract_rows: list[dict[str, Any]] = []
    for item in rows:
        row = require_keys(
            item,
            CREBAIN_STAGE_INVENTORY_FIELDS,
            "CREBAIN package inventory row",
        )
        relative = safe_relative(row["relative_path"], "CREBAIN package path")
        if (
            isinstance(row["byte_length"], bool)
            or not isinstance(row["byte_length"], int)
            or not 1 <= row["byte_length"] <= MAX_BINARY_BYTES
            or not valid_sha256(row["sha256"])
            or row["mode"] not in {0o600, 0o700}
            or row["role"] not in {"contract", "executable"}
        ):
            raise MatrixError("CREBAIN package inventory row differs")
        if row["role"] == "executable":
            if (
                relative.as_posix() != "bin/crebain-managed-simulation"
                or row["mode"] != 0o700
            ):
                raise MatrixError("CREBAIN staged executable path or mode differs")
            executable_rows.append(row)
        else:
            if (
                relative.parts[0] != "contracts"
                or relative.suffix != ".json"
                or not relative.name.endswith(".schema.json")
                or row["mode"] != 0o600
            ):
                raise MatrixError("CREBAIN staged contract path or mode differs")
            contract_rows.append(row)
        paths.append(relative.as_posix())
    if paths != sorted(set(paths)) or len(executable_rows) != 1:
        raise MatrixError("CREBAIN package inventory order or executable count differs")
    staged = stage_receipt["staged_executable"]
    if executable_rows[0] != {
        "relative_path": "bin/crebain-managed-simulation",
        "byte_length": staged["byte_length"],
        "sha256": staged["sha256"],
        "mode": staged["mode"],
        "role": "executable",
    }:
        raise MatrixError("CREBAIN package executable bytes differ")
    if verify_source_bytes:
        recipe_relative = Path(
            "integrations/engram/managed-simulation/authoring.macos-aarch64-darwin.json"
        )
        configuration_relative = Path(
            "integrations/engram/managed-simulation/configuration.json"
        )
        recipe_source = capture_repository_file(
            expected_crebain["root"],
            expected_crebain["commit"],
            recipe_relative,
            MAX_SCHEMA_BYTES,
            checkout_revision=expected_crebain["checkout_revision"],
        )
        configuration_source = capture_repository_file(
            expected_crebain["root"],
            expected_crebain["commit"],
            configuration_relative,
            MAX_SCHEMA_BYTES,
            checkout_revision=expected_crebain["checkout_revision"],
        )
        if (
            stage_receipt["recipe_exact_sha256"] != recipe_source["sha256"]
            or stage_receipt["configuration_exact_sha256"]
            != configuration_source["sha256"]
        ):
            raise MatrixError("CREBAIN stage recipe or configuration bytes differ")
        recipe_payload = snapshot_regular_file(
            expected_crebain["root"] / recipe_relative,
            MAX_SCHEMA_BYTES,
        )
        recipe = load_json_payload(recipe_payload, "CREBAIN package recipe")
        recipe_schemas = recipe.get("schemas")
        executable = recipe.get("executable")
        if (
            recipe.get("configuration_path") != "configuration.json"
            or not isinstance(executable, dict)
            or executable.get("package_relative_path")
            != "bin/crebain-managed-simulation"
            or not isinstance(recipe_schemas, list)
            or not recipe_schemas
        ):
            raise MatrixError("CREBAIN package recipe differs")
        expected_contracts: list[dict[str, Any]] = []
        schema_ids: list[str] = []
        for item in recipe_schemas:
            schema = require_keys(
                item,
                {"schema_id", "package_relative_path"},
                "CREBAIN package recipe schema",
            )
            package_relative = safe_relative(
                schema["package_relative_path"],
                "CREBAIN package recipe schema path",
                suffix=".json",
            )
            schema_id = schema["schema_id"]
            if (
                package_relative.parts[0] != "contracts"
                or not package_relative.name.endswith(".schema.json")
                or not isinstance(schema_id, str)
                or not schema_id
                or len(schema_id.encode("utf-8")) > 512
            ):
                raise MatrixError("CREBAIN package recipe schema differs")
            schema_ids.append(schema_id)
            source_relative = recipe_relative.parent.joinpath(*package_relative.parts)
            source = capture_repository_file(
                expected_crebain["root"],
                expected_crebain["commit"],
                source_relative,
                MAX_SCHEMA_BYTES,
                checkout_revision=expected_crebain["checkout_revision"],
            )
            expected_contracts.append(
                {
                    "relative_path": package_relative.as_posix(),
                    "byte_length": source["byte_count"],
                    "sha256": source["sha256"],
                    "mode": 0o600,
                    "role": "contract",
                }
            )
        expected_contracts.sort(key=lambda row: row["relative_path"])
        if schema_ids != sorted(set(schema_ids)) or contract_rows != expected_contracts:
            raise MatrixError("CREBAIN staged contract bytes differ from the recipe")
    return rows


def validate_crebain_stage_receipt(
    value: Any,
    *,
    build_receipt: dict[str, Any],
    build_receipt_bytes: bytes,
    expected_crebain: dict[str, Any],
    verify_source_bytes: bool,
) -> dict[str, Any]:
    """Validate the embedded package-stage receipt and byte identities."""

    receipt = require_keys(
        value,
        CREBAIN_STAGE_RECEIPT_FIELDS,
        "CREBAIN package-stage receipt",
    )
    if (
        receipt["schema_version"]
        != "crebain.managed-simulation-package-stage-receipt.v1"
        or receipt["observed_build_receipt_exact_sha256"]
        != digest_bytes(build_receipt_bytes)
        or receipt["observed_build_receipt_sha256"] != build_receipt["receipt_sha256"]
        or receipt["crebain_commit"] != expected_crebain["commit"]
        or receipt["crebain_tree"] != expected_crebain["tree"]
        or receipt["origin_main"] != expected_crebain["origin_main"]
        or receipt["target"] != EXPECTED_CREBAIN_TARGET
        or not valid_sha256(receipt["recipe_exact_sha256"])
        or not valid_sha256(receipt["configuration_exact_sha256"])
    ):
        raise MatrixError("CREBAIN package-stage build or source lineage differs")
    source = require_keys(
        receipt["source_executable"],
        CREBAIN_EXECUTABLE_FIELDS,
        "CREBAIN stage source executable",
    )
    staged = require_keys(
        receipt["staged_executable"],
        CREBAIN_EXECUTABLE_FIELDS,
        "CREBAIN staged executable",
    )
    output = build_receipt["output"]
    expected_source = {
        "byte_length": output["byte_length"],
        "sha256": output["sha256"],
        "mode": output["source_mode"],
        "format": output["format"],
        "architecture": output["architecture"],
        "file_type": output["file_type"],
    }
    if source != expected_source or staged != {**source, "mode": 0o700}:
        raise MatrixError("CREBAIN staged executable does not join the build output")
    inventory = validate_crebain_stage_inventory(
        receipt["package_inventory"],
        expected_crebain=expected_crebain,
        stage_receipt=receipt,
        verify_source_bytes=verify_source_bytes,
    )
    if (
        receipt["package_inventory_sha256"] != digest_bytes(canonical(inventory))
        or receipt["authority"] != CREBAIN_NO_AUTHORITY
        or not isinstance(receipt["disclosure"], str)
        or not receipt["disclosure"]
        or len(receipt["disclosure"].encode("utf-8")) > 4096
        or not valid_sha256(receipt["receipt_sha256"])
        or receipt["receipt_sha256"] != digest_without(receipt, "receipt_sha256")
    ):
        raise MatrixError("CREBAIN package-stage receipt closure differs")
    return receipt


def validate_engram_pack_receipt(
    value: Any,
    *,
    build_receipt: dict[str, Any],
    build_receipt_bytes: bytes,
    stage_receipt: dict[str, Any],
    stage_receipt_bytes: bytes,
    expected_engram: dict[str, Any],
    verify_source_bytes: bool,
) -> dict[str, Any]:
    """Validate one embedded pack receipt and its committed Engram tool."""

    receipt = require_keys(
        value,
        ENGRAM_PACK_RECEIPT_FIELDS,
        "CREBAIN Engram pack receipt",
    )
    repository = require_keys(
        receipt["engram_repository"],
        ENGRAM_PACK_REPOSITORY_FIELDS,
        "CREBAIN Engram pack repository",
    )
    expected_repository = {
        "origin": expected_engram["repository"],
        "commit": expected_engram["commit"],
        "tree": expected_engram["tree"],
        "origin_main": expected_engram["origin_main"],
        "object_format": expected_engram["object_format"],
        "clean": True,
    }
    tool = require_keys(
        receipt["engram_tool"],
        ENGRAM_PACK_TOOL_FIELDS,
        "CREBAIN Engram pack tool",
    )
    size = tool["size_bytes"]
    if (
        receipt["schema_version"] != "crebain.managed-simulation-engram-pack-receipt.v1"
        or repository != expected_repository
        or tool["relative_path"] != EXPECTED_ENGRAM_PACK_TOOL_PATH
        or isinstance(size, bool)
        or not isinstance(size, int)
        or not 1 <= size <= MAX_SOURCE_BYTES
        or not valid_sha256(tool["sha256"])
        or tool["git_mode"] not in {"100644", "100755"}
        or not valid_git_oid(tool["git_blob"], expected_engram["object_format"])
        or receipt["verification_policy"]
        != ("clean-head-origin-main-committed-tool-before-and-after-each-operation.v1")
        or receipt["operations"] != ENGRAM_PACK_OPERATIONS
        or receipt["claims"] != ENGRAM_PACK_CLAIMS
        or receipt["authority"] != CREBAIN_NO_AUTHORITY
        or receipt["observed_build_receipt_exact_sha256"]
        != digest_bytes(build_receipt_bytes)
        or receipt["observed_build_receipt_sha256"] != build_receipt["receipt_sha256"]
        or receipt["package_stage_receipt_exact_sha256"]
        != digest_bytes(stage_receipt_bytes)
        or receipt["package_stage_receipt_sha256"] != stage_receipt["receipt_sha256"]
        or not valid_sha256(receipt["seal_receipt_exact_sha256"])
        or not valid_sha256(receipt["bundle_receipt_exact_sha256"])
        or not valid_prefixed_sha256(receipt["package_generation_id"], "pkggen_")
        or not isinstance(receipt["disclosure"], str)
        or not receipt["disclosure"]
        or len(receipt["disclosure"].encode("utf-8")) > 4096
        or not valid_sha256(receipt["receipt_sha256"])
        or receipt["receipt_sha256"] != digest_without(receipt, "receipt_sha256")
    ):
        raise MatrixError("CREBAIN Engram pack receipt closure differs")
    if verify_source_bytes:
        validate_source_file(
            expected_engram["root"],
            expected_engram["commit"],
            tool,
        )
    return receipt


def validate_installed_package_proof(
    capture: dict[str, Any],
    index: dict[str, Any],
    expected_crebain: dict[str, Any],
    expected_engram: dict[str, Any],
    tool_sources: dict[str, Any],
    *,
    verify_source_bytes: bool,
) -> dict[str, Any]:
    proof = require_keys(
        capture["installed_package_proof"],
        INSTALLED_PACKAGE_PROOF_FIELDS,
        "CREBAIN installed-package proof",
    )
    receipt_sha256 = proof["receipt_sha256"]
    exact_sha256 = digest_bytes(canonical(proof) + b"\n")
    digest_fields = {
        "generation_core_sha256",
        "bundle_receipt_exact_sha256",
        "seal_receipt_exact_sha256",
        "install_observation_exact_sha256",
        "manifest_exact_sha256",
        "package_lock_exact_sha256",
        "configuration_exact_sha256",
        "package_sha256",
        "executable_sha256",
        "configuration_canonical_sha256",
        "operation_roster_sha256",
        "baseline_three_controls_sha256",
        "observed_build_receipt_exact_sha256",
        "observed_build_receipt_sha256",
        "package_stage_receipt_exact_sha256",
        "package_stage_receipt_sha256",
        "engram_pack_receipt_exact_sha256",
        "engram_pack_receipt_sha256",
        "engram_extension_tool_sha256",
        "build_source_roster_sha256",
        "build_input_identity_sha256",
        "receipt_sha256",
    }
    if (
        proof["schema_version"] != "crebain.standard-v3-installed-binary-proof.v3"
        or not valid_prefixed_sha256(proof["store_id"], "extstore_")
        or not valid_prefixed_sha256(proof["package_generation_id"], "pkggen_")
        or not valid_prefixed_sha256(proof["installation_id"], "inst_")
        or any(not valid_sha256(proof[field]) for field in digest_fields)
        or receipt_sha256 != digest_without(proof, "receipt_sha256")
        or capture["installed_package_proof_exact_sha256"] != exact_sha256
        or index["installed_package_proof_exact_sha256"] != exact_sha256
        or capture["package_generation_id"] != proof["package_generation_id"]
    ):
        raise MatrixError("CREBAIN installed-package proof identity differs")
    build = validate_crebain_build_receipt(
        proof["observed_build_receipt"],
        expected_crebain=expected_crebain,
        verify_source_bytes=verify_source_bytes,
    )
    build_bytes = canonical(build) + b"\n"
    stage = validate_crebain_stage_receipt(
        proof["package_stage_receipt"],
        build_receipt=build,
        build_receipt_bytes=build_bytes,
        expected_crebain=expected_crebain,
        verify_source_bytes=verify_source_bytes,
    )
    stage_bytes = canonical(stage) + b"\n"
    pack = validate_engram_pack_receipt(
        proof["engram_pack_receipt"],
        build_receipt=build,
        build_receipt_bytes=build_bytes,
        stage_receipt=stage,
        stage_receipt_bytes=stage_bytes,
        expected_engram=expected_engram,
        verify_source_bytes=verify_source_bytes,
    )
    pack_bytes = canonical(pack) + b"\n"
    pack_repository = pack["engram_repository"]
    pack_tool = pack["engram_tool"]
    if (
        proof["observed_build_receipt_exact_sha256"] != digest_bytes(build_bytes)
        or proof["observed_build_receipt_sha256"] != build["receipt_sha256"]
        or proof["package_stage_receipt_exact_sha256"] != digest_bytes(stage_bytes)
        or proof["package_stage_receipt_sha256"] != stage["receipt_sha256"]
        or proof["engram_pack_receipt_exact_sha256"] != digest_bytes(pack_bytes)
        or proof["engram_pack_receipt_sha256"] != pack["receipt_sha256"]
        or proof["crebain_commit"] != build["repository"]["commit"]
        or proof["crebain_tree"] != build["repository"]["tree"]
        or proof["crebain_origin_main"] != build["repository"]["origin_main"]
        or proof["engram_commit"] != pack_repository["commit"]
        or proof["engram_tree"] != pack_repository["tree"]
        or proof["engram_origin_main"] != pack_repository["origin_main"]
        or proof["engram_extension_tool_sha256"] != pack_tool["sha256"]
        or proof["engram_extension_tool_git_blob"] != pack_tool["git_blob"]
        or pack["seal_receipt_exact_sha256"] != proof["seal_receipt_exact_sha256"]
        or pack["bundle_receipt_exact_sha256"] != proof["bundle_receipt_exact_sha256"]
        or pack["package_generation_id"] != proof["package_generation_id"]
        or proof["build_source_roster_sha256"] != build["source"]["roster_sha256"]
        or proof["build_input_identity_sha256"] != build["input_identity_sha256"]
        or proof["configuration_exact_sha256"] != stage["configuration_exact_sha256"]
        or proof["executable_sha256"] != build["output"]["sha256"]
        or proof["executable_sha256"] != stage["staged_executable"]["sha256"]
        or proof["executable_format"] != "mach-o-64"
        or proof["executable_architecture"] != "arm64"
    ):
        raise MatrixError(
            "CREBAIN build, stage, pack, and installed-proof lineage differs"
        )
    generator_by_path = {
        row["relative_path"]: row for row in build["generator"]["files"]
    }
    tools_by_path = {row["path"]: row for row in tool_sources["files"]}
    for path in (
        "scripts/managed_simulation_authoring_files.py",
        "scripts/managed_simulation_build_provenance.py",
    ):
        if (
            path not in generator_by_path
            or path not in tools_by_path
            or generator_by_path[path]["sha256"] != tools_by_path[path]["exact_sha256"]
        ):
            raise MatrixError("CREBAIN tool and build-generator source bytes differ")
    check_false_authority(
        proof["authority"],
        CAPTURE_AUTHORITY_FIELDS,
        "CREBAIN installed-package proof authority",
    )
    package_projection = {field: proof[field] for field in INDEX_PACKAGE_FIELDS}
    if package_projection != index["package"]:
        raise MatrixError("CREBAIN installed-package index projection differs")
    standard_schemas = require_keys(
        proof["standard_schema_sha256"],
        EXPECTED_STANDARD_SCHEMA_IDS,
        "CREBAIN installed standard-schema roster",
    )
    recovery_digests = require_keys(
        proof["recovery_controls_sha256"],
        {"1", "2", "3"},
        "CREBAIN installed recovery-control roster",
    )
    if (
        any(not valid_sha256(value) for value in standard_schemas.values())
        or any(not valid_sha256(value) for value in recovery_digests.values())
        or proof["operation_ids"] != EXPECTED_CREBAIN_OPERATION_IDS
        or proof["drone_counts"] != [1, 2, 3]
        or proof["step_count"] != 6
        or proof["fault_step"] != 3
        or proof["fault"] != "sensor-unavailable"
        or proof["host_policy"]
        != [
            "fault-observed",
            "safe-hold",
            "bounded-zero-washout",
            "bounded-nonzero-resume",
        ]
        or proof["replay_exact"] is not True
        or proof["unaffected_lane_observations_exact"] is not True
        or proof["negative_clock_gate"] != "standard.clock-mismatch"
        or proof["signal_cancellation_gate"]
        != "active-SIGTERM-then-fresh-generation-prepared"
        or proof["installed_artifacts_reverified_after_execution"] is not True
        or proof["generation_seal_package_bundle_store_lineage_verified"] is not True
        or proof["build_stage_seal_install_lineage_verified"] is not True
        or proof["build_stage_seal_pack_install_lineage_verified"] is not True
        or not isinstance(proof["disclosure"], str)
        or not proof["disclosure"]
        or len(proof["disclosure"].encode("utf-8")) > 512
    ):
        raise MatrixError("CREBAIN installed-package proof closure differs")
    return {
        "store_id": proof["store_id"],
        "package_generation_id": proof["package_generation_id"],
        "installation_id": proof["installation_id"],
        "installed_package_proof_exact_sha256": exact_sha256,
        "installed_package_proof_receipt_sha256": receipt_sha256,
        "observed_build_receipt_exact_sha256": proof[
            "observed_build_receipt_exact_sha256"
        ],
        "observed_build_receipt_sha256": proof["observed_build_receipt_sha256"],
        "package_stage_receipt_exact_sha256": proof[
            "package_stage_receipt_exact_sha256"
        ],
        "package_stage_receipt_sha256": proof["package_stage_receipt_sha256"],
        "engram_pack_receipt_exact_sha256": proof["engram_pack_receipt_exact_sha256"],
        "engram_pack_receipt_sha256": proof["engram_pack_receipt_sha256"],
        "engram_pack_receipt": pack,
        "engram_commit": proof["engram_commit"],
        "engram_tree": proof["engram_tree"],
        "engram_origin_main": proof["engram_origin_main"],
        "engram_extension_tool_sha256": proof["engram_extension_tool_sha256"],
        "engram_extension_tool_git_blob": proof["engram_extension_tool_git_blob"],
        "build_stage_seal_pack_install_lineage_verified": proof[
            "build_stage_seal_pack_install_lineage_verified"
        ],
        "crebain_commit": proof["crebain_commit"],
        "crebain_tree": proof["crebain_tree"],
        "build_source_roster_sha256": proof["build_source_roster_sha256"],
        "build_input_identity_sha256": proof["build_input_identity_sha256"],
        "package_inventory_sha256": stage["package_inventory_sha256"],
        "executable_sha256": proof["executable_sha256"],
    }


def validate_receipt_store_sidecars(
    sidecars_value: Any,
    *,
    store_id: str,
    terminal_receipt_sha256: str,
    terminal_artifact_size_bytes: int,
    evidence_bundle_sha256: str,
    evidence_artifact_size_bytes: int,
    study_run_id: str,
    run_status: str,
    terminal_reason_code: str,
    package_generation_id: str,
    raw_capture: dict[str, Any] | None = None,
) -> dict[str, Any]:
    sidecars = require_keys(
        sidecars_value,
        RECEIPT_STORE_SIDECAR_FIELDS,
        "CREBAIN receipt-store sidecars",
    )
    if sidecars["schema_version"] != RECEIPT_STORE_SIDECARS_SCHEMA:
        raise MatrixError("CREBAIN receipt-store sidecar schema differs")
    require_canonical_digest(
        sidecars,
        "closure_sha256",
        "CREBAIN receipt-store sidecars",
    )
    metadata = require_keys(
        sidecars["store_metadata"],
        RECEIPT_STORE_METADATA_FIELDS,
        "CREBAIN receipt-store metadata",
    )
    finalization = require_keys(
        sidecars["finalized_reservation"],
        RECEIPT_STORE_FINALIZATION_FIELDS,
        "CREBAIN finalized reservation",
    )
    reservation = require_keys(
        finalization["reservation"],
        RECEIPT_STORE_RESERVATION_FIELDS,
        "CREBAIN receipt reservation",
    )
    observation = require_keys(
        sidecars["observation"],
        RECEIPT_STORE_OBSERVATION_FIELDS,
        "CREBAIN stored receipt observation",
    )
    anchor = require_keys(
        sidecars["publication_admission_anchor"],
        RECEIPT_STORE_ADMISSION_ANCHOR_FIELDS,
        "CREBAIN publication admission anchor",
    )
    authority = require_keys(
        sidecars["publication_authority"],
        RECEIPT_STORE_PUBLICATION_AUTHORITY_FIELDS,
        "CREBAIN publication authority",
    )
    artifact = require_keys(
        observation["artifact"],
        {"artifact_id", "kind", "sha256"},
        "CREBAIN stored receipt artifact",
    )
    handshake = require_keys(
        reservation["reviewed_native_handshake"],
        REVIEWED_HANDSHAKE_FIELDS,
        "CREBAIN reserved reviewed-native handshake",
    )
    reject_recursive_authority(sidecars, "CREBAIN receipt-store sidecars")
    require_canonical_digest(
        handshake,
        "receipt_sha256",
        "CREBAIN reserved reviewed-native handshake",
    )
    for document, field, label in (
        (reservation, "reservation_sha256", "CREBAIN receipt reservation"),
        (finalization, "finalization_sha256", "CREBAIN finalized reservation"),
        (observation, "record_sha256", "CREBAIN stored receipt observation"),
        (anchor, "anchor_sha256", "CREBAIN publication admission anchor"),
        (authority, "authority_sha256", "CREBAIN publication authority"),
    ):
        require_managed_runtime_digest(document, field, label)

    reservation_id = reservation["reservation_id"]
    pre_spawn_sha256 = reservation["pre_spawn_sha256"]
    nest_work_admission_sha256 = reservation["nest_work_admission_sha256"]
    if (
        not valid_prefixed_sha256(store_id, "clrs_")
        or not valid_prefixed_sha256(reservation_id, "clrr_")
        or not valid_prefixed_sha256(package_generation_id, "pkggen_")
        or not valid_prefixed_sha256(reservation["runtime_generation_id"], "gen_")
        or not isinstance(study_run_id, str)
        or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._:-]{0,127}", study_run_id) is None
        or not valid_sha256(terminal_receipt_sha256)
        or not valid_sha256(evidence_bundle_sha256)
        or terminal_receipt_sha256 == evidence_bundle_sha256
        or isinstance(terminal_artifact_size_bytes, bool)
        or not isinstance(terminal_artifact_size_bytes, int)
        or not 1 <= terminal_artifact_size_bytes <= MAX_CAPTURE_BYTES
        or isinstance(evidence_artifact_size_bytes, bool)
        or not isinstance(evidence_artifact_size_bytes, int)
        or not 1 <= evidence_artifact_size_bytes <= MAX_CAPTURE_BYTES
        or any(
            not valid_sha256(reservation[field])
            for field in (
                "closed_loop_definition_sha256",
                "nest_work_admission_sha256",
                "pre_spawn_sha256",
                "run_plan_sha256",
                "nest_configuration_sha256",
                "expected_runtime_binding_sha256",
                "reviewed_native_handshake_receipt_sha256",
                "reservation_sha256",
            )
        )
        or reservation["reviewed_native_handshake_receipt_sha256"]
        != handshake["receipt_sha256"]
        or reservation["package_generation_id"] != package_generation_id
        or reservation["package_generation_id"] != handshake["package_generation_id"]
        or reservation["runtime_generation_id"] != handshake["generation_id"]
        or reservation["reserved_record_count"] != 1
        or reservation["reserved_artifact_bytes"] != 16 * 1024 * 1024
        or isinstance(reservation["reserved_evidence_bytes"], bool)
        or not isinstance(reservation["reserved_evidence_bytes"], int)
        or not 1 <= reservation["reserved_evidence_bytes"] <= MAX_RECEIPT_STORE_BYTES
        or reservation["reserved_record_bytes"] != 4096
    ):
        raise MatrixError("CREBAIN receipt-store reservation identity differs")

    simulation_dispatch_sha256 = digest_bytes(
        managed_runtime_canonical(
            {
                "schema_version": "engram.extension-closed-loop-dispatch-intent.v1",
                "store_id": store_id,
                "reservation_id": reservation_id,
                "reservation_sha256": reservation["reservation_sha256"],
            }
        )
    )
    extension_dispatch_sha256 = digest_bytes(
        managed_runtime_canonical(
            {
                "schema_version": (
                    "engram.extension-closed-loop-extension-dispatch-intent.v1"
                ),
                "store_id": store_id,
                "reservation_id": reservation_id,
                "pre_spawn_sha256": pre_spawn_sha256,
            }
        )
    )
    publication_wal_sha256 = digest_bytes(
        managed_runtime_canonical(
            {
                "domain": (
                    "engram-extension-closed-loop-reserved-publication-wal-closure-v1"
                ),
                "store_id": store_id,
                "reservation_id": reservation_id,
                "pre_spawn_sha256": pre_spawn_sha256,
                "extension_dispatch_sha256": extension_dispatch_sha256,
                "reservation_sha256": reservation["reservation_sha256"],
                "simulation_dispatch_sha256": simulation_dispatch_sha256,
                "terminal_receipt_sha256": terminal_receipt_sha256,
            }
        )
    )
    study_run_key_sha256 = digest_bytes(
        managed_runtime_canonical(
            {
                "domain": "engram-extension-closed-loop-publication-study-run-key-v1",
                "store_id": store_id,
                "study_run_id": study_run_id,
            }
        )
    )
    receipt_path = (
        f"receipts/{terminal_receipt_sha256[:2]}/{terminal_receipt_sha256}.json"
    )
    evidence_path = (
        f"evidence/{evidence_bundle_sha256[:2]}/{evidence_bundle_sha256}.json"
    )
    finalization_path = (
        f"finalized-reservations/{reservation_id[5:7]}/{reservation_id}.json"
    )
    observation_path = (
        f"observations/{terminal_receipt_sha256[:2]}/{terminal_receipt_sha256}.json"
    )
    anchor_path = f"publication-admission-anchors/{study_run_key_sha256}.json"
    authority_path = (
        "publication-authorities/"
        f"{terminal_receipt_sha256[:2]}/{terminal_receipt_sha256}.json"
    )
    expected_metadata = {
        "schema_version": RECEIPT_STORE_SCHEMA,
        "store_id": store_id,
        "policy": RECEIPT_STORE_POLICY,
        "digest_canonicalization": RECEIPT_STORE_CANONICALIZATION,
        "execution_authority": False,
        "ncp_control": False,
        "physical_actuation": False,
        "scientific_authority": False,
        "is_paper_local_evidence": False,
        "calibrated_posterior": False,
    }
    expected_artifact = {
        "artifact_id": f"art_{terminal_receipt_sha256[:32]}",
        "kind": "closed_loop_receipt",
        "sha256": terminal_receipt_sha256,
    }
    if (
        metadata != expected_metadata
        or reservation["schema_version"]
        != "engram.extension-closed-loop-receipt-reservation.v1"
        or reservation["store_id"] != store_id
        or reservation["study_run_id"] != study_run_id
        or reservation["receipt_profile"]
        != "engram.extension-closed-loop-run-receipt.v2"
        or reservation["evidence_profile"]
        not in {
            "engram.nest-closed-loop-evidence-bundle.v2",
            "optional-engram.nest-closed-loop-evidence-bundle.v2",
        }
        or finalization["schema_version"]
        != "engram.extension-closed-loop-finalized-reservation.v1"
        or finalization["store_id"] != store_id
        or finalization["pre_spawn_sha256"] != pre_spawn_sha256
        or finalization["extension_dispatch_sha256"] != extension_dispatch_sha256
        or finalization["simulation_dispatch_sha256"] != simulation_dispatch_sha256
        or finalization["terminal_receipt_sha256"] != terminal_receipt_sha256
        or finalization["evidence_bundle_sha256"] != evidence_bundle_sha256
        or finalization["nest_work_admission_rejoined"] is not True
        or artifact != expected_artifact
        or observation["schema_version"]
        != "engram.extension-closed-loop-stored-receipt.v5"
        or observation["store_id"] != store_id
        or observation["study_run_id"] != study_run_id
        or observation["run_status"] != run_status
        or observation["terminal_reason_code"] != terminal_reason_code
        or observation["relative_artifact_path"] != receipt_path
        or observation["artifact_byte_length"] != terminal_artifact_size_bytes
        or observation["evidence_profile"] != "killable-nest-population-controller-v2"
        or observation["evidence_bundle_sha256"] != evidence_bundle_sha256
        or observation["relative_evidence_path"] != evidence_path
        or observation["evidence_byte_length"] != evidence_artifact_size_bytes
        or observation["admission_mode"] != "reserved"
        or observation["reservation_id"] != reservation_id
        or observation["reservation_sha256"] != reservation["reservation_sha256"]
        or observation["reservation_finalization_sha256"]
        != finalization["finalization_sha256"]
        or observation["nest_work_admission_sha256"] != nest_work_admission_sha256
        or observation["nest_work_admission_rejoined"] is not True
        or observation["digest_canonicalization"] != RECEIPT_STORE_CANONICALIZATION
        or anchor["schema_version"]
        != "engram.extension-closed-loop-publication-admission-anchor.v1"
        or anchor["store_id"] != store_id
        or anchor["study_run_key_sha256"] != study_run_key_sha256
        or anchor["study_run_id"] != study_run_id
        or anchor["terminal_receipt_sha256"] != terminal_receipt_sha256
        or anchor["admission_mode"] != "reserved"
        or anchor["publication_wal_sha256"] != publication_wal_sha256
        or anchor["evidence_bundle_sha256"] != evidence_bundle_sha256
        or anchor["reservation_id"] != reservation_id
        or anchor["reservation_sha256"] != reservation["reservation_sha256"]
        or anchor["pre_spawn_sha256"] != pre_spawn_sha256
        or anchor["extension_dispatch_sha256"] != extension_dispatch_sha256
        or anchor["simulation_dispatch_sha256"] != simulation_dispatch_sha256
        or anchor["reservation_finalization_sha256"]
        != finalization["finalization_sha256"]
        or authority["schema_version"]
        != "engram.extension-closed-loop-publication-authority.v1"
        or authority["store_id"] != store_id
        or authority["terminal_receipt_sha256"] != terminal_receipt_sha256
        or authority["study_run_id"] != study_run_id
        or authority["admission_mode"] != "reserved"
        or authority["publication_admission_anchor_sha256"] != anchor["anchor_sha256"]
        or authority["publication_wal_sha256"] != publication_wal_sha256
        or authority["evidence_bundle_sha256"] != evidence_bundle_sha256
        or authority["reservation_id"] != reservation_id
        or authority["reservation_sha256"] != reservation["reservation_sha256"]
        or authority["reservation_finalization_sha256"]
        != finalization["finalization_sha256"]
        or authority["nest_work_admission_sha256"] != nest_work_admission_sha256
        or observation["publication_authority_sha256"] != authority["authority_sha256"]
    ):
        raise MatrixError("CREBAIN receipt-store sidecar lineage differs")

    if raw_capture is not None:
        terminal = raw_capture["terminal_receipt"]
        evidence = raw_capture["nest_evidence_bundle"]
        session = evidence.get("nest_session_readback")
        work_admission = (
            session.get("work_admission") if isinstance(session, dict) else None
        )
        reviewed = raw_capture.get("reviewed_native_runtime")
        reviewed_handshake = (
            reviewed.get("handshake_receipt") if isinstance(reviewed, dict) else None
        )
        lifecycle = terminal.get("runtime_lifecycle")
        if not isinstance(work_admission, dict):
            raise MatrixError("CREBAIN receipt-store NEST work admission is absent")
        work_admission_sha256 = require_canonical_digest(
            work_admission,
            "receipt_sha256",
            "CREBAIN NEST work admission",
        )
        reviewed_handshake = require_keys(
            reviewed_handshake,
            REVIEWED_HANDSHAKE_FIELDS,
            "CREBAIN reviewed-native handshake",
        )
        require_canonical_digest(
            reviewed_handshake,
            "receipt_sha256",
            "CREBAIN reviewed-native handshake",
        )
        if (
            handshake != reviewed_handshake
            or reservation["closed_loop_definition_sha256"]
            != terminal.get("closed_loop_definition_sha256")
            or reservation["nest_work_admission_sha256"] != work_admission_sha256
            or reservation["nest_configuration_sha256"]
            != work_admission.get("controller_configuration_sha256")
            or reservation["expected_runtime_binding_sha256"]
            != terminal.get("runtime_binding_sha256")
            or reservation["run_plan_sha256"]
            != managed_runtime_digest(raw_capture["run_plan"])
            or reservation["nest_configuration_sha256"]
            != managed_runtime_digest(raw_capture["nest_config"])
            or reservation["runtime_generation_id"]
            != (lifecycle.get("generation_id") if isinstance(lifecycle, dict) else None)
            or reservation["reserved_evidence_bytes"]
            != work_admission.get("estimated_evidence_bundle_bytes")
        ):
            raise MatrixError("CREBAIN receipt-store source lineage differs")

    material = {
        "store.json": managed_runtime_canonical(metadata),
        "writer.lock": RECEIPT_STORE_LOCK_BYTES,
        finalization_path: managed_runtime_canonical(finalization),
        observation_path: managed_runtime_canonical(observation),
        anchor_path: managed_runtime_canonical(anchor),
        authority_path: managed_runtime_canonical(authority),
    }
    expected_rows = [
        {
            "relative_path": evidence_path,
            "size_bytes": evidence_artifact_size_bytes,
            "sha256": evidence_bundle_sha256,
        },
        *(
            {
                "relative_path": relative_path,
                "size_bytes": len(payload),
                "sha256": digest_bytes(payload),
            }
            for relative_path, payload in material.items()
        ),
        {
            "relative_path": receipt_path,
            "size_bytes": terminal_artifact_size_bytes,
            "sha256": terminal_receipt_sha256,
        },
    ]
    expected_rows.sort(key=lambda row: row["relative_path"])
    return {
        "reservation_id": reservation_id,
        "sidecars_sha256": sidecars["closure_sha256"],
        "expected_rows": expected_rows,
        "receipt_path": receipt_path,
        "evidence_path": evidence_path,
        "finalization_path": finalization_path,
    }


def validate_receipt_store_closure(
    capture: dict[str, Any],
    index_row: dict[str, Any],
) -> dict[str, Any]:
    closure = require_keys(
        capture["receipt_store_closure"],
        RECEIPT_STORE_CLOSURE_FIELDS,
        "CREBAIN receipt-store closure",
    )
    files = require_list(
        closure["files"],
        "CREBAIN receipt-store file roster",
        minimum=RECEIPT_STORE_FILE_COUNT,
        maximum=RECEIPT_STORE_FILE_COUNT,
    )
    paths: list[str] = []
    total_bytes = 0
    for item in files:
        row = require_keys(
            item,
            {"relative_path", "size_bytes", "sha256"},
            "CREBAIN receipt-store file",
        )
        relative = safe_relative(
            row["relative_path"],
            "CREBAIN receipt-store file path",
        )
        size_bytes = row["size_bytes"]
        if (
            isinstance(size_bytes, bool)
            or not isinstance(size_bytes, int)
            or not 1 <= size_bytes <= MAX_CAPTURE_BYTES
            or not valid_sha256(row["sha256"])
        ):
            raise MatrixError("CREBAIN receipt-store file identity differs")
        relative_text = relative.as_posix()
        paths.append(relative_text)
        total_bytes += size_bytes
    terminal = require_keys(
        capture["terminal_receipt"],
        TERMINAL_FIELDS,
        "CREBAIN embedded terminal receipt",
    )
    evidence = require_keys(
        capture["nest_evidence_bundle"],
        NEST_BUNDLE_FIELDS,
        "CREBAIN embedded NEST evidence bundle",
    )
    summary = require_keys(
        capture.get("summary"),
        SUMMARY_FIELDS,
        "CREBAIN receipt-store run summary",
    )
    validate_recorded_summary_authority(summary)
    study_run_id = summary["study_run_id"]
    terminal_payload = managed_runtime_canonical(
        {key: value for key, value in terminal.items() if key != "receipt_sha256"}
    )
    evidence_payload = managed_runtime_canonical(
        {key: value for key, value in evidence.items() if key != "bundle_sha256"}
    )
    receipt_sha256 = terminal["receipt_sha256"]
    evidence_sha256 = evidence["bundle_sha256"]
    sidecar_review = validate_receipt_store_sidecars(
        capture.get("receipt_store_sidecars"),
        store_id=closure["store_id"],
        terminal_receipt_sha256=receipt_sha256,
        terminal_artifact_size_bytes=len(terminal_payload),
        evidence_bundle_sha256=evidence_sha256,
        evidence_artifact_size_bytes=len(evidence_payload),
        study_run_id=study_run_id,
        run_status=terminal.get("status"),
        terminal_reason_code=terminal.get("terminal_reason_code"),
        package_generation_id=capture["package_generation_id"],
        raw_capture=capture,
    )
    reservation_id = sidecar_review["reservation_id"]
    expected_receipt_path = sidecar_review["receipt_path"]
    expected_evidence_path = sidecar_review["evidence_path"]
    expected_reservation_path = sidecar_review["finalization_path"]
    expected_paths = [row["relative_path"] for row in sidecar_review["expected_rows"]]
    receipt_path = safe_relative(
        closure["receipt_artifact_path"],
        "CREBAIN terminal receipt artifact path",
        suffix=".json",
    ).as_posix()
    evidence_path = safe_relative(
        closure["evidence_artifact_path"],
        "CREBAIN NEST evidence artifact path",
        suffix=".json",
    ).as_posix()
    expected_receipt_identity = {
        "relative_path": expected_receipt_path,
        "size_bytes": len(terminal_payload),
        "sha256": receipt_sha256,
    }
    expected_evidence_identity = {
        "relative_path": expected_evidence_path,
        "size_bytes": len(evidence_payload),
        "sha256": evidence_sha256,
    }
    if (
        paths != expected_paths
        or files != sidecar_review["expected_rows"]
        or len({row["sha256"] for row in files}) != RECEIPT_STORE_FILE_COUNT
        or total_bytes > MAX_RECEIPT_STORE_BYTES
        or closure["schema_version"] != "crebain.closed-loop-receipt-store-closure.v1"
        or not valid_prefixed_sha256(closure["store_id"], "clrs_")
        or closure["store_id"] != index_row["receipt_store_id"]
        or closure["store_id"] != summary.get("store_id")
        or reservation_id != summary.get("reservation_id")
        or summary.get("status") != "recorded"
        or summary.get("run_status") != "completed"
        or summary.get("terminal_reason_code") != "loop.completed"
        or summary.get("receipt_sha256") != receipt_sha256
        or summary.get("evidence_bundle_sha256") != evidence_sha256
        or terminal.get("study_run_id") != study_run_id
        or terminal.get("status") != "completed"
        or terminal.get("terminal_reason_code") != "loop.completed"
        or evidence.get("study_run_id") != study_run_id
        or evidence.get("run_receipt_sha256") != receipt_sha256
        or closure["receipt_sha256"] != index_row["receipt_sha256"]
        or closure["evidence_bundle_sha256"] != index_row["evidence_bundle_sha256"]
        or closure["file_count"] != len(files)
        or closure["file_count"] != RECEIPT_STORE_FILE_COUNT
        or closure["total_bytes"] != total_bytes
        or receipt_sha256 != closure["receipt_sha256"]
        or evidence_sha256 != closure["evidence_bundle_sha256"]
        or receipt_sha256 == evidence_sha256
        or digest_bytes(terminal_payload) != receipt_sha256
        or digest_bytes(evidence_payload) != evidence_sha256
        or receipt_path != expected_receipt_path
        or evidence_path != expected_evidence_path
        or sum(row["sha256"] == receipt_sha256 for row in files) != 1
        or sum(row["sha256"] == evidence_sha256 for row in files) != 1
        or closure["closure_sha256"] != index_row["receipt_store_closure_sha256"]
        or closure["closure_sha256"] != digest_without(closure, "closure_sha256")
    ):
        raise MatrixError("CREBAIN receipt-store closure differs")
    return {
        "receipt_store_id": closure["store_id"],
        "receipt_store_closure_sha256": closure["closure_sha256"],
        "receipt_store_file_count": len(files),
        "receipt_store_files": files,
        "receipt_store_file_roster_sha256": digest_bytes(canonical(files)),
        "receipt_store_sidecars": capture["receipt_store_sidecars"],
        "receipt_store_sidecars_sha256": sidecar_review["sidecars_sha256"],
        "reservation_id": reservation_id,
        "finalized_reservation_artifact_path": expected_reservation_path,
        "terminal_artifact_path": receipt_path,
        "terminal_artifact_size_bytes": len(terminal_payload),
        "terminal_artifact_exact_sha256": expected_receipt_identity["sha256"],
        "evidence_artifact_path": evidence_path,
        "evidence_artifact_size_bytes": len(evidence_payload),
        "evidence_artifact_exact_sha256": expected_evidence_identity["sha256"],
    }


def expected_population_names(roster: dict[str, Any]) -> list[str]:
    return [
        f"{prefix}.d{action_index:02}.{sign}"
        for prefix in roster["population_prefixes"]
        for action_index in range(3)
        for sign in ("negative", "positive")
    ]


def expected_population_roster(roster: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "channel_id": channel_id,
            "population_names": [
                f"{prefix}.d{action_index:02}.{sign}"
                for action_index in range(3)
                for sign in ("negative", "positive")
            ],
        }
        for channel_id, prefix in zip(
            roster["channel_ids"],
            roster["population_prefixes"],
            strict=True,
        )
    ]


def validate_population_topology(
    capture: dict[str, Any],
    index_row: dict[str, Any],
    roster: dict[str, Any],
) -> dict[str, Any]:
    topology = require_keys(
        capture["population_topology"],
        POPULATION_TOPOLOGY_FIELDS,
        "CREBAIN population topology",
    )
    population_names = expected_population_names(roster)
    population_roster = expected_population_roster(roster)
    population_size = capture["nest_config"]["population_size"]
    drone_count = index_row["drone_count"]
    expected = {
        "session_count": 1,
        "drone_count": drone_count,
        "action_axis_count": drone_count * 3,
        "population_count": drone_count * 6,
        "population_neuron_count": drone_count * 6 * population_size,
        "device_node_count": drone_count * 12,
        "connection_count": drone_count * 12 * population_size,
        "population_names": population_names,
        "derived_population_roster_sha256": digest_bytes(canonical(population_names)),
    }
    if topology != expected or any(
        topology[field] != index_row[field]
        for field in (
            "session_count",
            "population_count",
            "population_neuron_count",
            "device_node_count",
            "connection_count",
        )
    ):
        raise MatrixError("CREBAIN population-topology join differs")
    evidence = capture.get("nest_evidence_bundle")
    session = (
        evidence.get("nest_session_readback") if isinstance(evidence, dict) else None
    )
    work = session.get("work_admission") if isinstance(session, dict) else None
    executions = (
        evidence.get("step_execution_receipts") if isinstance(evidence, dict) else None
    )
    neural_steps = capture.get("neural_steps")
    if (
        not isinstance(session, dict)
        or not isinstance(work, dict)
        or not isinstance(executions, list)
        or not executions
        or not isinstance(neural_steps, list)
        or len(neural_steps) != len(executions)
    ):
        raise MatrixError("CREBAIN successful NEST topology evidence is absent")
    reported_population_roster = require_list(
        session.get("population_roster"),
        "CREBAIN NEST population roster",
        minimum=drone_count,
        maximum=drone_count,
    )
    for row in reported_population_roster:
        require_keys(
            row,
            SESSION_POPULATION_ROSTER_FIELDS,
            "CREBAIN NEST population roster row",
        )
    if (
        reported_population_roster != population_roster
        or session.get("population_roster_sha256")
        != digest_bytes(canonical(reported_population_roster))
        or work.get("expected_population_roster_sha256")
        != session.get("population_roster_sha256")
    ):
        raise MatrixError("CREBAIN NEST population roster bytes differ")
    expected_connections = [
        (population_name, direction)
        for population_name in population_names
        for direction in ("input", "recorder")
    ]
    connection_readbacks = require_list(
        session.get("connection_readbacks"),
        "CREBAIN NEST connection readbacks",
        minimum=len(expected_connections),
        maximum=len(expected_connections),
    )
    for row in connection_readbacks:
        require_keys(
            row,
            SESSION_CONNECTION_READBACK_FIELDS,
            "CREBAIN NEST connection readback",
        )
    if (
        [(row["population_name"], row["direction"]) for row in connection_readbacks]
        != expected_connections
        or any(
            row["connection_count"] != population_size for row in connection_readbacks
        )
        or session.get("connection_readback_sha256")
        != digest_bytes(canonical(connection_readbacks))
    ):
        raise MatrixError("CREBAIN NEST connection roster differs")
    expected_axes = [
        (channel_id, action_index)
        for channel_id in roster["channel_ids"]
        for action_index in range(3)
    ]
    control_bindings: list[dict[str, Any]] | None = None
    named_readback_fields = (
        ("generator_schedule_readbacks", "generator_schedule_readback_sha256"),
        ("input_weight_readbacks", "input_weight_readback_sha256"),
        ("completed_window_readbacks", "completed_window_readback_sha256"),
    )
    for step_index, (execution, neural_step) in enumerate(
        zip(executions, neural_steps, strict=True),
        start=1,
    ):
        if not isinstance(execution, dict) or not isinstance(neural_step, dict):
            raise MatrixError("CREBAIN NEST topology step is not an object")
        if execution.get("receipt_sha256") != digest_without(
            execution,
            "receipt_sha256",
        ):
            raise MatrixError(f"CREBAIN NEST step {step_index} digest differs")
        for rows_field, digest_field in named_readback_fields:
            rows = require_list(
                execution.get(rows_field),
                f"CREBAIN NEST step {step_index} {rows_field}",
                minimum=len(population_names),
                maximum=len(population_names),
            )
            if (
                any(not isinstance(row, dict) for row in rows)
                or [row.get("population_name") for row in rows] != population_names
                or execution.get(digest_field) != digest_bytes(canonical(rows))
            ):
                raise MatrixError(
                    f"CREBAIN NEST step {step_index} {rows_field} differs"
                )
        event_deltas = require_list(
            execution.get("population_event_deltas"),
            f"CREBAIN NEST step {step_index} population event deltas",
            minimum=len(population_names),
            maximum=len(population_names),
        )
        if (
            any(not isinstance(row, dict) for row in event_deltas)
            or [row.get("population_name") for row in event_deltas] != population_names
        ):
            raise MatrixError(
                f"CREBAIN NEST step {step_index} population event roster differs"
            )
        safety = require_list(
            execution.get("channel_safety_readbacks"),
            f"CREBAIN NEST step {step_index} channel safety readbacks",
            minimum=drone_count,
            maximum=drone_count,
        )
        if (
            any(not isinstance(row, dict) for row in safety)
            or [row.get("channel_id") for row in safety] != roster["channel_ids"]
            or execution.get("channel_safety_readback_sha256")
            != digest_bytes(canonical(safety))
        ):
            raise MatrixError(
                f"CREBAIN NEST step {step_index} channel safety roster differs"
            )
        encoded = require_list(
            execution.get("encoded_control_inputs"),
            f"CREBAIN NEST step {step_index} encoded controls",
            minimum=len(expected_axes),
            maximum=len(expected_axes),
        )
        if (
            any(not isinstance(row, dict) for row in encoded)
            or [(row.get("channel_id"), row.get("action_index")) for row in encoded]
            != expected_axes
            or execution.get("control_encoding_sha256")
            != digest_bytes(canonical(encoded))
        ):
            raise MatrixError(
                f"CREBAIN NEST step {step_index} control axis roster differs"
            )
        derived_bindings: list[dict[str, Any]] = []
        for channel_ordinal, channel_id in enumerate(roster["channel_ids"]):
            channel_axes = encoded[channel_ordinal * 3 : (channel_ordinal + 1) * 3]
            codecs = [row.get("neural_codec_sha256") for row in channel_axes]
            axis_digests = [row.get("axis_binding_sha256") for row in channel_axes]
            if (
                len(set(codecs)) != 1
                or not valid_sha256(codecs[0])
                or any(not valid_sha256(value) for value in axis_digests)
            ):
                raise MatrixError(
                    f"CREBAIN NEST step {step_index} control identity differs"
                )
            derived_bindings.append(
                {
                    "channel_id": channel_id,
                    "neural_codec_sha256": codecs[0],
                    "axis_binding_sha256s": axis_digests,
                }
            )
        if control_bindings is None:
            control_bindings = derived_bindings
        elif derived_bindings != control_bindings:
            raise MatrixError("CREBAIN NEST control bindings changed between steps")
        request = neural_step.get("request")
        result = neural_step.get("result")
        proposals = result.get("proposals") if isinstance(result, dict) else None
        if (
            not isinstance(request, dict)
            or not isinstance(proposals, list)
            or [row.get("channel_id") for row in request.get("channels", [])]
            != roster["channel_ids"]
            or [row.get("channel_id") for row in proposals if isinstance(row, dict)]
            != roster["channel_ids"]
            or any(
                not isinstance(proposal, dict)
                or proposal.get("source_populations")
                != population_roster[channel_ordinal]["population_names"]
                for channel_ordinal, proposal in enumerate(proposals)
            )
        ):
            raise MatrixError(f"CREBAIN NEST step {step_index} neural topology differs")
    reported_control_bindings = require_list(
        session.get("control_bindings"),
        "CREBAIN NEST control bindings",
        minimum=drone_count,
        maximum=drone_count,
    )
    for row in reported_control_bindings:
        require_keys(
            row,
            SESSION_CONTROL_BINDING_FIELDS,
            "CREBAIN NEST control binding",
        )
    if (
        control_bindings is None
        or reported_control_bindings != control_bindings
        or session.get("control_binding_sha256")
        != digest_bytes(canonical(reported_control_bindings))
        or work.get("expected_control_binding_sha256")
        != session.get("control_binding_sha256")
    ):
        raise MatrixError("CREBAIN NEST control-binding bytes differ")
    return {
        "derived_population_roster_sha256": topology[
            "derived_population_roster_sha256"
        ],
        "population_roster_sha256": session["population_roster_sha256"],
        "control_binding_sha256": session["control_binding_sha256"],
        "connection_readback_sha256": session["connection_readback_sha256"],
        "population_count": topology["population_count"],
        "population_neuron_count": topology["population_neuron_count"],
        "device_node_count": topology["device_node_count"],
        "connection_count": topology["connection_count"],
    }


def validate_worker_guardian_closure(capture: dict[str, Any]) -> dict[str, Any]:
    closure = require_keys(
        capture["nest_worker_guardian_closure"],
        WORKER_GUARDIAN_CLOSURE_FIELDS,
        "CREBAIN NEST worker guardian closure",
    )
    evidence = capture["nest_evidence_bundle"]
    binding = evidence.get("worker_session_binding")
    identity = evidence.get("worker_runtime_identity")
    lifecycle = evidence.get("worker_lifecycle_receipt")
    session = evidence.get("nest_session_readback")
    attempts = evidence.get("worker_termination_attempt_receipts")
    if (
        not isinstance(binding, dict)
        or not isinstance(identity, dict)
        or not isinstance(lifecycle, dict)
        or not isinstance(session, dict)
        or not isinstance(attempts, list)
        or not 1 <= len(attempts) <= 64
    ):
        raise MatrixError("CREBAIN NEST worker guardian evidence is incomplete")
    binding_sha256 = binding.get("receipt_sha256")
    identity_sha256 = identity.get("receipt_sha256")
    lifecycle_sha256 = lifecycle.get("receipt_sha256")
    worker_pid = lifecycle.get("worker_pid")
    worker_source_sha256 = lifecycle.get("worker_source_sha256")
    worker_command_sha256 = lifecycle.get("worker_command_sha256")
    if (
        any(
            not valid_sha256(value)
            for value in (binding_sha256, identity_sha256, lifecycle_sha256)
        )
        or binding_sha256 != digest_without(binding, "receipt_sha256")
        or identity_sha256 != digest_without(identity, "receipt_sha256")
        or lifecycle_sha256 != digest_without(lifecycle, "receipt_sha256")
        or isinstance(worker_pid, bool)
        or not isinstance(worker_pid, int)
        or worker_pid < 1
        or not valid_sha256(worker_source_sha256)
        or not valid_sha256(worker_command_sha256)
        or lifecycle.get("termination_attempts") != attempts
        or lifecycle.get("session_binding_receipt_sha256") != binding_sha256
        or lifecycle.get("runtime_identity_receipt_sha256") != identity_sha256
        or binding.get("worker_runtime_identity_sha256") != identity_sha256
        or binding.get("child_session_receipt_sha256") != session.get("receipt_sha256")
        or binding.get("child_lineage_verified") is not True
        or binding.get("loaded_bytes_attested") is not False
        or binding.get("response_bound_loaded_bytes") is not False
        or binding.get("ncp_transport") is not False
        or binding.get("scientific_authority") is not False
        or lifecycle.get("termination_attempt_roster_sha256")
        != digest_bytes(canonical(attempts))
        or lifecycle.get("child_reaped") is not True
        or lifecycle.get("containment_empty") is not True
        or lifecycle.get("diagnostic_stream_complete") is not True
        or lifecycle.get("hard_deadline_enforcement") is not True
        or lifecycle.get("ncp_transport") is not False
        or lifecycle.get("physical_authority") is not False
        or lifecycle.get("scientific_authority") is not False
    ):
        raise MatrixError("CREBAIN NEST worker guardian evidence differs")
    for expected_index, attempt in enumerate(attempts, start=1):
        if (
            not isinstance(attempt, dict)
            or not valid_sha256(attempt.get("receipt_sha256"))
            or attempt.get("receipt_sha256")
            != digest_without(attempt, "receipt_sha256")
            or attempt.get("attempt_index") != expected_index
            or attempt.get("worker_pid") != lifecycle.get("worker_pid")
            or attempt.get("worker_source_sha256")
            != lifecycle.get("worker_source_sha256")
            or attempt.get("worker_command_sha256")
            != lifecycle.get("worker_command_sha256")
            or attempt.get("child_reaped") is not True
            or attempt.get("containment_empty") is not True
            or attempt.get("diagnostic_stream_complete") is not True
            or attempt.get("hard_deadline_enforcement") is not True
            or attempt.get("ncp_transport") is not False
            or attempt.get("physical_authority") is not False
            or attempt.get("scientific_authority") is not False
        ):
            raise MatrixError("CREBAIN NEST worker termination attempt differs")
    expected_closure = {
        "worker_session_binding_receipt_sha256": binding_sha256,
        "worker_runtime_identity_receipt_sha256": identity_sha256,
        "worker_lifecycle_receipt_sha256": lifecycle_sha256,
        "termination_attempt_count": len(attempts),
        "termination_attempt_roster_sha256": digest_bytes(canonical(attempts)),
        "worker_pid": worker_pid,
        "worker_source_sha256": worker_source_sha256,
        "worker_command_sha256": worker_command_sha256,
        "child_reaped": True,
        "containment_empty": True,
        "diagnostic_stream_complete": True,
    }
    if closure != expected_closure:
        raise MatrixError("CREBAIN NEST worker guardian closure differs")
    return {
        "worker_session_binding_receipt_sha256": binding_sha256,
        "worker_runtime_identity_receipt_sha256": identity_sha256,
        "worker_lifecycle_receipt_sha256": lifecycle_sha256,
        "termination_attempt_roster_sha256": closure[
            "termination_attempt_roster_sha256"
        ],
    }


def validate_lifecycle(capture: dict[str, Any]) -> dict[str, Any]:
    terminal = capture["terminal_receipt"]
    installed_proof = capture["installed_package_proof"]
    lifecycle = require_keys(
        terminal.get("runtime_lifecycle"),
        RUNTIME_LIFECYCLE_FIELDS,
        "CREBAIN runtime lifecycle",
    )
    reviewed = require_keys(
        capture["reviewed_native_runtime"],
        REVIEWED_NATIVE_RUNTIME_FIELDS,
        "CREBAIN reviewed runtime",
    )
    handshake = require_keys(
        reviewed["handshake_receipt"],
        REVIEWED_HANDSHAKE_FIELDS,
        "CREBAIN reviewed runtime handshake",
    )
    termination = require_keys(
        reviewed["termination_receipt"],
        REVIEWED_TERMINATION_FIELDS,
        "CREBAIN reviewed runtime termination",
    )
    handshake_sha = handshake["receipt_sha256"]
    termination_sha = termination["receipt_sha256"]
    handshake_digest_fields = (
        "executable_sha256",
        "validator_set_sha256",
        "exec_gate_command_sha256",
        "exec_gate_source_sha256",
        "generation_directory_identity_sha256",
        "host_handshake_frame_sha256",
        "runtime_handshake_frame_sha256",
        "guardian_source_sha256",
        "guardian_command_sha256",
        "guardian_ready_frame_sha256",
        "sandbox_profile_sha256",
        "sandbox_launcher_sha256",
        "receipt_sha256",
    )
    if (
        any(not valid_sha256(handshake[field]) for field in handshake_digest_fields)
        or not valid_sha256(termination_sha)
        or handshake_sha != digest_without(handshake, "receipt_sha256")
        or termination_sha != digest_without(termination, "receipt_sha256")
    ):
        raise MatrixError("CREBAIN reviewed runtime receipt digest differs")
    guardian_pid = handshake["guardian_pid"]
    process_pid = handshake["process_pid"]
    process_group_id = handshake["process_group_id"]
    session_id = handshake["session_id"]
    source_closure = capture.get("engram_source_closure")
    source_rows = (
        source_closure.get("sources") if isinstance(source_closure, dict) else None
    )
    exec_gate_rows = (
        [
            row
            for row in source_rows
            if isinstance(row, dict)
            and row.get("relative_path")
            == "backend/integrations/contained_exec_gate.py"
        ]
        if isinstance(source_rows, list)
        else []
    )
    nest_evidence = capture.get("nest_evidence_bundle")
    worker_identity = (
        nest_evidence.get("worker_runtime_identity")
        if isinstance(nest_evidence, dict)
        else None
    )
    validate_reviewed_exec_gate_binding(
        reviewed["exec_gate_command_binding"],
        handshake,
        closure_source_sha256=(
            source_closure.get("reviewed_runtime_exec_gate_source_sha256")
            if isinstance(source_closure, dict)
            else None
        ),
        closure_command_sha256=(
            source_closure.get("reviewed_runtime_exec_gate_command_sha256")
            if isinstance(source_closure, dict)
            else None
        ),
        source_row_sha256=(
            exec_gate_rows[0].get("sha256") if len(exec_gate_rows) == 1 else None
        ),
        runtime_files_value=(
            worker_identity.get("files") if isinstance(worker_identity, dict) else None
        ),
    )
    expected_lifecycle = {
        "schema_version": "engram.closed-loop-runtime-lifecycle-binding.v1",
        "profile": EXPECTED_REVIEWED_PROFILE,
        "generation_id": handshake["generation_id"],
        "launch_source": "package-store-lease",
        "store_id": installed_proof["store_id"],
        "package_generation_id": capture["package_generation_id"],
        "generation_directory_identity_sha256": handshake[
            "generation_directory_identity_sha256"
        ],
        "package_generation_lease_retained_at_launch": True,
        "package_generation_lease_released": True,
        "handshake_receipt_sha256": handshake_sha,
        "termination_receipt_sha256": termination_sha,
        "termination_disposition": "clean-exit",
        "child_reaped": True,
        "containment_empty": True,
        "diagnostic_stream_complete": True,
        "private_work_directory_removed": True,
        "publisher_authenticated": False,
        "durable_process_launch_authority": False,
        "ncp_authority": False,
        "physical_authority": False,
        "scientific_authority": False,
    }
    if (
        any(
            lifecycle.get(field) != expected
            for field, expected in expected_lifecycle.items()
        )
        or lifecycle["binding_sha256"] != digest_without(lifecycle, "binding_sha256")
        or reviewed["lifecycle_binding_sha256"] != lifecycle["binding_sha256"]
        or reviewed["guardian_closure_verified"] is not True
        or reviewed["package_store_lineage_verified"] is not True
    ):
        raise MatrixError("CREBAIN reviewed runtime lifecycle differs")
    if (
        termination["schema_version"]
        != "engram.reviewed-native-development-termination.v1"
        or termination["handshake_receipt_sha256"] != handshake_sha
        or termination["generation_id"] != handshake["generation_id"]
        or termination["disposition"] != "clean-exit"
        or termination["reason_code"] != "runtime.clean-exit"
        or termination["exit_code"] != 0
        or termination["termination_signal"] is not None
        or termination["child_reaped"] is not True
        or termination["guardian_pid"] != guardian_pid
        or termination["process_group_id"] != process_group_id
        or termination["guardian_reaped"] is not True
        or termination["group_signal_while_guardian_unreaped"] is not True
        or termination["direct_child_signal_while_unreaped"] is not False
        or termination["containment_signal_scope"] != "process-group"
        or termination["containment_seal_signal"] != 9
        or termination["containment_empty"] is not True
        or termination["stderr_sha256"] != digest_bytes(b"")
        or termination["stderr_retained_bytes"] != 0
        or termination["stderr_truncated"] is not False
        or termination["diagnostic_stream_complete"] is not True
        or termination["private_work_directory_removed"] is not True
        or termination["package_generation_lease_released"] is not True
        or termination["guardian_generation_lease_held_until_containment"] is not True
        or any(
            termination[field] is not False
            for field in (
                "durable_process_launch_authority",
                "ncp_authority",
                "physical_authority",
                "scientific_authority",
            )
        )
    ):
        raise MatrixError("CREBAIN reviewed runtime termination differs")
    if (
        handshake["guardian_source_sha256"]
        != capture["engram_source_closure"]["reviewed_runtime_guardian_source_sha256"]
        or len(exec_gate_rows) != 1
        or exec_gate_rows[0].get("sha256") != handshake["exec_gate_source_sha256"]
        or handshake["installation_id"] != installed_proof["installation_id"]
        or handshake["store_id"] != installed_proof["store_id"]
        or handshake["executable_sha256"] != installed_proof["executable_sha256"]
        or handshake["package_generation_id"] != capture["package_generation_id"]
        or handshake["schema_version"]
        != "engram.reviewed-native-development-handshake.v1"
        or handshake["generation_ordinal"] != 1
        or handshake["extension_id"] != EXPECTED_CREBAIN_EXTENSION_ID
        or handshake["extension_version"] != EXPECTED_CREBAIN_EXTENSION_VERSION
        or handshake["target_id"] != EXPECTED_CREBAIN_TARGET_ID
        or handshake["profile"] != EXPECTED_REVIEWED_PROFILE
        or handshake["launch_source"] != "package-store-lease"
        or handshake["package_generation_lease_retained"] is not True
        or handshake["guardian_generation_lease_retained"] is not True
        or isinstance(guardian_pid, bool)
        or not isinstance(guardian_pid, int)
        or guardian_pid <= 1
        or isinstance(process_pid, bool)
        or not isinstance(process_pid, int)
        or process_pid <= 1
        or isinstance(process_group_id, bool)
        or not isinstance(process_group_id, int)
        or process_group_id != process_pid
        or guardian_pid == process_pid
        or isinstance(session_id, bool)
        or not isinstance(session_id, int)
        or session_id <= 1
        or handshake["runtime_process_group_leader"] is not True
        or handshake["guardian_group_member"] is not True
        or handshake["handshake_transcript_accepted"] is not True
        or handshake["child_ready_claim"] is not False
        or handshake["host_local_admission"] is not True
        or handshake["process_launch_performed"] is not True
        or handshake["explicit_absolute_path_spawn"] is not True
        or handshake["path_lookup_at_spawn"] is not True
        or handshake["package_path_reopened_for_spawn"] is not False
        or handshake["verified_executable_staged"] is not True
        or handshake["staged_executable_owner_private"] is not True
        or handshake["staged_executable_user_immutable"] is not True
        or handshake["process_group_containment"] is not True
        or handshake["guardian_owner_loss_seal"] is not True
        or handshake["guardian_uncertainty_record_prepared"] is not True
        or handshake["descendant_creation_denied"] is not True
        or handshake["os_sandbox_enforced"] is not True
        or handshake["network_isolation_enforced"] is not True
        or handshake["filesystem_isolation_enforced"] is not False
        or handshake["external_dependency_closure_attested"] is not False
        or handshake["automatic_restart"] is not False
        or any(
            handshake[field] is not False
            for field in (
                "publisher_authenticated",
                "durable_process_launch_authority",
                "replayable_live_launch_authority",
                "ncp_authority",
                "physical_authority",
                "scientific_authority",
            )
        )
    ):
        raise MatrixError("CREBAIN reviewed runtime handshake differs")
    cleanup = terminal.get("cleanup")
    if not isinstance(cleanup, list) or len(cleanup) != 2:
        raise MatrixError("CREBAIN cleanup roster differs")
    runtime_cleanup, neural_cleanup = cleanup
    evidence = capture.get("nest_evidence_bundle")
    tail_receipt = (
        evidence.get("tail_disposition_receipt") if isinstance(evidence, dict) else None
    )
    worker_lifecycle = (
        evidence.get("worker_lifecycle_receipt") if isinstance(evidence, dict) else None
    )
    runtime_owner = terminal.get("runtime_binding_sha256")
    neural_owner = terminal.get("neural_provider_identity_sha256")
    if (
        not isinstance(runtime_cleanup, dict)
        or not isinstance(neural_cleanup, dict)
        or set(runtime_cleanup) != CLEANUP_FIELDS
        or set(neural_cleanup) != CLEANUP_FIELDS
        or runtime_cleanup["schema_version"] != "engram.closed-loop-cleanup.v2"
        or runtime_cleanup["component"] != "runtime"
        or runtime_cleanup["owner_identity_sha256"] != runtime_owner
        or runtime_cleanup["mode"] != "finish"
        or runtime_cleanup["attempted"] is not True
        or runtime_cleanup["confirmed"] is not True
        or runtime_cleanup["containment_empty"] is not True
        or runtime_cleanup["reason_code"] != "loop.completed"
        or runtime_cleanup["runtime_lifecycle"] != lifecycle
        or runtime_cleanup["provider_terminal_receipt_sha256"] is not None
        or runtime_cleanup["provider_lifecycle_receipt_sha256"] is not None
        or neural_cleanup["schema_version"] != "engram.closed-loop-cleanup.v2"
        or neural_cleanup["component"] != "neural"
        or neural_cleanup["owner_identity_sha256"] != neural_owner
        or neural_cleanup["mode"] != "close"
        or neural_cleanup["attempted"] is not True
        or neural_cleanup["confirmed"] is not True
        or neural_cleanup["containment_empty"] is not True
        or neural_cleanup["reason_code"] != "loop.completed"
        or neural_cleanup["runtime_lifecycle"] is not None
        or not isinstance(tail_receipt, dict)
        or neural_cleanup["provider_terminal_receipt_sha256"]
        != tail_receipt.get("receipt_sha256")
        or not isinstance(worker_lifecycle, dict)
        or neural_cleanup["provider_lifecycle_receipt_sha256"]
        != worker_lifecycle.get("receipt_sha256")
        or terminal.get("cleanup_complete") is not True
        or runtime_cleanup["receipt_sha256"]
        != managed_runtime_digest_without(runtime_cleanup, "receipt_sha256")
        or neural_cleanup["receipt_sha256"]
        != managed_runtime_digest_without(neural_cleanup, "receipt_sha256")
    ):
        raise MatrixError("CREBAIN runtime or neural cleanup differs")
    binding_sha = lifecycle["binding_sha256"]
    if (
        not valid_sha256(binding_sha)
        or not valid_sha256(runtime_owner)
        or not valid_sha256(neural_owner)
    ):
        raise MatrixError("CREBAIN runtime lifecycle binding digest differs")
    return {
        "runtime_handshake_receipt_sha256": handshake_sha,
        "runtime_termination_receipt_sha256": termination_sha,
        "runtime_lifecycle_binding_sha256": binding_sha,
        "exec_gate_command_sha256": handshake["exec_gate_command_sha256"],
        "exec_gate_source_sha256": handshake["exec_gate_source_sha256"],
        "runtime_process_group_containment_verified": True,
        "termination_disposition": "clean-exit",
        "child_reaped": True,
        "guardian_reaped": True,
        "containment_empty": True,
        "diagnostic_stream_complete": True,
        "private_work_directory_removed": True,
        "package_generation_lease_released": True,
        "cleanup_complete": True,
        "filesystem_isolation_enforced": False,
    }


def validate_external_summary(
    summary: dict[str, Any],
    terminal: dict[str, Any],
    evidence: dict[str, Any],
    expected_prisoma: dict[str, Any],
    expected_engram: dict[str, Any],
    *,
    verify_validator_source_bytes: bool = True,
    verify_engram_source_bytes: bool = True,
) -> dict[str, Any]:
    required = {
        "schema_version",
        "validation_scope",
        "prisoma_repository",
        "prisoma_validator_source_roster_sha256",
        "prisoma_validator_source_roster",
        "engram_repository",
        "engram_revision",
        "engram_imported_source_roster_sha256",
        "engram_imported_source_roster",
        "inputs",
        "lineage",
        "authority",
        "receipt_sha256",
    }
    require_keys(summary, required, "external NEST validation summary")
    if (
        summary["schema_version"]
        != "prisoma.observer.engram-nest-evidence-validation-summary.v1"
        or summary["validation_scope"] != "engram-exact-validator-rejoin-only"
        or summary["engram_revision"] != expected_engram["commit"]
        or summary["prisoma_repository"]
        != {key: value for key, value in expected_prisoma.items() if key != "root"}
        or summary["engram_repository"]
        != {key: value for key, value in expected_engram.items() if key != "root"}
        or not valid_sha256(summary["receipt_sha256"])
        or summary["receipt_sha256"] != digest_without(summary, "receipt_sha256")
    ):
        raise MatrixError("external NEST validation summary identity differs")
    validator_sources = require_list(
        summary["prisoma_validator_source_roster"],
        "Prisoma NEST validator-source roster",
        minimum=len(NEST_VALIDATOR_SOURCE_PATHS),
        maximum=len(NEST_VALIDATOR_SOURCE_PATHS),
    )
    validator_paths: list[str] = []
    for item in validator_sources:
        row = require_keys(
            item,
            {"path", "sha256", "git_blob", "byte_count"},
            "Prisoma NEST validator-source row",
        )
        relative = safe_relative(row["path"], "Prisoma NEST validator-source path")
        if (
            not valid_sha256(row["sha256"])
            or not valid_git_oid(
                row["git_blob"],
                expected_prisoma["object_format"],
            )
            or isinstance(row["byte_count"], bool)
            or not isinstance(row["byte_count"], int)
            or not 1 <= row["byte_count"] <= MAX_SCHEMA_BYTES
        ):
            raise MatrixError("Prisoma NEST validator-source row differs")
        validator_paths.append(relative.as_posix())
    if validator_paths != [path.as_posix() for path in NEST_VALIDATOR_SOURCE_PATHS]:
        raise MatrixError("Prisoma NEST validator-source path roster differs")
    expected_validator_sources = validator_sources
    if verify_validator_source_bytes:
        expected_validator_sources = capture_repository_files(
            expected_prisoma["root"],
            expected_prisoma["commit"],
            NEST_VALIDATOR_SOURCE_PATHS,
            MAX_SCHEMA_BYTES,
        )
    expected_validator_roster_sha256 = digest_bytes(
        b"prisoma-nest-summary-validator-source-roster-v1\0"
        + canonical(expected_validator_sources)
    )
    if (
        validator_sources != expected_validator_sources
        or summary["prisoma_validator_source_roster_sha256"]
        != expected_validator_roster_sha256
    ):
        raise MatrixError("Prisoma NEST validator source closure differs")
    imported_sources = require_list(
        summary["engram_imported_source_roster"],
        "external Engram imported-source roster",
        minimum=1,
        maximum=256,
    )
    imported_paths: list[str] = []
    imported_total_bytes = 0
    for source in imported_sources:
        row = require_keys(
            source,
            {"path", "sha256", "git_blob", "byte_count", "module_names"},
            "external Engram imported-source row",
        )
        relative = safe_relative(
            row["path"], "external Engram source path", suffix=".py"
        )
        modules = row["module_names"]
        byte_count = row["byte_count"]
        if (
            not valid_sha256(row["sha256"])
            or not valid_git_oid(
                row["git_blob"],
                expected_engram["object_format"],
            )
            or isinstance(byte_count, bool)
            or not isinstance(byte_count, int)
            or not 0 <= byte_count <= MAX_SOURCE_BYTES
            or not isinstance(modules, list)
            or not 1 <= len(modules) <= 16
            or modules != sorted(set(modules))
            or any(
                not isinstance(module, str)
                or not module
                or len(module.encode("utf-8")) > 256
                for module in modules
            )
        ):
            raise MatrixError("external Engram imported-source row differs")
        imported_paths.append(relative.as_posix())
        imported_total_bytes += byte_count
    expected_roster_sha256 = digest_bytes(
        b"prisoma-engram-imported-source-roster-v1\0" + canonical(imported_sources)
    )
    if (
        imported_paths != sorted(set(imported_paths))
        or imported_total_bytes > 64 * 1024 * 1024
        or summary["engram_imported_source_roster_sha256"] != expected_roster_sha256
    ):
        raise MatrixError("external Engram imported-source roster differs")
    if verify_engram_source_bytes:
        try:
            verify_committed_source_roster(
                expected_engram["root"],
                expected_engram["commit"],
                imported_sources,
                path_field="path",
                size_field="byte_count",
                mode_field=None,
                max_files=256,
                max_file_bytes=MAX_SOURCE_BYTES,
                max_total_bytes=64 * 1024 * 1024,
                allow_empty=True,
            )
        except (OSError, ValueError) as error:
            raise MatrixError(
                "external Engram source bytes do not reopen from committed Git"
            ) from error
    inputs = require_keys(
        summary["inputs"],
        {
            "summary_schema_exact_sha256",
            "run_receipt_exact_sha256",
            "evidence_bundle_exact_sha256",
            "source_run_receipt_sha256",
            "source_bundle_sha256",
            "validation_input_sha256",
        },
        "external NEST validation inputs",
    )
    if (
        inputs["source_run_receipt_sha256"] != terminal["receipt_sha256"]
        or inputs["source_bundle_sha256"] != evidence["bundle_sha256"]
        or inputs["summary_schema_exact_sha256"]
        != digest_bytes(snapshot_regular_file(NEST_SUMMARY_SCHEMA, MAX_SCHEMA_BYTES))
        or inputs["run_receipt_exact_sha256"]
        != digest_bytes(canonical(terminal) + b"\n")
        or inputs["evidence_bundle_exact_sha256"]
        != digest_bytes(canonical(evidence) + b"\n")
        or not all(valid_sha256(value) for value in inputs.values())
    ):
        raise MatrixError("external NEST validation input lineage differs")
    validation_input = {
        "prisoma_repository": summary["prisoma_repository"],
        "prisoma_validator_source_roster_sha256": summary[
            "prisoma_validator_source_roster_sha256"
        ],
        "engram_repository": summary["engram_repository"],
        "engram_imported_source_roster_sha256": summary[
            "engram_imported_source_roster_sha256"
        ],
        "summary_schema_exact_sha256": inputs["summary_schema_exact_sha256"],
        "run_receipt_exact_sha256": inputs["run_receipt_exact_sha256"],
        "evidence_bundle_exact_sha256": inputs["evidence_bundle_exact_sha256"],
        "source_run_receipt_sha256": inputs["source_run_receipt_sha256"],
        "source_bundle_sha256": inputs["source_bundle_sha256"],
    }
    if inputs["validation_input_sha256"] != digest_bytes(
        b"prisoma-engram-nest-validation-input-v1\0" + canonical(validation_input)
    ):
        raise MatrixError("external NEST validation source-input binding differs")
    lineage = require_keys(
        summary["lineage"],
        EXTERNAL_LINEAGE_FIELDS,
        "external NEST validation lineage",
    )
    if (
        lineage.get("study_run_id") != terminal["study_run_id"]
        or lineage.get("neural_provider_identity_sha256")
        != terminal["neural_provider_identity_sha256"]
        or lineage.get("neural_durable_evidence_profile")
        != "engram.nest-closed-loop-evidence-bundle.v2"
        or lineage.get("run_status") != "completed"
        or lineage.get("completed_step_count") != len(terminal["steps"])
        or lineage.get("provider_step_execution_count")
        != len(evidence["step_execution_receipts"])
        or lineage.get("validated_against_run") is not True
        or lineage.get("neural_cleanup_confirmed") is not True
    ):
        raise MatrixError("external NEST validation lineage differs")
    authority = require_keys(
        summary["authority"],
        EXTERNAL_AUTHORITY_FIELDS,
        "external NEST validation authority",
    )
    if (
        authority.get("descriptive_only") is not True
        or authority.get("source_durable_evidence_verified") is not True
        or authority.get("engram_loaded_source_bytes_attested") is not False
        or any(
            authority.get(field) is not False
            for field in (
                "agent_bridge_command",
                "execution_authority",
                "pid_result",
                "ncp_authority",
                "physical_authority",
                "scientific_authority",
                "is_paper_local_evidence",
                "calibrated_posterior",
            )
        )
    ):
        raise MatrixError("external NEST validation grants authority")
    return summary


def channel_roster(run_plan: dict[str, Any], drone_count: int) -> dict[str, Any]:
    require_keys(run_plan, RUN_PLAN_FIELDS, "CREBAIN run plan")
    channels = require_list(
        run_plan["channels"],
        "CREBAIN run-plan channels",
        minimum=drone_count,
        maximum=drone_count,
    )
    channel_ids: list[str] = []
    subject_ids: list[str] = []
    population_prefixes: list[str] = []
    action_width = 0
    for channel in channels:
        row = require_keys(channel, CHANNEL_FIELDS, "CREBAIN run-plan channel")
        for field, output in (
            ("channel_id", channel_ids),
            ("subject_id", subject_ids),
            ("neural_population_prefix", population_prefixes),
        ):
            value = row[field]
            if (
                not isinstance(value, str)
                or not value
                or len(value.encode("utf-8")) > 128
            ):
                raise MatrixError(f"CREBAIN channel {field} differs")
            output.append(value)
        width = row["action_width"]
        observation_width = row["observation_width"]
        observation_components = row["observation_components"]
        action_components = row["action_components"]
        bounds = (row["action_min"], row["action_max"], row["safe_action"])
        axes = row["neural_control_axes"]
        if (
            row["subject_kind"] != "simulated.drone"
            or isinstance(width, bool)
            or not isinstance(width, int)
            or not 1 <= width <= 64
            or isinstance(observation_width, bool)
            or not isinstance(observation_width, int)
            or not 1 <= observation_width <= 256
            or not isinstance(observation_components, list)
            or len(observation_components) != observation_width
            or not isinstance(action_components, list)
            or len(action_components) != width
            or not isinstance(axes, list)
            or len(axes) != width
            or any(
                not isinstance(values, list) or len(values) != width
                for values in bounds
            )
        ):
            raise MatrixError("CREBAIN channel dimensions or kind differ")
        component_identifiers: list[tuple[str, str]] = []
        for component in [*observation_components, *action_components]:
            component_row = require_keys(
                component,
                COMPONENT_FIELDS,
                "CREBAIN channel component",
            )
            identifier = (component_row["component_id"], component_row["unit_id"])
            if any(
                not isinstance(value, str)
                or not value
                or len(value.encode("utf-8")) > 128
                for value in identifier
            ):
                raise MatrixError("CREBAIN channel component identifier differs")
            component_identifiers.append(identifier)
        if len(set(component_identifiers)) != len(component_identifiers):
            raise MatrixError("CREBAIN channel components are not unique")
        for minimum, maximum, safe in zip(*bounds, strict=True):
            if (
                isinstance(minimum, bool)
                or isinstance(maximum, bool)
                or isinstance(safe, bool)
                or not all(
                    isinstance(value, (int, float))
                    for value in (minimum, maximum, safe)
                )
                or not all(
                    math.isfinite(float(value)) for value in (minimum, maximum, safe)
                )
                or not minimum <= safe <= maximum
            ):
                raise MatrixError("CREBAIN action bound or safe action differs")
        axis_indices: list[int] = []
        for axis in axes:
            axis_row = require_keys(axis, NEURAL_AXIS_FIELDS, "CREBAIN neural axis")
            terms = require_list(
                axis_row["terms"],
                "CREBAIN neural-axis terms",
                minimum=1,
                maximum=64,
            )
            action_index = axis_row["action_index"]
            gain = axis_row["decoded_action_gain"]
            if (
                isinstance(action_index, bool)
                or not isinstance(action_index, int)
                or not 0 <= action_index < width
                or axis_row["encoder"] != "affine-sum-clamped-v1"
                or isinstance(gain, bool)
                or not isinstance(gain, (int, float))
                or not math.isfinite(float(gain))
            ):
                raise MatrixError("CREBAIN neural-axis identity differs")
            axis_indices.append(action_index)
            observation_indices: list[int] = []
            for term in terms:
                term_row = require_keys(
                    term,
                    NEURAL_TERM_FIELDS,
                    "CREBAIN neural-axis term",
                )
                observation_index = term_row["observation_index"]
                numeric_values = (
                    term_row["gain_per_observation_unit"],
                    term_row["reference_value"],
                )
                if (
                    isinstance(observation_index, bool)
                    or not isinstance(observation_index, int)
                    or not 0 <= observation_index < observation_width
                    or any(isinstance(value, bool) for value in numeric_values)
                    or not all(
                        isinstance(value, (int, float)) for value in numeric_values
                    )
                    or not all(math.isfinite(float(value)) for value in numeric_values)
                ):
                    raise MatrixError("CREBAIN neural-axis term differs")
                observation_indices.append(observation_index)
            if len(set(observation_indices)) != len(observation_indices):
                raise MatrixError("CREBAIN neural-axis terms repeat an observation")
        if axis_indices != list(range(width)):
            raise MatrixError("CREBAIN neural-axis roster differs")
        action_width += width
    if (
        channel_ids != sorted(channel_ids)
        or len(set(channel_ids)) != drone_count
        or len(set(subject_ids)) != drone_count
        or len(set(population_prefixes)) != drone_count
    ):
        raise MatrixError(
            "CREBAIN channel and subject rosters are not unique and ordered"
        )
    return {
        "channels": channels,
        "channel_ids": channel_ids,
        "subject_ids": subject_ids,
        "population_prefixes": population_prefixes,
        "action_dimension_count": action_width,
    }


def validate_fault_and_neural_lineage(
    capture: dict[str, Any],
    roster: dict[str, Any],
    *,
    require_six_step_fault_cycle: bool,
) -> None:
    terminal = capture["terminal_receipt"]
    evidence = capture["nest_evidence_bundle"]
    neural_steps = capture["neural_steps"]
    steps = terminal["steps"]
    executions = terminal["neural_executions"]
    provider_steps = evidence["step_execution_receipts"]
    if not (
        isinstance(neural_steps, list)
        and isinstance(steps, list)
        and isinstance(executions, list)
        and isinstance(provider_steps, list)
        and len(neural_steps) == len(steps) == len(executions) == len(provider_steps)
    ):
        raise MatrixError("CREBAIN neural step roster length differs")
    channel_count = len(roster["channel_ids"])
    for index, (step, execution, provider, neural) in enumerate(
        zip(steps, executions, provider_steps, neural_steps, strict=True),
        start=1,
    ):
        require_keys(neural, NEURAL_STEP_FIELDS, "CREBAIN neural step")
        request = require_keys(
            neural["request"],
            NEURAL_REQUEST_FIELDS,
            "CREBAIN neural request",
        )
        result = require_keys(
            neural["result"],
            NEURAL_RESULT_FIELDS,
            "CREBAIN neural result",
        )
        if (
            not isinstance(step, dict)
            or not isinstance(execution, dict)
            or not isinstance(provider, dict)
        ):
            raise MatrixError("CREBAIN neural lineage row is not an object")
        if (
            step.get("step_index") != index
            or execution.get("step_index") != index
            or provider.get("step_index") != index
            or request["step_index"] != index
            or result["step_index"] != index
            or len(step.get("fault_codes", [])) != channel_count
            or len(request["channels"]) != channel_count
            or len(result["proposals"]) != channel_count
            or request["request_sha256"] != result["request_sha256"]
            or result["result_sha256"] != execution.get("neural_result_sha256")
            or result["provider_execution_scope"] != "nest-exact-step-readback"
            or result["provider_execution_scope"]
            != execution.get("provider_execution_scope")
            or result["provider_execution_sha256"] != provider.get("receipt_sha256")
            or result["provider_execution_sha256"]
            != execution.get("provider_execution_sha256")
            or step.get("neural_request_sha256") != request["request_sha256"]
            or step.get("neural_result_sha256") != result["result_sha256"]
            or step.get("provider_execution_sha256")
            != result["provider_execution_sha256"]
            or request["request_sha256"]
            != managed_runtime_digest_without(request, "request_sha256")
            or result["result_sha256"]
            != managed_runtime_digest_without(result, "result_sha256")
        ):
            raise MatrixError(f"CREBAIN neural lineage differs at step {index}")
        request_roster: list[tuple[str, str]] = []
        for channel_ordinal, channel in enumerate(request["channels"]):
            request_channel = require_keys(
                channel,
                NEURAL_CHANNEL_FIELDS,
                "CREBAIN neural request channel",
            )
            expected_channel = roster["channels"][channel_ordinal]
            values = request_channel["observation_values"]
            if (
                request_channel["channel_id"] != expected_channel["channel_id"]
                or request_channel["subject_id"] != expected_channel["subject_id"]
                or not isinstance(values, list)
                or len(values) != expected_channel["observation_width"]
                or any(
                    isinstance(value, bool)
                    or not isinstance(value, (int, float))
                    or not math.isfinite(float(value))
                    for value in values
                )
                or not isinstance(request_channel["fault_code"], str)
                or not request_channel["fault_code"]
                or not isinstance(request_channel["hold_required"], bool)
            ):
                raise MatrixError(
                    f"CREBAIN neural request roster differs at step {index}"
                )
            request_roster.append(
                (request_channel["channel_id"], request_channel["subject_id"])
            )
        proposal_roster: list[str] = []
        for channel_ordinal, proposal in enumerate(result["proposals"]):
            result_proposal = require_keys(
                proposal,
                NEURAL_PROPOSAL_FIELDS,
                "CREBAIN neural result proposal",
            )
            expected_channel = roster["channels"][channel_ordinal]
            values = result_proposal["values"]
            populations = result_proposal["source_populations"]
            expected_population_prefix = expected_channel["neural_population_prefix"]
            if (
                result_proposal["channel_id"] != expected_channel["channel_id"]
                or not isinstance(values, list)
                or len(values) != expected_channel["action_width"]
                or any(
                    isinstance(value, bool)
                    or not isinstance(value, (int, float))
                    or not math.isfinite(float(value))
                    for value in values
                )
                or not isinstance(populations, list)
                or len(populations) != expected_channel["action_width"] * 2
                or len(set(populations)) != len(populations)
                or any(
                    not isinstance(population, str)
                    or not population.startswith(f"{expected_population_prefix}.")
                    for population in populations
                )
            ):
                raise MatrixError(
                    f"CREBAIN neural result roster differs at step {index}"
                )
            proposal_roster.append(result_proposal["channel_id"])
        if (
            request_roster
            != list(zip(roster["channel_ids"], roster["subject_ids"], strict=True))
            or proposal_roster != roster["channel_ids"]
        ):
            raise MatrixError(f"CREBAIN neural channel join differs at step {index}")
    if not require_six_step_fault_cycle:
        return
    if len(steps) != 6:
        raise MatrixError("CREBAIN real-NEST proof does not contain six steps")
    normal_faults = ["none"] * channel_count
    scheduled_faults = list(normal_faults)
    scheduled_faults[0] = "sensor-unavailable"
    expected_faults = [
        normal_faults,
        normal_faults,
        scheduled_faults,
        normal_faults,
        normal_faults,
        normal_faults,
    ]
    if [step["fault_codes"] for step in steps] != expected_faults:
        raise MatrixError("CREBAIN scheduled fault sequence differs")
    hold_request = neural_steps[3]["request"]["channels"]
    hold_proposals = neural_steps[3]["result"]["proposals"]
    recovery_proposals = neural_steps[4]["result"]["proposals"]
    resume_proposals = neural_steps[5]["result"]["proposals"]
    if (
        hold_request[0].get("hold_required") is not True
        or any(
            channel.get("hold_required") is not False for channel in hold_request[1:]
        )
        or any(value != 0 for value in hold_proposals[0].get("values", []))
        or any(value != 0 for value in recovery_proposals[0].get("values", []))
        or not any(value != 0 for value in resume_proposals[0].get("values", []))
        or any(
            all(value == 0 for value in proposal.get("values", []))
            for proposal in hold_proposals[1:]
        )
        or any(
            all(value == 0 for value in proposal.get("values", []))
            for proposal in recovery_proposals[1:]
        )
    ):
        raise MatrixError("CREBAIN hold, washout, recovery, or lane isolation differs")
    hold_readback = provider_steps[3].get("channel_safety_readbacks", [None])[0]
    recovery_readback = provider_steps[4].get("channel_safety_readbacks", [None])[0]
    if (
        not isinstance(hold_readback, dict)
        or not isinstance(recovery_readback, dict)
        or hold_readback.get("hold_required") is not True
        or hold_readback.get("safety_washout_performed") is not True
        or hold_readback.get("population_state_reset_verified") is not True
        or recovery_readback.get("recovery_from_hold") is not True
        or recovery_readback.get("safety_washout_performed") is not True
        or recovery_readback.get("population_state_reset_verified") is not True
    ):
        raise MatrixError("CREBAIN NEST safety readback differs")
    for provider in provider_steps:
        safety = provider.get("channel_safety_readbacks")
        if not isinstance(safety, list) or len(safety) != channel_count:
            raise MatrixError("CREBAIN NEST safety roster differs")
        if any(not isinstance(channel, dict) for channel in safety) or any(
            channel.get("hold_required") is True
            or channel.get("recovery_from_hold") is True
            for channel in safety[1:]
        ):
            raise MatrixError("CREBAIN nonfaulted lane entered safety state")


def validate_nest_v2_execution_lineage(
    capture: dict[str, Any],
    roster: dict[str, Any],
) -> dict[str, Any]:
    """Validate the V2 launch, preparation, and per-step attempt closure."""

    evidence = capture["nest_evidence_bundle"]
    terminal = capture["terminal_receipt"]
    launch = require_keys(
        evidence["runtime_launch_expectation"],
        RUNTIME_LAUNCH_EXPECTATION_FIELDS,
        "CREBAIN NEST runtime launch expectation",
    )
    launch_attempt = require_keys(
        evidence["worker_launch_attempt"],
        WORKER_LAUNCH_ATTEMPT_FIELDS,
        "CREBAIN NEST worker launch attempt",
    )
    preparation_attempt = require_keys(
        evidence["preparation_attempt"],
        PREPARATION_ATTEMPT_FIELDS,
        "CREBAIN NEST preparation attempt",
    )
    capabilities = require_keys(
        evidence["child_capabilities"],
        CHILD_CAPABILITIES_FIELDS,
        "CREBAIN NEST child capabilities",
    )
    child_prepared = require_keys(
        evidence["child_preparation_receipt"],
        PREPARATION_RECEIPT_FIELDS,
        "CREBAIN NEST child preparation receipt",
    )
    provider_prepared = require_keys(
        evidence["provider_preparation_receipt"],
        PREPARATION_RECEIPT_FIELDS,
        "CREBAIN NEST provider preparation receipt",
    )
    binding = evidence.get("worker_session_binding")
    identity = evidence.get("worker_runtime_identity")
    worker_lifecycle = evidence.get("worker_lifecycle_receipt")
    session = evidence.get("nest_session_readback")
    if not all(
        isinstance(value, dict)
        for value in (binding, identity, worker_lifecycle, session)
    ):
        raise MatrixError("CREBAIN NEST V2 worker lineage is absent")

    launch_sha256 = launch["receipt_sha256"]
    launch_attempt_sha256 = launch_attempt["receipt_sha256"]
    preparation_attempt_sha256 = preparation_attempt["receipt_sha256"]
    child_prepared_sha256 = child_prepared["receipt_sha256"]
    provider_prepared_sha256 = provider_prepared["receipt_sha256"]
    receipt_digests = (
        launch_sha256,
        launch_attempt_sha256,
        preparation_attempt_sha256,
        child_prepared_sha256,
        provider_prepared_sha256,
    )
    if (
        any(not valid_sha256(value) for value in receipt_digests)
        or launch_sha256 != digest_without(launch, "receipt_sha256")
        or launch_attempt_sha256 != digest_without(launch_attempt, "receipt_sha256")
        or preparation_attempt_sha256
        != digest_without(preparation_attempt, "receipt_sha256")
        or child_prepared_sha256
        != managed_runtime_digest_without(child_prepared, "receipt_sha256")
        or provider_prepared_sha256
        != managed_runtime_digest_without(provider_prepared, "receipt_sha256")
        or evidence.get("bundle_sha256")
        != managed_runtime_digest_without(evidence, "bundle_sha256")
    ):
        raise MatrixError("CREBAIN NEST V2 receipt digest differs")

    runtime_files = require_list(
        launch["required_runtime_files"],
        "CREBAIN NEST required runtime-file roster",
        minimum=1,
        maximum=64,
    )
    runtime_roles: list[str] = []
    runtime_by_role: dict[str, dict[str, Any]] = {}
    for item in runtime_files:
        row = require_keys(
            item,
            RUNTIME_FILE_FIELDS,
            "CREBAIN NEST required runtime file",
        )
        role = row["role"]
        absolute_path = row["absolute_path"]
        size_bytes = row["size_bytes"]
        if (
            not isinstance(role, str)
            or not role
            or len(role.encode("utf-8")) > 256
            or not isinstance(absolute_path, str)
            or not absolute_path.startswith("/")
            or "\\" in absolute_path
            or "\0" in absolute_path
            or len(absolute_path.encode("utf-8")) > 4096
            or ".." in PurePosixPath(absolute_path).parts
            or not valid_sha256(row["sha256"])
            or isinstance(size_bytes, bool)
            or not isinstance(size_bytes, int)
            or not 0 <= size_bytes <= MAX_SOURCE_BYTES
        ):
            raise MatrixError("CREBAIN NEST required runtime-file identity differs")
        runtime_roles.append(role)
        runtime_by_role[role] = row
    project_files = [
        row for row in runtime_files if row["role"].startswith("project-module:")
    ]
    expected_nonproject_roles = [
        "pydantic-core-native",
        "pydantic-package-init",
        "python-executable",
        "worker-source",
    ]
    if (
        runtime_roles
        != [row["role"] for row in project_files] + expected_nonproject_roles
        or [row["role"] for row in project_files]
        != sorted(row["role"] for row in project_files)
        or len(runtime_roles) != len(set(runtime_roles))
        or launch["required_runtime_file_roster_sha256"]
        != digest_bytes(canonical(runtime_files))
        or launch["required_project_source_roster_sha256"]
        != digest_bytes(canonical(project_files))
    ):
        raise MatrixError("CREBAIN NEST runtime-file roster differs")

    source_closure = capture.get("engram_source_closure")
    source_rows = (
        source_closure.get("sources") if isinstance(source_closure, dict) else None
    )
    worker_modules = (
        source_closure.get("worker_project_modules")
        if isinstance(source_closure, dict)
        else None
    )
    worker_identity = evidence.get("worker_runtime_identity")
    identity_files = (
        worker_identity.get("files") if isinstance(worker_identity, dict) else None
    )
    if (
        not isinstance(source_rows, list)
        or not isinstance(worker_modules, list)
        or not isinstance(identity_files, list)
    ):
        raise MatrixError("CREBAIN Engram source roster is absent")
    sources_by_path = {
        row.get("relative_path"): row
        for row in source_rows
        if isinstance(row, dict) and isinstance(row.get("relative_path"), str)
    }
    identity_project_files = [
        row
        for row in identity_files
        if isinstance(row, dict)
        and isinstance(row.get("role"), str)
        and row["role"].startswith("project-module:")
    ]
    if (
        project_files != identity_project_files
        or launch["required_project_source_roster_sha256"]
        != source_closure.get("worker_project_source_roster_sha256")
        or launch["required_project_source_roster_sha256"]
        != worker_identity.get("project_source_roster_sha256")
    ):
        raise MatrixError("CREBAIN NEST project-source roster lineage differs")

    project_roots: set[str] = set()
    expected_project_roles: list[str] = []
    for item in worker_modules:
        module = require_keys(
            item,
            {"module_name", "relative_path"},
            "CREBAIN NEST worker project module",
        )
        module_name = module["module_name"]
        relative_path = module["relative_path"]
        role = f"project-module:{module_name}"
        source = sources_by_path.get(relative_path)
        runtime_file = runtime_by_role.get(role)
        suffix = f"/{relative_path}"
        if (
            not isinstance(module_name, str)
            or not module_name
            or any(not part.isidentifier() for part in module_name.split("."))
            or not isinstance(relative_path, str)
            or not isinstance(source, dict)
            or not isinstance(runtime_file, dict)
            or runtime_file["sha256"] != source.get("sha256")
            or runtime_file["size_bytes"] != source.get("size_bytes")
            or not runtime_file["absolute_path"].endswith(suffix)
        ):
            raise MatrixError("CREBAIN NEST project-source byte join differs")
        project_roots.add(runtime_file["absolute_path"][: -len(suffix)])
        expected_project_roles.append(role)
    if (
        expected_project_roles != [row["role"] for row in project_files]
        or len(project_roots) != 1
    ):
        raise MatrixError("CREBAIN NEST project-source root differs")
    project_root = next(iter(project_roots))

    def join_source_file(
        relative_path: str,
        role: str,
        expected_sha256: Any,
        *,
        embedded: Any | None = None,
        embedded_role: str | None = None,
        require_runtime_role: bool = True,
    ) -> dict[str, Any]:
        source = sources_by_path.get(relative_path)
        runtime_file = runtime_by_role.get(role) if require_runtime_role else None
        suffix = f"/{relative_path}"
        if not isinstance(source, dict) or source.get("sha256") != expected_sha256:
            raise MatrixError(f"CREBAIN NEST {role} source join differs")
        if require_runtime_role and (
            not isinstance(runtime_file, dict)
            or runtime_file.get("sha256") != expected_sha256
            or runtime_file.get("size_bytes") != source.get("size_bytes")
            or not runtime_file.get("absolute_path", "").endswith(suffix)
        ):
            raise MatrixError(f"CREBAIN NEST {role} runtime source differs")
        if embedded is not None:
            reference = runtime_file if require_runtime_role else source
            if (
                not isinstance(reference, dict)
                or embedded.get("role") != embedded_role
                or embedded.get("sha256") != expected_sha256
                or embedded.get("size_bytes") != source.get("size_bytes")
                or not embedded.get("absolute_path", "").endswith(suffix)
                or (
                    require_runtime_role
                    and embedded.get("absolute_path")
                    != runtime_file.get("absolute_path")
                )
            ):
                raise MatrixError(f"CREBAIN NEST {role} embedded source differs")
            return embedded
        if not isinstance(runtime_file, dict):
            raise MatrixError(f"CREBAIN NEST {role} runtime source is absent")
        return runtime_file

    join_source_file(
        "backend/optimization/extension_closed_loop_nest_process.py",
        "project-module:backend.optimization.extension_closed_loop_nest_process",
        launch["adapter_source_sha256"],
    )
    join_source_file(
        "backend/integrations/contained_exec_gate.py",
        "project-module:backend.integrations.contained_exec_gate",
        launch["exec_gate_source_sha256"],
        embedded=require_keys(
            launch["exec_gate_source_file"],
            RUNTIME_FILE_FIELDS,
            "CREBAIN NEST exec-gate source file",
        ),
        embedded_role="exec-gate-source",
    )
    guardian_source_file = join_source_file(
        "backend/optimization/extension_closed_loop_nest_guardian.py",
        "guardian-source",
        launch["guardian_source_sha256"],
        embedded=require_keys(
            launch["guardian_source_file"],
            RUNTIME_FILE_FIELDS,
            "CREBAIN NEST guardian source file",
        ),
        embedded_role="guardian-source",
        require_runtime_role=False,
    )
    join_source_file(
        "backend/optimization/extension_closed_loop_nest_worker.py",
        "worker-source",
        launch["worker_source_sha256"],
    )

    worker_command = require_list(
        launch["worker_command"],
        "CREBAIN NEST worker command",
        minimum=5,
        maximum=64,
    )
    dispatch_command = require_list(
        launch["worker_dispatch_command"],
        "CREBAIN NEST worker dispatch command",
        minimum=len(worker_command),
        maximum=68,
    )
    guardian_command = require_list(
        launch["guardian_command"],
        "CREBAIN NEST guardian command",
        minimum=6,
        maximum=64,
    )
    sys_path = require_list(
        launch["sys_path"],
        "CREBAIN NEST sys.path",
        minimum=1,
        maximum=32,
    )
    command_values = [*worker_command, *dispatch_command, *guardian_command, *sys_path]
    if any(
        not isinstance(value, str)
        or not value
        or len(value.encode("utf-8")) > 256 * 1024
        for value in command_values
    ):
        raise MatrixError("CREBAIN NEST launch command value differs")
    worker_source_file = runtime_by_role["worker-source"]
    python_file = runtime_by_role["python-executable"]
    guardian_source_bytes = guardian_command[5].encode("utf-8")
    sandbox_profile = dispatch_command[2] if len(dispatch_command) >= 3 else None
    expected_worker_command = [
        python_file["absolute_path"],
        "-I",
        "-S",
        "-B",
        worker_source_file["absolute_path"],
        "--resource-limit-profile",
        "portable-posix-rlimit-v1",
        "--address-space-bytes",
        "0",
        "--cpu-time-seconds",
        "300",
        "--file-size-bytes",
        "67108864",
        "--open-file-count",
        "256",
    ]
    expected_worker_command_sha256 = digest_bytes(
        canonical(
            {
                "guardian_command": guardian_command,
                "worker_command": worker_command,
                "worker_dispatch_command": dispatch_command,
                "exec_gate_source_sha256": launch["exec_gate_source_sha256"],
                "session_escape_prevention_profile": launch[
                    "session_escape_prevention_profile"
                ],
                "darwin_sandbox_profile_sha256": launch[
                    "darwin_sandbox_profile_sha256"
                ],
                "darwin_sandbox_launcher_sha256": launch[
                    "darwin_sandbox_launcher_sha256"
                ],
            }
        )
    )
    if (
        launch["schema_version"] != "engram.nest-worker-launch-expectation.v4"
        or launch["controller_configuration"] != capture["nest_config"]
        or launch["environment"]
        != [["LANG", "C"], ["LC_ALL", "C"], ["PATH", "/usr/bin:/bin"], ["TZ", "UTC"]]
        or launch["platform"] != "darwin"
        or launch["resource_limit_profile"] != "portable-posix-rlimit-v1"
        or launch["project_source_discovery_policy"]
        != "minimum-direct-worker-import-roster-v1"
        or launch["session_escape_prevention_profile"]
        != "darwin-gated-group-leader-deny-fork-v1"
        or launch["child_provider_test_failure_phase"] != "none"
        or launch["address_space_bytes"] is not None
        or launch["address_space_limit_enforced"] is not False
        or launch["core_file_bytes"] != 0
        or launch["cpu_time_seconds"] != 300
        or launch["file_size_bytes"] != 67_108_864
        or launch["open_file_count"] != 256
        or launch["descendant_creation_denied"] is not True
        or launch["runtime_process_group_leader"] is not True
        or launch["guardian_group_member"] is not True
        or any(
            launch[field] is not False
            for field in (
                "external_dependency_closure_attested",
                "loaded_bytes_attested",
                "network_namespace_isolation",
                "production_isolation",
                "syscall_filter",
            )
        )
        or not all(
            valid_sha256(launch[field])
            for field in (
                "adapter_source_sha256",
                "darwin_sandbox_launcher_sha256",
                "darwin_sandbox_profile_sha256",
                "exec_gate_source_sha256",
                "expected_child_provider_identity_sha256",
                "guardian_source_sha256",
                "python_executable_sha256",
                "required_project_source_roster_sha256",
                "required_runtime_file_roster_sha256",
                "worker_command_sha256",
                "worker_source_sha256",
            )
        )
        or launch["python_executable_sha256"] != python_file["sha256"]
        or launch["worker_source_sha256"] != worker_source_file["sha256"]
        or worker_command != expected_worker_command
        or guardian_command[:5]
        != [python_file["absolute_path"], "-I", "-S", "-B", "-c"]
        or len(guardian_command) != 6
        or len(guardian_source_bytes) != guardian_source_file["size_bytes"]
        or digest_bytes(guardian_source_bytes) != launch["guardian_source_sha256"]
        or dispatch_command
        != ["/usr/bin/sandbox-exec", "-p", sandbox_profile, *worker_command]
        or not isinstance(sandbox_profile, str)
        or digest_bytes(sandbox_profile.encode("utf-8"))
        != launch["darwin_sandbox_profile_sha256"]
        or launch["worker_command_sha256"] != expected_worker_command_sha256
        or sys_path[0] != project_root
        or len(sys_path) != len(set(sys_path))
        or any(not path.startswith("/") for path in sys_path)
    ):
        raise MatrixError("CREBAIN NEST runtime launch expectation differs")

    worker_pid = launch_attempt["worker_pid"]
    guardian_pid = launch_attempt["guardian_pid"]
    session_id = launch_attempt["session_id"]
    if (
        launch_attempt["schema_version"] != "engram.nest-worker-launch-attempt.v1"
        or launch_attempt["launch_expectation_sha256"] != launch_sha256
        or launch_attempt["outcome"] != "succeeded"
        or launch_attempt["phase"] != "worker-ready"
        or launch_attempt["reason_code"] != "neural.nest-worker-launch-succeeded"
        or launch_attempt["posix_process_group_portability_scope"]
        != "darwin-linux-reviewed-local-development"
        or any(
            isinstance(value, bool) or not isinstance(value, int) or value <= 1
            for value in (worker_pid, guardian_pid, session_id)
        )
        or launch_attempt["process_group_id"] != worker_pid
        or guardian_pid == worker_pid
        or any(
            launch_attempt[field] is not True
            for field in (
                "guardian_ready_observed",
                "guardian_started",
                "stderr_drain_started",
                "worker_started",
            )
        )
        or any(
            launch_attempt[field] is not False
            for field in (
                "anchored_group_kill_delivered",
                "bounded_cleanup_observation_complete",
                "containment_empty",
                "group_signal_attempted",
                "guardian_reaped",
                "production_isolation",
                "scientific_authority",
                "worker_reaped",
            )
        )
        or launch_attempt["containment_seal_signal"] is not None
        or launch_attempt["group_signal_basis"] != "none"
    ):
        raise MatrixError("CREBAIN NEST worker launch attempt differs")

    capabilities_sha256 = digest_bytes(canonical(capabilities))
    expected_populations = expected_population_roster(roster)
    step_duration_tics = terminal.get("timebase", {}).get("neural_step_duration_tics")
    if (
        capabilities["schema_version"] != "engram.closed-loop-neural-capabilities.v1"
        or capabilities["provider"] != "engram.nest-population-controller"
        or capabilities["deadline_enforcement"] != "cooperative-observed"
        or capabilities["durable_evidence_profile"] != "none"
        or capabilities["session_model"] != "one-session-named-populations"
        or capabilities["max_channels"] != 64
        or capabilities["declared_step_duration_tics"] != step_duration_tics
        or capabilities["provider_identity_sha256"]
        != launch["expected_child_provider_identity_sha256"]
        or any(
            capabilities[field] is not False
            for field in (
                "automatic_restart",
                "loaded_bytes_attested",
                "ncp_transport",
                "physical_actuation",
            )
        )
    ):
        raise MatrixError("CREBAIN NEST child capabilities differ")

    child_session_sha256 = session.get("receipt_sha256")
    binding_sha256 = binding.get("receipt_sha256")
    identity_sha256 = identity.get("receipt_sha256")
    if (
        not all(
            valid_sha256(value)
            for value in (child_session_sha256, binding_sha256, identity_sha256)
        )
        or child_session_sha256 != digest_without(session, "receipt_sha256")
        or binding_sha256 != digest_without(binding, "receipt_sha256")
        or identity_sha256 != digest_without(identity, "receipt_sha256")
        or binding.get("runtime_launch_expectation_sha256") != launch_sha256
        or binding.get("worker_launch_attempt_sha256") != launch_attempt_sha256
        or binding.get("child_capabilities_sha256") != capabilities_sha256
        or binding.get("child_prepared_receipt_sha256") != child_prepared_sha256
        or binding.get("child_provider_identity_sha256")
        != capabilities["provider_identity_sha256"]
        or binding.get("child_session_receipt_sha256") != child_session_sha256
        or binding.get("worker_runtime_identity_sha256") != identity_sha256
        or binding.get("worker_command_sha256") != launch["worker_command_sha256"]
        or binding.get("worker_source_sha256") != launch["worker_source_sha256"]
        or binding.get("guardian_source_sha256") != launch["guardian_source_sha256"]
        or binding.get("adapter_source_sha256") != launch["adapter_source_sha256"]
        or binding.get("worker_project_source_roster_sha256")
        != launch["required_project_source_roster_sha256"]
        or binding.get("study_run_id") != terminal.get("study_run_id")
        or binding.get("parent_provider_identity_sha256")
        != terminal.get("neural_provider_identity_sha256")
        or binding.get("child_lineage_verified") is not True
        or any(
            binding.get(field) is not False
            for field in (
                "loaded_bytes_attested",
                "ncp_transport",
                "response_bound_loaded_bytes",
                "scientific_authority",
            )
        )
        or worker_lifecycle.get("runtime_launch_expectation_sha256") != launch_sha256
        or worker_lifecycle.get("session_binding_receipt_sha256") != binding_sha256
        or worker_lifecycle.get("runtime_identity_receipt_sha256") != identity_sha256
        or worker_lifecycle.get("worker_pid") != worker_pid
        or worker_lifecycle.get("process_group_id") != worker_pid
        or worker_lifecycle.get("guardian_pid") != guardian_pid
        or worker_lifecycle.get("session_id") != session_id
    ):
        raise MatrixError("CREBAIN NEST session binding lineage differs")

    common_prepared = {
        "schema_version": "engram.closed-loop-neural-prepared.v1",
        "definition_sha256": terminal.get("closed_loop_definition_sha256"),
        "populations": expected_populations,
        "single_session": True,
        "step_duration_tics": step_duration_tics,
        "study_run_id": terminal.get("study_run_id"),
    }
    if (
        any(child_prepared.get(key) != value for key, value in common_prepared.items())
        or any(
            provider_prepared.get(key) != value
            for key, value in common_prepared.items()
        )
        or child_prepared["provider_identity_sha256"]
        != capabilities["provider_identity_sha256"]
        or child_prepared["provider_session_receipt_sha256"] != child_session_sha256
        or provider_prepared["provider_identity_sha256"]
        != terminal.get("neural_provider_identity_sha256")
        or provider_prepared["provider_session_receipt_sha256"] != binding_sha256
        or evidence.get("neural_preparation_sha256") != provider_prepared_sha256
        or terminal.get("neural_preparation_sha256") != provider_prepared_sha256
    ):
        raise MatrixError("CREBAIN NEST preparation receipt lineage differs")
    if (
        preparation_attempt["schema_version"]
        != "engram.nest-worker-preparation-attempt.v1"
        or preparation_attempt["definition_sha256"]
        != terminal.get("closed_loop_definition_sha256")
        or preparation_attempt["study_run_id"] != terminal.get("study_run_id")
        or preparation_attempt["outcome"] != "succeeded"
        or preparation_attempt["phase"] != "provider-prepare"
        or preparation_attempt["reason_code"] != "neural.prepare-succeeded"
        or preparation_attempt["provider_preparation_receipt_sha256"]
        != provider_prepared_sha256
        or preparation_attempt["runtime_identity_receipt_sha256"] != identity_sha256
        or preparation_attempt["runtime_launch_expectation_sha256"] != launch_sha256
        or preparation_attempt["session_binding_receipt_sha256"] != binding_sha256
        or preparation_attempt["worker_launch_attempt_sha256"] != launch_attempt_sha256
        or preparation_attempt["worker_request_dispatched"] is not True
        or preparation_attempt["worker_response_observed"] is not True
        or preparation_attempt["scientific_authority"] is not False
    ):
        raise MatrixError("CREBAIN NEST preparation attempt differs")

    attempts = require_list(
        evidence["step_attempt_receipts"],
        "CREBAIN NEST step-attempt roster",
        minimum=1,
        maximum=1024,
    )
    provider_steps = evidence.get("step_execution_receipts")
    neural_steps = capture.get("neural_steps")
    terminal_executions = terminal.get("neural_executions")
    if not (
        isinstance(provider_steps, list)
        and isinstance(neural_steps, list)
        and isinstance(terminal_executions, list)
        and len(attempts)
        == len(provider_steps)
        == len(neural_steps)
        == len(terminal_executions)
    ):
        raise MatrixError("CREBAIN NEST step-attempt cardinality differs")
    previous_after = 0
    for index, (attempt, provider_step, neural_step, terminal_execution) in enumerate(
        zip(attempts, provider_steps, neural_steps, terminal_executions, strict=True),
        start=1,
    ):
        row = require_keys(
            attempt,
            STEP_ATTEMPT_FIELDS,
            "CREBAIN NEST step attempt",
        )
        request = neural_step.get("request") if isinstance(neural_step, dict) else None
        if not all(
            isinstance(value, dict)
            for value in (provider_step, request, terminal_execution)
        ):
            raise MatrixError("CREBAIN NEST step-attempt join row differs")
        after = row["observed_after_biological_time_tics"]
        requested = row["requested_run_tics"]
        if (
            row["schema_version"] != "engram.nest-step-attempt.v1"
            or row["attempt_index"] != index
            or row["step_index"] != index
            or row["outcome"] != "succeeded"
            or row["reason_code"] != "neural.step-succeeded"
            or row["observation_scope"] != "child-reported"
            or row["before_biological_time_tics"] != previous_after
            or isinstance(after, bool)
            or not isinstance(after, int)
            or isinstance(requested, bool)
            or not isinstance(requested, int)
            or requested <= 0
            or after != previous_after + requested
            or row["request_sha256"] != request.get("request_sha256")
            or row["execution_receipt_sha256"] != provider_step.get("receipt_sha256")
            or row["execution_receipt_sha256"]
            != terminal_execution.get("provider_execution_sha256")
            or provider_step.get("step_index") != index
            or provider_step.get("before_biological_time_tics") != previous_after
            or provider_step.get("after_biological_time_tics") != after
            or provider_step.get("requested_run_tics") != requested
            or not valid_sha256(row["partial_readback_sha256"])
            or not valid_sha256(row["receipt_sha256"])
            or row["receipt_sha256"] != digest_without(row, "receipt_sha256")
            or any(
                row[field] is not True
                for field in (
                    "decoded_proposal_produced",
                    "simulation_dispatched",
                    "simulation_returned",
                )
            )
            or row["scientific_authority"] is not False
        ):
            raise MatrixError(f"CREBAIN NEST step attempt differs at step {index}")
        previous_after = after

    return {
        "runtime_launch_expectation_sha256": launch_sha256,
        "worker_launch_attempt_sha256": launch_attempt_sha256,
        "preparation_attempt_sha256": preparation_attempt_sha256,
        "child_capabilities_sha256": capabilities_sha256,
        "child_preparation_receipt_sha256": child_prepared_sha256,
        "provider_preparation_receipt_sha256": provider_prepared_sha256,
        "step_attempt_roster_sha256": digest_bytes(canonical(attempts)),
        "step_attempt_count": len(attempts),
    }


def validate_nest_topology(
    capture: dict[str, Any],
    roster: dict[str, Any],
    external_summary: dict[str, Any],
    expected_prisoma: dict[str, Any],
    expected_engram: dict[str, Any],
) -> dict[str, Any]:
    evidence = require_keys(
        capture["nest_evidence_bundle"],
        NEST_BUNDLE_FIELDS,
        "CREBAIN NEST evidence bundle",
    )
    terminal = capture["terminal_receipt"]
    validate_external_summary(
        external_summary,
        terminal,
        evidence,
        expected_prisoma,
        expected_engram,
    )
    session = evidence["nest_session_readback"]
    work = session.get("work_admission") if isinstance(session, dict) else None
    if not isinstance(session, dict) or not isinstance(work, dict):
        raise MatrixError("CREBAIN NEST session or work admission is absent")
    expected_dimensions = roster["action_dimension_count"]
    expected_populations = expected_dimensions * 2
    population_size = capture["nest_config"].get("population_size")
    if (
        evidence["schema_version"] != "engram.nest-closed-loop-evidence-bundle.v2"
        or evidence["profile"] != "killable-nest-population-controller-v2"
        or evidence["study_run_id"] != terminal["study_run_id"]
        or evidence["run_receipt_sha256"] != terminal["receipt_sha256"]
        or evidence["neural_provider_identity_sha256"]
        != terminal["neural_provider_identity_sha256"]
        or session.get("schema_version") != "engram.nest-session-readback.v2"
        or session.get("reported_version") != "3.9.0"
        or session.get("one_session") is not True
        or session.get("ncp_transport") is not False
        or session.get("loaded_bytes_attested") is not False
        or not valid_sha256(session.get("population_roster_sha256"))
        or work.get("schema_version") != "engram.nest-work-admission.v1"
        or work.get("admitted") is not True
        or work.get("action_dimension_count") != expected_dimensions
        or work.get("signed_population_count") != expected_populations
        or not isinstance(population_size, int)
        or work.get("population_neuron_count") != expected_populations * population_size
        or session.get("observed_population_neuron_count")
        != work.get("population_neuron_count")
        or session.get("observed_device_node_count") != work.get("device_node_count")
        or session.get("observed_total_connection_count")
        != work.get("total_connection_count")
        or any(
            evidence.get(field) is not False
            for field in (
                "execution_authority",
                "ncp_control",
                "physical_actuation",
                "scientific_authority",
                "is_paper_local_evidence",
                "calibrated_posterior",
            )
        )
    ):
        raise MatrixError("CREBAIN NEST topology, identity, or authority differs")
    return {
        "profile": evidence["profile"],
        "reported_version": session["reported_version"],
        "one_session": True,
        "population_roster_sha256": session["population_roster_sha256"],
        "neural_provider_identity_sha256": evidence["neural_provider_identity_sha256"],
        "signed_population_count": work["signed_population_count"],
        "population_neuron_count": work["population_neuron_count"],
        "external_validator_receipt_sha256": external_summary["receipt_sha256"],
        "source_durable_evidence_verified": True,
    }


def load_transcript_generator() -> ModuleType:
    specification = importlib.util.spec_from_file_location(
        "prisoma_managed_observer_transcript_generator",
        TRANSCRIPT_GENERATOR,
    )
    if specification is None or specification.loader is None:
        raise MatrixError("managed observer transcript generator cannot be loaded")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def prepare_control(
    source: dict[str, Any],
    channel_ids: list[str],
    subject_ids: list[str],
) -> dict[str, Any]:
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
        "channel_ids": channel_ids,
        "subject_ids": subject_ids,
        "planned_step_count": source["planned_step_count"],
        "max_steps": source["planned_step_count"],
    }


def observer_response_boundary(control: dict[str, Any]) -> None:
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
        raise MatrixError("managed observer response grants authority")


def run_observer(
    binary: Path,
    source: dict[str, Any],
    channel_ids: list[str],
    subject_ids: list[str],
) -> dict[str, Any]:
    generator = load_transcript_generator()
    checked_binary = generator.checked_binary(binary)
    roster = generator.operation_roster()
    roster_digest = digest_bytes(
        b"engram-managed-operation-roster-v1\0" + canonical(roster)
    )
    by_name = {row["operation_id"].split(".")[-2]: row for row in roster}
    controls: list[tuple[str, dict[str, Any]]] = [
        ("prepare", prepare_control(source, channel_ids, subject_ids)),
        *[("observe", generator.observe_control(step)) for step in source["steps"]],
        ("finish", generator.finish_control(source)),
    ]
    host_frames = [generator.host_handshake(roster_digest)]
    for sequence, (name, control) in enumerate(controls, start=1):
        frame = generator.operation_request(
            by_name[name],
            sequence,
            "2",
            control,
        )
        frame["message_id"] = "msg_" + format(sequence + 1, "032x")
        frame["body"]["idempotency_key"] = "idem_" + format(sequence + 1, "064x")
        host_frames.append(frame)
    wire = b"".join(
        struct.pack(">I", len(canonical(frame))) + canonical(frame)
        for frame in host_frames
    )
    completed = subprocess.run(
        [str(checked_binary)],
        input=wire,
        env={
            "LANG": "C",
            "LC_ALL": "C",
            "PATH": os.defpath,
            "TZ": "UTC",
        },
        capture_output=True,
        check=False,
        timeout=15,
        close_fds=True,
    )
    if completed.returncode != 0 or completed.stderr:
        raise MatrixError("managed observer rejected a CREBAIN source receipt")
    if len(completed.stdout) > MAX_OBSERVER_OUTPUT_BYTES:
        raise MatrixError("managed observer matrix output exceeds its bound")
    runtime_frames = generator.decode_frames(completed.stdout)
    if len(runtime_frames) != len(host_frames):
        raise MatrixError("managed observer matrix response count differs")
    handshake = runtime_frames[0]
    if (
        handshake.get("kind") != "runtime.handshake"
        or handshake.get("sender") != "runtime"
        or handshake.get("body", {}).get("ready_claim") is not False
        or handshake.get("body", {}).get("identity", {}).get("operation_roster_sha256")
        != roster_digest
    ):
        raise MatrixError("managed observer matrix handshake differs")
    expected_sources = [
        None,
        *[step["receipt_sha256"] for step in source["steps"]],
        source["receipt_sha256"],
    ]
    operation_controls: list[dict[str, Any]] = []
    for index, (frame, expected_source) in enumerate(
        zip(runtime_frames[1:], expected_sources, strict=True),
        start=1,
    ):
        body = frame.get("body")
        control = body.get("control") if isinstance(body, dict) else None
        if (
            frame.get("kind") != "operation.response"
            or frame.get("sender") != "runtime"
            or frame.get("sequence") != index
            or not isinstance(body, dict)
            or body.get("status") != "succeeded"
            or not isinstance(control, dict)
            or control.get("source_receipt_sha256") != expected_source
            or control.get("study_run_id") != source["study_run_id"]
        ):
            raise MatrixError("managed observer matrix operation response differs")
        observer_response_boundary(control)
        operation_controls.append(control)
    final = operation_controls[-1]
    if (
        final.get("terminal") is not True
        or final.get("state_cleared") is not True
        or final.get("step_index") != len(source["steps"])
        or not valid_sha256(final.get("observer_receipt_sha256"))
        or not valid_sha256(final.get("observer_transcript_sha256"))
    ):
        raise MatrixError("managed observer matrix did not clear terminal state")
    return {
        "direct_process_executed": True,
        "operation_count": len(operation_controls),
        "observed_step_count": len(source["steps"]),
        "final_observer_receipt_sha256": final["observer_receipt_sha256"],
        "final_observer_transcript_sha256": final["observer_transcript_sha256"],
        "terminal_source_receipt_sha256": final["source_receipt_sha256"],
        "state_cleared": True,
        "authority": "read-only-observer",
        "roster_authority": "host-declared-projection",
        "source_roster_authenticated": False,
        "source_durable_evidence_verified": False,
    }


def write_new(path: Path, payload: bytes) -> None:
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
                raise OSError("matrix receipt write made no progress")
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


def require_canonical_directory(path: Path, label: str) -> Path:
    """Require one absolute, existing directory without lexical aliases."""

    lexical = Path(path)
    normalized = Path(os.path.abspath(lexical))
    try:
        resolved = lexical.resolve(strict=True)
        observed = resolved.stat()
    except OSError as error:
        raise MatrixError(f"{label} is unavailable") from error
    if (
        not lexical.is_absolute()
        or lexical != normalized
        or lexical != resolved
        or not stat.S_ISDIR(observed.st_mode)
    ):
        raise MatrixError(f"{label} is not one canonical directory")
    return resolved


def canonical_temporary_parent() -> Path:
    """Resolve the platform temp alias before creating validation inputs."""

    configured = tempfile.gettempdir()
    return require_canonical_directory(
        Path(os.path.realpath(configured)),
        "temporary parent",
    )


def exact_nest_summary(
    engram_root: Path,
    expected_prisoma_revision: str,
    expected_engram_revision: str,
    terminal: dict[str, Any],
    evidence: dict[str, Any],
) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(
        prefix="prisoma-nest-matrix-",
        dir=canonical_temporary_parent(),
    ) as directory:
        root = Path(directory)
        os.chmod(root, 0o700)
        run_path = root / "run-receipt.json"
        bundle_path = root / "nest-evidence.json"
        output_path = root / "summary.json"
        write_new(run_path, canonical(terminal) + b"\n")
        write_new(bundle_path, canonical(evidence) + b"\n")
        completed = subprocess.run(
            [
                sys.executable,
                str(NEST_SUMMARIZER),
                "--expected-prisoma-revision",
                expected_prisoma_revision,
                "--engram-root",
                str(engram_root),
                "--expected-engram-revision",
                expected_engram_revision,
                "--run-receipt",
                str(run_path),
                "--evidence-bundle",
                str(bundle_path),
                "--output",
                str(output_path),
            ],
            cwd=ROOT,
            env={
                "LANG": "C",
                "LC_ALL": "C",
                "PATH": os.defpath,
                "PYTHONHASHSEED": "0",
                "TZ": "UTC",
            },
            stdin=subprocess.DEVNULL,
            capture_output=True,
            check=False,
            timeout=120,
            close_fds=True,
        )
        if (
            completed.returncode != 0
            or completed.stderr
            or len(completed.stdout) > MAX_TOOL_DIAGNOSTIC_BYTES
        ):
            error = completed.stderr.decode("utf-8", errors="replace")[-1024:]
            raise MatrixError(
                f"Engram exact NEST validator rejected a capture: {error}"
            )
        payload = snapshot_regular_file(output_path, 2 * 1024 * 1024)
        return load_json_payload(payload, "external NEST validation summary")


def validate_capture(
    capture: dict[str, Any],
    capture_payload: bytes,
    index_row: dict[str, Any],
    index: dict[str, Any],
    input_suite: dict[str, Any],
    expected_prisoma: dict[str, Any],
    expected_crebain: dict[str, Any],
    expected_engram: dict[str, Any],
    tool_sources: dict[str, Any],
    external_summary: dict[str, Any],
    binary: Path,
    *,
    verify_source_bytes: bool,
    require_six_step_fault_cycle: bool,
) -> dict[str, Any]:
    require_keys(capture, CAPTURE_FIELDS, "CREBAIN real-NEST capture")
    evidence = require_keys(
        capture["nest_evidence_bundle"],
        NEST_BUNDLE_FIELDS,
        "CREBAIN NEST evidence bundle",
    )
    drone_count = index_row["drone_count"]
    suite_run = input_suite["runs"][drone_count]
    if (
        capture["schema_version"] != CAPTURE_SCHEMA_VERSION
        or digest_bytes(capture_payload) != index_row["capture_sha256"]
        or capture["package_generation_id"] != index["package"]["package_generation_id"]
        or capture["plan_exact_sha256"] != index_row["plan_exact_sha256"]
        or capture["plan_exact_sha256"] != suite_run["plan_exact_sha256"]
        or capture["run_plan"] != suite_run["plan"]
        or capture["nest_config_exact_sha256"] != input_suite["config_exact_sha256"]
        or capture["nest_config"] != input_suite["config"]
        or isinstance(capture["receipt_lock_timeout_ms"], bool)
        or not isinstance(capture["receipt_lock_timeout_ms"], int)
        or not 1 <= capture["receipt_lock_timeout_ms"] <= 300_000
        or not isinstance(capture["disclosure"], str)
        or not capture["disclosure"]
        or len(capture["disclosure"].encode("utf-8")) > 512
    ):
        raise MatrixError(f"CREBAIN capture identity differs for {drone_count} drones")
    required_boolean_fields(
        capture["assertions"],
        CAPTURE_ASSERTION_FIELDS,
        "CREBAIN capture assertions",
    )
    check_false_authority(
        capture["authority"],
        CAPTURE_AUTHORITY_FIELDS,
        "CREBAIN capture authority",
    )
    run_plan = capture["run_plan"]
    roster = channel_roster(run_plan, drone_count)
    package = validate_installed_package_proof(
        capture,
        index,
        expected_crebain,
        expected_engram,
        tool_sources,
        verify_source_bytes=verify_source_bytes,
    )
    if (
        index_row["observed_build_receipt_exact_sha256"]
        != package["observed_build_receipt_exact_sha256"]
    ):
        raise MatrixError("CREBAIN capture and observed-build receipt differ")
    terminal = require_keys(
        capture["terminal_receipt"],
        TERMINAL_FIELDS,
        "CREBAIN terminal run receipt",
    )
    summary = require_keys(capture["summary"], SUMMARY_FIELDS, "CREBAIN run summary")
    validate_recorded_summary_authority(summary)
    completed_steps = len(terminal["steps"])
    if (
        run_plan["schema_version"] != "engram.extension-closed-loop-run-plan.v1"
        or run_plan["study_run_id"] != terminal["study_run_id"]
        or run_plan["study_definition_sha256"] != terminal["study_definition_sha256"]
        or run_plan["step_count"] != terminal["planned_step_count"]
        or summary["status"] != "recorded"
        or summary["study_run_id"] != terminal["study_run_id"]
        or summary["run_status"] != "completed"
        or summary["terminal_reason_code"] != "loop.completed"
        or summary["planned_step_count"] != terminal["planned_step_count"]
        or summary["completed_step_count"] != completed_steps
        or summary["channel_count"] != drone_count
        or summary["receipt_sha256"] != terminal["receipt_sha256"]
        or summary["receipt_sha256"] != index_row["receipt_sha256"]
        or summary["evidence_bundle_sha256"] != index_row["evidence_bundle_sha256"]
        or evidence["bundle_sha256"] != index_row["evidence_bundle_sha256"]
        or terminal["status"] != "completed"
        or terminal["terminal_reason_code"] != "loop.completed"
        or terminal["runtime_progress_disposition"] != "finished-and-host-verified"
        or run_plan["simulator_only"] is not True
        or terminal["simulator_only"] is not True
        or any(
            value is not False
            for value in (
                run_plan["agent_action_authority"],
                run_plan["physical_actuation"],
                run_plan["ncp_transport_used"],
                run_plan["music_transport_used"],
                run_plan["scientific_authority"],
                run_plan["is_paper_local_evidence"],
                run_plan["calibrated_posterior"],
                terminal["physical_actuation"],
                terminal["ncp_qualified"],
                terminal["scientific_authority"],
                terminal["is_paper_local_evidence"],
                terminal["calibrated_posterior"],
                summary["physical_actuation"],
                summary["ncp_qualified"],
                summary["scientific_authority"],
                summary["calibrated_posterior"],
            )
        )
    ):
        raise MatrixError(
            f"CREBAIN run, roster, or authority differs for {drone_count} drones"
        )
    if require_six_step_fault_cycle and (
        run_plan["step_count"] != 6 or completed_steps != 6
    ):
        raise MatrixError("CREBAIN operational matrix is not the exact six-step proof")
    source = validate_source_closure(
        capture,
        expected_engram,
        package["engram_pack_receipt"],
        verify_source_bytes=verify_source_bytes,
    )
    if (
        source["engram_source_closure_sha256"]
        != index_row["engram_source_closure_sha256"]
        or source["engram_source_roster_sha256"]
        != index_row["engram_source_roster_sha256"]
    ):
        raise MatrixError("CREBAIN indexed Engram source closure or roster differs")
    v2_execution_lineage = validate_nest_v2_execution_lineage(capture, roster)
    receipt_store = validate_receipt_store_closure(capture, index_row)
    topology = validate_population_topology(capture, index_row, roster)
    worker_guardian = validate_worker_guardian_closure(capture)
    lifecycle = validate_lifecycle(capture)
    validate_fault_and_neural_lineage(
        capture,
        roster,
        require_six_step_fault_cycle=require_six_step_fault_cycle,
    )
    nest = validate_nest_topology(
        capture,
        roster,
        external_summary,
        expected_prisoma,
        expected_engram,
    )
    nest.update(topology)
    nest.update(worker_guardian)
    nest.update(v2_execution_lineage)
    observer = run_observer(
        binary,
        terminal,
        roster["channel_ids"],
        roster["subject_ids"],
    )
    return {
        "drone_count": drone_count,
        "capture": {
            "path": index_row["path"],
            "exact_sha256": index_row["capture_sha256"],
            "schema_version": capture["schema_version"],
            "plan_exact_sha256": capture["plan_exact_sha256"],
            "package_generation_id": package["package_generation_id"],
            "installed_package_proof_exact_sha256": package[
                "installed_package_proof_exact_sha256"
            ],
            "installed_package_proof_receipt_sha256": package[
                "installed_package_proof_receipt_sha256"
            ],
            "observed_build_receipt_exact_sha256": package[
                "observed_build_receipt_exact_sha256"
            ],
            "observed_build_receipt_sha256": package["observed_build_receipt_sha256"],
            "package_stage_receipt_exact_sha256": package[
                "package_stage_receipt_exact_sha256"
            ],
            "package_stage_receipt_sha256": package["package_stage_receipt_sha256"],
            "engram_pack_receipt_exact_sha256": package[
                "engram_pack_receipt_exact_sha256"
            ],
            "engram_pack_receipt_sha256": package["engram_pack_receipt_sha256"],
            "engram_extension_tool_sha256": package["engram_extension_tool_sha256"],
            "engram_extension_tool_git_blob": package["engram_extension_tool_git_blob"],
            "build_source_roster_sha256": package["build_source_roster_sha256"],
            "build_input_identity_sha256": package["build_input_identity_sha256"],
            "package_inventory_sha256": package["package_inventory_sha256"],
            "executable_sha256": package["executable_sha256"],
            "terminal_receipt_sha256": terminal["receipt_sha256"],
            "nest_evidence_bundle_sha256": capture["nest_evidence_bundle"][
                "bundle_sha256"
            ],
            "receipt_store_id": receipt_store["receipt_store_id"],
            "reservation_id": receipt_store["reservation_id"],
            "receipt_store_closure_sha256": receipt_store[
                "receipt_store_closure_sha256"
            ],
            "receipt_store_file_count": receipt_store["receipt_store_file_count"],
            "receipt_store_files": receipt_store["receipt_store_files"],
            "receipt_store_file_roster_sha256": receipt_store[
                "receipt_store_file_roster_sha256"
            ],
            "receipt_store_sidecars": receipt_store["receipt_store_sidecars"],
            "receipt_store_sidecars_sha256": receipt_store[
                "receipt_store_sidecars_sha256"
            ],
            "terminal_artifact_path": receipt_store["terminal_artifact_path"],
            "terminal_artifact_size_bytes": receipt_store[
                "terminal_artifact_size_bytes"
            ],
            "terminal_artifact_exact_sha256": receipt_store[
                "terminal_artifact_exact_sha256"
            ],
            "evidence_artifact_path": receipt_store["evidence_artifact_path"],
            "evidence_artifact_size_bytes": receipt_store[
                "evidence_artifact_size_bytes"
            ],
            "evidence_artifact_exact_sha256": receipt_store[
                "evidence_artifact_exact_sha256"
            ],
        },
        "source": source,
        "scenario": {
            "study_run_id": terminal["study_run_id"],
            "channel_ids": roster["channel_ids"],
            "subject_ids": roster["subject_ids"],
            "subject_kind": "simulated.drone",
            "neural_population_prefixes": roster["population_prefixes"],
            "planned_step_count": terminal["planned_step_count"],
            "completed_step_count": completed_steps,
            "faulted_channel_ordinal": 1,
        },
        "nest": nest,
        "observer": observer,
        "lifecycle": lifecycle,
        "authority": {
            "simulator_only": True,
            "descriptive_only": True,
            "agent_bridge_command": False,
            "execution_authority": False,
            "ncp_used": False,
            "music_used": False,
            "ncp_qualified": False,
            "physical_actuation": False,
            "plant_control": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
    }


def load_contract_checker() -> ModuleType:
    specification = importlib.util.spec_from_file_location(
        "prisoma_managed_observer_contract_checker",
        CONTRACT_CHECKER,
    )
    if specification is None or specification.loader is None:
        raise MatrixError("managed observer contract checker cannot be loaded")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def checked_runtime_binary(path: Path) -> dict[str, Any]:
    expected = RELEASE_BINARY.absolute()
    candidate = path.absolute()
    if candidate != expected:
        raise MatrixError(f"observer binary must be the release artifact at {expected}")
    lexical = os.lstat(candidate)
    if (
        not stat.S_ISREG(lexical.st_mode)
        or lexical.st_nlink != 1
        or lexical.st_uid != os.geteuid()
        or candidate.resolve(strict=True) != candidate
    ):
        raise MatrixError("observer release binary path traverses a link")
    generator = load_transcript_generator()
    checked = generator.checked_binary(candidate)
    observed = os.lstat(checked)
    if (
        stat.S_ISLNK(observed.st_mode)
        or not stat.S_ISREG(observed.st_mode)
        or not os.access(checked, os.X_OK)
    ):
        raise MatrixError("observer release binary is not one executable regular file")
    binary_payload = snapshot_regular_file(checked, MAX_BINARY_BYTES)
    manifest_payload = snapshot_regular_file(CRATE_MANIFEST, MAX_SCHEMA_BYTES)
    lock_payload = snapshot_regular_file(CRATE_LOCK, MAX_SCHEMA_BYTES)
    return {
        "path": checked,
        "payload": binary_payload,
        "exact_sha256": digest_bytes(binary_payload),
        "cargo_manifest_exact_sha256": digest_bytes(manifest_payload),
        "cargo_lock_exact_sha256": digest_bytes(lock_payload),
    }


def checked_release_binary(
    path: Path,
    build_receipt_path: Path,
    expected_revision: str,
) -> dict[str, Any]:
    runtime = checked_runtime_binary(path)
    expected_receipt = RELEASE_BINARY.with_name(
        f"{RELEASE_BINARY.name}.observed-build.json"
    ).absolute()
    candidate_receipt = build_receipt_path.absolute()
    if candidate_receipt != expected_receipt:
        raise MatrixError(
            f"observer build receipt must be the release receipt at {expected_receipt}"
        )
    schema_payload = snapshot_regular_file(BUILD_RECEIPT_SCHEMA, MAX_SCHEMA_BYTES)
    schema = load_json_payload(schema_payload, "observer build receipt schema")
    checker = load_contract_checker()
    try:
        checker.validate_safe_project_schema(
            observed_build.SCHEMA_VERSION,
            schema,
        )
        verification = observed_build.verify_observed_build_receipt(
            candidate_receipt,
            runtime["path"],
            expected_revision,
        )
    except (SystemExit, observed_build.BuildObservationError) as error:
        raise MatrixError(
            f"observer observed-build receipt differs: {error}"
        ) from error
    document = verification["document"]
    if not checker.schema_accepts(document, schema):
        raise MatrixError("observer observed-build receipt fails its closed schema")
    artifact = verification["artifact"]
    if (
        artifact["sha256"] != runtime["exact_sha256"]
        or runtime["cargo_manifest_exact_sha256"]
        != next(
            row["sha256"]
            for row in verification["source"]["source_roster"]
            if row["path"] == "crates/engram-managed-observer/Cargo.toml"
        )
        or runtime["cargo_lock_exact_sha256"]
        != next(
            row["sha256"]
            for row in verification["source"]["source_roster"]
            if row["path"] == "crates/engram-managed-observer/Cargo.lock"
        )
    ):
        raise MatrixError("observer build receipt source or artifact join differs")
    runtime.update(
        {
            "build_receipt_path": candidate_receipt,
            "build_receipt_payload": verification["payload"],
            "build_receipt_exact_sha256": verification["exact_sha256"],
            "build_receipt_sha256": document["receipt_sha256"],
            "source_roster_sha256": verification["source"]["source_roster_sha256"],
            "cargo_tool_sha256": verification["toolchain"]["cargo"]["sha256"],
            "rustc_tool_sha256": verification["toolchain"]["rustc"]["sha256"],
            "rustc_host": verification["toolchain"]["rustc_host"],
            "rustc_release": verification["toolchain"]["rustc_release"],
            "macho": artifact["macho"],
        }
    )
    return runtime


def observer_binary_projection(binary_identity: dict[str, Any]) -> dict[str, Any]:
    return {
        "path": str(binary_identity["path"].relative_to(ROOT)),
        "exact_sha256": binary_identity["exact_sha256"],
        "cargo_manifest_exact_sha256": binary_identity["cargo_manifest_exact_sha256"],
        "cargo_lock_exact_sha256": binary_identity["cargo_lock_exact_sha256"],
        "build_receipt_path": str(
            binary_identity["build_receipt_path"].relative_to(ROOT)
        ),
        "build_receipt_exact_sha256": binary_identity["build_receipt_exact_sha256"],
        "build_receipt_sha256": binary_identity["build_receipt_sha256"],
        "source_roster_sha256": binary_identity["source_roster_sha256"],
        "cargo_tool_sha256": binary_identity["cargo_tool_sha256"],
        "rustc_tool_sha256": binary_identity["rustc_tool_sha256"],
        "rustc_host": binary_identity["rustc_host"],
        "rustc_release": binary_identity["rustc_release"],
        "macho": binary_identity["macho"],
        "release_profile": True,
        "direct_process_executed": True,
    }


def validate_matrix_document(document: dict[str, Any]) -> None:
    schema_payload = snapshot_regular_file(MATRIX_SCHEMA, MAX_SCHEMA_BYTES)
    schema = load_json_payload(schema_payload, "CREBAIN observer matrix schema")
    checker = load_contract_checker()
    try:
        checker.validate_safe_project_schema(SCHEMA_VERSION, schema)
    except SystemExit as error:
        raise MatrixError(
            f"CREBAIN observer matrix schema is unsafe: {error}"
        ) from error
    if not checker.schema_accepts(document, schema):
        raise MatrixError("CREBAIN observer matrix fails its closed schema")
    stack: list[tuple[str | None, Any]] = [(None, document)]
    while stack:
        key, value = stack.pop()
        if key is not None and key.endswith("sha256") and not valid_sha256(value):
            raise MatrixError(f"matrix digest is not lowercase SHA-256: {key}")
        if (
            key is not None
            and key
            in {
                "commit",
                "tree",
                "origin_main",
                "origin_main_at_capture",
                "parent_commit",
            }
            and not valid_git_oid(value)
        ):
            raise MatrixError(f"matrix Git identity differs: {key}")
        if isinstance(value, dict):
            stack.extend(value.items())
        elif isinstance(value, list):
            stack.extend((None, item) for item in value)
    if document.get("receipt_sha256") != digest_without(document, "receipt_sha256"):
        raise MatrixError("CREBAIN observer matrix self-digest differs")
    sources = document["sources"]
    for field in (
        "prisoma_repository",
        "engram_repository",
    ):
        repository = require_keys(
            sources[field],
            {"repository", "commit", "tree", "origin_main", "object_format", "clean"},
            f"matrix {field}",
        )
        if (
            not isinstance(repository["repository"], str)
            or not repository["repository"]
            or "\n" in repository["repository"]
            or repository["object_format"] not in {"sha1", "sha256"}
            or not valid_git_oid(repository["commit"], repository["object_format"])
            or not valid_git_oid(repository["tree"], repository["object_format"])
            or repository["origin_main"] != repository["commit"]
            or repository["clean"] is not True
        ):
            raise MatrixError(f"matrix {field} identity differs")
    crebain_source = require_keys(
        sources["crebain_source_repository"],
        {
            "repository",
            "commit",
            "tree",
            "origin_main_at_capture",
            "object_format",
            "clean_at_capture",
        },
        "matrix CREBAIN source repository",
    )
    if (
        not isinstance(crebain_source["repository"], str)
        or not crebain_source["repository"]
        or "\n" in crebain_source["repository"]
        or crebain_source["object_format"] not in {"sha1", "sha256"}
        or not valid_git_oid(crebain_source["commit"], crebain_source["object_format"])
        or not valid_git_oid(crebain_source["tree"], crebain_source["object_format"])
        or crebain_source["origin_main_at_capture"] != crebain_source["commit"]
        or crebain_source["clean_at_capture"] is not True
    ):
        raise MatrixError("matrix CREBAIN source repository identity differs")
    publication = require_keys(
        sources["crebain_evidence_publication"],
        {
            "repository",
            "commit",
            "tree",
            "origin_main",
            "object_format",
            "clean",
            "parent_commit",
            "policy",
            "evidence_directory",
            "files",
            "file_count",
            "roster_sha256",
        },
        "matrix CREBAIN evidence publication",
    )
    publication_files = require_list(
        publication["files"],
        "matrix CREBAIN evidence publication files",
        minimum=len(EVIDENCE_PUBLICATION_PATHS),
        maximum=len(EVIDENCE_PUBLICATION_PATHS),
    )
    expected_publication_paths = [
        path.as_posix() for path in EVIDENCE_PUBLICATION_PATHS
    ]
    for row in publication_files:
        require_keys(
            row,
            {"path", "size_bytes", "sha256", "git_mode", "git_blob"},
            "matrix CREBAIN evidence Git row",
        )
    if (
        publication["repository"] != crebain_source["repository"]
        or publication["object_format"] != crebain_source["object_format"]
        or publication["origin_main"] != publication["commit"]
        or publication["parent_commit"] != crebain_source["commit"]
        or publication["commit"] == crebain_source["commit"]
        or not valid_git_oid(publication["commit"], publication["object_format"])
        or not valid_git_oid(publication["tree"], publication["object_format"])
        or publication["clean"] is not True
        or publication["policy"] != EVIDENCE_PUBLICATION_POLICY
        or publication["evidence_directory"] != EVIDENCE_DIRECTORY_RELATIVE.as_posix()
        or publication["file_count"] != len(EVIDENCE_PUBLICATION_PATHS)
        or [row["path"] for row in publication_files] != expected_publication_paths
        or any(
            isinstance(row["size_bytes"], bool)
            or not isinstance(row["size_bytes"], int)
            or not 1 <= row["size_bytes"] <= MAX_CAPTURE_BYTES
            or not valid_sha256(row["sha256"])
            or row["git_mode"] != "100644"
            or not valid_git_oid(row["git_blob"], publication["object_format"])
            for row in publication_files
        )
        or publication["roster_sha256"]
        != digest_bytes(
            EVIDENCE_PUBLICATION_ROSTER_DOMAIN + canonical(publication_files)
        )
    ):
        raise MatrixError("matrix CREBAIN evidence publication identity differs")
    publication_by_path = {row["path"]: row for row in publication_files}
    if publication_by_path[INDEX_RELATIVE.as_posix()]["sha256"] != sources[
        "index_exact_sha256"
    ] or any(
        publication_by_path[
            (EVIDENCE_DIRECTORY_RELATIVE / row["capture"]["path"]).as_posix()
        ]["sha256"]
        != row["capture"]["exact_sha256"]
        for row in document["captures"]
    ):
        raise MatrixError("matrix evidence rows do not join INDEX and capture bytes")
    if not valid_git_oid(
        sources["engram_extension_tool_git_blob"],
        sources["engram_repository"]["object_format"],
    ):
        raise MatrixError("matrix Engram extension-tool Git identity differs")
    captures = document.get("captures")
    if not isinstance(captures, list) or len(captures) != 3:
        raise MatrixError("CREBAIN observer matrix capture count differs")
    receipt_store_ids: set[str] = set()
    receipt_store_closures: set[str] = set()
    runtime_source_closures: set[str] = set()
    stable_source_rosters: set[str] = set()
    stable_source_file_counts: set[int] = set()
    capture_digests: set[str] = set()
    terminal_digests: set[str] = set()
    evidence_digests: set[str] = set()
    for drone_count, row in enumerate(captures, start=1):
        scenario = row["scenario"]
        observer = row["observer"]
        capture = row["capture"]
        nest = row["nest"]
        lifecycle = row["lifecycle"]
        store_files = capture.get("receipt_store_files")
        if not isinstance(store_files, list):
            raise MatrixError("matrix receipt-store file roster is absent")
        store_paths: list[str] = []
        store_rows: dict[str, dict[str, Any]] = {}
        for item in store_files:
            store_row = require_keys(
                item,
                {"relative_path", "size_bytes", "sha256"},
                "matrix receipt-store file",
            )
            store_path = safe_relative(
                store_row["relative_path"],
                "matrix receipt-store path",
            ).as_posix()
            store_paths.append(store_path)
            store_rows[store_path] = store_row
        reservation_id = capture.get("reservation_id")
        reservation_digest = (
            reservation_id.removeprefix("clrr_")
            if isinstance(reservation_id, str)
            else ""
        )
        study_run_key_sha256 = digest_bytes(
            managed_runtime_canonical(
                {
                    "domain": (
                        "engram-extension-closed-loop-publication-study-run-key-v1"
                    ),
                    "store_id": capture["receipt_store_id"],
                    "study_run_id": scenario["study_run_id"],
                }
            )
        )
        sidecar_review = validate_receipt_store_sidecars(
            capture.get("receipt_store_sidecars"),
            store_id=capture["receipt_store_id"],
            terminal_receipt_sha256=capture["terminal_receipt_sha256"],
            terminal_artifact_size_bytes=capture["terminal_artifact_size_bytes"],
            evidence_bundle_sha256=capture["nest_evidence_bundle_sha256"],
            evidence_artifact_size_bytes=capture["evidence_artifact_size_bytes"],
            study_run_id=scenario["study_run_id"],
            run_status="completed",
            terminal_reason_code="loop.completed",
            package_generation_id=capture["package_generation_id"],
        )
        expected_store_paths = sorted(
            (
                capture["evidence_artifact_path"],
                (
                    "finalized-reservations/"
                    f"{reservation_digest[:2]}/{reservation_id}.json"
                ),
                (
                    "observations/"
                    f"{capture['terminal_receipt_sha256'][:2]}/"
                    f"{capture['terminal_receipt_sha256']}.json"
                ),
                f"publication-admission-anchors/{study_run_key_sha256}.json",
                (
                    "publication-authorities/"
                    f"{capture['terminal_receipt_sha256'][:2]}/"
                    f"{capture['terminal_receipt_sha256']}.json"
                ),
                capture["terminal_artifact_path"],
                "store.json",
                "writer.lock",
            )
        )
        store_metadata_payload = managed_runtime_canonical(
            {
                "schema_version": RECEIPT_STORE_SCHEMA,
                "policy": RECEIPT_STORE_POLICY,
                "store_id": capture["receipt_store_id"],
                "digest_canonicalization": RECEIPT_STORE_CANONICALIZATION,
                "execution_authority": False,
                "ncp_control": False,
                "physical_actuation": False,
                "scientific_authority": False,
                "is_paper_local_evidence": False,
                "calibrated_posterior": False,
            }
        )
        total_store_bytes = sum(item["size_bytes"] for item in store_files)
        reconstructed_store_closure = {
            "schema_version": "crebain.closed-loop-receipt-store-closure.v1",
            "store_id": capture["receipt_store_id"],
            "receipt_sha256": capture["terminal_receipt_sha256"],
            "receipt_artifact_path": capture["terminal_artifact_path"],
            "evidence_bundle_sha256": capture["nest_evidence_bundle_sha256"],
            "evidence_artifact_path": capture["evidence_artifact_path"],
            "file_count": len(store_files),
            "total_bytes": total_store_bytes,
            "files": store_files,
        }
        if (
            not valid_prefixed_sha256(reservation_id, "clrr_")
            or len(store_files) != RECEIPT_STORE_FILE_COUNT
            or store_paths != sorted(set(store_paths))
            or store_paths != expected_store_paths
            or len({item["sha256"] for item in store_files}) != RECEIPT_STORE_FILE_COUNT
            or total_store_bytes > MAX_RECEIPT_STORE_BYTES
            or capture.get("receipt_store_file_roster_sha256")
            != digest_bytes(canonical(store_files))
            or capture.get("receipt_store_sidecars_sha256")
            != sidecar_review["sidecars_sha256"]
            or capture.get("reservation_id") != sidecar_review["reservation_id"]
            or store_files != sidecar_review["expected_rows"]
            or lifecycle["runtime_handshake_receipt_sha256"]
            != capture["receipt_store_sidecars"]["finalized_reservation"][
                "reservation"
            ]["reviewed_native_handshake_receipt_sha256"]
            or capture["receipt_store_closure_sha256"]
            != digest_bytes(canonical(reconstructed_store_closure))
            or store_rows.get(capture["terminal_artifact_path"])
            != {
                "relative_path": capture["terminal_artifact_path"],
                "size_bytes": capture["terminal_artifact_size_bytes"],
                "sha256": capture["terminal_artifact_exact_sha256"],
            }
            or store_rows.get(capture["evidence_artifact_path"])
            != {
                "relative_path": capture["evidence_artifact_path"],
                "size_bytes": capture["evidence_artifact_size_bytes"],
                "sha256": capture["evidence_artifact_exact_sha256"],
            }
            or store_rows.get("store.json")
            != {
                "relative_path": "store.json",
                "size_bytes": len(store_metadata_payload),
                "sha256": digest_bytes(store_metadata_payload),
            }
            or store_rows.get("writer.lock")
            != {
                "relative_path": "writer.lock",
                "size_bytes": len(RECEIPT_STORE_LOCK_BYTES),
                "sha256": digest_bytes(RECEIPT_STORE_LOCK_BYTES),
            }
        ):
            raise MatrixError("matrix receipt-store V5 roster differs")
        if (
            row["drone_count"] != drone_count
            or any(
                len(scenario[field]) != drone_count
                for field in (
                    "channel_ids",
                    "subject_ids",
                    "neural_population_prefixes",
                )
            )
            or observer["operation_count"] != observer["observed_step_count"] + 2
            or observer["terminal_source_receipt_sha256"]
            != capture["terminal_receipt_sha256"]
            or capture["installed_package_proof_exact_sha256"]
            != document["sources"]["installed_package_proof_exact_sha256"]
            or capture["installed_package_proof_receipt_sha256"]
            != document["sources"]["installed_package_proof_receipt_sha256"]
            or not valid_prefixed_sha256(capture["receipt_store_id"], "clrs_")
            or capture["receipt_store_file_count"] != RECEIPT_STORE_FILE_COUNT
            or capture["terminal_artifact_path"]
            != (
                "receipts/"
                f"{capture['terminal_receipt_sha256'][:2]}/"
                f"{capture['terminal_receipt_sha256']}.json"
            )
            or capture["terminal_artifact_exact_sha256"]
            != capture["terminal_receipt_sha256"]
            or capture["evidence_artifact_path"]
            != (
                "evidence/"
                f"{capture['nest_evidence_bundle_sha256'][:2]}/"
                f"{capture['nest_evidence_bundle_sha256']}.json"
            )
            or capture["evidence_artifact_exact_sha256"]
            != capture["nest_evidence_bundle_sha256"]
            or any(
                capture[field] != sources[field]
                for field in (
                    "observed_build_receipt_exact_sha256",
                    "observed_build_receipt_sha256",
                    "package_stage_receipt_exact_sha256",
                    "package_stage_receipt_sha256",
                    "engram_pack_receipt_exact_sha256",
                    "engram_pack_receipt_sha256",
                    "engram_extension_tool_sha256",
                    "engram_extension_tool_git_blob",
                    "build_source_roster_sha256",
                    "build_input_identity_sha256",
                    "package_inventory_sha256",
                    "executable_sha256",
                )
            )
            or nest["signed_population_count"] != drone_count * 6
            or nest["population_count"] != nest["signed_population_count"]
            or nest["population_neuron_count"] < nest["signed_population_count"]
            or nest["device_node_count"] != drone_count * 12
            or row["source"]["engram_revision"]
            != sources["engram_repository"]["commit"]
            or row["source"]["engram_tree"] != sources["engram_repository"]["tree"]
        ):
            raise MatrixError("CREBAIN observer matrix semantic join differs")
        receipt_store_ids.add(capture["receipt_store_id"])
        receipt_store_closures.add(capture["receipt_store_closure_sha256"])
        runtime_source_closures.add(row["source"]["engram_source_closure_sha256"])
        stable_source_rosters.add(row["source"]["engram_source_roster_sha256"])
        stable_source_file_counts.add(row["source"]["engram_source_file_count"])
        capture_digests.add(capture["exact_sha256"])
        terminal_digests.add(capture["terminal_receipt_sha256"])
        evidence_digests.add(capture["nest_evidence_bundle_sha256"])
    if len(receipt_store_ids) != 3 or len(receipt_store_closures) != 3:
        raise MatrixError("CREBAIN observer matrix receipt stores are not distinct")
    if len(runtime_source_closures) != 3:
        raise MatrixError("CREBAIN runtime source closures are not distinct")
    if not all(
        len(digests) == 3
        for digests in (capture_digests, terminal_digests, evidence_digests)
    ):
        raise MatrixError("CREBAIN run-specific evidence identities are not distinct")
    if stable_source_rosters != {sources["shared_engram_source_roster_sha256"]}:
        raise MatrixError("CREBAIN stable Engram source roster differs")
    if stable_source_file_counts != {sources["shared_engram_source_file_count"]}:
        raise MatrixError("CREBAIN stable Engram source count differs")


def verify_matrix_prefixes(captures: list[dict[str, Any]]) -> None:
    for smaller, larger in zip(captures[:-1], captures[1:], strict=True):
        for field in (
            "channel_ids",
            "subject_ids",
            "neural_population_prefixes",
        ):
            prior = smaller["scenario"][field]
            current = larger["scenario"][field]
            if current[: len(prior)] != prior:
                raise MatrixError(
                    f"CREBAIN {field} matrix does not preserve its prefix"
                )


def build_matrix(
    *,
    binary: Path,
    binary_build_receipt: Path,
    crebain_root: Path,
    engram_root: Path,
    expected_prisoma_revision: str,
    expected_crebain_source_revision: str,
    expected_crebain_publication_revision: str,
    expected_engram_revision: str,
) -> tuple[dict[str, Any], bytes]:
    binary_identity = checked_release_binary(
        binary,
        binary_build_receipt,
        expected_prisoma_revision,
    )
    prisoma = verify_repository(ROOT, expected_prisoma_revision, "Prisoma")
    try:
        publication_review = capture_evidence_publication(
            crebain_root,
            expected_crebain_source_revision,
            expected_crebain_publication_revision,
            EVIDENCE_PUBLICATION_PATHS,
            MAX_CAPTURE_BYTES,
            max_total_bytes=3 * MAX_CAPTURE_BYTES + MAX_INDEX_BYTES,
        )
    except (OSError, ValueError) as error:
        raise MatrixError(
            "CREBAIN evidence is not one exact clean two-revision publication"
        ) from error
    resolved_crebain_root = crebain_root.resolve(strict=True)
    crebain_publication = {
        "root": resolved_crebain_root,
        **publication_review["publication"],
    }
    crebain_source = {
        "root": resolved_crebain_root,
        **publication_review["source"],
        "origin_main": expected_crebain_source_revision,
        "clean": True,
        "checkout_revision": expected_crebain_publication_revision,
    }
    engram = verify_repository(engram_root, expected_engram_revision, "Engram")
    index_path = repository_path(
        crebain_source["root"],
        INDEX_RELATIVE,
        "CREBAIN index",
    )
    index_payload = snapshot_regular_file(index_path, MAX_INDEX_BYTES)
    index = load_json_payload(index_payload, "CREBAIN evidence index")
    publication_files = {row["path"]: row for row in crebain_publication["files"]}
    index_publication = publication_files.get(INDEX_RELATIVE.as_posix())
    if (
        index_publication is None
        or index_publication["size_bytes"] != len(index_payload)
        or index_publication["sha256"] != digest_bytes(index_payload)
    ):
        raise MatrixError("CREBAIN INDEX bytes differ from the publication commit")
    index_review = validate_index(index, crebain_source, engram)
    rows = index_review["rows"]
    input_suite = index_review["input_suite"]
    tool_sources = index_review["tool_sources"]
    captures: list[dict[str, Any]] = []
    capture_snapshots: list[tuple[Path, bytes]] = []
    pack_tool_records: list[dict[str, Any]] = []
    for position, row in enumerate(rows):
        relative = safe_relative(row["path"], "CREBAIN capture path", suffix=".json")
        capture_path = repository_path(index_path.parent, relative, "CREBAIN capture")
        capture_payload = snapshot_regular_file(capture_path, MAX_CAPTURE_BYTES)
        capture = load_json_payload(capture_payload, f"CREBAIN capture {relative}")
        require_keys(capture, CAPTURE_FIELDS, "CREBAIN real-NEST capture")
        if capture["schema_version"] != CAPTURE_SCHEMA_VERSION:
            raise MatrixError("CREBAIN real-NEST capture version differs")
        external_summary = exact_nest_summary(
            engram_root=engram["root"],
            expected_prisoma_revision=expected_prisoma_revision,
            expected_engram_revision=expected_engram_revision,
            terminal=capture["terminal_receipt"],
            evidence=capture["nest_evidence_bundle"],
        )
        captures.append(
            validate_capture(
                capture,
                capture_payload,
                row,
                index,
                input_suite,
                prisoma,
                crebain_source,
                engram,
                tool_sources,
                external_summary,
                binary_identity["path"],
                verify_source_bytes=position == 0,
                require_six_step_fault_cycle=True,
            )
        )
        published_capture_path = (
            EVIDENCE_DIRECTORY_RELATIVE / relative.as_posix()
        ).as_posix()
        published_capture = publication_files.get(published_capture_path)
        if (
            published_capture is None
            or published_capture["size_bytes"] != len(capture_payload)
            or published_capture["sha256"] != digest_bytes(capture_payload)
            or published_capture["sha256"] != row["capture_sha256"]
        ):
            raise MatrixError("CREBAIN capture differs from the publication commit")
        pack_tool_records.append(
            copy.deepcopy(
                capture["installed_package_proof"]["engram_pack_receipt"]["engram_tool"]
            )
        )
        capture_snapshots.append((capture_path, capture_payload))
    verify_matrix_prefixes(captures)
    source_rosters = {row["source"]["engram_source_roster_sha256"] for row in captures}
    source_file_counts = {row["source"]["engram_source_file_count"] for row in captures}
    if len(source_rosters) != 1 or len(source_file_counts) != 1:
        raise MatrixError("CREBAIN captures do not share one Engram source roster")
    if any(
        canonical(record) != canonical(pack_tool_records[0])
        for record in pack_tool_records[1:]
    ):
        raise MatrixError("CREBAIN captures do not share one Engram pack tool")
    document: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "review_scope": (
            "crebain-real-nest-captures-through-prisoma-read-only-observer"
        ),
        "reviewed_development_only": True,
        "production_manager_execution": False,
        "sources": {
            "prisoma_repository": {
                key: value for key, value in prisoma.items() if key != "root"
            },
            "crebain_source_repository": index_review["crebain_source_repository"],
            "crebain_evidence_publication": {
                key: value
                for key, value in crebain_publication.items()
                if key != "root"
            },
            "engram_repository": {
                key: value for key, value in engram.items() if key != "root"
            },
            "index_path": INDEX_RELATIVE.as_posix(),
            "index_schema_version": index["schema_version"],
            "index_exact_sha256": digest_bytes(index_payload),
            "input_suite_path": INPUT_SUITE_RELATIVE.as_posix(),
            "input_suite_exact_sha256": input_suite["exact_sha256"],
            "suite_definition_sha256": input_suite["document"][
                "suite_definition_sha256"
            ],
            "nest_config_exact_sha256": input_suite["config_exact_sha256"],
            "tool_source_closure_sha256": tool_sources["roster_sha256"],
            "installed_package_proof_exact_sha256": index[
                "installed_package_proof_exact_sha256"
            ],
            "installed_package_proof_receipt_sha256": index["package"][
                "receipt_sha256"
            ],
            "observed_build_receipt_exact_sha256": index["package"][
                "observed_build_receipt_exact_sha256"
            ],
            "observed_build_receipt_sha256": index["package"][
                "observed_build_receipt_sha256"
            ],
            "package_stage_receipt_exact_sha256": index["package"][
                "package_stage_receipt_exact_sha256"
            ],
            "package_stage_receipt_sha256": index["package"][
                "package_stage_receipt_sha256"
            ],
            "engram_pack_receipt_exact_sha256": index["package"][
                "engram_pack_receipt_exact_sha256"
            ],
            "engram_pack_receipt_sha256": index["package"][
                "engram_pack_receipt_sha256"
            ],
            "engram_extension_tool_sha256": index["package"][
                "engram_extension_tool_sha256"
            ],
            "engram_extension_tool_git_blob": index["package"][
                "engram_extension_tool_git_blob"
            ],
            "build_source_roster_sha256": index["package"][
                "build_source_roster_sha256"
            ],
            "build_input_identity_sha256": index["package"][
                "build_input_identity_sha256"
            ],
            "package_inventory_sha256": captures[0]["capture"][
                "package_inventory_sha256"
            ],
            "executable_sha256": index["package"]["executable_sha256"],
            "shared_engram_source_roster_sha256": next(iter(source_rosters)),
            "shared_engram_source_file_count": next(iter(source_file_counts)),
        },
        "observer_binary": observer_binary_projection(binary_identity),
        "captures": captures,
        "assertions": {
            "exact_drone_count_matrix": True,
            "capture_bytes_bound_by_index": True,
            "index_v2_closed_schema_verified": True,
            "tracked_input_suite_joined": True,
            "tool_source_closure_joined": True,
            "installed_package_proof_joined": True,
            "observed_build_source_closure_joined": True,
            "package_stage_byte_inventory_joined": True,
            "engram_pack_source_lineage_common": True,
            "observed_build_stage_seal_pack_install_lineage_common": True,
            "external_validator_source_closure_joined": True,
            "distinct_receipt_stores_verified": True,
            "immutable_source_revisions_verified": True,
            "crebain_bootstrap_source_lineage_verified": True,
            "crebain_evidence_publication_verified": True,
            "shared_source_roster_verified": True,
            "distinct_runtime_source_closures_verified": True,
            "channel_subject_rosters_joined": True,
            "terminal_and_nest_digests_joined": True,
            "neural_step_lineage_joined": True,
            "population_topology_joined": True,
            "nest_population_readback_rosters_joined": True,
            "worker_guardian_closure_joined": True,
            "nest_v2_execution_lineage_joined": True,
            "receipt_store_closure_joined": True,
            "receipt_store_terminal_evidence_bytes_joined": True,
            "receipt_store_v5_metadata_lock_joined": True,
            "receipt_store_sidecar_path_identities_joined": True,
            "real_nest_3_9_verified": True,
            "fault_hold_recovery_verified": True,
            "reviewed_runtime_lifecycle_joined": True,
            "observer_observed_build_receipt_joined": True,
            "observer_release_binary_executed": True,
            "observer_terminal_state_cleared": True,
            "authority_remained_absent": True,
        },
        "authority": {
            "descriptive_only": True,
            "observer_role": "read-only-observer",
            "observer_source_durable_evidence_verified": False,
            "agent_bridge_command": False,
            "execution_authority": False,
            "store_installation_authority": False,
            "publisher_authenticated": False,
            "durable_process_launch_authority": False,
            "replayable_live_launch_authority": False,
            "ncp_authority": False,
            "music_authority": False,
            "physical_authority": False,
            "plant_control": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
    }
    document["receipt_sha256"] = digest_bytes(canonical(document))
    validate_matrix_document(document)
    verify_repository(ROOT, expected_prisoma_revision, "Prisoma")
    try:
        if (
            capture_evidence_publication(
                crebain_source["root"],
                expected_crebain_source_revision,
                expected_crebain_publication_revision,
                EVIDENCE_PUBLICATION_PATHS,
                MAX_CAPTURE_BYTES,
                max_total_bytes=3 * MAX_CAPTURE_BYTES + MAX_INDEX_BYTES,
            )
            != publication_review
        ):
            raise MatrixError("CREBAIN evidence publication changed during review")
    except (OSError, ValueError) as error:
        raise MatrixError(
            "CREBAIN evidence publication changed during review"
        ) from error
    verify_repository(engram["root"], expected_engram_revision, "Engram")
    try:
        final_build_verification = observed_build.verify_observed_build_receipt(
            binary_identity["build_receipt_path"],
            binary_identity["path"],
            expected_prisoma_revision,
        )
    except observed_build.BuildObservationError as error:
        raise MatrixError(
            f"observer observed-build receipt changed during review: {error}"
        ) from error
    if (
        snapshot_regular_file(binary_identity["path"], MAX_BINARY_BYTES)
        != binary_identity["payload"]
        or final_build_verification["payload"]
        != binary_identity["build_receipt_payload"]
        or snapshot_regular_file(index_path, MAX_INDEX_BYTES) != index_payload
        or any(
            snapshot_regular_file(path, MAX_CAPTURE_BYTES) != payload
            for path, payload in capture_snapshots
        )
    ):
        raise MatrixError("matrix input changed during review")
    verify_repository(engram["root"], expected_engram_revision, "Engram")
    validate_source_file(
        engram["root"],
        engram["commit"],
        pack_tool_records[0],
    )
    verify_repository(engram["root"], expected_engram_revision, "Engram")
    payload = canonical(document) + b"\n"
    if len(payload) > MAX_MATRIX_OUTPUT_BYTES:
        raise MatrixError("CREBAIN observer matrix output exceeds its bound")
    return document, payload


def receipt_store_handshake_fixture(
    package_generation_id: str,
    runtime_generation_id: str,
) -> dict[str, Any]:
    handshake: dict[str, Any] = {
        "schema_version": "engram.reviewed-native-development-handshake.v1",
        "installation_id": "inst_" + "3" * 64,
        "generation_id": runtime_generation_id,
        "generation_ordinal": 1,
        "extension_id": EXPECTED_CREBAIN_EXTENSION_ID,
        "extension_version": EXPECTED_CREBAIN_EXTENSION_VERSION,
        "target_id": EXPECTED_CREBAIN_TARGET_ID,
        "profile": EXPECTED_REVIEWED_PROFILE,
        "executable_sha256": "4" * 64,
        "validator_set_sha256": "5" * 64,
        "launch_source": "package-store-lease",
        "store_id": "extstore_" + "2" * 64,
        "package_generation_id": package_generation_id,
        "package_generation_lease_retained": True,
        "generation_directory_identity_sha256": "6" * 64,
        "host_handshake_frame_sha256": "7" * 64,
        "runtime_handshake_frame_sha256": "8" * 64,
        "handshake_transcript_accepted": True,
        "child_ready_claim": False,
        "host_local_admission": True,
        "process_launch_performed": True,
        "explicit_absolute_path_spawn": True,
        "exec_gate_command_sha256": "d" * 64,
        "exec_gate_source_sha256": "e" * 64,
        "path_lookup_at_spawn": True,
        "package_path_reopened_for_spawn": False,
        "verified_executable_staged": True,
        "staged_executable_owner_private": True,
        "staged_executable_user_immutable": True,
        "process_group_containment": True,
        "guardian_source_sha256": "a" * 64,
        "guardian_command_sha256": "9" * 64,
        "guardian_pid": 124,
        "process_pid": 123,
        "process_group_id": 123,
        "session_id": 122,
        "runtime_process_group_leader": True,
        "guardian_group_member": True,
        "guardian_ready_frame_sha256": "b" * 64,
        "guardian_owner_loss_seal": True,
        "guardian_generation_lease_retained": True,
        "guardian_uncertainty_record_prepared": True,
        "descendant_creation_denied": True,
        "os_sandbox_enforced": True,
        "network_isolation_enforced": True,
        "filesystem_isolation_enforced": False,
        "sandbox_profile_sha256": "c" * 64,
        "sandbox_launcher_sha256": "f" * 64,
        "external_dependency_closure_attested": False,
        "automatic_restart": False,
        "publisher_authenticated": False,
        "durable_process_launch_authority": False,
        "replayable_live_launch_authority": False,
        "ncp_authority": False,
        "physical_authority": False,
        "scientific_authority": False,
    }
    handshake["receipt_sha256"] = digest_bytes(canonical(handshake))
    return handshake


def receipt_store_sidecar_fixture(
    *,
    store_id: str,
    reservation_id: str,
    study_run_id: str,
    terminal_receipt_sha256: str,
    terminal_artifact_size_bytes: int,
    evidence_bundle_sha256: str,
    evidence_artifact_size_bytes: int,
    package_generation_id: str,
    runtime_generation_id: str,
    closed_loop_definition_sha256: str,
    runtime_binding_sha256: str,
    run_plan_sha256: str,
    nest_configuration_sha256: str,
    nest_work_admission_sha256: str,
    reserved_evidence_bytes: int,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    def seal(document: dict[str, Any], field: str) -> dict[str, Any]:
        result = copy.deepcopy(document)
        result[field] = managed_runtime_digest(result)
        return result

    handshake = receipt_store_handshake_fixture(
        package_generation_id,
        runtime_generation_id,
    )
    pre_spawn_sha256 = "0" * 64
    reservation = seal(
        {
            "schema_version": "engram.extension-closed-loop-receipt-reservation.v1",
            "store_id": store_id,
            "reservation_id": reservation_id,
            "study_run_id": study_run_id,
            "closed_loop_definition_sha256": closed_loop_definition_sha256,
            "receipt_profile": "engram.extension-closed-loop-run-receipt.v2",
            "evidence_profile": ("optional-engram.nest-closed-loop-evidence-bundle.v2"),
            "nest_work_admission_sha256": nest_work_admission_sha256,
            "pre_spawn_sha256": pre_spawn_sha256,
            "run_plan_sha256": run_plan_sha256,
            "nest_configuration_sha256": nest_configuration_sha256,
            "expected_runtime_binding_sha256": runtime_binding_sha256,
            "reviewed_native_handshake_receipt_sha256": handshake["receipt_sha256"],
            "reviewed_native_handshake": handshake,
            "package_generation_id": package_generation_id,
            "runtime_generation_id": runtime_generation_id,
            "reserved_record_count": 1,
            "reserved_artifact_bytes": 16 * 1024 * 1024,
            "reserved_evidence_bytes": reserved_evidence_bytes,
            "reserved_record_bytes": 4096,
            "execution_authority": False,
            "ncp_control": False,
            "physical_actuation": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
        "reservation_sha256",
    )
    simulation_dispatch_sha256 = digest_bytes(
        managed_runtime_canonical(
            {
                "schema_version": "engram.extension-closed-loop-dispatch-intent.v1",
                "store_id": store_id,
                "reservation_id": reservation_id,
                "reservation_sha256": reservation["reservation_sha256"],
            }
        )
    )
    extension_dispatch_sha256 = digest_bytes(
        managed_runtime_canonical(
            {
                "schema_version": (
                    "engram.extension-closed-loop-extension-dispatch-intent.v1"
                ),
                "store_id": store_id,
                "reservation_id": reservation_id,
                "pre_spawn_sha256": pre_spawn_sha256,
            }
        )
    )
    finalization = seal(
        {
            "schema_version": "engram.extension-closed-loop-finalized-reservation.v1",
            "store_id": store_id,
            "reservation": reservation,
            "pre_spawn_sha256": pre_spawn_sha256,
            "extension_dispatch_sha256": extension_dispatch_sha256,
            "simulation_dispatch_sha256": simulation_dispatch_sha256,
            "terminal_receipt_sha256": terminal_receipt_sha256,
            "evidence_bundle_sha256": evidence_bundle_sha256,
            "nest_work_admission_rejoined": True,
            "execution_authority": False,
            "ncp_control": False,
            "physical_actuation": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
        "finalization_sha256",
    )
    publication_wal_sha256 = digest_bytes(
        managed_runtime_canonical(
            {
                "domain": (
                    "engram-extension-closed-loop-reserved-publication-wal-closure-v1"
                ),
                "store_id": store_id,
                "reservation_id": reservation_id,
                "pre_spawn_sha256": pre_spawn_sha256,
                "extension_dispatch_sha256": extension_dispatch_sha256,
                "reservation_sha256": reservation["reservation_sha256"],
                "simulation_dispatch_sha256": simulation_dispatch_sha256,
                "terminal_receipt_sha256": terminal_receipt_sha256,
            }
        )
    )
    study_run_key_sha256 = digest_bytes(
        managed_runtime_canonical(
            {
                "domain": "engram-extension-closed-loop-publication-study-run-key-v1",
                "store_id": store_id,
                "study_run_id": study_run_id,
            }
        )
    )
    anchor = seal(
        {
            "schema_version": (
                "engram.extension-closed-loop-publication-admission-anchor.v1"
            ),
            "store_id": store_id,
            "study_run_key_sha256": study_run_key_sha256,
            "study_run_id": study_run_id,
            "terminal_receipt_sha256": terminal_receipt_sha256,
            "admission_mode": "reserved",
            "publication_wal_sha256": publication_wal_sha256,
            "evidence_bundle_sha256": evidence_bundle_sha256,
            "reservation_id": reservation_id,
            "reservation_sha256": reservation["reservation_sha256"],
            "pre_spawn_sha256": pre_spawn_sha256,
            "extension_dispatch_sha256": extension_dispatch_sha256,
            "simulation_dispatch_sha256": simulation_dispatch_sha256,
            "reservation_finalization_sha256": finalization["finalization_sha256"],
            "execution_authority": False,
            "ncp_control": False,
            "physical_actuation": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
        "anchor_sha256",
    )
    authority = seal(
        {
            "schema_version": "engram.extension-closed-loop-publication-authority.v1",
            "store_id": store_id,
            "terminal_receipt_sha256": terminal_receipt_sha256,
            "study_run_id": study_run_id,
            "admission_mode": "reserved",
            "publication_admission_anchor_sha256": anchor["anchor_sha256"],
            "publication_wal_sha256": publication_wal_sha256,
            "evidence_bundle_sha256": evidence_bundle_sha256,
            "reservation_id": reservation_id,
            "reservation_sha256": reservation["reservation_sha256"],
            "reservation_finalization_sha256": finalization["finalization_sha256"],
            "nest_work_admission_sha256": nest_work_admission_sha256,
            "execution_authority": False,
            "ncp_control": False,
            "physical_actuation": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
        "authority_sha256",
    )
    receipt_path = (
        f"receipts/{terminal_receipt_sha256[:2]}/{terminal_receipt_sha256}.json"
    )
    evidence_path = (
        f"evidence/{evidence_bundle_sha256[:2]}/{evidence_bundle_sha256}.json"
    )
    observation = seal(
        {
            "schema_version": "engram.extension-closed-loop-stored-receipt.v5",
            "store_id": store_id,
            "artifact": {
                "artifact_id": f"art_{terminal_receipt_sha256[:32]}",
                "kind": "closed_loop_receipt",
                "sha256": terminal_receipt_sha256,
            },
            "study_run_id": study_run_id,
            "run_status": "completed",
            "terminal_reason_code": "loop.completed",
            "relative_artifact_path": receipt_path,
            "artifact_byte_length": terminal_artifact_size_bytes,
            "evidence_profile": "killable-nest-population-controller-v2",
            "evidence_bundle_sha256": evidence_bundle_sha256,
            "relative_evidence_path": evidence_path,
            "evidence_byte_length": evidence_artifact_size_bytes,
            "admission_mode": "reserved",
            "publication_authority_sha256": authority["authority_sha256"],
            "reservation_id": reservation_id,
            "reservation_sha256": reservation["reservation_sha256"],
            "reservation_finalization_sha256": finalization["finalization_sha256"],
            "nest_work_admission_sha256": nest_work_admission_sha256,
            "nest_work_admission_rejoined": True,
            "digest_canonicalization": RECEIPT_STORE_CANONICALIZATION,
            "execution_authority": False,
            "ncp_control": False,
            "physical_actuation": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
        "record_sha256",
    )
    metadata = {
        "schema_version": RECEIPT_STORE_SCHEMA,
        "store_id": store_id,
        "policy": RECEIPT_STORE_POLICY,
        "digest_canonicalization": RECEIPT_STORE_CANONICALIZATION,
        "execution_authority": False,
        "ncp_control": False,
        "physical_actuation": False,
        "scientific_authority": False,
        "is_paper_local_evidence": False,
        "calibrated_posterior": False,
    }
    sidecars = {
        "schema_version": RECEIPT_STORE_SIDECARS_SCHEMA,
        "store_metadata": metadata,
        "finalized_reservation": finalization,
        "observation": observation,
        "publication_admission_anchor": anchor,
        "publication_authority": authority,
    }
    sidecars["closure_sha256"] = digest_bytes(canonical(sidecars))
    material = {
        "store.json": managed_runtime_canonical(metadata),
        "writer.lock": RECEIPT_STORE_LOCK_BYTES,
        evidence_path: bytes.fromhex(evidence_bundle_sha256),
        (
            f"finalized-reservations/{reservation_id[5:7]}/{reservation_id}.json"
        ): managed_runtime_canonical(finalization),
        (
            f"observations/{terminal_receipt_sha256[:2]}/{terminal_receipt_sha256}.json"
        ): managed_runtime_canonical(observation),
        f"publication-admission-anchors/{study_run_key_sha256}.json": (
            managed_runtime_canonical(anchor)
        ),
        (
            f"publication-authorities/{terminal_receipt_sha256[:2]}/"
            f"{terminal_receipt_sha256}.json"
        ): managed_runtime_canonical(authority),
        receipt_path: bytes.fromhex(terminal_receipt_sha256),
    }
    rows = [
        {
            "relative_path": path,
            "size_bytes": (
                evidence_artifact_size_bytes
                if path == evidence_path
                else terminal_artifact_size_bytes
                if path == receipt_path
                else len(payload)
            ),
            "sha256": (
                evidence_bundle_sha256
                if path == evidence_path
                else terminal_receipt_sha256
                if path == receipt_path
                else digest_bytes(payload)
            ),
        }
        for path, payload in sorted(material.items())
    ]
    return sidecars, rows


def matrix_fixture_capture(
    drone_count: int,
    observer: dict[str, Any],
) -> dict[str, Any]:
    digest = format(drone_count, "x") * 64
    evidence_digest = format(drone_count + 3, "x") * 64
    store_digest = format(drone_count + 6, "x") * 64
    receipt_store_id = f"clrs_{store_digest}"
    study_run_id = f"matrix-{drone_count}-drone"
    channels = [f"channel-{index:02d}" for index in range(1, drone_count + 1)]
    subjects = [f"subject-{index:02d}" for index in range(1, drone_count + 1)]
    populations = [f"fleet.channel-{index:02d}" for index in range(1, drone_count + 1)]
    observer = copy.deepcopy(observer)
    observer["terminal_source_receipt_sha256"] = digest
    reservation_digest = digest_bytes(f"reservation-{drone_count}".encode("ascii"))
    reservation_id = f"clrr_{reservation_digest}"
    package_generation_id = "pkggen_" + "f" * 64
    receipt_store_sidecars, receipt_store_files = receipt_store_sidecar_fixture(
        store_id=receipt_store_id,
        reservation_id=reservation_id,
        study_run_id=study_run_id,
        terminal_receipt_sha256=digest,
        terminal_artifact_size_bytes=1,
        evidence_bundle_sha256=evidence_digest,
        evidence_artifact_size_bytes=1,
        package_generation_id=package_generation_id,
        runtime_generation_id="gen_" + format(drone_count + 9, "x") * 64,
        closed_loop_definition_sha256="a" * 64,
        runtime_binding_sha256="b" * 64,
        run_plan_sha256="c" * 64,
        nest_configuration_sha256="d" * 64,
        nest_work_admission_sha256="e" * 64,
        reserved_evidence_bytes=4096,
    )
    fixture_store_closure = {
        "schema_version": "crebain.closed-loop-receipt-store-closure.v1",
        "store_id": receipt_store_id,
        "receipt_sha256": digest,
        "receipt_artifact_path": f"receipts/{digest[:2]}/{digest}.json",
        "evidence_bundle_sha256": evidence_digest,
        "evidence_artifact_path": (
            f"evidence/{evidence_digest[:2]}/{evidence_digest}.json"
        ),
        "file_count": len(receipt_store_files),
        "total_bytes": sum(row["size_bytes"] for row in receipt_store_files),
        "files": receipt_store_files,
    }
    fixture_store_closure_sha256 = digest_bytes(canonical(fixture_store_closure))
    return {
        "drone_count": drone_count,
        "capture": {
            "path": CAPTURE_PATHS[drone_count],
            "exact_sha256": digest,
            "schema_version": CAPTURE_SCHEMA_VERSION,
            "plan_exact_sha256": digest,
            "package_generation_id": package_generation_id,
            "installed_package_proof_exact_sha256": "a" * 64,
            "installed_package_proof_receipt_sha256": "b" * 64,
            "observed_build_receipt_exact_sha256": "c" * 64,
            "observed_build_receipt_sha256": "d" * 64,
            "package_stage_receipt_exact_sha256": "e" * 64,
            "package_stage_receipt_sha256": "f" * 64,
            "engram_pack_receipt_exact_sha256": "6" * 64,
            "engram_pack_receipt_sha256": "7" * 64,
            "engram_extension_tool_sha256": "8" * 64,
            "engram_extension_tool_git_blob": "9" * 40,
            "build_source_roster_sha256": "0" * 64,
            "build_input_identity_sha256": "1" * 64,
            "package_inventory_sha256": "2" * 64,
            "executable_sha256": "3" * 64,
            "terminal_receipt_sha256": digest,
            "nest_evidence_bundle_sha256": evidence_digest,
            "receipt_store_id": receipt_store_id,
            "reservation_id": reservation_id,
            "receipt_store_closure_sha256": fixture_store_closure_sha256,
            "receipt_store_file_count": RECEIPT_STORE_FILE_COUNT,
            "receipt_store_files": receipt_store_files,
            "receipt_store_file_roster_sha256": digest_bytes(
                canonical(receipt_store_files)
            ),
            "receipt_store_sidecars": receipt_store_sidecars,
            "receipt_store_sidecars_sha256": receipt_store_sidecars["closure_sha256"],
            "terminal_artifact_path": f"receipts/{digest[:2]}/{digest}.json",
            "terminal_artifact_size_bytes": 1,
            "terminal_artifact_exact_sha256": digest,
            "evidence_artifact_path": (
                f"evidence/{evidence_digest[:2]}/{evidence_digest}.json"
            ),
            "evidence_artifact_size_bytes": 1,
            "evidence_artifact_exact_sha256": evidence_digest,
        },
        "source": {
            "engram_revision": "3" * 40,
            "engram_tree": "4" * 40,
            "engram_source_closure_sha256": digest,
            "engram_source_file_count": 8,
            "engram_source_roster_sha256": "5" * 64,
        },
        "scenario": {
            "study_run_id": study_run_id,
            "channel_ids": channels,
            "subject_ids": subjects,
            "subject_kind": "simulated.drone",
            "neural_population_prefixes": populations,
            "planned_step_count": observer["observed_step_count"],
            "completed_step_count": observer["observed_step_count"],
            "faulted_channel_ordinal": 1,
        },
        "nest": {
            "profile": "killable-nest-population-controller-v2",
            "reported_version": "3.9.0",
            "one_session": True,
            "population_roster_sha256": digest,
            "control_binding_sha256": digest,
            "connection_readback_sha256": digest,
            "neural_provider_identity_sha256": digest,
            "signed_population_count": drone_count * 6,
            "population_count": drone_count * 6,
            "population_neuron_count": drone_count * 48,
            "device_node_count": drone_count * 12,
            "connection_count": drone_count * 96,
            "derived_population_roster_sha256": "d" * 64,
            "worker_session_binding_receipt_sha256": "e" * 64,
            "worker_runtime_identity_receipt_sha256": "f" * 64,
            "worker_lifecycle_receipt_sha256": "0" * 64,
            "termination_attempt_roster_sha256": "1" * 64,
            "runtime_launch_expectation_sha256": "2" * 64,
            "worker_launch_attempt_sha256": "3" * 64,
            "preparation_attempt_sha256": "4" * 64,
            "child_capabilities_sha256": "5" * 64,
            "child_preparation_receipt_sha256": "6" * 64,
            "provider_preparation_receipt_sha256": "7" * 64,
            "step_attempt_roster_sha256": "8" * 64,
            "step_attempt_count": observer["observed_step_count"],
            "external_validator_receipt_sha256": digest,
            "source_durable_evidence_verified": True,
        },
        "observer": observer,
        "lifecycle": {
            "runtime_handshake_receipt_sha256": receipt_store_sidecars[
                "finalized_reservation"
            ]["reservation"]["reviewed_native_handshake_receipt_sha256"],
            "runtime_termination_receipt_sha256": digest,
            "runtime_lifecycle_binding_sha256": digest,
            "exec_gate_command_sha256": "9" * 64,
            "exec_gate_source_sha256": "a" * 64,
            "runtime_process_group_containment_verified": True,
            "termination_disposition": "clean-exit",
            "child_reaped": True,
            "guardian_reaped": True,
            "containment_empty": True,
            "diagnostic_stream_complete": True,
            "private_work_directory_removed": True,
            "package_generation_lease_released": True,
            "cleanup_complete": True,
            "filesystem_isolation_enforced": False,
        },
        "authority": {
            "simulator_only": True,
            "descriptive_only": True,
            "agent_bridge_command": False,
            "execution_authority": False,
            "ncp_used": False,
            "music_used": False,
            "ncp_qualified": False,
            "physical_actuation": False,
            "plant_control": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
    }


def reseal_matrix_store_projection(capture: dict[str, Any]) -> None:
    """Reseal one projected V5 roster after a controlled hostile mutation."""

    files = capture["receipt_store_files"]
    files.sort(key=lambda row: row["relative_path"])
    capture["receipt_store_file_count"] = len(files)
    capture["receipt_store_file_roster_sha256"] = digest_bytes(canonical(files))
    closure = {
        "schema_version": "crebain.closed-loop-receipt-store-closure.v1",
        "store_id": capture["receipt_store_id"],
        "receipt_sha256": capture["terminal_receipt_sha256"],
        "receipt_artifact_path": capture["terminal_artifact_path"],
        "evidence_bundle_sha256": capture["nest_evidence_bundle_sha256"],
        "evidence_artifact_path": capture["evidence_artifact_path"],
        "file_count": len(files),
        "total_bytes": sum(row["size_bytes"] for row in files),
        "files": files,
    }
    capture["receipt_store_closure_sha256"] = digest_bytes(canonical(closure))


def matrix_fixture_document(
    binary_identity: dict[str, Any],
    observers: list[dict[str, Any]],
) -> dict[str, Any]:
    captures = [
        matrix_fixture_capture(count, observers[count - 1]) for count in (1, 2, 3)
    ]
    publication_sha256 = {
        INDEX_RELATIVE.as_posix(): "8" * 64,
        **{
            (EVIDENCE_DIRECTORY_RELATIVE / row["capture"]["path"]).as_posix(): (
                row["capture"]["exact_sha256"]
            )
            for row in captures
        },
    }
    publication_files = [
        {
            "path": path.as_posix(),
            "size_bytes": 1,
            "sha256": publication_sha256[path.as_posix()],
            "git_mode": "100644",
            "git_blob": format(position, "x") * 40,
        }
        for position, path in enumerate(EVIDENCE_PUBLICATION_PATHS, start=10)
    ]
    document: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "review_scope": (
            "crebain-real-nest-captures-through-prisoma-read-only-observer"
        ),
        "reviewed_development_only": True,
        "production_manager_execution": False,
        "sources": {
            "prisoma_repository": {
                "repository": "https://github.com/sepahead/prisoma.git",
                "commit": "1" * 40,
                "tree": "2" * 40,
                "origin_main": "1" * 40,
                "object_format": "sha1",
                "clean": True,
            },
            "crebain_source_repository": {
                "repository": "https://github.com/sepahead/crebain.git",
                "commit": "6" * 40,
                "tree": "7" * 40,
                "origin_main_at_capture": "6" * 40,
                "object_format": "sha1",
                "clean_at_capture": True,
            },
            "crebain_evidence_publication": {
                "repository": "https://github.com/sepahead/crebain.git",
                "commit": "8" * 40,
                "tree": "9" * 40,
                "origin_main": "8" * 40,
                "object_format": "sha1",
                "clean": True,
                "parent_commit": "6" * 40,
                "policy": EVIDENCE_PUBLICATION_POLICY,
                "evidence_directory": EVIDENCE_DIRECTORY_RELATIVE.as_posix(),
                "files": publication_files,
                "file_count": len(publication_files),
                "roster_sha256": digest_bytes(
                    EVIDENCE_PUBLICATION_ROSTER_DOMAIN + canonical(publication_files)
                ),
            },
            "engram_repository": {
                "repository": "https://github.com/sepahead/engram.git",
                "commit": "3" * 40,
                "tree": "4" * 40,
                "origin_main": "3" * 40,
                "object_format": "sha1",
                "clean": True,
            },
            "index_path": INDEX_RELATIVE.as_posix(),
            "index_schema_version": INDEX_SCHEMA_VERSION,
            "index_exact_sha256": "8" * 64,
            "input_suite_path": INPUT_SUITE_RELATIVE.as_posix(),
            "input_suite_exact_sha256": "a" * 64,
            "suite_definition_sha256": "b" * 64,
            "nest_config_exact_sha256": "c" * 64,
            "tool_source_closure_sha256": "9" * 64,
            "installed_package_proof_exact_sha256": "a" * 64,
            "installed_package_proof_receipt_sha256": "b" * 64,
            "observed_build_receipt_exact_sha256": "c" * 64,
            "observed_build_receipt_sha256": "d" * 64,
            "package_stage_receipt_exact_sha256": "e" * 64,
            "package_stage_receipt_sha256": "f" * 64,
            "engram_pack_receipt_exact_sha256": "6" * 64,
            "engram_pack_receipt_sha256": "7" * 64,
            "engram_extension_tool_sha256": "8" * 64,
            "engram_extension_tool_git_blob": "9" * 40,
            "build_source_roster_sha256": "0" * 64,
            "build_input_identity_sha256": "1" * 64,
            "package_inventory_sha256": "2" * 64,
            "executable_sha256": "3" * 64,
            "shared_engram_source_roster_sha256": "5" * 64,
            "shared_engram_source_file_count": 8,
        },
        "observer_binary": observer_binary_projection(binary_identity),
        "captures": captures,
        "assertions": {
            "exact_drone_count_matrix": True,
            "capture_bytes_bound_by_index": True,
            "index_v2_closed_schema_verified": True,
            "tracked_input_suite_joined": True,
            "tool_source_closure_joined": True,
            "installed_package_proof_joined": True,
            "observed_build_source_closure_joined": True,
            "package_stage_byte_inventory_joined": True,
            "engram_pack_source_lineage_common": True,
            "observed_build_stage_seal_pack_install_lineage_common": True,
            "external_validator_source_closure_joined": True,
            "distinct_receipt_stores_verified": True,
            "immutable_source_revisions_verified": True,
            "crebain_bootstrap_source_lineage_verified": True,
            "crebain_evidence_publication_verified": True,
            "shared_source_roster_verified": True,
            "distinct_runtime_source_closures_verified": True,
            "channel_subject_rosters_joined": True,
            "terminal_and_nest_digests_joined": True,
            "neural_step_lineage_joined": True,
            "population_topology_joined": True,
            "nest_population_readback_rosters_joined": True,
            "worker_guardian_closure_joined": True,
            "nest_v2_execution_lineage_joined": True,
            "receipt_store_closure_joined": True,
            "receipt_store_terminal_evidence_bytes_joined": True,
            "receipt_store_v5_metadata_lock_joined": True,
            "receipt_store_sidecar_path_identities_joined": True,
            "real_nest_3_9_verified": True,
            "fault_hold_recovery_verified": True,
            "reviewed_runtime_lifecycle_joined": True,
            "observer_observed_build_receipt_joined": True,
            "observer_release_binary_executed": True,
            "observer_terminal_state_cleared": True,
            "authority_remained_absent": True,
        },
        "authority": {
            "descriptive_only": True,
            "observer_role": "read-only-observer",
            "observer_source_durable_evidence_verified": False,
            "agent_bridge_command": False,
            "execution_authority": False,
            "store_installation_authority": False,
            "publisher_authenticated": False,
            "durable_process_launch_authority": False,
            "replayable_live_launch_authority": False,
            "ncp_authority": False,
            "music_authority": False,
            "physical_authority": False,
            "plant_control": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        },
    }
    document["receipt_sha256"] = digest_bytes(canonical(document))
    return document


def source_closure_fixture() -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    exec_gate_path = "backend/integrations/contained_exec_gate.py"
    exec_gate_sha256 = "9" * 64
    source_path = "backend/integrations/reviewed_native_process_guardian.py"
    source_sha256 = "a" * 64
    nest_guardian_path = "backend/optimization/extension_closed_loop_nest_guardian.py"
    nest_guardian_sha256 = "8" * 64
    nest_worker_path = "backend/optimization/extension_closed_loop_nest_worker.py"
    nest_worker_sha256 = "7" * 64
    pack_tool_path = EXPECTED_ENGRAM_PACK_TOOL_PATH
    pack_tool_sha256 = "b" * 64
    pack_tool = {
        "relative_path": pack_tool_path,
        "size_bytes": 2,
        "sha256": pack_tool_sha256,
        "git_mode": "100755",
        "git_blob": "e" * 40,
    }
    worker_project_files = [
        {
            "role": (
                "project-module:backend.integrations.reviewed_native_process_guardian"
            ),
            "absolute_path": f"/reviewed-engram/{source_path}",
            "sha256": source_sha256,
            "size_bytes": 1,
        }
    ]
    python_executable_sha256 = "6" * 64
    worker_runtime_files = [
        {
            "role": "python-executable",
            "absolute_path": "/fixture/python",
            "sha256": python_executable_sha256,
            "size_bytes": 1,
        },
        *worker_project_files,
    ]
    worker_roster_sha256 = digest_bytes(canonical(worker_project_files))
    exec_gate_command_binding: dict[str, Any] = {
        "schema_version": "engram.contained-exec-command.v1",
        "python_executable_sha256": python_executable_sha256,
        "exec_gate_source_sha256": exec_gate_sha256,
        "argument_shape": EXPECTED_EXEC_GATE_ARGUMENT_SHAPE,
        "target_command_sha256": "d" * 64,
    }
    exec_gate_command_binding["exec_gate_command_sha256"] = digest_bytes(
        canonical(exec_gate_command_binding)
    )
    handshake_sha256 = "c" * 64
    closure: dict[str, Any] = {
        "schema_version": "crebain.engram-python-source-closure.v1",
        "discovery_policy": (
            "loaded-host-modules-plus-worker-runtime-identity-and-entrypoints.v1"
        ),
        "git": {
            "repository": "https://example.invalid/engram.git",
            "commit": "3" * 40,
            "tree": "4" * 40,
            "origin_main": "3" * 40,
            "object_format": "sha1",
            "clean": True,
        },
        "host_modules": [
            {
                "module_name": "backend.integrations.contained_exec_gate",
                "relative_path": exec_gate_path,
            },
            {
                "module_name": (
                    "backend.integrations.reviewed_native_process_guardian"
                ),
                "relative_path": source_path,
            },
            {
                "module_name": "scripts.engram_extension",
                "relative_path": pack_tool_path,
            },
        ],
        "worker_project_modules": [
            {
                "module_name": (
                    "backend.integrations.reviewed_native_process_guardian"
                ),
                "relative_path": source_path,
            }
        ],
        "worker_project_source_roster_sha256": worker_roster_sha256,
        "reviewed_runtime_handshake_receipt_sha256": handshake_sha256,
        "reviewed_runtime_guardian_source_sha256": source_sha256,
        "reviewed_runtime_exec_gate_source_sha256": exec_gate_sha256,
        "reviewed_runtime_exec_gate_command_sha256": exec_gate_command_binding[
            "exec_gate_command_sha256"
        ],
        "exercised_entrypoints": [
            {"role": "nest-guardian", "relative_path": nest_guardian_path},
            {"role": "nest-worker", "relative_path": nest_worker_path},
            {"role": "reviewed-runtime-guardian", "relative_path": source_path},
        ],
        "sources": [
            {
                "relative_path": exec_gate_path,
                "size_bytes": 1,
                "sha256": exec_gate_sha256,
                "git_mode": "100644",
                "git_blob": "c" * 40,
            },
            {
                "relative_path": source_path,
                "size_bytes": 1,
                "sha256": source_sha256,
                "git_mode": "100644",
                "git_blob": "d" * 40,
            },
            {
                "relative_path": nest_guardian_path,
                "size_bytes": 1,
                "sha256": nest_guardian_sha256,
                "git_mode": "100644",
                "git_blob": "8" * 40,
            },
            {
                "relative_path": nest_worker_path,
                "size_bytes": 1,
                "sha256": nest_worker_sha256,
                "git_mode": "100644",
                "git_blob": "7" * 40,
            },
            pack_tool,
        ],
    }
    closure["source_roster_sha256"] = digest_bytes(
        b"crebain.engram-source-roster.v1\0" + canonical(closure["sources"])
    )
    closure["closure_sha256"] = digest_bytes(canonical(closure))
    capture = {
        "engram_source_sha256": {
            exec_gate_path: exec_gate_sha256,
            source_path: source_sha256,
            nest_guardian_path: nest_guardian_sha256,
            nest_worker_path: nest_worker_sha256,
            pack_tool_path: pack_tool_sha256,
        },
        "engram_source_closure": closure,
        "reviewed_native_runtime": {
            "handshake_receipt": {
                "receipt_sha256": handshake_sha256,
                "guardian_source_sha256": source_sha256,
                "exec_gate_source_sha256": exec_gate_sha256,
                "exec_gate_command_sha256": exec_gate_command_binding[
                    "exec_gate_command_sha256"
                ],
            },
            "termination_receipt": {"receipt_sha256": "e" * 64},
            "exec_gate_command_binding": exec_gate_command_binding,
            "lifecycle_binding_sha256": "f" * 64,
            "guardian_closure_verified": True,
            "package_store_lineage_verified": True,
        },
        "nest_evidence_bundle": {
            "worker_runtime_identity": {
                "project_source_closure_verified": True,
                "project_source_roster_sha256": worker_roster_sha256,
                "file_roster_sha256": digest_bytes(canonical(worker_runtime_files)),
                "files": worker_runtime_files,
            },
            "worker_session_binding": {
                "worker_project_source_roster_sha256": worker_roster_sha256,
            },
        },
    }
    expected_engram = {
        "root": ROOT,
        "repository": "https://example.invalid/engram.git",
        "commit": "3" * 40,
        "tree": "4" * 40,
        "origin_main": "3" * 40,
        "object_format": "sha1",
    }
    pack_receipt = {
        "engram_repository": {
            "origin": expected_engram["repository"],
            "commit": expected_engram["commit"],
            "tree": expected_engram["tree"],
            "origin_main": expected_engram["origin_main"],
            "object_format": expected_engram["object_format"],
            "clean": True,
        },
        "engram_tool": pack_tool,
    }
    return capture, expected_engram, pack_receipt


def lifecycle_fixture(source_capture: dict[str, Any]) -> dict[str, Any]:
    capture = copy.deepcopy(source_capture)
    package_generation_id = "pkggen_" + "f" * 64
    generation_id = "gen_" + "1" * 64
    store_id = "extstore_" + "2" * 64
    source_sha256 = capture["engram_source_closure"][
        "reviewed_runtime_guardian_source_sha256"
    ]
    exec_gate_command_binding = capture["reviewed_native_runtime"][
        "exec_gate_command_binding"
    ]
    handshake: dict[str, Any] = {
        "schema_version": "engram.reviewed-native-development-handshake.v1",
        "installation_id": "inst_" + "3" * 64,
        "generation_id": generation_id,
        "generation_ordinal": 1,
        "extension_id": EXPECTED_CREBAIN_EXTENSION_ID,
        "extension_version": EXPECTED_CREBAIN_EXTENSION_VERSION,
        "target_id": EXPECTED_CREBAIN_TARGET_ID,
        "profile": EXPECTED_REVIEWED_PROFILE,
        "executable_sha256": "4" * 64,
        "validator_set_sha256": "5" * 64,
        "launch_source": "package-store-lease",
        "store_id": store_id,
        "guardian_source_sha256": capture["engram_source_closure"][
            "reviewed_runtime_guardian_source_sha256"
        ],
        "package_generation_id": package_generation_id,
        "package_generation_lease_retained": True,
        "generation_directory_identity_sha256": "6" * 64,
        "host_handshake_frame_sha256": "7" * 64,
        "runtime_handshake_frame_sha256": "8" * 64,
        "handshake_transcript_accepted": True,
        "child_ready_claim": False,
        "host_local_admission": True,
        "process_launch_performed": True,
        "explicit_absolute_path_spawn": True,
        "exec_gate_command_sha256": exec_gate_command_binding[
            "exec_gate_command_sha256"
        ],
        "exec_gate_source_sha256": exec_gate_command_binding["exec_gate_source_sha256"],
        "path_lookup_at_spawn": True,
        "package_path_reopened_for_spawn": False,
        "verified_executable_staged": True,
        "staged_executable_owner_private": True,
        "staged_executable_user_immutable": True,
        "process_group_containment": True,
        "guardian_command_sha256": "9" * 64,
        "guardian_pid": 124,
        "process_pid": 123,
        "process_group_id": 123,
        "session_id": 122,
        "runtime_process_group_leader": True,
        "guardian_group_member": True,
        "guardian_ready_frame_sha256": "a" * 64,
        "guardian_owner_loss_seal": True,
        "guardian_generation_lease_retained": True,
        "guardian_uncertainty_record_prepared": True,
        "descendant_creation_denied": True,
        "os_sandbox_enforced": True,
        "network_isolation_enforced": True,
        "filesystem_isolation_enforced": False,
        "sandbox_profile_sha256": "b" * 64,
        "sandbox_launcher_sha256": "c" * 64,
        "external_dependency_closure_attested": False,
        "automatic_restart": False,
        "publisher_authenticated": False,
        "durable_process_launch_authority": False,
        "replayable_live_launch_authority": False,
        "ncp_authority": False,
        "physical_authority": False,
        "scientific_authority": False,
    }
    handshake["receipt_sha256"] = digest_bytes(canonical(handshake))
    handshake_sha256 = handshake["receipt_sha256"]
    termination: dict[str, Any] = {
        "schema_version": "engram.reviewed-native-development-termination.v1",
        "handshake_receipt_sha256": handshake_sha256,
        "generation_id": generation_id,
        "disposition": "clean-exit",
        "reason_code": "runtime.clean-exit",
        "exit_code": 0,
        "termination_signal": None,
        "child_reaped": True,
        "guardian_pid": 124,
        "process_group_id": 123,
        "guardian_reaped": True,
        "group_signal_while_guardian_unreaped": True,
        "direct_child_signal_while_unreaped": False,
        "containment_signal_scope": "process-group",
        "containment_seal_signal": 9,
        "containment_empty": True,
        "stderr_sha256": digest_bytes(b""),
        "stderr_retained_bytes": 0,
        "stderr_truncated": False,
        "diagnostic_stream_complete": True,
        "private_work_directory_removed": True,
        "package_generation_lease_released": True,
        "guardian_generation_lease_held_until_containment": True,
        "durable_process_launch_authority": False,
        "ncp_authority": False,
        "physical_authority": False,
        "scientific_authority": False,
    }
    termination["receipt_sha256"] = digest_bytes(canonical(termination))
    termination_sha256 = termination["receipt_sha256"]
    lifecycle: dict[str, Any] = {
        "schema_version": "engram.closed-loop-runtime-lifecycle-binding.v1",
        "profile": EXPECTED_REVIEWED_PROFILE,
        "generation_id": generation_id,
        "launch_source": "package-store-lease",
        "store_id": store_id,
        "package_generation_id": package_generation_id,
        "generation_directory_identity_sha256": "6" * 64,
        "package_generation_lease_retained_at_launch": True,
        "package_generation_lease_released": True,
        "handshake_receipt_sha256": handshake_sha256,
        "termination_receipt_sha256": termination_sha256,
        "termination_disposition": "clean-exit",
        "child_reaped": True,
        "containment_empty": True,
        "diagnostic_stream_complete": True,
        "private_work_directory_removed": True,
        "publisher_authenticated": False,
        "durable_process_launch_authority": False,
        "ncp_authority": False,
        "physical_authority": False,
        "scientific_authority": False,
    }
    lifecycle["binding_sha256"] = digest_bytes(canonical(lifecycle))
    runtime_owner = "e" * 64
    neural_owner = "f" * 64
    tail_receipt_sha256 = "0" * 64
    worker_lifecycle_sha256 = "1" * 64
    runtime_cleanup: dict[str, Any] = {
        "schema_version": "engram.closed-loop-cleanup.v2",
        "component": "runtime",
        "owner_identity_sha256": runtime_owner,
        "mode": "finish",
        "attempted": True,
        "confirmed": True,
        "containment_empty": True,
        "reason_code": "loop.completed",
        "runtime_lifecycle": lifecycle,
        "provider_terminal_receipt_sha256": None,
        "provider_lifecycle_receipt_sha256": None,
    }
    runtime_cleanup["receipt_sha256"] = managed_runtime_digest(runtime_cleanup)
    neural_cleanup: dict[str, Any] = {
        "schema_version": "engram.closed-loop-cleanup.v2",
        "component": "neural",
        "owner_identity_sha256": neural_owner,
        "mode": "close",
        "attempted": True,
        "confirmed": True,
        "containment_empty": True,
        "reason_code": "loop.completed",
        "runtime_lifecycle": None,
        "provider_terminal_receipt_sha256": tail_receipt_sha256,
        "provider_lifecycle_receipt_sha256": worker_lifecycle_sha256,
    }
    neural_cleanup["receipt_sha256"] = managed_runtime_digest(neural_cleanup)
    capture["package_generation_id"] = package_generation_id
    capture["installed_package_proof"] = {
        "store_id": store_id,
        "installation_id": handshake["installation_id"],
        "executable_sha256": handshake["executable_sha256"],
    }
    closure = capture["engram_source_closure"]
    closure["reviewed_runtime_handshake_receipt_sha256"] = handshake_sha256
    closure["closure_sha256"] = digest_without(closure, "closure_sha256")
    capture["reviewed_native_runtime"] = {
        "handshake_receipt": handshake,
        "termination_receipt": termination,
        "exec_gate_command_binding": exec_gate_command_binding,
        "lifecycle_binding_sha256": lifecycle["binding_sha256"],
        "guardian_closure_verified": True,
        "package_store_lineage_verified": True,
    }
    capture["nest_evidence_bundle"].update(
        {
            "tail_disposition_receipt": {
                "receipt_sha256": tail_receipt_sha256,
            },
            "worker_lifecycle_receipt": {
                "receipt_sha256": worker_lifecycle_sha256,
            },
        }
    )
    capture["terminal_receipt"] = {
        "runtime_binding_sha256": runtime_owner,
        "neural_provider_identity_sha256": neural_owner,
        "runtime_lifecycle": lifecycle,
        "cleanup_complete": True,
        "cleanup": [runtime_cleanup, neural_cleanup],
    }
    assert source_sha256 == handshake["guardian_source_sha256"]
    return capture


def reseal_lifecycle_fixture(capture: dict[str, Any]) -> None:
    reviewed = capture["reviewed_native_runtime"]
    handshake = reviewed["handshake_receipt"]
    handshake["receipt_sha256"] = digest_without(handshake, "receipt_sha256")
    handshake_sha256 = handshake["receipt_sha256"]
    source_closure = capture["engram_source_closure"]
    source_closure["reviewed_runtime_handshake_receipt_sha256"] = handshake_sha256
    source_closure["closure_sha256"] = digest_without(
        source_closure,
        "closure_sha256",
    )
    termination = reviewed["termination_receipt"]
    termination.update(
        {
            "handshake_receipt_sha256": handshake_sha256,
            "generation_id": handshake["generation_id"],
            "guardian_pid": handshake["guardian_pid"],
            "process_group_id": handshake["process_group_id"],
        }
    )
    termination["receipt_sha256"] = digest_without(termination, "receipt_sha256")
    lifecycle = capture["terminal_receipt"]["runtime_lifecycle"]
    lifecycle.update(
        {
            "generation_id": handshake["generation_id"],
            "generation_directory_identity_sha256": handshake[
                "generation_directory_identity_sha256"
            ],
            "handshake_receipt_sha256": handshake_sha256,
            "termination_receipt_sha256": termination["receipt_sha256"],
        }
    )
    lifecycle["binding_sha256"] = digest_without(lifecycle, "binding_sha256")
    reviewed["lifecycle_binding_sha256"] = lifecycle["binding_sha256"]
    runtime_cleanup = capture["terminal_receipt"]["cleanup"][0]
    runtime_cleanup["runtime_lifecycle"] = lifecycle
    runtime_cleanup["receipt_sha256"] = managed_runtime_digest_without(
        runtime_cleanup,
        "receipt_sha256",
    )


def index_shape_fixture() -> dict[str, Any]:
    index = {field: None for field in INDEX_FIELDS}
    index.update(
        {
            "schema_version": INDEX_SCHEMA_VERSION,
            "input_suite": {
                "schema_version": None,
                "exact_sha256": None,
                "suite_definition_sha256": None,
                "nest_config_exact_sha256": None,
            },
            "tool_source_closure": {
                "schema_version": "crebain.real-nest-tool-source-closure.v1",
                "files": [
                    {"role": role, "path": path, "exact_sha256": None}
                    for path, role in sorted(EXPECTED_CREBAIN_TOOL_SOURCES.items())
                ],
                "roster_sha256": None,
            },
            "crebain_source_repository": {
                "repository": None,
                "commit": None,
                "tree": None,
                "origin_main_at_capture": None,
                "object_format": None,
                "clean_at_capture": None,
            },
            "engram": {
                "repository": None,
                "commit": None,
                "tree": None,
                "origin_main": None,
                "object_format": None,
                "clean": None,
            },
            "package": {field: None for field in INDEX_PACKAGE_FIELDS},
            "captures": [
                {field: None for field in INDEX_CAPTURE_FIELDS}
                for _drone_count in (1, 2, 3)
            ],
            "assertions": {field: True for field in INDEX_ASSERTION_FIELDS},
            "authority": {field: False for field in CAPTURE_AUTHORITY_FIELDS},
        }
    )
    index["authority"]["simulator_only"] = True
    return index


def installed_package_fixture(
    seed: str = "primary",
) -> tuple[
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
]:
    expected_crebain = {
        "root": Path("/fixture/crebain"),
        "repository": "https://github.com/sepahead/crebain.git",
        "commit": "a" * 40,
        "tree": "b" * 40,
        "origin_main": "a" * 40,
        "object_format": "sha1",
        "clean": True,
        "checkout_revision": "a" * 40,
    }

    def source_row(path: str) -> dict[str, Any]:
        payload = f"synthetic committed fixture {seed}: {path}\n".encode()
        return {
            "relative_path": path,
            "size_bytes": len(payload),
            "sha256": digest_bytes(payload),
            "git_mode": "100755" if path.endswith(".py") else "100644",
            "git_blob": hashlib.sha1(payload, usedforsecurity=False).hexdigest(),
        }

    source_paths = sorted(
        {
            "rust-toolchain.toml",
            "src-tauri/Cargo.lock",
            "src-tauri/Cargo.toml",
            "src-tauri/crates/managed-simulation/Cargo.toml",
            "src-tauri/crates/managed-simulation/src/lib.rs",
            "src-tauri/crates/managed-simulation/src/main.rs",
            "src-tauri/src/pid_observation.rs",
            "src-tauri/src/sensor_fusion.rs",
            *CREBAIN_BUILD_CONTRACT_PATHS,
        }
    )
    source_rows = [source_row(path) for path in source_paths]
    generator_rows = [source_row(path) for path in EXPECTED_CREBAIN_BUILD_GENERATORS]
    source = {
        "policy": "clean-origin-main-git-blob-and-rustc-dep-info-build-inputs.v1",
        "files": source_rows,
        "roster_sha256": digest_bytes(canonical(source_rows)),
    }
    generator = {
        "files": generator_rows,
        "roster_sha256": digest_bytes(canonical(generator_rows)),
    }
    source_by_path = {row["relative_path"]: row for row in source_rows}
    cargo = {
        "workspace_manifest_path": "src-tauri/Cargo.toml",
        "workspace_manifest_exact_sha256": source_by_path["src-tauri/Cargo.toml"][
            "sha256"
        ],
        "package_manifest_path": "src-tauri/crates/managed-simulation/Cargo.toml",
        "package_manifest_exact_sha256": source_by_path[
            "src-tauri/crates/managed-simulation/Cargo.toml"
        ]["sha256"],
        "lock_path": "src-tauri/Cargo.lock",
        "lock_exact_sha256": source_by_path["src-tauri/Cargo.lock"]["sha256"],
        "toolchain_path": "rust-toolchain.toml",
        "toolchain_exact_sha256": source_by_path["rust-toolchain.toml"]["sha256"],
        "rust_toolchain": "1.91.1",
        "rustc_version": "rustc 1.91.1 (ed61e7d7e 2025-11-07)",
        "cargo_version": "cargo 1.91.1 (ea2d97820 2025-10-10)",
        "argv": EXPECTED_CREBAIN_CARGO_ARGV,
        "profile": "release",
        "target": EXPECTED_CREBAIN_TARGET,
        "target_directory_policy": "fresh-fixed-owner-private-removed-after-copy.v1",
        "environment_policy": (
            "reject-build-override-environment-and-record-output-bytes.v1"
        ),
    }
    repository = {
        "origin": expected_crebain["repository"],
        "commit": expected_crebain["commit"],
        "tree": expected_crebain["tree"],
        "origin_main": expected_crebain["origin_main"],
        "object_format": expected_crebain["object_format"],
        "clean": True,
    }
    input_identity = {
        "repository": repository,
        "source_roster_sha256": source["roster_sha256"],
        "generator_roster_sha256": generator["roster_sha256"],
        "cargo": cargo,
    }
    executable_sha256 = digest_bytes(f"synthetic Mach-O executable: {seed}".encode())
    build: dict[str, Any] = {
        "schema_version": "crebain.managed-simulation-observed-build-receipt.v1",
        "repository": repository,
        "source": source,
        "generator": generator,
        "cargo": cargo,
        "output": {
            "file_name": "crebain-managed-simulation",
            "byte_length": 4096,
            "sha256": executable_sha256,
            "source_mode": 0o755,
            "format": "mach-o-64",
            "architecture": "arm64",
            "file_type": "executable",
        },
        "input_identity_sha256": digest_bytes(canonical(input_identity)),
        "claims": {
            "observed_local_build": True,
            "reproducible_build": False,
            "signature": False,
            "external_dependency_bytes_attested": False,
            "complete_environment_attested": False,
        },
        "authority": CREBAIN_NO_AUTHORITY,
        "disclosure": "Synthetic observed build for provider-free tests.",
    }
    build["receipt_sha256"] = digest_bytes(canonical(build))
    build_bytes = canonical(build) + b"\n"
    source_executable = {
        "byte_length": build["output"]["byte_length"],
        "sha256": build["output"]["sha256"],
        "mode": build["output"]["source_mode"],
        "format": "mach-o-64",
        "architecture": "arm64",
        "file_type": "executable",
    }
    staged_executable = {**source_executable, "mode": 0o700}
    inventory = [
        {
            "relative_path": "bin/crebain-managed-simulation",
            "byte_length": staged_executable["byte_length"],
            "sha256": staged_executable["sha256"],
            "mode": 0o700,
            "role": "executable",
        },
        {
            "relative_path": "contracts/configuration.schema.json",
            "byte_length": 128,
            "sha256": digest_bytes(b"synthetic staged contract"),
            "mode": 0o600,
            "role": "contract",
        },
    ]
    stage: dict[str, Any] = {
        "schema_version": "crebain.managed-simulation-package-stage-receipt.v1",
        "observed_build_receipt_exact_sha256": digest_bytes(build_bytes),
        "observed_build_receipt_sha256": build["receipt_sha256"],
        "crebain_commit": expected_crebain["commit"],
        "crebain_tree": expected_crebain["tree"],
        "origin_main": expected_crebain["origin_main"],
        "target": EXPECTED_CREBAIN_TARGET,
        "recipe_exact_sha256": digest_bytes(b"synthetic package recipe"),
        "configuration_exact_sha256": digest_bytes(b"synthetic configuration"),
        "source_executable": source_executable,
        "staged_executable": staged_executable,
        "package_inventory": inventory,
        "package_inventory_sha256": digest_bytes(canonical(inventory)),
        "authority": CREBAIN_NO_AUTHORITY,
        "disclosure": "Synthetic package stage for provider-free tests.",
    }
    stage["receipt_sha256"] = digest_bytes(canonical(stage))
    stage_bytes = canonical(stage) + b"\n"
    expected_engram = {
        "root": Path("/fixture/engram"),
        "repository": "https://github.com/sepahead/engram.git",
        "commit": "c" * 40,
        "tree": "d" * 40,
        "origin_main": "c" * 40,
        "object_format": "sha1",
        "clean": True,
    }
    pack_tool_payload = f"synthetic Engram pack tool: {seed}\n".encode()
    pack_tool = {
        "relative_path": EXPECTED_ENGRAM_PACK_TOOL_PATH,
        "size_bytes": len(pack_tool_payload),
        "sha256": digest_bytes(pack_tool_payload),
        "git_mode": "100755",
        "git_blob": hashlib.sha1(
            pack_tool_payload,
            usedforsecurity=False,
        ).hexdigest(),
    }
    package_generation_id = "pkggen_" + digest_bytes(f"{seed}: generation".encode())
    bundle_receipt_exact_sha256 = digest_bytes(f"{seed}: bundle receipt".encode())
    seal_receipt_exact_sha256 = digest_bytes(f"{seed}: seal receipt".encode())
    pack: dict[str, Any] = {
        "schema_version": "crebain.managed-simulation-engram-pack-receipt.v1",
        "engram_repository": {
            "origin": expected_engram["repository"],
            "commit": expected_engram["commit"],
            "tree": expected_engram["tree"],
            "origin_main": expected_engram["origin_main"],
            "object_format": expected_engram["object_format"],
            "clean": True,
        },
        "engram_tool": pack_tool,
        "verification_policy": (
            "clean-head-origin-main-committed-tool-before-and-after-each-operation.v1"
        ),
        "operations": copy.deepcopy(ENGRAM_PACK_OPERATIONS),
        "observed_build_receipt_exact_sha256": digest_bytes(build_bytes),
        "observed_build_receipt_sha256": build["receipt_sha256"],
        "package_stage_receipt_exact_sha256": digest_bytes(stage_bytes),
        "package_stage_receipt_sha256": stage["receipt_sha256"],
        "seal_receipt_exact_sha256": seal_receipt_exact_sha256,
        "bundle_receipt_exact_sha256": bundle_receipt_exact_sha256,
        "package_generation_id": package_generation_id,
        "claims": copy.deepcopy(ENGRAM_PACK_CLAIMS),
        "authority": copy.deepcopy(CREBAIN_NO_AUTHORITY),
        "disclosure": "Synthetic Engram pack observation for provider-free tests.",
    }
    pack["receipt_sha256"] = digest_bytes(canonical(pack))
    pack_bytes = canonical(pack) + b"\n"
    proof: dict[str, Any] = {
        "schema_version": "crebain.standard-v3-installed-binary-proof.v3",
        "observed_build_receipt_exact_sha256": digest_bytes(build_bytes),
        "observed_build_receipt_sha256": build["receipt_sha256"],
        "observed_build_receipt": build,
        "package_stage_receipt_exact_sha256": digest_bytes(stage_bytes),
        "package_stage_receipt_sha256": stage["receipt_sha256"],
        "package_stage_receipt": stage,
        "engram_pack_receipt_exact_sha256": digest_bytes(pack_bytes),
        "engram_pack_receipt_sha256": pack["receipt_sha256"],
        "engram_pack_receipt": pack,
        "crebain_commit": expected_crebain["commit"],
        "crebain_tree": expected_crebain["tree"],
        "crebain_origin_main": expected_crebain["origin_main"],
        "engram_commit": expected_engram["commit"],
        "engram_tree": expected_engram["tree"],
        "engram_origin_main": expected_engram["origin_main"],
        "engram_extension_tool_sha256": pack_tool["sha256"],
        "engram_extension_tool_git_blob": pack_tool["git_blob"],
        "build_source_roster_sha256": source["roster_sha256"],
        "build_input_identity_sha256": build["input_identity_sha256"],
        "executable_format": "mach-o-64",
        "executable_architecture": "arm64",
        "store_id": "extstore_" + digest_bytes(f"{seed}: store".encode()),
        "package_generation_id": package_generation_id,
        "installation_id": "inst_" + digest_bytes(f"{seed}: installation".encode()),
        "generation_core_sha256": digest_bytes(f"{seed}: generation core".encode()),
        "bundle_receipt_exact_sha256": bundle_receipt_exact_sha256,
        "seal_receipt_exact_sha256": seal_receipt_exact_sha256,
        "install_observation_exact_sha256": digest_bytes(
            f"{seed}: install observation".encode()
        ),
        "manifest_exact_sha256": digest_bytes(f"{seed}: manifest".encode()),
        "package_lock_exact_sha256": digest_bytes(f"{seed}: package lock".encode()),
        "configuration_exact_sha256": stage["configuration_exact_sha256"],
        "package_sha256": digest_bytes(f"{seed}: package".encode()),
        "executable_sha256": executable_sha256,
        "configuration_canonical_sha256": digest_bytes(
            f"{seed}: configuration".encode()
        ),
        "operation_roster_sha256": digest_bytes(f"{seed}: operations".encode()),
        "operation_ids": EXPECTED_CREBAIN_OPERATION_IDS,
        "standard_schema_sha256": {
            schema_id: "f" * 64 for schema_id in sorted(EXPECTED_STANDARD_SCHEMA_IDS)
        },
        "drone_counts": [1, 2, 3],
        "step_count": 6,
        "fault_step": 3,
        "fault": "sensor-unavailable",
        "host_policy": [
            "fault-observed",
            "safe-hold",
            "bounded-zero-washout",
            "bounded-nonzero-resume",
        ],
        "recovery_controls_sha256": {
            "1": "0" * 64,
            "2": "1" * 64,
            "3": "2" * 64,
        },
        "baseline_three_controls_sha256": digest_bytes(f"{seed}: baseline".encode()),
        "replay_exact": True,
        "unaffected_lane_observations_exact": True,
        "negative_clock_gate": "standard.clock-mismatch",
        "signal_cancellation_gate": ("active-SIGTERM-then-fresh-generation-prepared"),
        "installed_artifacts_reverified_after_execution": True,
        "generation_seal_package_bundle_store_lineage_verified": True,
        "build_stage_seal_install_lineage_verified": True,
        "build_stage_seal_pack_install_lineage_verified": True,
        "authority": {
            "simulator_only": True,
            "ncp_qualified": False,
            "physical_actuation": False,
            "plant_control": False,
            "scientific_authority": False,
        },
        "disclosure": "Fixture for closed installed-package lineage review.",
    }
    proof["receipt_sha256"] = digest_bytes(canonical(proof))
    exact_sha256 = digest_bytes(canonical(proof) + b"\n")
    capture = {
        "package_generation_id": proof["package_generation_id"],
        "installed_package_proof_exact_sha256": exact_sha256,
        "installed_package_proof": proof,
    }
    index = {
        "package": {field: proof[field] for field in INDEX_PACKAGE_FIELDS},
        "installed_package_proof_exact_sha256": exact_sha256,
    }
    generator_by_path = {row["relative_path"]: row for row in generator_rows}
    tool_rows = [
        {
            "role": role,
            "path": path,
            "exact_sha256": (
                generator_by_path[path]["sha256"]
                if path in generator_by_path
                else digest_bytes(f"synthetic tool: {path}".encode())
            ),
        }
        for path, role in sorted(EXPECTED_CREBAIN_TOOL_SOURCES.items())
    ]
    tool_sources = {
        "document": {
            "schema_version": "crebain.real-nest-tool-source-closure.v1",
            "files": tool_rows,
            "roster_sha256": digest_bytes(canonical(tool_rows)),
        },
        "files": tool_rows,
        "committed_sources": [],
        "roster_sha256": digest_bytes(canonical(tool_rows)),
    }
    return capture, index, expected_crebain, expected_engram, tool_sources


def reseal_installed_package_fixture(
    capture: dict[str, Any],
    index: dict[str, Any],
) -> None:
    """Reseal a hostile synthetic fixture without repairing its semantics."""

    proof = capture["installed_package_proof"]
    build = proof["observed_build_receipt"]
    build["source"]["roster_sha256"] = digest_bytes(canonical(build["source"]["files"]))
    build["generator"]["roster_sha256"] = digest_bytes(
        canonical(build["generator"]["files"])
    )
    input_identity = {
        "repository": build["repository"],
        "source_roster_sha256": build["source"]["roster_sha256"],
        "generator_roster_sha256": build["generator"]["roster_sha256"],
        "cargo": build["cargo"],
    }
    build["input_identity_sha256"] = digest_bytes(canonical(input_identity))
    build["receipt_sha256"] = digest_without(build, "receipt_sha256")
    build_bytes = canonical(build) + b"\n"
    stage = proof["package_stage_receipt"]
    stage["observed_build_receipt_exact_sha256"] = digest_bytes(build_bytes)
    stage["observed_build_receipt_sha256"] = build["receipt_sha256"]
    stage["package_inventory_sha256"] = digest_bytes(
        canonical(stage["package_inventory"])
    )
    stage["receipt_sha256"] = digest_without(stage, "receipt_sha256")
    stage_bytes = canonical(stage) + b"\n"
    pack = proof["engram_pack_receipt"]
    pack["observed_build_receipt_exact_sha256"] = digest_bytes(build_bytes)
    pack["observed_build_receipt_sha256"] = build["receipt_sha256"]
    pack["package_stage_receipt_exact_sha256"] = digest_bytes(stage_bytes)
    pack["package_stage_receipt_sha256"] = stage["receipt_sha256"]
    pack["receipt_sha256"] = digest_without(pack, "receipt_sha256")
    pack_bytes = canonical(pack) + b"\n"
    pack_repository = pack["engram_repository"]
    pack_tool = pack["engram_tool"]
    proof["observed_build_receipt_exact_sha256"] = digest_bytes(build_bytes)
    proof["observed_build_receipt_sha256"] = build["receipt_sha256"]
    proof["package_stage_receipt_exact_sha256"] = digest_bytes(stage_bytes)
    proof["package_stage_receipt_sha256"] = stage["receipt_sha256"]
    proof["engram_pack_receipt_exact_sha256"] = digest_bytes(pack_bytes)
    proof["engram_pack_receipt_sha256"] = pack["receipt_sha256"]
    proof["engram_commit"] = pack_repository["commit"]
    proof["engram_tree"] = pack_repository["tree"]
    proof["engram_origin_main"] = pack_repository["origin_main"]
    proof["engram_extension_tool_sha256"] = pack_tool["sha256"]
    proof["engram_extension_tool_git_blob"] = pack_tool["git_blob"]
    proof["build_source_roster_sha256"] = build["source"]["roster_sha256"]
    proof["build_input_identity_sha256"] = build["input_identity_sha256"]
    reseal_installed_outer_fixture(capture, index)


def reseal_installed_outer_fixture(
    capture: dict[str, Any],
    index: dict[str, Any],
) -> None:
    """Reseal only an installed proof and its projections."""

    proof = capture["installed_package_proof"]
    proof["receipt_sha256"] = digest_without(proof, "receipt_sha256")
    exact_sha256 = digest_bytes(canonical(proof) + b"\n")
    capture["installed_package_proof_exact_sha256"] = exact_sha256
    capture["package_generation_id"] = proof["package_generation_id"]
    index["installed_package_proof_exact_sha256"] = exact_sha256
    index["package"] = {field: proof[field] for field in INDEX_PACKAGE_FIELDS}


def receipt_store_fixture() -> tuple[dict[str, Any], dict[str, Any]]:
    store_id = "clrs_" + "5" * 64
    reservation_id = "clrr_" + "6" * 64
    study_run_id = "receipt-store-fixture"
    package_generation_id = "pkggen_" + "7" * 64
    runtime_generation_id = "gen_" + "8" * 64
    run_plan = {
        "schema_version": "crebain.test-run-plan.v1",
        "portable_threshold": 1.0e-6,
    }
    nest_config = {"population_size": 8, "resolution_ms": 1.0e-6}
    nest_configuration_sha256 = managed_runtime_digest(nest_config)
    work_admission = {
        "controller_configuration_sha256": nest_configuration_sha256,
        "estimated_evidence_bundle_bytes": 4096,
        "portable_threshold": 1.0e-6,
    }
    work_admission["receipt_sha256"] = digest_bytes(canonical(work_admission))
    terminal = {field: None for field in TERMINAL_FIELDS}
    terminal.update(
        {
            "schema_version": "engram.extension-closed-loop-run-receipt.v2",
            "study_run_id": study_run_id,
            "closed_loop_definition_sha256": "1" * 64,
            "runtime_binding_sha256": "2" * 64,
            "runtime_lifecycle": {"generation_id": runtime_generation_id},
            "timebase": {"portable_threshold": 1.0e-6},
            "planned_step_count": 0,
            "steps": [],
            "status": "completed",
            "terminal_reason_code": "loop.completed",
        }
    )
    terminal["receipt_sha256"] = managed_runtime_digest_without(
        terminal,
        "receipt_sha256",
    )
    evidence = {field: None for field in NEST_BUNDLE_FIELDS}
    evidence.update(
        {
            "schema_version": "engram.nest-closed-loop-evidence-bundle.v2",
            "study_run_id": study_run_id,
            "run_receipt_sha256": terminal["receipt_sha256"],
            "nest_session_readback": {"work_admission": work_admission},
        }
    )
    evidence["bundle_sha256"] = managed_runtime_digest_without(
        evidence,
        "bundle_sha256",
    )
    receipt_sha256 = terminal["receipt_sha256"]
    evidence_sha256 = evidence["bundle_sha256"]
    receipt_payload = managed_runtime_canonical(
        {key: value for key, value in terminal.items() if key != "receipt_sha256"}
    )
    evidence_payload = managed_runtime_canonical(
        {key: value for key, value in evidence.items() if key != "bundle_sha256"}
    )
    sidecars, files = receipt_store_sidecar_fixture(
        store_id=store_id,
        reservation_id=reservation_id,
        study_run_id=study_run_id,
        terminal_receipt_sha256=receipt_sha256,
        terminal_artifact_size_bytes=len(receipt_payload),
        evidence_bundle_sha256=evidence_sha256,
        evidence_artifact_size_bytes=len(evidence_payload),
        package_generation_id=package_generation_id,
        runtime_generation_id=runtime_generation_id,
        closed_loop_definition_sha256=terminal["closed_loop_definition_sha256"],
        runtime_binding_sha256=terminal["runtime_binding_sha256"],
        run_plan_sha256=managed_runtime_digest(run_plan),
        nest_configuration_sha256=nest_configuration_sha256,
        nest_work_admission_sha256=work_admission["receipt_sha256"],
        reserved_evidence_bytes=work_admission["estimated_evidence_bundle_bytes"],
    )
    receipt_path = f"receipts/{receipt_sha256[:2]}/{receipt_sha256}.json"
    evidence_path = f"evidence/{evidence_sha256[:2]}/{evidence_sha256}.json"
    closure: dict[str, Any] = {
        "schema_version": "crebain.closed-loop-receipt-store-closure.v1",
        "store_id": store_id,
        "receipt_sha256": receipt_sha256,
        "receipt_artifact_path": receipt_path,
        "evidence_bundle_sha256": evidence_sha256,
        "evidence_artifact_path": evidence_path,
        "file_count": len(files),
        "total_bytes": sum(row["size_bytes"] for row in files),
        "files": files,
    }
    closure["closure_sha256"] = digest_bytes(canonical(closure))
    capture = {
        "terminal_receipt": terminal,
        "nest_evidence_bundle": evidence,
        "receipt_store_closure": closure,
        "receipt_store_sidecars": sidecars,
        "run_plan": run_plan,
        "nest_config": nest_config,
        "package_generation_id": package_generation_id,
        "reviewed_native_runtime": {
            "handshake_receipt": sidecars["finalized_reservation"]["reservation"][
                "reviewed_native_handshake"
            ]
        },
        "summary": {
            "authority": False,
            "calibrated_posterior": False,
            "channel_count": 1,
            "completed_step_count": 0,
            "evidence_bundle_sha256": evidence_sha256,
            "ncp_qualified": False,
            "physical_actuation": False,
            "planned_step_count": 0,
            "receipt_sha256": receipt_sha256,
            "reservation_id": reservation_id,
            "run_status": "completed",
            "scientific_authority": False,
            "simulator_only": True,
            "status": "recorded",
            "store_id": store_id,
            "study_run_id": study_run_id,
            "terminal_reason_code": "loop.completed",
        },
    }
    index_row = {
        "receipt_store_id": closure["store_id"],
        "receipt_sha256": receipt_sha256,
        "evidence_bundle_sha256": evidence_sha256,
        "receipt_store_closure_sha256": closure["closure_sha256"],
    }
    return capture, index_row


def reseal_receipt_store_fixture(
    capture: dict[str, Any],
    index_row: dict[str, Any],
) -> None:
    """Reseal one hostile receipt-store fixture after a controlled mutation."""

    closure = capture["receipt_store_closure"]
    closure["files"].sort(key=lambda row: row["relative_path"])
    closure["file_count"] = len(closure["files"])
    closure["total_bytes"] = sum(row["size_bytes"] for row in closure["files"])
    closure["closure_sha256"] = digest_without(closure, "closure_sha256")
    index_row.update(
        {
            "receipt_store_id": closure["store_id"],
            "receipt_sha256": closure["receipt_sha256"],
            "evidence_bundle_sha256": closure["evidence_bundle_sha256"],
            "receipt_store_closure_sha256": closure["closure_sha256"],
        }
    )


def population_topology_fixture() -> tuple[
    dict[str, Any], dict[str, Any], dict[str, Any]
]:
    roster = {
        "channel_ids": ["channel-01", "channel-02"],
        "population_prefixes": ["fleet.channel-01", "fleet.channel-02"],
    }
    names = expected_population_names(roster)
    population_roster = expected_population_roster(roster)
    control_bindings = [
        {
            "channel_id": channel_id,
            "neural_codec_sha256": format(channel_ordinal + 1, "x") * 64,
            "axis_binding_sha256s": [
                format(channel_ordinal * 3 + axis_index + 3, "x") * 64
                for axis_index in range(3)
            ],
        }
        for channel_ordinal, channel_id in enumerate(roster["channel_ids"])
    ]
    connection_readbacks = [
        {
            "population_name": population_name,
            "direction": direction,
            "connection_count": 8,
            "synapse_model": "static_synapse",
            "requested_weight": 5.0 if direction == "input" else 1.0,
            "effective_weight": 5.0 if direction == "input" else 1.0,
            "requested_delay_tics": 100,
            "delay_api_argument_ms": 0.1,
            "effective_delay_ms": 0.1,
            "effective_delay_tics": 100,
            "requested_receptor": 0,
            "effective_receptor": 0,
        }
        for population_name in names
        for direction in ("input", "recorder")
    ]
    encoded_controls = [
        {
            "channel_id": channel_id,
            "action_index": action_index,
            "neural_codec_sha256": binding["neural_codec_sha256"],
            "axis_binding_sha256": binding["axis_binding_sha256s"][action_index],
        }
        for channel_id, binding in zip(
            roster["channel_ids"],
            control_bindings,
            strict=True,
        )
        for action_index in range(3)
    ]
    named_rows = [{"population_name": name} for name in names]
    safety_rows = [{"channel_id": channel_id} for channel_id in roster["channel_ids"]]
    execution: dict[str, Any] = {
        "generator_schedule_readbacks": copy.deepcopy(named_rows),
        "generator_schedule_readback_sha256": digest_bytes(canonical(named_rows)),
        "input_weight_readbacks": copy.deepcopy(named_rows),
        "input_weight_readback_sha256": digest_bytes(canonical(named_rows)),
        "completed_window_readbacks": copy.deepcopy(named_rows),
        "completed_window_readback_sha256": digest_bytes(canonical(named_rows)),
        "population_event_deltas": copy.deepcopy(named_rows),
        "channel_safety_readbacks": safety_rows,
        "channel_safety_readback_sha256": digest_bytes(canonical(safety_rows)),
        "encoded_control_inputs": encoded_controls,
        "control_encoding_sha256": digest_bytes(canonical(encoded_controls)),
    }
    execution["receipt_sha256"] = digest_bytes(canonical(execution))
    session = {
        "population_roster": population_roster,
        "population_roster_sha256": digest_bytes(canonical(population_roster)),
        "control_bindings": control_bindings,
        "control_binding_sha256": digest_bytes(canonical(control_bindings)),
        "connection_readbacks": connection_readbacks,
        "connection_readback_sha256": digest_bytes(canonical(connection_readbacks)),
        "work_admission": {
            "expected_population_roster_sha256": digest_bytes(
                canonical(population_roster)
            ),
            "expected_control_binding_sha256": digest_bytes(
                canonical(control_bindings)
            ),
        },
    }
    neural_step = {
        "request": {
            "channels": [
                {"channel_id": channel_id} for channel_id in roster["channel_ids"]
            ],
        },
        "result": {
            "proposals": [
                {
                    "channel_id": row["channel_id"],
                    "source_populations": row["population_names"],
                }
                for row in population_roster
            ],
        },
    }
    topology = {
        "session_count": 1,
        "drone_count": 2,
        "action_axis_count": 6,
        "population_count": 12,
        "population_neuron_count": 96,
        "device_node_count": 24,
        "connection_count": 192,
        "population_names": names,
        "derived_population_roster_sha256": digest_bytes(canonical(names)),
    }
    capture = {
        "nest_config": {"population_size": 8},
        "population_topology": topology,
        "nest_evidence_bundle": {
            "nest_session_readback": session,
            "step_execution_receipts": [execution],
        },
        "neural_steps": [neural_step],
    }
    index_row = {
        "drone_count": 2,
        "session_count": 1,
        "population_count": 12,
        "population_neuron_count": 96,
        "device_node_count": 24,
        "connection_count": 192,
    }
    return capture, index_row, roster


def nest_v2_execution_lineage_fixture() -> tuple[dict[str, Any], dict[str, Any]]:
    """Build one exact, synthetic V2 execution-lineage positive control."""

    roster = {
        "channel_ids": ["channel-01"],
        "population_prefixes": ["fleet.channel-01"],
    }
    populations = expected_population_roster(roster)
    project_root = "/fixture/engram-source"
    source_specs = [
        (
            "backend/integrations/contained_exec_gate.py",
            "backend.integrations.contained_exec_gate",
            b"exec-gate\n",
        ),
        (
            "backend/optimization/extension_closed_loop_nest_process.py",
            "backend.optimization.extension_closed_loop_nest_process",
            b"adapter\n",
        ),
    ]
    guardian_relative = "backend/optimization/extension_closed_loop_nest_guardian.py"
    guardian_bytes = b"guardian\n"
    worker_relative = "backend/optimization/extension_closed_loop_nest_worker.py"
    worker_bytes = b"worker\n"
    sources = [
        {
            "relative_path": relative,
            "size_bytes": len(payload),
            "sha256": digest_bytes(payload),
        }
        for relative, _module, payload in source_specs
    ]
    sources.extend(
        (
            {
                "relative_path": guardian_relative,
                "size_bytes": len(guardian_bytes),
                "sha256": digest_bytes(guardian_bytes),
            },
            {
                "relative_path": worker_relative,
                "size_bytes": len(worker_bytes),
                "sha256": digest_bytes(worker_bytes),
            },
        )
    )
    sources.sort(key=lambda row: row["relative_path"])
    project_files = [
        {
            "role": f"project-module:{module}",
            "absolute_path": f"{project_root}/{relative}",
            "sha256": digest_bytes(payload),
            "size_bytes": len(payload),
        }
        for relative, module, payload in source_specs
    ]
    project_files.sort(key=lambda row: row["role"])
    python_file = {
        "role": "python-executable",
        "absolute_path": "/fixture/python/bin/python3",
        "sha256": "1" * 64,
        "size_bytes": 1024,
    }
    worker_file = {
        "role": "worker-source",
        "absolute_path": f"{project_root}/{worker_relative}",
        "sha256": digest_bytes(worker_bytes),
        "size_bytes": len(worker_bytes),
    }
    runtime_files = [
        *project_files,
        {
            "role": "pydantic-core-native",
            "absolute_path": "/fixture/python/site-packages/pydantic_core.so",
            "sha256": "2" * 64,
            "size_bytes": 2048,
        },
        {
            "role": "pydantic-package-init",
            "absolute_path": "/fixture/python/site-packages/pydantic/__init__.py",
            "sha256": "3" * 64,
            "size_bytes": 512,
        },
        python_file,
        worker_file,
    ]
    project_roster_sha256 = digest_bytes(canonical(project_files))
    runtime_roster_sha256 = digest_bytes(canonical(runtime_files))
    guardian_command = [
        python_file["absolute_path"],
        "-I",
        "-S",
        "-B",
        "-c",
        guardian_bytes.decode("utf-8"),
    ]
    worker_command = [
        python_file["absolute_path"],
        "-I",
        "-S",
        "-B",
        worker_file["absolute_path"],
        "--resource-limit-profile",
        "portable-posix-rlimit-v1",
        "--address-space-bytes",
        "0",
        "--cpu-time-seconds",
        "300",
        "--file-size-bytes",
        "67108864",
        "--open-file-count",
        "256",
    ]
    sandbox_profile = "(version 1)(allow default)(deny process-fork)"
    dispatch_command = [
        "/usr/bin/sandbox-exec",
        "-p",
        sandbox_profile,
        *worker_command,
    ]
    exec_source = next(
        row
        for row in sources
        if row["relative_path"] == "backend/integrations/contained_exec_gate.py"
    )
    adapter_source = next(
        row
        for row in sources
        if row["relative_path"]
        == "backend/optimization/extension_closed_loop_nest_process.py"
    )
    launch: dict[str, Any] = {
        "adapter_source_sha256": adapter_source["sha256"],
        "address_space_bytes": None,
        "address_space_limit_enforced": False,
        "child_provider_test_failure_phase": "none",
        "controller_configuration": {"population_size": 8},
        "core_file_bytes": 0,
        "cpu_time_seconds": 300,
        "darwin_sandbox_launcher_sha256": "4" * 64,
        "darwin_sandbox_profile_sha256": digest_bytes(sandbox_profile.encode("utf-8")),
        "descendant_creation_denied": True,
        "environment": [
            ["LANG", "C"],
            ["LC_ALL", "C"],
            ["PATH", "/usr/bin:/bin"],
            ["TZ", "UTC"],
        ],
        "exec_gate_source_file": {
            "role": "exec-gate-source",
            "absolute_path": (
                f"{project_root}/backend/integrations/contained_exec_gate.py"
            ),
            "sha256": exec_source["sha256"],
            "size_bytes": exec_source["size_bytes"],
        },
        "exec_gate_source_sha256": exec_source["sha256"],
        "expected_child_provider_identity_sha256": "5" * 64,
        "external_dependency_closure_attested": False,
        "file_size_bytes": 67_108_864,
        "guardian_command": guardian_command,
        "guardian_group_member": True,
        "guardian_source_file": {
            "role": "guardian-source",
            "absolute_path": f"{project_root}/{guardian_relative}",
            "sha256": digest_bytes(guardian_bytes),
            "size_bytes": len(guardian_bytes),
        },
        "guardian_source_sha256": digest_bytes(guardian_bytes),
        "loaded_bytes_attested": False,
        "network_namespace_isolation": False,
        "open_file_count": 256,
        "platform": "darwin",
        "production_isolation": False,
        "project_source_discovery_policy": ("minimum-direct-worker-import-roster-v1"),
        "python_executable_sha256": python_file["sha256"],
        "required_project_source_roster_sha256": project_roster_sha256,
        "required_runtime_file_roster_sha256": runtime_roster_sha256,
        "required_runtime_files": runtime_files,
        "resource_limit_profile": "portable-posix-rlimit-v1",
        "runtime_process_group_leader": True,
        "schema_version": "engram.nest-worker-launch-expectation.v4",
        "session_escape_prevention_profile": ("darwin-gated-group-leader-deny-fork-v1"),
        "sys_path": [project_root, "/fixture/python/site-packages"],
        "syscall_filter": False,
        "worker_command": worker_command,
        "worker_dispatch_command": dispatch_command,
        "worker_source_sha256": worker_file["sha256"],
    }
    launch["worker_command_sha256"] = digest_bytes(
        canonical(
            {
                "guardian_command": guardian_command,
                "worker_command": worker_command,
                "worker_dispatch_command": dispatch_command,
                "exec_gate_source_sha256": launch["exec_gate_source_sha256"],
                "session_escape_prevention_profile": launch[
                    "session_escape_prevention_profile"
                ],
                "darwin_sandbox_profile_sha256": launch[
                    "darwin_sandbox_profile_sha256"
                ],
                "darwin_sandbox_launcher_sha256": launch[
                    "darwin_sandbox_launcher_sha256"
                ],
            }
        )
    )
    launch["receipt_sha256"] = digest_bytes(canonical(launch))
    launch_attempt = {field: None for field in WORKER_LAUNCH_ATTEMPT_FIELDS}
    launch_attempt.update(
        {
            "anchored_group_kill_delivered": False,
            "bounded_cleanup_observation_complete": False,
            "containment_empty": False,
            "containment_seal_signal": None,
            "group_signal_attempted": False,
            "group_signal_basis": "none",
            "guardian_pid": 124,
            "guardian_ready_observed": True,
            "guardian_reaped": False,
            "guardian_started": True,
            "launch_expectation_sha256": launch["receipt_sha256"],
            "outcome": "succeeded",
            "phase": "worker-ready",
            "posix_process_group_portability_scope": (
                "darwin-linux-reviewed-local-development"
            ),
            "process_group_id": 123,
            "production_isolation": False,
            "reason_code": "neural.nest-worker-launch-succeeded",
            "schema_version": "engram.nest-worker-launch-attempt.v1",
            "scientific_authority": False,
            "session_id": 122,
            "stderr_drain_started": True,
            "worker_pid": 123,
            "worker_reaped": False,
            "worker_started": True,
        }
    )
    launch_attempt["receipt_sha256"] = digest_without(launch_attempt, "receipt_sha256")
    capabilities = {
        "automatic_restart": False,
        "deadline_enforcement": "cooperative-observed",
        "declared_step_duration_tics": 10_000,
        "durable_evidence_profile": "none",
        "loaded_bytes_attested": False,
        "max_channels": 64,
        "ncp_transport": False,
        "physical_actuation": False,
        "provider": "engram.nest-population-controller",
        "provider_identity_sha256": launch["expected_child_provider_identity_sha256"],
        "schema_version": "engram.closed-loop-neural-capabilities.v1",
        "session_model": "one-session-named-populations",
    }
    session: dict[str, Any] = {"fixture": "child-session"}
    session["receipt_sha256"] = digest_bytes(canonical(session))
    identity: dict[str, Any] = {
        "files": runtime_files,
        "file_roster_sha256": runtime_roster_sha256,
        "project_source_closure_verified": True,
        "project_source_roster_sha256": project_roster_sha256,
    }
    identity["receipt_sha256"] = digest_bytes(canonical(identity))
    definition_sha256 = "6" * 64
    parent_provider_sha256 = "7" * 64
    study_run_id = "v2-lineage-fixture"
    common_prepared = {
        "schema_version": "engram.closed-loop-neural-prepared.v1",
        "definition_sha256": definition_sha256,
        "populations": populations,
        "single_session": True,
        "step_duration_tics": 10_000,
        "study_run_id": study_run_id,
    }
    child_prepared = {
        **common_prepared,
        "provider_identity_sha256": capabilities["provider_identity_sha256"],
        "provider_session_receipt_sha256": session["receipt_sha256"],
    }
    child_prepared["receipt_sha256"] = managed_runtime_digest(child_prepared)
    binding: dict[str, Any] = {
        "runtime_launch_expectation_sha256": launch["receipt_sha256"],
        "worker_launch_attempt_sha256": launch_attempt["receipt_sha256"],
        "child_capabilities_sha256": digest_bytes(canonical(capabilities)),
        "child_prepared_receipt_sha256": child_prepared["receipt_sha256"],
        "child_provider_identity_sha256": capabilities["provider_identity_sha256"],
        "child_session_receipt_sha256": session["receipt_sha256"],
        "worker_runtime_identity_sha256": identity["receipt_sha256"],
        "worker_command_sha256": launch["worker_command_sha256"],
        "worker_source_sha256": launch["worker_source_sha256"],
        "guardian_source_sha256": launch["guardian_source_sha256"],
        "adapter_source_sha256": launch["adapter_source_sha256"],
        "worker_project_source_roster_sha256": project_roster_sha256,
        "study_run_id": study_run_id,
        "parent_provider_identity_sha256": parent_provider_sha256,
        "child_lineage_verified": True,
        "loaded_bytes_attested": False,
        "ncp_transport": False,
        "response_bound_loaded_bytes": False,
        "scientific_authority": False,
    }
    binding["receipt_sha256"] = digest_bytes(canonical(binding))
    provider_prepared = {
        **common_prepared,
        "provider_identity_sha256": parent_provider_sha256,
        "provider_session_receipt_sha256": binding["receipt_sha256"],
    }
    provider_prepared["receipt_sha256"] = managed_runtime_digest(provider_prepared)
    preparation_attempt = {field: None for field in PREPARATION_ATTEMPT_FIELDS}
    preparation_attempt.update(
        {
            "definition_sha256": definition_sha256,
            "outcome": "succeeded",
            "phase": "provider-prepare",
            "provider_preparation_receipt_sha256": provider_prepared["receipt_sha256"],
            "reason_code": "neural.prepare-succeeded",
            "runtime_identity_receipt_sha256": identity["receipt_sha256"],
            "runtime_launch_expectation_sha256": launch["receipt_sha256"],
            "schema_version": "engram.nest-worker-preparation-attempt.v1",
            "scientific_authority": False,
            "session_binding_receipt_sha256": binding["receipt_sha256"],
            "study_run_id": study_run_id,
            "worker_launch_attempt_sha256": launch_attempt["receipt_sha256"],
            "worker_request_dispatched": True,
            "worker_response_observed": True,
        }
    )
    preparation_attempt["receipt_sha256"] = digest_without(
        preparation_attempt, "receipt_sha256"
    )
    request_sha256 = "8" * 64
    execution_sha256 = "9" * 64
    step_attempt = {field: None for field in STEP_ATTEMPT_FIELDS}
    step_attempt.update(
        {
            "attempt_index": 1,
            "before_biological_time_tics": 0,
            "decoded_proposal_produced": True,
            "execution_receipt_sha256": execution_sha256,
            "observation_scope": "child-reported",
            "observed_after_biological_time_tics": 10_000,
            "outcome": "succeeded",
            "partial_readback_sha256": "a" * 64,
            "reason_code": "neural.step-succeeded",
            "request_sha256": request_sha256,
            "requested_run_tics": 10_000,
            "schema_version": "engram.nest-step-attempt.v1",
            "scientific_authority": False,
            "simulation_dispatched": True,
            "simulation_returned": True,
            "step_index": 1,
        }
    )
    step_attempt["receipt_sha256"] = digest_without(step_attempt, "receipt_sha256")
    worker_lifecycle = {
        "runtime_launch_expectation_sha256": launch["receipt_sha256"],
        "session_binding_receipt_sha256": binding["receipt_sha256"],
        "runtime_identity_receipt_sha256": identity["receipt_sha256"],
        "worker_pid": 123,
        "process_group_id": 123,
        "guardian_pid": 124,
        "session_id": 122,
    }
    evidence = {field: None for field in NEST_BUNDLE_FIELDS}
    evidence.update(
        {
            "schema_version": "engram.nest-closed-loop-evidence-bundle.v2",
            "digest_canonicalization": "engram.managed-runtime-json.v1",
            "profile": "killable-nest-population-controller-v2",
            "study_run_id": study_run_id,
            "run_receipt_sha256": "b" * 64,
            "neural_provider_identity_sha256": parent_provider_sha256,
            "neural_preparation_sha256": provider_prepared["receipt_sha256"],
            "runtime_launch_expectation": launch,
            "worker_launch_attempt": launch_attempt,
            "preparation_attempt": preparation_attempt,
            "child_capabilities": capabilities,
            "child_preparation_receipt": child_prepared,
            "provider_preparation_receipt": provider_prepared,
            "nest_session_readback": session,
            "step_attempt_receipts": [step_attempt],
            "step_execution_receipts": [
                {
                    "receipt_sha256": execution_sha256,
                    "step_index": 1,
                    "before_biological_time_tics": 0,
                    "after_biological_time_tics": 10_000,
                    "requested_run_tics": 10_000,
                }
            ],
            "worker_termination_attempt_receipts": [],
            "tail_disposition_receipt": {},
            "worker_lifecycle_receipt": worker_lifecycle,
            "worker_terminal_disposition": "confirmed-lifecycle",
            "worker_runtime_identity": identity,
            "worker_session_binding": binding,
            "execution_authority": False,
            "ncp_control": False,
            "physical_actuation": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        }
    )
    evidence["bundle_sha256"] = managed_runtime_digest_without(
        evidence,
        "bundle_sha256",
    )
    capture = {
        "nest_config": launch["controller_configuration"],
        "engram_source_closure": {
            "sources": sources,
            "worker_project_modules": [
                {"module_name": module, "relative_path": relative}
                for relative, module, _payload in source_specs
            ],
            "worker_project_source_roster_sha256": project_roster_sha256,
        },
        "terminal_receipt": {
            "study_run_id": study_run_id,
            "closed_loop_definition_sha256": definition_sha256,
            "neural_provider_identity_sha256": parent_provider_sha256,
            "neural_preparation_sha256": provider_prepared["receipt_sha256"],
            "timebase": {"neural_step_duration_tics": 10_000},
            "neural_executions": [{"provider_execution_sha256": execution_sha256}],
        },
        "neural_steps": [{"request": {"request_sha256": request_sha256}}],
        "nest_evidence_bundle": evidence,
    }
    return capture, roster


def reseal_nest_v2_execution_lineage_fixture(capture: dict[str, Any]) -> None:
    """Reseal dependent V2 receipts after one controlled hostile mutation."""

    evidence = capture["nest_evidence_bundle"]
    terminal = capture["terminal_receipt"]
    launch = evidence["runtime_launch_expectation"]
    launch["receipt_sha256"] = digest_without(launch, "receipt_sha256")
    launch_attempt = evidence["worker_launch_attempt"]
    launch_attempt["launch_expectation_sha256"] = launch["receipt_sha256"]
    launch_attempt["receipt_sha256"] = digest_without(launch_attempt, "receipt_sha256")
    session = evidence["nest_session_readback"]
    session["receipt_sha256"] = digest_without(session, "receipt_sha256")
    identity = evidence["worker_runtime_identity"]
    identity["receipt_sha256"] = digest_without(identity, "receipt_sha256")
    child_prepared = evidence["child_preparation_receipt"]
    child_prepared["provider_session_receipt_sha256"] = session["receipt_sha256"]
    child_prepared["receipt_sha256"] = managed_runtime_digest_without(
        child_prepared,
        "receipt_sha256",
    )
    capabilities_sha256 = digest_bytes(canonical(evidence["child_capabilities"]))
    binding = evidence["worker_session_binding"]
    binding.update(
        {
            "runtime_launch_expectation_sha256": launch["receipt_sha256"],
            "worker_launch_attempt_sha256": launch_attempt["receipt_sha256"],
            "child_capabilities_sha256": capabilities_sha256,
            "child_prepared_receipt_sha256": child_prepared["receipt_sha256"],
            "child_session_receipt_sha256": session["receipt_sha256"],
            "worker_runtime_identity_sha256": identity["receipt_sha256"],
        }
    )
    binding["receipt_sha256"] = digest_without(binding, "receipt_sha256")
    provider_prepared = evidence["provider_preparation_receipt"]
    provider_prepared["provider_session_receipt_sha256"] = binding["receipt_sha256"]
    provider_prepared["receipt_sha256"] = managed_runtime_digest_without(
        provider_prepared, "receipt_sha256"
    )
    terminal["neural_preparation_sha256"] = provider_prepared["receipt_sha256"]
    evidence["neural_preparation_sha256"] = provider_prepared["receipt_sha256"]
    preparation_attempt = evidence["preparation_attempt"]
    preparation_attempt.update(
        {
            "provider_preparation_receipt_sha256": provider_prepared["receipt_sha256"],
            "runtime_identity_receipt_sha256": identity["receipt_sha256"],
            "runtime_launch_expectation_sha256": launch["receipt_sha256"],
            "session_binding_receipt_sha256": binding["receipt_sha256"],
            "worker_launch_attempt_sha256": launch_attempt["receipt_sha256"],
        }
    )
    preparation_attempt["receipt_sha256"] = digest_without(
        preparation_attempt, "receipt_sha256"
    )
    worker_lifecycle = evidence["worker_lifecycle_receipt"]
    worker_lifecycle.update(
        {
            "runtime_launch_expectation_sha256": launch["receipt_sha256"],
            "session_binding_receipt_sha256": binding["receipt_sha256"],
            "runtime_identity_receipt_sha256": identity["receipt_sha256"],
        }
    )
    for attempt in evidence["step_attempt_receipts"]:
        attempt["receipt_sha256"] = digest_without(attempt, "receipt_sha256")
    evidence["bundle_sha256"] = managed_runtime_digest_without(
        evidence,
        "bundle_sha256",
    )


def worker_guardian_fixture() -> dict[str, Any]:
    worker_pid = 123
    worker_source_sha256 = "1" * 64
    worker_command_sha256 = "2" * 64
    session = {"receipt_sha256": "3" * 64}
    identity: dict[str, Any] = {"source": "fixture"}
    identity["receipt_sha256"] = digest_bytes(canonical(identity))
    binding: dict[str, Any] = {
        "worker_runtime_identity_sha256": identity["receipt_sha256"],
        "child_session_receipt_sha256": session["receipt_sha256"],
        "child_lineage_verified": True,
        "loaded_bytes_attested": False,
        "response_bound_loaded_bytes": False,
        "ncp_transport": False,
        "scientific_authority": False,
    }
    binding["receipt_sha256"] = digest_bytes(canonical(binding))
    attempt: dict[str, Any] = {
        "attempt_index": 1,
        "worker_pid": worker_pid,
        "worker_source_sha256": worker_source_sha256,
        "worker_command_sha256": worker_command_sha256,
        "child_reaped": True,
        "containment_empty": True,
        "diagnostic_stream_complete": True,
        "hard_deadline_enforcement": True,
        "ncp_transport": False,
        "physical_authority": False,
        "scientific_authority": False,
    }
    attempt["receipt_sha256"] = digest_bytes(canonical(attempt))
    attempts = [attempt]
    lifecycle: dict[str, Any] = {
        "termination_attempts": attempts,
        "termination_attempt_roster_sha256": digest_bytes(canonical(attempts)),
        "session_binding_receipt_sha256": binding["receipt_sha256"],
        "runtime_identity_receipt_sha256": identity["receipt_sha256"],
        "worker_pid": worker_pid,
        "worker_source_sha256": worker_source_sha256,
        "worker_command_sha256": worker_command_sha256,
        "child_reaped": True,
        "containment_empty": True,
        "diagnostic_stream_complete": True,
        "hard_deadline_enforcement": True,
        "ncp_transport": False,
        "physical_authority": False,
        "scientific_authority": False,
    }
    lifecycle["receipt_sha256"] = digest_bytes(canonical(lifecycle))
    closure = {
        "worker_session_binding_receipt_sha256": binding["receipt_sha256"],
        "worker_runtime_identity_receipt_sha256": identity["receipt_sha256"],
        "worker_lifecycle_receipt_sha256": lifecycle["receipt_sha256"],
        "termination_attempt_count": 1,
        "termination_attempt_roster_sha256": digest_bytes(canonical(attempts)),
        "worker_pid": worker_pid,
        "worker_source_sha256": worker_source_sha256,
        "worker_command_sha256": worker_command_sha256,
        "child_reaped": True,
        "containment_empty": True,
        "diagnostic_stream_complete": True,
    }
    return {
        "nest_worker_guardian_closure": closure,
        "nest_evidence_bundle": {
            "worker_session_binding": binding,
            "worker_runtime_identity": identity,
            "worker_lifecycle_receipt": lifecycle,
            "nest_session_readback": session,
            "worker_termination_attempt_receipts": attempts,
        },
    }


def external_summary_fixture(
    seed: str = "primary",
) -> tuple[
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
]:
    digest = digest_bytes(f"external summary fixture: {seed}".encode())
    expected_prisoma = {
        "root": Path("/fixture/prisoma"),
        "repository": "https://github.com/sepahead/prisoma.git",
        "commit": "1" * 40,
        "tree": "2" * 40,
        "origin_main": "1" * 40,
        "object_format": "sha1",
        "clean": True,
    }
    expected_engram = {
        "root": Path("/fixture/engram"),
        "repository": "https://github.com/sepahead/engram.git",
        "commit": "3" * 40,
        "tree": "4" * 40,
        "origin_main": "3" * 40,
        "object_format": "sha1",
        "clean": True,
    }
    validator_sources = [
        {
            "path": path.as_posix(),
            "sha256": digest_bytes(f"validator: {path.as_posix()}".encode()),
            "git_blob": hashlib.sha1(
                f"validator: {path.as_posix()}".encode(),
                usedforsecurity=False,
            ).hexdigest(),
            "byte_count": len(f"validator: {path.as_posix()}".encode()),
        }
        for path in NEST_VALIDATOR_SOURCE_PATHS
    ]
    validator_roster_sha256 = digest_bytes(
        b"prisoma-nest-summary-validator-source-roster-v1\0"
        + canonical(validator_sources)
    )
    imported_sources = [
        {
            "path": "backend/optimization/extension_closed_loop.py",
            "sha256": digest,
            "git_blob": hashlib.sha1(
                f"external Engram source: {seed}".encode(),
                usedforsecurity=False,
            ).hexdigest(),
            "byte_count": 1,
            "module_names": ["backend.optimization.extension_closed_loop"],
        }
    ]
    terminal = {
        "receipt_sha256": digest_bytes(f"{seed}: run receipt".encode()),
        "study_run_id": f"matrix-test-{seed}",
        "neural_provider_identity_sha256": digest_bytes(
            f"{seed}: neural provider".encode()
        ),
        "steps": [{}, {}],
    }
    evidence = {
        "bundle_sha256": digest_bytes(f"{seed}: evidence bundle".encode()),
        "step_execution_receipts": [{}, {}],
    }
    summary: dict[str, Any] = {
        "schema_version": (
            "prisoma.observer.engram-nest-evidence-validation-summary.v1"
        ),
        "validation_scope": "engram-exact-validator-rejoin-only",
        "prisoma_repository": {
            key: value for key, value in expected_prisoma.items() if key != "root"
        },
        "prisoma_validator_source_roster_sha256": validator_roster_sha256,
        "prisoma_validator_source_roster": validator_sources,
        "engram_repository": {
            key: value for key, value in expected_engram.items() if key != "root"
        },
        "engram_revision": expected_engram["commit"],
        "engram_imported_source_roster_sha256": digest_bytes(
            b"prisoma-engram-imported-source-roster-v1\0" + canonical(imported_sources)
        ),
        "engram_imported_source_roster": imported_sources,
        "inputs": {
            "summary_schema_exact_sha256": digest_bytes(
                snapshot_regular_file(NEST_SUMMARY_SCHEMA, MAX_SCHEMA_BYTES)
            ),
            "run_receipt_exact_sha256": digest_bytes(canonical(terminal) + b"\n"),
            "evidence_bundle_exact_sha256": digest_bytes(canonical(evidence) + b"\n"),
            "source_run_receipt_sha256": terminal["receipt_sha256"],
            "source_bundle_sha256": evidence["bundle_sha256"],
        },
        "lineage": {
            "study_run_id": terminal["study_run_id"],
            "neural_durable_evidence_profile": (
                "engram.nest-closed-loop-evidence-bundle.v2"
            ),
            "neural_provider_identity_sha256": terminal[
                "neural_provider_identity_sha256"
            ],
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
    }
    validation_input = {
        "prisoma_repository": summary["prisoma_repository"],
        "prisoma_validator_source_roster_sha256": validator_roster_sha256,
        "engram_repository": summary["engram_repository"],
        "engram_imported_source_roster_sha256": summary[
            "engram_imported_source_roster_sha256"
        ],
        "summary_schema_exact_sha256": summary["inputs"]["summary_schema_exact_sha256"],
        "run_receipt_exact_sha256": summary["inputs"]["run_receipt_exact_sha256"],
        "evidence_bundle_exact_sha256": summary["inputs"][
            "evidence_bundle_exact_sha256"
        ],
        "source_run_receipt_sha256": summary["inputs"]["source_run_receipt_sha256"],
        "source_bundle_sha256": summary["inputs"]["source_bundle_sha256"],
    }
    summary["inputs"]["validation_input_sha256"] = digest_bytes(
        b"prisoma-engram-nest-validation-input-v1\0" + canonical(validation_input)
    )
    summary["receipt_sha256"] = digest_bytes(canonical(summary))
    return summary, terminal, evidence, expected_prisoma, expected_engram


def expect_rejected(label: str, action: Any) -> None:
    try:
        action()
    except (MatrixError, OSError, ValueError):
        return
    raise MatrixError(f"hostile control was accepted: {label}")


def observed_build_fixture() -> dict[str, Any]:
    source_paths = sorted(
        {
            "crates/engram-managed-observer/Cargo.lock",
            "crates/engram-managed-observer/Cargo.toml",
            "crates/engram-managed-observer/src/canonical.rs",
            "crates/engram-managed-observer/src/contract.rs",
            "crates/engram-managed-observer/src/lib.rs",
            "crates/engram-managed-observer/src/main.rs",
            "crates/engram-managed-observer/src/observer.rs",
            "crates/engram-managed-observer/src/protocol.rs",
            "integrations/engram/managed-observer/contracts/configuration.schema.json",
            "integrations/engram/managed-observer/contracts/finish-request.schema.json",
            "integrations/engram/managed-observer/contracts/finish-response.schema.json",
            "integrations/engram/managed-observer/contracts/managed-runtime-ipc.schema.json",
            "integrations/engram/managed-observer/contracts/observe-request.schema.json",
            "integrations/engram/managed-observer/contracts/observe-response.schema.json",
            "integrations/engram/managed-observer/contracts/prepare-request.schema.json",
            "integrations/engram/managed-observer/contracts/prepare-response.schema.json",
        }
    )
    source_rows = [
        {
            "path": path,
            "size_bytes": index + 1,
            "sha256": format(index + 1, "064x"),
            "git_mode": "100644",
            "git_blob": format(index + 1, "040x"),
        }
        for index, path in enumerate(source_paths)
    ]
    source = {
        "manifest_path": "crates/engram-managed-observer/Cargo.toml",
        "lock_path": "crates/engram-managed-observer/Cargo.lock",
        "source_roster": source_rows,
        "source_roster_sha256": digest_bytes(
            b"prisoma-observer-build-source-roster-v1\0" + canonical(source_rows)
        ),
        "cargo_configuration_files": [],
    }

    def tool(name: str, ordinal: int, version: str) -> dict[str, Any]:
        return {
            "path": f"/fixture/bin/{name}",
            "resolved_path": f"/fixture/toolchain/{name}",
            "size_bytes": ordinal,
            "sha256": format(ordinal, "064x"),
            "version_verbose": version,
            "version_verbose_sha256": digest_bytes(version.encode("utf-8") + b"\n"),
        }

    toolchain = {
        "cargo": tool("cargo", 1, "cargo 1.93.0 fixture"),
        "rustc": tool(
            "rustc",
            2,
            "\n".join(
                (
                    "rustc 1.93.0 fixture",
                    "binary: rustc",
                    f"commit-hash: {'3' * 40}",
                    "commit-date: 2026-01-22",
                    "host: aarch64-apple-darwin",
                    "release: 1.93.0",
                    "LLVM version: 21.1.8",
                )
            ),
        ),
        "rustc_host": "aarch64-apple-darwin",
        "rustc_release": "1.93.0",
        "rustc_commit_hash": "3" * 40,
        "rustc_commit_date": "2026-01-22",
        "llvm_version": "21.1.8",
    }
    macho_payload = struct.pack(
        "<8I6I",
        0xFEEDFACF,
        observed_build.MACHO_CPU_TYPE_ARM64,
        0,
        observed_build.MACHO_FILE_TYPE_EXECUTE,
        1,
        24,
        0,
        0,
        observed_build.MACHO_BUILD_VERSION_COMMAND,
        24,
        observed_build.MACHO_PLATFORM_MACOS,
        0x000B0000,
        0x001A0500,
        0,
    )
    artifact = {
        "path": (
            "crates/engram-managed-observer/target/release/"
            "prisoma-engram-managed-observer"
        ),
        "size_bytes": len(macho_payload),
        "sha256": digest_bytes(macho_payload),
        "mode": "0700",
        "owner_private": True,
        "executable": True,
        "link_count": 1,
        "macho": observed_build.parse_arm64_macho(macho_payload),
    }
    repository = {
        "repository": "https://example.invalid/prisoma.git",
        "commit": "1" * 40,
        "tree": "2" * 40,
        "origin_main": "1" * 40,
        "object_format": "sha1",
        "clean": True,
    }
    target_directory = CRATE_MANIFEST.parent / "target" / ".observed-build-fixture_0"
    document: dict[str, Any] = {
        "schema_version": observed_build.SCHEMA_VERSION,
        "observation_scope": "one-local-clean-source-build-observation-v1",
        "repository": repository,
        "source": source,
        "toolchain": toolchain,
        "build": {
            "argv": [
                toolchain["cargo"]["path"],
                "build",
                "--locked",
                "--offline",
                "--release",
                "--manifest-path",
                str(CRATE_MANIFEST.absolute()),
                "--bin",
                "prisoma-engram-managed-observer",
                "--target-dir",
                str(target_directory.absolute()),
            ],
            "profile": "release",
            "locked": True,
            "offline": True,
            "incremental": False,
            "target_directory_isolated": True,
            "environment": observed_build.BUILD_ENVIRONMENT,
            "exit_code": 0,
            "stdout": observed_build.stream_identity(b""),
            "stderr": observed_build.stream_identity(b""),
        },
        "artifact": artifact,
        "authority": observed_build.AUTHORITY,
        "disclosure": "One synthetic local-build validation fixture without authority.",
        "receipt_sha256": "",
    }
    document["receipt_sha256"] = observed_build.digest_without(
        document,
        "receipt_sha256",
    )
    observed_build.validate_receipt_document(
        document,
        repository=repository,
        source=source,
        toolchain=toolchain,
        artifact=artifact,
    )
    payload = canonical(document) + b"\n"
    return {
        "document": document,
        "payload": payload,
        "repository": repository,
        "source": source,
        "toolchain": toolchain,
        "artifact": artifact,
        "macho_payload": macho_payload,
    }


def validate_observed_build_fixture(
    document: dict[str, Any],
    fixture: dict[str, Any],
) -> None:
    observed_build.validate_receipt_document(
        document,
        repository=fixture["repository"],
        source=fixture["source"],
        toolchain=fixture["toolchain"],
        artifact=fixture["artifact"],
    )
    schema = load_json_payload(
        snapshot_regular_file(BUILD_RECEIPT_SCHEMA, MAX_SCHEMA_BYTES),
        "observer build receipt schema",
    )
    checker = load_contract_checker()
    try:
        checker.validate_safe_project_schema(observed_build.SCHEMA_VERSION, schema)
    except SystemExit as error:
        raise MatrixError(
            f"observer build receipt schema is unsafe: {error}"
        ) from error
    if not checker.schema_accepts(document, schema):
        raise MatrixError("observer build fixture fails its closed schema")


def self_test(binary: Path) -> int:
    validate_managed_runtime_canonicalizer_contract()
    temporary_parent = canonical_temporary_parent()
    if (
        require_canonical_directory(
            temporary_parent,
            "temporary-parent positive control",
        )
        != temporary_parent
    ):
        raise MatrixError("canonical temporary-parent positive control differs")
    managed_digest_fixture = {"threshold": 1.0e-6}
    managed_digest_fixture["receipt_sha256"] = managed_runtime_digest(
        managed_digest_fixture
    )
    require_managed_runtime_digest(
        managed_digest_fixture,
        "receipt_sha256",
        "managed-runtime positive control",
    )
    binary_identity = checked_runtime_binary(binary)
    build_fixture = observed_build_fixture()
    validate_observed_build_fixture(build_fixture["document"], build_fixture)
    build_document = build_fixture["document"]
    binary_identity.update(
        {
            "build_receipt_path": RELEASE_BINARY.with_name(
                f"{RELEASE_BINARY.name}.observed-build.json"
            ).absolute(),
            "build_receipt_payload": build_fixture["payload"],
            "build_receipt_exact_sha256": digest_bytes(build_fixture["payload"]),
            "build_receipt_sha256": build_document["receipt_sha256"],
            "source_roster_sha256": build_document["source"]["source_roster_sha256"],
            "cargo_tool_sha256": build_document["toolchain"]["cargo"]["sha256"],
            "rustc_tool_sha256": build_document["toolchain"]["rustc"]["sha256"],
            "rustc_host": build_document["toolchain"]["rustc_host"],
            "rustc_release": build_document["toolchain"]["rustc_release"],
            "macho": build_document["artifact"]["macho"],
        }
    )
    source = load_json_payload(
        snapshot_regular_file(SOURCE_RECEIPT, MAX_CAPTURE_BYTES),
        "managed observer source fixture",
    )
    zero_step_source = load_json_payload(
        snapshot_regular_file(ZERO_STEP_SOURCE_RECEIPT, MAX_CAPTURE_BYTES),
        "managed observer zero-step source fixture",
    )
    cardinality_observers = [
        run_observer(
            binary_identity["path"],
            zero_step_source,
            [f"channel-{index:02d}" for index in range(1, count + 1)],
            [f"subject-{index:02d}" for index in range(1, count + 1)],
        )
        for count in (1, 2, 3)
    ]
    if any(
        observer["operation_count"] != 2 or observer["observed_step_count"] != 0
        for observer in cardinality_observers
    ):
        raise MatrixError("managed observer cardinality positive control differs")
    three_drone_observer = run_observer(
        binary_identity["path"],
        source,
        ["channel-01", "channel-02", "channel-03"],
        ["subject-01", "subject-02", "subject-03"],
    )
    observers = [copy.deepcopy(three_drone_observer) for _count in (1, 2, 3)]
    document = matrix_fixture_document(binary_identity, observers)
    validate_matrix_document(document)
    verify_matrix_prefixes(document["captures"])

    source_capture, expected_engram, pack_receipt = source_closure_fixture()
    validate_source_closure(
        source_capture,
        expected_engram,
        pack_receipt,
        verify_source_bytes=False,
    )
    lifecycle_capture = lifecycle_fixture(source_capture)
    validate_lifecycle(lifecycle_capture)
    summary_authority_fixture = {"authority": False, "simulator_only": True}
    validate_recorded_summary_authority(summary_authority_fixture)
    (
        external,
        terminal,
        evidence,
        expected_prisoma_summary,
        expected_engram_summary,
    ) = external_summary_fixture()
    validate_external_summary(
        external,
        terminal,
        evidence,
        expected_prisoma_summary,
        expected_engram_summary,
        verify_validator_source_bytes=False,
        verify_engram_source_bytes=False,
    )
    (
        package_capture,
        package_index,
        expected_crebain,
        expected_package_engram,
        tool_sources,
    ) = installed_package_fixture()
    validate_installed_package_proof(
        package_capture,
        package_index,
        expected_crebain,
        expected_package_engram,
        tool_sources,
        verify_source_bytes=False,
    )
    store_capture, store_index = receipt_store_fixture()
    validate_receipt_store_closure(store_capture, store_index)
    topology_capture, topology_index, topology_roster = population_topology_fixture()
    validate_population_topology(
        topology_capture,
        topology_index,
        topology_roster,
    )
    v2_capture, v2_roster = nest_v2_execution_lineage_fixture()
    validate_nest_v2_execution_lineage(v2_capture, v2_roster)
    guardian_capture = worker_guardian_fixture()
    validate_worker_guardian_closure(guardian_capture)
    index_shape = index_shape_fixture()
    validate_index_shape(index_shape)

    controls: list[tuple[str, Any]] = [
        (
            "duplicate JSON member",
            lambda: load_json_payload(b'{"a":1,"a":2}', "hostile JSON"),
        ),
        (
            "non-finite JSON number",
            lambda: load_json_payload(b'{"a":NaN}', "hostile JSON"),
        ),
        ("relative path traversal", lambda: safe_relative("../capture.json", "path")),
        (
            "non-release binary path",
            lambda: checked_runtime_binary(ROOT / "README.md"),
        ),
        (
            "duplicate observer subject roster",
            lambda: run_observer(
                binary_identity["path"],
                zero_step_source,
                ["channel-01", "channel-02"],
                ["subject-01", "subject-01"],
            ),
        ),
    ]
    ledger_sealed_managed_digest = {"threshold": 1.0e-6}
    ledger_sealed_managed_digest["receipt_sha256"] = digest_without(
        ledger_sealed_managed_digest,
        "receipt_sha256",
    )
    cyclic_managed_value: list[Any] = []
    cyclic_managed_value.append(cyclic_managed_value)
    overdeep_managed_value: Any = None
    for _depth in range(MANAGED_RUNTIME_MAX_DEPTH):
        overdeep_managed_value = [overdeep_managed_value]
    overlarge_managed_value = [None] * MANAGED_RUNTIME_MAX_NODES
    controls.extend(
        (
            (
                "managed-runtime digest sealed with ledger JSON",
                lambda: require_managed_runtime_digest(
                    ledger_sealed_managed_digest,
                    "receipt_sha256",
                    "ledger-sealed managed-runtime hostile control",
                ),
            ),
            (
                "managed-runtime negative zero",
                lambda: managed_runtime_canonical({"value": -0.0}),
            ),
            (
                "managed-runtime unsafe integer",
                lambda: managed_runtime_canonical(
                    {"value": MANAGED_RUNTIME_MAX_SAFE_INTEGER + 1}
                ),
            ),
            (
                "managed-runtime float above portable bound",
                lambda: managed_runtime_canonical(
                    {"value": math.nextafter(MANAGED_RUNTIME_MAX_FLOAT_ABS, math.inf)}
                ),
            ),
            (
                "managed-runtime nonportable Unicode",
                lambda: managed_runtime_canonical({"value": "\u0085"}),
            ),
            (
                "managed-runtime cyclic value",
                lambda: managed_runtime_canonical(cyclic_managed_value),
            ),
            (
                "managed-runtime excessive depth",
                lambda: managed_runtime_canonical(overdeep_managed_value),
            ),
            (
                "managed-runtime excessive node count",
                lambda: managed_runtime_canonical(overlarge_managed_value),
            ),
        )
    )
    changed = copy.deepcopy(document)
    changed["unreviewed"] = True
    controls.append(
        ("matrix open field", lambda changed=changed: validate_matrix_document(changed))
    )
    changed_sidecar = copy.deepcopy(document)
    changed_sidecars = changed_sidecar["captures"][0]["capture"][
        "receipt_store_sidecars"
    ]
    changed_sidecars["observation"]["unreviewed"] = False
    changed_sidecars["closure_sha256"] = digest_without(
        changed_sidecars,
        "closure_sha256",
    )
    changed_sidecar["receipt_sha256"] = digest_without(
        changed_sidecar,
        "receipt_sha256",
    )
    controls.append(
        (
            "matrix open V5 sidecar field",
            lambda changed=changed_sidecar: validate_matrix_document(changed),
        )
    )
    missing_sidecar_digest = copy.deepcopy(document)
    del missing_sidecar_digest["captures"][0]["capture"][
        "receipt_store_sidecars_sha256"
    ]
    missing_sidecar_digest["receipt_sha256"] = digest_without(
        missing_sidecar_digest,
        "receipt_sha256",
    )
    controls.append(
        (
            "matrix missing V5 sidecar digest",
            lambda changed=missing_sidecar_digest: validate_matrix_document(changed),
        )
    )
    missing_population_count = copy.deepcopy(document)
    del missing_population_count["captures"][0]["nest"]["population_count"]
    missing_population_count["receipt_sha256"] = digest_without(
        missing_population_count,
        "receipt_sha256",
    )
    controls.append(
        (
            "matrix missing topology population count",
            lambda changed=missing_population_count: validate_matrix_document(changed),
        )
    )
    hostile_index_shape = copy.deepcopy(index_shape)
    hostile_index_shape["unreviewed"] = True
    controls.append(
        (
            "evidence-index open field",
            lambda: validate_index_shape(hostile_index_shape),
        )
    )
    hostile_index_row = copy.deepcopy(index_shape)
    del hostile_index_row["captures"][0]["session_count"]
    controls.append(
        (
            "evidence-index row missing field",
            lambda: validate_index_shape(hostile_index_row),
        )
    )
    hostile_index_roster = copy.deepcopy(index_shape)
    del hostile_index_roster["captures"][0]["engram_source_roster_sha256"]
    controls.append(
        (
            "evidence-index row missing stable source roster",
            lambda: validate_index_shape(hostile_index_roster),
        )
    )
    hostile_index_assertion = copy.deepcopy(index_shape)
    del hostile_index_assertion["assertions"]["distinct_engram_runtime_source_closures"]
    controls.append(
        (
            "evidence-index missing distinct runtime closure assertion",
            lambda: validate_index_shape(hostile_index_assertion),
        )
    )
    changed = copy.deepcopy(document)
    changed["captures"].pop()
    controls.append(
        (
            "matrix missing cardinality",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["captures"][0]["drone_count"] = 2
    controls.append(
        (
            "matrix cardinality order",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    collapsed_capture = copy.deepcopy(document)
    collapsed_capture["captures"][1]["capture"]["exact_sha256"] = collapsed_capture[
        "captures"
    ][0]["capture"]["exact_sha256"]
    collapsed_capture["receipt_sha256"] = digest_without(
        collapsed_capture,
        "receipt_sha256",
    )
    controls.append(
        (
            "matrix collapsed capture identity",
            lambda changed=collapsed_capture: validate_matrix_document(changed),
        )
    )
    collapsed_terminal = copy.deepcopy(document)
    collapsed_terminal_capture = collapsed_terminal["captures"][1]["capture"]
    old_terminal_path = collapsed_terminal_capture["terminal_artifact_path"]
    new_terminal_digest = collapsed_terminal["captures"][0]["capture"][
        "terminal_receipt_sha256"
    ]
    new_terminal_path = f"receipts/{new_terminal_digest[:2]}/{new_terminal_digest}.json"
    collapsed_terminal_capture.update(
        {
            "terminal_receipt_sha256": new_terminal_digest,
            "terminal_artifact_path": new_terminal_path,
            "terminal_artifact_exact_sha256": new_terminal_digest,
        }
    )
    collapsed_terminal["captures"][1]["observer"]["terminal_source_receipt_sha256"] = (
        new_terminal_digest
    )
    terminal_store_row = next(
        row
        for row in collapsed_terminal_capture["receipt_store_files"]
        if row["relative_path"] == old_terminal_path
    )
    terminal_store_row.update(
        {"relative_path": new_terminal_path, "sha256": new_terminal_digest}
    )
    for prefix in ("observations", "publication-authorities"):
        sidecar_row = next(
            row
            for row in collapsed_terminal_capture["receipt_store_files"]
            if row["relative_path"].startswith(f"{prefix}/")
        )
        sidecar_row["relative_path"] = (
            f"{prefix}/{new_terminal_digest[:2]}/{new_terminal_digest}.json"
        )
    reseal_matrix_store_projection(collapsed_terminal_capture)
    collapsed_terminal["receipt_sha256"] = digest_without(
        collapsed_terminal,
        "receipt_sha256",
    )
    controls.append(
        (
            "matrix collapsed terminal identity",
            lambda changed=collapsed_terminal: validate_matrix_document(changed),
        )
    )
    collapsed_evidence = copy.deepcopy(document)
    collapsed_evidence_capture = collapsed_evidence["captures"][1]["capture"]
    old_evidence_path = collapsed_evidence_capture["evidence_artifact_path"]
    new_evidence_digest = collapsed_evidence["captures"][0]["capture"][
        "nest_evidence_bundle_sha256"
    ]
    new_evidence_path = f"evidence/{new_evidence_digest[:2]}/{new_evidence_digest}.json"
    collapsed_evidence_capture.update(
        {
            "nest_evidence_bundle_sha256": new_evidence_digest,
            "evidence_artifact_path": new_evidence_path,
            "evidence_artifact_exact_sha256": new_evidence_digest,
        }
    )
    evidence_store_row = next(
        row
        for row in collapsed_evidence_capture["receipt_store_files"]
        if row["relative_path"] == old_evidence_path
    )
    evidence_store_row.update(
        {"relative_path": new_evidence_path, "sha256": new_evidence_digest}
    )
    reseal_matrix_store_projection(collapsed_evidence_capture)
    collapsed_evidence["receipt_sha256"] = digest_without(
        collapsed_evidence,
        "receipt_sha256",
    )
    controls.append(
        (
            "matrix collapsed evidence identity",
            lambda changed=collapsed_evidence: validate_matrix_document(changed),
        )
    )
    changed_source_count = copy.deepcopy(document)
    changed_source_count["captures"][1]["source"]["engram_source_file_count"] += 1
    changed_source_count["receipt_sha256"] = digest_without(
        changed_source_count,
        "receipt_sha256",
    )
    controls.append(
        (
            "matrix stable source count drift",
            lambda changed=changed_source_count: validate_matrix_document(changed),
        )
    )
    for prefix, label in (
        ("observations/", "observation"),
        ("publication-authorities/", "publication authority"),
        ("finalized-reservations/", "finalized reservation"),
        ("publication-admission-anchors/", "publication admission anchor"),
    ):
        changed_sidecar_path = copy.deepcopy(document)
        changed_store_capture = changed_sidecar_path["captures"][0]["capture"]
        sidecar_row = next(
            row
            for row in changed_store_capture["receipt_store_files"]
            if row["relative_path"].startswith(prefix)
        )
        sidecar_row["relative_path"] = f"{prefix}ff/{'f' * 64}.json"
        reseal_matrix_store_projection(changed_store_capture)
        changed_sidecar_path["receipt_sha256"] = digest_without(
            changed_sidecar_path,
            "receipt_sha256",
        )
        controls.append(
            (
                f"matrix V5 {label} path drift",
                lambda changed=changed_sidecar_path: validate_matrix_document(changed),
            )
        )
    for metadata_path, label in (
        ("store.json", "store metadata"),
        ("writer.lock", "writer lock"),
    ):
        changed_metadata = copy.deepcopy(document)
        changed_store_capture = changed_metadata["captures"][0]["capture"]
        metadata_row = next(
            row
            for row in changed_store_capture["receipt_store_files"]
            if row["relative_path"] == metadata_path
        )
        metadata_row.update({"size_bytes": 1, "sha256": "0" * 64})
        reseal_matrix_store_projection(changed_store_capture)
        changed_metadata["receipt_sha256"] = digest_without(
            changed_metadata,
            "receipt_sha256",
        )
        controls.append(
            (
                f"matrix V5 {label} byte drift",
                lambda changed=changed_metadata: validate_matrix_document(changed),
            )
        )
    changed = copy.deepcopy(document)
    changed["authority"]["ncp_authority"] = True
    controls.append(
        (
            "matrix NCP authority",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["authority"]["music_authority"] = True
    controls.append(
        (
            "matrix MUSIC authority",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["prisoma_repository"]["object_format"] = "sha256"
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "repository object-format mismatch",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["crebain_source_repository"]["origin_main_at_capture"] = "0" * 40
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "source repository capture-main drift",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["crebain_evidence_publication"]["parent_commit"] = "0" * 40
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "evidence publication wrong parent",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["crebain_evidence_publication"]["commit"] = changed["sources"][
        "crebain_source_repository"
    ]["commit"]
    changed["sources"]["crebain_evidence_publication"]["origin_main"] = changed[
        "sources"
    ]["crebain_source_repository"]["commit"]
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "identical source and publication commits",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["crebain_evidence_publication"]["origin_main"] = "0" * 40
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "evidence publication origin-main drift",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["crebain_evidence_publication"]["repository"] = (
        "https://example.invalid/swapped.git"
    )
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "evidence publication repository drift",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["crebain_evidence_publication"]["files"].pop()
    changed["sources"]["crebain_evidence_publication"]["file_count"] = 3
    changed["sources"]["crebain_evidence_publication"]["roster_sha256"] = digest_bytes(
        EVIDENCE_PUBLICATION_ROSTER_DOMAIN
        + canonical(changed["sources"]["crebain_evidence_publication"]["files"])
    )
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "evidence publication missing file",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["crebain_evidence_publication"]["files"][0]["git_mode"] = (
        "100755"
    )
    changed["sources"]["crebain_evidence_publication"]["roster_sha256"] = digest_bytes(
        EVIDENCE_PUBLICATION_ROSTER_DOMAIN
        + canonical(changed["sources"]["crebain_evidence_publication"]["files"])
    )
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "evidence publication executable mode",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    files = changed["sources"]["crebain_evidence_publication"]["files"]
    files[1]["sha256"], files[2]["sha256"] = (
        files[2]["sha256"],
        files[1]["sha256"],
    )
    changed["sources"]["crebain_evidence_publication"]["roster_sha256"] = digest_bytes(
        EVIDENCE_PUBLICATION_ROSTER_DOMAIN + canonical(files)
    )
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "swapped valid evidence publication rows",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["crebain_evidence_publication"]["roster_sha256"] = "0" * 64
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "evidence publication roster digest drift",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["assertions"]["crebain_evidence_publication_verified"] = False
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "evidence publication assertion denial",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["captures"][0]["observer"]["source_durable_evidence_verified"] = True
    controls.append(
        (
            "observer durable-evidence claim",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["captures"][0]["lifecycle"]["filesystem_isolation_enforced"] = True
    controls.append(
        (
            "filesystem isolation escalation",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["captures"][0]["capture"]["engram_pack_receipt_sha256"] = "0" * 64
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "matrix pack-receipt projection drift",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["captures"][0]["capture"]["engram_extension_tool_git_blob"] = "0" * 40
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "matrix Engram tool Git projection drift",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["assertions"]["engram_pack_source_lineage_common"] = False
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "matrix pack-source assertion denial",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["captures"][1]["source"]["engram_source_closure_sha256"] = changed[
        "captures"
    ][0]["source"]["engram_source_closure_sha256"]
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "matrix reused runtime source closure",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["assertions"]["distinct_runtime_source_closures_verified"] = False
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "matrix distinct runtime source-closure assertion denial",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["captures"][0]["source"]["engram_source_roster_sha256"] = "0" * 64
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "matrix per-run stable source roster drift",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["sources"]["shared_engram_source_roster_sha256"] = "0" * 64
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "matrix shared source roster projection drift",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed = copy.deepcopy(document)
    changed["captures"][0]["capture"]["terminal_artifact_path"] = (
        "receipts/ff/"
        + changed["captures"][0]["capture"]["terminal_receipt_sha256"]
        + ".json"
    )
    changed["receipt_sha256"] = digest_without(changed, "receipt_sha256")
    controls.append(
        (
            "matrix non-content-addressed receipt path",
            lambda changed=changed: validate_matrix_document(changed),
        )
    )
    changed_source = copy.deepcopy(source_capture)
    changed_source["engram_source_closure"]["closure_sha256"] = "0" * 64
    controls.append(
        (
            "source closure digest drift",
            lambda changed_source=changed_source: validate_source_closure(
                changed_source,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    changed_source = copy.deepcopy(source_capture)
    changed_source["engram_source_closure"]["source_roster_sha256"] = "0" * 64
    changed_source["engram_source_closure"]["closure_sha256"] = digest_without(
        changed_source["engram_source_closure"],
        "closure_sha256",
    )
    controls.append(
        (
            "coherently resealed stable source roster digest drift",
            lambda changed_source=changed_source: validate_source_closure(
                changed_source,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    missing_exec_gate_field = copy.deepcopy(source_capture)
    del missing_exec_gate_field["engram_source_closure"][
        "reviewed_runtime_exec_gate_command_sha256"
    ]
    missing_exec_gate_field["engram_source_closure"]["closure_sha256"] = digest_without(
        missing_exec_gate_field["engram_source_closure"],
        "closure_sha256",
    )
    controls.append(
        (
            "coherently resealed source closure missing exec-gate command",
            lambda capture=missing_exec_gate_field: validate_source_closure(
                capture,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    open_exec_gate_closure = copy.deepcopy(source_capture)
    open_exec_gate_closure["engram_source_closure"]["unreviewed_exec_gate"] = True
    open_exec_gate_closure["engram_source_closure"]["closure_sha256"] = digest_without(
        open_exec_gate_closure["engram_source_closure"],
        "closure_sha256",
    )
    controls.append(
        (
            "coherently resealed source closure with extra exec-gate field",
            lambda capture=open_exec_gate_closure: validate_source_closure(
                capture,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    swapped_exec_gate_fields = copy.deepcopy(source_capture)
    swapped_exec_gate_closure = swapped_exec_gate_fields["engram_source_closure"]
    (
        swapped_exec_gate_closure["reviewed_runtime_exec_gate_source_sha256"],
        swapped_exec_gate_closure["reviewed_runtime_exec_gate_command_sha256"],
    ) = (
        swapped_exec_gate_closure["reviewed_runtime_exec_gate_command_sha256"],
        swapped_exec_gate_closure["reviewed_runtime_exec_gate_source_sha256"],
    )
    swapped_exec_gate_closure["closure_sha256"] = digest_without(
        swapped_exec_gate_closure,
        "closure_sha256",
    )
    controls.append(
        (
            "coherently resealed swapped exec-gate source and command",
            lambda capture=swapped_exec_gate_fields: validate_source_closure(
                capture,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    drifted_exec_gate_command = copy.deepcopy(source_capture)
    drifted_exec_gate_command["engram_source_closure"][
        "reviewed_runtime_exec_gate_command_sha256"
    ] = "0" * 64
    drifted_exec_gate_command["engram_source_closure"]["closure_sha256"] = (
        digest_without(
            drifted_exec_gate_command["engram_source_closure"],
            "closure_sha256",
        )
    )
    controls.append(
        (
            "coherently resealed one-sided exec-gate command drift",
            lambda capture=drifted_exec_gate_command: validate_source_closure(
                capture,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    resealed_exec_gate_shape = copy.deepcopy(source_capture)
    resealed_binding = resealed_exec_gate_shape["reviewed_native_runtime"][
        "exec_gate_command_binding"
    ]
    resealed_binding["argument_shape"][0] = "launcher"
    resealed_binding["exec_gate_command_sha256"] = digest_without(
        resealed_binding,
        "exec_gate_command_sha256",
    )
    resealed_handshake = resealed_exec_gate_shape["reviewed_native_runtime"][
        "handshake_receipt"
    ]
    resealed_handshake["exec_gate_command_sha256"] = resealed_binding[
        "exec_gate_command_sha256"
    ]
    resealed_handshake["receipt_sha256"] = digest_without(
        resealed_handshake,
        "receipt_sha256",
    )
    resealed_exec_gate_closure = resealed_exec_gate_shape["engram_source_closure"]
    resealed_exec_gate_closure["reviewed_runtime_exec_gate_command_sha256"] = (
        resealed_binding["exec_gate_command_sha256"]
    )
    resealed_exec_gate_closure["reviewed_runtime_handshake_receipt_sha256"] = (
        resealed_handshake["receipt_sha256"]
    )
    resealed_exec_gate_closure["closure_sha256"] = digest_without(
        resealed_exec_gate_closure,
        "closure_sha256",
    )
    controls.append(
        (
            "coherently resealed contained-exec argument-shape drift",
            lambda capture=resealed_exec_gate_shape: validate_source_closure(
                capture,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    changed_source = copy.deepcopy(source_capture)
    changed_source["nest_evidence_bundle"]["worker_session_binding"][
        "worker_project_source_roster_sha256"
    ] = "0" * 64
    controls.append(
        (
            "worker source roster drift",
            lambda changed_source=changed_source: validate_source_closure(
                changed_source,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    changed_source = copy.deepcopy(source_capture)
    changed_source["engram_source_closure"]["host_modules"].pop()
    changed_source["engram_source_closure"]["closure_sha256"] = digest_without(
        changed_source["engram_source_closure"],
        "closure_sha256",
    )
    controls.append(
        (
            "pack tool absent from loaded host-module closure",
            lambda changed_source=changed_source: validate_source_closure(
                changed_source,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    orphan_source = copy.deepcopy(source_capture)
    orphan_row = {
        "relative_path": "backend/unused.py",
        "size_bytes": 1,
        "sha256": "0" * 64,
        "git_mode": "100644",
        "git_blob": "0" * 40,
    }
    orphan_source["engram_source_closure"]["sources"].append(orphan_row)
    orphan_source["engram_source_closure"]["sources"].sort(
        key=lambda row: row["relative_path"]
    )
    orphan_source["engram_source_sha256"][orphan_row["relative_path"]] = orphan_row[
        "sha256"
    ]
    orphan_source["engram_source_closure"]["source_roster_sha256"] = digest_bytes(
        b"crebain.engram-source-roster.v1\0"
        + canonical(orphan_source["engram_source_closure"]["sources"])
    )
    orphan_source["engram_source_closure"]["closure_sha256"] = digest_without(
        orphan_source["engram_source_closure"],
        "closure_sha256",
    )
    controls.append(
        (
            "orphan Engram source row",
            lambda capture=orphan_source: validate_source_closure(
                capture,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    misleading_host_module = copy.deepcopy(source_capture)
    misleading_host_module["engram_source_closure"]["host_modules"][0][
        "module_name"
    ] = "misleading.module"
    misleading_host_module["engram_source_closure"]["host_modules"].sort(
        key=lambda row: (row["module_name"], row["relative_path"])
    )
    misleading_host_module["engram_source_closure"]["closure_sha256"] = digest_without(
        misleading_host_module["engram_source_closure"],
        "closure_sha256",
    )
    controls.append(
        (
            "misleading Engram host-module name",
            lambda capture=misleading_host_module: validate_source_closure(
                capture,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    misleading_entrypoint = copy.deepcopy(source_capture)
    misleading_entrypoint["engram_source_closure"]["exercised_entrypoints"][0][
        "role"
    ] = "arbitrary-entrypoint"
    misleading_entrypoint["engram_source_closure"]["closure_sha256"] = digest_without(
        misleading_entrypoint["engram_source_closure"],
        "closure_sha256",
    )
    controls.append(
        (
            "misleading Engram exercised-entrypoint role",
            lambda capture=misleading_entrypoint: validate_source_closure(
                capture,
                expected_engram,
                pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    changed_pack_receipt = copy.deepcopy(pack_receipt)
    changed_pack_receipt["engram_tool"]["sha256"] = "0" * 64
    controls.append(
        (
            "pack tool differs from loaded source row",
            lambda: validate_source_closure(
                source_capture,
                expected_engram,
                changed_pack_receipt,
                verify_source_bytes=False,
            ),
        )
    )
    changed_lifecycle = copy.deepcopy(lifecycle_capture)
    changed_lifecycle["reviewed_native_runtime"]["termination_receipt"][
        "guardian_reaped"
    ] = False
    controls.append(
        ("guardian not reaped", lambda: validate_lifecycle(changed_lifecycle))
    )
    resealed_lifecycle = copy.deepcopy(lifecycle_capture)
    resealed_lifecycle["reviewed_native_runtime"]["handshake_receipt"][
        "extension_id"
    ] = "hostile.resealed.extension"
    reseal_lifecycle_fixture(resealed_lifecycle)
    controls.append(
        (
            "coherently resealed handshake extension identity",
            lambda: validate_lifecycle(resealed_lifecycle),
        )
    )
    resealed_spawn = copy.deepcopy(lifecycle_capture)
    resealed_spawn["reviewed_native_runtime"]["handshake_receipt"][
        "path_lookup_at_spawn"
    ] = False
    reseal_lifecycle_fixture(resealed_spawn)
    controls.append(
        (
            "coherently resealed non-PATH exec-gate spawn",
            lambda: validate_lifecycle(resealed_spawn),
        )
    )
    summary_with_authority = copy.deepcopy(summary_authority_fixture)
    summary_with_authority["authority"] = True
    controls.append(
        (
            "recorded summary grants authority",
            lambda: validate_recorded_summary_authority(summary_with_authority),
        )
    )
    summary_without_simulator_scope = copy.deepcopy(summary_authority_fixture)
    summary_without_simulator_scope["simulator_only"] = False
    controls.append(
        (
            "recorded summary omits simulator-only scope",
            lambda: validate_recorded_summary_authority(
                summary_without_simulator_scope
            ),
        )
    )
    missing_v2_field = copy.deepcopy(v2_capture)
    del missing_v2_field["nest_evidence_bundle"]["runtime_launch_expectation"][
        "guardian_group_member"
    ]
    controls.append(
        (
            "V2 launch expectation missing closed field",
            lambda capture=missing_v2_field: validate_nest_v2_execution_lineage(
                capture,
                v2_roster,
            ),
        )
    )
    drifted_v2_runtime = copy.deepcopy(v2_capture)
    drifted_launch = drifted_v2_runtime["nest_evidence_bundle"][
        "runtime_launch_expectation"
    ]
    drifted_launch["required_runtime_files"][0]["sha256"] = "f" * 64
    drifted_project_files = [
        row
        for row in drifted_launch["required_runtime_files"]
        if row["role"].startswith("project-module:")
    ]
    drifted_launch["required_runtime_file_roster_sha256"] = digest_bytes(
        canonical(drifted_launch["required_runtime_files"])
    )
    drifted_launch["required_project_source_roster_sha256"] = digest_bytes(
        canonical(drifted_project_files)
    )
    reseal_nest_v2_execution_lineage_fixture(drifted_v2_runtime)
    controls.append(
        (
            "V2 runtime project-source roster drift",
            lambda capture=drifted_v2_runtime: validate_nest_v2_execution_lineage(
                capture,
                v2_roster,
            ),
        )
    )
    drifted_exec_gate = copy.deepcopy(v2_capture)
    drifted_exec_gate["nest_evidence_bundle"]["runtime_launch_expectation"][
        "exec_gate_source_file"
    ]["role"] = "project-module:backend.integrations.contained_exec_gate"
    reseal_nest_v2_execution_lineage_fixture(drifted_exec_gate)
    controls.append(
        (
            "V2 embedded exec-gate role drift",
            lambda capture=drifted_exec_gate: validate_nest_v2_execution_lineage(
                capture,
                v2_roster,
            ),
        )
    )
    drifted_capabilities = copy.deepcopy(v2_capture)
    drifted_capabilities["nest_evidence_bundle"]["child_capabilities"][
        "ncp_transport"
    ] = True
    reseal_nest_v2_execution_lineage_fixture(drifted_capabilities)
    controls.append(
        (
            "V2 child capability authority drift",
            lambda capture=drifted_capabilities: validate_nest_v2_execution_lineage(
                capture,
                v2_roster,
            ),
        )
    )
    drifted_preparation = copy.deepcopy(v2_capture)
    drifted_preparation["nest_evidence_bundle"]["provider_preparation_receipt"][
        "receipt_sha256"
    ] = "0" * 64
    drifted_preparation["nest_evidence_bundle"]["bundle_sha256"] = (
        managed_runtime_digest_without(
            drifted_preparation["nest_evidence_bundle"],
            "bundle_sha256",
        )
    )
    controls.append(
        (
            "V2 provider preparation digest drift",
            lambda capture=drifted_preparation: validate_nest_v2_execution_lineage(
                capture,
                v2_roster,
            ),
        )
    )
    drifted_step_attempt = copy.deepcopy(v2_capture)
    drifted_step_attempt["nest_evidence_bundle"]["step_attempt_receipts"][0][
        "observed_after_biological_time_tics"
    ] += 1
    reseal_nest_v2_execution_lineage_fixture(drifted_step_attempt)
    controls.append(
        (
            "V2 step-attempt time drift",
            lambda capture=drifted_step_attempt: validate_nest_v2_execution_lineage(
                capture,
                v2_roster,
            ),
        )
    )
    changed_external = copy.deepcopy(external)
    changed_external["inputs"]["source_bundle_sha256"] = "0" * 64
    changed_external["receipt_sha256"] = digest_without(
        changed_external,
        "receipt_sha256",
    )
    controls.append(
        (
            "NEST bundle lineage drift",
            lambda: validate_external_summary(
                changed_external,
                terminal,
                evidence,
                expected_prisoma_summary,
                expected_engram_summary,
                verify_validator_source_bytes=False,
                verify_engram_source_bytes=False,
            ),
        )
    )
    changed_external = copy.deepcopy(external)
    changed_external["authority"]["source_durable_evidence_verified"] = False
    changed_external["receipt_sha256"] = digest_without(
        changed_external,
        "receipt_sha256",
    )
    controls.append(
        (
            "external validator durable-evidence denial",
            lambda: validate_external_summary(
                changed_external,
                terminal,
                evidence,
                expected_prisoma_summary,
                expected_engram_summary,
                verify_validator_source_bytes=False,
                verify_engram_source_bytes=False,
            ),
        )
    )
    (
        secondary_external,
        secondary_terminal,
        secondary_evidence,
        secondary_prisoma,
        secondary_engram,
    ) = external_summary_fixture("secondary")
    validate_external_summary(
        secondary_external,
        secondary_terminal,
        secondary_evidence,
        secondary_prisoma,
        secondary_engram,
        verify_validator_source_bytes=False,
        verify_engram_source_bytes=False,
    )
    controls.append(
        (
            "swapped individually valid NEST summary and evidence",
            lambda: validate_external_summary(
                secondary_external,
                terminal,
                evidence,
                expected_prisoma_summary,
                expected_engram_summary,
                verify_validator_source_bytes=False,
                verify_engram_source_bytes=False,
            ),
        )
    )
    swapped_external_source = copy.deepcopy(external)
    swapped_external_source["engram_imported_source_roster"] = copy.deepcopy(
        secondary_external["engram_imported_source_roster"]
    )
    swapped_external_source["engram_imported_source_roster_sha256"] = (
        secondary_external["engram_imported_source_roster_sha256"]
    )
    swapped_external_source["receipt_sha256"] = digest_without(
        swapped_external_source,
        "receipt_sha256",
    )
    controls.append(
        (
            "swapped individually valid Engram source roster",
            lambda: validate_external_summary(
                swapped_external_source,
                terminal,
                evidence,
                expected_prisoma_summary,
                expected_engram_summary,
                verify_validator_source_bytes=False,
                verify_engram_source_bytes=False,
            ),
        )
    )
    hostile_package = copy.deepcopy(package_capture)
    hostile_package_index = copy.deepcopy(package_index)
    hostile_package["installed_package_proof"]["authority"]["ncp_qualified"] = True
    hostile_package["installed_package_proof"]["receipt_sha256"] = digest_without(
        hostile_package["installed_package_proof"],
        "receipt_sha256",
    )
    hostile_package_sha256 = digest_bytes(
        canonical(hostile_package["installed_package_proof"]) + b"\n"
    )
    hostile_package["installed_package_proof_exact_sha256"] = hostile_package_sha256
    hostile_package_index["installed_package_proof_exact_sha256"] = (
        hostile_package_sha256
    )
    hostile_package_index["package"]["receipt_sha256"] = hostile_package[
        "installed_package_proof"
    ]["receipt_sha256"]
    controls.append(
        (
            "installed-package NCP authority",
            lambda: validate_installed_package_proof(
                hostile_package,
                hostile_package_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    (
        secondary_package,
        secondary_package_index,
        secondary_crebain,
        secondary_package_engram,
        secondary_tool_sources,
    ) = installed_package_fixture("secondary")
    validate_installed_package_proof(
        secondary_package,
        secondary_package_index,
        secondary_crebain,
        secondary_package_engram,
        secondary_tool_sources,
        verify_source_bytes=False,
    )

    def add_package_control(
        label: str,
        hostile_capture: dict[str, Any],
        hostile_index: dict[str, Any],
    ) -> None:
        controls.append(
            (
                label,
                lambda: validate_installed_package_proof(
                    hostile_capture,
                    hostile_index,
                    expected_crebain,
                    expected_package_engram,
                    tool_sources,
                    verify_source_bytes=False,
                ),
            )
        )

    hostile_pack_schema = copy.deepcopy(package_capture)
    hostile_pack_schema_index = copy.deepcopy(package_index)
    hostile_pack_schema["installed_package_proof"]["engram_pack_receipt"][
        "schema_version"
    ] = "crebain.managed-simulation-engram-pack-receipt.v0"
    reseal_installed_package_fixture(
        hostile_pack_schema,
        hostile_pack_schema_index,
    )
    add_package_control(
        "historical Engram pack-receipt schema",
        hostile_pack_schema,
        hostile_pack_schema_index,
    )
    hostile_pack_open = copy.deepcopy(package_capture)
    hostile_pack_open_index = copy.deepcopy(package_index)
    hostile_pack_open["installed_package_proof"]["engram_pack_receipt"][
        "unreviewed"
    ] = True
    reseal_installed_package_fixture(hostile_pack_open, hostile_pack_open_index)
    add_package_control(
        "Engram pack receipt open field",
        hostile_pack_open,
        hostile_pack_open_index,
    )
    hostile_pack_operation = copy.deepcopy(package_capture)
    hostile_pack_operation_index = copy.deepcopy(package_index)
    hostile_pack_operation["installed_package_proof"]["engram_pack_receipt"][
        "operations"
    ][0]["source_reverified"] = False
    reseal_installed_package_fixture(
        hostile_pack_operation,
        hostile_pack_operation_index,
    )
    add_package_control(
        "Engram pack operation without source re-verification",
        hostile_pack_operation,
        hostile_pack_operation_index,
    )
    hostile_pack_claim = copy.deepcopy(package_capture)
    hostile_pack_claim_index = copy.deepcopy(package_index)
    hostile_pack_claim["installed_package_proof"]["engram_pack_receipt"]["claims"][
        "publisher_authenticated"
    ] = True
    reseal_installed_package_fixture(hostile_pack_claim, hostile_pack_claim_index)
    add_package_control(
        "Engram pack publisher-authentication claim",
        hostile_pack_claim,
        hostile_pack_claim_index,
    )
    hostile_pack_authority = copy.deepcopy(package_capture)
    hostile_pack_authority_index = copy.deepcopy(package_index)
    hostile_pack_authority["installed_package_proof"]["engram_pack_receipt"][
        "authority"
    ]["execution"] = True
    reseal_installed_package_fixture(
        hostile_pack_authority,
        hostile_pack_authority_index,
    )
    add_package_control(
        "Engram pack execution authority",
        hostile_pack_authority,
        hostile_pack_authority_index,
    )
    hostile_pack_repository = copy.deepcopy(package_capture)
    hostile_pack_repository_index = copy.deepcopy(package_index)
    hostile_pack_repository_row = hostile_pack_repository["installed_package_proof"][
        "engram_pack_receipt"
    ]["engram_repository"]
    hostile_pack_repository_row.update(
        {
            "commit": "e" * 40,
            "tree": "f" * 40,
            "origin_main": "e" * 40,
        }
    )
    reseal_installed_package_fixture(
        hostile_pack_repository,
        hostile_pack_repository_index,
    )
    add_package_control(
        "coherently resealed foreign Engram pack revision",
        hostile_pack_repository,
        hostile_pack_repository_index,
    )
    hostile_pack_tool = copy.deepcopy(package_capture)
    hostile_pack_tool_index = copy.deepcopy(package_index)
    hostile_pack_tool["installed_package_proof"]["engram_pack_receipt"]["engram_tool"][
        "relative_path"
    ] = "scripts/hostile_extension.py"
    reseal_installed_package_fixture(hostile_pack_tool, hostile_pack_tool_index)
    add_package_control(
        "coherently resealed foreign Engram pack tool",
        hostile_pack_tool,
        hostile_pack_tool_index,
    )
    hostile_pack_seal = copy.deepcopy(package_capture)
    hostile_pack_seal_index = copy.deepcopy(package_index)
    hostile_pack_seal["installed_package_proof"]["engram_pack_receipt"][
        "seal_receipt_exact_sha256"
    ] = "0" * 64
    reseal_installed_package_fixture(hostile_pack_seal, hostile_pack_seal_index)
    add_package_control(
        "coherently resealed Engram pack seal drift",
        hostile_pack_seal,
        hostile_pack_seal_index,
    )
    hostile_pack_lineage = copy.deepcopy(package_capture)
    hostile_pack_lineage_index = copy.deepcopy(package_index)
    hostile_pack_lineage["installed_package_proof"][
        "build_stage_seal_pack_install_lineage_verified"
    ] = False
    reseal_installed_outer_fixture(
        hostile_pack_lineage,
        hostile_pack_lineage_index,
    )
    add_package_control(
        "installed proof denies pack-install lineage",
        hostile_pack_lineage,
        hostile_pack_lineage_index,
    )
    hostile_pack_index = copy.deepcopy(package_index)
    hostile_pack_index["package"]["engram_pack_receipt_sha256"] = "0" * 64
    add_package_control(
        "evidence index pack-receipt digest drift",
        package_capture,
        hostile_pack_index,
    )
    swapped_pack_capture = copy.deepcopy(package_capture)
    swapped_pack_index = copy.deepcopy(package_index)
    swapped_pack_proof = swapped_pack_capture["installed_package_proof"]
    swapped_pack = copy.deepcopy(
        secondary_package["installed_package_proof"]["engram_pack_receipt"]
    )
    swapped_pack_proof["engram_pack_receipt"] = swapped_pack
    swapped_pack_proof["engram_pack_receipt_exact_sha256"] = digest_bytes(
        canonical(swapped_pack) + b"\n"
    )
    swapped_pack_proof["engram_pack_receipt_sha256"] = swapped_pack["receipt_sha256"]
    swapped_pack_repository = swapped_pack["engram_repository"]
    swapped_pack_tool = swapped_pack["engram_tool"]
    swapped_pack_proof["engram_commit"] = swapped_pack_repository["commit"]
    swapped_pack_proof["engram_tree"] = swapped_pack_repository["tree"]
    swapped_pack_proof["engram_origin_main"] = swapped_pack_repository["origin_main"]
    swapped_pack_proof["engram_extension_tool_sha256"] = swapped_pack_tool["sha256"]
    swapped_pack_proof["engram_extension_tool_git_blob"] = swapped_pack_tool["git_blob"]
    reseal_installed_outer_fixture(swapped_pack_capture, swapped_pack_index)
    add_package_control(
        "swapped individually valid Engram pack receipt",
        swapped_pack_capture,
        swapped_pack_index,
    )
    controls.append(
        (
            "swapped individually valid package proof and evidence index",
            lambda: validate_installed_package_proof(
                package_capture,
                secondary_package_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    swapped_stage_capture = copy.deepcopy(package_capture)
    swapped_stage_index = copy.deepcopy(package_index)
    swapped_stage_capture["installed_package_proof"]["package_stage_receipt"] = (
        copy.deepcopy(
            secondary_package["installed_package_proof"]["package_stage_receipt"]
        )
    )
    reseal_installed_package_fixture(swapped_stage_capture, swapped_stage_index)
    controls.append(
        (
            "swapped individually valid build and package-stage receipts",
            lambda: validate_installed_package_proof(
                swapped_stage_capture,
                swapped_stage_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    hostile_command_capture = copy.deepcopy(package_capture)
    hostile_command_index = copy.deepcopy(package_index)
    hostile_command_capture["installed_package_proof"]["observed_build_receipt"][
        "cargo"
    ]["argv"][-1] = "hostile-target-directory"
    reseal_installed_package_fixture(hostile_command_capture, hostile_command_index)
    controls.append(
        (
            "coherently resealed build command",
            lambda: validate_installed_package_proof(
                hostile_command_capture,
                hostile_command_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    hostile_generator_capture = copy.deepcopy(package_capture)
    hostile_generator_index = copy.deepcopy(package_index)
    hostile_generator_capture["installed_package_proof"]["observed_build_receipt"][
        "generator"
    ]["files"][0]["relative_path"] = "scripts/hostile-builder.py"
    reseal_installed_package_fixture(
        hostile_generator_capture,
        hostile_generator_index,
    )
    controls.append(
        (
            "coherently resealed build-generator path",
            lambda: validate_installed_package_proof(
                hostile_generator_capture,
                hostile_generator_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    hostile_stage_path_capture = copy.deepcopy(package_capture)
    hostile_stage_path_index = copy.deepcopy(package_index)
    hostile_stage_path_capture["installed_package_proof"]["package_stage_receipt"][
        "package_inventory"
    ][0]["relative_path"] = "bin/hostile-managed-simulation"
    reseal_installed_package_fixture(
        hostile_stage_path_capture,
        hostile_stage_path_index,
    )
    controls.append(
        (
            "coherently resealed staged executable path",
            lambda: validate_installed_package_proof(
                hostile_stage_path_capture,
                hostile_stage_path_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    hostile_build_authority_capture = copy.deepcopy(package_capture)
    hostile_build_authority_index = copy.deepcopy(package_index)
    hostile_build_authority_capture["installed_package_proof"][
        "observed_build_receipt"
    ]["authority"]["execution"] = True
    reseal_installed_package_fixture(
        hostile_build_authority_capture,
        hostile_build_authority_index,
    )
    controls.append(
        (
            "coherently resealed observed-build authority",
            lambda: validate_installed_package_proof(
                hostile_build_authority_capture,
                hostile_build_authority_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    hostile_stage_authority_capture = copy.deepcopy(package_capture)
    hostile_stage_authority_index = copy.deepcopy(package_index)
    hostile_stage_authority_capture["installed_package_proof"]["package_stage_receipt"][
        "authority"
    ]["installation"] = True
    reseal_installed_package_fixture(
        hostile_stage_authority_capture,
        hostile_stage_authority_index,
    )
    controls.append(
        (
            "coherently resealed package-stage authority",
            lambda: validate_installed_package_proof(
                hostile_stage_authority_capture,
                hostile_stage_authority_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    stale_proof_capture = copy.deepcopy(package_capture)
    stale_proof_index = copy.deepcopy(package_index)
    stale_proof_capture["installed_package_proof"]["schema_version"] = (
        "crebain.standard-v3-installed-binary-proof.v2"
    )
    reseal_installed_package_fixture(stale_proof_capture, stale_proof_index)
    controls.append(
        (
            "historical installed-package proof v2",
            lambda: validate_installed_package_proof(
                stale_proof_capture,
                stale_proof_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    forged_source_capture = copy.deepcopy(package_capture)
    forged_source_index = copy.deepcopy(package_index)
    forged_source_capture["installed_package_proof"]["observed_build_receipt"][
        "source"
    ]["files"][0]["sha256"] = "f" * 64
    forged_source_capture["installed_package_proof"]["receipt_sha256"] = digest_without(
        forged_source_capture["installed_package_proof"],
        "receipt_sha256",
    )
    controls.append(
        (
            "forged committed-source digest",
            lambda: validate_installed_package_proof(
                forged_source_capture,
                forged_source_index,
                expected_crebain,
                expected_package_engram,
                tool_sources,
                verify_source_bytes=False,
            ),
        )
    )
    hostile_store = copy.deepcopy(store_capture)
    hostile_store_index = copy.deepcopy(store_index)
    hostile_store_closure = hostile_store["receipt_store_closure"]
    hostile_store_closure["files"][0]["size_bytes"] += 1
    hostile_store_closure["total_bytes"] = sum(
        row["size_bytes"] for row in hostile_store_closure["files"]
    )
    hostile_store_closure["closure_sha256"] = digest_without(
        hostile_store_closure,
        "closure_sha256",
    )
    hostile_store_index["receipt_store_closure_sha256"] = hostile_store_closure[
        "closure_sha256"
    ]
    controls.append(
        (
            "coherently resealed receipt-store byte size",
            lambda: validate_receipt_store_closure(
                hostile_store,
                hostile_store_index,
            ),
        )
    )
    hostile_store_path = copy.deepcopy(store_capture)
    hostile_store_path_index = copy.deepcopy(store_index)
    hostile_path_closure = hostile_store_path["receipt_store_closure"]
    (
        hostile_path_closure["receipt_artifact_path"],
        hostile_path_closure["evidence_artifact_path"],
    ) = (
        hostile_path_closure["evidence_artifact_path"],
        hostile_path_closure["receipt_artifact_path"],
    )
    hostile_path_closure["closure_sha256"] = digest_without(
        hostile_path_closure,
        "closure_sha256",
    )
    hostile_store_path_index["receipt_store_closure_sha256"] = hostile_path_closure[
        "closure_sha256"
    ]
    controls.append(
        (
            "coherently resealed receipt-store artifact swap",
            lambda: validate_receipt_store_closure(
                hostile_store_path,
                hostile_store_path_index,
            ),
        )
    )
    missing_store_metadata = copy.deepcopy(store_capture)
    missing_store_metadata_index = copy.deepcopy(store_index)
    missing_store_metadata["receipt_store_closure"]["files"] = [
        row
        for row in missing_store_metadata["receipt_store_closure"]["files"]
        if row["relative_path"] != "store.json"
    ]
    reseal_receipt_store_fixture(
        missing_store_metadata,
        missing_store_metadata_index,
    )
    controls.append(
        (
            "receipt store missing store metadata",
            lambda capture=missing_store_metadata, index=missing_store_metadata_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    missing_store_lock = copy.deepcopy(store_capture)
    missing_store_lock_index = copy.deepcopy(store_index)
    missing_store_lock["receipt_store_closure"]["files"] = [
        row
        for row in missing_store_lock["receipt_store_closure"]["files"]
        if row["relative_path"] != "writer.lock"
    ]
    reseal_receipt_store_fixture(missing_store_lock, missing_store_lock_index)
    controls.append(
        (
            "receipt store missing writer lock",
            lambda capture=missing_store_lock, index=missing_store_lock_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    legacy_store_metadata = copy.deepcopy(store_capture)
    legacy_store_metadata_index = copy.deepcopy(store_index)
    legacy_store_closure = legacy_store_metadata["receipt_store_closure"]
    legacy_store_payload = canonical(
        {
            "schema_version": "engram.extension-closed-loop-receipt-store.v4",
            "policy": "engram.extension-closed-loop-receipt-store-policy.v4",
            "store_id": legacy_store_closure["store_id"],
            "digest_canonicalization": RECEIPT_STORE_CANONICALIZATION,
            "execution_authority": False,
            "ncp_control": False,
            "physical_actuation": False,
            "scientific_authority": False,
            "is_paper_local_evidence": False,
            "calibrated_posterior": False,
        }
    )
    legacy_store_row = next(
        row
        for row in legacy_store_closure["files"]
        if row["relative_path"] == "store.json"
    )
    legacy_store_row.update(
        {
            "size_bytes": len(legacy_store_payload),
            "sha256": digest_bytes(legacy_store_payload),
        }
    )
    reseal_receipt_store_fixture(legacy_store_metadata, legacy_store_metadata_index)
    controls.append(
        (
            "receipt store legacy metadata policy",
            lambda capture=legacy_store_metadata, index=legacy_store_metadata_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    hostile_store_lock = copy.deepcopy(store_capture)
    hostile_store_lock_index = copy.deepcopy(store_index)
    hostile_lock_payload = b"engram-extension-closed-loop-receipt-store-lock-v2\n"
    hostile_lock_row = next(
        row
        for row in hostile_store_lock["receipt_store_closure"]["files"]
        if row["relative_path"] == "writer.lock"
    )
    hostile_lock_row.update(
        {
            "size_bytes": len(hostile_lock_payload),
            "sha256": digest_bytes(hostile_lock_payload),
        }
    )
    reseal_receipt_store_fixture(hostile_store_lock, hostile_store_lock_index)
    controls.append(
        (
            "receipt store writer-lock payload drift",
            lambda capture=hostile_store_lock, index=hostile_store_lock_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    arbitrary_store_row = copy.deepcopy(store_capture)
    arbitrary_store_row_index = copy.deepcopy(store_index)
    arbitrary_row = next(
        row
        for row in arbitrary_store_row["receipt_store_closure"]["files"]
        if row["relative_path"].startswith("observations/")
    )
    arbitrary_row["relative_path"] = "arbitrary/opaque.json"
    reseal_receipt_store_fixture(arbitrary_store_row, arbitrary_store_row_index)
    controls.append(
        (
            "receipt store arbitrary sidecar row",
            lambda capture=arbitrary_store_row, index=arbitrary_store_row_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    wrong_reservation_path = copy.deepcopy(store_capture)
    wrong_reservation_path_index = copy.deepcopy(store_index)
    reservation_row = next(
        row
        for row in wrong_reservation_path["receipt_store_closure"]["files"]
        if row["relative_path"].startswith("finalized-reservations/")
    )
    reservation_row["relative_path"] = reservation_row["relative_path"].replace(
        "finalized-reservations/66/",
        "finalized-reservations/ff/",
        1,
    )
    reseal_receipt_store_fixture(
        wrong_reservation_path,
        wrong_reservation_path_index,
    )
    controls.append(
        (
            "receipt store reservation-shard drift",
            lambda capture=wrong_reservation_path, index=wrong_reservation_path_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    wrong_admission_path = copy.deepcopy(store_capture)
    wrong_admission_path_index = copy.deepcopy(store_index)
    admission_row = next(
        row
        for row in wrong_admission_path["receipt_store_closure"]["files"]
        if row["relative_path"].startswith("publication-admission-anchors/")
    )
    admission_row["relative_path"] = (
        "publication-admission-anchors/" + "0" * 64 + ".json"
    )
    reseal_receipt_store_fixture(wrong_admission_path, wrong_admission_path_index)
    controls.append(
        (
            "receipt store admission-key drift",
            lambda capture=wrong_admission_path, index=wrong_admission_path_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    duplicated_sidecar_bytes = copy.deepcopy(store_capture)
    duplicated_sidecar_bytes_index = copy.deepcopy(store_index)
    sidecar_rows = [
        row
        for row in duplicated_sidecar_bytes["receipt_store_closure"]["files"]
        if row["relative_path"].startswith(
            ("observations/", "publication-authorities/")
        )
    ]
    sidecar_rows[0]["sha256"] = sidecar_rows[1]["sha256"]
    reseal_receipt_store_fixture(
        duplicated_sidecar_bytes,
        duplicated_sidecar_bytes_index,
    )
    controls.append(
        (
            "receipt store duplicated sidecar bytes",
            lambda capture=duplicated_sidecar_bytes,
            index=duplicated_sidecar_bytes_index: validate_receipt_store_closure(
                capture,
                index,
            ),
        )
    )
    non_addressed_store = copy.deepcopy(store_capture)
    non_addressed_store_index = copy.deepcopy(store_index)
    non_addressed_closure = non_addressed_store["receipt_store_closure"]
    old_receipt_path = non_addressed_closure["receipt_artifact_path"]
    new_receipt_path = f"receipts/ff/{non_addressed_closure['receipt_sha256']}.json"
    non_addressed_closure["receipt_artifact_path"] = new_receipt_path
    next(
        row
        for row in non_addressed_closure["files"]
        if row["relative_path"] == old_receipt_path
    )["relative_path"] = new_receipt_path
    reseal_receipt_store_fixture(non_addressed_store, non_addressed_store_index)
    controls.append(
        (
            "receipt store non-content-addressed path",
            lambda capture=non_addressed_store, index=non_addressed_store_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    forged_store_row = copy.deepcopy(store_capture)
    forged_store_row_index = copy.deepcopy(store_index)
    forged_store_closure = forged_store_row["receipt_store_closure"]
    next(
        row
        for row in forged_store_closure["files"]
        if row["relative_path"] == forged_store_closure["receipt_artifact_path"]
    )["sha256"] = "0" * 64
    reseal_receipt_store_fixture(forged_store_row, forged_store_row_index)
    controls.append(
        (
            "receipt store artifact row digest drift",
            lambda capture=forged_store_row, index=forged_store_row_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    duplicated_store_artifact = copy.deepcopy(store_capture)
    duplicated_store_artifact_index = copy.deepcopy(store_index)
    duplicated_store_closure = duplicated_store_artifact["receipt_store_closure"]
    receipt_row = next(
        row
        for row in duplicated_store_closure["files"]
        if row["relative_path"] == duplicated_store_closure["receipt_artifact_path"]
    )
    duplicated_store_closure["files"].append(
        {**receipt_row, "relative_path": "mirrors/terminal-receipt.json"}
    )
    reseal_receipt_store_fixture(
        duplicated_store_artifact,
        duplicated_store_artifact_index,
    )
    controls.append(
        (
            "receipt store duplicated terminal artifact bytes",
            lambda capture=duplicated_store_artifact,
            index=duplicated_store_artifact_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    forged_store_identity = copy.deepcopy(store_capture)
    forged_store_identity_index = copy.deepcopy(store_index)
    forged_store_identity["summary"]["store_id"] = "clrs_" + "0" * 64
    controls.append(
        (
            "receipt store summary identity drift",
            lambda capture=forged_store_identity, index=forged_store_identity_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    forged_terminal_digest = copy.deepcopy(store_capture)
    forged_terminal_digest_index = copy.deepcopy(store_index)
    forged_terminal_closure = forged_terminal_digest["receipt_store_closure"]
    old_receipt_path = forged_terminal_closure["receipt_artifact_path"]
    new_receipt_sha256 = "0" * 64
    new_receipt_path = f"receipts/00/{new_receipt_sha256}.json"
    forged_terminal_digest["terminal_receipt"]["receipt_sha256"] = new_receipt_sha256
    forged_terminal_closure["receipt_sha256"] = new_receipt_sha256
    forged_terminal_closure["receipt_artifact_path"] = new_receipt_path
    forged_terminal_row = next(
        row
        for row in forged_terminal_closure["files"]
        if row["relative_path"] == old_receipt_path
    )
    forged_terminal_row["relative_path"] = new_receipt_path
    forged_terminal_row["sha256"] = new_receipt_sha256
    reseal_receipt_store_fixture(
        forged_terminal_digest,
        forged_terminal_digest_index,
    )
    controls.append(
        (
            "coherently resealed terminal self-digest drift",
            lambda capture=forged_terminal_digest, index=forged_terminal_digest_index: (
                validate_receipt_store_closure(capture, index)
            ),
        )
    )
    hostile_topology = copy.deepcopy(topology_capture)
    hostile_topology["population_topology"]["population_count"] += 1
    controls.append(
        (
            "population topology drift",
            lambda: validate_population_topology(
                hostile_topology,
                topology_index,
                topology_roster,
            ),
        )
    )
    hostile_population = copy.deepcopy(topology_capture)
    hostile_name = "fleet.channel-01.d00.hostile"
    hostile_population["population_topology"]["population_names"][0] = hostile_name
    hostile_population["population_topology"]["derived_population_roster_sha256"] = (
        digest_bytes(
            canonical(hostile_population["population_topology"]["population_names"])
        )
    )
    hostile_session = hostile_population["nest_evidence_bundle"][
        "nest_session_readback"
    ]
    hostile_session["population_roster"][0]["population_names"][0] = hostile_name
    hostile_session["population_roster_sha256"] = digest_bytes(
        canonical(hostile_session["population_roster"])
    )
    hostile_session["work_admission"]["expected_population_roster_sha256"] = (
        hostile_session["population_roster_sha256"]
    )
    for connection in hostile_session["connection_readbacks"][:2]:
        connection["population_name"] = hostile_name
    hostile_session["connection_readback_sha256"] = digest_bytes(
        canonical(hostile_session["connection_readbacks"])
    )
    hostile_execution = hostile_population["nest_evidence_bundle"][
        "step_execution_receipts"
    ][0]
    for rows_field, digest_field in (
        ("generator_schedule_readbacks", "generator_schedule_readback_sha256"),
        ("input_weight_readbacks", "input_weight_readback_sha256"),
        ("completed_window_readbacks", "completed_window_readback_sha256"),
    ):
        hostile_execution[rows_field][0]["population_name"] = hostile_name
        hostile_execution[digest_field] = digest_bytes(
            canonical(hostile_execution[rows_field])
        )
    hostile_execution["population_event_deltas"][0]["population_name"] = hostile_name
    hostile_execution["receipt_sha256"] = digest_without(
        hostile_execution,
        "receipt_sha256",
    )
    hostile_population["neural_steps"][0]["result"]["proposals"][0][
        "source_populations"
    ][0] = hostile_name
    controls.append(
        (
            "coherently resealed same-count NEST population rename",
            lambda: validate_population_topology(
                hostile_population,
                topology_index,
                topology_roster,
            ),
        )
    )
    hostile_guardian = copy.deepcopy(guardian_capture)
    hostile_guardian["nest_worker_guardian_closure"]["child_reaped"] = False
    controls.append(
        (
            "worker guardian not reaped",
            lambda: validate_worker_guardian_closure(hostile_guardian),
        )
    )
    controls.append(
        (
            "historical evidence-index v1",
            lambda: require_index_schema_version(
                {"schema_version": AUDIT_ONLY_INDEX_SCHEMA_VERSION}
            ),
        )
    )
    controls.append(
        (
            "unknown evidence-index version",
            lambda: require_index_schema_version(
                {"schema_version": "crebain.real-nest-closed-loop-evidence-index.v3"}
            ),
        )
    )
    hostile_response = {
        "authority": "read-only-observer",
        "roster_authority": "host-declared-projection",
        "source_roster_authenticated": False,
        "descriptive_only": True,
        "agent_bridge_command": True,
        "physical_actuation": False,
        "ncp_used": False,
        "pid_result": False,
        "source_durable_evidence_verified": False,
        "scientific_authority": False,
        "is_paper_local_evidence": False,
        "calibrated_posterior": False,
    }
    controls.append(
        (
            "observer Agent Bridge authority",
            lambda: observer_response_boundary(hostile_response),
        )
    )
    hostile_build = copy.deepcopy(build_document)
    hostile_build["unreviewed"] = True
    hostile_build["receipt_sha256"] = observed_build.digest_without(
        hostile_build,
        "receipt_sha256",
    )
    controls.append(
        (
            "observed-build receipt open field",
            lambda hostile_build=hostile_build: validate_observed_build_fixture(
                hostile_build,
                build_fixture,
            ),
        )
    )
    hostile_build = copy.deepcopy(build_document)
    hostile_build["disclosure"] = ""
    hostile_build["receipt_sha256"] = observed_build.digest_without(
        hostile_build,
        "receipt_sha256",
    )
    controls.append(
        (
            "observed-build empty text",
            lambda hostile_build=hostile_build: validate_observed_build_fixture(
                hostile_build,
                build_fixture,
            ),
        )
    )
    hostile_build = copy.deepcopy(build_document)
    hostile_build["disclosure"] = "x" * 4097
    hostile_build["receipt_sha256"] = observed_build.digest_without(
        hostile_build,
        "receipt_sha256",
    )
    controls.append(
        (
            "observed-build oversized text",
            lambda hostile_build=hostile_build: validate_observed_build_fixture(
                hostile_build,
                build_fixture,
            ),
        )
    )
    hostile_build = copy.deepcopy(build_document)
    hostile_build["source"]["source_roster"][0]["sha256"] = "f" * 64
    hostile_build["source"]["source_roster_sha256"] = digest_bytes(
        b"prisoma-observer-build-source-roster-v1\0"
        + canonical(hostile_build["source"]["source_roster"])
    )
    hostile_build["receipt_sha256"] = observed_build.digest_without(
        hostile_build,
        "receipt_sha256",
    )
    controls.append(
        (
            "coherently resealed observed source swap",
            lambda hostile_build=hostile_build: validate_observed_build_fixture(
                hostile_build,
                build_fixture,
            ),
        )
    )
    hostile_build = copy.deepcopy(build_document)
    hostile_build["toolchain"]["cargo"]["sha256"] = "f" * 64
    hostile_build["receipt_sha256"] = observed_build.digest_without(
        hostile_build,
        "receipt_sha256",
    )
    controls.append(
        (
            "coherently resealed observed toolchain swap",
            lambda hostile_build=hostile_build: validate_observed_build_fixture(
                hostile_build,
                build_fixture,
            ),
        )
    )
    hostile_build = copy.deepcopy(build_document)
    hostile_build["artifact"]["path"] = (
        "crates/engram-managed-observer/target/release/spoofed-observer"
    )
    hostile_build["receipt_sha256"] = observed_build.digest_without(
        hostile_build,
        "receipt_sha256",
    )
    controls.append(
        (
            "coherently resealed observed artifact path spoof",
            lambda hostile_build=hostile_build: validate_observed_build_fixture(
                hostile_build,
                build_fixture,
            ),
        )
    )
    hostile_build = copy.deepcopy(build_document)
    hostile_build["build"]["argv"][-1] = str(
        (CRATE_MANIFEST.parent / ".observed-build-path-spoof").absolute()
    )
    hostile_build["receipt_sha256"] = observed_build.digest_without(
        hostile_build,
        "receipt_sha256",
    )
    controls.append(
        (
            "coherently resealed observed target-directory path spoof",
            lambda hostile_build=hostile_build: validate_observed_build_fixture(
                hostile_build,
                build_fixture,
            ),
        )
    )
    hostile_build = copy.deepcopy(build_document)
    hostile_build["artifact"]["sha256"] = "f" * 64
    hostile_build["receipt_sha256"] = observed_build.digest_without(
        hostile_build,
        "receipt_sha256",
    )
    controls.append(
        (
            "coherently resealed observed receipt artifact swap",
            lambda hostile_build=hostile_build: validate_observed_build_fixture(
                hostile_build,
                build_fixture,
            ),
        )
    )
    wrong_arch = bytearray(build_fixture["macho_payload"])
    struct.pack_into("<I", wrong_arch, 4, 0x01000007)
    controls.append(
        (
            "wrong-architecture observed release bytes",
            lambda wrong_arch=bytes(wrong_arch): observed_build.parse_arm64_macho(
                wrong_arch
            ),
        )
    )
    with tempfile.TemporaryDirectory(
        prefix="prisoma-observer-matrix-hostile-",
        dir=temporary_parent,
    ) as raw:
        temporary = Path(raw)
        if (
            require_canonical_directory(
                temporary,
                "private temporary-directory positive control",
            )
            != temporary
        ):
            raise MatrixError("private temporary-directory positive control differs")
        alias_parent = temporary / "alias-parent"
        alias_parent.mkdir(mode=0o700)
        lexical_alias = alias_parent / ".."
        symlink_alias = temporary / "temporary-alias"
        symlink_alias.symlink_to(temporary, target_is_directory=True)
        controls.extend(
            (
                (
                    "lexically aliased temporary directory",
                    lambda: require_canonical_directory(
                        lexical_alias,
                        "hostile temporary directory",
                    ),
                ),
                (
                    "symlink-aliased temporary directory",
                    lambda: require_canonical_directory(
                        symlink_alias,
                        "hostile temporary directory",
                    ),
                ),
            )
        )
        promoted_binary = temporary / "chmod-promoted-observer"
        promoted_binary.write_bytes(b"#!/bin/sh\nexit 0\n")
        os.chmod(promoted_binary, 0o700)
        controls.append(
            (
                "chmod-promoted arbitrary release bytes",
                lambda: observed_build.parse_arm64_macho(
                    snapshot_regular_file(
                        promoted_binary,
                        observed_build.MAX_BINARY_BYTES,
                    )
                ),
            )
        )
        link = temporary / "capture.json"
        link.symlink_to(SOURCE_RECEIPT)
        controls.append(
            (
                "symlink capture input",
                lambda: snapshot_regular_file(link, MAX_CAPTURE_BYTES),
            )
        )
        for label, action in controls:
            expect_rejected(label, action)
    print(
        "OK: release observer accepted 1/2/3 rosters and matrix review rejected "
        f"{len(controls)} hostile controls"
    )
    return 0


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--binary-build-receipt", type=Path)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--crebain-root", type=Path)
    parser.add_argument("--engram-root", type=Path)
    parser.add_argument("--expected-prisoma-revision")
    parser.add_argument("--expected-crebain-source-revision")
    parser.add_argument("--expected-crebain-publication-revision")
    parser.add_argument("--expected-engram-revision")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--verify", action="store_true")
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    try:
        production_values = (
            arguments.binary_build_receipt,
            arguments.crebain_root,
            arguments.engram_root,
            arguments.expected_prisoma_revision,
            arguments.expected_crebain_source_revision,
            arguments.expected_crebain_publication_revision,
            arguments.expected_engram_revision,
            arguments.output,
        )
        if arguments.self_test:
            if (
                any(value is not None for value in production_values)
                or arguments.verify
            ):
                raise MatrixError(
                    "self-test does not accept production review arguments"
                )
            return self_test(arguments.binary)
        if any(value is None for value in production_values):
            raise MatrixError(
                "production matrix review requires every source and output"
            )
        _document, payload = build_matrix(
            binary=arguments.binary,
            binary_build_receipt=arguments.binary_build_receipt,
            crebain_root=arguments.crebain_root,
            engram_root=arguments.engram_root,
            expected_prisoma_revision=arguments.expected_prisoma_revision,
            expected_crebain_source_revision=(
                arguments.expected_crebain_source_revision
            ),
            expected_crebain_publication_revision=(
                arguments.expected_crebain_publication_revision
            ),
            expected_engram_revision=arguments.expected_engram_revision,
        )
        output = arguments.output.absolute()
        if output.parent.resolve(strict=True) != output.parent:
            raise MatrixError("matrix output parent traverses a link")
        if arguments.verify:
            if snapshot_regular_file(output, MAX_MATRIX_OUTPUT_BYTES) != payload:
                raise MatrixError("CREBAIN observer matrix receipt differs")
            print(f"OK: verified CREBAIN observer matrix at {output}")
            return 0
        write_new(output, payload)
        print(f"OK: wrote CREBAIN observer matrix to {output}")
        return 0
    except (MatrixError, OSError, subprocess.TimeoutExpired) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
