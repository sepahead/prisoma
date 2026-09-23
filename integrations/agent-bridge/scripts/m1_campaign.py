#!/usr/bin/env python3
"""Frozen installed sensor-transfer controls. No simulator or process observer lives here."""

from __future__ import annotations

import argparse
import base64
from contextlib import contextmanager
from email.parser import BytesParser
import hashlib
from importlib import import_module, metadata
import io
import json
import os
from pathlib import Path
import re
import selectors
import signal
import stat
import struct
import subprocess
import sys
import time
from types import SimpleNamespace
import zipfile


MAX_JSON = 8 * 1024 * 1024
MAX_FILE = 128 * 1024 * 1024
MAX_OUTPUT = 512 * 1024 * 1024
MAX_RAW = 64 * 1024 * 1024
MAX_EXCHANGES = 4096
MAX_CHECKPOINT = 4096
PAIR = struct.Struct(">II")
TOKEN = re.compile(r"[a-z][a-z0-9_-]{0,63}\Z")
HEX = re.compile(r"[0-9a-f]{64}\Z")
UUID = re.compile(
    r"[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\Z"
)
SHARED_DISTRIBUTIONS = {"ncp-local", "crebain-ncp-sensors"}
CANONICAL_DISTRIBUTIONS = SHARED_DISTRIBUTIONS | {
    "prisoma-agent-bridge",
    "prisoma-ncp-transcript",
}


class CampaignError(ValueError):
    """A selected experiment or its evidence failed a named predicate."""

    def __init__(self, code):
        self.code = code
        super().__init__(code)


def require(condition, code):
    if not condition:
        raise CampaignError(code)


def closed(value, keys, code="shape"):
    require(type(value) is dict and value.keys() == set(keys), code)
    return value


def integer(value, low, high):
    return type(value) is int and low <= value <= high


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode()


def _pairs(rows):
    result = {}
    for key, value in rows:
        require(key not in result, "duplicate_key")
        result[key] = value
    return result


def parse(raw):
    require(type(raw) is bytes and len(raw) <= MAX_JSON, "json_bound")
    try:
        value = json.loads(
            raw,
            object_pairs_hook=_pairs,
            parse_constant=lambda _: (_ for _ in ()).throw(CampaignError("nonfinite")),
        )
    except (UnicodeError, json.JSONDecodeError, RecursionError) as error:
        raise CampaignError("json") from error
    stack, count = [(value, 0)], 0
    while stack:
        item, depth = stack.pop()
        count += 1
        require(count <= 100_000 and depth <= 32, "json_bound")
        if type(item) is dict:
            require(len(item) <= 20_000, "json_bound")
            stack.extend((child, depth + 1) for child in item.values())
        elif type(item) is list:
            require(len(item) <= 20_000, "json_bound")
            stack.extend((child, depth + 1) for child in item)
    return value


def _identity(info):
    return (
        info.st_dev,
        info.st_ino,
        info.st_mode,
        info.st_uid,
        info.st_nlink,
        info.st_size,
        info.st_mtime_ns,
        info.st_ctime_ns,
    )


def read(path, maximum=MAX_FILE):
    selected = Path(path)
    require(
        selected.is_absolute() and selected.resolve(strict=True) == selected,
        "direct_path",
    )
    fd = os.open(selected, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC)
    with os.fdopen(fd, "rb") as stream:
        before = os.fstat(stream.fileno())
        require(
            stat.S_ISREG(before.st_mode) and 0 <= before.st_size <= maximum,
            "file_bound",
        )
        raw = stream.read(maximum + 1)
        require(
            len(raw) == before.st_size
            and _identity(before)
            == _identity(os.fstat(stream.fileno()))
            == _identity(selected.lstat()),
            "file_changed",
        )
    return raw


def file_identity(path):
    path = Path(path)
    raw = read(path)
    return {"path": str(path), "bytes": len(raw), "sha256": digest(raw)}


def selected_file(row, *, base=None, maximum=MAX_FILE):
    closed(row, {"path", "bytes", "sha256"}, "file_shape")
    require(
        type(row["path"]) is str
        and integer(row["bytes"], 0, maximum)
        and type(row["sha256"]) is str
        and HEX.fullmatch(row["sha256"]),
        "file_shape",
    )
    if base is None:
        path = Path(row["path"])
    else:
        require(
            Path(row["path"]).name == row["path"]
            and row["path"] not in {"", ".", ".."},
            "artifact_path",
        )
        path = base / row["path"]
    raw = read(path, maximum)
    require(len(raw) == row["bytes"] and digest(raw) == row["sha256"], "file_digest")
    return raw


def private_directory(path):
    require(path.is_absolute() and path.resolve(strict=True) == path, "output_path")
    info = path.lstat()
    require(
        stat.S_ISDIR(info.st_mode)
        and info.st_uid == os.getuid()
        and stat.S_IMODE(info.st_mode) == 0o700,
        "output_owner",
    )
    return {
        "device": info.st_dev,
        "inode": info.st_ino,
        "uid": info.st_uid,
        "mode": stat.S_IMODE(info.st_mode),
    }


class Output:
    def __init__(self, path):
        self.path, self.total, self.names = path, 0, set()
        path.mkdir(mode=0o700)
        self.identity = private_directory(path)

    @contextmanager
    def open(self, name):
        require(Path(name).name == name, "artifact_path")
        fd = os.open(
            self.path / name,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
            0o600,
        )
        self.names.add(name)
        with os.fdopen(fd, "wb") as stream:
            yield stream
            stream.flush()
            os.fsync(stream.fileno())

    def append(self, stream, raw):
        require(
            type(raw) is bytes
            and len(raw) <= MAX_FILE
            and self.total + len(raw) <= MAX_OUTPUT,
            "output_bound",
        )
        stream.write(raw)
        stream.flush()
        self.total += len(raw)

    def write(self, name, raw):
        with self.open(name) as stream:
            self.append(stream, raw)
        return {"path": name, "bytes": len(raw), "sha256": digest(raw)}

    def json(self, name, value):
        raw = canonical(value) + b"\n"
        require(len(raw) <= MAX_JSON, "json_bound")
        return self.write(name, raw)


def inventory():
    """Observe installed files and import origins without loading a simulator."""
    prefix = Path(sys.prefix).resolve(strict=True)
    distributions, total, count = {}, 0, 0
    for distribution in metadata.distributions():
        name = re.sub(r"[-_.]+", "-", distribution.metadata["Name"]).lower()
        require(name not in distributions, "duplicate_distribution")
        files = {}
        for row in distribution.files or ():
            if "__pycache__" in Path(str(row)).parts or str(row).endswith(".pyc"):
                continue
            path = Path(distribution.locate_file(row)).resolve(strict=True)
            require(path.is_relative_to(prefix), "installed_origin")
            raw = read(path)
            total, count = total + len(raw), count + 1
            require(total <= 1024 * 1024 * 1024 and count <= 20_000, "inventory_bound")
            if row.hash is not None:
                observed = (
                    base64.urlsafe_b64encode(hashlib.new(row.hash.mode, raw).digest())
                    .rstrip(b"=")
                    .decode()
                )
                require(observed == row.hash.value, "installed_record")
            files[str(path.relative_to(prefix))] = {
                "bytes": len(raw),
                "sha256": digest(raw),
            }
        require(bool(files), "installed_record")
        distributions[name] = {"version": distribution.version, "files": files}
    require(SHARED_DISTRIBUTIONS <= distributions.keys(), "installed_packages")
    modules = {}
    names = ["ncp_local", "crebain_ncp_sensors"]
    if "prisoma-agent-bridge" in distributions:
        names += [
            "prisoma_agent_bridge",
            "prisoma_agent_bridge._native",
            "prisoma_agent_bridge.crebain",
        ]
    if "prisoma-ncp-transcript" in distributions:
        names += ["prisoma_ncp_transcript"]
    for name in names:
        path = Path(import_module(name).__file__).resolve(strict=True)
        require(path.is_relative_to(prefix), "installed_origin")
        modules[name] = {
            "path": str(path.relative_to(prefix)),
            "sha256": digest(read(path)),
        }
    return {
        "schema": "prisoma.m1-installed-inventory.v1",
        "prefix": str(prefix),
        "python": file_identity(Path(sys.executable).resolve(strict=True)),
        "distributions": distributions,
        "modules": modules,
    }


def _git_state(source):
    closed(source, {"root", "commit", "tree"}, "source_shape")
    root = Path(source["root"])
    require(root.is_absolute() and root.resolve(strict=True) == root, "source_root")
    for key in ("commit", "tree"):
        require(
            type(source[key]) is str and re.fullmatch(r"[0-9a-f]{40}", source[key]),
            "source_identity",
        )
    environment = {
        key: value for key, value in os.environ.items() if not key.startswith("GIT_")
    }
    environment.update(
        GIT_CONFIG_NOSYSTEM="1",
        GIT_CONFIG_GLOBAL=os.devnull,
        GIT_NO_REPLACE_OBJECTS="1",
        GIT_OPTIONAL_LOCKS="0",
    )
    commands = [
        ("rev-parse", "HEAD"),
        ("rev-parse", "HEAD^{tree}"),
        ("status", "--porcelain=v1", "--untracked-files=normal"),
    ]
    observed = []
    for command in commands:
        # Only clean HEAD/tree observations are accepted. Bound output while
        # reading, rather than after communicate() has allocated a dirty roster.
        child = subprocess.Popen(
            ["git", "-c", "core.fsmonitor=false", "-C", str(root), *command],
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        raw, deadline = bytearray(), time.monotonic() + 10
        try:
            with selectors.DefaultSelector() as selector:
                os.set_blocking(child.stdout.fileno(), False)
                selector.register(child.stdout, selectors.EVENT_READ)
                while selector.get_map():
                    remaining = deadline - time.monotonic()
                    require(remaining > 0, "source_deadline")
                    for key, _ in selector.select(remaining):
                        part = os.read(key.fd, 4096)
                        if not part:
                            selector.unregister(key.fd)
                            continue
                        require(len(raw) + len(part) <= 8192, "source_observation")
                        raw.extend(part)
                require(
                    child.wait(timeout=max(0, deadline - time.monotonic())) == 0,
                    "source_observation",
                )
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=5)
            child.stdout.close()
        observed.append(raw.decode().strip())
    require(observed == [source["commit"], source["tree"], ""], "source_changed")


def wheel_installation(package, selected, inventory):
    """Join selected wheel payloads to the already observed installed files."""
    raw = selected_file(selected)
    try:
        archive = zipfile.ZipFile(io.BytesIO(raw))
    except zipfile.BadZipFile as error:
        raise CampaignError("wheel_archive") from error
    with archive:
        entries = archive.infolist()
        require(
            len(entries) <= 20_000
            and len({row.filename for row in entries}) == len(entries)
            and sum(row.file_size for row in entries) <= 1024 * 1024 * 1024,
            "wheel_bound",
        )
        require(
            all(
                not Path(row.filename).is_absolute()
                and ".." not in Path(row.filename).parts
                and row.file_size <= MAX_FILE
                for row in entries
            ),
            "wheel_path",
        )
        metadata_names = [
            row.filename
            for row in entries
            if row.filename.endswith(".dist-info/METADATA")
        ]
        require(len(metadata_names) == 1, "wheel_distribution")
        metadata_name = metadata_names[0]
        require(archive.getinfo(metadata_name).file_size <= 65536, "wheel_bound")
        metadata_row = BytesParser().parsebytes(archive.read(metadata_name))
        require(
            len(metadata_row.get_all("Name", [])) == 1
            and len(metadata_row.get_all("Version", [])) == 1,
            "wheel_distribution",
        )
        installed = inventory["distributions"][package]
        require(
            re.sub(r"[-_.]+", "-", metadata_row["Name"]).lower() == package
            and metadata_row["Version"] == installed["version"],
            "wheel_distribution",
        )
        locations = [
            name for name in installed["files"] if name.endswith("/" + metadata_name)
        ]
        require(len(locations) == 1, "wheel_installation")
        prefix = locations[0][: -len(metadata_name)]
        verified = set()
        for row in entries:
            if row.is_dir() or row.filename == str(
                Path(metadata_name).with_name("RECORD")
            ):
                continue
            # These selected wheels install package/data files directly. Reject
            # an unqualified alternate install scheme instead of guessing paths.
            require(
                not any(part.endswith(".data") for part in Path(row.filename).parts),
                "wheel_install_scheme",
            )
            value = archive.read(row)
            require(
                installed["files"].get(prefix + row.filename)
                == {"bytes": len(value), "sha256": digest(value)},
                "wheel_installation",
            )
            verified.add(prefix + row.filename)
        generated = {
            prefix + str(Path(metadata_name).with_name(name))
            for name in ("RECORD", "INSTALLER", "REQUESTED", "direct_url.json")
        }
        require(set(installed["files"]) - generated == verified, "wheel_installation")


def load_freeze(path, *, verify_source=True):
    raw = read(Path(path), MAX_JSON)
    value = closed(
        parse(raw),
        {
            "schema",
            "campaign_id",
            "workload",
            "source",
            "tools",
            "runtime",
            "renderer",
            "renderer_parent",
            "environments",
            "limits",
            "output_root",
            "cases",
        },
        "freeze_shape",
    )
    require(
        value["schema"] == "prisoma.m1-freeze.v1"
        and type(value["campaign_id"]) is str
        and TOKEN.fullmatch(value["campaign_id"]),
        "freeze_schema",
    )
    selected_file(value["workload"], maximum=65536)
    closed(
        value["tools"],
        {
            "worker",
            "runner",
            "supervisor",
            "observer_source",
            "observer_binary",
            "observer_python",
            "fault_source",
            "fault_binary",
        },
        "tools_shape",
    )
    for row in value["tools"].values():
        selected_file(row)
    source = closed(value["source"], {"root", "commit", "tree"}, "source_shape")
    source_root = Path(source["root"])
    require(
        source_root.is_absolute() and source_root.resolve(strict=True) == source_root,
        "source_root",
    )
    require(
        all(
            Path(value["tools"][role]["path"]).is_relative_to(source_root)
            for role in (
                "worker",
                "runner",
                "supervisor",
                "observer_source",
                "observer_python",
                "fault_source",
            )
        )
        and Path(value["workload"]["path"]).is_relative_to(source_root),
        "source_file_join",
    )
    selected_file(value["renderer"])
    selected_file(value["renderer_parent"])
    require(
        value["tools"]["worker"] == file_identity(Path(__file__).resolve()),
        "worker_identity",
    )
    closed(
        value["runtime"],
        {"prefix", "manifest_sha256", "source_identity"},
        "runtime_shape",
    )
    closed(value["limits"], {"session_seconds", "checkpoint_seconds"}, "limits_shape")
    require(
        integer(value["limits"]["session_seconds"], 1, 600)
        and integer(value["limits"]["checkpoint_seconds"], 1, 60)
        and value["limits"]["checkpoint_seconds"] < value["limits"]["session_seconds"],
        "limits",
    )
    private_directory(Path(value["output_root"]))
    environments = closed(value["environments"], {"body", "canonical"}, "environments")
    for arm, environment in environments.items():
        closed(environment, {"prefix", "inventory", "wheels"}, "environment_shape")
        selected = parse(selected_file(environment["inventory"], maximum=MAX_JSON))
        require(
            selected["schema"] == "prisoma.m1-installed-inventory.v1"
            and selected["prefix"] == environment["prefix"],
            "environment_binding",
        )
        packages = SHARED_DISTRIBUTIONS if arm == "body" else CANONICAL_DISTRIBUTIONS
        require(
            set(environment["wheels"]) == packages
            and packages <= selected["distributions"].keys(),
            "wheel_roster",
        )
        require(
            not (
                {"numpy", "nest", "nest-simulator"} & selected["distributions"].keys()
            ),
            "unexpected_research_package",
        )
        if arm == "body":
            require(
                not (
                    {"prisoma-agent-bridge", "prisoma-ncp-transcript"}
                    & selected["distributions"].keys()
                ),
                "body_independence",
            )
        for package, row in environment["wheels"].items():
            wheel_installation(package, row, selected)
    require(
        environments["body"]["prefix"] != environments["canonical"]["prefix"],
        "distinct_environments",
    )
    for package in SHARED_DISTRIBUTIONS:
        left, right = (
            environments[arm]["wheels"][package] for arm in ("body", "canonical")
        )
        require(
            (left["bytes"], left["sha256"]) == (right["bytes"], right["sha256"]),
            "shared_wheel_identity",
        )
    require(
        type(value["cases"]) is list and 2 <= len(value["cases"]) <= 16, "case_roster"
    )
    identifiers = set()
    for case in value["cases"]:
        closed(case, {"case_id", "arm", "fault"}, "case_shape")
        require(
            type(case["case_id"]) is str
            and TOKEN.fullmatch(case["case_id"])
            and case["case_id"] not in identifiers
            and case["arm"] in {"body", "canonical"}
            and case["fault"] in {"none", "caller_loss", "renderer_loss"}
            and (case["arm"] == "canonical" or case["fault"] == "none"),
            "case_roster",
        )
        identifiers.add(case["case_id"])
    if verify_source:
        _git_state(value["source"])
    return value, digest(raw)


def selected_case(freeze, case_id):
    rows = [row for row in freeze["cases"] if row["case_id"] == case_id]
    require(len(rows) == 1, "case_selection")
    return rows[0]


def workload(row):
    from crebain_ncp_sensors import codec as c
    from crebain_ncp_sensors.contract import SensorContract
    from ncp_local.modular_buffer import CHUNK_BYTES

    value = closed(
        parse(selected_file(row, maximum=65536)),
        {
            "id",
            "status",
            "specification",
            "planned_ticks",
            "targets",
            "expected",
            "cases",
            "claim",
        },
        "workload_shape",
    )
    prepare = SensorContract.decode_prepare(
        {
            "specification": value["specification"],
            "planned_ticks": value["planned_ticks"],
            "composition_digest": c.COMPOSITION_DIGEST,
        }
    )
    require(
        1 <= prepare.planned_ticks <= 1022 and len(prepare.specification.drones) == 1,
        "workload_bound",
    )
    targets, previous = {}, 0
    require(
        type(value["targets"]) is list
        and len(value["targets"]) <= prepare.planned_ticks,
        "target_roster",
    )
    for row in value["targets"]:
        closed(row, {"tick", "droneId", "armed", "control"}, "target_shape")
        require(
            integer(row["tick"], previous + 1, prepare.planned_ticks)
            and row["droneId"] == prepare.specification.drones[0].id,
            "target_binding",
        )
        control = closed(
            row["control"],
            {"kind", "roll_rad", "pitch_rad", "heading_rad", "altitude_m"},
            "control_shape",
        )
        require(control["kind"] == "force_attitude_height", "target_kind")
        target = c.decode(
            "SetTarget", {**control, "kind": "set_target", "armed": row["armed"]}
        )
        c.validate_target(target, prepare.specification.controller)
        targets[row["tick"]], previous = target, row["tick"]
    require(1 in targets, "initial_target")
    scene = prepare.specification.scene
    counts, chunks, sizes, pressure = [2, 2], 0, [], 0
    for tick in range(1, prepare.planned_ticks + 1):
        due = [
            camera.width * camera.height * 4
            for camera in (*scene.rgbCameras, *scene.thermalCameras)
            if tick % camera.periodTicks == 0
        ]
        samples = (
            tick * prepare.specification.acoustic.sampleRateHz // 120
            - (tick - 1) * prepare.specification.acoustic.sampleRateHz // 120
        )
        pressure += samples * len(scene.microphones)
        due.extend([8 * samples] * len(scene.microphones))
        due_chunks = sum((size + CHUNK_BYTES - 1) // CHUNK_BYTES for size in due)
        counts.append(2 + 2 * due_chunks + 2 * len(due))
        chunks += due_chunks
        sizes.extend(due)
    expected = {
        "ticks": prepare.planned_ticks,
        "payloads": len(sizes),
        "raw_bytes": sum(sizes),
        "pressure_samples": pressure,
        "chunks": chunks,
        "exchanges": sum(counts),
        "max_call_exchanges": max(counts),
        "canonical_calls": prepare.planned_ticks + 2,
        "canonical_events": 3 * (prepare.planned_ticks + 2) + 4,
    }
    require(
        expected["raw_bytes"] <= MAX_RAW and expected["exchanges"] <= MAX_EXCHANGES,
        "workload_bound",
    )
    return prepare, targets, expected


def diagnostic(root, *, node_limit=128, edge_limit=256):
    """Retain identity and bounded primitive fields without formatting exceptions."""
    nodes, edges, identities = [], [], {}
    pending = [((root,), 0, None, "root")]
    truncated = False
    while pending:
        children, index, parent, relation = pending[-1]
        if index == len(children):
            pending.pop()
            continue
        if len(edges) >= edge_limit:
            truncated = True
            break
        error = children[index]
        pending[-1] = (children, index + 1, parent, relation)
        identifier = identities.get(id(error))
        repeated = identifier is not None
        if identifier is None:
            if len(nodes) >= node_limit:
                truncated = True
                break
            identifier = len(nodes)
            identities[id(error)] = identifier
            name = type.__dict__["__name__"].__get__(type(error))
            args = BaseException.args.__get__(error)
            messages = [item[:512] for item in args[:4] if type(item) is str]
            nodes.append(
                {
                    "id": identifier,
                    "type": name[:128],
                    "messages": messages,
                    "fields_truncated": len(args) > 4
                    or any(type(item) is str and len(item) > 512 for item in args[:4]),
                }
            )
        edges.append(
            {
                "from": parent,
                "to": identifier,
                "relation": relation,
                "repeated": repeated,
            }
        )
        if repeated:
            continue
        if issubclass(type(error), BaseExceptionGroup):
            pending.append(
                (BaseExceptionGroup.exceptions.__get__(error), 0, identifier, "group")
            )
        for attribute in ("__context__", "__cause__"):
            child = BaseException.__dict__[attribute].__get__(error)
            if child is not None:
                pending.append(((child,), 0, identifier, attribute))
    return {
        "schema": "prisoma.m1-error-graph.v1",
        "nodes": nodes,
        "edges": edges,
        "truncated": truncated,
        "messages_are_partial": True,
    }


def checkpoint(binding, stage, progress_fd, control_fd, deadline, timeout):
    message = {
        "schema": "prisoma.m1-checkpoint.v1",
        "freeze_sha256": binding["freeze_sha256"],
        "case_id": binding["case"]["case_id"],
        "stage": stage,
        **{
            key: binding["binding"][key]
            for key in ("run_id", "endpoint_id", "generation")
        },
    }
    expected = {**message, "schema": "prisoma.m1-continue.v1"}
    raw = canonical(message) + b"\n"
    require(len(raw) <= MAX_CHECKPOINT, "checkpoint_bound")
    for fd in (progress_fd, control_fd):
        require(
            integer(fd, 3, 1_048_576) and stat.S_ISFIFO(os.fstat(fd).st_mode),
            "checkpoint_pipe",
        )
    require(progress_fd != control_fd, "checkpoint_pipe")
    old = {fd: os.get_blocking(fd) for fd in (progress_fd, control_fd)}
    limit = min(deadline, time.monotonic() + timeout)
    try:
        for fd in old:
            os.set_blocking(fd, False)
        with selectors.DefaultSelector() as selector:
            selector.register(progress_fd, selectors.EVENT_WRITE)
            while raw:
                remaining = limit - time.monotonic()
                require(
                    remaining > 0 and selector.select(remaining), "checkpoint_timeout"
                )
                try:
                    count = os.write(progress_fd, raw)
                except BlockingIOError:
                    continue
                require(count > 0, "checkpoint_eof")
                raw = raw[count:]
            selector.unregister(progress_fd)
            selector.register(control_fd, selectors.EVENT_READ)
            raw = b""
            while b"\n" not in raw:
                remaining = limit - time.monotonic()
                require(
                    remaining > 0 and selector.select(remaining), "checkpoint_timeout"
                )
                try:
                    chunk = os.read(control_fd, MAX_CHECKPOINT + 1 - len(raw))
                except BlockingIOError:
                    continue
                require(bool(chunk), "checkpoint_eof")
                raw += chunk
                require(len(raw) <= MAX_CHECKPOINT, "checkpoint_bound")
            require(
                raw.endswith(b"\n")
                and raw.count(b"\n") == 1
                and canonical(parse(raw)) == canonical(expected),
                "checkpoint_binding",
            )
    finally:
        for fd, blocking in old.items():
            os.set_blocking(fd, blocking)


def runtime_selection(freeze):
    from crebain_ncp_sensors.runtime import InstalledRuntime

    selected = freeze["runtime"]
    runtime = InstalledRuntime.open(selected["prefix"])
    require(
        runtime.manifest_sha256 == selected["manifest_sha256"]
        and runtime.source_identity == selected["source_identity"],
        "runtime_changed",
    )
    return runtime


def environment_selection(freeze, arm):
    selected = freeze["environments"][arm]
    require(str(Path(sys.prefix).resolve()) == selected["prefix"], "environment_prefix")
    expected = parse(selected_file(selected["inventory"], maximum=MAX_JSON))
    require(inventory() == expected, "installed_changed")


def bind_case(freeze, freeze_digest, case, output):
    from crebain_ncp_sensors import codec as c, new_binding

    binding = new_binding()
    receipt = {
        "schema": "prisoma.m1-case-binding.v1",
        "freeze_sha256": freeze_digest,
        "campaign_id": freeze["campaign_id"],
        "case": case,
        "binding": c.raw(binding),
        "workload_sha256": freeze["workload"]["sha256"],
        "source": freeze["source"],
        "tools": freeze["tools"],
        "runtime": freeze["runtime"],
        "environment": freeze["environments"][case["arm"]],
        "output_path": str(output.path),
        "output_owner": output.identity,
    }
    identity = output.json("binding.json", receipt)
    return binding, receipt, identity["sha256"]


class WireRecorder:
    """Record complete body-only exchanges without importing a capture package."""

    def __init__(self, output, stream, maximum):
        self.output, self.stream, self.maximum, self.count = output, stream, maximum, 0

    def exchange(self, request, reader, writer, *, deadline):
        from ncp_local import wire as framing, modular_wire as w

        require(
            self.count < self.maximum
            and type(request) is bytes
            and 0 < len(request) <= w.FRAME_BYTES,
            "exchange_bound",
        )
        framing.write_local_frame(writer, request, deadline=deadline)
        response = framing.read_local_frame(reader, deadline=deadline)
        require(
            type(response) is bytes and 0 < len(response) <= w.FRAME_BYTES,
            "response_missing",
        )
        self.output.append(
            self.stream, PAIR.pack(len(request), len(response)) + request + response
        )
        self.count += 1
        return response


def export_step(output, stream, observation, target, counter):
    from crebain_ncp_sensors import codec as c

    readings = []
    for reading in observation.readings:
        name = f"payload-{counter:05d}.bin"
        counter += 1
        readings.append(
            {
                "manifest": c.raw(reading.manifest),
                "byte_manifest": c.raw(reading.byte_manifest),
                "payload": output.write(name, reading.payload),
            }
        )
    row = {
        "tick": observation.batch.body_tick,
        "requested_target": None if target is None else c.raw(target),
        "batch": c.raw(observation.batch),
        "readings": readings,
    }
    raw = canonical(row) + b"\n"
    require(len(raw) <= MAX_JSON, "step_bound")
    output.append(stream, raw)
    return counter


def execute_session(owner, *, canonical_arm, prepare, targets, output, notify, state):
    """Use an already selected public owner. The caller supplies process ownership."""
    from crebain_ncp_sensors import codec as c

    count = 0
    with output.open("steps.jsonl") as steps:
        with owner as session:
            state["session"] = session
            state["prepared"] = True
            notify("prepared")
            try:
                for tick in range(1, prepare.planned_ticks + 1):
                    state["attempted_tick"] = tick
                    target = targets.get(tick)
                    if canonical_arm:
                        observation = session.advance(target)
                    else:
                        with session.advance(target) as pending:
                            observation = pending.observation
                    state["validated_ticks"] = observation.batch.body_tick
                    count = export_step(output, steps, observation, target, count)
                    state["exported_ticks"] = tick
                    if tick == 1:
                        notify("tick1")
                completed = session.finish()
                state["explicit_finish"] = True
                if canonical_arm:
                    state["capture_finalized"] = (
                        completed.capture.store_completion == "complete"
                    )
                    state["canonical_finalized"] = True
                    completed = completed.session
                state["session_result"] = c.raw(completed)
            finally:
                # The public owner populates process_exit only after its __exit__.
                state["session"] = session


def _artifact_roster(output):
    rows = {}
    names = output.names | {"run.jsonl", "capture.ncp"}
    total = 0
    for name in sorted(names - {"result.json"}):
        path = output.path / name
        if not path.exists():
            continue
        raw = read(path)
        total += len(raw)
        require(total <= MAX_OUTPUT, "output_bound")
        rows[name] = {"path": name, "bytes": len(raw), "sha256": digest(raw)}
    return rows


def terminal_result(output, binding, binding_digest, state, primary=None):
    """Write bounded diagnostics without replacing the primary failure."""
    session = state.get("session")
    process_exit = None if session is None else session.process_exit
    diagnostics = b"" if session is None else session.diagnostics
    truncated = False if session is None else session.diagnostics_truncated
    require(
        type(diagnostics) is bytes
        and len(diagnostics) <= MAX_FILE
        and type(truncated) is bool,
        "diagnostics_bound",
    )
    output.write("diagnostics.bin", diagnostics)
    error_graph = None if primary is None else diagnostic(primary)
    result = {
        "schema": "prisoma.m1-worker-result.v1",
        "freeze_sha256": binding["freeze_sha256"],
        "case_id": binding["case"]["case_id"],
        "binding_sha256": binding_digest,
        "healthy": primary is None,
        "completion": {key: value for key, value in state.items() if key != "session"},
        "process_exit": process_exit,
        "diagnostics_truncated": truncated,
        "failure": error_graph,
        "artifacts": _artifact_roster(output),
    }
    output.json("result.json", result)
    return result


def worker(freeze_path, case_id, output_path, progress_fd, control_fd):
    require(sys.flags.isolated and sys.dont_write_bytecode, "isolated_python_required")
    freeze, freeze_digest = load_freeze(freeze_path)
    case = selected_case(freeze, case_id)
    environment_selection(freeze, case["arm"])
    runtime = runtime_selection(freeze)
    prepare, targets, expected = workload(freeze["workload"])
    path = Path(output_path)
    require(path == Path(freeze["output_root"]) / case_id, "case_output")
    output = Output(path)
    binding, receipt, binding_digest = bind_case(freeze, freeze_digest, case, output)
    state = {
        "prepared": False,
        "attempted_tick": None,
        "validated_ticks": 0,
        "exported_ticks": 0,
        "explicit_finish": False,
        "capture_finalized": None,
        "canonical_finalized": None,
        "session_result": None,
        "selection_rechecked": False,
    }
    # This additional worker budget starts before owner admission. It can only be
    # stricter than the public owner's later absolute session deadline.
    deadline = time.monotonic() + freeze["limits"]["session_seconds"]

    def notify(stage):
        checkpoint(
            receipt,
            stage,
            progress_fd,
            control_fd,
            deadline,
            freeze["limits"]["checkpoint_seconds"],
        )

    try:
        if case["arm"] == "canonical":
            from prisoma_agent_bridge.crebain import owned_sensor_experiment

            owner = owned_sensor_experiment(
                runtime,
                prepare,
                path / "run.jsonl",
                path / "capture.ncp",
                binding=binding,
                timeout_s=freeze["limits"]["session_seconds"],
            )
            execute_session(
                owner,
                canonical_arm=True,
                prepare=prepare,
                targets=targets,
                output=output,
                notify=notify,
                state=state,
            )
        else:
            from crebain_ncp_sensors import body_session

            with output.open("wire.bin") as stream:
                recorder = WireRecorder(output, stream, expected["exchanges"])
                owner = body_session(
                    runtime,
                    prepare,
                    binding=binding,
                    timeout_s=freeze["limits"]["session_seconds"],
                    exchange=recorder.exchange,
                )
                execute_session(
                    owner,
                    canonical_arm=False,
                    prepare=prepare,
                    targets=targets,
                    output=output,
                    notify=notify,
                    state=state,
                )
                require(recorder.count == expected["exchanges"], "exchange_count")
        require(case["fault"] == "none", "expected_fault_not_observed")
        environment_selection(freeze, case["arm"])
        require(runtime_selection(freeze) == runtime, "runtime_changed")
        require(load_freeze(freeze_path)[1] == freeze_digest, "freeze_changed")
        state["selection_rechecked"] = True
        process_exit = state["session"].process_exit
        require(
            type(process_exit) is dict
            and type(process_exit.get("returncode")) is int
            and process_exit["returncode"] == 0
            and process_exit.get("cleanup_confirmed") is True,
            "owner_cleanup",
        )
        return terminal_result(output, receipt, binding_digest, state)
    except BaseException as primary:
        try:
            if not (path / "result.json").exists():
                terminal_result(output, receipt, binding_digest, state, primary)
        except BaseException:
            # The original failure and its traceback remain the operation result.
            # Missing failure evidence blocks qualification instead of hiding it.
            pass
        raise


def _wire_pairs(path):
    from ncp_local import modular_wire as w

    raw = read(path)
    position, count = 0, 0
    while position < len(raw):
        require(len(raw) - position >= PAIR.size, "wire_truncated")
        request_size, response_size = PAIR.unpack_from(raw, position)
        position += PAIR.size
        require(
            0 < request_size <= w.FRAME_BYTES
            and 0 < response_size <= w.FRAME_BYTES
            and position + request_size + response_size <= len(raw)
            and count < MAX_EXCHANGES,
            "wire_bound",
        )
        request = raw[position : position + request_size]
        position += request_size
        response = raw[position : position + response_size]
        position += response_size
        count += 1
        yield request, response


def _equal(left, right):
    # These are already typed public records. JSON bytes preserve signed zero.
    return canonical(left) == canonical(right)


def _binding_type(value):
    from ncp_local.modular_buffer import BufferBinding

    closed(
        value,
        {"profile_digest", "application_digest", "run_id", "endpoint_id", "generation"},
        "binding_shape",
    )
    binding = BufferBinding(**value)
    binding.validate()
    return binding


def _validate_step(row, observation, target, output):
    from crebain_ncp_sensors import codec as c

    closed(row, {"tick", "requested_target", "batch", "readings"}, "step_shape")
    require(
        type(row["tick"]) is int
        and row["tick"] == observation.batch.body_tick
        and _equal(row["requested_target"], None if target is None else c.raw(target))
        and _equal(row["batch"], c.raw(observation.batch)),
        "step_binding",
    )
    require(
        type(row["readings"]) is list
        and len(row["readings"]) == len(observation.readings),
        "payload_roster",
    )
    payloads = []
    for exported, reading in zip(row["readings"], observation.readings):
        closed(exported, {"manifest", "byte_manifest", "payload"}, "reading_shape")
        raw = selected_file(exported["payload"], base=output)
        require(
            raw == reading.payload
            and _equal(exported["manifest"], c.raw(reading.manifest))
            and _equal(exported["byte_manifest"], c.raw(reading.byte_manifest)),
            "payload_binding",
        )
        manifest = reading.manifest
        payloads.append(
            {
                "sensor_id": manifest.sensor_id,
                "sensor_contract_digest": manifest.sensor_contract_digest,
                "source_body_tick": manifest.source_body_tick,
                "available_after_body_tick": manifest.available_after_body_tick,
                "tensor": c.raw(manifest.tensor),
                "payload": raw,
            }
        )
    slots = [
        {
            "kind": slot.kind,
            "sensor_id": slot.sensor_id,
            **({"next_due_tick": slot.next_due_tick} if slot.kind == "not_due" else {}),
        }
        for slot in observation.batch.slots
    ]
    return {
        "tick": row["tick"],
        "target": row["requested_target"],
        "slots": slots,
        "payloads": payloads,
    }


def _case_binding(freeze, freeze_digest, case_id, output):
    case = selected_case(freeze, case_id)
    require(output == Path(freeze["output_root"]) / case_id, "case_output")
    binding_raw = read(output / "binding.json", MAX_JSON)
    binding = closed(
        parse(binding_raw),
        {
            "schema",
            "freeze_sha256",
            "campaign_id",
            "case",
            "binding",
            "workload_sha256",
            "source",
            "tools",
            "runtime",
            "environment",
            "output_path",
            "output_owner",
        },
        "case_binding_shape",
    )
    require(
        binding["schema"] == "prisoma.m1-case-binding.v1"
        and binding["freeze_sha256"] == freeze_digest
        and binding["campaign_id"] == freeze["campaign_id"]
        and _equal(binding["case"], case)
        and binding["workload_sha256"] == freeze["workload"]["sha256"]
        and _equal(binding["source"], freeze["source"])
        and _equal(binding["tools"], freeze["tools"])
        and _equal(binding["runtime"], freeze["runtime"])
        and _equal(binding["environment"], freeze["environments"][case["arm"]])
        and binding["output_path"] == str(output)
        and _equal(binding["output_owner"], private_directory(output)),
        "case_binding",
    )
    selected_binding = _binding_type(binding["binding"])
    return case, binding, selected_binding, digest(binding_raw)


def _load_case(freeze, freeze_digest, case_id, output):
    case, binding, selected_binding, binding_digest = _case_binding(
        freeze, freeze_digest, case_id, output
    )
    result_raw = read(output / "result.json", MAX_JSON)
    result = closed(
        parse(result_raw),
        {
            "schema",
            "freeze_sha256",
            "case_id",
            "binding_sha256",
            "healthy",
            "completion",
            "process_exit",
            "diagnostics_truncated",
            "failure",
            "artifacts",
        },
        "result_shape",
    )
    require(
        result["schema"] == "prisoma.m1-worker-result.v1"
        and result["freeze_sha256"] == freeze_digest
        and result["case_id"] == case_id
        and result["binding_sha256"] == binding_digest,
        "result_binding",
    )
    require(
        type(result["artifacts"]) is dict and len(result["artifacts"]) <= 4096,
        "artifact_roster",
    )
    total = 0
    for name, row in result["artifacts"].items():
        require(row["path"] == name, "artifact_roster")
        total += len(selected_file(row, base=output))
        require(total <= MAX_OUTPUT, "output_bound")
    return case, binding, selected_binding, result, digest(result_raw)


def _birth(value):
    require(
        type(value) is list
        and len(value) == 3
        and integer(value[0], 1, 2**31 - 1)
        and integer(value[1], 1, 2**64 - 1)
        and integer(value[2], 0, 999_999),
        "owner_births",
    )
    return tuple(value)


def _process_row(value):
    closed(
        value,
        {"pid", "ppid", "uid", "status", "start_seconds", "start_microseconds"},
        "observation_identity",
    )
    identity = _birth(
        [value[key] for key in ("pid", "start_seconds", "start_microseconds")]
    )
    require(
        integer(value["ppid"], 0, 2**31 - 1)
        and integer(value["uid"], 0, 2**32 - 1)
        and integer(value["status"], 1, 5),
        "observation_identity",
    )
    return identity


def _observation(freeze, freeze_digest, case, binding_digest, owner, output):
    value = closed(
        parse(selected_file(owner["observer_events"], base=output)),
        {"schema", "freeze_sha256", "case_id", "observation", "supervisor_events"},
        "observation_shape",
    )
    require(
        value["schema"] == "prisoma.m1-owner-observation.v1"
        and value["freeze_sha256"] == freeze_digest
        and value["case_id"] == case["case_id"],
        "observation_binding",
    )
    observation = closed(
        value["observation"],
        {
            "schema",
            "observed_identity_count",
            "observed_identities_retired",
            "remaining_births",
            "events",
            "hostile_process_containment",
            "signals_sent_by_observer",
            "independent_application_receipt",
        },
        "observation_shape",
    )
    require(
        observation["schema"] == "local.observed-owned-tree.v1"
        and integer(observation["observed_identity_count"], 1, 256)
        and observation["observed_identity_count"] == len(owner["observed_births"])
        and observation["observed_identities_retired"] is True
        and observation["remaining_births"] == []
        and observation["hostile_process_containment"] is False
        and type(observation["signals_sent_by_observer"]) is int
        and observation["signals_sent_by_observer"] == 0
        and observation["independent_application_receipt"] is False,
        "observation_retirement",
    )
    events = observation["events"]
    require(type(events) is list and 2 <= len(events) <= 512, "observation_bound")
    known, absent, latest, root = {}, set(), {}, None
    for event in events:
        require(type(event) is dict, "observation_event")
        if event.get("kind") == "observed":
            closed(event, {"kind", "identity"}, "observation_event")
            row = event["identity"]
            identity = _process_row(row)
            require(
                identity not in known and row["uid"] == os.getuid(),
                "observation_history",
            )
            previous = latest.get(identity[0])
            require(
                previous is None or identity[1:] > previous[1:], "observation_history"
            )
            if root is None:
                root = identity
            else:
                parent = latest.get(row["ppid"])
                require(
                    parent is not None
                    and parent not in absent
                    and identity[1:] >= parent[1:],
                    "observation_parent",
                )
            known[identity] = row
            latest[identity[0]] = identity
        else:
            closed(event, {"kind", "birth"}, "observation_event")
            identity = _birth(event["birth"])
            require(
                event["kind"] == "identity_absent"
                and identity in known
                and identity not in absent,
                "observation_history",
            )
            absent.add(identity)
    require(
        set(known) == {tuple(row) for row in owner["observed_births"]}
        and absent == {tuple(row) for row in owner["retired_births"]},
        "observation_roster",
    )
    checkpoints = [
        {"kind": "checkpoint", "stage": stage, "binding_sha256": binding_digest}
        for stage in (
            ("prepared",) if case["fault"] == "caller_loss" else ("prepared", "tick1")
        )
    ]
    events = value["supervisor_events"]
    require(
        type(events) is list
        and len(events) <= 5
        and _equal(events[: len(checkpoints)], checkpoints),
        "observation_checkpoints",
    )
    if case["fault"] == "none":
        require(len(events) == 2, "observation_fault")
    elif case["fault"] == "caller_loss":
        require(
            _equal(
                events[1:],
                [
                    {
                        "kind": "caller_fault_sent",
                        "birth": list(root),
                        "signal": int(signal.SIGKILL),
                    }
                ],
            ),
            "observation_fault",
        )
    else:
        require(len(events) == 5, "observation_fault")
        intent = closed(
            events[2],
            {
                "kind",
                "identity",
                "executable",
                "parent_identity",
                "parent_executable",
                "signal",
            },
            "observation_fault",
        )
        selected = _process_row(intent["identity"])
        parent = _process_row(intent["parent_identity"])
        require(
            selected in known
            and selected != root
            and intent["identity"]["status"] != 5
            and intent["identity"]["uid"] == known[selected]["uid"]
            and parent in known
            and parent != selected
            and intent["parent_identity"]["status"] != 5
            and intent["parent_identity"]["uid"] == known[parent]["uid"]
            and intent["identity"]["ppid"] == intent["parent_identity"]["pid"]
            and intent["kind"] == "renderer_fault_intent"
            and _equal(intent["executable"], freeze["renderer"])
            and _equal(intent["parent_executable"], freeze["renderer_parent"])
            and type(intent["signal"]) is int
            and intent["signal"] == signal.SIGTERM
            and _equal(events[3], {"kind": "renderer_fault_return", "returncode": 0}),
            "observation_fault",
        )
        observed = closed(events[4], {"kind", "receipt"}, "observation_fault")
        receipt = closed(
            observed["receipt"],
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
            "observation_fault",
        )
        require(
            observed["kind"] == "renderer_fault_observed"
            and receipt["schema"] == "local.darwin-version-bound-fault.v1"
            and all(
                type(receipt[key]) is int and receipt[key] == intent["identity"][key]
                for key in ("pid", "start_seconds", "start_microseconds", "ppid")
            )
            and integer(receipt["pid_version"], 0, 2**32 - 1)
            and type(receipt["signal"]) is int
            and receipt["signal"] == signal.SIGTERM
            and type(receipt["signal_result"]) is int
            and receipt["signal_result"] == 0,
            "observation_fault",
        )
    return value


def verify_owner(freeze, freeze_digest, case, binding_digest, result_digest, output):
    owner = closed(
        parse(read(output / "owner.json", MAX_JSON)),
        {
            "schema",
            "freeze_sha256",
            "case_id",
            "binding_sha256",
            "result_sha256",
            "worker_returncode",
            "observed_births",
            "retired_births",
            "emergency_cleanup",
            "observer_events",
            "observer_sha256",
        },
        "owner_shape",
    )
    require(
        owner["schema"] == "prisoma.m1-owner.v1"
        and owner["freeze_sha256"] == freeze_digest
        and owner["case_id"] == case["case_id"]
        and owner["binding_sha256"] == binding_digest
        and owner["result_sha256"] == result_digest
        and owner["observer_sha256"] == freeze["tools"]["observer_binary"]["sha256"],
        "owner_binding",
    )
    sets = []
    for name in ("observed_births", "retired_births"):
        rows = owner[name]
        require(type(rows) is list and 1 <= len(rows) <= 256, "owner_births")
        observed = {_birth(row) for row in rows}
        require(len(observed) == len(rows), "owner_births")
        sets.append(observed)
    require(
        sets[0] == sets[1]
        and owner["emergency_cleanup"] is False
        and type(owner["worker_returncode"]) is int
        and (
            owner["worker_returncode"] == 0
            if case["fault"] == "none"
            else owner["worker_returncode"] == -signal.SIGKILL
            if case["fault"] == "caller_loss"
            else owner["worker_returncode"] != 0
        ),
        "owner_retirement",
    )
    _observation(freeze, freeze_digest, case, binding_digest, owner, output)
    return owner


def replay_pairs(
    pairs, selected_binding, prepare, targets, rows, output, expected, source_identity
):
    """Generate requests from the frozen input and require byte-exact originals."""
    from crebain_ncp_sensors import SensorSession, codec as c
    from crebain_ncp_sensors.contract import SensorContract
    from ncp_local import modular_wire as w

    iterator = iter(pairs)
    count, chunks, call_max = 0, 0, 0

    def exchange(request, _reader, _writer, *, deadline):
        nonlocal count, chunks
        require(time.monotonic() < deadline, "replay_deadline")
        row = next(iterator, None)
        require(row is not None and request == row[0], "original_request")
        decoded = w.Request.decode(row[0], selected_binding, SensorContract)
        if (
            type(decoded.command) is w.Execute
            and type(decoded.command.operation) is w.Read
        ):
            chunks += 1
        count += 1
        require(count <= expected["exchanges"], "exchange_bound")
        return row[1]

    stream = io.BytesIO()
    session = SensorSession(
        stream,
        stream,
        selected_binding,
        prepare,
        deadline=time.monotonic() + 180,
        exchange=exchange,
    )
    semantic = []
    try:
        prepared = session.prepare()
        require(prepared.source_identity == source_identity, "runtime_source")
        call_max = count
        require(len(rows) == prepare.planned_ticks, "step_roster")
        for tick, row in enumerate(rows, 1):
            before = count
            with session.advance(targets.get(tick)) as pending:
                observation = pending.observation
            call_max = max(call_max, count - before)
            semantic.append(_validate_step(row, observation, targets.get(tick), output))
        before = count
        completed = session.finish()
        call_max = max(call_max, count - before)
        require(next(iterator, None) is None, "unexplained_exchange")
        require(
            count == expected["exchanges"]
            and chunks == expected["chunks"]
            and call_max == expected["max_call_exchanges"]
            and completed.payload_count == expected["payloads"]
            and completed.raw_bytes == expected["raw_bytes"],
            "actual_accounting",
        )
        pressure = sum(
            reading["tensor"]["shape"][0]
            for step in semantic
            for reading in step["payloads"]
            if reading["tensor"]["kind"] == "pressure"
        )
        require(pressure == expected["pressure_samples"], "pressure_accounting")
        return semantic, c.raw(completed)
    finally:
        session.close()
        stream.close()


def _capture_pairs(output, selected_binding, expected):
    from crebain_ncp_sensors.contract import SensorContract
    from prisoma_ncp_transcript import Peer, inspect

    pairs, size = [], 0

    def visit(row):
        nonlocal size
        size += len(row.request) + len(row.response)
        require(len(pairs) < expected["exchanges"] and size <= MAX_FILE, "replay_bound")
        require(row.peer.binding == selected_binding, "captured_binding")
        pairs.append((row.request, row.response))

    capture = inspect(
        output / "capture.ncp", (Peer(selected_binding, SensorContract),), visit
    )
    require(capture.store_completion == "complete", "capture_completion")
    return pairs


def verify_arm(freeze, freeze_digest, case_id, output, *, require_owner=True):
    case, binding, selected_binding, result, result_digest = _load_case(
        freeze, freeze_digest, case_id, output
    )
    require(
        case["fault"] == "none"
        and result["healthy"] is True
        and result["failure"] is None,
        "healthy_result",
    )
    completion = closed(
        result["completion"],
        {
            "prepared",
            "attempted_tick",
            "validated_ticks",
            "exported_ticks",
            "explicit_finish",
            "capture_finalized",
            "canonical_finalized",
            "session_result",
            "selection_rechecked",
        },
        "completion_shape",
    )
    prepare, targets, expected = workload(freeze["workload"])
    require(
        completion["prepared"] is True
        and completion["explicit_finish"] is True
        and completion["selection_rechecked"] is True
        and all(
            type(completion[key]) is int and completion[key] == expected["ticks"]
            for key in ("attempted_tick", "validated_ticks", "exported_ticks")
        ),
        "completion_prefix",
    )
    require(
        type(result["diagnostics_truncated"]) is bool
        and type(result["process_exit"]) is dict
        and type(result["process_exit"].get("returncode")) is int
        and result["process_exit"]["returncode"] == 0
        and result["process_exit"].get("cleanup_confirmed") is True,
        "api_retirement",
    )
    step_bytes = read(output / "steps.jsonl")
    require(
        step_bytes.count(b"\n") == expected["ticks"] and step_bytes.endswith(b"\n"),
        "step_roster",
    )
    rows = [parse(line) for line in step_bytes.splitlines()]
    payload_names = []
    for row in rows:
        closed(row, {"tick", "requested_target", "batch", "readings"}, "step_shape")
        require(type(row["readings"]) is list, "payload_roster")
        for reading in row["readings"]:
            closed(reading, {"manifest", "byte_manifest", "payload"}, "reading_shape")
            payload_names.append(reading["payload"]["path"])
    require(
        len(payload_names) == expected["payloads"]
        and len(set(payload_names)) == len(payload_names),
        "payload_roster",
    )
    artifacts = {"binding.json", "steps.jsonl", "diagnostics.bin", *payload_names}
    artifacts.update(
        {"run.jsonl", "capture.ncp"} if case["arm"] == "canonical" else {"wire.bin"}
    )
    require(result["artifacts"].keys() == artifacts, "artifact_roster")
    canonical_report = None
    if case["arm"] == "canonical":
        from prisoma_agent_bridge.crebain import verify_sensor_run

        require(
            completion["capture_finalized"] is True
            and completion["canonical_finalized"] is True,
            "canonical_completion",
        )
        canonical_report = verify_sensor_run(output / "run.jsonl")
        require(
            canonical_report["canonical"]["events"] == expected["canonical_events"],
            "canonical_accounting",
        )
        pairs = _capture_pairs(output, selected_binding, expected)
    else:
        require(
            completion["capture_finalized"] is None
            and completion["canonical_finalized"] is None,
            "body_completion",
        )
        pairs = _wire_pairs(output / "wire.bin")
    semantic, completed = replay_pairs(
        pairs,
        selected_binding,
        prepare,
        targets,
        rows,
        output,
        expected,
        freeze["runtime"]["source_identity"],
    )
    require(_equal(completed, completion["session_result"]), "terminal_result")
    owner = None
    if require_owner:
        owner = verify_owner(
            freeze, freeze_digest, case, result["binding_sha256"], result_digest, output
        )
    return {
        "case": case,
        "binding": binding,
        "semantic": semantic,
        "expected": expected,
        "canonical": canonical_report,
        "owner": owner,
    }


def verify_command(freeze_path, freeze, freeze_digest, case_id):
    """Join the separately reaped supervisor result, never its self-reported intent."""
    case = selected_case(freeze, case_id)
    root = Path(freeze["output_root"])
    receipt = closed(
        parse(read(root / (case_id + ".command.json"), MAX_JSON)),
        {
            "schema",
            "freeze",
            "case",
            "runner",
            "supervisor",
            "argv",
            "limits",
            "returncode",
            "timed_out",
            "output_overflow",
            "interrupted",
            "forced_kill",
            "failure",
            "stdout",
            "stderr",
            "owner",
        },
        "command_shape",
    )
    seconds, checkpoint_seconds = (
        freeze["limits"][key] for key in ("session_seconds", "checkpoint_seconds")
    )
    expected_argv = [
        str(Path(freeze["environments"][case["arm"]]["prefix"]) / "bin/python"),
        "-I",
        "-B",
        freeze["tools"]["supervisor"]["path"],
        "--freeze",
        str(freeze_path),
        "--case",
        case_id,
    ]
    limits = {
        "wall_seconds": seconds + 2 * checkpoint_seconds + 30,
        "grace_seconds": checkpoint_seconds,
        "stream_bytes": 65536 + 4096 * (seconds + checkpoint_seconds),
    }
    require(
        receipt["schema"] == "prisoma.m1-command.v1"
        and _equal(receipt["freeze"], file_identity(freeze_path))
        and receipt["freeze"]["sha256"] == freeze_digest
        and _equal(receipt["case"], case)
        and _equal(receipt["runner"], freeze["tools"]["runner"])
        and _equal(receipt["supervisor"], freeze["tools"]["supervisor"])
        and _equal(receipt["argv"], expected_argv)
        and _equal(receipt["limits"], limits),
        "command_binding",
    )
    require(
        type(receipt["returncode"]) is int
        and receipt["returncode"] == 0
        and all(
            receipt[key] is False
            for key in ("timed_out", "output_overflow", "interrupted", "forced_kill")
        )
        and receipt["failure"] is None,
        "command_failed",
    )
    private_directory(root / (case_id + ".command"))
    for key in ("stdout", "stderr"):
        require(
            receipt[key]["path"] == str(root / (case_id + ".command") / (key + ".bin")),
            "command_path",
        )
        selected_file(receipt[key], maximum=limits["stream_bytes"])
    require(
        _equal(receipt["owner"], file_identity(root / case_id / "owner.json")),
        "command_owner",
    )
    return receipt


def _failure_graph(value):
    closed(
        value,
        {"schema", "nodes", "edges", "truncated", "messages_are_partial"},
        "failure_shape",
    )
    require(
        value["schema"] == "prisoma.m1-error-graph.v1"
        and type(value["truncated"]) is bool
        and value["messages_are_partial"] is True
        and type(value["nodes"]) is list
        and 1 <= len(value["nodes"]) <= 128
        and type(value["edges"]) is list
        and 1 <= len(value["edges"]) <= 256,
        "failure_bound",
    )
    for index, node in enumerate(value["nodes"]):
        closed(node, {"id", "type", "messages", "fields_truncated"}, "failure_shape")
        require(
            type(node["id"]) is int
            and node["id"] == index
            and type(node["type"]) is str
            and 1 <= len(node["type"]) <= 128
            and type(node["fields_truncated"]) is bool
            and type(node["messages"]) is list
            and len(node["messages"]) <= 4
            and all(
                type(text) is str and len(text) <= 512 for text in node["messages"]
            ),
            "failure_bound",
        )
    seen = set()
    for index, edge in enumerate(value["edges"]):
        closed(edge, {"from", "to", "relation", "repeated"}, "failure_shape")
        require(
            integer(edge["to"], 0, len(value["nodes"]) - 1)
            and type(edge["repeated"]) is bool
            and edge["repeated"] is (edge["to"] in seen),
            "failure_edge",
        )
        if index == 0:
            require(
                _equal(
                    edge, {"from": None, "to": 0, "relation": "root", "repeated": False}
                ),
                "failure_root",
            )
        else:
            require(
                type(edge["from"]) is int
                and edge["from"] in seen
                and edge["relation"] in {"group", "__cause__", "__context__"},
                "failure_edge",
            )
        seen.add(edge["to"])
    require(seen == set(range(len(value["nodes"]))), "failure_roster")


def _accepted_prefix(
    events, selected_binding, prepare, targets, source_identity, exported_row, output
):
    """Join an incomplete run's accepted prefix without claiming terminal replay."""
    from crebain_ncp_sensors import codec as c, types as t
    from prisoma_agent_bridge import hash_object

    require(
        len(events) >= 5
        and events[0].get("type") == "run_started"
        and events[0].get("run_id") == selected_binding.run_id
        and events[1].get("type") == "config_logged",
        "fault_canonical_prefix",
    )
    app = events[1]["config"]["application"]
    require(
        _equal(app["binding"], c.raw(selected_binding))
        and _equal(app["prepare"], c.raw(prepare)),
        "fault_canonical_binding",
    )
    accepted = []
    for index, method in enumerate(
        ("crebain.prepare",)
        if exported_row is None
        else ("crebain.prepare", "crebain.advance")
    ):
        request, label, response = events[2 + 3 * index : 5 + 3 * index]
        envelope = closed(
            label["value"], {"request_id", "method", "result"}, "fault_receipt"
        )
        require(
            request.get("type") == "bridge_request"
            and request.get("method") == method
            and label.get("type") == "label_observed"
            and label.get("name") == "prisoma.application_result.v1"
            and response.get("type") == "bridge_response"
            and response.get("ok") is True
            and request["request_id"]
            == response["request_id"]
            == envelope["request_id"]
            and envelope["method"] == method
            and request["payload_hash"] == hash_object(json.dumps(request["payload"]))
            and response["result_hash"] == hash_object(json.dumps(envelope["result"])),
            "fault_receipt",
        )
        accepted.append(envelope["result"])
    prepared = c.decode("Prepared", accepted[0]["observation"])
    c.validate_prepared(prepare, prepared, selected_binding)
    require(prepared.source_identity == source_identity, "runtime_source")
    if exported_row is not None:
        receipt = accepted[1]
        batch = c.decode("SensorBatch", exported_row["batch"])
        request_digest = receipt["ncp"]["request_digest"]
        context = SimpleNamespace(
            binding=selected_binding,
            request_digest=request_digest,
            predecessor=accepted[0]["ncp"]["result_digest"],
        )
        c.validate_batch_envelope(batch, context)
        command = t.Command("advance", 1, None, targets[1], t.CaptureReservation())
        c.validate_batch(
            prepare, prepared, command, t.Advanced("advanced", 1, request_digest, batch)
        )
        due = [slot for slot in batch.slots if slot.kind == "due"]
        require(len(due) == len(exported_row["readings"]), "payload_roster")
        summaries = []
        for slot, reading in zip(due, exported_row["readings"]):
            require(
                _equal(reading["manifest"], c.raw(slot.typed_manifest))
                and _equal(reading["byte_manifest"], c.raw(slot.byte_manifest)),
                "payload_binding",
            )
            raw = selected_file(reading["payload"], base=output)
            c.validate_payload(slot.typed_manifest, slot.byte_manifest, raw)
            summaries.append(
                {"sensor_id": slot.sensor_id, "bytes": len(raw), "sha256": digest(raw)}
            )
        require(
            _equal(
                receipt["observation"],
                {
                    "body_tick": 1,
                    "batch_digest": batch.batch_digest,
                    "readings": summaries,
                },
            ),
            "fault_receipt",
        )
    return prepared


def _runtime_diagnostics(raw, binding, prepared, freeze, observation):
    """Read the public diagnostic export after execution; it never selects a target."""
    prefix = b"CREBAIN_SENSOR_RUNTIME_V1 "
    lines = [
        line[len(prefix) :] for line in raw.splitlines() if line.startswith(prefix)
    ]
    require(
        len(lines) == 1 and len(lines[0]) + len(prefix) + 1 <= 4096,
        "runtime_diagnostics",
    )
    row = closed(
        parse(lines[0]),
        {
            "schema",
            "generation",
            "sequence",
            "run_id",
            "source_identity",
            "engine_owner_id",
            "scene_sha256",
            "graphics",
            "identity_scope",
        },
        "runtime_diagnostics",
    )
    require(
        row["schema"] == "crebain.sensor-engine-runtime-receipt.v1"
        and type(row["sequence"]) is int
        and row["sequence"] == 1
        and row["run_id"] == binding.run_id
        and row["source_identity"] == freeze["runtime"]["source_identity"]
        and row["engine_owner_id"] == prepared.engine_owner_id
        and row["scene_sha256"] == prepared.scene_sha256
        and row["identity_scope"]
        == "browser-reported-strings-not-loaded-code-or-hardware-proof",
        "runtime_diagnostics",
    )
    graphics = closed(
        row["graphics"],
        {
            "generation",
            "plan_sha256",
            "browser_pid",
            "worker_pid",
            "browser_version",
            "webgl_version",
            "renderer",
            "vendor",
            "distribution_scope",
        },
        "runtime_graphics",
    )
    require(
        type(row["generation"]) is str
        and UUID.fullmatch(row["generation"])
        and type(graphics["generation"]) is str
        and UUID.fullmatch(graphics["generation"])
        and type(graphics["plan_sha256"]) is str
        and HEX.fullmatch(graphics["plan_sha256"])
        and all(
            type(graphics[key]) is str
            and 1 <= len(graphics[key]) <= 256
            and all(32 <= ord(char) <= 126 for char in graphics[key])
            and graphics[key] != "unavailable"
            for key in ("browser_version", "webgl_version", "renderer", "vendor")
        ),
        "runtime_graphics",
    )
    intent = observation["supervisor_events"][2]
    require(
        integer(graphics["browser_pid"], 1, 2**31 - 1)
        and integer(graphics["worker_pid"], 1, 2**31 - 1)
        and graphics["browser_pid"] == intent["identity"]["pid"]
        and graphics["worker_pid"] == intent["parent_identity"]["pid"]
        and graphics["distribution_scope"] == "development-component",
        "runtime_graphics",
    )
    return row


def verify_fault_case(freeze, freeze_digest, case_id, output, *, require_owner=True):
    """Qualify the selected fault observation while preserving incomplete evidence."""
    from crebain_ncp_sensors import codec as c
    from crebain_ncp_sensors.contract import SensorContract
    from prisoma_ncp_transcript import CaptureError, Peer, verify

    case, binding, selected_binding, binding_digest = _case_binding(
        freeze, freeze_digest, case_id, output
    )
    require(
        case["arm"] == "canonical"
        and case["fault"] in {"caller_loss", "renderer_loss"},
        "fault_case",
    )
    prepare, targets, _ = workload(freeze["workload"])
    result, result_digest, row = None, None, None
    steps = read(output / "steps.jsonl")
    if case["fault"] == "caller_loss":
        require(
            not (output / "result.json").exists()
            and not (output / "result.json").is_symlink()
            and not steps,
            "caller_prefix",
        )
    else:
        _, _, _, result, result_digest = _load_case(
            freeze, freeze_digest, case_id, output
        )
        require(
            result["healthy"] is False
            and type(result["diagnostics_truncated"]) is bool,
            "fault_result",
        )
        _failure_graph(result["failure"])
        expected_completion = {
            "prepared": True,
            "attempted_tick": 2,
            "validated_ticks": 1,
            "exported_ticks": 1,
            "explicit_finish": False,
            "capture_finalized": None,
            "canonical_finalized": None,
            "session_result": None,
            "selection_rechecked": False,
        }
        require(_equal(result["completion"], expected_completion), "fault_prefix")
        require(steps.count(b"\n") == 1 and steps.endswith(b"\n"), "fault_prefix")
        row = closed(
            parse(steps),
            {"tick", "requested_target", "batch", "readings"},
            "step_shape",
        )
        require(
            type(row["tick"]) is int
            and row["tick"] == 1
            and _equal(row["requested_target"], c.raw(targets[1]))
            and type(row["readings"]) is list
            and len(row["readings"]) <= 128,
            "fault_prefix",
        )
        names = []
        for reading in row["readings"]:
            closed(reading, {"manifest", "byte_manifest", "payload"}, "reading_shape")
            selected_file(reading["payload"], base=output)
            names.append(reading["payload"]["path"])
        require(
            len(names) == len(set(names))
            and result["artifacts"].keys()
            == {
                "binding.json",
                "steps.jsonl",
                "diagnostics.bin",
                "run.jsonl",
                "capture.ncp",
                *names,
            },
            "artifact_roster",
        )
        # Preserve public uncertainty. Independent disappearance cannot rewrite it.
        process = result["process_exit"]
        require(
            process is None
            or (
                type(process) is dict
                and type(process.get("cleanup_confirmed")) is bool
                and (
                    process.get("returncode") is None
                    or type(process["returncode"]) is int
                )
            ),
            "fault_api_receipt",
        )
    raw = read(output / "run.jsonl")
    require(raw.endswith(b"\n") and raw.count(b"\n") <= 16, "fault_canonical_bound")
    events = [parse(row) for row in raw.splitlines()]
    require(
        all(type(row) is dict for row in events)
        and not any(
            row.get("type") == "run_ended" and row.get("status") == "succeeded"
            for row in events
        ),
        "false_terminal_claim",
    )
    requests = [row for row in events if row.get("type") == "bridge_request"]
    expected = [("crebain.prepare", c.raw(prepare))]
    if case["fault"] == "renderer_loss":
        expected += [
            ("crebain.advance", {"tick": 1, "target": c.raw(targets[1])}),
            ("crebain.advance", {"tick": 2, "target": None}),
        ]
    require(
        len(requests) == len(expected)
        and all(
            row.get("method") == method and _equal(row.get("payload"), payload)
            for row, (method, payload) in zip(requests, expected)
        ),
        "fault_canonical_prefix",
    )
    prepared = _accepted_prefix(
        events,
        selected_binding,
        prepare,
        targets,
        freeze["runtime"]["source_identity"],
        row,
        output,
    )
    capture_identity = file_identity(output / "capture.ncp")
    try:
        verify(output / "capture.ncp", (Peer(selected_binding, SensorContract),))
    except CaptureError as error:
        require(error.code == "terminal_missing", "wrong_incomplete_boundary")
    else:
        raise CampaignError("fault_capture_finalized")
    require(
        file_identity(output / "capture.ncp") == capture_identity, "capture_changed"
    )
    diagnostics = None
    if require_owner:
        owner = verify_owner(
            freeze, freeze_digest, case, binding_digest, result_digest, output
        )
        if case["fault"] == "renderer_loss":
            observation = _observation(
                freeze, freeze_digest, case, binding_digest, owner, output
            )
            diagnostics = _runtime_diagnostics(
                read(output / "diagnostics.bin"),
                selected_binding,
                prepared,
                freeze,
                observation,
            )
    process = None if result is None else result["process_exit"]
    return {
        "schema": "prisoma.m1-fault-verification.v1",
        "freeze_sha256": freeze_digest,
        "case_id": case_id,
        "binding_sha256": binding_digest,
        "status": "expected_fault_observed",
        "healthy": False,
        "application_process_exit": process,
        "application_cleanup_confirmed": None
        if process is None
        else process["cleanup_confirmed"],
        "completion": None if result is None else result["completion"],
        "capture_completeness_failure": "terminal_missing",
        "runtime_diagnostics": diagnostics,
        "partial_capture_is_authenticated": False,
        "observed_retirement_verified": require_owner,
        "scientific_validation": False,
    }


def verify_fault(freeze_path, case_id):
    freeze, freeze_digest = load_freeze(freeze_path)
    environment_selection(freeze, "canonical")
    runtime = runtime_selection(freeze)
    result = verify_fault_case(
        freeze, freeze_digest, case_id, Path(freeze["output_root"]) / case_id
    )
    verify_command(freeze_path, freeze, freeze_digest, case_id)
    environment_selection(freeze, "canonical")
    require(
        runtime_selection(freeze) == runtime
        and load_freeze(freeze_path)[1] == freeze_digest,
        "selection_changed",
    )
    return result


def compare(freeze_path, body_id, canonical_id):
    freeze, freeze_digest = load_freeze(freeze_path)
    environment_selection(freeze, "canonical")
    runtime = runtime_selection(freeze)
    root = Path(freeze["output_root"])
    body = verify_arm(freeze, freeze_digest, body_id, root / body_id)
    canonical_arm = verify_arm(freeze, freeze_digest, canonical_id, root / canonical_id)
    verify_command(freeze_path, freeze, freeze_digest, body_id)
    verify_command(freeze_path, freeze, freeze_digest, canonical_id)
    require(
        body["case"]["arm"] == "body" and canonical_arm["case"]["arm"] == "canonical",
        "arm_selection",
    )
    left, right = (row["binding"]["binding"] for row in (body, canonical_arm))
    require(
        all(left[key] != right[key] for key in ("run_id", "endpoint_id", "generation")),
        "independent_bindings",
    )
    # Semantic metadata is compared exactly. Binding-dependent commitments are
    # checked within each public replay, and never erased in exported evidence.
    require(body["semantic"] == canonical_arm["semantic"], "arm_payload_mismatch")
    environment_selection(freeze, "canonical")
    require(
        runtime_selection(freeze) == runtime
        and load_freeze(freeze_path)[1] == freeze_digest,
        "selection_changed",
    )
    return {
        "schema": "prisoma.m1-comparison.v1",
        "freeze_sha256": freeze_digest,
        "body_case": body_id,
        "canonical_case": canonical_id,
        "status": "pass",
        "accounting": body["expected"],
        "complete_payload_bytes_equal": True,
        "original_requests_reconstructed": True,
        "observed_retirement_verified": True,
        "scientific_validation": False,
        "real_time_qualification": False,
    }


def copied_negatives(log_path, output_path, *, selection=None, final_check=None):
    """Change fresh copies, reach deep verification, and reverify the originals."""
    from crebain_ncp_sensors import SessionError
    from prisoma_agent_bridge import hash_object
    from prisoma_agent_bridge.crebain import ExperimentError, verify_sensor_run
    from prisoma_ncp_transcript import CaptureError

    log_path, output_path = Path(log_path), Path(output_path)
    original = read(log_path)
    events = [parse(line) for line in original.splitlines()]
    capture_name = events[-2]["uri"]
    require(Path(capture_name).name == capture_name, "artifact_path")
    capture = read(log_path.parent / capture_name)
    positive = verify_sensor_run(log_path)
    output = Output(output_path)
    if selection is not None:
        output.json("binding.json", selection)
    outcomes = {}
    for control in ("copied-command", "capture-terminal"):
        child = Output(output.path / control)
        changed = parse(canonical(events))
        altered_capture = capture
        if control == "copied-command":
            requests = [
                event
                for event in changed
                if event["type"] == "bridge_request"
                and event["method"] == "crebain.advance"
                and event["payload"]["target"] is not None
            ]
            require(bool(requests), "target_roster")
            request = requests[0]
            require(
                request["payload"]["target"]["pitch_rad"] != 0.01,
                "control_not_distinct",
            )
            request["payload"]["target"]["pitch_rad"] = 0.01
            request["payload_hash"] = hash_object(json.dumps(request["payload"]))
        else:
            require(bool(capture), "capture_empty")
            altered_capture = capture[:-1] + bytes([capture[-1] ^ 1])
            changed[-2]["sha256"] = digest(altered_capture)
        child.write(capture_name, altered_capture)
        child.write(log_path.name, b"".join(canonical(row) + b"\n" for row in changed))
        try:
            verify_sensor_run(child.path / log_path.name)
        except SessionError as error:
            require(
                control == "copied-command"
                and type(error.__cause__) is ExperimentError
                and error.stage == "advance"
                and str(error.__cause__)
                == "recorded control does not reconstruct the original NCP request",
                "wrong_negative_boundary",
            )
            outcomes[control] = "original_request_rejected"
        except CaptureError as error:
            require(
                control == "capture-terminal" and error.code == "record_digest",
                "wrong_negative_boundary",
            )
            outcomes[control] = "terminal_record_digest_rejected"
        else:
            raise CampaignError("negative_accepted")
    require(
        read(log_path) == original
        and read(log_path.parent / capture_name) == capture
        and verify_sensor_run(log_path) == positive,
        "original_changed",
    )
    result = {
        "schema": "prisoma.m1-copied-controls.v1",
        "status": "pass",
        "controls": outcomes,
        "original_log_sha256": digest(original),
        "original_capture_sha256": digest(capture),
        "selection": selection,
    }
    if final_check is not None:
        final_check()
    output.json("result.json", result)
    return result


def selected_copied_negatives(freeze_path, case_id, output_path):
    freeze, freeze_digest = load_freeze(freeze_path)
    environment_selection(freeze, "canonical")
    runtime = runtime_selection(freeze)
    root = Path(freeze["output_root"])
    require(Path(output_path).parent == root, "control_output")
    original = verify_arm(freeze, freeze_digest, case_id, root / case_id)
    require(original["case"]["arm"] == "canonical", "canonical_case")
    command = verify_command(freeze_path, freeze, freeze_digest, case_id)
    selection = {
        "freeze": file_identity(freeze_path),
        "case": original["case"],
        "case_binding": file_identity(root / case_id / "binding.json"),
        "owner": command["owner"],
        "source": freeze["source"],
        "tools": freeze["tools"],
        "runtime": freeze["runtime"],
        "environment": freeze["environments"]["canonical"],
    }

    def final_check():
        verify_arm(freeze, freeze_digest, case_id, root / case_id)
        verify_command(freeze_path, freeze, freeze_digest, case_id)
        environment_selection(freeze, "canonical")
        require(
            runtime_selection(freeze) == runtime
            and load_freeze(freeze_path)[1] == freeze_digest,
            "selection_changed",
        )

    return copied_negatives(
        root / case_id / "run.jsonl",
        output_path,
        selection=selection,
        final_check=final_check,
    )


def provenance_negatives(freeze_path, body_id, canonical_id, output_path):
    """Apply selected readback variants to unchanged native arm evidence."""
    from crebain_ncp_sensors import SessionError, codec as c, new_binding

    freeze, freeze_digest = load_freeze(freeze_path)
    environment_selection(freeze, "canonical")
    runtime = runtime_selection(freeze)
    root = Path(freeze["output_root"])
    require(Path(output_path).parent == root, "control_output")
    cases = (body_id, canonical_id)

    def originals():
        identities = {}
        for case_id, arm in zip(cases, ("body", "canonical")):
            result = verify_arm(freeze, freeze_digest, case_id, root / case_id)
            require(result["case"]["arm"] == arm, "arm_selection")
            command = verify_command(freeze_path, freeze, freeze_digest, case_id)
            worker = parse(read(root / case_id / "result.json", MAX_JSON))
            owner = parse(read(root / case_id / "owner.json", MAX_JSON))
            names = {
                *worker["artifacts"],
                "result.json",
                "owner.json",
                owner["observer_events"]["path"],
            }
            identities[case_id] = {
                name: file_identity(root / case_id / name) for name in sorted(names)
            }
            identities[case_id]["command"] = file_identity(
                root / (case_id + ".command.json")
            )
            identities[case_id]["command_streams"] = {
                key: command[key] for key in ("stdout", "stderr")
            }
        return identities

    before = originals()
    output = Output(Path(output_path))
    output.json(
        "binding.json",
        {
            "freeze": file_identity(freeze_path),
            "cases": list(cases),
            "originals": before,
            "source": freeze["source"],
        },
    )
    outcomes = {}

    def rejected(name, expected, variant, action):
        output.json(name + ".json", variant)
        try:
            action()
        except CampaignError as error:
            require(error.code == expected, "wrong_negative_boundary")
        except SessionError as error:
            cause = BaseException.__cause__.__get__(error)
            require(
                type(cause) is CampaignError and cause.code == expected,
                "wrong_negative_boundary",
            )
        else:
            raise CampaignError("negative_accepted")
        outcomes[name] = expected

    stale = digest(read(freeze_path, MAX_JSON) + b"selected-stale-control")
    rejected(
        "stale-freeze",
        "case_binding",
        {"selected_freeze_sha256": stale},
        lambda: verify_arm(freeze, stale, body_id, root / body_id),
    )
    for case_id, arm in zip(cases, ("body", "canonical")):
        rejected(
            "two-stale-" + arm,
            "case_binding",
            {"selected_freeze_sha256": stale, "case_id": case_id},
            lambda case_id=case_id: verify_arm(freeze, stale, case_id, root / case_id),
        )
    mixed = parse(canonical(freeze))
    mixed["environments"]["canonical"]["wheels"]["prisoma-agent-bridge"] = freeze[
        "environments"
    ]["canonical"]["wheels"]["prisoma-ncp-transcript"]
    mixed_path = output.path / "mixed-wheel-freeze.json"
    rejected(
        "mixed-wheel-freeze",
        "wheel_distribution",
        mixed,
        lambda: load_freeze(mixed_path),
    )
    mixed_environment = parse(canonical(freeze))
    mixed_environment["environments"]["body"] = freeze["environments"]["canonical"]
    rejected(
        "mixed-arm-environment",
        "case_binding",
        mixed_environment,
        lambda: verify_arm(mixed_environment, freeze_digest, body_id, root / body_id),
    )
    rejected(
        "wrong-case",
        "case_output",
        {"selected_case": canonical_id, "directory_case": body_id},
        lambda: verify_arm(freeze, freeze_digest, canonical_id, root / body_id),
    )
    owner_copy = Output(output.path / "owner-copy")
    selected_owner = parse(read(root / canonical_id / "owner.json", MAX_JSON))
    changed_owner = {**selected_owner, "result_sha256": stale}
    owner_copy.json("owner.json", changed_owner)
    case = selected_case(freeze, canonical_id)
    rejected(
        "wrong-owner",
        "owner_binding",
        {"copy": file_identity(owner_copy.path / "owner.json")},
        lambda: verify_owner(
            freeze,
            freeze_digest,
            case,
            selected_owner["binding_sha256"],
            selected_owner["result_sha256"],
            owner_copy.path,
        ),
    )
    altered = parse(selected_file(freeze["workload"], maximum=65536))
    require(
        altered["targets"][0]["control"]["pitch_rad"] != 0.01, "control_not_distinct"
    )
    altered["targets"][0]["control"]["pitch_rad"] = 0.01
    output.json("altered-workload.json", altered)
    altered_identity = file_identity(output.path / "altered-workload.json")
    prepare, targets, expected = workload(altered_identity)
    original_prepare, original_targets, original_expected = workload(freeze["workload"])
    for case_id, arm in zip(cases, ("body", "canonical")):
        path = root / case_id
        binding = _binding_type(parse(read(path / "binding.json", MAX_JSON))["binding"])
        pairs = (
            _wire_pairs(path / "wire.bin")
            if arm == "body"
            else _capture_pairs(path, binding, expected)
        )
        rows = [parse(line) for line in read(path / "steps.jsonl").splitlines()]
        rejected(
            "matching-altered-workload-" + arm,
            "original_request",
            {"workload": altered_identity, "case_id": case_id},
            lambda pairs=pairs, binding=binding, rows=rows, path=path: replay_pairs(
                pairs,
                binding,
                prepare,
                targets,
                rows,
                path,
                expected,
                freeze["runtime"]["source_identity"],
            ),
        )
        for field in ("run_id", "generation"):
            changed_binding = _binding_type(
                {**c.raw(binding), field: getattr(new_binding(), field)}
            )
            rejected(
                "wrong-" + field + "-" + arm,
                "original_request",
                {"binding": c.raw(changed_binding), "case_id": case_id},
                lambda pairs=pairs,
                selected=changed_binding,
                rows=rows,
                path=path: replay_pairs(
                    pairs,
                    selected,
                    original_prepare,
                    original_targets,
                    rows,
                    path,
                    original_expected,
                    freeze["runtime"]["source_identity"],
                ),
            )
    require(_equal(originals(), before), "original_changed")
    environment_selection(freeze, "canonical")
    require(
        runtime_selection(freeze) == runtime
        and load_freeze(freeze_path)[1] == freeze_digest,
        "selection_changed",
    )
    report = {
        "schema": "prisoma.m1-provenance-controls.v1",
        "freeze_sha256": freeze_digest,
        "body_case": body_id,
        "canonical_case": canonical_id,
        "status": "pass",
        "healthy_baselines_reverified": True,
        "original_bytes_unchanged": True,
        "controls": outcomes,
        "scientific_validation": False,
    }
    output.json("result.json", report)
    return report


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("inventory")
    execute = commands.add_parser("worker")
    execute.add_argument("--freeze", type=Path, required=True)
    execute.add_argument("--case", required=True)
    execute.add_argument("--output", type=Path, required=True)
    execute.add_argument("--progress-fd", type=int, required=True)
    execute.add_argument("--control-fd", type=int, required=True)
    comparison = commands.add_parser("compare")
    comparison.add_argument("--freeze", type=Path, required=True)
    comparison.add_argument("--body", required=True)
    comparison.add_argument("--canonical", required=True)
    fault = commands.add_parser("verify-fault")
    fault.add_argument("--freeze", type=Path, required=True)
    fault.add_argument("--case", required=True)
    negative = commands.add_parser("copied-negatives")
    negative.add_argument("--freeze", type=Path, required=True)
    negative.add_argument("--case", required=True)
    negative.add_argument("--output", type=Path, required=True)
    provenance = commands.add_parser("provenance-negatives")
    provenance.add_argument("--freeze", type=Path, required=True)
    provenance.add_argument("--body", required=True)
    provenance.add_argument("--canonical", required=True)
    provenance.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    require(sys.flags.isolated and sys.dont_write_bytecode, "isolated_python_required")
    if args.command == "inventory":
        result = inventory()
    elif args.command == "worker":
        result = worker(
            args.freeze, args.case, args.output, args.progress_fd, args.control_fd
        )
    elif args.command == "compare":
        result = compare(args.freeze, args.body, args.canonical)
    elif args.command == "verify-fault":
        result = verify_fault(args.freeze, args.case)
    elif args.command == "provenance-negatives":
        result = provenance_negatives(
            args.freeze, args.body, args.canonical, args.output
        )
    else:
        result = selected_copied_negatives(args.freeze, args.case, args.output)
    sys.stdout.buffer.write(canonical(result) + b"\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
