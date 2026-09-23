"""Freeze and observe a bounded matched workload; no release or timing guarantee."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import stat
import subprocess
import sys
import time
import uuid


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    sys.modules[name] = result
    spec.loader.exec_module(result)
    return result


HERE = Path(__file__).resolve().parent
m = module("perf_campaign_common", HERE / "perf_common.py")
c = module("perf_campaign_m1", HERE / "m1_campaign.py")
o = module("perf_campaign_observer", HERE / "owned_observer.py")


def planned_cases():
    arms = [
        (route, level)
        for route in ("direct", "body", "canonical")
        for level in ("minimal", "detailed")
    ]
    rows = []
    for block in range(32):
        offset = block % len(arms)
        ordered = arms[offset:] + arms[:offset]
        if (block // len(arms)) % 2:
            ordered.reverse()
        for route, level in ordered:
            rows.append(
                {
                    "case_id": f"b{block:02d}-{route}-{level}",
                    "block": block,
                    "route": route,
                    "instrumentation": level,
                }
            )
    return rows


def tree_size(path):
    total = count = 0
    for base, directories, names in os.walk(path, followlinks=False):
        for name in directories + names:
            value = Path(base) / name
            info = value.lstat()
            m.require(not stat.S_ISLNK(info.st_mode), "unselected artifact symlink")
            m.require(
                stat.S_ISREG(info.st_mode) or stat.S_ISDIR(info.st_mode),
                "nonregular artifact",
            )
            if stat.S_ISREG(info.st_mode):
                total += info.st_size
            count += 1
            m.require(count <= 100000, "artifact roster extent")
    return total


def validate_freeze(freeze, raw):
    from crebain_ncp_sensors import new_binding

    m.require(freeze["schema"] == "local.m1-performance-freeze.v1", "freeze schema")
    projection = [
        {key: row[key] for key in ("case_id", "block", "route", "instrumentation")}
        for row in freeze["cases"]
    ]
    m.require(projection == planned_cases(), "complete prospective case order")
    for key in ("run_id", "clock_id"):
        values = [row[key] for row in freeze["cases"]]
        m.require(
            all(type(value) is str for value in values) and len(set(values)) == 192,
            "fresh case identity roster",
        )
        if key == "run_id":
            # The installed owner admits the exact string before any case effects.
            for value in values:
                new_binding(run_id=value)
        else:
            m.require(
                all(
                    len(value) == 32 and all(x in "0123456789abcdef" for x in value)
                    for value in values
                ),
                "fresh case identity roster",
            )
    m.require(
        freeze["limits"]
        == {
            "session_seconds": 180,
            "cleanup_seconds": 205,
            "campaign_seconds": 2700,
            "case_artifact_bytes": 64 * 1024**2,
            "campaign_artifact_bytes": 6 * 1024**3,
            "stream_bytes": 8 * 1024**2,
            "minimum_free_disk_bytes": 24 * 1024**3,
        },
        "selected bounds",
    )
    for row in freeze["tools"].values():
        m.selected_file(row)
    m.require(
        m.identity(Path(__file__).resolve()) == freeze["tools"]["campaign"],
        "campaign tool identity",
    )
    m.selected_file(freeze["workload"])
    for row in freeze["executables"].values():
        c.selected_file(row)
    for environment in freeze["environments"].values():
        m.selected_file(environment["inventory"])
        for row in environment["wheels"].values():
            c.selected_file(row)
    m.require(len(raw) <= m.MAX_JSON, "freeze extent")


def selected_input(path):
    raw = c.read(path, maximum=m.MAX_JSON)
    m.require(bool(raw), "nonempty selected input")
    return {"path": str(path), "bytes": len(raw), "sha256": c.digest(raw)}, raw


def freeze_study(m1_freeze, design, review, campaign):
    from crebain_ncp_sensors.runtime import InstalledRuntime

    m.require(not campaign.exists(), "fresh campaign directory required")
    prior_identity, prior_raw = selected_input(m1_freeze)
    design_identity, _ = selected_input(design)
    review_identity, _ = selected_input(review)
    old = m.parse_json(prior_raw)
    m.require(
        type(old) is dict and old.get("schema") == "prisoma.m1-freeze.v1",
        "original M1 freeze schema",
    )
    runtime = InstalledRuntime.open(old["runtime"]["prefix"])
    m.require(
        runtime.manifest_sha256 == old["runtime"]["manifest_sha256"]
        and runtime.source_identity == old["runtime"]["source_identity"],
        "unchanged qualified runtime",
    )
    m.require(runtime.node is not None, "selected graphics runtime Node executable")
    executables = {
        name: c.file_identity(getattr(runtime, name)) for name in ("bun", "node")
    }
    tools = {
        name: m.identity(HERE / filename)
        for name, filename in {
            "campaign": "perf_campaign.py",
            "common": "perf_common.py",
            "python_worker": "perf_worker.py",
            "direct_worker": "perf_direct.ts",
            "report": "perf_report.py",
            "m1_helpers": "m1_campaign.py",
            "observer_python": "owned_observer.py",
        }.items()
    }
    tools["observer_binary"] = old["tools"]["observer_binary"]
    tools["observer_source"] = old["tools"]["observer_source"]
    tools["design"] = design_identity
    tools["design_closure"] = review_identity
    cases = [
        {**row, "run_id": str(uuid.uuid4()), "clock_id": uuid.uuid4().hex}
        for row in planned_cases()
    ]
    value = {
        "schema": "local.m1-performance-freeze.v1",
        "campaign_id": campaign.name,
        "original_m1_freeze": prior_identity,
        "workload": old["workload"],
        "tools": tools,
        "cases": cases,
        "output_root": str(campaign / "runs"),
        "runtime": old["runtime"],
        "environments": old["environments"],
        "executables": executables,
        "node": executables["node"]["path"],
        "process_environment": {**runtime.environment, "PYTHONDONTWRITEBYTECODE": "1"},
        "limits": {
            "session_seconds": 180,
            "cleanup_seconds": 205,
            "campaign_seconds": 2700,
            "case_artifact_bytes": 64 * 1024**2,
            "campaign_artifact_bytes": 6 * 1024**3,
            "stream_bytes": 8 * 1024**2,
            "minimum_free_disk_bytes": 24 * 1024**3,
        },
        "runtime_verification": "InstalledRuntime.open before and after campaign; individual Python owners also reopen selected runtime",
        "authority": {
            "dedicated_quiescent_machine": False,
            "loaded_bytes_attested": False,
            "allocator_or_copy_counts": False,
            "release_qualified": False,
            "real_time_qualified": False,
        },
        "decision_rule": "All planned outcomes retained. Accounting requires exact payload bytes, deadlines reported separately. Failure or uncertain cleanup halts future cases.",
    }
    encoded = (json.dumps(value, indent=2, allow_nan=False) + "\n").encode()
    validate_freeze(value, encoded)
    campaign.mkdir(mode=0o700)
    (campaign / "runs").mkdir(mode=0o700)
    (campaign / "commands").mkdir(mode=0o700)
    with (campaign / "freeze.json").open("xb") as stream:
        stream.write(encoded)
        stream.flush()
        os.fsync(stream.fileno())
    return m.identity(campaign / "freeze.json")


def command_for(freeze, path, case):
    if case["route"] == "direct":
        return [
            freeze["executables"]["bun"]["path"],
            freeze["tools"]["direct_worker"]["path"],
            str(path),
            case["case_id"],
        ]
    return [
        str(Path(freeze["environments"][case["route"]]["prefix"]) / "bin/python"),
        "-I",
        "-B",
        freeze["tools"]["python_worker"]["path"],
        "--freeze",
        str(path),
        "--case",
        case["case_id"],
    ]


def capture(argv, environment, directory, observer_path, limits, campaign_deadline):
    """Own one direct child; observe descendant births without signaling them."""
    directory.mkdir(mode=0o700)
    observer = o.Observer(observer_path)
    streams = [open(directory / name, "xb") for name in ("stdout.bin", "stderr.bin")]
    child = tracker = None
    failure = None
    cleanup_errors = []
    stop_at = None
    forced = interrupted = timed_out = overflow = False
    counts = [0, 0]
    started = time.monotonic()
    receipt = None
    interrupt = [False]
    previous_handler = signal.signal(
        signal.SIGINT, lambda *_: interrupt.__setitem__(0, True)
    )
    selector = selectors.DefaultSelector()

    def request_stop(reason):
        nonlocal failure, stop_at
        failure = failure or reason
        if stop_at is None:
            stop_at = time.monotonic()
            if child is not None and child.returncode is None:
                child.send_signal(signal.SIGINT)

    try:
        child = subprocess.Popen(
            argv,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            cwd=directory,
            close_fds=True,
        )
        tracker = o.OwnedTree(observer, child, observer.snapshot())
        for index, pipe in enumerate((child.stdout, child.stderr)):
            os.set_blocking(pipe.fileno(), False)
            selector.register(pipe, selectors.EVENT_READ, index)
        next_observation = time.monotonic()
        while True:
            current = time.monotonic()
            if interrupt[0]:
                interrupted = True
                request_stop("interrupted")
            if current >= min(started + limits["session_seconds"], campaign_deadline):
                timed_out = True
                request_stop("command_deadline")
            if stop_at is not None and current - stop_at >= limits["cleanup_seconds"]:
                if child.returncode is None:
                    child.kill()
                    forced = True
                    child.wait(timeout=5)
                break
            if current >= next_observation:
                try:
                    tracker.update(observer.snapshot())
                except BaseException as error:
                    request_stop("process_observation_failed: " + str(error)[:2048])
                next_observation = current + 0.25
            for key, _ in selector.select(0.05):
                part = os.read(key.fileobj.fileno(), 65536)
                if not part:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                    continue
                index = key.data
                remaining = limits["stream_bytes"] - sum(counts)
                retained = part[:remaining]
                streams[index].write(retained)
                counts[index] += len(retained)
                if len(retained) != len(part):
                    overflow = True
                    request_stop("output_overflow")
            returncode = child.poll()
            if returncode is not None and not selector.get_map():
                break
        if child.returncode is None:
            child.wait(timeout=5)
        retirement_deadline = min(
            time.monotonic() + 45,
            started + limits["session_seconds"] + limits["cleanup_seconds"],
        )
        while True:
            try:
                receipt = tracker.receipt(observer.snapshot())
            except BaseException as error:
                failure = (
                    failure or "retirement_observation_failed: " + str(error)[:2048]
                )
                break
            if (
                receipt["observed_identities_retired"]
                or time.monotonic() >= retirement_deadline
            ):
                break
            time.sleep(0.1)
    except BaseException as error:
        failure = (
            c.diagnostic(error)
            if failure is None
            else {"prior_failure": failure, "later_failure": c.diagnostic(error)}
        )
        if child is not None and child.returncode is None:
            try:
                child.send_signal(signal.SIGINT)
                try:
                    child.wait(timeout=limits["cleanup_seconds"])
                except subprocess.TimeoutExpired:
                    child.kill()
                    forced = True
                    child.wait(timeout=5)
            except BaseException as cleanup:
                cleanup_errors.append(cleanup)
        # An observation failure cannot be replaced by a fresh successful tracker.
    finally:

        def attempt(action):
            try:
                action()
            except BaseException as error:
                cleanup_errors.append(error)

        attempt(lambda: signal.signal(signal.SIGINT, previous_handler))
        attempt(selector.close)
        if child is not None:
            for pipe in (child.stdout, child.stderr):
                if pipe is not None and not pipe.closed:
                    attempt(pipe.close)
        for stream in streams:
            attempt(stream.flush)
            attempt(lambda: os.fsync(stream.fileno()))
            attempt(stream.close)
    identities = []
    for name in ("stdout.bin", "stderr.bin"):
        try:
            identities.append(m.identity(directory / name))
        except BaseException as error:
            cleanup_errors.append(error)
            identities.append(
                {
                    "path": str(directory / name),
                    "identity_unavailable": c.diagnostic(error),
                }
            )
    if cleanup_errors:
        failure = {
            "primary_failure": failure,
            "cleanup_errors": [c.diagnostic(error) for error in cleanup_errors],
        }
    return {
        "argv": argv,
        "returncode": None if child is None else child.returncode,
        "elapsed_seconds": time.monotonic() - started,
        "timed_out": timed_out,
        "interrupted": interrupted,
        "forced_kill": forced,
        "output_overflow": overflow,
        "failure": failure,
        "observation": receipt,
        "streams": identities,
        "signals_to_descendants": 0,
        "hostile_process_containment": False,
    }


def payload_rows(directory):
    raw = c.read(directory / "steps.jsonl", maximum=1024 * 1024)
    rows = [m.parse_json(line) for line in raw.splitlines()]
    m.require(
        len(rows) == 24 and [row["tick"] for row in rows] == list(range(1, 25)),
        "complete export tick roster",
    )
    flattened = []
    for row in rows:
        for item in row["payloads"]:
            selected = {key: item[key] for key in ("path", "bytes", "sha256")}
            raw = c.selected_file(selected, base=directory)
            flattened.append((row["tick"], item["sensor_id"], raw))
    m.require(
        len(flattened) == 44 and sum(len(row[2]) for row in flattened) == 4326400,
        "complete original payload accounting",
    )
    return rows, flattened


def verify_case(freeze, freeze_sha, case, command):
    from crebain_ncp_sensors import codec

    directory = Path(freeze["output_root"]) / case["case_id"]
    result = m.parse_json(c.read(directory / "result.json", maximum=m.MAX_JSON))
    m.require(
        result["case"] == case and result["freeze_sha256"] == freeze_sha,
        "exact case and freeze join",
    )
    m.require(
        result["status"] == "complete" and result["failure"] is None,
        "complete application outcome",
    )
    m.require(
        command["returncode"] == 0
        and command["failure"] is None
        and all(
            command[key] is False
            for key in ("timed_out", "interrupted", "forced_kill", "output_overflow")
        ),
        "complete owned command",
    )
    m.require(
        command["observation"] is not None
        and command["observation"]["observed_identities_retired"] is True,
        "observed process identities retired",
    )
    m.require(
        result["planned_ticks"] == result["completed_ticks"] == 24
        and result["payload_count"] == 44
        and result["span_overflow"] is False,
        "complete measured roster",
    )
    m.validate_lifecycle(
        result,
        clock_id=case["clock_id"],
        command_elapsed_seconds=command["elapsed_seconds"],
    )
    exports, flattened = payload_rows(directory)
    if case["route"] == "direct":
        m.require(result["cleanup_confirmed"] is True, "standalone cleanup receipt")
    else:
        process = result["process_exit"]
        m.require(
            process["returncode"] == 0
            and process["forced"] is False
            and process["cleanup_confirmed"] is True,
            "application cleanup receipt",
        )
        prepare, targets, expected = c.workload(freeze["workload"])
        observations = m.parse_json(
            c.read(directory / "observations.json", maximum=m.MAX_JSON)
        )
        m.require(len(observations) == 24, "observation roster")
        joined = []
        for observation, exported in zip(observations, exports, strict=True):
            m.require(
                observation["tick"] == exported["tick"], "observation export tick"
            )
            readings = []
            for reading, item in zip(
                observation["readings"], exported["payloads"], strict=True
            ):
                m.require(
                    reading["manifest"]["sensor_id"] == item["sensor_id"], "sensor join"
                )
                readings.append(
                    {
                        **reading,
                        "payload": {
                            key: item[key] for key in ("path", "bytes", "sha256")
                        },
                    }
                )
            joined.append(
                {
                    "tick": observation["tick"],
                    "batch": observation["batch"],
                    "requested_target": None
                    if targets.get(observation["tick"]) is None
                    else codec.raw(targets[observation["tick"]]),
                    "readings": readings,
                }
            )
        binding = c._binding_type(
            m.parse_json(c.read(directory / "binding.json", maximum=m.MAX_JSON))
        )
        pairs = list(c._wire_pairs(directory / "wire.bin"))
        _, terminal = c.replay_pairs(
            pairs,
            binding,
            prepare,
            targets,
            joined,
            directory,
            expected,
            freeze["runtime"]["source_identity"],
        )
        m.require(terminal == result["terminal"], "actual reconstructed terminal")
        if case["route"] == "canonical":
            m.require(
                c._capture_pairs(directory, binding, expected) == pairs,
                "canonical complete original wire equality",
            )
            from prisoma_agent_bridge.crebain import verify_sensor_run

            verify_sensor_run(directory / "run.jsonl")
    return result, flattened


def run_study(path):
    from crebain_ncp_sensors.runtime import InstalledRuntime

    m.require(sys.flags.isolated and sys.dont_write_bytecode, "isolated runner")
    raw = c.read(path, maximum=m.MAX_JSON)
    freeze, campaign = m.parse_json(raw), path.parent
    validate_freeze(freeze, raw)
    m.require(
        Path(sys.prefix).resolve()
        == Path(freeze["environments"]["canonical"]["prefix"]),
        "selected canonical verifier environment",
    )
    m.require(
        not (campaign / "execution-start.json").exists(), "campaign cannot be rerun"
    )
    runtime = InstalledRuntime.open(freeze["runtime"]["prefix"])
    m.require(
        runtime.manifest_sha256 == freeze["runtime"]["manifest_sha256"],
        "runtime before",
    )
    expected_inventory = m.parse_json(
        m.selected_file(freeze["environments"]["canonical"]["inventory"])
    )
    m.require(c.inventory() == expected_inventory, "verifier installed inventory")
    m.exclusive_json(
        campaign / "execution-start.json",
        {
            "freeze_sha256": hashlib.sha256(raw).hexdigest(),
            "started_unix_ns": time.time_ns(),
        },
    )
    deadline = time.monotonic() + freeze["limits"]["campaign_seconds"]
    completed, failure, reference = [], None, None
    try:
        for case in freeze["cases"]:
            m.require(time.monotonic() < deadline, "campaign deadline before launch")
            m.require(
                shutil.disk_usage(campaign).free
                >= freeze["limits"]["minimum_free_disk_bytes"],
                "selected free-disk floor",
            )
            # Reserve a full bounded case plus two stream files before effects.
            reserve = (
                freeze["limits"]["case_artifact_bytes"]
                + freeze["limits"]["stream_bytes"]
            )
            m.require(
                tree_size(campaign) + reserve
                <= freeze["limits"]["campaign_artifact_bytes"],
                "campaign artifact reservation",
            )
            validate_freeze(freeze, raw)
            command = capture(
                command_for(freeze, path, case),
                freeze["process_environment"],
                campaign / "commands" / case["case_id"],
                freeze["tools"]["observer_binary"]["path"],
                freeze["limits"],
                deadline,
            )
            command_path = campaign / "commands" / case["case_id"] / "result.json"
            m.exclusive_json(command_path, command)
            directory = Path(freeze["output_root"]) / case["case_id"]
            m.require(
                tree_size(directory) <= freeze["limits"]["case_artifact_bytes"],
                "case artifact accounting",
            )
            result, payloads = verify_case(
                freeze, hashlib.sha256(raw).hexdigest(), case, command
            )
            if reference is None:
                reference = payloads
            else:
                m.require(reference == payloads, "all arms original payload equality")
            completed.append(
                {
                    "case": case,
                    "command": m.identity(command_path),
                    "result": m.identity(directory / "result.json"),
                }
            )
            print(
                json.dumps(
                    {
                        "case": case["case_id"],
                        "completed_cases": len(completed),
                        "route_misses": sum(
                            row["route_missed"] for row in result["ticks"]
                        ),
                        "export_misses": sum(
                            row["export_missed"] for row in result["ticks"]
                        ),
                    }
                ),
                flush=True,
            )
    except BaseException as error:
        failure = c.diagnostic(error)
    try:
        validate_freeze(freeze, raw)
        after = InstalledRuntime.open(freeze["runtime"]["prefix"])
        m.require(
            after.manifest_sha256 == freeze["runtime"]["manifest_sha256"],
            "runtime after",
        )
        m.require(c.inventory() == expected_inventory, "verifier inventory after")
    except BaseException as error:
        failure = {"primary": failure, "final_identity_failure": c.diagnostic(error)}
    result = {
        "schema": "local.m1-performance-campaign.v1",
        "status": "COMPLETE_ACCOUNTING"
        if failure is None and len(completed) == 192
        else "INCOMPLETE",
        "freeze": m.identity(path),
        "completed": completed,
        "uncompleted_cases": freeze["cases"][len(completed) :],
        "failure": failure,
        "artifact_bytes": tree_size(campaign),
        "original_payload_bytes_equal": failure is None and len(completed) == 192,
        "authority": freeze["authority"],
    }
    m.exclusive_json(campaign / "result.json", result)
    m.require(
        result["status"] == "COMPLETE_ACCOUNTING",
        "campaign incomplete; no replacement trials",
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    operations = parser.add_subparsers(dest="operation", required=True)
    freezing = operations.add_parser("freeze")
    freezing.add_argument("path", type=Path, help="new private output directory")
    freezing.add_argument("--m1-freeze", type=Path, required=True)
    freezing.add_argument("--design", type=Path, required=True)
    freezing.add_argument("--review", type=Path, required=True)
    running = operations.add_parser("run")
    running.add_argument("path", type=Path, help="selected performance freeze")
    args = parser.parse_args()
    if args.operation == "freeze":
        print(
            json.dumps(
                freeze_study(
                    args.m1_freeze, args.design, args.review, args.path.resolve()
                )
            )
        )
    else:
        run_study(args.path)
