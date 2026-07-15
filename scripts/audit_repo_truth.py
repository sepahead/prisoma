#!/usr/bin/env python3
"""Fail closed on repository-truth drift missed by prose heuristics.

This audit binds active commands and claims to checked-out dependencies and living protocol
ledgers rather than remembered version strings. It covers the pid-rs 1.0 migration, excluded NCP
consumer lock, firebreak wording, generated notices, current claim boundaries, and the dated
ecosystem-overlay reconciliation. Network refresh remains deliberate and manual; normal CI proves
that the reviewed offline state and active prose agree.
"""

from __future__ import annotations

import csv
import hashlib
import io
import json
import os
import re
import selectors
import signal
import stat
import subprocess
import time
import tomllib
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
PID_RS = ROOT / "pid-rs"
EXP0_COMMAND_FILES = [
    ROOT / "Cargo.toml",
    ROOT / "flake.nix",
    ROOT / "justfile",
    ROOT / ".github/workflows/ci.yml",
    ROOT / "README.md",
    ROOT / "AGENTS.md",
    ROOT / "EXPERIMENTS.md",
]
MAX_REPO_FILE_BYTES = 16 * 1024 * 1024
MAX_GIT_OUTPUT_BYTES = 4 * 1024 * 1024
GIT_TIMEOUT_SECONDS = 30.0
MAX_LEDGER_ITEMS = 10_000


class TruthAuditError(RuntimeError):
    """An input violated the bounded repository-truth audit contract."""


def _read_regular_bytes(path: Path, *, label: str) -> bytes:
    try:
        before = path.lstat()
    except OSError as error:
        raise TruthAuditError(f"cannot inspect {label} {path}: {error}") from error
    if not stat.S_ISREG(before.st_mode):
        raise TruthAuditError(f"{label} must be a regular, non-symlink file: {path}")
    if before.st_size > MAX_REPO_FILE_BYTES:
        raise TruthAuditError(
            f"{label} exceeds the {MAX_REPO_FILE_BYTES}-byte limit: {path}"
        )

    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as error:
        raise TruthAuditError(f"cannot open {label} {path}: {error}") from error
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or (opened.st_dev, opened.st_ino) != (
            before.st_dev,
            before.st_ino,
        ):
            raise TruthAuditError(f"{label} changed while opening: {path}")
        chunks: list[bytes] = []
        remaining = MAX_REPO_FILE_BYTES + 1
        while remaining:
            chunk = os.read(descriptor, min(65_536, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        after = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    payload = b"".join(chunks)
    if len(payload) > MAX_REPO_FILE_BYTES:
        raise TruthAuditError(
            f"{label} exceeds the {MAX_REPO_FILE_BYTES}-byte limit: {path}"
        )
    if (opened.st_dev, opened.st_ino, opened.st_size, opened.st_mtime_ns) != (
        after.st_dev,
        after.st_ino,
        after.st_size,
        after.st_mtime_ns,
    ) or len(payload) != after.st_size:
        raise TruthAuditError(f"{label} changed while reading: {path}")
    return payload


def _read_regular_text(path: Path, *, label: str) -> str:
    try:
        return _read_regular_bytes(path, label=label).decode("utf-8")
    except UnicodeDecodeError as error:
        raise TruthAuditError(f"{label} is not valid UTF-8: {path}") from error


def _is_regular_file(path: Path) -> bool:
    try:
        return stat.S_ISREG(path.lstat().st_mode)
    except OSError:
        return False


def _json_object(path: Path, *, label: str) -> dict[str, Any]:
    def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise TruthAuditError(f"{label} has duplicate JSON key {key!r}")
            result[key] = value
        return result

    def reject_constant(value: str) -> Any:
        raise TruthAuditError(f"{label} has invalid JSON constant {value}")

    try:
        value = json.loads(
            _read_regular_text(path, label=label),
            object_pairs_hook=reject_duplicates,
            parse_constant=reject_constant,
        )
    except (ValueError, RecursionError) as error:
        raise TruthAuditError(f"cannot parse {label}: {error}") from error
    if not isinstance(value, dict):
        raise TruthAuditError(f"{label} must be a JSON object")
    return value


def _toml_object(path: Path, *, label: str) -> dict[str, Any]:
    try:
        value = tomllib.loads(_read_regular_text(path, label=label))
    except (tomllib.TOMLDecodeError, RecursionError) as error:
        raise TruthAuditError(f"cannot parse {label}: {error}") from error
    if not isinstance(value, dict):
        raise TruthAuditError(f"{label} must be a TOML table")
    return value


def _object_list(value: Any, *, label: str) -> list[dict[str, Any]]:
    if (
        not isinstance(value, list)
        or len(value) > MAX_LEDGER_ITEMS
        or any(not isinstance(item, dict) for item in value)
    ):
        raise TruthAuditError(f"{label} must be a bounded list of objects")
    return value


def _repo_relative_path(raw: Any, *, label: str) -> Path:
    if not isinstance(raw, str) or not raw or "\\" in raw:
        raise TruthAuditError(f"{label} must be a repository-relative POSIX path")
    relative = Path(raw)
    if relative.is_absolute() or ".." in relative.parts:
        raise TruthAuditError(f"{label} escapes the repository: {raw!r}")
    candidate = ROOT / relative
    component = ROOT
    for part in relative.parts:
        component /= part
        if component.is_symlink():
            raise TruthAuditError(f"{label} may not traverse a symlink: {raw!r}")
    try:
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(ROOT.resolve(strict=True))
    except (FileNotFoundError, ValueError) as error:
        raise TruthAuditError(
            f"{label} is missing or escapes the repository"
        ) from error
    return resolved


def _terminate(process: subprocess.Popen[bytes]) -> None:
    try:
        if os.name == "posix":
            os.killpg(process.pid, signal.SIGKILL)
        elif process.poll() is None:
            process.kill()
    except OSError:
        try:
            process.kill()
        except OSError:
            pass


def _run_bounded(
    command: list[str],
    *,
    timeout_seconds: float = GIT_TIMEOUT_SECONDS,
    max_output_bytes: int = MAX_GIT_OUTPUT_BYTES,
) -> subprocess.CompletedProcess[str]:
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=os.name == "posix",
    )
    assert process.stdout is not None and process.stderr is not None
    selector = selectors.DefaultSelector()
    buffers = {"stdout": bytearray(), "stderr": bytearray()}
    total = 0
    deadline = time.monotonic() + timeout_seconds
    try:
        selector.register(process.stdout, selectors.EVENT_READ, "stdout")
        selector.register(process.stderr, selectors.EVENT_READ, "stderr")
        while selector.get_map():
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise subprocess.TimeoutExpired(command, timeout_seconds)
            for key, _events in selector.select(min(remaining, 0.25)):
                chunk = os.read(key.fd, min(65_536, max_output_bytes - total + 1))
                if not chunk:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                    continue
                total += len(chunk)
                if total > max_output_bytes:
                    raise TruthAuditError(
                        "Git output exceeds the aggregate "
                        f"{max_output_bytes}-byte limit"
                    )
                buffers[key.data].extend(chunk)
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise subprocess.TimeoutExpired(command, timeout_seconds)
        return_code = process.wait(timeout=remaining)
    except BaseException:
        _terminate(process)
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        raise
    finally:
        selector.close()
        for stream in (process.stdout, process.stderr):
            if not stream.closed:
                stream.close()

    try:
        stdout = bytes(buffers["stdout"]).decode("utf-8")
        stderr = bytes(buffers["stderr"]).decode("utf-8")
    except UnicodeDecodeError as error:
        raise TruthAuditError("Git output is not valid UTF-8") from error
    if return_code != 0:
        raise subprocess.CalledProcessError(
            return_code, command, output=stdout, stderr=stderr
        )
    return subprocess.CompletedProcess(command, return_code, stdout, stderr)


def git_output(*args: str) -> str:
    return _run_bounded(["git", *args]).stdout.strip()


def package_version() -> str:
    data = _toml_object(PID_RS / "Cargo.toml", label="pid-rs Cargo.toml")
    workspace = data.get("workspace")
    package = workspace.get("package") if isinstance(workspace, dict) else None
    version = package.get("version") if isinstance(package, dict) else None
    if not isinstance(version, str) or not version:
        raise TruthAuditError("pid-rs Cargo.toml has no workspace package version")
    return version


def gitlink_revision() -> str:
    fields = git_output("ls-files", "--stage", "pid-rs").split()
    if len(fields) < 4 or fields[0] != "160000":
        raise RuntimeError("pid-rs is not recorded as a gitlink in the index")
    return fields[1]


def locked_package_version(lock_path: Path, name: str) -> str | None:
    data = _toml_object(lock_path, label=str(lock_path.relative_to(ROOT)))
    packages = _object_list(data.get("package"), label=f"{lock_path}.package")
    matches = [
        package
        for package in packages
        if package.get("name") == name and "source" not in package
    ]
    if len(matches) != 1:
        return None
    version = matches[0].get("version")
    return version if isinstance(version, str) else None


def locked_git_packages(lock_path: Path, names: set[str]) -> list[dict[str, object]]:
    data = _toml_object(lock_path, label=str(lock_path.relative_to(ROOT)))
    packages = _object_list(data.get("package"), label=f"{lock_path}.package")
    return [
        package
        for package in packages
        if package.get("name") in names
        and str(package.get("source", "")).startswith("git+")
    ]


def exp0_command_problems() -> list[str]:
    problems: list[str] = []
    for path in EXP0_COMMAND_FILES:
        for line_no, line in enumerate(
            _read_regular_text(path, label=str(path.relative_to(ROOT))).splitlines(), 1
        ):
            if "pid-rs/crates/pid-core/Cargo.toml" not in line:
                continue
            if "--bin exp0" not in line and not re.search(r"\s+exp0\s+--", line):
                continue
            if (
                "--features experimental-all" not in line
                and "--all-features" not in line
            ):
                problems.append(
                    f"{path.relative_to(ROOT)}:{line_no}: Exp0 command omits "
                    "`--features experimental-all`"
                )
    return problems


def _audit() -> int:
    problems: list[str] = []
    if not _is_regular_file(PID_RS / "Cargo.toml"):
        print("Repository-truth problems: 1")
        print("- pid-rs submodule is not checked out")
        return 1

    version = package_version()
    revision = gitlink_revision()
    short = revision[:7]

    worktree_revision = git_output("-C", "pid-rs", "rev-parse", "HEAD")
    if worktree_revision != revision:
        problems.append(
            f"pid-rs worktree {worktree_revision} does not match gitlink {revision}"
        )

    required_claims = {
        ROOT / "grandplan.md": (version, revision),
        ROOT / "AGENTS.md": (version, short),
        ROOT / "CHANGELOG.md": (version, short),
    }
    for path, needles in required_claims.items():
        text = _read_regular_text(path, label=str(path.relative_to(ROOT)))
        for needle in needles:
            if needle not in text:
                problems.append(
                    f"{path.relative_to(ROOT)} does not record current pid-rs identity {needle!r}"
                )

    harness = _read_regular_text(
        ROOT / "crates/pid-sim/src/offline_harness.rs",
        label="crates/pid-sim/src/offline_harness.rs",
    )
    expected_stamp = f"pid-core {version} (pid-rs {short})"
    if expected_stamp not in harness:
        problems.append(
            "offline harness estimator revision stamp does not match "
            f"{expected_stamp!r}"
        )

    observer_version = locked_package_version(
        ROOT / "crates/ncp-observer/Cargo.lock", "pid-runlog"
    )
    if observer_version != version:
        problems.append(
            "crates/ncp-observer/Cargo.lock resolves local pid-runlog "
            f"{observer_version!r}, expected {version!r}"
        )

    ncp_packages = locked_git_packages(
        ROOT / "crates/ncp-observer/Cargo.lock", {"ncp-core", "ncp-zenoh"}
    )
    ncp_revisions = {
        str(package["source"]).rsplit("#", 1)[-1] for package in ncp_packages
    }
    if len(ncp_packages) != 2 or len(ncp_revisions) != 1:
        problems.append(
            "ncp-observer must lock ncp-core and ncp-zenoh to one exact revision"
        )
    else:
        ncp_revision = next(iter(ncp_revisions))
        observer_source = _read_regular_text(
            ROOT / "crates/ncp-observer/src/lib.rs",
            label="crates/ncp-observer/src/lib.rs",
        )
        if ncp_revision not in observer_source:
            problems.append(
                "ncp-observer canonical configuration does not record its exact locked NCP revision"
            )
        for constant in ("NCP_VERSION", "CONTRACT_HASH"):
            if constant not in observer_source:
                problems.append(
                    f"ncp-observer canonical configuration does not use ncp-core::{constant}"
                )

    notices = _read_regular_text(
        ROOT / "THIRD_PARTY_NOTICES.generated.md",
        label="THIRD_PARTY_NOTICES.generated.md",
    )
    for package in ("pid-core", "pid-runlog"):
        if f"| `{package}` | {version} |" not in notices:
            problems.append(
                f"THIRD_PARTY_NOTICES.generated.md does not record {package} {version}"
            )

    problems.extend(exp0_command_problems())

    for relative in ("justfile", ".github/workflows/ci.yml"):
        text = _read_regular_text(ROOT / relative, label=relative)
        for required in ("--pid-mode none", "pid_metrics=0", "pid_metric_events=0"):
            if required not in text:
                problems.append(f"{relative} firebreak is missing {required!r}")
        if "H1/H2 baseline predictors" in text:
            problems.append(
                f"{relative} overstates the static label-baseline firebreak as H1/H2 execution"
            )
        for required in (
            "pid-h1-preflight",
            "h1_preflight_valid.json",
            "h1_preflight_invalid.json",
            "h1_preflight_parse_invalid.json",
            "establishes_h1_evidence",
        ):
            if required not in text:
                problems.append(
                    f"{relative} H1 preflight smoke is missing {required!r}"
                )

    if not _is_regular_file(ROOT / "crates/pid-sim/src/bin/h1_preflight.rs"):
        problems.append("pid-h1-preflight binary is missing")
    for fixture in (
        "h1_preflight_valid.json",
        "h1_preflight_invalid.json",
        "h1_preflight_parse_invalid.json",
    ):
        if not _is_regular_file(ROOT / "crates/pid-sim/fixtures" / fixture):
            problems.append(f"H1 preflight fixture is missing: {fixture}")

    power_doc = _read_regular_text(
        ROOT / "docs/power-gate/README.md", label="docs/power-gate/README.md"
    )
    if (
        "Historical idealized grid outputs — withdrawn; not capture requirements"
        not in power_doc
    ):
        problems.append(
            "docs/power-gate/README.md does not withdraw retired capture counts"
        )
    if "## Capture-scale requirements" in power_doc:
        problems.append(
            "docs/power-gate/README.md revives retired capture requirements"
        )
    power_artifact = _json_object(
        ROOT / "docs/power-gate/power-gate-2026-07-10.json",
        label="docs/power-gate/power-gate-2026-07-10.json",
    )
    power_verdict_rows = _object_list(
        power_artifact.get("verdicts"), label="power-gate verdicts"
    )
    power_verdicts = {
        verdict["endpoint_id"]: (
            verdict["smallest_passing_grid_n"],
            verdict["null_rate_at_smallest_passing_grid_n"],
            verdict["passed"],
        )
        for verdict in power_verdict_rows
        if isinstance(verdict.get("endpoint_id"), str)
    }
    if len(power_verdicts) != len(power_verdict_rows):
        raise TruthAuditError(
            "power-gate verdict endpoint_id values must be unique strings"
        )
    expected_power_verdicts = {
        "h1": (None, None, False),
        "h2": (64, 0.05, True),
        "h3": (40, 0.0675, True),
        "h4": (96, 0.0675, True),
    }
    if power_verdicts != expected_power_verdicts:
        problems.append(
            "power-gate artifact verdicts changed; reconcile active interpretation"
        )
    for required in (
        "artifact verdict `passed=false`",
        "Smallest same-n passing grid point: 64",
        "Smallest same-n passing grid point: 40",
        "Smallest same-n passing grid point: 96",
    ):
        if required not in power_doc:
            problems.append(
                f"docs/power-gate/README.md omits artifact-backed verdict {required!r}"
            )

    splat_spec = _read_regular_text(ROOT / "pidsplatspecs.md", label="pidsplatspecs.md")
    if "Schema 2; partial M2/EC1 groundwork" not in splat_spec:
        problems.append(
            "pidsplatspecs.md does not record the current run-log schema/M2 status"
        )
    if "This is partial M4" in splat_spec:
        problems.append("pidsplatspecs.md mislabels run-log groundwork as M4")
    if "pid-h1-preflight" not in splat_spec:
        problems.append("pidsplatspecs.md omits the implemented H1 common preflight")
    for relative in ("pidsplatspecs.md", "ARCHITECTURE.md"):
        text = _read_regular_text(ROOT / relative, label=relative)
        if re.search(r"partial\s+M4\s+groundwork", text, flags=re.IGNORECASE):
            problems.append(f"{relative} mislabels viewer/run-log groundwork as M4")

    changelog = _read_regular_text(ROOT / "CHANGELOG.md", label="CHANGELOG.md")
    for required in (
        "H1 did not reach a passing grid point",
        "H2 first passed\n  at 64 tasks",
        "H3 first passed at 40 matched cases",
        "H4 first passed at 96 tasks",
    ):
        if required not in changelog:
            problems.append(f"CHANGELOG.md omits corrected power verdict {required!r}")
    for stale in (
        "using the *preregistered\n  procedures themselves*",
        "H3 powered at 30 matched cases",
        "H2/H4 at 96 tasks",
    ):
        if stale in changelog:
            problems.append(f"CHANGELOG.md retains stale power claim {stale!r}")

    grandplan = _read_regular_text(ROOT / "grandplan.md", label="grandplan.md")
    for false_engram_claim in (
        "a6f2c5f973783373db9d90769d981b1d549a5b6b",
        "contains an implemented neurocontrol/NCP reference stack",
    ):
        if false_engram_claim in grandplan:
            problems.append(
                "grandplan.md attributes unreachable implementation evidence to the public "
                f"sepahead/engram repository: {false_engram_claim!r}"
            )
    for required in (
        "a4ce6ab9897dd3f1265b4cacc53f0afc349087cd",
        "README-only placeholder",
    ):
        if required not in grandplan:
            problems.append(
                f"grandplan.md omits the verified public Engram boundary {required!r}"
            )
    if "current public main `2a55f3d" in grandplan:
        problems.append(
            "grandplan.md mislabels a reviewed Haldir revision as current main"
        )
    for project, dated_revision in {
        "galadriel": "ff272dc814080c6766a53c872ca4d0e24bcd5132",
        "crebain": "09dd5ec1556bd56e6934e1ef019f95de84cf9b4f",
        "manwe": "0f6505bb5dadf5d70359b5ea6d545216342bd30a",
        "haldir": "fbf5a5308da1c6a82eebe1afb56635bf0d6fd798",
        "cortexel": "16f2da71a5beb863235a90e552e6772639638be3",
        "melkor": "21c8fb53f58e19a78d92a4b01ce479374a7b8633",
    }.items():
        if dated_revision not in grandplan:
            problems.append(
                "grandplan.md omits the dated currentness revision for "
                f"{project}: {dated_revision}"
            )

    ecosystem_ledger_path = ROOT / "protocols/ecosystem_evidence_current_v1.json"
    if not _is_regular_file(ecosystem_ledger_path):
        problems.append("current ecosystem evidence overlay is missing")
    else:
        ecosystem_ledger = _json_object(
            ecosystem_ledger_path,
            label="protocols/ecosystem_evidence_current_v1.json",
        )
        baseline = ecosystem_ledger.get("baseline", {})
        if not isinstance(baseline, dict):
            raise TruthAuditError("ecosystem evidence baseline must be an object")
        baseline_path = _repo_relative_path(
            baseline.get("path"), label="ecosystem evidence baseline path"
        )
        baseline_bytes = _read_regular_bytes(
            baseline_path, label="ecosystem evidence baseline"
        )
        baseline_sha256 = hashlib.sha256(baseline_bytes).hexdigest()
        baseline_hash = baseline.get("sha256")
        baseline_row_count = baseline.get("row_count")
        if (
            not isinstance(baseline_hash, str)
            or re.fullmatch(r"[0-9a-f]{64}", baseline_hash) is None
        ):
            raise TruthAuditError("ecosystem evidence baseline sha256 is invalid")
        if (
            not isinstance(baseline_row_count, int)
            or isinstance(baseline_row_count, bool)
            or baseline_row_count < 0
            or baseline_row_count > MAX_LEDGER_ITEMS
        ):
            raise TruthAuditError("ecosystem evidence baseline row_count is invalid")
        if baseline_sha256 != baseline_hash:
            problems.append("ecosystem evidence overlay baseline hash does not match")
        try:
            baseline_text = baseline_bytes.decode("utf-8")
        except UnicodeDecodeError as error:
            raise TruthAuditError(
                "ecosystem evidence baseline is not valid UTF-8"
            ) from error
        baseline_rows = sum(
            1 for _ in csv.DictReader(io.StringIO(baseline_text, newline=""))
        )
        if baseline_rows != baseline_row_count:
            problems.append(
                "ecosystem evidence overlay baseline row count does not match"
            )

        override_rows = _object_list(
            ecosystem_ledger.get("overrides"),
            label="ecosystem evidence overrides",
        )
        overrides = {
            entry.get("project"): entry
            for entry in override_rows
            if isinstance(entry.get("project"), str)
        }
        if len(overrides) != len(override_rows):
            raise TruthAuditError(
                "ecosystem evidence override project values must be unique strings"
            )
        required_overrides = {
            "pid-rs",
            "NCP",
            "galadriel",
            "crebain",
            "manwe",
            "engram",
            "haldir",
            "cortexel",
            "melkor",
        }
        missing_overrides = sorted(required_overrides - overrides.keys())
        if missing_overrides:
            problems.append(
                f"ecosystem evidence overlay omits current edges: {missing_overrides}"
            )
        expected_revisions = {
            "pid-rs": "43ab60517b5387c61ae339f664d0d7afae3b8988",
            "NCP": "v0.8.0",
            "galadriel": "ff272dc814080c6766a53c872ca4d0e24bcd5132",
            "crebain": "09dd5ec1556bd56e6934e1ef019f95de84cf9b4f",
            "manwe": "0f6505bb5dadf5d70359b5ea6d545216342bd30a",
            "engram": "a4ce6ab9897dd3f1265b4cacc53f0afc349087cd",
            "haldir": "fbf5a5308da1c6a82eebe1afb56635bf0d6fd798",
            "cortexel": "16f2da71a5beb863235a90e552e6772639638be3",
            "melkor": "21c8fb53f58e19a78d92a4b01ce479374a7b8633",
        }
        for project, expected_revision in expected_revisions.items():
            if overrides.get(project, {}).get("observed_revision") != expected_revision:
                problems.append(
                    "ecosystem evidence overlay has an unreconciled reviewed revision for "
                    f"{project}"
                )
        if overrides.get("NCP", {}).get("resolved_revision") != (
            "2f5bd586d4bb20c90362bb6f5698b7f64057ba4e"
        ):
            problems.append(
                "ecosystem evidence overlay omits the peeled NCP v0.8.0 commit"
            )
        if overrides.get("crebain", {}).get("prior_frame_capability_revision") != (
            "49d7b3614f24d21a40fe2af6dbeac082338ae9d7"
        ):
            problems.append(
                "ecosystem evidence overlay omits Crebain's prior frame-capability revision"
            )
        boundary_requirements = {
            "galadriel": (
                "optional evidence-sender component",
                "deployed receiver-verified Crebain-to-Galadriel path",
                "no direct Prisoma adapter",
            ),
            "crebain": (
                "standard release omits ncp",
                "no release candidate or exact-head release evidence is sealed",
                "local put is not receiver receipt",
                "not called by admission",
                "action/control commands remain unregistered",
                "no direct Prisoma adapter",
            ),
            "manwe": ("no drop-in Prisoma adapter",),
            "haldir": (
                "declaration is cooperative, process-local",
                "not durably bound",
                "test-only seams",
                "no live Zenoh session",
                "no runnable service",
                "direct Prisoma route",
            ),
            "cortexel": ("lacks headless export", "does not supersede"),
            "melkor": ("no calibrated", "Prisoma adapter"),
        }
        for project, required_phrases in boundary_requirements.items():
            boundary = str(overrides.get(project, {}).get("current_boundary", ""))
            for required_phrase in required_phrases:
                if required_phrase not in boundary:
                    problems.append(
                        f"ecosystem evidence overlay overstates {project}; "
                        f"missing boundary {required_phrase!r}"
                    )
        engram_override = overrides.get("engram", {})
        if engram_override.get("observed_revision") != (
            "a4ce6ab9897dd3f1265b4cacc53f0afc349087cd"
        ) or "README-only" not in str(engram_override.get("current_boundary", "")):
            problems.append(
                "ecosystem evidence overlay revives unsupported Engram maturity"
            )

    claim_registry_path = ROOT / "protocols/research_claim_registry_v1.json"
    if not _is_regular_file(claim_registry_path):
        problems.append("current research claim registry is missing")
    else:
        claim_registry = _json_object(
            claim_registry_path,
            label="protocols/research_claim_registry_v1.json",
        )
        claims = _object_list(
            claim_registry.get("claims"), label="research claim registry claims"
        )
        claim_ids = [claim.get("claim_id") for claim in claims]
        if claim_ids != ["EC1", "H1", "H2", "H3", "H4"]:
            problems.append(
                "research claim registry must contain EC1 and H1-H4 exactly once in order"
            )
        claims_by_id = {
            claim.get("claim_id"): claim
            for claim in claims
            if isinstance(claim.get("claim_id"), str)
        }
        if len(claims_by_id) != len(claims):
            raise TruthAuditError(
                "research claim claim_id values must be unique strings"
            )
        h1 = claims_by_id.get("H1", {})
        if h1.get("execution_status") != (
            "deterministic_protocol_a_software_reference_fixture_runnable_"
            "protocol_b_unimplemented"
        ):
            problems.append(
                "research claim registry misstates the H1 execution boundary"
            )
        h1_artifact_rows = _object_list(
            h1.get("current_artifacts"), label="H1 current_artifacts"
        )
        h1_artifacts = {
            artifact.get("path")
            for artifact in h1_artifact_rows
            if isinstance(artifact.get("path"), str)
        }
        if len(h1_artifacts) != len(h1_artifact_rows):
            raise TruthAuditError("H1 artifact paths must be unique strings")
        required_h1_artifacts = {
            "crates/pid-sim/src/h1_preflight.rs",
            "crates/pid-sim/src/bin/h1_preflight.rs",
            "crates/pid-sim/fixtures/h1_preflight_valid.json",
            "crates/pid-sim/src/h1_protocol_a.rs",
            "crates/pid-sim/src/bin/h1_protocol_a.rs",
            "crates/pid-sim/fixtures/h1_protocol_a_valid.json",
        }
        if not required_h1_artifacts.issubset(h1_artifacts):
            problems.append(
                "research claim registry omits implemented H1 software artifacts"
            )
        for artifact in required_h1_artifacts:
            if not _is_regular_file(ROOT / artifact):
                problems.append(
                    f"research claim registry names missing H1 artifact: {artifact}"
                )
        h1_commands = h1.get("proof_commands")
        if not isinstance(h1_commands, list) or not all(
            isinstance(command, str) for command in h1_commands
        ):
            raise TruthAuditError("H1 proof_commands must be a string list")
        if "just h1-protocol-a" not in h1_commands:
            problems.append(
                "research claim registry omits the H1 Protocol-A proof command"
            )
        h2 = claims_by_id.get("H2", {})
        if h2.get("execution_status") != (
            "deterministic_synthetic_fixed_horizon_software_reference_fixture_"
            "runnable_real_execution_unimplemented"
        ):
            problems.append(
                "research claim registry misstates the H2 execution boundary"
            )
        h2_artifact_rows = _object_list(
            h2.get("current_artifacts"), label="H2 current_artifacts"
        )
        h2_artifacts = {
            artifact.get("path")
            for artifact in h2_artifact_rows
            if isinstance(artifact.get("path"), str)
        }
        if len(h2_artifacts) != len(h2_artifact_rows):
            raise TruthAuditError("H2 artifact paths must be unique strings")
        required_h2_files = {
            "crates/pid-sim/src/h2_reference.rs",
            "crates/pid-sim/src/bin/h2_reference.rs",
            "crates/pid-sim/fixtures/h2_reference/analysis_plan.json",
            "crates/pid-sim/fixtures/h2_reference/dataset_complete.json",
            "crates/pid-sim/fixtures/h2_reference/dataset_censored.json",
        }
        if not {
            "crates/pid-sim/src/h2_reference.rs",
            "crates/pid-sim/src/bin/h2_reference.rs",
            "crates/pid-sim/fixtures/h2_reference",
        }.issubset(h2_artifacts):
            problems.append(
                "research claim registry omits implemented H2 software artifacts"
            )
        for artifact in required_h2_files:
            if not _is_regular_file(ROOT / artifact):
                problems.append(
                    f"research claim registry names missing H2 artifact: {artifact}"
                )
        h2_commands = h2.get("proof_commands")
        if not isinstance(h2_commands, list) or not all(
            isinstance(command, str) for command in h2_commands
        ):
            raise TruthAuditError("H2 proof_commands must be a string list")
        if "just h2-reference" not in h2_commands:
            problems.append(
                "research claim registry omits the H2 reference proof command"
            )
        h2_boundary_text = " ".join(
            str(h2.get(field, ""))
            for field in ("permitted_language", "prohibited_language")
        )
        for required in (
            "synthetic",
            "prospective",
            "H2 passed",
            "deployment validity",
        ):
            if required not in h2_boundary_text:
                problems.append(
                    f"research claim registry H2 boundary omits {required!r}"
                )
        if claims_by_id.get("H3", {}).get("execution_status") != "not_eligible":
            problems.append("research claim registry overstates H3 eligibility")

    for relative in ("justfile", ".github/workflows/ci.yml"):
        text = _read_regular_text(ROOT / relative, label=relative)
        for required in (
            "pid-h1-protocol-a",
            "h1_protocol_a_valid.json",
            "h1_protocol_a_parse_invalid.json",
            "synthetic_fixture_only",
            "establishes_h1_evidence",
            "evaluation_metric_events=0",
            "pid_metric_events=0",
        ):
            if required not in text:
                problems.append(
                    f"{relative} H1 Protocol-A smoke is missing {required!r}"
                )
        for required in (
            "pid-h2-reference",
            "dataset_complete.json",
            "dataset_censored.json",
            "establishes_h2_evidence",
            "prospective_capture",
            "pid_metric_events=0",
        ):
            if required not in text:
                problems.append(
                    f"{relative} H2 reference smoke is missing {required!r}"
                )
    if "H1/H2 baselines execute with PID disabled" in grandplan:
        problems.append("grandplan.md overstates the static label-baseline firebreak")
    for relative in ("grandplan.md", "RESEARCH_VLA_D_NCP.md"):
        text = _read_regular_text(ROOT / relative, label=relative)
        if "PID-disabled H1/H2 path" in text:
            problems.append(
                f"{relative} overstates the dependency firebreak as H1/H2 execution"
            )
    for stale in (
        "while its submodule remains pinned to `8a5a9d",
        "`pid-rs@70b45f7…` as a **candidate upgrade**",
    ):
        if stale in grandplan:
            problems.append(
                f"grandplan.md retains stale active estimator claim: {stale}"
            )

    if problems:
        print(f"Repository-truth problems: {len(problems)}")
        for problem in problems:
            print(f"- {problem}")
        return 1

    print(
        "OK: repo truth matches pid-rs "
        f"{version} ({revision}); Exp0/firebreak/consumer-lock/notices checks pass."
    )
    return 0


def main() -> int:
    try:
        return _audit()
    except (
        TruthAuditError,
        OSError,
        csv.Error,
        KeyError,
        TypeError,
        ValueError,
        subprocess.SubprocessError,
    ) as error:
        print("Repository-truth problems: 1")
        print(f"- audit input invalid or unavailable: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
