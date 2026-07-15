#!/usr/bin/env python3
"""Run SMT obligations with bounded input/output and wall-clock execution."""

from __future__ import annotations

import argparse
import math
import os
import selectors
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FORMAL_DIR = ROOT / "formal"
MAX_MODEL_BYTES = 256 * 1024
MAX_OUTPUT_BYTES = 64 * 1024
TIMEOUT_SECONDS = 15.0
WALLCLOCK_GRACE_SECONDS = 2.0

ALLOWED_TOP_LEVEL_COMMANDS = frozenset(
    {
        "set-logic",
        "declare-const",
        "define-fun",
        "assert",
        "push",
        "pop",
        "check-sat",
    }
)

EXPECTED: dict[str, tuple[str, ...]] = {
    "bridge_log_before_dispatch.smt2": ("unsat", "unsat", "sat", "sat"),
    "receipt_last_publication.smt2": (
        "unsat",
        "unsat",
        "sat",
        "sat",
        "unsat",
        "unsat",
        "sat",
        "sat",
    ),
    "typed_outcome_publication.smt2": (
        "sat",
        "sat",
        "sat",
        "sat",
        "unsat",
        "unsat",
        "unsat",
        "unsat",
        "unsat",
        "unsat",
        "unsat",
        "unsat",
    ),
    "shannon_two_source_identity.smt2": ("sat", "unsat"),
    "pid_nonidentification.smt2": ("sat",),
    "coupling_nonidentification.smt2": ("sat",),
}


def _same_file_snapshot(left: os.stat_result, right: os.stat_result) -> bool:
    return (
        stat.S_IFMT(left.st_mode) == stat.S_IFMT(right.st_mode)
        and left.st_dev == right.st_dev
        and left.st_ino == right.st_ino
        and left.st_size == right.st_size
        and left.st_mtime_ns == right.st_mtime_ns
        and left.st_ctime_ns == right.st_ctime_ns
    )


def _read_regular_model(path: Path) -> bytes:
    """Snapshot one bounded regular model without following a final symlink."""

    try:
        path_before = path.lstat()
    except OSError as exc:
        raise RuntimeError(f"cannot inspect formal model {path}: {exc}") from exc
    if not stat.S_ISREG(path_before.st_mode):
        raise RuntimeError(f"formal model must be a regular non-symlink file: {path}")
    if path_before.st_size > MAX_MODEL_BYTES:
        raise RuntimeError(f"formal model exceeds {MAX_MODEL_BYTES} bytes: {path}")

    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0)
    try:
        descriptor = os.open(path, flags)
    except OSError as exc:
        raise RuntimeError(f"cannot open formal model {path}: {exc}") from exc
    try:
        opened_before = os.fstat(descriptor)
        if not stat.S_ISREG(opened_before.st_mode) or not _same_file_snapshot(
            path_before, opened_before
        ):
            raise RuntimeError(f"formal model changed while opening: {path}")
        chunks: list[bytes] = []
        remaining = MAX_MODEL_BYTES + 1
        while remaining > 0:
            chunk = os.read(descriptor, min(65_536, remaining))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        model = b"".join(chunks)
        if len(model) > MAX_MODEL_BYTES:
            raise RuntimeError(f"formal model exceeds {MAX_MODEL_BYTES} bytes: {path}")
        opened_after = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    try:
        path_after = path.lstat()
    except OSError as exc:
        raise RuntimeError(f"formal model changed while reading {path}: {exc}") from exc
    if not _same_file_snapshot(opened_before, opened_after) or not _same_file_snapshot(
        opened_after, path_after
    ):
        raise RuntimeError(f"formal model changed while reading: {path}")
    return model


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
                if source[index] == "\\" and index + 1 < len(source):
                    index += 2
                    continue
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


def _top_level_commands(model: bytes) -> tuple[str, ...]:
    try:
        source = model.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise RuntimeError("formal model is not valid UTF-8") from exc
    if "\x00" in source:
        raise RuntimeError("formal model contains a NUL byte")

    commands: list[str] = []
    depth = 0
    awaiting_command = False
    for token in _smt_tokens(source):
        if token == "(":
            if depth == 0:
                awaiting_command = True
            depth += 1
        elif token == ")":
            if depth == 0:
                raise RuntimeError("formal model has an unmatched closing parenthesis")
            if depth == 1 and awaiting_command:
                raise RuntimeError("formal model contains an empty top-level command")
            depth -= 1
        elif depth == 0:
            raise RuntimeError(
                "formal model contains an atom outside a top-level command"
            )
        elif awaiting_command:
            commands.append(token)
            awaiting_command = False
    if depth != 0:
        raise RuntimeError("formal model has unbalanced parentheses")
    return tuple(commands)


def _validate_model_source(model: bytes, expected: tuple[str, ...], name: str) -> None:
    if not expected or any(result not in {"sat", "unsat"} for result in expected):
        raise RuntimeError(f"invalid expected-result registry entry for {name}")
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


def check_model(z3: str, path: Path, expected: tuple[str, ...]) -> None:
    model = _read_regular_model(path)
    _validate_model_source(model, expected, path.name)
    solver_timeout = max(1, math.ceil(TIMEOUT_SECONDS))
    with tempfile.TemporaryDirectory(prefix="prisoma-formal-") as temp_dir:
        snapshot_path = Path(temp_dir) / path.name
        snapshot_path.write_bytes(model)
        try:
            process = subprocess.Popen(
                [z3, f"-T:{solver_timeout}", os.fspath(snapshot_path)],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
        except OSError as exc:
            raise RuntimeError(f"cannot start Z3 for {path.name}: {exc}") from exc
        if process.stdout is None or process.stderr is None:
            process.kill()
            raise RuntimeError(f"cannot capture Z3 output for {path.name}")
        selector = selectors.DefaultSelector()
        buffers = {"stdout": bytearray(), "stderr": bytearray()}
        deadline = time.monotonic() + TIMEOUT_SECONDS + WALLCLOCK_GRACE_SECONDS
        overflow: str | None = None
        timed_out = False
        try:
            for stream, label in (
                (process.stdout, "stdout"),
                (process.stderr, "stderr"),
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
            if timed_out or overflow is not None:
                process.kill()
            remaining = max(0.0, deadline - time.monotonic())
            try:
                returncode = process.wait(timeout=max(1.0, remaining))
            except subprocess.TimeoutExpired:
                timed_out = True
                process.kill()
                try:
                    returncode = process.wait(timeout=2.0)
                except subprocess.TimeoutExpired as exc:
                    raise RuntimeError(
                        f"could not terminate Z3 for {path.name}"
                    ) from exc
        finally:
            selector.close()
            for stream in (process.stdout, process.stderr):
                stream.close()
    if timed_out:
        raise RuntimeError(f"Z3 timed out on {path.name}")
    if overflow is not None:
        raise RuntimeError(
            f"Z3 output exceeded the {MAX_OUTPUT_BYTES}-byte bound for {path.name}"
        )
    stdout = bytes(buffers["stdout"])
    stderr_bytes = bytes(buffers["stderr"])
    if returncode != 0:
        stderr = stderr_bytes.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"Z3 failed on {path.name}: {stderr}")
    if stderr_bytes:
        stderr = stderr_bytes.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"Z3 wrote unexpected stderr on {path.name}: {stderr}")
    try:
        lines = tuple(
            line.strip() for line in stdout.decode("ascii").splitlines() if line.strip()
        )
    except UnicodeDecodeError as exc:
        raise RuntimeError(f"Z3 returned non-ASCII output for {path.name}") from exc
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
        for name, expected in EXPECTED.items():
            check_model(z3, FORMAL_DIR / name, expected)
            print(f"formal OK: {name} -> {', '.join(expected)}")
    except (OSError, RuntimeError) as exc:
        print(str(exc), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
