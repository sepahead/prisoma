#!/usr/bin/env python3
"""Validate the honest, intentionally non-promotable M0 v1 governance bundle.

The five files audited here are policy records, not empirical evidence.  The default
audit accepts the checked-in, deliberately unfinished scaffold.  ``--require-freeze-ready``
adds the stronger pre-unblinding readiness check and reports stable blocker codes instead
of pretending that missing scientific choices have already been made.  V1 cannot be made
freeze-ready by filling its nulls in place: a real freeze requires a reviewed schema and
validator revision with typed, content-bound receipts.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
import stat
import sys
from datetime import date, datetime
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
PROTOCOLS = Path("protocols")
PREREGISTRATION_PATH = PROTOCOLS / "m0_preregistration_skeleton_v1.json"
HOLDOUT_REGISTRY_PATH = PROTOCOLS / "holdout_registry_v1.json"
HOLDOUT_LEDGER_PATH = PROTOCOLS / "holdout_access_ledger_v1.jsonl"
TRANSPORT_PATH = PROTOCOLS / "transport_contamination_ledger_v1.json"
LITERATURE_PATH = PROTOCOLS / "literature_screening_ledger_v1.json"
CLAIM_REGISTRY_PATH = PROTOCOLS / "research_claim_registry_v1.json"

FREEZE_BLOCKERS = [
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

# These hashes pin exact reviewed honesty-bearing prose/subtrees without copying hundreds
# of words into validation code.  Each digest is SHA-256 over compact, sorted-key UTF-8 JSON.
SEMANTIC_SNAPSHOT_SHA256 = {
    "prereg_scope": "7eb19d1f03a7790e78d3871dfc501c9ca2444371170300f1aeca84e76f942ef2",
    "prereg_interpretation_boundaries": (
        "d37032290e677135fb944ae51c67f66cb27b25d1208fdca443946b257bd99163"
    ),
    "h3_gate_binding_rule": (
        "5a2f4cade173ca33949a88d165775ead489bf7630e97e938c0aeb61552dfde73"
    ),
    "freeze_requirements": (
        "cbd1a0cdb9a714477c507724ec8b5b01c711d34753c8d061f7e3972d0e15622b"
    ),
    "transport_scope": "b9f9c0ae574ec4ab8c308db7c9546805599d760c6049a7a180e8f400b5faf894",
    "transport_completion_boundary": (
        "c38415a745f07474e1fe960c12b0cbb23454f36acf08b30c5e93908a3056c080"
    ),
    "transport_dimensions": (
        "6b2704f0dc64611942cfbc599d008f2b2eece604f2292943e6f02f79b98d4a68"
    ),
    "literature_scope": (
        "9facb7dbcdf7bc69af8fd26fd36fee2f936679db638bd06969fa2a47323c518b"
    ),
    "literature_inventory_role": (
        "3bf43f0aa878ea646cbd9e76b9de3d4cb058d7985e4c589c1560415d23a375e5"
    ),
    "literature_legacy_interpretation": (
        "82a4f09f91e9a451cb2bed528ab459236489d81f42bf4583a3bff6f699d07a22"
    ),
    "literature_endpoint_policy": (
        "5518416bc01d57b6a3e564919502409a49f2cc911750d9dabd2f965b091dac59"
    ),
    "literature_comparator_scope": (
        "2eddbee32eca20df02c476c998f5c5f29e6ef74b14306e1bdd725261c74ff7b1"
    ),
    "literature_family_descriptions": (
        "4e018cae133bb8676096d25835d467a3605b96d2ce636b57e32a6ba3a84aff26"
    ),
    "literature_requirements_limits": (
        "a6225ee80b287bca853c469778fb2690cf7feaf3065eeb1c3ef480cf8d520ad0"
    ),
    "claim_registry_scope": (
        "3d0c81c164f99b491aea8f2ddae889ffceef93bf52956f1f251e15db7944e638"
    ),
    "claim_snapshots": "a829e848ed3d5a392b07b31d0c987be428f843ab6f6dcea679e45fe94964b3b2",
}

SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
PLACEHOLDER_RE = re.compile(
    r"(?:\b(?:tbd|todo|tk|changeme|placeholder|fixme)\b|<[^>]+>|\?\?\?)",
    re.IGNORECASE,
)
HOLDOUT_HASH_DOMAIN = b"prisoma-holdout-access-v1\0"
FREEZE_BLOCKED_EXIT = 3


class GovernanceError(ValueError):
    """The M0 governance bundle is malformed, unsafe, or internally dishonest."""


def _reject_constant(token: str) -> None:
    raise GovernanceError(f"non-finite JSON number {token!r} is forbidden")


def _object_without_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise GovernanceError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def _parse_json_bytes(raw: bytes, *, context: str) -> Any:
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise GovernanceError(f"{context} is not UTF-8") from error
    try:
        value = json.loads(
            text,
            object_pairs_hook=_object_without_duplicate_keys,
            parse_constant=_reject_constant,
        )
    except GovernanceError:
        raise
    except json.JSONDecodeError as error:
        raise GovernanceError(
            f"{context} is invalid JSON at line {error.lineno}, column {error.colno}"
        ) from error
    _reject_placeholders_and_nonfinite(value, context=context)
    return value


def _reject_placeholders_and_nonfinite(value: Any, *, context: str) -> None:
    if isinstance(value, dict):
        for key, item in value.items():
            if not isinstance(key, str):
                raise GovernanceError(f"{context} contains a non-string object key")
            _reject_surrogates(key, context=f"{context} key")
            _reject_placeholders_and_nonfinite(item, context=f"{context}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            _reject_placeholders_and_nonfinite(item, context=f"{context}[{index}]")
    elif isinstance(value, str):
        _reject_surrogates(value, context=context)
        if PLACEHOLDER_RE.search(value):
            raise GovernanceError(f"{context} contains a placeholder token")
    elif isinstance(value, float) and not math.isfinite(value):
        raise GovernanceError(f"{context} contains a non-finite number")


def _exact_keys(value: Any, expected: set[str], *, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise GovernanceError(f"{context} must be an object")
    missing = sorted(expected - value.keys())
    unknown = sorted(value.keys() - expected)
    if missing or unknown:
        details: list[str] = []
        if missing:
            details.append(f"missing={missing}")
        if unknown:
            details.append(f"unknown={unknown}")
        raise GovernanceError(f"{context} has invalid fields: {', '.join(details)}")
    return value


def _string(value: Any, *, context: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise GovernanceError(f"{context} must be a non-empty string")
    return value


def _reject_surrogates(value: str, *, context: str) -> None:
    if any(0xD800 <= ord(character) <= 0xDFFF for character in value):
        raise GovernanceError(f"{context} contains a Unicode surrogate code point")


def _enum(value: Any, allowed: set[str], *, context: str) -> str:
    text = _string(value, context=context)
    if text not in allowed:
        raise GovernanceError(f"{context} has unknown value {text!r}")
    return text


def _boolean(value: Any, *, context: str) -> bool:
    if not isinstance(value, bool):
        raise GovernanceError(f"{context} must be a boolean")
    return value


def _integer(value: Any, *, context: str, minimum: int | None = None) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise GovernanceError(f"{context} must be an integer, not a boolean")
    if minimum is not None and value < minimum:
        raise GovernanceError(f"{context} must be >= {minimum}")
    return value


def _array(value: Any, *, context: str) -> list[Any]:
    if not isinstance(value, list):
        raise GovernanceError(f"{context} must be an array")
    return value


def _string_array(
    value: Any,
    *,
    context: str,
    allow_empty: bool = True,
    unique: bool = True,
) -> list[str]:
    items = _array(value, context=context)
    if not allow_empty and not items:
        raise GovernanceError(f"{context} must not be empty")
    strings = [_string(item, context=f"{context}[{index}]") for index, item in enumerate(items)]
    if unique and len(strings) != len(set(strings)):
        raise GovernanceError(f"{context} must not contain duplicates")
    return strings


def _timestamp(value: Any, *, context: str) -> datetime:
    text = _string(value, context=context)
    if not text.endswith("Z"):
        raise GovernanceError(f"{context} must be an RFC 3339 UTC timestamp ending in Z")
    try:
        parsed = datetime.fromisoformat(text[:-1] + "+00:00")
    except ValueError as error:
        raise GovernanceError(f"{context} is not a valid RFC 3339 timestamp") from error
    if parsed.isoformat().replace("+00:00", "Z") != text:
        raise GovernanceError(f"{context} must use canonical RFC 3339 UTC form")
    return parsed


def _date(value: Any, *, context: str) -> date:
    text = _string(value, context=context)
    try:
        parsed = date.fromisoformat(text)
    except ValueError as error:
        raise GovernanceError(f"{context} is not a valid ISO date") from error
    if parsed.isoformat() != text:
        raise GovernanceError(f"{context} must use canonical YYYY-MM-DD form")
    return parsed


def _safe_file(root: Path, relative: Path | str, *, context: str) -> Path:
    raw = relative.as_posix() if isinstance(relative, Path) else relative
    if not isinstance(raw, str) or not raw:
        raise GovernanceError(f"{context} must be a repository-relative path")
    if any(ord(character) < 32 or ord(character) == 127 for character in raw):
        raise GovernanceError(f"{context} contains a forbidden ASCII control character")
    if "\\" in raw:
        raise GovernanceError(f"{context} must use POSIX path separators")
    pure = PurePosixPath(raw)
    if pure.is_absolute() or not pure.parts or ".." in pure.parts or "." in pure.parts:
        raise GovernanceError(f"{context} escapes the repository: {raw!r}")

    root_resolved = root.resolve(strict=True)
    current = root
    for part in pure.parts:
        current /= part
        try:
            mode = current.lstat().st_mode
        except FileNotFoundError as error:
            raise GovernanceError(f"{context} does not exist: {raw!r}") from error
        if stat.S_ISLNK(mode):
            raise GovernanceError(f"{context} may not traverse a symlink: {raw!r}")

    resolved = current.resolve(strict=True)
    try:
        resolved.relative_to(root_resolved)
    except ValueError as error:
        raise GovernanceError(f"{context} resolves outside the repository: {raw!r}") from error
    if not resolved.is_file():
        raise GovernanceError(f"{context} must resolve to a regular file: {raw!r}")
    return resolved


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def _canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def _require_semantic_snapshot(value: Any, *, snapshot: str, context: str) -> None:
    digest = hashlib.sha256(_canonical_json_bytes(value)).hexdigest()
    if digest != SEMANTIC_SNAPSHOT_SHA256[snapshot]:
        raise GovernanceError(f"{context} differs from its exact reviewed semantic snapshot")


def _read_json(root: Path, relative: Path) -> tuple[dict[str, Any], Path]:
    path = _safe_file(root, relative, context=relative.as_posix())
    value = _parse_json_bytes(path.read_bytes(), context=relative.as_posix())
    if not isinstance(value, dict):
        raise GovernanceError(f"{relative.as_posix()} must contain one JSON object")
    return value, path


def _read_jsonl(root: Path, relative: Path) -> tuple[list[dict[str, Any]], Path]:
    path = _safe_file(root, relative, context=relative.as_posix())
    raw = path.read_bytes()
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise GovernanceError(f"{relative.as_posix()} is not UTF-8") from error
    if not text.endswith("\n"):
        raise GovernanceError(f"{relative.as_posix()} must end with a newline")
    lines = text.splitlines()
    if not lines or any(not line.strip() for line in lines):
        raise GovernanceError(f"{relative.as_posix()} must contain non-empty JSONL records")
    records: list[dict[str, Any]] = []
    for index, line in enumerate(lines, 1):
        value = _parse_json_bytes(
            line.encode("utf-8"),
            context=f"{relative.as_posix()} line {index}",
        )
        if not isinstance(value, dict):
            raise GovernanceError(
                f"{relative.as_posix()} line {index} must contain one JSON object"
            )
        records.append(value)
    return records, path


def _validate_sha(value: Any, *, context: str) -> str:
    text = _string(value, context=context)
    if not SHA256_RE.fullmatch(text):
        raise GovernanceError(f"{context} must be a lowercase SHA-256 digest")
    return text


def _validate_bound_file(
    root: Path,
    *,
    raw_path: Any,
    raw_sha256: Any,
    context: str,
    raw_byte_count: Any | None = None,
) -> Path:
    relative = _string(raw_path, context=f"{context}.path")
    path = _safe_file(root, relative, context=f"{context}.path")
    expected_sha256 = _validate_sha(raw_sha256, context=f"{context}.sha256")
    actual_sha256 = _sha256(path)
    if actual_sha256 != expected_sha256:
        raise GovernanceError(
            f"{context} SHA-256 mismatch: declared {expected_sha256}, "
            f"computed {actual_sha256}"
        )
    if raw_byte_count is not None:
        expected_bytes = _integer(
            raw_byte_count,
            context=f"{context}.byte_count",
            minimum=0,
        )
        actual_bytes = path.stat().st_size
        if actual_bytes != expected_bytes:
            raise GovernanceError(
                f"{context} byte count mismatch: declared {expected_bytes}, "
                f"computed {actual_bytes}"
            )
    return path


HOLDOUT_REGISTRY_KEYS = {
    "schema_version",
    "artifact_id",
    "as_of_date",
    "canonical_spec",
    "scope",
    "registry_status",
    "holdout_count",
    "registered_holdouts",
    "access_authorization_policy",
    "authorized_actor_roles",
    "unblinding_policy",
    "access_ledger",
    "limitations",
}
HOLDOUT_CANONICAL_SPEC_KEYS = {"path", "version"}
HOLDOUT_ACCESS_LEDGER_KEYS = {
    "path",
    "file_sha256",
    "file_byte_count",
    "event_count",
    "head_event_sha256",
    "hash_algorithm",
    "domain_separator",
    "canonicalization",
    "chain_rule",
    "payload_boundary",
}
HOLDOUT_SCOPE = (
    "confirmatory holdout registration state and repository-local access-event chain metadata; "
    "not a claim that no off-repository data or outcome exposure has occurred"
)
HOLDOUT_CANONICALIZATION = (
    "for each event, remove event_sha256; encode the remaining object as compact UTF-8 JSON "
    "with lexicographically sorted keys and no ASCII escaping; hash domain_separator bytes "
    "followed immediately by those canonical event bytes"
)
HOLDOUT_CHAIN_RULE = (
    "event_index starts at zero and increments by one; genesis previous_event_sha256 is null; "
    "every later event previous_event_sha256 equals the preceding event_sha256"
)
HOLDOUT_PAYLOAD_BOUNDARY = (
    "events record governance metadata only and must not contain raw holdout labels, sample "
    "identifiers, credentials, secrets, or outcome payloads"
)
HOLDOUT_LIMITATIONS = [
    (
        "the sole non_access_genesis event initializes the chain and does not attest to "
        "historical or off-repository non-access"
    ),
    (
        "only the recording date is known for the genesis event; recorded_at is null and no "
        "immutable or trusted timestamp is claimed"
    ),
    (
        "with no registered holdout, there is no holdout-specific access authorization, split "
        "manifest, custodian, or unblinding procedure to audit"
    ),
    (
        "the chain is unsigned and locally anchored only by reviewed repository history; edits, "
        "truncation, or replacement can be rehashed before review, so this is local tamper "
        "evidence rather than signing or external attestation"
    ),
    "a repository ledger cannot prove that every exposure outside the repository was recorded",
]
HOLDOUT_EVENT_KEYS = {
    "schema_version",
    "event_index",
    "event_type",
    "recorded_on",
    "recorded_at",
    "holdout_id",
    "actor_id",
    "authorization_id",
    "purpose",
    "exposure_occurred",
    "exposure_scope",
    "previous_event_sha256",
    "event_scope",
    "event_sha256",
}


def _holdout_event_digest(event: dict[str, Any]) -> str:
    payload = {key: value for key, value in event.items() if key != "event_sha256"}
    return hashlib.sha256(HOLDOUT_HASH_DOMAIN + _canonical_json_bytes(payload)).hexdigest()


def _validate_holdout_bundle(
    root: Path,
    registry: dict[str, Any],
    ledger: list[dict[str, Any]],
    ledger_path: Path,
) -> None:
    context = HOLDOUT_REGISTRY_PATH.as_posix()
    registry = _exact_keys(registry, HOLDOUT_REGISTRY_KEYS, context=context)
    if _integer(registry["schema_version"], context=f"{context}.schema_version") != 1:
        raise GovernanceError(f"{context}.schema_version must equal 1")
    if registry["artifact_id"] != "prisoma_holdout_registry_v1":
        raise GovernanceError(f"{context}.artifact_id is not the v1 registry identifier")
    _date(registry["as_of_date"], context=f"{context}.as_of_date")
    canonical = _exact_keys(
        registry["canonical_spec"],
        HOLDOUT_CANONICAL_SPEC_KEYS,
        context=f"{context}.canonical_spec",
    )
    if canonical != {"path": "grandplan.md", "version": "12.5"}:
        raise GovernanceError(f"{context}.canonical_spec must identify grandplan.md v12.5")
    _safe_file(root, canonical["path"], context=f"{context}.canonical_spec.path")
    if registry["scope"] != HOLDOUT_SCOPE:
        raise GovernanceError(f"{context}.scope loses the holdout honesty boundary")
    status_value = _enum(
        registry["registry_status"],
        {"no_confirmatory_holdout_registered"},
        context=f"{context}.registry_status",
    )
    count = _integer(
        registry["holdout_count"], context=f"{context}.holdout_count", minimum=0
    )
    holdouts = _array(registry["registered_holdouts"], context=f"{context}.registered_holdouts")
    if count != len(holdouts):
        raise GovernanceError(f"{context}.holdout_count does not match registered_holdouts")
    if status_value == "no_confirmatory_holdout_registered":
        if count != 0 or holdouts:
            raise GovernanceError(
                f"{context} falsely combines no-confirmatory-holdout status with registrations"
            )
        for field in (
            "access_authorization_policy",
            "authorized_actor_roles",
            "unblinding_policy",
        ):
            if registry[field] is not None:
                raise GovernanceError(
                    f"{context}.{field} must remain null until a holdout is registered"
                )

    limitations = _string_array(
        registry["limitations"],
        context=f"{context}.limitations",
        allow_empty=False,
    )
    if limitations != HOLDOUT_LIMITATIONS:
        raise GovernanceError(f"{context}.limitations must preserve all honesty boundaries")

    metadata_context = f"{context}.access_ledger"
    metadata = _exact_keys(
        registry["access_ledger"],
        HOLDOUT_ACCESS_LEDGER_KEYS,
        context=metadata_context,
    )
    if metadata["path"] != HOLDOUT_LEDGER_PATH.as_posix():
        raise GovernanceError(f"{metadata_context}.path must bind the canonical ledger")
    declared_file_sha = _validate_sha(
        metadata["file_sha256"], context=f"{metadata_context}.file_sha256"
    )
    if declared_file_sha != _sha256(ledger_path):
        raise GovernanceError(f"{metadata_context}.file_sha256 does not bind ledger bytes")
    declared_file_bytes = _integer(
        metadata["file_byte_count"],
        context=f"{metadata_context}.file_byte_count",
        minimum=1,
    )
    if declared_file_bytes != ledger_path.stat().st_size:
        raise GovernanceError(f"{metadata_context}.file_byte_count does not bind ledger bytes")
    event_count = _integer(
        metadata["event_count"], context=f"{metadata_context}.event_count", minimum=1
    )
    if event_count != len(ledger):
        raise GovernanceError(f"{metadata_context}.event_count does not match the ledger")
    if metadata["hash_algorithm"] != "sha256":
        raise GovernanceError(f"{metadata_context}.hash_algorithm must equal 'sha256'")
    if metadata["domain_separator"] != HOLDOUT_HASH_DOMAIN.decode("utf-8"):
        raise GovernanceError(f"{metadata_context}.domain_separator is not the v1 hash domain")
    if metadata["canonicalization"] != HOLDOUT_CANONICALIZATION:
        raise GovernanceError(f"{metadata_context}.canonicalization drifted")
    if metadata["chain_rule"] != HOLDOUT_CHAIN_RULE:
        raise GovernanceError(f"{metadata_context}.chain_rule drifted")
    if metadata["payload_boundary"] != HOLDOUT_PAYLOAD_BOUNDARY:
        raise GovernanceError(f"{metadata_context}.payload_boundary loses the data boundary")

    previous_hash: str | None = None
    previous_date: date | None = None
    for index, raw_event in enumerate(ledger):
        event_context = f"{HOLDOUT_LEDGER_PATH.as_posix()}[{index}]"
        event = _exact_keys(raw_event, HOLDOUT_EVENT_KEYS, context=event_context)
        if _integer(event["schema_version"], context=f"{event_context}.schema_version") != 1:
            raise GovernanceError(f"{event_context}.schema_version must equal 1")
        event_index = _integer(
            event["event_index"], context=f"{event_context}.event_index", minimum=0
        )
        if event_index != index:
            raise GovernanceError(f"{event_context}.event_index breaks the exact sequence")
        event_date = _date(event["recorded_on"], context=f"{event_context}.recorded_on")
        if previous_date is not None and event_date < previous_date:
            raise GovernanceError(f"{event_context}.recorded_on moves backwards")
        previous_date = event_date
        if event["recorded_at"] is not None:
            _timestamp(event["recorded_at"], context=f"{event_context}.recorded_at")
        if event["previous_event_sha256"] != previous_hash:
            raise GovernanceError(f"{event_context}.previous_event_sha256 breaks the chain")
        event_hash = _validate_sha(
            event["event_sha256"], context=f"{event_context}.event_sha256"
        )
        computed_hash = _holdout_event_digest(event)
        if event_hash != computed_hash:
            raise GovernanceError(f"{event_context}.event_sha256 does not bind the event")
        previous_hash = event_hash

        event_type = _enum(
            event["event_type"], {"non_access_genesis"}, context=f"{event_context}.event_type"
        )
        if event_type == "non_access_genesis":
            if index != 0:
                raise GovernanceError("non_access_genesis may appear only at event_index zero")
            if event["exposure_occurred"] is not False:
                raise GovernanceError("non_access_genesis cannot record holdout exposure")
            if event["recorded_at"] is not None:
                raise GovernanceError(
                    "non_access_genesis recorded_at must remain null when no instant is known"
                )
            if event["recorded_on"] != registry["as_of_date"]:
                raise GovernanceError(
                    "non_access_genesis recorded_on must match the registry as_of_date"
                )
            for field in (
                "holdout_id",
                "actor_id",
                "authorization_id",
                "purpose",
                "exposure_scope",
            ):
                if event[field] is not None:
                    raise GovernanceError(f"{event_context}.{field} must be null at genesis")
            if (
                event["event_scope"]
                != "ledger_initialization_only_not_a_historical_or_off_repository_access_attestation"
            ):
                raise GovernanceError(
                    f"{event_context}.event_scope must equal the safe genesis literal"
                )

    if len(ledger) != 1 or ledger[0]["event_type"] != "non_access_genesis":
        raise GovernanceError(
            "no_confirmatory_holdout_registered requires exactly one non-access genesis event"
        )
    head = _validate_sha(
        metadata["head_event_sha256"],
        context=f"{metadata_context}.head_event_sha256",
    )
    if head != previous_hash:
        raise GovernanceError(f"{metadata_context}.head_event_sha256 does not match the chain")


PREREGISTRATION_KEYS = {
    "schema_version",
    "artifact_id",
    "as_of_date",
    "canonical_spec",
    "scope",
    "freeze_status",
    "freeze_receipt",
    "freeze_revision",
    "frozen_at",
    "m0_completion_status",
    "holdout_registry_status",
    "global_freeze_fields",
    "protocols",
    "freeze_requirements",
}
PREREGISTRATION_CANONICAL_KEYS = {"path", "version"}
GLOBAL_FREEZE_FIELDS = [
    "causal_graph",
    "variable_dictionary",
    "treatment_version_ontology",
    "interference_and_reset_boundary",
    "source_tensor_contract",
    "target_tensor_contract",
    "measurement_validation_plan",
    "target_populations",
    "minimum_useful_effects",
    "pre_treatment_feature_whitelist",
    "automated_lineage_rule",
    "baseline_definitions",
    "intervention_definitions",
    "outcome_definitions",
    "competing_event_definitions",
    "censoring_definitions",
    "multiplicity_plan",
    "power_plan",
    "holdout_split_manifest",
    "ecosystem_evidence_ledger_binding",
    "dependency_firebreak_proof_binding",
    "optional_component_map_binding",
    "scientific_review_receipt_binding",
    "statistical_review_receipt_binding",
    "code_container_environment_digest_bundle",
    "result_interpretation_table_binding",
]
PROTOCOL_ORDER = ["h1_protocol_a", "h1_protocol_b", "h2", "h3", "h4"]
PROTOCOL_BASE_KEYS = {
    "registered_claim_id",
    "protocol_label",
    "activation_status",
    "interpretation_boundary",
    "supporting_software_artifacts",
    "freeze_fields",
}
PROTOCOL_WITH_ESTIMAND_KEYS = PROTOCOL_BASE_KEYS | {"estimand_rows"}
H3_PROTOCOL_KEYS = PROTOCOL_BASE_KEYS | {
    "estimand_rows",
    "gate_binding_rule",
    "pid_gates",
}
SUPPORTING_ARTIFACT_KEYS = {"path", "sha256", "status"}
ESTIMAND_SCIENTIFIC_FIELDS = [
    "scientific_question",
    "target_population",
    "unit_and_cluster",
    "eligibility_and_time_zero",
    "treatment_or_predictor",
    "comparator",
    "outcome",
    "potential_outcome_or_predictive_estimand",
    "assignment_or_sampling_mechanism",
    "identification_assumptions",
    "estimator_and_uncertainty",
    "missingness_and_receipt",
    "multiplicity_family",
    "minimum_useful_effect",
    "validation_target",
    "abstention_rule",
    "permitted_interpretation",
]
ESTIMAND_ROW_KEYS = {"estimand_id", "role", *ESTIMAND_SCIENTIFIC_FIELDS}
PROTOCOL_METADATA = {
    "h1_protocol_a": (
        "H1",
        "paired_frozen_snapshot_algorithmic_response",
        "candidate_primary_not_selected",
    ),
    "h1_protocol_b": (
        "H1",
        "randomized_closed_loop_effect_modification",
        "candidate_primary_not_selected",
    ),
    "h2": ("H2", "prospective_censoring_aware_failure_prediction", "unfrozen"),
    "h3": (
        "H3",
        "conditional_incremental_pid_value",
        "blocked_pending_all_four_pid_gates_and_an_active_h1_or_h2_dataset",
    ),
    "h4": (
        "H4",
        "representational_availability_versus_causal_policy_use",
        "exploratory_groundwork_only_unfrozen",
    ),
}
ESTIMAND_IDS = {
    "h1_protocol_a": "h1_protocol_a_primary",
    "h1_protocol_b": "h1_protocol_b_primary",
    "h2": "h2_primary",
    "h3": "h3_primary",
    "h4": "h4_primary",
}
EXPECTED_SUPPORTING_ARTIFACTS = {
    "h1_protocol_a": [
        (
            "crates/pid-sim/fixtures/h1_preflight_valid.json",
            "synthetic_common_preflight_fixture_only_not_a_scientific_freeze",
        ),
        (
            "crates/pid-sim/fixtures/h1_protocol_a_valid.json",
            "synthetic_scoring_fixture_only_not_h1_evidence",
        ),
    ],
    "h1_protocol_b": [],
    "h2": [
        (
            "crates/pid-sim/fixtures/h2_reference/analysis_plan.json",
            "synthetic_arithmetic_fixture_only_not_a_domain_specific_analysis_freeze",
        )
    ],
    "h3": [],
    "h4": [],
}
FREEZE_FIELD_INVENTORIES = {
    "h1_protocol_a": [
        "primary_protocol_selection",
        "policy_environment_checkpoint",
        "immutable_baseline_snapshot_boundary",
        "moderator_definition_timestamp_and_train_only_transform",
        "diagnostic_noninterference_tolerance",
        "treatment_versions_site_dose",
        "placebo_positive_control_manipulation_specificity_and_receipt",
        "clone_state_hash_contract",
        "cache_memory_sampler_hook_reset_contract",
        "rng_coupling_and_draw_ledger",
        "evaluation_order_and_worker_process_controls",
        "response_functional_output_metric_and_scale",
        "repeatability_and_monte_carlo_precision",
        "response_predictor_and_tuning_budget",
        "design_non_pid_baseline",
        "outer_task_family_split",
        "primary_score_calibration_bins_and_decision_rule",
        "coupling_and_second_metric_sensitivity",
        "testing_hierarchy",
        "replication_target",
    ],
    "h1_protocol_b": [
        "primary_protocol_selection",
        "policy_environment",
        "randomized_unit_and_interference_reset_cluster",
        "moderator_definition_timestamp_and_train_only_transform",
        "diagnostic_noninterference_tolerance",
        "treatment_versions_site_dose",
        "placebo_positive_control_manipulation_specificity_and_receipt",
        "assignment_probabilities_blocks_rng_and_archived_ledger",
        "noncompliance_carryover_reset_and_censoring",
        "separate_policy_execution_physical_outcome_families",
        "itt_primary_and_per_protocol_assumptions",
        "effect_learner_nuisance_models_and_tuning_budget",
        "propensity_and_truncation_rules",
        "effect_validation_stack",
        "train_defined_calibration_bins",
        "allocation_rule",
        "randomization_or_cluster_uncertainty",
        "synthetic_oracle_and_negative_controls",
        "outer_task_family_split",
        "testing_hierarchy",
        "replication_target",
    ],
    "h2": [
        "target_type",
        "landmark_time_zero_eligibility_and_update_schedule",
        "prediction_horizon",
        "episode_case_persistent_world_grouping",
        "feature_cutoff_deployment_availability_and_train_fit_transforms",
        "event_ontology",
        "named_failure",
        "competing_events",
        "censoring_and_missingness",
        "sampled_and_target_prevalence",
        "censoring_model_strata_crossfit_positivity_and_sensitivity",
        "matched_access_comparator_registry_and_budgets",
        "primary_proper_score",
        "calibration_intercept_slope_reliability",
        "conformal_plan_or_not_used",
        "alarm_threshold_persistence_refractory_matching_reset_missing_score_policy",
        "nondetection_retaining_lead_time",
        "utility_costs_capacity_latency",
        "uncertainty_cluster_and_independent_episode_event_family_counts",
        "nested_outer_split",
        "external_or_later_time_holdout",
        "separate_recalibration_split",
        "shift_subgroups",
        "replication_target",
    ],
    "h3": [
        "parent_h1_or_h2_protocol",
        "source_target_definitions",
        "sampling_law",
        "population_support_declaration",
        "pid_measure",
        "dimensionality",
        "scaling_or_projection",
        "estimator_configuration",
        "neighborhood_parameters",
        "dependence_treatment",
        "local_score_construction",
        "regime_hash",
        "training_reference_feature_construction",
        "model_m0",
        "model_m1",
        "model_m2",
        "primary_incremental_endpoint",
        "minimum_useful_effect",
        "fit_eligibility_and_evaluation_folds",
        "local_ranking_and_aggregate_reconstruction_oracles",
        "matched_capacity_tuning_budget",
        "nested_outer_holdout",
        "useful_margin_and_equivalence_region",
        "abstention_denominator",
        "abstention_rules",
        "kill_rules",
        "replication_target",
    ],
    "h4": [
        "task_variable_q",
        "availability_estimand_a_q",
        "locked_probe_decoder_and_outer_design",
        "intervention_constructions",
        "policy_effect_estimand_u_q",
        "execution_or_physical_effect_estimand_e_q",
        "engagement_specificity_support_g_q",
        "positive_and_negative_controls",
        "dose_layer_time_scope",
        "high_availability_threshold",
        "near_zero_equivalence_margin",
        "discordance_prevalence_and_magnitude_estimand",
        "explanatory_factor_family",
        "outer_holdout",
        "testing_hierarchy",
        "replication_target",
        "missingness_exclusion_abstention",
        "permitted_interpretation",
    ],
}
PID_GATE_ORDER = ["population", "measure", "estimator", "application"]
PID_GATE_KEYS = {"status", "regime_hash", "evidence_bindings"}
CONTENT_BINDING_KEYS = {"path", "sha256"}


def _validate_preregistration(root: Path, artifact: dict[str, Any]) -> None:
    context = PREREGISTRATION_PATH.as_posix()
    artifact = _exact_keys(artifact, PREREGISTRATION_KEYS, context=context)
    if _integer(artifact["schema_version"], context=f"{context}.schema_version") != 1:
        raise GovernanceError(f"{context}.schema_version must equal 1")
    if artifact["artifact_id"] != "prisoma_m0_preregistration_skeleton_v1":
        raise GovernanceError(f"{context}.artifact_id is not the v1 scaffold identifier")
    _date(artifact["as_of_date"], context=f"{context}.as_of_date")
    _require_semantic_snapshot(
        artifact["scope"], snapshot="prereg_scope", context=f"{context}.scope"
    )
    canonical = _exact_keys(
        artifact["canonical_spec"],
        PREREGISTRATION_CANONICAL_KEYS,
        context=f"{context}.canonical_spec",
    )
    if canonical != {"path": "grandplan.md", "version": "12.5"}:
        raise GovernanceError(f"{context}.canonical_spec must identify grandplan.md v12.5")
    _safe_file(root, canonical["path"], context=f"{context}.canonical_spec.path")

    freeze_status = _string(artifact["freeze_status"], context=f"{context}.freeze_status")
    if (
        freeze_status != "unfrozen_draft"
        or artifact["freeze_receipt"] is not None
        or artifact["freeze_revision"] is not None
        or artifact["frozen_at"] is not None
        or artifact["m0_completion_status"] != "incomplete"
    ):
        raise GovernanceError(
            f"{context} falsely claims a freeze or completion in the v1 unfinished scaffold"
        )
    _enum(
        artifact["holdout_registry_status"],
        {"no_confirmatory_holdout_registered"},
        context=f"{context}.holdout_registry_status",
    )

    global_fields = _exact_keys(
        artifact["global_freeze_fields"],
        set(GLOBAL_FREEZE_FIELDS),
        context=f"{context}.global_freeze_fields",
    )
    if list(global_fields) != GLOBAL_FREEZE_FIELDS:
        raise GovernanceError(f"{context}.global_freeze_fields inventory is reordered")
    if any(value is not None for value in global_fields.values()):
        raise GovernanceError(f"{context}.global_freeze_fields must remain null while unfrozen")

    protocols = _exact_keys(
        artifact["protocols"], set(PROTOCOL_ORDER), context=f"{context}.protocols"
    )
    if list(protocols) != PROTOCOL_ORDER:
        raise GovernanceError(f"{context}.protocols must be ordered H1-A, H1-B, H2, H3, H4")
    h1_statuses = [
        protocols[branch].get("activation_status")
        for branch in ("h1_protocol_a", "h1_protocol_b")
        if isinstance(protocols[branch], dict)
    ]
    if h1_statuses.count("active_primary") > 1:
        raise GovernanceError(f"{context} blends H1-A and H1-B as simultaneous primary branches")

    for branch_id in PROTOCOL_ORDER:
        branch_context = f"{context}.protocols.{branch_id}"
        expected_keys = (
            H3_PROTOCOL_KEYS
            if branch_id == "h3"
            else PROTOCOL_WITH_ESTIMAND_KEYS
            if branch_id in ESTIMAND_IDS
            else PROTOCOL_BASE_KEYS
        )
        branch = _exact_keys(protocols[branch_id], expected_keys, context=branch_context)
        claim_id, label, activation = PROTOCOL_METADATA[branch_id]
        if (
            branch["registered_claim_id"] != claim_id
            or branch["protocol_label"] != label
            or branch["activation_status"] != activation
        ):
            raise GovernanceError(f"{branch_context} has an unknown or falsely activated state")
        _string(
            branch["interpretation_boundary"],
            context=f"{branch_context}.interpretation_boundary",
        )

        support = _array(
            branch["supporting_software_artifacts"],
            context=f"{branch_context}.supporting_software_artifacts",
        )
        observed_support: list[tuple[str, str]] = []
        for index, raw_binding in enumerate(support):
            binding_context = f"{branch_context}.supporting_software_artifacts[{index}]"
            binding = _exact_keys(
                raw_binding, SUPPORTING_ARTIFACT_KEYS, context=binding_context
            )
            observed_support.append((binding["path"], binding["status"]))
            _string(binding["status"], context=f"{binding_context}.status")
            _validate_bound_file(
                root,
                raw_path=binding["path"],
                raw_sha256=binding["sha256"],
                context=binding_context,
            )
        if observed_support != EXPECTED_SUPPORTING_ARTIFACTS[branch_id]:
            raise GovernanceError(f"{branch_context} supporting-artifact inventory drifted")

        if branch_id in ESTIMAND_IDS:
            rows = _array(branch["estimand_rows"], context=f"{branch_context}.estimand_rows")
            if len(rows) != 1:
                raise GovernanceError(f"{branch_context}.estimand_rows must contain one primary row")
            row_context = f"{branch_context}.estimand_rows[0]"
            row = _exact_keys(rows[0], ESTIMAND_ROW_KEYS, context=row_context)
            if row["estimand_id"] != ESTIMAND_IDS[branch_id] or row["role"] != "primary":
                raise GovernanceError(f"{row_context} has an unknown estimand identity or role")
            if any(row[field] is not None for field in ESTIMAND_SCIENTIFIC_FIELDS):
                raise GovernanceError(f"{row_context} must remain null while unfrozen")

        fields = _exact_keys(
            branch["freeze_fields"],
            set(FREEZE_FIELD_INVENTORIES[branch_id]),
            context=f"{branch_context}.freeze_fields",
        )
        if list(fields) != FREEZE_FIELD_INVENTORIES[branch_id]:
            raise GovernanceError(f"{branch_context}.freeze_fields inventory is reordered")
        if any(value is not None for value in fields.values()):
            raise GovernanceError(f"{branch_context}.freeze_fields must remain null while unfrozen")

    h3 = protocols["h3"]
    boundaries = {
        branch_id: protocols[branch_id]["interpretation_boundary"]
        for branch_id in PROTOCOL_ORDER
    }
    _require_semantic_snapshot(
        boundaries,
        snapshot="prereg_interpretation_boundaries",
        context=f"{context}.protocols interpretation boundaries",
    )

    _require_semantic_snapshot(
        h3["gate_binding_rule"],
        snapshot="h3_gate_binding_rule",
        context=f"{context}.protocols.h3.gate_binding_rule",
    )
    gates = _exact_keys(
        h3["pid_gates"], set(PID_GATE_ORDER), context=f"{context}.protocols.h3.pid_gates"
    )
    if list(gates) != PID_GATE_ORDER:
        raise GovernanceError("H3 PID gates must be ordered population, measure, estimator, application")
    observed_regimes: set[str] = set()
    all_passed = True
    for gate_name in PID_GATE_ORDER:
        gate_context = f"{context}.protocols.h3.pid_gates.{gate_name}"
        gate = _exact_keys(gates[gate_name], PID_GATE_KEYS, context=gate_context)
        gate_status = _enum(gate["status"], {"blocked", "passed"}, context=f"{gate_context}.status")
        bindings = _array(gate["evidence_bindings"], context=f"{gate_context}.evidence_bindings")
        if gate_status == "blocked":
            all_passed = False
            if gate["regime_hash"] is not None or bindings:
                raise GovernanceError(f"{gate_context} blocked state cannot carry pass evidence")
        else:
            regime = _validate_sha(gate["regime_hash"], context=f"{gate_context}.regime_hash")
            observed_regimes.add(regime)
            if not bindings:
                raise GovernanceError(f"{gate_context} pass requires evidence bindings")
            for binding_index, raw_binding in enumerate(bindings):
                binding_context = f"{gate_context}.evidence_bindings[{binding_index}]"
                binding = _exact_keys(
                    raw_binding, CONTENT_BINDING_KEYS, context=binding_context
                )
                _validate_bound_file(
                    root,
                    raw_path=binding["path"],
                    raw_sha256=binding["sha256"],
                    context=binding_context,
                )
    if len(observed_regimes) > 1:
        raise GovernanceError("H3 PID gate pass evidence does not share one exact regime hash")
    if all_passed:
        raise GovernanceError("H3 cannot be promoted while the claim registry remains not_eligible")
    if any(gates[name]["status"] != "blocked" for name in PID_GATE_ORDER):
        raise GovernanceError("H3 v1 current state must keep all four PID gates blocked")

    requirements = _string_array(
        artifact["freeze_requirements"],
        context=f"{context}.freeze_requirements",
        allow_empty=False,
    )
    if len(requirements) != 6:
        raise GovernanceError(f"{context}.freeze_requirements inventory drifted")
    _require_semantic_snapshot(
        requirements,
        snapshot="freeze_requirements",
        context=f"{context}.freeze_requirements",
    )
    if (
        "domain- and decision-justified minimum useful effects" not in requirements[2]
        or "development or blinded pilots only for nuisance and design parameters"
        not in requirements[2]
    ):
        raise GovernanceError(
            f"{context}.freeze_requirements must not derive useful effects from pilots"
        )


TRANSPORT_KEYS = {
    "schema_version",
    "artifact_id",
    "as_of_date",
    "canonical_spec",
    "scope",
    "status",
    "target_outcome_access_state",
    "holdout_governance_binding",
    "required_shift_variable_ids",
    "source_binding",
    "target_binding",
    "transport_assumptions",
    "effect_modifiers",
    "selection_diagram_or_equivalent",
    "changed_variable_assessments",
    "invariant_variable_assessments",
    "effect_modifier_assessments",
    "overlap_and_abstention_assessments",
    "transport_assessments",
    "conformance_assessments",
    "contamination_assessments",
    "required_contamination_subtypes",
    "rights_assessments",
    "required_rights_artifact_classes",
    "required_transport_dimensions",
    "required_contamination_dimensions",
    "known_limitations",
}
TRANSPORT_CANONICAL_SPEC_KEYS = {"path", "version", "relevant_sections"}
POPULATION_BINDING_KEYS = {
    "status",
    "binding_id",
    "population_definition",
    "sampling_frame",
    "policy_or_model_id",
    "policy_or_model_revision",
    "environment_id",
    "observation_window",
    "representation_contract",
    "outcome_contract",
    "dataset_id",
    "dataset_artifact_path",
    "dataset_artifact_sha256",
    "split_manifest_path",
    "split_manifest_sha256",
    "model_checkpoint_path",
    "model_checkpoint_sha256",
    "rights_receipt_path",
    "rights_receipt_sha256",
}
HOLDOUT_GOVERNANCE_BINDING_KEYS = {
    "registry_path",
    "ledger_path",
    "registry_status",
    "ledger_file_sha256",
    "ledger_head_event_sha256",
}
REQUIRED_SHIFT_VARIABLE_IDS = [
    "morphology",
    "action_parameterization",
    "camera_geometry",
    "controller",
    "dynamics",
    "instruction_distribution",
    "object_set",
    "failure_prevalence",
    "latency",
    "observation_noise",
]
REQUIRED_CONTAMINATION_SUBTYPES = [
    "exact_duplicate",
    "near_duplicate",
    "semantic_paraphrase",
    "asset_family_clone",
    "mirrored_layout",
    "trajectory_subsequence",
    "pretraining_exposure",
    "transform_leakage",
]
REQUIRED_RIGHTS_ARTIFACT_CLASSES = ["dataset", "model", "asset", "generated_content"]
ASSESSMENT_WRAPPER_KEYS = {"status", "record_schema", "records"}
CONTAMINATION_WRAPPER_KEYS = ASSESSMENT_WRAPPER_KEYS | {
    "completion_language_boundary"
}
ASSESSMENT_RECORD_SCHEMAS = {
    "changed_variable_assessments": {
        "variable_id": "nonempty_string",
        "source_definition": "nonempty_string",
        "target_definition": "nonempty_string",
        "measurement_scale_and_units": "nonempty_string",
        "source_evidence_bindings": "nonempty_array_of_content_bound_artifacts",
        "target_evidence_bindings": "nonempty_array_of_content_bound_artifacts",
        "change_assessment": "nonempty_string",
        "handling_status": (
            "enum_measured_harmonized_adjusted_intentionally_varied_or_unobserved"
        ),
        "handling": "nonempty_string",
        "evidence_bindings": "nonempty_array_of_content_bound_artifacts",
        "residual_limitation": "nonempty_string",
        "uncertainty": "nonempty_string",
        "assessment_status": "enum_assessed_or_unresolved",
    },
    "invariant_variable_assessments": {
        "variable_id": "nonempty_string",
        "invariance_claim": "nonempty_string",
        "tolerance_or_equivalence_region": "nonempty_string",
        "source_evidence_bindings": "nonempty_array_of_content_bound_artifacts",
        "target_evidence_bindings": "nonempty_array_of_content_bound_artifacts",
        "uncertainty": "nonempty_string",
        "assessment_status": "enum_assessed_or_unresolved",
    },
    "effect_modifier_assessments": {
        "modifier_id": "nonempty_string",
        "definition": "nonempty_string",
        "source_support": "nonempty_string",
        "target_support": "nonempty_string",
        "overlap_metric": "nonempty_string",
        "overlap_threshold": "nonempty_string",
        "evidence_bindings": "nonempty_array_of_content_bound_artifacts",
        "assessment_status": "enum_assessed_or_unresolved",
    },
    "overlap_and_abstention_assessments": {
        "assessment_id": "nonempty_string",
        "effect_modifier_ids": "nonempty_array_of_registered_modifier_ids",
        "support_or_overlap_method": "nonempty_string",
        "target_restriction": "nonempty_string",
        "abstention_rule": "nonempty_string",
        "evidence_bindings": "nonempty_array_of_content_bound_artifacts",
        "assessment_status": "enum_assessed_or_unresolved",
    },
    "transport_assessments": {
        "analysis_id": "nonempty_string",
        "estimand_id": "nonempty_string",
        "transport_assumptions": "nonempty_array_of_strings",
        "selection_diagram_or_equivalent_binding": "content_bound_artifact",
        "analysis_method": "nonempty_string",
        "fit_data_scope": "nonempty_string",
        "untouched_target_outcomes": "boolean",
        "separate_recalibration_split": "nonempty_string",
        "target_restriction_or_abstention": "nonempty_string",
        "validation_endpoint": "nonempty_string",
        "estimate_and_uncertainty_binding": "content_bound_artifact",
        "assessment_status": "enum_assessed_or_unresolved",
    },
    "conformance_assessments": {
        "assessment_id": "nonempty_string",
        "interface_or_contract": "nonempty_string",
        "producer_revision": "nonempty_string",
        "consumer_revision": "nonempty_string",
        "frame_clock_schema_outcome_contract": "nonempty_string",
        "fixture_or_report_binding": "content_bound_artifact",
        "assessment_status": "enum_pass_fail_or_unresolved",
    },
    "contamination_assessments": {
        "assessment_id": "nonempty_string",
        "contamination_class": (
            "enum_exact_duplicate_near_duplicate_semantic_asset_trajectory_"
            "pretraining_or_transform_leakage"
        ),
        "contamination_subtype": (
            "enum_exact_duplicate_near_duplicate_semantic_paraphrase_asset_family_clone_"
            "mirrored_layout_trajectory_subsequence_pretraining_exposure_or_transform_leakage"
        ),
        "source_artifact_bindings": "nonempty_array_of_content_bound_artifacts",
        "target_artifact_bindings": "nonempty_array_of_content_bound_artifacts",
        "partition_scope": "nonempty_string",
        "threshold_fit_scope": "nonempty_string",
        "method_and_threshold": "nonempty_string",
        "findings_and_uncertainty": "nonempty_string",
        "resolution": "nonempty_string",
        "unresolved_limitation": "nonempty_string",
        "unresolved_exposure": "boolean",
        "assessment_status": "enum_bounded_assessment_complete_or_unresolved",
    },
    "rights_assessments": {
        "assessment_id": "nonempty_string",
        "artifact_class": "enum_dataset_model_asset_or_generated_content",
        "artifact_binding": "content_bound_artifact",
        "license_or_rights_basis": "nonempty_string",
        "use_and_redistribution_scope": "nonempty_string",
        "restrictions": "nonempty_string",
        "review_receipt_binding": "content_bound_artifact",
        "assessment_status": "enum_cleared_restricted_or_unresolved",
    },
}


def _validate_transport(
    root: Path,
    artifact: dict[str, Any],
    holdout_registry: dict[str, Any],
    ledger_path: Path,
) -> None:
    context = TRANSPORT_PATH.as_posix()
    artifact = _exact_keys(artifact, TRANSPORT_KEYS, context=context)
    if _integer(artifact["schema_version"], context=f"{context}.schema_version") != 1:
        raise GovernanceError(f"{context}.schema_version must equal 1")
    if artifact["artifact_id"] != "prisoma_transport_contamination_ledger_v1":
        raise GovernanceError(f"{context}.artifact_id is not the v1 ledger identifier")
    _date(artifact["as_of_date"], context=f"{context}.as_of_date")
    _require_semantic_snapshot(
        artifact["scope"], snapshot="transport_scope", context=f"{context}.scope"
    )
    canonical = _exact_keys(
        artifact["canonical_spec"],
        TRANSPORT_CANONICAL_SPEC_KEYS,
        context=f"{context}.canonical_spec",
    )
    if canonical["path"] != "grandplan.md" or canonical["version"] != "12.5":
        raise GovernanceError(f"{context}.canonical_spec must identify grandplan.md v12.5")
    _safe_file(root, canonical["path"], context=f"{context}.canonical_spec.path")
    sections = _string_array(
        canonical["relevant_sections"],
        context=f"{context}.canonical_spec.relevant_sections",
        allow_empty=False,
    )
    if sections != ["5.10", "5.11", "12.M0"]:
        raise GovernanceError(f"{context}.canonical_spec sections drifted")
    _enum(
        artifact["status"],
        {"structure_only_pending_dataset_and_target_selection"},
        context=f"{context}.status",
    )
    if artifact["target_outcome_access_state"] != "no_confirmatory_holdout_registered":
        raise GovernanceError(f"{context}.target_outcome_access_state overstates access state")
    governance_context = f"{context}.holdout_governance_binding"
    governance = _exact_keys(
        artifact["holdout_governance_binding"],
        HOLDOUT_GOVERNANCE_BINDING_KEYS,
        context=governance_context,
    )
    if governance["registry_path"] != HOLDOUT_REGISTRY_PATH.as_posix():
        raise GovernanceError(f"{governance_context}.registry_path drifted")
    if governance["ledger_path"] != HOLDOUT_LEDGER_PATH.as_posix():
        raise GovernanceError(f"{governance_context}.ledger_path drifted")
    _safe_file(root, governance["registry_path"], context=f"{governance_context}.registry_path")
    _safe_file(root, governance["ledger_path"], context=f"{governance_context}.ledger_path")
    if governance["registry_status"] != holdout_registry["registry_status"]:
        raise GovernanceError(f"{governance_context}.registry_status disagrees with registry")
    ledger_sha = _validate_sha(
        governance["ledger_file_sha256"],
        context=f"{governance_context}.ledger_file_sha256",
    )
    if ledger_sha != _sha256(ledger_path):
        raise GovernanceError(f"{governance_context}.ledger_file_sha256 drifted")
    head = _validate_sha(
        governance["ledger_head_event_sha256"],
        context=f"{governance_context}.ledger_head_event_sha256",
    )
    if head != holdout_registry["access_ledger"]["head_event_sha256"]:
        raise GovernanceError(f"{governance_context}.ledger_head_event_sha256 drifted")
    shifts = _string_array(
        artifact["required_shift_variable_ids"],
        context=f"{context}.required_shift_variable_ids",
        allow_empty=False,
    )
    if shifts != REQUIRED_SHIFT_VARIABLE_IDS:
        raise GovernanceError(f"{context}.required_shift_variable_ids is incomplete or reordered")
    subtypes = _string_array(
        artifact["required_contamination_subtypes"],
        context=f"{context}.required_contamination_subtypes",
        allow_empty=False,
    )
    if subtypes != REQUIRED_CONTAMINATION_SUBTYPES:
        raise GovernanceError(
            f"{context}.required_contamination_subtypes is incomplete or reordered"
        )
    rights_classes = _string_array(
        artifact["required_rights_artifact_classes"],
        context=f"{context}.required_rights_artifact_classes",
        allow_empty=False,
    )
    if rights_classes != REQUIRED_RIGHTS_ARTIFACT_CLASSES:
        raise GovernanceError(
            f"{context}.required_rights_artifact_classes is incomplete or reordered"
        )

    for side in ("source_binding", "target_binding"):
        binding_context = f"{context}.{side}"
        binding = _exact_keys(
            artifact[side], POPULATION_BINDING_KEYS, context=binding_context
        )
        if binding["status"] != "unselected":
            raise GovernanceError(f"{binding_context}.status must equal 'unselected'")
        if any(value is not None for key, value in binding.items() if key != "status"):
            raise GovernanceError(f"{binding_context} must remain empty while unselected")
    for field in ("transport_assumptions", "effect_modifiers", "selection_diagram_or_equivalent"):
        if artifact[field] is not None:
            raise GovernanceError(f"{context}.{field} must remain null in structure-only state")

    for field, expected_schema in ASSESSMENT_RECORD_SCHEMAS.items():
        wrapper_context = f"{context}.{field}"
        wrapper_keys = (
            CONTAMINATION_WRAPPER_KEYS
            if field == "contamination_assessments"
            else ASSESSMENT_WRAPPER_KEYS
        )
        wrapper = _exact_keys(artifact[field], wrapper_keys, context=wrapper_context)
        if wrapper["status"] != "not_assessed":
            raise GovernanceError(f"{wrapper_context}.status must equal 'not_assessed'")
        schema = _exact_keys(
            wrapper["record_schema"], set(expected_schema), context=f"{wrapper_context}.record_schema"
        )
        if schema != expected_schema:
            raise GovernanceError(f"{wrapper_context}.record_schema drifted")
        if _array(wrapper["records"], context=f"{wrapper_context}.records"):
            raise GovernanceError(
                f"{wrapper_context}.records must be empty while not assessed"
            )
        if field == "contamination_assessments":
            _require_semantic_snapshot(
                wrapper["completion_language_boundary"],
                snapshot="transport_completion_boundary",
                context=f"{wrapper_context}.completion_language_boundary",
            )
    for field, minimum in (
        ("required_transport_dimensions", 5),
        ("required_contamination_dimensions", 6),
        ("known_limitations", 3),
    ):
        items = _string_array(
            artifact[field], context=f"{context}.{field}", allow_empty=False
        )
        if len(items) < minimum:
            raise GovernanceError(f"{context}.{field} is incomplete")
    _require_semantic_snapshot(
        {
            "required_transport_dimensions": artifact["required_transport_dimensions"],
            "required_contamination_dimensions": artifact[
                "required_contamination_dimensions"
            ],
            "known_limitations": artifact["known_limitations"],
        },
        snapshot="transport_dimensions",
        context=f"{context} required dimensions and limitations",
    )


CLAIM_REGISTRY_KEYS = {
    "schema_version",
    "as_of_date",
    "canonical_spec",
    "scope",
    "m0_freeze_status",
    "claims",
}
CLAIM_REGISTRY_CANONICAL_KEYS = {"path", "version", "status"}
M0_STATUS_KEYS = {
    "overall",
    "preregistration_skeleton",
    "causal_graph",
    "variable_dictionary",
    "treatment_ontology",
    "h1_estimand",
    "h2_estimand",
    "minimum_useful_effects",
    "pre_treatment_feature_whitelist",
    "holdout_registry",
    "holdout_access_ledger",
    "transport_contamination_ledger",
    "literature_screening_ledger",
    "ecosystem_evidence",
    "capability_matrix",
}
EXPECTED_M0_STATUSES = {
    "overall": "not_freeze_ready",
    "preregistration_skeleton": (
        "implemented_unfrozen_scaffold_not_a_preregistration"
    ),
    "causal_graph": "unfrozen_pending_policy_environment_and_intervention_selection",
    "variable_dictionary": "partial_software_contracts_only",
    "treatment_ontology": "partial_h1_common_preflight_only",
    "h1_estimand": (
        "deterministic_protocol_a_software_reference_fixture_only_real_estimand_unfrozen"
    ),
    "h2_estimand": (
        "synthetic_fixed_horizon_reference_only_real_estimand_and_ontology_unfrozen"
    ),
    "minimum_useful_effects": (
        "unfrozen_pending_domain_and_decision_justification"
    ),
    "pre_treatment_feature_whitelist": "partial_h1_common_preflight_lineage_rule",
    "holdout_registry": "no_confirmatory_holdout_registered",
    "holdout_access_ledger": (
        "initialized_genesis_only_no_access_events_not_proof_of_nonaccess"
    ),
    "transport_contamination_ledger": (
        "implemented_structure_only_dataset_and_target_unselected"
    ),
    "literature_screening_ledger": (
        "implemented_legacy_reference_inventory_only_fresh_reproducible_search_required"
    ),
    "ecosystem_evidence": "current_offline_overlay_available",
    "capability_matrix": "generated_content_bound_current_views_available_no_validated_rows",
}
CLAIM_KEYS = {
    "claim_id",
    "registered_role",
    "execution_status",
    "scientific_status",
    "pid_dependency",
    "current_artifacts",
    "proof_commands",
    "remaining_required_artifacts",
    "permitted_language",
    "prohibited_language",
}
CLAIM_WITH_MUE_KEYS = CLAIM_KEYS | {"minimum_useful_effect_status"}
CLAIM_ARTIFACT_KEYS = {"path", "status"}
EXPECTED_CLAIM_STATES = {
    "H1": (
        "deterministic_protocol_a_software_reference_fixture_runnable_protocol_b_unimplemented",
        "blocked_on_pilot_and_real_capture",
        "unfrozen_pending_domain_and_decision_justification",
    ),
    "H2": (
        "deterministic_synthetic_fixed_horizon_software_reference_fixture_runnable_real_execution_unimplemented",
        "blocked_on_domain_freeze_real_capture_comparator_frontier_and_external_validation",
        "unfrozen_pending_domain_and_decision_justification",
    ),
    "H3": (
        "not_eligible",
        "blocked_on_population_measure_estimator_and_application_gates",
        "unfrozen_until_h3_activation",
    ),
    "H4": (
        "exploratory_attribution_groundwork_only",
        "blocked_on_real_probe_and_intervention_study",
        "unfrozen_pending_domain_and_decision_justification",
    ),
}


def _validate_claim_registry(root: Path, registry: dict[str, Any]) -> None:
    context = CLAIM_REGISTRY_PATH.as_posix()
    registry = _exact_keys(registry, CLAIM_REGISTRY_KEYS, context=context)
    if _integer(registry["schema_version"], context=f"{context}.schema_version") != 1:
        raise GovernanceError(f"{context}.schema_version must equal 1")
    _date(registry["as_of_date"], context=f"{context}.as_of_date")
    _require_semantic_snapshot(
        registry["scope"], snapshot="claim_registry_scope", context=f"{context}.scope"
    )
    canonical = _exact_keys(
        registry["canonical_spec"],
        CLAIM_REGISTRY_CANONICAL_KEYS,
        context=f"{context}.canonical_spec",
    )
    if (
        canonical["path"] != "grandplan.md"
        or canonical["version"] != "12.5"
        or canonical["status"] != "living_post_review_reconciliation"
    ):
        raise GovernanceError(f"{context}.canonical_spec drifted")
    _safe_file(root, canonical["path"], context=f"{context}.canonical_spec.path")

    m0 = _exact_keys(
        registry["m0_freeze_status"], M0_STATUS_KEYS, context=f"{context}.m0_freeze_status"
    )
    for field, expected in EXPECTED_M0_STATUSES.items():
        if m0[field] != expected:
            raise GovernanceError(
                f"{context}.m0_freeze_status.{field} must equal {expected!r}"
            )

    claims = _array(registry["claims"], context=f"{context}.claims")
    identifiers = [
        item.get("claim_id") if isinstance(item, dict) else None for item in claims
    ]
    if identifiers != ["EC1", "H1", "H2", "H3", "H4"]:
        raise GovernanceError(f"{context}.claims must preserve EC1, H1, H2, H3, H4 order")
    for index, raw_claim in enumerate(claims):
        claim_id = identifiers[index]
        claim_context = f"{context}.claims[{index}]"
        expected_keys = CLAIM_KEYS if claim_id == "EC1" else CLAIM_WITH_MUE_KEYS
        claim = _exact_keys(raw_claim, expected_keys, context=claim_context)
        for field in (
            "registered_role",
            "execution_status",
            "scientific_status",
            "pid_dependency",
            "permitted_language",
            "prohibited_language",
        ):
            _string(claim[field], context=f"{claim_context}.{field}")
        for field in ("proof_commands", "remaining_required_artifacts"):
            _string_array(
                claim[field], context=f"{claim_context}.{field}", allow_empty=False
            )
        artifacts = _array(
            claim["current_artifacts"], context=f"{claim_context}.current_artifacts"
        )
        if not artifacts:
            raise GovernanceError(f"{claim_context}.current_artifacts must not be empty")
        for artifact_index, raw_artifact in enumerate(artifacts):
            artifact_context = f"{claim_context}.current_artifacts[{artifact_index}]"
            artifact = _exact_keys(
                raw_artifact, CLAIM_ARTIFACT_KEYS, context=artifact_context
            )
            _string(artifact["path"], context=f"{artifact_context}.path")
            _string(artifact["status"], context=f"{artifact_context}.status")

        if claim_id in EXPECTED_CLAIM_STATES:
            execution, scientific, mue = EXPECTED_CLAIM_STATES[claim_id]
            if (
                claim["execution_status"] != execution
                or claim["scientific_status"] != scientific
                or claim["minimum_useful_effect_status"] != mue
            ):
                raise GovernanceError(f"{claim_context} overstates its current claim state")
    _require_semantic_snapshot(
        claims, snapshot="claim_snapshots", context=f"{context}.claims"
    )


LITERATURE_KEYS = {
    "schema_version",
    "artifact_id",
    "as_of_date",
    "canonical_spec",
    "scope",
    "status",
    "inventory_artifact",
    "legacy_review_rounds",
    "systematic_review_claimed",
    "fresh_search_required",
    "fresh_search_required_before",
    "endpoint_change_policy",
    "unresolved_h2_comparator_registry",
    "fresh_search_requirements",
    "known_limitations",
}
LITERATURE_CANONICAL_KEYS = {"path", "version"}
INVENTORY_KEYS = {
    "path",
    "sha256",
    "byte_count",
    "row_count",
    "inventory_date",
    "inventory_role",
}
LEGACY_ROUND_KEYS = {
    "round_id",
    "status",
    "search_provenance",
    "screening_decisions",
    "interpretation",
}
SEARCH_PROVENANCE_KEYS = {
    "protocol_registration",
    "databases_and_indexes",
    "query_strings",
    "search_run_dates",
    "date_limits",
    "language_limits",
    "grey_literature_policy",
    "deduplication_method",
    "inclusion_criteria",
    "exclusion_criteria",
    "reviewer_assignment",
    "conflict_resolution",
    "screening_flow_counts",
}
ENDPOINT_CHANGE_KEYS = {
    "change_triggers",
    "required_action",
    "silent_overwrite_allowed",
    "automatic_evidence_promotion_allowed",
    "network_polling_in_ci",
}
COMPARATOR_REGISTRY_KEYS = {"scope", "registry_status", "families"}
COMPARATOR_FAMILY_KEYS = {
    "family_id",
    "hierarchy_level",
    "canonical_description",
    "canonical_reference_ids",
    "screening_status",
    "applicability",
    "implementation_or_evidence",
    "disposition",
}
EXPECTED_COMPARATOR_FAMILIES = [
    ("prevalence_only", "level_0_design_and_naive", []),
    ("design_covariates", "level_0_design_and_naive", []),
    ("last_action_or_progress", "level_0_design_and_naive", []),
    (
        "policy_entropy_or_action_uncertainty",
        "level_1_policy_uncertainty_and_temporal",
        [],
    ),
    (
        "ensemble_or_stochastic_pass_disagreement",
        "level_1_policy_uncertainty_and_temporal",
        [],
    ),
    (
        "action_smoothness_or_chunk_inconsistency",
        "level_1_policy_uncertainty_and_temporal",
        [],
    ),
    (
        "dynamics_or_world_model_prediction_error",
        "level_1_policy_uncertainty_and_temporal",
        [],
    ),
    (
        "ood_or_representation_distance",
        "level_1_policy_uncertainty_and_temporal",
        [],
    ),
    ("tri_info", "level_1_policy_uncertainty_and_temporal", ["R25"]),
    (
        "safe_supervised_internal_state",
        "level_1_policy_uncertainty_and_temporal",
        ["R110"],
    ),
    (
        "hide_and_seek_temporal_localization",
        "level_1_policy_uncertainty_and_temporal",
        ["R95"],
    ),
    ("actprobe_action_space", "level_1_policy_uncertainty_and_temporal", ["R102"]),
    (
        "rewind_il_inter_chunk_discrepancy",
        "level_1_policy_uncertainty_and_temporal",
        ["R111"],
    ),
    (
        "tide_inter_chunk_discrepancy",
        "level_1_policy_uncertainty_and_temporal",
        [],
    ),
    (
        "architecture_stratified_black_box_action_features",
        "level_1_policy_uncertainty_and_temporal",
        ["R109"],
    ),
    (
        "vlaconf_one_class_internal_confidence",
        "level_1_policy_uncertainty_and_temporal",
        ["R103"],
    ),
    (
        "perturbation_induced_action_disagreement",
        "level_1_policy_uncertainty_and_temporal",
        ["R104"],
    ),
    (
        "activation_probe_warning",
        "level_1_policy_uncertainty_and_temporal",
        ["R105"],
    ),
    (
        "foresight_action_conditioned_world_model_latents",
        "level_1_policy_uncertainty_and_temporal",
        ["R101"],
    ),
    (
        "temporal_difference_or_sequential_calibration",
        "level_1_policy_uncertainty_and_temporal",
        ["R112"],
    ),
    ("individual_and_joint_mi_or_cmi", "level_2_information", []),
    ("co_information_or_shannon_invariants", "level_2_information", []),
    ("simple_association_or_predictive_likelihood", "level_2_information", []),
    ("categorical_contingency_statistics", "level_2_information", []),
    ("capacity_matched_frozen_representation_model", "level_3_learned", []),
    ("temporal_learned_model", "level_3_learned", []),
    ("attribution_or_intervention_features", "level_3_learned", []),
    (
        "vla_trace_or_better_mechanism_features",
        "level_3_learned",
        ["R26", "R27"],
    ),
]


def _validate_literature(root: Path, artifact: dict[str, Any]) -> None:
    context = LITERATURE_PATH.as_posix()
    artifact = _exact_keys(artifact, LITERATURE_KEYS, context=context)
    if _integer(artifact["schema_version"], context=f"{context}.schema_version") != 1:
        raise GovernanceError(f"{context}.schema_version must equal 1")
    if artifact["artifact_id"] != "prisoma_literature_screening_ledger_v1":
        raise GovernanceError(f"{context}.artifact_id is not the v1 ledger identifier")
    _date(artifact["as_of_date"], context=f"{context}.as_of_date")
    _require_semantic_snapshot(
        artifact["scope"], snapshot="literature_scope", context=f"{context}.scope"
    )
    canonical = _exact_keys(
        artifact["canonical_spec"],
        LITERATURE_CANONICAL_KEYS,
        context=f"{context}.canonical_spec",
    )
    if canonical != {"path": "grandplan.md", "version": "12.5"}:
        raise GovernanceError(f"{context}.canonical_spec must identify grandplan.md v12.5")
    _safe_file(root, canonical["path"], context=f"{context}.canonical_spec.path")
    _enum(
        artifact["status"],
        {"legacy_dated_reference_inventory_only"},
        context=f"{context}.status",
    )

    inventory_context = f"{context}.inventory_artifact"
    inventory = _exact_keys(
        artifact["inventory_artifact"], INVENTORY_KEYS, context=inventory_context
    )
    inventory_path = _validate_bound_file(
        root,
        raw_path=inventory["path"],
        raw_sha256=inventory["sha256"],
        raw_byte_count=inventory["byte_count"],
        context=inventory_context,
    )
    row_count = _integer(
        inventory["row_count"], context=f"{inventory_context}.row_count", minimum=0
    )
    with inventory_path.open(encoding="utf-8", newline="") as stream:
        rows = list(csv.reader(stream))
    actual_row_count = max(0, len(rows) - 1)
    if not rows or row_count != actual_row_count:
        raise GovernanceError(f"{inventory_context}.row_count does not bind the CSV rows")
    _date(inventory["inventory_date"], context=f"{inventory_context}.inventory_date")
    _require_semantic_snapshot(
        inventory["inventory_role"],
        snapshot="literature_inventory_role",
        context=f"{inventory_context}.inventory_role",
    )

    rounds = _array(artifact["legacy_review_rounds"], context=f"{context}.legacy_review_rounds")
    if len(rounds) != 1:
        raise GovernanceError(f"{context}.legacy_review_rounds must contain the one legacy import")
    round_context = f"{context}.legacy_review_rounds[0]"
    review_round = _exact_keys(rounds[0], LEGACY_ROUND_KEYS, context=round_context)
    if review_round["round_id"] != "grandplan_v12_5_reference_inventory_2026_07_12":
        raise GovernanceError(f"{round_context}.round_id drifted")
    _enum(
        review_round["status"],
        {"incomplete_search_provenance"},
        context=f"{round_context}.status",
    )
    provenance = _exact_keys(
        review_round["search_provenance"],
        SEARCH_PROVENANCE_KEYS,
        context=f"{round_context}.search_provenance",
    )
    if any(value is not None for value in provenance.values()):
        raise GovernanceError(
            f"{round_context}.search_provenance cannot claim unrecorded legacy search details"
        )
    if _array(
        review_round["screening_decisions"],
        context=f"{round_context}.screening_decisions",
    ):
        raise GovernanceError(
            f"{round_context}.screening_decisions must be empty for the legacy import"
        )
    _require_semantic_snapshot(
        review_round["interpretation"],
        snapshot="literature_legacy_interpretation",
        context=f"{round_context}.interpretation",
    )

    if _boolean(
        artifact["systematic_review_claimed"],
        context=f"{context}.systematic_review_claimed",
    ):
        raise GovernanceError(f"{context} cannot claim a systematic review")
    if not _boolean(
        artifact["fresh_search_required"], context=f"{context}.fresh_search_required"
    ):
        raise GovernanceError(f"{context} must require a fresh reproducible search")
    lifecycle = _string_array(
        artifact["fresh_search_required_before"],
        context=f"{context}.fresh_search_required_before",
        allow_empty=False,
    )
    if lifecycle != ["preregistration", "submission", "rebuttal", "camera_ready"]:
        raise GovernanceError(f"{context}.fresh_search_required_before drifted")

    policy_context = f"{context}.endpoint_change_policy"
    policy = _exact_keys(
        artifact["endpoint_change_policy"], ENDPOINT_CHANGE_KEYS, context=policy_context
    )
    triggers = _string_array(
        policy["change_triggers"],
        context=f"{policy_context}.change_triggers",
        allow_empty=False,
    )
    if len(triggers) != 3:
        raise GovernanceError(f"{policy_context}.change_triggers must preserve all triggers")
    _string(policy["required_action"], context=f"{policy_context}.required_action")
    for field in (
        "silent_overwrite_allowed",
        "automatic_evidence_promotion_allowed",
        "network_polling_in_ci",
    ):
        if _boolean(policy[field], context=f"{policy_context}.{field}"):
            raise GovernanceError(f"{policy_context}.{field} must remain false")
    _require_semantic_snapshot(
        policy,
        snapshot="literature_endpoint_policy",
        context=policy_context,
    )

    comparator_context = f"{context}.unresolved_h2_comparator_registry"
    comparator = _exact_keys(
        artifact["unresolved_h2_comparator_registry"],
        COMPARATOR_REGISTRY_KEYS,
        context=comparator_context,
    )
    _require_semantic_snapshot(
        comparator["scope"],
        snapshot="literature_comparator_scope",
        context=f"{comparator_context}.scope",
    )
    _enum(
        comparator["registry_status"],
        {"unresolved_not_screened_or_frozen"},
        context=f"{comparator_context}.registry_status",
    )
    families = _array(comparator["families"], context=f"{comparator_context}.families")
    observed_family_inventory: list[tuple[str, str, list[str]]] = []
    for index, raw_family in enumerate(families):
        family_context = f"{comparator_context}.families[{index}]"
        family = _exact_keys(raw_family, COMPARATOR_FAMILY_KEYS, context=family_context)
        family_id = _string(family["family_id"], context=f"{family_context}.family_id")
        hierarchy = _enum(
            family["hierarchy_level"],
            {
                "level_0_design_and_naive",
                "level_1_policy_uncertainty_and_temporal",
                "level_2_information",
                "level_3_learned",
            },
            context=f"{family_context}.hierarchy_level",
        )
        references = _string_array(
            family["canonical_reference_ids"],
            context=f"{family_context}.canonical_reference_ids",
        )
        observed_family_inventory.append((family_id, hierarchy, references))
        _string(
            family["canonical_description"],
            context=f"{family_context}.canonical_description",
        )
        screening_status = _enum(
            family["screening_status"],
            {
                "not_screened_in_reproducible_search",
                "source_unresolved_pending_fresh_search",
            },
            context=f"{family_context}.screening_status",
        )
        expected_screening_status = (
            "source_unresolved_pending_fresh_search"
            if family_id == "tide_inter_chunk_discrepancy"
            else "not_screened_in_reproducible_search"
        )
        if screening_status != expected_screening_status:
            raise GovernanceError(
                f"{family_context}.screening_status loses its exact unresolved state"
            )
        for field in ("applicability", "implementation_or_evidence", "disposition"):
            if family[field] is not None:
                raise GovernanceError(
                    f"{family_context}.{field} must remain null while unresolved"
                )
    if observed_family_inventory != EXPECTED_COMPARATOR_FAMILIES:
        raise GovernanceError(f"{comparator_context}.families is incomplete or reordered")
    _require_semantic_snapshot(
        [
            (family["family_id"], family["canonical_description"])
            for family in families
        ],
        snapshot="literature_family_descriptions",
        context=f"{comparator_context}.families descriptions",
    )

    for field, minimum in (("fresh_search_requirements", 4), ("known_limitations", 3)):
        items = _string_array(
            artifact[field], context=f"{context}.{field}", allow_empty=False
        )
        if len(items) < minimum:
            raise GovernanceError(f"{context}.{field} is incomplete")
    _require_semantic_snapshot(
        {
            "fresh_search_requirements": artifact["fresh_search_requirements"],
            "known_limitations": artifact["known_limitations"],
        },
        snapshot="literature_requirements_limits",
        context=f"{context} requirements and limitations",
    )


def audit_bundle(root: Path = ROOT, *, require_freeze_ready: bool = False) -> list[str]:
    """Audit the five-file bundle and return stable freeze-readiness blocker codes."""

    root = root.resolve(strict=True)
    preregistration, _ = _read_json(root, PREREGISTRATION_PATH)
    holdout_registry, _ = _read_json(root, HOLDOUT_REGISTRY_PATH)
    holdout_ledger, holdout_ledger_path = _read_jsonl(root, HOLDOUT_LEDGER_PATH)
    transport, _ = _read_json(root, TRANSPORT_PATH)
    literature, _ = _read_json(root, LITERATURE_PATH)
    claim_registry, _ = _read_json(root, CLAIM_REGISTRY_PATH)

    _validate_preregistration(root, preregistration)
    _validate_holdout_bundle(
        root,
        holdout_registry,
        holdout_ledger,
        holdout_ledger_path,
    )
    _validate_transport(root, transport, holdout_registry, holdout_ledger_path)
    _validate_literature(root, literature)
    _validate_claim_registry(root, claim_registry)

    dated_artifacts = {
        PREREGISTRATION_PATH.as_posix(): preregistration["as_of_date"],
        HOLDOUT_REGISTRY_PATH.as_posix(): holdout_registry["as_of_date"],
        TRANSPORT_PATH.as_posix(): transport["as_of_date"],
        LITERATURE_PATH.as_posix(): literature["as_of_date"],
        CLAIM_REGISTRY_PATH.as_posix(): claim_registry["as_of_date"],
    }
    if len(set(dated_artifacts.values())) != 1:
        raise GovernanceError(f"governance as_of_date values disagree: {dated_artifacts}")

    registry_status = holdout_registry["registry_status"]
    if preregistration["holdout_registry_status"] != registry_status:
        raise GovernanceError("preregistration and holdout registry status disagree")
    if transport["target_outcome_access_state"] != registry_status:
        raise GovernanceError("transport target-outcome state and holdout registry disagree")
    governance_binding = transport["holdout_governance_binding"]
    if governance_binding["registry_status"] != registry_status:
        raise GovernanceError("transport holdout binding and holdout registry disagree")

    m0 = claim_registry["m0_freeze_status"]
    if m0["holdout_registry"] != registry_status:
        raise GovernanceError("claim registry and holdout registry status disagree")
    if preregistration["freeze_status"] != "unfrozen_draft":
        raise GovernanceError("claim registry cannot remain not-ready after a claimed freeze")
    if transport["status"] != "structure_only_pending_dataset_and_target_selection":
        raise GovernanceError("claim registry cannot describe completed transport evidence")
    if literature["status"] != "legacy_dated_reference_inventory_only":
        raise GovernanceError("claim registry cannot describe a completed literature search")

    # A valid v1 artifact is intentionally an unfinished scaffold.  Future freeze-ready
    # support requires a reviewed schema revision with content-bound receipts; arbitrary
    # non-null strings can never clear this list.
    blockers = list(FREEZE_BLOCKERS)
    if require_freeze_ready:
        return blockers
    return blockers


def _parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=ROOT,
        help=argparse.SUPPRESS,
    )
    parser.add_argument(
        "--require-freeze-ready",
        action="store_true",
        help=(
            "report v1's stable closed-gate blockers (a real freeze requires a reviewed "
            "schema/validator revision with typed content-bound receipts)"
        ),
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = _parse_args(argv)
    try:
        blockers = audit_bundle(
            args.root,
            require_freeze_ready=args.require_freeze_ready,
        )
    except (GovernanceError, OSError) as error:
        print(f"Research-governance audit failed: {error}", file=sys.stderr)
        return 1

    if args.require_freeze_ready and blockers:
        print("Research-governance freeze blockers:", file=sys.stderr)
        for code in blockers:
            print(f"- {code}", file=sys.stderr)
        return FREEZE_BLOCKED_EXIT

    print(
        "Research-governance audit passed "
        f"(honest unfinished scaffold; freeze blockers={len(blockers)})."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
