"""Own one frozen M1 worker and observe its captured Darwin descendants.

The reviewed observer reports only identities seen during polling. It grants no
hostile-process containment or application cleanup authority. Faults and any
emergency root termination remain separate from natural descendant retirement.
"""

import argparse
import importlib.util
import os
from pathlib import Path
import selectors
import signal
import stat
import subprocess
import sys
import time


def sibling(name):
    path = Path(__file__).resolve().with_name(name + ".py")
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


c = sibling("m1_campaign")
observations = sibling("owned_observer")
MAX_LOG = 262144
MAX_CHECKPOINT = 4096
POLL_SECONDS = 0.05


def checkpoint_message(raw, binding, freeze_digest, case_id, stage):
    c.require(
        type(raw) is bytes
        and len(raw) <= MAX_CHECKPOINT
        and raw.endswith(b"\n")
        and raw.count(b"\n") == 1,
        "supervisor_checkpoint_bound",
    )
    expected = {
        "schema": "prisoma.m1-checkpoint.v1",
        "freeze_sha256": freeze_digest,
        "case_id": case_id,
        "stage": stage,
        **{
            key: binding["binding"][key]
            for key in ("run_id", "endpoint_id", "generation")
        },
    }
    c.require(
        c.canonical(c.parse(raw)) == c.canonical(expected),
        "supervisor_checkpoint_binding",
    )
    return {**expected, "schema": "prisoma.m1-continue.v1"}


def binding_for_checkpoint(output, freeze, freeze_digest, case):
    raw = c.read(output / "binding.json", c.MAX_JSON)
    value = c.parse(raw)
    c.require(
        value["schema"] == "prisoma.m1-case-binding.v1"
        and value["freeze_sha256"] == freeze_digest
        and value["case"] == case
        and value["source"] == freeze["source"]
        and value["tools"] == freeze["tools"]
        and value["runtime"] == freeze["runtime"]
        and value["workload_sha256"] == freeze["workload"]["sha256"]
        and value["environment"] == freeze["environments"][case["arm"]]
        and value["output_path"] == str(output)
        and value["output_owner"] == c.private_directory(output),
        "supervisor_case_binding",
    )
    return value, c.digest(raw)


def renderer_target(observer, tree, snapshot, executable, parent_executable):
    active = tree.update(snapshot)
    selected = []
    for identity in sorted(active - {tree.root}):
        row = snapshot[0][identity[0]]
        if row["status"] == 5:
            continue
        detail = observer.selected(identity)
        if os.fsdecode(bytes.fromhex(detail["executable_path_hex"])) == executable:
            selected.append(row)
    c.require(len(selected) == 1, "unique_owned_renderer_required")
    browser = selected[0]
    parent = snapshot[0].get(browser["ppid"])
    c.require(
        parent is not None
        and parent["status"] != 5
        and observations.birth(parent) in active,
        "active_owned_browser_parent_required",
    )
    parent_detail = observer.selected(observations.birth(parent))
    c.require(
        os.fsdecode(bytes.fromhex(parent_detail["executable_path_hex"]))
        == parent_executable,
        "browser_parent_executable",
    )
    return browser, parent


def renderer_fault(freeze, observer, tree, journal, events):
    c.selected_file(freeze["renderer"])
    c.selected_file(freeze["renderer_parent"])
    c.selected_file(freeze["tools"]["fault_binary"])
    row, parent = renderer_target(
        observer,
        tree,
        observer.snapshot(),
        freeze["renderer"]["path"],
        freeze["renderer_parent"]["path"],
    )
    intent = {
        "kind": "renderer_fault_intent",
        "identity": row,
        "executable": freeze["renderer"],
        "parent_identity": parent,
        "parent_executable": freeze["renderer_parent"],
        "signal": signal.SIGTERM,
    }
    journal.json("fault-intent.json", intent)
    events.append(intent)
    result = subprocess.run(
        [
            freeze["tools"]["fault_binary"]["path"],
            "terminate",
            str(row["pid"]),
            str(row["start_seconds"]),
            str(row["start_microseconds"]),
            str(row["ppid"]),
            freeze["renderer"]["path"],
        ],
        capture_output=True,
        timeout=5,
        check=False,
    )
    c.require(
        len(result.stdout) <= MAX_CHECKPOINT and len(result.stderr) <= MAX_CHECKPOINT,
        "fault_output_bound",
    )
    journal.write("fault.stdout", result.stdout)
    journal.write("fault.stderr", result.stderr)
    events.append({"kind": "renderer_fault_return", "returncode": result.returncode})
    c.require(result.returncode == 0, "renderer_fault_failed")
    receipt = c.closed(
        c.parse(result.stdout),
        {
            "schema",
            "pid",
            "start_seconds",
            "start_microseconds",
            "ppid",
            "pid_version",
            "signal",
            "signal_result",
        },
        "fault_shape",
    )
    c.require(
        receipt["schema"] == "local.darwin-version-bound-fault.v1"
        and all(
            receipt[key] == row[key]
            for key in ("pid", "start_seconds", "start_microseconds", "ppid")
        )
        and c.integer(receipt["pid_version"], 0, 2**32 - 1)
        and type(receipt["signal_result"]) is int
        and receipt["signal_result"] == 0
        and type(receipt["signal"]) is int
        and receipt["signal"] == signal.SIGTERM,
        "fault_binding",
    )
    events.append({"kind": "renderer_fault_observed", "receipt": receipt})


def publish_json(directory, name, value):
    c.private_directory(directory)
    c.require(Path(name).name == name and name not in {"", ".", ".."}, "artifact_path")
    raw = c.canonical(value) + b"\n"
    c.require(len(raw) <= c.MAX_JSON, "json_bound")
    path = directory / name
    pending = directory / ("." + name + ".pending")
    descriptor = os.open(
        pending, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600
    )
    with os.fdopen(descriptor, "wb") as stream:
        c.require(stat.S_ISREG(os.fstat(stream.fileno()).st_mode), "artifact_kind")
        stream.write(raw)
        stream.flush()
        os.fsync(stream.fileno())
    c.require(c.read(pending, c.MAX_JSON) == raw, "receipt_reopen")
    # No-replace commit. Keep the staging link so no cleanup can fail after it.
    # This does not promise directory or power-loss durability.
    os.link(pending, path, follow_symlinks=False)
    return {"path": name, "bytes": len(raw), "sha256": c.digest(raw)}


def publish_owner(
    output,
    freeze,
    freeze_digest,
    case,
    binding_digest,
    child,
    tree,
    observer_receipt,
    emergency,
    events,
):
    c.private_directory(output)
    selected_result = output / "result.json"
    result_digest = (
        c.digest(c.read(selected_result, c.MAX_JSON))
        if selected_result.exists()
        else None
    )
    evidence = publish_json(
        output,
        "observer-events.json",
        {
            "schema": "prisoma.m1-owner-observation.v1",
            "freeze_sha256": freeze_digest,
            "case_id": case["case_id"],
            "observation": observer_receipt,
            "supervisor_events": events,
        },
    )
    owner = {
        "schema": "prisoma.m1-owner.v1",
        "freeze_sha256": freeze_digest,
        "case_id": case["case_id"],
        "binding_sha256": binding_digest,
        "result_sha256": result_digest,
        "worker_returncode": child.returncode,
        "observed_births": [list(row) for row in sorted(tree.known)],
        "retired_births": [list(row) for row in sorted(tree.absent)],
        "emergency_cleanup": emergency,
        "observer_events": evidence,
        "observer_sha256": freeze["tools"]["observer_binary"]["sha256"],
    }
    publish_json(output, "owner.json", owner)
    return owner


def joined_failures(failures):
    if not failures:
        return None
    if len(failures) == 1:
        return failures[0]
    return BaseExceptionGroup(
        "Supervisor operation and retirement failures", list(failures)
    )


def supervise(freeze_path, case_id):
    c.require(
        sys.flags.isolated and sys.dont_write_bytecode, "isolated_python_required"
    )
    freeze, freeze_digest = c.load_freeze(freeze_path)
    c.require(
        freeze["tools"]["supervisor"] == c.file_identity(Path(__file__).resolve())
        and freeze["tools"]["observer_python"]
        == c.file_identity(Path(observations.__file__).resolve()),
        "supervisor_identity",
    )
    case = c.selected_case(freeze, case_id)
    output = Path(freeze["output_root"]) / case_id
    c.require(not output.exists() and not output.is_symlink(), "fresh_case_required")
    journal = c.Output(Path(freeze["output_root"]) / (case_id + ".supervisor"))
    command = [
        str(Path(freeze["environments"][case["arm"]]["prefix"]) / "bin/python"),
        "-I",
        "-B",
        freeze["tools"]["worker"]["path"],
        "worker",
        "--freeze",
        str(freeze_path),
        "--case",
        case_id,
        "--output",
        str(output),
    ]
    progress_read = progress_write = control_read = control_write = None
    environment = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith(("PYTHON", "LD_", "DYLD_", "GIT_"))
    }
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    child = None
    tree = None
    primary = None
    emergency = False
    events = []
    binding_digest = None
    observer_receipt = None
    stdout, stderr, pending = bytearray(), bytearray(), bytearray()
    stage_index = 0
    started = time.monotonic()
    deadline = started + freeze["limits"]["session_seconds"]
    observer = observations.Observer(freeze["tools"]["observer_binary"]["path"])
    try:
        progress_read, progress_write = os.pipe()
        control_read, control_write = os.pipe()
        command += [
            "--progress-fd",
            str(progress_write),
            "--control-fd",
            str(control_read),
        ]
        journal.json(
            "launch.json",
            {
                "freeze_sha256": freeze_digest,
                "case": case,
                "command": command,
                "deadline_seconds": freeze["limits"]["session_seconds"],
            },
        )
        child = subprocess.Popen(
            command,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            pass_fds=(progress_write, control_read),
        )
        os.close(progress_write)
        progress_write = None
        os.close(control_read)
        control_read = None
        tree = observations.OwnedTree(observer, child, observer.snapshot())
        journal.json(
            "worker-owned.json", {"birth": list(tree.root), "parent_pid": os.getpid()}
        )
        with selectors.DefaultSelector() as selector:
            for fd, role in (
                (progress_read, "checkpoint"),
                (child.stdout.fileno(), "stdout"),
                (child.stderr.fileno(), "stderr"),
            ):
                os.set_blocking(fd, False)
                selector.register(fd, selectors.EVENT_READ, role)
            os.set_blocking(control_write, False)
            while True:
                c.require(time.monotonic() < deadline, "supervisor_deadline")
                snapshot = observer.snapshot()
                active = tree.update(snapshot)
                child.poll()
                for key, _ in selector.select(POLL_SECONDS):
                    raw = os.read(key.fd, 65536)
                    if not raw:
                        selector.unregister(key.fd)
                        if key.data == "checkpoint":
                            c.require(not pending, "partial_checkpoint")
                        continue
                    if key.data in {"stdout", "stderr"}:
                        selected = stdout if key.data == "stdout" else stderr
                        c.require(
                            len(selected) + len(raw) <= MAX_LOG, "worker_log_bound"
                        )
                        selected.extend(raw)
                        continue
                    pending.extend(raw)
                    c.require(
                        len(pending) <= MAX_CHECKPOINT, "supervisor_checkpoint_bound"
                    )
                    if b"\n" not in pending:
                        continue
                    c.require(stage_index < 2, "extra_checkpoint")
                    stage = ("prepared", "tick1")[stage_index]
                    binding, observed_digest = binding_for_checkpoint(
                        output, freeze, freeze_digest, case
                    )
                    c.require(
                        binding_digest is None or binding_digest == observed_digest,
                        "binding_changed",
                    )
                    binding_digest = observed_digest
                    reply = checkpoint_message(
                        bytes(pending), binding, freeze_digest, case_id, stage
                    )
                    pending.clear()
                    stage_index += 1
                    events.append(
                        {
                            "kind": "checkpoint",
                            "stage": stage,
                            "binding_sha256": binding_digest,
                        }
                    )
                    if case["fault"] == "caller_loss" and stage == "prepared":
                        c.require(
                            child.returncode is None
                            and tree.root in tree.update(observer.snapshot()),
                            "caller_already_retired",
                        )
                        journal.json(
                            "fault-intent.json",
                            {
                                "kind": "caller_loss",
                                "birth": list(tree.root),
                                "signal": signal.SIGKILL,
                            },
                        )
                        # Only this Popen owns waitpid. Its unreaped child cannot be PID-reused.
                        child.kill()
                        events.append(
                            {
                                "kind": "caller_fault_sent",
                                "birth": list(tree.root),
                                "signal": signal.SIGKILL,
                            }
                        )
                        os.close(control_write)
                        control_write = None
                    else:
                        if case["fault"] == "renderer_loss" and stage == "tick1":
                            renderer_fault(freeze, observer, tree, journal, events)
                        wire = c.canonical(reply) + b"\n"
                        c.require(
                            len(wire) <= MAX_CHECKPOINT
                            and os.write(control_write, wire) == len(wire),
                            "checkpoint_write",
                        )
                if (
                    child.returncode is not None
                    and not active
                    and not selector.get_map()
                ):
                    break
        observer_receipt = tree.receipt(observer.snapshot())
        c.require(
            observer_receipt["observed_identities_retired"], "observed_processes_remain"
        )
        c.require(
            stage_index == (1 if case["fault"] == "caller_loss" else 2),
            "checkpoint_count",
        )
        c.require(c.load_freeze(freeze_path)[1] == freeze_digest, "freeze_changed")
        result_exists = (output / "result.json").exists()
        c.require(
            result_exists == (case["fault"] != "caller_loss"), "worker_result_presence"
        )
        if result_exists:
            c.parse(c.read(output / "result.json", c.MAX_JSON))
        if case["fault"] == "none":
            c.require(child.returncode == 0, "worker_failed")
        elif case["fault"] == "caller_loss":
            c.require(child.returncode == -signal.SIGKILL, "caller_fault_return")
        else:
            c.require(child.returncode != 0, "renderer_fault_not_observed")
    except BaseException as error:
        primary = error
    finally:
        failures = [] if primary is None else [primary]

        def attempt(action):
            try:
                return action()
            except BaseException as failure:
                failures.append(failure)
                return None

        for fd in (progress_read, progress_write, control_read, control_write):
            if fd is not None:
                attempt(lambda fd=fd: os.close(fd))
        if child is not None:
            attempt(child.poll)
            if child.returncode is None:
                emergency = True
                events.append(
                    {
                        "kind": "emergency_root_termination",
                        "pid": child.pid,
                        "signal": signal.SIGKILL,
                    }
                )
                attempt(child.kill)
                attempt(lambda: child.wait(timeout=5))
            attempt(child.stdout.close)
            attempt(child.stderr.close)
        if failures and tree is not None:
            # Observe the owner's response to pipe closure. Never signal a
            # descendant from a stale numeric PID, even during failure cleanup.
            retirement_deadline = (
                time.monotonic() + freeze["limits"]["checkpoint_seconds"]
            )
            try:
                while True:
                    observer_receipt = tree.receipt(observer.snapshot())
                    if (
                        observer_receipt["observed_identities_retired"]
                        or time.monotonic() >= retirement_deadline
                    ):
                        break
                    time.sleep(POLL_SECONDS)
            except BaseException as observation_error:
                failures.append(observation_error)
                events.append(
                    {
                        "kind": "retirement_observation_failed",
                        "failure": c.diagnostic(observation_error),
                    }
                )
        attempt(lambda: journal.write("worker.stdout", bytes(stdout)))
        attempt(lambda: journal.write("worker.stderr", bytes(stderr)))
        primary = joined_failures(failures)
        attempt(
            lambda: journal.json(
                "supervisor-result.json",
                {
                    "schema": "prisoma.m1-supervisor-result.v1",
                    "freeze_sha256": freeze_digest,
                    "case_id": case_id,
                    "ready_for_owner_publication": primary is None,
                    "elapsed_seconds": time.monotonic() - started,
                    "emergency_cleanup": emergency,
                    "failure": None if primary is None else c.diagnostic(primary),
                    "observed_tree": observer_receipt,
                    "events": events,
                },
            )
        )
        primary = joined_failures(failures)
    if primary is not None:
        raise primary
    # Final fallible action: commit the owner receipt only after every required
    # observation and journal write succeeded. The caller also checks CLI exit.
    publish_owner(
        output,
        freeze,
        freeze_digest,
        case,
        binding_digest,
        child,
        tree,
        observer_receipt,
        emergency,
        events,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--freeze", type=Path, required=True)
    parser.add_argument("--case", required=True)
    args = parser.parse_args()
    supervise(args.freeze, args.case)


if __name__ == "__main__":
    main()
