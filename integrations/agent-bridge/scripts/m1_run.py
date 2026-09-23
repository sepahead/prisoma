#!/usr/bin/env python3
"""Observe one frozen supervisor command through exit, without simulator imports."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import importlib.util
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time


def _campaign():
    path = Path(__file__).resolve().with_name("m1_campaign.py")
    spec = importlib.util.spec_from_file_location("m1_runner_campaign", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


c = _campaign()
REAP_SECONDS = 5
POLL_SECONDS = 0.05


def _identity(path, maximum):
    raw = c.read(path, maximum)
    return {"path": str(path), "bytes": len(raw), "sha256": c.digest(raw)}


def command_limits(freeze):
    """Derive separate command, cleanup, and retained-stream bounds."""
    session = freeze["limits"]["session_seconds"]
    checkpoint = freeze["limits"]["checkpoint_seconds"]
    return {
        "wall_seconds": session + 2 * checkpoint + 30,
        "grace_seconds": checkpoint,
        "stream_bytes": 65536 + 4096 * (session + checkpoint),
    }


def command_argv(freeze, freeze_path, case):
    prefix = Path(freeze["environments"][case["arm"]]["prefix"])
    c.require(
        prefix.is_absolute() and prefix.resolve(strict=True) == prefix,
        "python_prefix",
    )
    selected = c.parse(
        c.selected_file(
            freeze["environments"][case["arm"]]["inventory"], maximum=c.MAX_JSON
        )
    )
    # Preserve the venv launcher path. Its resolved binary must match inventory.
    launcher = prefix / "bin" / "python"
    c.require(
        c.file_identity(launcher.resolve(strict=True)) == selected["python"],
        "python_identity",
    )
    return [
        str(launcher),
        "-I",
        "-B",
        freeze["tools"]["supervisor"]["path"],
        "--freeze",
        str(freeze_path),
        "--case",
        case["case_id"],
    ]


@contextmanager
def _interrupts(state):
    def record(_number, _frame):
        state["interrupted"] = True

    previous = signal.signal(signal.SIGINT, record)
    try:
        yield
    finally:
        signal.signal(signal.SIGINT, previous)


def _signal_owned(child, number):
    # Popen retains ownership until wait/poll reaps this direct child. Its own
    # send_signal polls again, preventing a signal after observed retirement.
    if child.poll() is None:
        child.send_signal(number)
        return True
    return False


def _retire_owned(child, grace):
    """Exceptional I/O cleanup concerns only the unreaped direct child."""
    _signal_owned(child, signal.SIGINT)
    try:
        return child.wait(timeout=grace), False
    except subprocess.TimeoutExpired:
        killed = _signal_owned(child, signal.SIGKILL)
        return child.wait(timeout=REAP_SECONDS), killed


def capture(argv, environment, limits, stdout, stderr):
    """Bound the owned command and capture prefixes; never signal descendants."""
    state = {
        "returncode": None,
        "timed_out": False,
        "output_overflow": False,
        "interrupted": False,
        "forced_kill": False,
        "failure": None,
    }
    counts = {"stdout": 0, "stderr": 0}
    stop_at = kill_at = None
    failures = []
    handled_io = False
    with _interrupts(state):
        child = subprocess.Popen(
            argv,
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            close_fds=True,
        )
        deadline = time.monotonic() + limits["wall_seconds"]

        def stop(code):
            nonlocal stop_at
            if state["failure"] is None:
                state["failure"] = code
            if stop_at is None:
                _signal_owned(child, signal.SIGINT)
                stop_at = time.monotonic() + limits["grace_seconds"]

        try:
            with selectors.DefaultSelector() as selector:
                for name, pipe, stream in (
                    ("stdout", child.stdout, stdout),
                    ("stderr", child.stderr, stderr),
                ):
                    os.set_blocking(pipe.fileno(), False)
                    selector.register(pipe, selectors.EVENT_READ, (name, stream))
                while True:
                    now = time.monotonic()
                    returncode = child.poll()
                    if state["interrupted"]:
                        stop("runner_interrupted")
                    if now >= deadline and stop_at is None:
                        state["timed_out"] = True
                        stop("supervisor_timeout")
                    if returncode is not None and not selector.get_map():
                        break
                    if stop_at is not None and now >= stop_at:
                        if returncode is not None:
                            # A descendant can retain a pipe after root exit.
                            # Close it without assigning descendant retirement.
                            break
                        if kill_at is None:
                            state["forced_kill"] = _signal_owned(child, signal.SIGKILL)
                            kill_at = time.monotonic() + REAP_SECONDS
                    if kill_at is not None and now >= kill_at:
                        child.wait(timeout=0)
                        break
                    for key, _ in selector.select(POLL_SECONDS):
                        name, stream = key.data
                        part = os.read(key.fd, 65536)
                        if not part:
                            selector.unregister(key.fd)
                            continue
                        retained = part[: max(0, limits["stream_bytes"] - counts[name])]
                        if retained:
                            stream.write(retained)
                            counts[name] += len(retained)
                        if len(retained) != len(part):
                            state["output_overflow"] = True
                            stop("supervisor_output_bound")
                state["returncode"] = child.wait(timeout=0)
        except BaseException as error:
            failures.append(error)
            handled_io = issubclass(type(error), (OSError, ValueError))
            state["failure"] = state["failure"] or "supervisor_io"
            try:
                state["returncode"], killed = _retire_owned(
                    child, limits["grace_seconds"]
                )
                state["forced_kill"] |= killed
            except BaseException as cleanup_error:
                failures.append(cleanup_error)
        finally:
            # One operation, one exceptional retirement, and two pipe closures
            # bound the failure roster. Never replace an earlier failure or
            # retry failed retirement from a second cleanup path.
            for pipe in (child.stdout, child.stderr):
                try:
                    pipe.close()
                except BaseException as cleanup_error:
                    failures.append(cleanup_error)
    if failures and not (handled_io and len(failures) == 1):
        if len(failures) == 1:
            raise failures[0]
        raise BaseExceptionGroup("Runner operation and retirement failures", failures)
    c.require(type(state["returncode"]) is int, "supervisor_not_reaped")
    if state["returncode"] != 0:
        state["failure"] = state["failure"] or "supervisor_returncode"
    return state


def publish(directory, name, value):
    """The final hard link commits a fresh receipt without replacement."""
    c.private_directory(directory)
    c.require(Path(name).name == name and name not in {"", ".", ".."}, "receipt_name")
    raw = c.canonical(value) + b"\n"
    c.require(len(raw) <= c.MAX_JSON, "receipt_bound")
    pending = directory / ("." + name + ".pending")
    fd = os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "wb") as stream:
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    c.require(c.read(pending, c.MAX_JSON) == raw, "receipt_reopen")
    os.link(pending, directory / name, follow_symlinks=False)


def run(freeze_path, case_id):
    c.require(sys.flags.isolated and sys.dont_write_bytecode, "isolated_python")
    freeze_path = Path(freeze_path)
    freeze, freeze_digest = c.load_freeze(freeze_path)
    freeze_identity = _identity(freeze_path, c.MAX_JSON)
    c.require(freeze_identity["sha256"] == freeze_digest, "freeze_changed")
    c.require(
        c.file_identity(Path(__file__).resolve()) == freeze["tools"]["runner"],
        "runner_identity",
    )
    case = c.selected_case(freeze, case_id)
    argv, limits = command_argv(freeze, freeze_path, case), command_limits(freeze)
    root = Path(freeze["output_root"])
    root_identity = c.private_directory(root)
    name = case_id + ".command.json"
    for target in (root / name, root / ("." + name + ".pending")):
        c.require(not target.exists() and not target.is_symlink(), "receipt_exists")
    journal = c.Output(root / (case_id + ".command"))
    environment = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith(("PYTHON", "LD_", "DYLD_", "GIT_"))
        and key != "__PYVENV_LAUNCHER__"
    }
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    with journal.open("stdout.bin") as stdout, journal.open("stderr.bin") as stderr:
        observed = capture(argv, environment, limits, stdout, stderr)
    try:
        rechecked, rechecked_digest = c.load_freeze(freeze_path)
        c.require(
            rechecked == freeze
            and rechecked_digest == freeze_digest
            and _identity(freeze_path, c.MAX_JSON) == freeze_identity
            and command_argv(freeze, freeze_path, case) == argv,
            "selection_changed",
        )
    except (c.CampaignError, OSError, KeyError, ValueError):
        observed["failure"] = observed["failure"] or "selection_changed"
    owner = None
    owner_path = root / case_id / "owner.json"
    try:
        owner = _identity(owner_path, c.MAX_JSON)
    except FileNotFoundError:
        observed["failure"] = observed["failure"] or "owner_missing"
    except (c.CampaignError, OSError, ValueError):
        observed["failure"] = observed["failure"] or "owner_read"
    c.require(c.private_directory(root) == root_identity, "output_changed")
    c.require(c.private_directory(journal.path) == journal.identity, "output_changed")
    receipt = {
        "schema": "prisoma.m1-command.v1",
        "freeze": freeze_identity,
        "case": case,
        "runner": freeze["tools"]["runner"],
        "supervisor": freeze["tools"]["supervisor"],
        "argv": argv,
        "limits": limits,
        **observed,
        "stdout": _identity(journal.path / "stdout.bin", limits["stream_bytes"]),
        "stderr": _identity(journal.path / "stderr.bin", limits["stream_bytes"]),
        "owner": owner,
    }
    success = (
        observed["returncode"] == 0
        and not any(
            observed[key]
            for key in ("timed_out", "output_overflow", "interrupted", "forced_kill")
        )
        and observed["failure"] is None
        and owner is not None
    )
    publish(root, name, receipt)
    return 0 if success else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--freeze", type=Path, required=True)
    parser.add_argument("--case", required=True)
    args = parser.parse_args()
    return run(args.freeze, args.case)


if __name__ == "__main__":
    raise SystemExit(main())
