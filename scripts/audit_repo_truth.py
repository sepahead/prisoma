#!/usr/bin/env python3
"""Fail closed on repository-truth drift missed by prose heuristics.

This audit binds active commands and claims to checked-out dependencies and living protocol
ledgers rather than remembered version strings. It covers the exact pinned pid-rs review source,
excluded NCP consumer lock, firebreak wording, generated notices, current claim boundaries, and
the dated ecosystem-overlay reconciliation. Network refresh remains deliberate and manual; normal
CI proves that the reviewed offline state and active prose agree.
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
import sys
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
ROOT_RELEASE_VERSION = "0.9.0"
ROOT_RELEASE_DATE = "2026-07-16"
ROOT_AUTHOR = "Sepehr Mahmoudian"
ROOT_REPOSITORY = "https://github.com/sepahead/prisoma"
NIXPKGS_CHANNEL = "nixos-26.05"
NIX_UV_VERSION = "0.11.28"
NIX_JUST_VERSION = "1.56.0"
PRE_COMMIT_VERSION = "4.6.0"
GITLEAKS_VERSION = "8.30.1"
GITLEAKS_REVISION = "83d9cd684c87d95d656c1458ef04895a7f1cbd8e"


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
    if process.returncode is not None:
        return
    if os.name == "posix":
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        except PermissionError:
            if sys.platform != "darwin":
                raise
    else:
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
    if timeout_seconds <= 0:
        raise subprocess.TimeoutExpired(command, timeout_seconds)
    if max_output_bytes < 0:
        raise TruthAuditError(
            f"invalid negative aggregate Git output budget: {max_output_bytes}"
        )
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=os.name == "posix",
    )
    stdout_stream = process.stdout
    stderr_stream = process.stderr
    selector: selectors.BaseSelector | None = None
    buffers = {"stdout": bytearray(), "stderr": bytearray()}
    total = 0
    deadline = time.monotonic() + timeout_seconds
    try:
        if stdout_stream is None or stderr_stream is None:
            raise TruthAuditError("Git subprocess pipes were not created")
        selector = selectors.DefaultSelector()
        selector.register(stdout_stream, selectors.EVENT_READ, "stdout")
        selector.register(stderr_stream, selectors.EVENT_READ, "stderr")
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
        if os.name == "posix":
            _terminate(process)
        return_code = process.wait(timeout=remaining)
    except BaseException:
        _terminate(process)
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        raise
    finally:
        _terminate(process)
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        if selector is not None:
            selector.close()
        for stream in (stdout_stream, stderr_stream):
            if stream is not None and not stream.closed:
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
    output = git_output("ls-files", "--stage", "--", "pid-rs")
    lines = output.splitlines()
    if len(lines) != 1:
        raise TruthAuditError("pid-rs must have exactly one gitlink entry in the index")
    match = re.fullmatch(r"160000 ([0-9a-f]{40}) 0\tpid-rs", lines[0])
    if match is None:
        raise TruthAuditError(
            "pid-rs is not recorded as a stage-0 SHA-1 gitlink in the index"
        )
    return match.group(1)


def root_release_identity_problems() -> list[str]:
    problems: list[str] = []

    pyproject = _toml_object(ROOT / "pyproject.toml", label="pyproject.toml")
    project = pyproject.get("project")
    if not isinstance(project, dict):
        raise TruthAuditError("pyproject.toml project table is missing")
    if project.get("version") != ROOT_RELEASE_VERSION:
        problems.append(f"pyproject.toml release version is not {ROOT_RELEASE_VERSION}")
    if project.get("authors") != [{"name": ROOT_AUTHOR}]:
        problems.append("pyproject.toml does not name the single canonical author")
    project_urls = project.get("urls")
    if (
        not isinstance(project_urls, dict)
        or project_urls.get("Repository") != ROOT_REPOSITORY
        or project_urls.get("Homepage") != ROOT_REPOSITORY
    ):
        problems.append("pyproject.toml repository identity is not canonical")
    tool = pyproject.get("tool")
    uv = tool.get("uv") if isinstance(tool, dict) else None
    required_uv = uv.get("required-version") if isinstance(uv, dict) else None
    if (
        not isinstance(required_uv, str)
        or re.fullmatch(r"==([0-9]+\.[0-9]+\.[0-9]+)", required_uv) is None
    ):
        problems.append("pyproject.toml does not pin one exact required uv version")

    cargo = _toml_object(ROOT / "Cargo.toml", label="Cargo.toml")
    workspace = cargo.get("workspace")
    workspace_package = (
        workspace.get("package") if isinstance(workspace, dict) else None
    )
    if not isinstance(workspace_package, dict):
        raise TruthAuditError("Cargo.toml workspace.package table is missing")
    if workspace_package.get("version") != ROOT_RELEASE_VERSION:
        problems.append(f"Cargo.toml release version is not {ROOT_RELEASE_VERSION}")
    if workspace_package.get("authors") != [ROOT_AUTHOR]:
        problems.append("Cargo.toml does not name the single canonical author")
    if workspace_package.get("repository") != ROOT_REPOSITORY:
        problems.append("Cargo.toml repository identity is not canonical")
    rust_version = workspace_package.get("rust-version")
    if not isinstance(rust_version, str):
        raise TruthAuditError("Cargo.toml workspace rust-version is missing")
    if re.fullmatch(r"[0-9]+\.[0-9]+", rust_version):
        expected_toolchain = f"{rust_version}.0"
    elif re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", rust_version):
        expected_toolchain = rust_version
    else:
        raise TruthAuditError("Cargo.toml workspace rust-version is malformed")

    ci = _read_regular_text(
        ROOT / ".github/workflows/ci.yml", label=".github/workflows/ci.yml"
    )
    rustup_invocations = ci.count("rustup toolchain install")
    installed_toolchains = re.findall(
        r"rustup toolchain install ([0-9]+\.[0-9]+\.[0-9]+)", ci
    )
    if (
        rustup_invocations == 0
        or len(installed_toolchains) != rustup_invocations
        or set(installed_toolchains) != {expected_toolchain}
        or ci.count(f"rustc --version | grep -F 'rustc {expected_toolchain} '")
        != rustup_invocations
    ):
        problems.append(
            "CI Rust installation and version receipts do not match workspace rust-version"
        )
    if isinstance(required_uv, str):
        expected_uv = required_uv.removeprefix("==")
        setup_uv_invocations = ci.count("uses: astral-sh/setup-uv@")
        configured_uv_versions = re.findall(
            r"^\s+version: \"([0-9]+\.[0-9]+\.[0-9]+)\"\s*$",
            ci,
            flags=re.MULTILINE,
        )
        if (
            setup_uv_invocations == 0
            or len(configured_uv_versions) != setup_uv_invocations
            or set(configured_uv_versions) != {expected_uv}
        ):
            problems.append(
                "CI setup-uv versions do not match pyproject.toml required-version"
            )

    citation = _read_regular_text(ROOT / "CITATION.cff", label="CITATION.cff")
    for required in (
        f"version: {ROOT_RELEASE_VERSION}",
        "family-names: Mahmoudian",
        "given-names: Sepehr",
        f'repository-code: "{ROOT_REPOSITORY}"',
        f"date-released: {ROOT_RELEASE_DATE}",
        "public GitHub source prerelease",
    ):
        if required not in citation:
            problems.append(f"CITATION.cff omits release identity {required!r}")
    if re.search(r"(?im)^\s*doi\s*:", citation):
        problems.append(
            "CITATION.cff assigns a DOI before an archive identifier exists"
        )
    if re.search(r"(?im)^\s*zenodo(?:-record)?\s*:", citation):
        problems.append(
            "CITATION.cff assigns a Zenodo record before an archive identifier exists"
        )
    if re.search(r"(?im)^\s*(?:identifiers|repository-artifact)\s*:", citation):
        problems.append(
            "CITATION.cff assigns archive identifier metadata before one exists"
        )

    release_notes = _read_regular_text(
        ROOT / "release/0.9.0/RELEASE_NOTES.md",
        label="release/0.9.0/RELEASE_NOTES.md",
    )
    release_notes_semantic = " ".join(release_notes.split())
    for required in (
        f"Prisoma {ROOT_RELEASE_VERSION} is a public GitHub source prerelease",
        f"released on {ROOT_RELEASE_DATE}",
        ROOT_AUTHOR,
        "`published:false` in the candidate decision manifest",
        "does not deny public availability of this source prerelease.",
        "No DOI, Zenodo record, or archive identifier is assigned.",
    ):
        if required not in release_notes_semantic:
            problems.append(
                f"release/0.9.0/RELEASE_NOTES.md omits release identity {required!r}"
            )

    candidate_generator = _read_regular_text(
        ROOT / "scripts/generate_candidate_release.py",
        label="scripts/generate_candidate_release.py",
    )
    for required in (
        f'REPOSITORY = "{ROOT_REPOSITORY}"',
        f'AUTHOR = "{ROOT_AUTHOR}"',
        f'RELEASE_VERSION = "{ROOT_RELEASE_VERSION}"',
    ):
        if required not in candidate_generator:
            problems.append(
                "scripts/generate_candidate_release.py omits release identity "
                f"{required!r}"
            )

    return problems


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


def exp0_documentation_problems() -> list[str]:
    """Bind the reported seeded-case counts to the current deterministic sweep."""

    problems: list[str] = []
    source = _read_regular_text(
        PID_RS / "crates/pid-core/src/bin/exp0.rs",
        label="pid-rs/crates/pid-core/src/bin/exp0.rs",
    )
    for required in (
        "let mut seeds = 3usize;",
        "let dims = [10usize, 64, 256];",
        '"independent_additive"',
        '"redundant_copy"',
        '"unique_s1"',
        '"xor_like"',
    ):
        if required not in source:
            problems.append(
                "Exp0 binary-default sweep contract changed; reconcile the "
                f"documented seeded-case counts (missing {required!r})"
            )

    justfile = _read_regular_text(ROOT / "justfile", label="justfile")
    if 'exp0-runlog path="outputs/exp0_runlog.jsonl" ' not in justfile or (
        'seeds="1":' not in justfile
    ):
        problems.append(
            "justfile Exp0 run-log recipe no longer has the documented one-seed default"
        )

    grandplan = " ".join(
        _read_regular_text(ROOT / "grandplan.md", label="grandplan.md").split()
    )
    findings = " ".join(
        _read_regular_text(ROOT / "findings.md", label="findings.md").split()
    )
    if (
        "12 scenario–dimension cells over three deterministic seeds (36 case results)"
    ) not in grandplan:
        problems.append(
            "grandplan.md does not distinguish the 12 Exp0 cells from the "
            "36 binary-default seeded case results"
        )
    for required in (
        "36 case results from 12 scenario–dimension cells over three "
        "deterministic seeds",
        "nine geometry warnings, zero geometry abstentions, nine monotonicity "
        "violations, three normalized-invariant bound violations",
        "The `just exp0-runlog` recipe deliberately passes one seed",
        "corresponding counts are 12, three, zero, three, and one",
    ):
        if required not in findings:
            problems.append(
                "findings.md does not preserve the deterministic Exp0 count "
                f"boundary {required!r}"
            )
    return problems


def justfile_reproducibility_problems() -> list[str]:
    """Reject recipe drift that can bypass locks, quoting, or runtime checks."""

    problems: list[str] = []
    text = _read_regular_text(ROOT / "justfile", label="justfile")
    strict_shell = 'set shell := ["bash", "-euo", "pipefail", "-c"]'
    raw_interpolation = re.compile(r"\{\{\s*[A-Za-z_][A-Za-z0-9_]*\s*\}\}")
    cargo_resolver_command = re.compile(r"\bcargo\s+(?:build|clippy|run|test)\b")
    optimization_guard = (
        "sys.flags.optimize == 0 or sys.exit("
        '"recipe checks require unoptimized Python")'
    )
    if text.count(strict_shell) != 1:
        problems.append(
            "justfile must declare one exact Bash strict-mode shell with pipefail"
        )

    for line_no, line in enumerate(text.splitlines(), 1):
        if cargo_resolver_command.search(line) and "--locked" not in line:
            problems.append(
                f"justfile:{line_no}: dependency-resolving Cargo command omits `--locked`"
            )
        if raw_interpolation.search(line):
            problems.append(
                f"justfile:{line_no}: recipe parameter is interpolated without `quote(...)`"
            )
        if "python -c" in line and "assert " in line and optimization_guard not in line:
            problems.append(
                f"justfile:{line_no}: Python assertion lacks the optimization-mode guard"
            )
    return problems


def readme_reproducibility_problems() -> list[str]:
    """Keep public dependency-resolving examples on the checked lockfiles."""

    problems: list[str] = []
    text = _read_regular_text(ROOT / "README.md", label="README.md")
    cargo_resolver_command = re.compile(r"\bcargo\s+(?:build|check|clippy|run|test)\b")
    for line_no, line in enumerate(text.splitlines(), 1):
        if cargo_resolver_command.search(line) and "--locked" not in line:
            problems.append(
                f"README.md:{line_no}: dependency-resolving Cargo example omits `--locked`"
            )
    return problems


def flake_reproducibility_problems() -> list[str]:
    """Require one minimal, content-addressed nixpkgs flake input."""

    problems: list[str] = []
    flake_text = _read_regular_text(ROOT / "flake.nix", label="flake.nix")
    expected_url = f'inputs.nixpkgs.url = "github:NixOS/nixpkgs/{NIXPKGS_CHANNEL}";'
    if flake_text.count(expected_url) != 1:
        problems.append(
            f"flake.nix must declare one exact nixpkgs input for {NIXPKGS_CHANNEL}"
        )
    if "flake-utils" in flake_text:
        problems.append("flake.nix must not retain the unnecessary flake-utils input")
    for fragment in (
        "uvPinned = mkPinnedArchiveTool {",
        f'version = "{NIX_UV_VERSION}";',
        'repository = "astral-sh/uv";',
        "justPinned = mkPinnedArchiveTool {",
        f'version = "{NIX_JUST_VERSION}";',
        'repository = "casey/just";',
        'assert pkgs.lib.versionAtLeast pkgs.rustc.version "1.93.0";',
        'assert pkgs.python311.version == "3.11.15";',
    ):
        if fragment not in flake_text:
            problems.append(f"flake.nix omits pinned toolchain fragment {fragment!r}")
    packages_match = re.search(
        r"packages = (?:with pkgs; )?\[(?P<body>.*?)\];",
        flake_text,
        flags=re.DOTALL,
    )
    if packages_match is None or any(
        tool not in packages_match.group("body") for tool in ("uvPinned", "justPinned")
    ):
        problems.append(
            "flake.nix dev shell must use the exact pinned uv and just tools"
        )
    for line_no, line in enumerate(flake_text.splitlines(), 1):
        if re.search(r"\bcargo (?:build|clippy|run|test)\b", line) and (
            "--locked" not in line
        ):
            problems.append(
                f"flake.nix:{line_no}: suggested Cargo command omits `--locked`"
            )
        if re.search(r"\buv sync\b", line) and "--locked" not in line:
            problems.append(f"flake.nix:{line_no}: suggested uv sync omits `--locked`")

    lock = _json_object(ROOT / "flake.lock", label="flake.lock")
    if set(lock) != {"nodes", "root", "version"}:
        problems.append("flake.lock must contain only nodes, root, and version")
    if lock.get("version") != 7:
        problems.append("flake.lock schema version must be 7")
    if lock.get("root") != "root":
        problems.append("flake.lock root node must be named root")

    nodes = lock.get("nodes")
    if not isinstance(nodes, dict):
        raise TruthAuditError("flake.lock nodes must be an object")
    if set(nodes) != {"nixpkgs", "root"}:
        problems.append("flake.lock must contain exactly the nixpkgs and root nodes")

    root_node = nodes.get("root")
    if not isinstance(root_node, dict):
        raise TruthAuditError("flake.lock root node must be an object")
    if root_node != {"inputs": {"nixpkgs": "nixpkgs"}}:
        problems.append("flake.lock root must bind only the nixpkgs input")

    nixpkgs_node = nodes.get("nixpkgs")
    if not isinstance(nixpkgs_node, dict):
        raise TruthAuditError("flake.lock nixpkgs node must be an object")
    if set(nixpkgs_node) != {"locked", "original"}:
        problems.append("flake.lock nixpkgs node must contain only locked and original")

    original = nixpkgs_node.get("original")
    expected_original = {
        "owner": "NixOS",
        "ref": NIXPKGS_CHANNEL,
        "repo": "nixpkgs",
        "type": "github",
    }
    if original != expected_original:
        problems.append(
            "flake.lock original nixpkgs input does not match "
            f"github:NixOS/nixpkgs/{NIXPKGS_CHANNEL}"
        )

    locked = nixpkgs_node.get("locked")
    if not isinstance(locked, dict):
        raise TruthAuditError("flake.lock locked nixpkgs input must be an object")
    if set(locked) != {
        "lastModified",
        "narHash",
        "owner",
        "repo",
        "rev",
        "type",
    }:
        problems.append("flake.lock locked nixpkgs input has an unexpected field set")
    if (
        locked.get("owner") != "NixOS"
        or locked.get("repo") != "nixpkgs"
        or locked.get("type") != "github"
    ):
        problems.append(
            "flake.lock locked nixpkgs repository identity is not canonical"
        )
    revision = locked.get("rev")
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        problems.append(
            "flake.lock nixpkgs revision must be one lowercase 40-hex commit"
        )
    nar_hash = locked.get("narHash")
    if (
        not isinstance(nar_hash, str)
        or re.fullmatch(r"sha256-[A-Za-z0-9+/]{43}=", nar_hash) is None
    ):
        problems.append("flake.lock nixpkgs narHash must be one SHA-256 SRI digest")
    last_modified = locked.get("lastModified")
    if (
        not isinstance(last_modified, int)
        or isinstance(last_modified, bool)
        or last_modified <= 0
    ):
        problems.append("flake.lock nixpkgs lastModified must be a positive integer")
    return problems


def precommit_reproducibility_problems() -> list[str]:
    """Reject mutable hook revisions and ambiguous installation guidance."""

    problems: list[str] = []
    text = _read_regular_text(
        ROOT / ".pre-commit-config.yaml", label=".pre-commit-config.yaml"
    )
    required_fragments = (
        f"`uv tool install pre-commit=={PRE_COMMIT_VERSION} && pre-commit install`",
        "repo: https://github.com/gitleaks/gitleaks",
        f"# Immutable commit for the verified v{GITLEAKS_VERSION} release.",
        f"rev: {GITLEAKS_REVISION}",
    )
    for fragment in required_fragments:
        if text.count(fragment) != 1:
            problems.append(
                ".pre-commit-config.yaml must contain one exact occurrence of "
                f"{fragment!r}"
            )
    if re.search(r"\bpip(?:3)? install pre-commit\b", text):
        problems.append(
            ".pre-commit-config.yaml must not recommend an unpinned pip installation"
        )
    revisions = re.findall(r"(?m)^\s+rev:\s*(\S+)\s*$", text)
    if not revisions:
        problems.append(".pre-commit-config.yaml contains no pinned hook revisions")
    for revision in revisions:
        if re.fullmatch(r"[0-9a-f]{40}", revision) is None:
            problems.append(
                ".pre-commit-config.yaml hook revisions must be lowercase 40-hex commits"
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
    problems.extend(root_release_identity_problems())

    worktree_revision = git_output("-C", "pid-rs", "rev-parse", "HEAD")
    if worktree_revision != revision:
        problems.append(
            f"pid-rs worktree {worktree_revision} does not match gitlink {revision}"
        )

    required_claims = {
        ROOT / "grandplan.md": (version, revision),
        ROOT / "AGENTS.md": (version, short, "no 1.x compatibility promise"),
        ROOT / "README.md": (version, short, "no 1.x compatibility promise"),
        ROOT / "LIMITATIONS.md": (version, short, "no 1.x compatibility promise"),
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
    problems.extend(exp0_documentation_problems())
    problems.extend(justfile_reproducibility_problems())
    problems.extend(readme_reproducibility_problems())
    problems.extend(flake_reproducibility_problems())
    problems.extend(precommit_reproducibility_problems())

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
        "galadriel": "80506dd2ce52b33c3334c7d1760a8155c7631241",
        "crebain": "0a58a5b8dd799884ddb06f1308b1748216fab322",
        "manwe": "6d73405bbf5365039ee1d0db9c466ed6346a9c57",
        "haldir": "555108666cb82e8a36dcd4b08b5b30c62367a6f4",
        "cortexel": "d29669e6d5b1766fd96e1eacefb02b3f43c5ce61",
        "melkor": "529260f568c62250b0541a11f5c24b45767bf1cf",
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
            "pid-rs": revision,
            "NCP": "v0.8.0",
            "galadriel": "80506dd2ce52b33c3334c7d1760a8155c7631241",
            "crebain": "0a58a5b8dd799884ddb06f1308b1748216fab322",
            "manwe": "6d73405bbf5365039ee1d0db9c466ed6346a9c57",
            "engram": "a4ce6ab9897dd3f1265b4cacc53f0afc349087cd",
            "haldir": "555108666cb82e8a36dcd4b08b5b30c62367a6f4",
            "cortexel": "d29669e6d5b1766fd96e1eacefb02b3f43c5ce61",
            "melkor": "529260f568c62250b0541a11f5c24b45767bf1cf",
        }
        for project, expected_revision in expected_revisions.items():
            if overrides.get(project, {}).get("observed_revision") != expected_revision:
                problems.append(
                    "ecosystem evidence overlay has an unreconciled reviewed revision for "
                    f"{project}"
                )
        pid_override = overrides.get("pid-rs", {})
        if pid_override.get("source") != (
            f"https://github.com/sepahead/pid-rs/tree/{revision}"
        ):
            problems.append(
                "ecosystem evidence overlay pid-rs source does not match the indexed gitlink"
            )
        upstream_head_observed = pid_override.get("upstream_head_observed")
        if (
            not isinstance(upstream_head_observed, str)
            or re.fullmatch(r"[0-9a-f]{40}", upstream_head_observed) is None
        ):
            raise TruthAuditError(
                "ecosystem evidence overlay pid-rs upstream-head observation is invalid"
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
            "pid-rs": (
                "0.9.0 post-tag review source",
                "additional unadopted scientific-contract and exact-certifier work",
                "no 1.x compatibility promise",
                "fixtures do not establish high-dimensional VLA application validity",
            ),
            "galadriel": (
                "no reciprocal Prisoma pin",
                "producer-consumer golden fixture",
                "no direct Prisoma adapter",
            ),
            "crebain": (
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
            "cortexel": (
                "deterministic accessible SVG export",
                "no published package or DOI",
                "does not supersede",
            ),
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
