#!/usr/bin/env python3
"""Require immutable action pins and reproducible CI runtime/dependency settings."""

from __future__ import annotations

import argparse
import os
import re
import stat
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
WORKFLOW_DIR = Path(".github/workflows")
CANONICAL_USES_RE = re.compile(
    r"^\s*(?:-\s+)?uses:\s+"
    r"(?:\"(?P<double>[^\"]+)\"|'(?P<single>[^']+)'|(?P<plain>[^\s#]+))"
    r"(?:\s+#.*)?\s*$"
)
BLOCK_USES_KEY_RE = re.compile(r"^\s*(?:-\s*)?(?:uses|['\"]uses['\"])\s*:")
EXPLICIT_USES_KEY_RE = re.compile(
    r"^\s*(?:-\s*)?\?\s*(?:uses|['\"]uses['\"])(?:\s|:|$)"
)
FLOW_USES_KEY_RE = re.compile(r"(?:^|[,{])\s*(?:uses|['\"]uses['\"])\s*:")
BLOCK_SCALAR_HEADER_RE = re.compile(r":\s*[|>][0-9+-]*\s*$")
QUOTED_MAPPING_KEY_RE = re.compile(
    r"^\s*(?:-\s*)?(?:\"(?:[^\"\\]|\\.)*\"|'(?:[^']|'')*')\s*:"
)
FLOW_STEP_RE = re.compile(r"^\s*-\s*\{")
ANY_EXPLICIT_KEY_RE = re.compile(r"^\s*(?:-\s*)?\?(?:\s|$)")
TAGGED_STEP_RE = re.compile(r"^\s*(?:-\s*)?!")
YAML_COMPOSITION_RE = re.compile(
    r"(?:^|[\s\[\]{},:])(?:[&*][A-Za-z0-9_-]+|<<\s*:)(?=$|[\s\[\]{},])"
)
FLOW_COLLECTION_RE = re.compile(r"[\[\]{}]")
YAML_TAG_RE = re.compile(r"(?:^|[\s\[\]{},:])![^\s]*")
COMMIT_RE = re.compile(r"[0-9a-f]{40}")
EXACT_VERSION_RE = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")
PYTHON_VERSION_RE = re.compile(
    r"^\s*python-version:\s*[\"']?(?P<version>[^\"'\s#]+)[\"']?\s*(?:#.*)?$"
)
RUNNER_RE = re.compile(r"^\s*runs-on:\s*[\"']?(?P<runner>[^\"'\s#]+)[\"']?\s*(?:#.*)?$")
CARGO_RESOLUTION_RE = re.compile(
    r"(?<![A-Za-z0-9_-])cargo\s+"
    r"(?P<command>build|check|clippy|metadata|run|test|tree)\b"
)
UV_SYNC_RE = re.compile(r"(?<![A-Za-z0-9_-])uv\s+sync\b")
MATURIN_DEVELOP_RE = re.compile(r"(?<![A-Za-z0-9_-])maturin\s+develop\b")
RUSTUP_TOOLCHAIN_RE = re.compile(
    r"(?<![A-Za-z0-9_-])rustup\s+toolchain\s+install\s+(?P<version>\S+)"
)
CURL_RE = re.compile(r"(?<![A-Za-z0-9_-])curl(?:\s|$)")
PYTHON_ASSERT_RE = re.compile(r"(?<![A-Za-z0-9_])assert\s+")
MAX_WORKFLOW_BYTES = 1024 * 1024
MAX_WORKFLOW_FILES = 64
MAX_WORKFLOW_DIRECTORY_ENTRIES = 256
MAX_WORKFLOW_AGGREGATE_BYTES = 8 * 1024 * 1024


def _snapshot_identity(value: os.stat_result) -> tuple[int, int, int, int, int, int]:
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _read_workflow(path: Path) -> str:
    try:
        named_before = os.stat(path, follow_symlinks=False)
    except OSError as exc:
        raise ValueError(f"cannot inspect workflow: {exc}") from exc
    if not stat.S_ISREG(named_before.st_mode):
        raise ValueError("workflow must be a regular non-symlink file")
    if named_before.st_size > MAX_WORKFLOW_BYTES:
        raise ValueError(f"workflow exceeds {MAX_WORKFLOW_BYTES} bytes")
    flags = (
        os.O_RDONLY
        | getattr(os, "O_CLOEXEC", 0)
        | getattr(os, "O_NOFOLLOW", 0)
        | getattr(os, "O_NONBLOCK", 0)
    )
    try:
        descriptor = os.open(path, flags)
    except OSError as exc:
        raise ValueError(f"cannot open workflow: {exc}") from exc
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or (opened.st_dev, opened.st_ino) != (
            named_before.st_dev,
            named_before.st_ino,
        ):
            raise ValueError("workflow path changed while opening")
        raw = bytearray()
        while len(raw) <= MAX_WORKFLOW_BYTES:
            chunk = os.read(
                descriptor,
                min(1024 * 1024, MAX_WORKFLOW_BYTES + 1 - len(raw)),
            )
            if not chunk:
                break
            raw.extend(chunk)
        if len(raw) > MAX_WORKFLOW_BYTES:
            raise ValueError(f"workflow exceeds {MAX_WORKFLOW_BYTES} bytes")
        opened_after = os.fstat(descriptor)
        named_after = os.stat(path, follow_symlinks=False)
        if (
            _snapshot_identity(opened) != _snapshot_identity(opened_after)
            or _snapshot_identity(opened_after) != _snapshot_identity(named_after)
            or len(raw) != opened_after.st_size
        ):
            raise ValueError("workflow changed while it was read")
    except OSError as exc:
        raise ValueError(f"cannot read stable workflow snapshot: {exc}") from exc
    finally:
        os.close(descriptor)
    try:
        return bytes(raw).decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise ValueError(f"workflow is not UTF-8: {exc}") from exc


def _strip_yaml_comment(line: str) -> str:
    single_quoted = False
    double_quoted = False
    escaped = False
    for index, character in enumerate(line):
        if double_quoted:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                double_quoted = False
            continue
        if single_quoted:
            if character == "'":
                if index + 1 < len(line) and line[index + 1] == "'":
                    continue
                single_quoted = False
            continue
        if character == '"':
            double_quoted = True
        elif character == "'":
            single_quoted = True
        elif character == "#" and (index == 0 or line[index - 1].isspace()):
            return line[:index].rstrip()
    return line.rstrip()


def _has_double_quoted_escape(line: str) -> bool:
    double_quoted = False
    escaped = False
    for character in line:
        if not double_quoted:
            if character == '"':
                double_quoted = True
            continue
        if escaped:
            escaped = False
            continue
        if character == "\\":
            return True
        if character == '"':
            double_quoted = False
    return False


def _mask_quoted_and_expressions(line: str) -> str:
    """Mask scalar text and GitHub expressions, leaving YAML structure visible."""

    masked = list(line)
    index = 0
    while index < len(line):
        if line.startswith("${{", index):
            end = line.find("}}", index + 3)
            if end < 0:
                break
            for position in range(index, end + 2):
                masked[position] = " "
            index = end + 2
            continue
        quote = line[index]
        if quote not in {'"', "'"}:
            index += 1
            continue
        masked[index] = " "
        index += 1
        while index < len(line):
            character = line[index]
            masked[index] = " "
            if quote == '"' and character == "\\" and index + 1 < len(line):
                index += 1
                masked[index] = " "
            elif character == quote:
                if quote == "'" and index + 1 < len(line) and line[index + 1] == "'":
                    index += 1
                    masked[index] = " "
                else:
                    index += 1
                    break
            index += 1
    return "".join(masked)


def _workflow_code_lines(text: str) -> list[tuple[int, str]]:
    """Return YAML source lines while excluding literal/folded scalar bodies."""

    result: list[tuple[int, str]] = []
    block_parent_indent: int | None = None
    for line_number, line in enumerate(text.splitlines(), start=1):
        stripped = line.lstrip(" ")
        indent = len(line) - len(stripped)
        if block_parent_indent is not None:
            if not stripped:
                continue
            if indent > block_parent_indent:
                continue
            block_parent_indent = None
        code = _strip_yaml_comment(line)
        result.append((line_number, code))
        if BLOCK_SCALAR_HEADER_RE.search(code) is not None:
            compact_mapping = re.match(r"^-([ ]+)", stripped)
            compact_mapping_offset = (
                1 + len(compact_mapping.group(1)) if compact_mapping is not None else 0
            )
            block_parent_indent = indent + compact_mapping_offset
    return result


def audit_text(path: Path, text: str) -> tuple[int, list[str]]:
    """Return the number of references and any pin errors in one workflow."""
    errors: list[str] = []
    references: list[str] = []
    candidate_count = 0

    for line_number, line in _workflow_code_lines(text):
        if line.lstrip().startswith("#"):
            continue

        canonical = CANONICAL_USES_RE.fullmatch(line)
        if canonical is not None:
            candidate_count += 1
            reference = next(
                value
                for value in (
                    canonical.group("double"),
                    canonical.group("single"),
                    canonical.group("plain"),
                )
                if value is not None
            )
            references.append(reference)
            continue

        block_candidate = BLOCK_USES_KEY_RE.search(line) is not None
        explicit_candidate = EXPLICIT_USES_KEY_RE.search(line) is not None
        flow_candidates = len(FLOW_USES_KEY_RE.findall(line))
        noncanonical_count = max(
            int(block_candidate), int(explicit_candidate), flow_candidates
        )
        if noncanonical_count:
            candidate_count += noncanonical_count
            errors.append(
                f"{path}:{line_number}: uses key must use canonical block form "
                "`- uses: owner/action@<40-hex>`"
            )
            continue

        structural = _mask_quoted_and_expressions(line)
        unsafe_yaml = (
            QUOTED_MAPPING_KEY_RE.search(line) is not None
            or FLOW_STEP_RE.search(line) is not None
            or FLOW_COLLECTION_RE.search(structural) is not None
            or ANY_EXPLICIT_KEY_RE.search(line) is not None
            or TAGGED_STEP_RE.search(line) is not None
            or YAML_TAG_RE.search(structural) is not None
            or YAML_COMPOSITION_RE.search(structural) is not None
            or line.lstrip().startswith("%")
            or _has_double_quoted_escape(line)
        )
        if unsafe_yaml:
            candidate_count += 1
            errors.append(
                f"{path}:{line_number}: workflow YAML must use canonical block mappings; "
                "quoted/escaped keys, explicit keys, flow steps, tags, anchors, aliases, "
                "merges, and directives are forbidden"
            )

    for reference in references:
        if reference.startswith("./"):
            errors.append(
                f"{path}: local actions are forbidden because nested action references "
                f"are outside this workflow-only audit: {reference}"
            )
            continue
        action, separator, revision = reference.rpartition("@")
        if not separator or not action or not COMMIT_RE.fullmatch(revision):
            errors.append(
                f"{path}: external action must use a lowercase 40-hex commit pin: "
                f"{reference}"
            )
    return candidate_count, errors


def _step_body(lines: list[str], index: int) -> list[str]:
    step_line = lines[index]
    step_indent = len(step_line) - len(step_line.lstrip(" "))
    result: list[str] = []
    for line in lines[index + 1 :]:
        stripped = line.lstrip(" ")
        if not stripped:
            result.append(line)
            continue
        indent = len(line) - len(stripped)
        if indent < step_indent or (
            indent == step_indent and stripped.startswith("- ")
        ):
            break
        result.append(line)
    return result


def _step_scalar(body: list[str], key: str) -> str | None:
    pattern = re.compile(
        rf"^\s*{re.escape(key)}:\s*[\"']?(?P<value>[^\"'\s#]+)[\"']?"
        r"\s*(?:#.*)?$"
    )
    for line in body:
        match = pattern.fullmatch(line)
        if match is not None:
            return match.group("value")
    return None


def _continued_command(lines: list[str], index: int) -> str:
    parts: list[str] = []
    while index < len(lines):
        code = _strip_yaml_comment(lines[index]).strip()
        continued = code.endswith("\\")
        parts.append(code[:-1].rstrip() if continued else code)
        index += 1
        if not continued:
            break
    return " ".join(parts)


def audit_reproducibility_text(path: Path, text: str) -> list[str]:
    """Reject mutable CI runtimes and dependency-resolving commands without locks."""

    errors: list[str] = []
    lines = text.splitlines()
    for line_number, line in enumerate(lines, start=1):
        code = _strip_yaml_comment(line)
        stripped = code.lstrip()
        if not stripped or stripped.startswith("#"):
            continue

        if PYTHON_ASSERT_RE.search(code) is not None:
            errors.append(
                f"{path}:{line_number}: CI checks may not use Python `assert`; "
                "use an explicit exception or exit status"
            )

        runner = RUNNER_RE.fullmatch(code)
        if runner is not None:
            value = runner.group("runner")
            if value.endswith("-latest") or "${{" in value:
                errors.append(
                    f"{path}:{line_number}: runner must use a static versioned image, "
                    f"not {value}"
                )

        python_version = PYTHON_VERSION_RE.fullmatch(code)
        if python_version is not None:
            value = python_version.group("version")
            if EXACT_VERSION_RE.fullmatch(value) is None:
                errors.append(
                    f"{path}:{line_number}: python-version must pin an exact patch "
                    f"release, not {value}"
                )

        for command in CARGO_RESOLUTION_RE.finditer(code):
            command_tail = code[command.start() :]
            command_segment = re.split(r"[|;&]", command_tail, maxsplit=1)[0]
            if "--locked" not in command_segment.split():
                errors.append(
                    f"{path}:{line_number}: cargo {command.group('command')} must "
                    "use --locked"
                )

        if UV_SYNC_RE.search(code) is not None:
            command_tail = code[UV_SYNC_RE.search(code).start() :]
            command_segment = re.split(r"[|;&]", command_tail, maxsplit=1)[0]
            if "--locked" not in command_segment.split():
                errors.append(f"{path}:{line_number}: uv sync must use --locked")

        if MATURIN_DEVELOP_RE.search(code) is not None:
            command_tail = code[MATURIN_DEVELOP_RE.search(code).start() :]
            command_segment = re.split(r"[|;&]", command_tail, maxsplit=1)[0]
            if "--locked" not in command_segment.split():
                errors.append(
                    f"{path}:{line_number}: maturin develop must use --locked"
                )

        rustup = RUSTUP_TOOLCHAIN_RE.search(code)
        if rustup is not None:
            command = _continued_command(lines, line_number - 1)
            version = rustup.group("version")
            if EXACT_VERSION_RE.fullmatch(version) is None:
                errors.append(
                    f"{path}:{line_number}: rustup must install an exact toolchain "
                    f"version, not {version}"
                )
            if "--no-self-update" not in command.split():
                errors.append(
                    f"{path}:{line_number}: rustup toolchain installation must disable "
                    "self-update"
                )

        if CURL_RE.search(code) is not None:
            command = _continued_command(lines, line_number - 1)
            required_flags = {
                "--connect-timeout",
                "--fail",
                "--location",
                "--max-filesize",
                "--max-time",
                "--output",
                "--proto",
                "--proto-redir",
                "--retry",
                "--retry-all-errors",
                "--retry-max-time",
                "--tlsv1.2",
            }
            missing = sorted(required_flags.difference(command.split()))
            if missing:
                errors.append(
                    f"{path}:{line_number}: curl download omits bounded HTTPS flags: "
                    f"{', '.join(missing)}"
                )
            checksum_window = "\n".join(lines[line_number : line_number + 12])
            if "sha256sum --check --strict" not in checksum_window:
                errors.append(
                    f"{path}:{line_number}: curl download must be followed by strict "
                    "SHA-256 verification"
                )
            if "/latest/" in command:
                errors.append(
                    f"{path}:{line_number}: curl download may not use a mutable latest URL"
                )

        if "uses: dtolnay/rust-toolchain@" in code:
            toolchain = _step_scalar(_step_body(lines, line_number - 1), "toolchain")
            if toolchain is None or EXACT_VERSION_RE.fullmatch(toolchain) is None:
                errors.append(
                    f"{path}:{line_number}: rust-toolchain action must pin an exact "
                    "toolchain version"
                )

        if "uses: astral-sh/setup-uv@" in code:
            uv_version = _step_scalar(_step_body(lines, line_number - 1), "version")
            if uv_version is None or EXACT_VERSION_RE.fullmatch(uv_version) is None:
                errors.append(
                    f"{path}:{line_number}: setup-uv must pin an exact uv version"
                )

        if "uses: actions/checkout@" in code:
            persist = _step_scalar(
                _step_body(lines, line_number - 1), "persist-credentials"
            )
            if persist != "false":
                errors.append(
                    f"{path}:{line_number}: checkout must disable persisted credentials"
                )
    return errors


def audit_workflows(repo_root: Path) -> tuple[int, list[str]]:
    workflow_root = repo_root / WORKFLOW_DIR
    try:
        workflow_metadata = os.stat(workflow_root, follow_symlinks=False)
    except OSError as exc:
        return 0, [f"{workflow_root}: cannot inspect workflow directory: {exc}"]
    if not stat.S_ISDIR(workflow_metadata.st_mode):
        return 0, [f"{workflow_root}: workflow directory must be a real directory"]
    files: list[Path] = []
    try:
        with os.scandir(workflow_root) as iterator:
            for entry_count, entry in enumerate(iterator, start=1):
                if entry_count > MAX_WORKFLOW_DIRECTORY_ENTRIES:
                    return 0, [
                        f"{workflow_root}: workflow directory exceeds "
                        f"{MAX_WORKFLOW_DIRECTORY_ENTRIES} entries"
                    ]
                if Path(entry.name).suffix not in {".yml", ".yaml"}:
                    continue
                files.append(Path(entry.path))
                if len(files) > MAX_WORKFLOW_FILES:
                    return 0, [
                        f"{workflow_root}: more than {MAX_WORKFLOW_FILES} workflow files"
                    ]
    except OSError as exc:
        return 0, [f"{workflow_root}: cannot enumerate workflows: {exc}"]
    files.sort()
    if not files:
        return 0, [f"{workflow_root}: no workflow files found"]

    count = 0
    errors: list[str] = []
    aggregate_bytes = 0
    for path in files:
        relative = path.relative_to(repo_root)
        try:
            text = _read_workflow(path)
        except ValueError as exc:
            errors.append(f"{relative}: {exc}")
            continue
        aggregate_bytes += len(text.encode("utf-8"))
        if aggregate_bytes > MAX_WORKFLOW_AGGREGATE_BYTES:
            errors.append(
                f"{workflow_root}: workflow bytes exceed "
                f"{MAX_WORKFLOW_AGGREGATE_BYTES} aggregate bytes"
            )
            break
        references, file_errors = audit_text(relative, text)
        count += references
        errors.extend(file_errors)
        errors.extend(audit_reproducibility_text(relative, text))
    if count == 0:
        errors.append(f"{workflow_root}: no action references found")
    return count, errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    args = parser.parse_args(argv)

    count, errors = audit_workflows(args.repo_root.resolve())
    if errors:
        print("CI reproducibility audit failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print(f"CI reproducibility audit passed: {count} action reference(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
