#!/usr/bin/env python3
"""Run SMT obligations with bounded input/output and wall-clock execution."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import selectors
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FORMAL_DIR = ROOT / "formal"
REGISTRY_PATH = FORMAL_DIR / "model_registry.json"
MAX_MODEL_BYTES = 256 * 1024
MAX_OUTPUT_BYTES = 64 * 1024
TIMEOUT_SECONDS = 15.0
WALLCLOCK_GRACE_SECONDS = 2.0
REQUIRED_Z3_VERSION = "Z3 version 4.16.0 - 64 bit"
MAX_VERSION_OUTPUT_BYTES = 4096

ALLOWED_TOP_LEVEL_COMMANDS = frozenset(
    {
        "set-logic",
        "declare-sort",
        "declare-fun",
        "declare-const",
        "define-fun",
        "assert",
        "push",
        "pop",
        "check-sat",
    }
)


class _DuplicateJsonKeyError(ValueError):
    """Raised when a supposedly strict JSON object repeats a member name."""


def _strict_json_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    document: dict[str, object] = {}
    for key, value in pairs:
        if key in document:
            raise _DuplicateJsonKeyError(f"duplicate JSON member {key!r}")
        document[key] = value
    return document


def _same_file_snapshot(left: os.stat_result, right: os.stat_result) -> bool:
    return (
        stat.S_IFMT(left.st_mode) == stat.S_IFMT(right.st_mode)
        and left.st_dev == right.st_dev
        and left.st_ino == right.st_ino
        and left.st_size == right.st_size
        and left.st_mtime_ns == right.st_mtime_ns
        and left.st_ctime_ns == right.st_ctime_ns
    )


def _read_bounded_regular_file(
    path: Path, *, max_bytes: int, description: str
) -> bytes:
    """Snapshot one bounded regular file without following a final symlink."""

    try:
        path_before = path.lstat()
    except OSError as exc:
        raise RuntimeError(f"cannot inspect {description} {path}: {exc}") from exc
    if not stat.S_ISREG(path_before.st_mode):
        raise RuntimeError(f"{description} must be a regular non-symlink file: {path}")
    if path_before.st_size > max_bytes:
        raise RuntimeError(f"{description} exceeds {max_bytes} bytes: {path}")

    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as exc:
        raise RuntimeError(f"cannot open {description} {path}: {exc}") from exc
    try:
        opened_before = os.fstat(descriptor)
        if not stat.S_ISREG(opened_before.st_mode) or not _same_file_snapshot(
            path_before, opened_before
        ):
            raise RuntimeError(f"{description} changed while opening: {path}")
        chunks: list[bytes] = []
        remaining = max_bytes + 1
        while remaining > 0:
            chunk = os.read(descriptor, min(65_536, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        contents = b"".join(chunks)
        if len(contents) > max_bytes:
            raise RuntimeError(f"{description} exceeds {max_bytes} bytes: {path}")
        opened_after = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    try:
        path_after = path.lstat()
    except OSError as exc:
        raise RuntimeError(
            f"{description} changed while reading {path}: {exc}"
        ) from exc
    if not _same_file_snapshot(opened_before, opened_after) or not _same_file_snapshot(
        opened_after, path_after
    ):
        raise RuntimeError(f"{description} changed while reading: {path}")
    return contents


def _load_registry() -> tuple[dict[str, tuple[str, ...]], dict[str, str]]:
    raw = _read_bounded_regular_file(
        REGISTRY_PATH,
        max_bytes=MAX_MODEL_BYTES,
        description="formal model registry",
    )
    try:
        document = json.loads(raw, object_pairs_hook=_strict_json_object)
    except (UnicodeDecodeError, json.JSONDecodeError, _DuplicateJsonKeyError) as exc:
        raise RuntimeError(f"formal model registry is invalid JSON: {exc}") from exc
    if not isinstance(document, dict) or set(document) != {
        "schema_version",
        "z3_version",
        "models",
    }:
        raise RuntimeError("formal model registry has an unexpected shape")
    if document["schema_version"] != 1:
        raise RuntimeError("formal model registry schema_version must be 1")
    if document["z3_version"] != REQUIRED_Z3_VERSION:
        raise RuntimeError("formal model registry Z3 version does not match the runner")
    models = document["models"]
    if not isinstance(models, dict) or not models:
        raise RuntimeError(
            "formal model registry must contain a nonempty models object"
        )

    expected: dict[str, tuple[str, ...]] = {}
    digests: dict[str, str] = {}
    for name, entry in models.items():
        if (
            not isinstance(name, str)
            or not name.endswith(".smt2")
            or Path(name).name != name
            or not isinstance(entry, dict)
            or set(entry) != {"expected", "sha256"}
        ):
            raise RuntimeError("formal model registry contains an invalid model entry")
        results = entry["expected"]
        digest = entry["sha256"]
        if (
            not isinstance(results, list)
            or not results
            or any(result not in {"sat", "unsat"} for result in results)
            or not isinstance(digest, str)
            or len(digest) != 64
            or any(char not in "0123456789abcdef" for char in digest)
        ):
            raise RuntimeError(f"invalid formal registry entry for {name}")
        expected[name] = tuple(results)
        digests[name] = digest
    return expected, digests


EXPECTED, MODEL_SHA256 = _load_registry()


def _read_regular_model(path: Path) -> bytes:
    """Snapshot one bounded regular model without following a final symlink."""

    return _read_bounded_regular_file(
        path, max_bytes=MAX_MODEL_BYTES, description="formal model"
    )


def _smt_tokens(source: str) -> tuple[str, ...]:
    """Tokenize enough SMT-LIB to identify top-level commands without comment spoofing."""

    tokens: list[str] = []
    index = 0
    while index < len(source):
        char = source[index]
        if char.isspace():
            index += 1
            continue
        if char == ";":
            newline = source.find("\n", index + 1)
            index = len(source) if newline < 0 else newline + 1
            continue
        if char in "()":
            tokens.append(char)
            index += 1
            continue
        if char == '"':
            index += 1
            while index < len(source):
                if source[index] == '"':
                    if index + 1 < len(source) and source[index + 1] == '"':
                        index += 2
                        continue
                    index += 1
                    tokens.append("<string>")
                    break
                index += 1
            else:
                raise RuntimeError(
                    "formal model contains an unterminated SMT-LIB string"
                )
            continue
        if char == "|":
            index += 1
            while index < len(source):
                if source[index] == "\\":
                    raise RuntimeError(
                        "formal model contains a backslash in an SMT-LIB quoted symbol"
                    )
                if source[index] == "|":
                    index += 1
                    tokens.append("<quoted-symbol>")
                    break
                index += 1
            else:
                raise RuntimeError(
                    "formal model contains an unterminated quoted symbol"
                )
            continue
        end = index
        while (
            end < len(source) and not source[end].isspace() and source[end] not in "();"
        ):
            end += 1
        if end == index:
            raise RuntimeError(
                f"formal model contains an invalid token at offset {index}"
            )
        tokens.append(source[index:end])
        index = end
    return tuple(tokens)


SExpression = str | tuple["SExpression", ...]


def _top_level_forms(model: bytes) -> tuple[tuple[SExpression, ...], ...]:
    try:
        source = model.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise RuntimeError("formal model is not valid UTF-8") from exc
    if "\x00" in source:
        raise RuntimeError("formal model contains a NUL byte")

    stack: list[list[SExpression]] = []
    forms: list[tuple[SExpression, ...]] = []
    for token in _smt_tokens(source):
        if token == "(":
            stack.append([])
        elif token == ")":
            if not stack:
                raise RuntimeError("formal model has an unmatched closing parenthesis")
            completed = tuple(stack.pop())
            if stack:
                stack[-1].append(completed)
            else:
                if not completed:
                    raise RuntimeError(
                        "formal model contains an empty top-level command"
                    )
                forms.append(completed)
        elif not stack:
            raise RuntimeError(
                "formal model contains an atom outside a top-level command"
            )
        else:
            stack[-1].append(token)
    if stack:
        raise RuntimeError("formal model has unbalanced parentheses")
    return tuple(forms)


def _top_level_commands(model: bytes) -> tuple[str, ...]:
    commands: list[str] = []
    for form in _top_level_forms(model):
        head = form[0]
        if not isinstance(head, str):
            raise RuntimeError("formal model contains a non-symbol command head")
        commands.append(head)
    return tuple(commands)


def _validate_model_source(
    model: bytes,
    expected: tuple[str, ...],
    name: str,
    *,
    verify_digest: bool = True,
) -> None:
    if not expected or any(result not in {"sat", "unsat"} for result in expected):
        raise RuntimeError(f"invalid expected-result registry entry for {name}")
    if verify_digest:
        actual_digest = hashlib.sha256(model).hexdigest()
        registered_digest = MODEL_SHA256.get(name)
        if registered_digest is None:
            raise RuntimeError(
                f"formal model {name} is absent from the digest registry"
            )
        if actual_digest != registered_digest:
            raise RuntimeError(
                f"formal model {name} digest is {actual_digest}; "
                f"registry requires {registered_digest}"
            )

    forms = _top_level_forms(model)
    commands = _top_level_commands(model)
    if not commands or commands[0] != "set-logic" or commands.count("set-logic") != 1:
        raise RuntimeError(
            f"formal model {name} must begin with exactly one set-logic command"
        )
    disallowed = sorted(set(commands) - ALLOWED_TOP_LEVEL_COMMANDS)
    if disallowed:
        raise RuntimeError(
            f"formal model {name} uses disallowed top-level commands: {disallowed}"
        )
    for form in forms:
        command = form[0]
        if command == "set-logic":
            if len(form) != 2 or not isinstance(form[1], str):
                raise RuntimeError(
                    f"formal model {name} has a malformed set-logic command"
                )
        elif command == "check-sat" and len(form) != 1:
            raise RuntimeError(
                f"formal model {name} requires zero-argument (check-sat)"
            )
        elif command in {"push", "pop"} and (len(form) != 2 or form[1] != "1"):
            raise RuntimeError(
                f"formal model {name} requires canonical single-level ({command} 1)"
            )
    checks = commands.count("check-sat")
    if checks != len(expected):
        raise RuntimeError(
            f"formal model {name} contains {checks} check-sat commands; "
            f"registry expects {len(expected)}"
        )
    stack_depth = 0
    for command in commands:
        if command == "push":
            stack_depth += 1
        elif command == "pop":
            if stack_depth == 0:
                raise RuntimeError(f"formal model {name} pops an empty assertion stack")
            stack_depth -= 1
    if stack_depth != 0:
        raise RuntimeError(
            f"formal model {name} leaves {stack_depth} unmatched push commands"
        )


def _posix_waitid_available() -> bool:
    return os.name == "posix" and all(
        hasattr(os, name)
        for name in ("waitid", "P_PID", "WEXITED", "WNOHANG", "WNOWAIT")
    )


def _reap_group_anchor(anchor: subprocess.Popen[bytes]) -> None:
    """Stop and reap the private POSIX process-group anchor."""

    if anchor.returncode is None:
        try:
            anchor.kill()
        except OSError:
            pass
    try:
        anchor.wait(timeout=2.0)
    except (ChildProcessError, subprocess.TimeoutExpired):
        pass


def _kill_owned_process_group(
    pgid: int, *, allow_darwin_empty_group: bool = False
) -> None:
    """Signal a process group while an unreaped process still owns its PGID."""

    try:
        os.killpg(pgid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    except PermissionError:
        if not (allow_darwin_empty_group and sys.platform == "darwin"):
            raise
        # Darwin reports EPERM rather than ESRCH for kill(-pgid, signal) when
        # waitid observes an exited, unreaped session leader but no signalable
        # process remains in its group.


def _start_owned_tool(
    command: list[str],
) -> tuple[subprocess.Popen[bytes], subprocess.Popen[bytes] | None]:
    """Start a tool as the unreaped leader of its own POSIX process group."""

    common: dict[str, object] = {
        "cwd": ROOT,
        "stdin": subprocess.DEVNULL,
        "stdout": subprocess.PIPE,
        "stderr": subprocess.PIPE,
    }
    if os.name != "posix":
        return subprocess.Popen(command, **common), None
    return subprocess.Popen(command, start_new_session=True, **common), None


def _wait_for_exit_without_reaping(
    process: subprocess.Popen[bytes], timeout_seconds: float
) -> None:
    """Observe a POSIX child exit while its PID still anchors the owned process group."""

    if timeout_seconds <= 0:
        raise subprocess.TimeoutExpired(process.args, timeout_seconds)
    if not _posix_waitid_available():
        raise RuntimeError(
            "safe POSIX descendant cleanup requires waitid with WNOWAIT support"
        )

    deadline = time.monotonic() + timeout_seconds
    flags = os.WEXITED | os.WNOHANG | os.WNOWAIT
    while True:
        try:
            observation = os.waitid(os.P_PID, process.pid, flags)
        except InterruptedError:
            continue
        except ChildProcessError as exc:
            raise RuntimeError(
                "tool process leader was reaped before owned-group cleanup"
            ) from exc
        if observation is not None:
            if observation.si_pid != process.pid:
                raise RuntimeError(
                    "waitid returned an unexpected process during owned-group cleanup"
                )
            return
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise subprocess.TimeoutExpired(process.args, timeout_seconds)
        time.sleep(min(remaining, 0.01))


def _wait_and_reap_owned_process_group(
    process: subprocess.Popen[bytes],
    group_anchor: subprocess.Popen[bytes] | None,
    timeout_seconds: float,
) -> int:
    """Wait and clean same-group descendants while the PGID remains owned."""

    if os.name != "posix":
        return process.wait(timeout=timeout_seconds)

    if group_anchor is not None:
        returncode = process.wait(timeout=timeout_seconds)
        try:
            _kill_owned_process_group(group_anchor.pid)
        finally:
            _reap_group_anchor(group_anchor)
        return returncode

    if not _posix_waitid_available():
        # Both captured pipes reached EOF, but the leader has deliberately not
        # been waited or polled. Its PID therefore still reserves the process
        # group identifier. Signal the owned group before reaping the leader.
        try:
            _kill_owned_process_group(
                process.pid,
                allow_darwin_empty_group=True,
            )
        except OSError as exc:
            try:
                process.wait(timeout=2.0)
            except subprocess.TimeoutExpired:
                pass
            raise RuntimeError(
                f"cannot clean owned tool process group {process.pid}: {exc}"
            ) from exc
        return process.wait(timeout=2.0)

    _wait_for_exit_without_reaping(process, timeout_seconds)
    try:
        _kill_owned_process_group(
            process.pid,
            allow_darwin_empty_group=True,
        )
    except OSError as exc:
        try:
            process.wait(timeout=2.0)
        except subprocess.TimeoutExpired:
            pass
        raise RuntimeError(
            f"cannot clean owned tool process group {process.pid}: {exc}"
        ) from exc
    return process.wait(timeout=2.0)


def _terminate_unreaped_process(
    process: subprocess.Popen[bytes],
    group_anchor: subprocess.Popen[bytes] | None,
) -> None:
    """Best-effort cleanup that never signals a POSIX group after guard reaping."""

    if group_anchor is not None:
        if process.returncode is not None and group_anchor.returncode is not None:
            return
        try:
            _kill_owned_process_group(group_anchor.pid)
        except OSError:
            for owned_process in (process, group_anchor):
                try:
                    owned_process.kill()
                except OSError:
                    pass
        for owned_process in (process, group_anchor):
            try:
                owned_process.wait(timeout=2.0)
            except (ChildProcessError, subprocess.TimeoutExpired):
                pass
        return

    if process.returncode is not None:
        return

    if os.name == "posix":
        if _posix_waitid_available():
            try:
                # Either a running child (None) or an exited-but-unreaped child
                # keeps its PID reserved. ChildProcessError means another waiter
                # already reaped it; killpg would then be unsafe.
                os.waitid(
                    os.P_PID,
                    process.pid,
                    os.WEXITED | os.WNOHANG | os.WNOWAIT,
                )
            except InterruptedError:
                pass
            except ChildProcessError:
                return
        try:
            _kill_owned_process_group(
                process.pid,
                allow_darwin_empty_group=True,
            )
        except OSError:
            try:
                process.kill()
            except OSError:
                pass
    else:
        try:
            process.kill()
        except OSError:
            pass
    try:
        process.wait(timeout=2.0)
    except (ChildProcessError, subprocess.TimeoutExpired):
        pass


def _run_bounded_tool(
    command: list[str], *, timeout_seconds: float, max_output_bytes: int
) -> tuple[int, bytes, bytes]:
    """Run one trusted local tool with bounded output and descendant cleanup."""

    if timeout_seconds <= 0:
        raise subprocess.TimeoutExpired(command, timeout_seconds)
    if max_output_bytes < 0:
        raise RuntimeError(f"invalid negative tool output budget: {max_output_bytes}")
    process, group_anchor = _start_owned_tool(command)
    stdout_stream = process.stdout
    stderr_stream = process.stderr
    selector: selectors.BaseSelector | None = None
    buffers = {"stdout": bytearray(), "stderr": bytearray()}
    total = 0
    deadline = time.monotonic() + timeout_seconds
    try:
        if stdout_stream is None or stderr_stream is None:
            raise RuntimeError("tool subprocess pipes were not created")
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
                    raise RuntimeError(
                        f"tool output exceeded the {max_output_bytes}-byte bound"
                    )
                buffers[key.data].extend(chunk)
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise subprocess.TimeoutExpired(command, timeout_seconds)
        returncode = _wait_and_reap_owned_process_group(
            process, group_anchor, remaining
        )
    finally:
        _terminate_unreaped_process(process, group_anchor)
        if selector is not None:
            selector.close()
        for stream in (stdout_stream, stderr_stream):
            if stream is not None and not stream.closed:
                stream.close()
    return returncode, bytes(buffers["stdout"]), bytes(buffers["stderr"])


def _require_z3_version(z3: str) -> None:
    try:
        returncode, stdout, stderr = _run_bounded_tool(
            [z3, "-version"],
            timeout_seconds=5.0,
            max_output_bytes=MAX_VERSION_OUTPUT_BYTES,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise RuntimeError(f"cannot identify Z3: {exc}") from exc
    if returncode != 0 or stderr:
        detail = stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"Z3 version query failed: {detail}")
    try:
        version = stdout.decode("ascii").strip()
    except UnicodeDecodeError as exc:
        raise RuntimeError("Z3 returned a non-ASCII version string") from exc
    if version != REQUIRED_Z3_VERSION:
        raise RuntimeError(
            f"unsupported Z3 version {version!r}; required {REQUIRED_Z3_VERSION!r}"
        )


def _run_model_results(
    z3: str,
    path: Path,
    expected: tuple[str, ...],
    *,
    verify_digest: bool = True,
) -> tuple[str, ...]:
    model = _read_regular_model(path)
    _validate_model_source(model, expected, path.name, verify_digest=verify_digest)
    solver_timeout = max(1, math.ceil(TIMEOUT_SECONDS))
    with tempfile.TemporaryDirectory(prefix="prisoma-formal-") as temp_dir:
        snapshot_path = Path(temp_dir) / path.name
        snapshot_path.write_bytes(model)
        try:
            process, group_anchor = _start_owned_tool(
                [z3, f"-T:{solver_timeout}", os.fspath(snapshot_path)]
            )
        except OSError as exc:
            raise RuntimeError(f"cannot start Z3 for {path.name}: {exc}") from exc
        stdout_stream = process.stdout
        stderr_stream = process.stderr
        selector: selectors.BaseSelector | None = None
        buffers = {"stdout": bytearray(), "stderr": bytearray()}
        deadline = time.monotonic() + TIMEOUT_SECONDS + WALLCLOCK_GRACE_SECONDS
        overflow: str | None = None
        timed_out = False
        try:
            if stdout_stream is None or stderr_stream is None:
                raise RuntimeError(f"cannot capture Z3 output for {path.name}")
            selector = selectors.DefaultSelector()
            for stream, label in (
                (stdout_stream, "stdout"),
                (stderr_stream, "stderr"),
            ):
                os.set_blocking(stream.fileno(), False)
                selector.register(stream, selectors.EVENT_READ, label)
            while selector.get_map():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    timed_out = True
                    break
                for key, _ in selector.select(min(remaining, 0.5)):
                    target = buffers[key.data]
                    total = len(buffers["stdout"]) + len(buffers["stderr"])
                    allowance = MAX_OUTPUT_BYTES + 1 - total
                    if allowance <= 0:
                        overflow = "combined output"
                        break
                    try:
                        chunk = os.read(key.fd, min(65_536, allowance))
                    except BlockingIOError:
                        continue
                    if not chunk:
                        selector.unregister(key.fileobj)
                        key.fileobj.close()
                        continue
                    target.extend(chunk)
                    if (
                        len(buffers["stdout"]) + len(buffers["stderr"])
                        > MAX_OUTPUT_BYTES
                    ):
                        overflow = "combined output"
                        break
                if overflow is not None:
                    break
            if timed_out:
                raise RuntimeError(f"Z3 timed out on {path.name}")
            if overflow is not None:
                raise RuntimeError(
                    f"Z3 output exceeded the {MAX_OUTPUT_BYTES}-byte bound for {path.name}"
                )
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError(f"Z3 timed out on {path.name}")
            try:
                returncode = _wait_and_reap_owned_process_group(
                    process, group_anchor, remaining
                )
            except subprocess.TimeoutExpired as exc:
                raise RuntimeError(f"Z3 timed out on {path.name}") from exc
        finally:
            _terminate_unreaped_process(process, group_anchor)
            if selector is not None:
                selector.close()
            for stream in (stdout_stream, stderr_stream):
                if stream is not None and not stream.closed:
                    stream.close()
    stdout = bytes(buffers["stdout"])
    stderr_bytes = bytes(buffers["stderr"])
    if returncode != 0:
        stderr = stderr_bytes.decode("utf-8", errors="replace").strip()
        stdout_detail = stdout.decode("utf-8", errors="replace").strip()
        detail = (
            stderr or stdout_detail or f"exit status {returncode} without diagnostics"
        )
        raise RuntimeError(f"Z3 failed on {path.name}: {detail}")
    if stderr_bytes:
        stderr = stderr_bytes.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"Z3 wrote unexpected stderr on {path.name}: {stderr}")
    try:
        lines = tuple(
            line.strip() for line in stdout.decode("ascii").splitlines() if line.strip()
        )
    except UnicodeDecodeError as exc:
        raise RuntimeError(f"Z3 returned non-ASCII output for {path.name}") from exc
    return lines


def check_model(
    z3: str,
    path: Path,
    expected: tuple[str, ...],
    *,
    verify_digest: bool = True,
) -> None:
    lines = _run_model_results(z3, path, expected, verify_digest=verify_digest)
    if lines != expected:
        raise RuntimeError(
            f"formal obligation {path.name} returned {lines!r}; expected {expected!r}"
        )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--z3", default="z3", help="Z3 executable (default: z3)")
    args = parser.parse_args(argv)
    z3 = shutil.which(args.z3)
    if z3 is None:
        print(f"Z3 executable not found: {args.z3}", file=sys.stderr)
        return 2

    actual = {path.name for path in FORMAL_DIR.glob("*.smt2")}
    if actual != set(EXPECTED):
        missing = sorted(set(EXPECTED) - actual)
        unexpected = sorted(actual - set(EXPECTED))
        print(
            f"formal model registry mismatch: missing={missing}, unexpected={unexpected}",
            file=sys.stderr,
        )
        return 1
    try:
        _require_z3_version(z3)
        for name, expected in EXPECTED.items():
            check_model(z3, FORMAL_DIR / name, expected)
            print(f"formal OK: {name} -> {', '.join(expected)}")
    except (OSError, RuntimeError) as exc:
        print(str(exc), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
