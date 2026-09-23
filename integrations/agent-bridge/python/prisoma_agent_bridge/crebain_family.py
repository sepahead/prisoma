"""One canonical experiment timeline over independently bound checkpoint peers.

The installed CREBAIN owner supplies native custody. This adapter records commands
before effects and verifies their original NCP bytes without starting a producer.
"""

from __future__ import annotations

import base64
from collections import deque
from contextlib import contextmanager
from dataclasses import dataclass
from io import BytesIO
import json
import os
from pathlib import Path
import stat
import threading
import time

from crebain_ncp_sensors import (
    codec as c,
    family_contract as fc,
    family_types as f,
    types as t,
)
from crebain_ncp_sensors.family import FamilySession
from crebain_ncp_sensors.family_owned import family_session
from ncp_local import modular_wire as w
from ncp_local.modular_buffer import CHUNK_BYTES
from prisoma_ncp_transcript import Journal, Peer, capacity_bytes, inspect

from . import Bridge, artifact_identity, hash_object, inspect_runlog
from .crebain import (
    CLOCK,
    ExperimentError,
    _contains_error,
    _json,
    _position,
    _require,
    _same,
    _sha,
)

SCHEMA = "prisoma.crebain_family_bridge.v1"
RECEIPT_SCHEMA = "prisoma.crebain_family_execution_receipt.v1"
INDEX_SCHEMA = "prisoma.crebain_family_capture_index.v1"
METHODS = tuple(
    "crebain.family." + name
    for name in (
        "prepare",
        "advance",
        "checkpoint",
        "commit_decision",
        "reserve",
        "restore",
        "evaluate",
        "branch_finish",
        "release_checkpoint",
        "finish",
    )
)
MAX_FORECAST_BYTES = 32_768
MAX_INDEX_BYTES = 65_536
MAX_EXCHANGES = 8190
MAX_CAPTURE_BYTES = 1024**3


@dataclass(frozen=True, slots=True)
class EndpointBudget:
    max_exchanges: int
    quota_bytes: int


@dataclass(frozen=True, slots=True)
class FamilyBudget:
    endpoints: tuple[EndpointBudget, ...]
    max_calls: int
    max_exchanges: int
    quota_bytes: int
    max_call_exchanges: int
    max_call_frame_bytes: int


def _peers(plan):
    return (
        Peer(plan.canonical_binding, fc.CanonicalContract),
        *(Peer(branch.binding, fc.EvaluationContract) for branch in plan.branches),
    )


def capture_budget(plan: f.FamilyPlan) -> FamilyBudget:
    """Admit the entire family before creating files or launching a producer."""
    plan = fc.freeze_plan(plan)
    total, landmark, branches = (
        plan.body.planned_ticks,
        plan.landmark_tick,
        len(plan.branches),
    )
    calls = total + 5 + branches * (total - landmark + 4)
    _require(calls <= 1024, "family application call budget")
    scene = plan.body.specification.scene
    advances = []
    for tick in range(1, total + 1):
        sizes = [
            camera.width * camera.height * 4
            for camera in (*scene.rgbCameras, *scene.thermalCameras)
            if tick % camera.periodTicks == 0
        ]
        samples = tick * 16_000 // 120 - (tick - 1) * 16_000 // 120
        sizes.extend([8 * samples] * len(scene.microphones))
        advances.append(
            2 + sum(2 * ((size + CHUNK_BYTES - 1) // CHUNK_BYTES) + 2 for size in sizes)
        )
    counts = (
        10 + 2 * branches + sum(advances),
        *([6 + sum(advances[landmark:])] * branches),
    )
    _require(sum(counts) <= MAX_EXCHANGES, "family aggregate exchange budget")
    endpoints = tuple(
        EndpointBudget(count, capacity_bytes((peer,), max_exchanges=count))
        for peer, count in zip(_peers(plan), counts, strict=True)
    )
    quota = sum(row.quota_bytes for row in endpoints)
    _require(quota <= MAX_CAPTURE_BYTES, "family aggregate journal budget")
    maximum = max(advances)
    return FamilyBudget(
        endpoints, calls, sum(counts), quota, maximum, 2 * maximum * w.FRAME_BYTES
    )


def _schedule(plan):
    """The frozen generic plan determines command order, endpoint, and tick."""
    rows = [(METHODS[0], 0, None)]
    rows.extend((METHODS[1], 0, tick) for tick in range(1, plan.landmark_tick + 1))
    rows.extend(((METHODS[2], 0, None), (METHODS[3], 0, None)))
    rows.extend(
        (METHODS[1], 0, tick)
        for tick in range(plan.landmark_tick + 1, plan.body.planned_ticks + 1)
    )
    for branch in plan.branches:
        rows.extend(((METHODS[4], 0, branch.slot), (METHODS[5], branch.slot, None)))
        rows.extend(
            (METHODS[1], branch.slot, tick)
            for tick in range(plan.landmark_tick + 1, plan.body.planned_ticks + 1)
        )
        rows.extend(((METHODS[6], branch.slot, None), (METHODS[7], branch.slot, None)))
    rows.extend(((METHODS[8], 0, None), (METHODS[9], 0, None)))
    return tuple(rows)


def _names(log, index_name, count):
    _require(
        type(index_name) is str and 0 < len(index_name.encode()) <= 96,
        "bounded capture index name required",
    )
    _require(
        index_name not in (".", "..")
        and Path(index_name).name == index_name
        and "\0" not in index_name
        and "/" not in index_name
        and "\\" not in index_name,
        "capture index must be a sibling basename",
    )
    names = tuple(f"{index_name}.{slot:02d}.ncp" for slot in range(count))
    _require(
        log.name not in (index_name, *names),
        "distinct canonical and capture names required",
    )
    return names


def _primary(request, response, peer):
    decoded = w.Request.decode(request, peer.binding, peer.contract)
    if type(decoded.command) is not w.Execute or type(
        decoded.command.operation
    ) not in (w.Prepare, w.Application, w.Finish):
        return None
    result = w.Response.decode(response, peer.binding, peer.contract)
    _require(result.outcome is w.Outcome.COMMITTED, "family primary did not commit")
    return {
        "request_sha256": _sha(request),
        "response_sha256": _sha(response),
        "request_digest": decoded.request_digest,
        "result_digest": result.result_digest,
        "operation": result.operation.value,
    }


@dataclass(frozen=True, slots=True)
class FamilyObservation:
    """Original bytes and the primary response, after every public buffer release."""

    slot: int
    requested_target: t.SetTarget | None
    response: f.FamilyAdvanced
    observation: t.BatchObservation


def _value(value):
    if type(value) is FamilyObservation:
        return {
            "response": c.raw(value.response),
            "readings": [
                {
                    "sensor_id": row.manifest.sensor_id,
                    "bytes": len(row.payload),
                    "sha256": _sha(row.payload),
                }
                for row in value.observation.readings
            ],
        }
    return value if type(value) is dict else c.raw(value)


def _receipt(slot, before, after, primary, value):
    _require(primary is not None, "family primary exchange is missing")
    return {
        "schema": RECEIPT_SCHEMA,
        "slot": slot,
        "span": {"before": c.raw(before), "after": c.raw(after)},
        "ncp": primary,
        "result": _value(value),
    }


def _forecast(payload):
    row = w.closed(payload, {"forecast_base64", "selected_case_id"})
    encoded = row["forecast_base64"]
    _require(
        type(encoded) is str
        and 0 < len(encoded) <= 4 * ((MAX_FORECAST_BYTES + 2) // 3),
        "bounded original forecast bytes required",
    )
    try:
        raw = base64.b64decode(encoded, validate=True)
    except (ValueError, TypeError) as error:
        raise ExperimentError("invalid original forecast encoding") from error
    _require(
        0 < len(raw) <= MAX_FORECAST_BYTES
        and base64.b64encode(raw).decode("ascii") == encoded,
        "noncanonical forecast encoding",
    )
    fc.decode("Token", row["selected_case_id"])
    return raw, row["selected_case_id"]


def _exit_receipt(value, count):
    """Check the recorded owner observation without claiming historical reobservation."""
    row = w.closed(
        value,
        {
            "schema",
            "pid",
            "returncode",
            "reason",
            "forced",
            "diagnostics_bytes",
            "diagnostics_truncated",
            "cleanup_confirmed",
            "endpoint_count",
        },
    )
    _require(
        row["schema"] == "crebain.family-process-exit.v1"
        and type(row["pid"]) is int
        and row["pid"] > 0
        and type(row["returncode"]) is int
        and row["returncode"] == 0
        and row["reason"] in ("producer_exit", "owner_shutdown")
        and row["forced"] is False
        and row["cleanup_confirmed"] is True
        and type(row["endpoint_count"]) is int
        and row["endpoint_count"] == count
        and type(row["diagnostics_bytes"]) is int
        and 0 <= row["diagnostics_bytes"] <= 1_048_576
        and type(row["diagnostics_truncated"]) is bool
        and row["diagnostics_truncated"] == (row["diagnostics_bytes"] > 131_072),
        "family owner retirement observation is incomplete",
    )
    return row


def _execute(family, step, payload):
    """Execute only the exact recorded command through public typed family methods."""
    method, slot, tick = step
    endpoint = family.canonical if slot == 0 else family.evaluations[slot - 1]
    if method == METHODS[1]:
        row = w.closed(payload, {"tick", "target"})
        _require(
            type(row["tick"]) is int and row["tick"] == tick == endpoint.next_tick,
            "family advance tick mismatch",
        )
        target = None if row["target"] is None else c.decode("SetTarget", row["target"])
        with endpoint.advance(target) as batch:
            result = FamilyObservation(
                slot, target, batch.response.body.data, batch.observation
            )
        return result
    if method == METHODS[3]:
        raw, selected = _forecast(payload)
        return endpoint.commit_decision(_sha(raw), selected)
    if method == METHODS[4]:
        row = w.closed(payload, {"case_id"})
        _require(
            row["case_id"] == family.plan.branches[tick - 1].case_id,
            "frozen branch order mismatch",
        )
        return family.reserve(row["case_id"]).reservation
    w.closed(payload, set())
    functions = {
        METHODS[2]: endpoint.checkpoint if slot == 0 else None,
        METHODS[5]: endpoint.prepare,
        METHODS[6]: getattr(endpoint, "evaluate", None),
        METHODS[7]: endpoint.finish,
        METHODS[8]: endpoint.release_checkpoint if slot == 0 else None,
    }
    invoke = functions.get(method)
    _require(callable(invoke), "unsupported family operation")
    return invoke()


def _descriptor(plan, budget, index_name):
    return {
        "schema": SCHEMA,
        "plan": c.raw(plan),
        "descriptor_sha256": _sha(fc.CanonicalContract.descriptor()),
        "capture": {"index_name": index_name, **c.raw(budget)},
    }


def _terminal(capture, budget):
    _require(
        capture.store_completion == "complete"
        and capture.exchange_pairs == budget.max_exchanges
        and capture.unanswered_request is False
        and capture.pending_peer_operations == 0
        and capture.finished_peers == capture.peer_count == 1,
        "incomplete family journal terminal",
    )


def _entry(slot, peer, name, identity, verification):
    return {
        "slot": slot,
        "binding": c.raw(peer.binding),
        "role": "canonical" if slot == 0 else "evaluation",
        "name": name,
        "identity": identity,
        "verification": verification,
    }


def _capture_index(plan, entries):
    return {
        "schema": INDEX_SCHEMA,
        "family_id": plan.family_id,
        "plan_hash": hash_object(_json(c.raw(plan))),
        "journals": entries,
    }


def _admit_index(plan, names, budget):
    """Reserve an upper bound before effects, using each admitted journal quota."""
    entries = []
    for slot, (peer, name, bound) in enumerate(
        zip(_peers(plan), names, budget.endpoints, strict=True)
    ):
        verification = {
            "store_completion": "complete",
            "exchange_pairs": bound.max_exchanges,
            "unanswered_request": False,
            "pending_peer_operations": 0,
            "finished_peers": 1,
            "peer_count": 1,
            "journal_bytes": bound.quota_bytes,
            "journal_digest": "0" * 64,
            "application_completion_validated": False,
            "scientific_validation": False,
        }
        entries.append(
            _entry(
                slot,
                peer,
                name,
                {"bytes": bound.quota_bytes, "sha256": "0" * 64},
                verification,
            )
        )
    _require(
        len((_json(_capture_index(plan, entries)) + "\n").encode()) <= MAX_INDEX_BYTES,
        "family index reservation exceeds its bound",
    )


def _file_metadata(value):
    return (
        value.st_dev,
        value.st_ino,
        value.st_mode,
        value.st_uid,
        value.st_nlink,
        value.st_size,
        value.st_mtime_ns,
        value.st_ctime_ns,
    )


def _index_bytes(path, writing=None):
    """One bounded no-follow descriptor; keep primary and cleanup failures intact."""
    flags = os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NONBLOCK
    flags |= os.O_RDONLY if writing is None else os.O_WRONLY | os.O_CREAT | os.O_EXCL
    fd = os.open(path, flags, 0o600)
    errors, result = [], None
    try:
        initial = os.fstat(fd)
        _require(
            stat.S_ISREG(initial.st_mode)
            and initial.st_uid == os.geteuid()
            and initial.st_nlink == 1
            and initial.st_mode & 0o077 == 0,
            "capture index must be an owner-private regular file",
        )
        if writing is None:
            _require(initial.st_size <= MAX_INDEX_BYTES, "capture index byte limit")
            data = bytearray()
            while len(data) <= MAX_INDEX_BYTES:
                chunk = os.read(fd, min(8192, MAX_INDEX_BYTES + 1 - len(data)))
                if not chunk:
                    break
                data.extend(chunk)
            _require(len(data) <= MAX_INDEX_BYTES, "capture index byte limit")
            _require(
                _file_metadata(initial)
                == _file_metadata(os.fstat(fd))
                == _file_metadata(path.lstat()),
                "capture index changed during read",
            )
            result = bytes(data)
        else:
            _require(
                type(writing) is bytes and len(writing) <= MAX_INDEX_BYTES,
                "capture index byte limit",
            )
            offset = 0
            while offset < len(writing):
                count = os.write(fd, memoryview(writing)[offset:])
                _require(count > 0, "capture index write made no progress")
                offset += count
            os.fsync(fd)
    except BaseException as error:
        errors.append(error)
    finally:
        try:
            os.close(fd)
        except BaseException as error:
            errors.append(error)
    if len(errors) == 1:
        raise errors[0]
    if errors:
        raise BaseExceptionGroup("capture index operation and cleanup failed", errors)
    return result


@dataclass(frozen=True, slots=True)
class FamilyRun:
    terminal: f.CanonicalTerminal
    captures: tuple
    run_log: dict


class FamilyExperiment:
    """Trusted collector; do not expose this capability to a predictor."""

    def __init__(
        self,
        runtime,
        plan,
        log_path,
        index_name="family.capture.json",
        *,
        actor_id="prisoma.family_host",
    ):
        self._thread = threading.get_ident()
        self._plan = fc.freeze_plan(plan)
        self._budget = capture_budget(self._plan)
        self._steps = _schedule(self._plan)
        self._log = Path(log_path).absolute()
        self._names = _names(self._log, index_name, self._plan.limits.endpoint_count)
        self._index_name = index_name
        _admit_index(self._plan, self._names, self._budget)
        _require(
            not os.path.lexists(self._log.parent / index_name),
            "capture index already exists",
        )
        self._runtime = runtime
        self._peers = _peers(self._plan)
        self._journals, self._captures = [], [None] * len(self._peers)
        self._bridge = self._family = self._context = None
        self._owner_entered = self._closed = False
        self._failure = self._completed = self._primary = None
        self._cursor = 0
        config = {
            "schema": "prisoma.application_bridge.v1",
            "run_id": self._plan.canonical_binding.run_id,
            "actor_id": actor_id,
            "methods": list(METHODS),
            "max_calls": self._budget.max_calls,
            "application": _descriptor(self._plan, self._budget, index_name),
        }
        # The Rust constructor checks its full canonical configuration before launch.
        try:
            self._bridge = Bridge(self._log, _json(config))
            for name, peer, budget in zip(
                self._names, self._peers, self._budget.endpoints, strict=True
            ):
                self._journals.append(
                    Journal(
                        self._log.parent / name,
                        (peer,),
                        max_exchanges=budget.max_exchanges,
                        quota_bytes=budget.quota_bytes,
                    )
                )
        except BaseException as error:
            self._close(error)

    @property
    def process_exit(self):
        return None if self._family is None else self._family.process_exit

    @property
    def diagnostics(self):
        return b"" if self._family is None else self._family.diagnostics

    @property
    def diagnostics_truncated(self):
        return False if self._family is None else self._family.diagnostics_truncated

    def _active(self):
        _require(
            threading.get_ident() == self._thread,
            "family experiment belongs to another thread",
        )
        _require(not self._closed, "family experiment is retired")

    def _exchange(self, slot):
        def exchange(request, reader, writer, *, deadline):
            _require(
                self._cursor < len(self._steps)
                and self._steps[self._cursor][1] == slot,
                "command used another family endpoint",
            )
            response = self._journals[slot].exchange(
                self._peers[slot].binding.endpoint_id,
                request,
                reader,
                writer,
                deadline=deadline,
            )
            primary = _primary(request, response, self._peers[slot])
            if primary is not None:
                _require(self._primary is None, "multiple family primary operations")
                self._primary = primary
            return response

        return exchange

    def _call(self, method, slot, payload):
        self._active()
        _require(
            type(slot) is int and 0 <= slot < len(self._peers), "invalid family slot"
        )
        _require(self._cursor < len(self._steps), "family command roster exhausted")
        step = self._steps[self._cursor]
        _require(step[:2] == (method, slot), "family command order or role mismatch")
        before = self._journals[slot].position()
        self._primary = None
        observed = None

        def callback(encoded):
            nonlocal observed
            recorded = json.loads(encoded)
            if method == METHODS[0]:
                decoded = fc.decode("FamilyPlan", recorded)
                _require(
                    _same(c.raw(decoded), c.raw(self._plan)),
                    "recorded family plan mismatch",
                )
                self._context = family_session(
                    self._runtime,
                    self._plan,
                    exchanges=tuple(self._exchange(i) for i in range(len(self._peers))),
                )
                self._family = self._context.__enter__()
                self._owner_entered = True
                observed = self._family.canonical.last_committed.body.data
            elif method == METHODS[9]:
                w.closed(recorded, set())
                self._owner_entered = False
                self._context.__exit__(None, None, None)
                observed = {
                    "terminal": c.raw(self._family.canonical.terminal),
                    "process_exit": _exit_receipt(
                        self._family.process_exit, len(self._peers)
                    ),
                }
            else:
                observed = _execute(self._family, step, recorded)
            after = self._journals[slot].position()
            _require(
                0
                < after.exchange_pairs - before.exchange_pairs
                <= self._budget.max_call_exchanges,
                "family call exchange bound",
            )
            return _json(_receipt(slot, before, after, self._primary, observed))

        try:
            self._bridge.dispatch(method, _json(payload), callback)
            self._cursor += 1
            if method == METHODS[7]:
                self._captures[slot] = self._journals[slot].finish()
                _terminal(self._captures[slot], self._budget.endpoints[slot])
        except BaseException as error:
            self._close(error)
        return observed

    def prepare(self):
        return self._call(METHODS[0], 0, c.raw(self._plan))

    def advance(self, target=None, *, slot=0):
        self._active()
        _require(
            type(slot) is int and 0 <= slot < len(self._peers), "invalid family slot"
        )
        _require(
            target is None or type(target) is t.SetTarget,
            "typed family target required",
        )
        if target is not None:
            c.validate_target(target, self._plan.body.specification.controller)
        _require(self._cursor < len(self._steps), "family command roster exhausted")
        return self._call(
            METHODS[1],
            slot,
            {
                "tick": self._steps[self._cursor][2],
                "target": None if target is None else c.raw(target),
            },
        )

    def checkpoint(self):
        return self._call(METHODS[2], 0, {})

    def commit_decision(self, forecast_bytes, selected_case_id):
        _require(
            type(forecast_bytes) is bytes
            and 0 < len(forecast_bytes) <= MAX_FORECAST_BYTES,
            "bounded original forecast bytes required",
        )
        payload = {
            "forecast_base64": base64.b64encode(forecast_bytes).decode("ascii"),
            "selected_case_id": selected_case_id,
        }
        _forecast(payload)
        return self._call(METHODS[3], 0, payload)

    def reserve(self, case_id):
        return self._call(METHODS[4], 0, {"case_id": case_id})

    def restore(self, slot):
        return self._call(METHODS[5], slot, {})

    def evaluate(self, slot):
        return self._call(METHODS[6], slot, {})

    def branch_finish(self, slot):
        return self._call(METHODS[7], slot, {})

    def release_checkpoint(self):
        return self._call(METHODS[8], 0, {})

    def finish(self):
        self._call(METHODS[9], 0, {})
        try:
            self._captures[0] = self._journals[0].finish()
            _terminal(self._captures[0], self._budget.endpoints[0])
            entries = [
                _entry(
                    slot,
                    peer,
                    name,
                    json.loads(artifact_identity(self._log, name)),
                    c.raw(capture),
                )
                for slot, (peer, name, capture) in enumerate(
                    zip(self._peers, self._names, self._captures, strict=True)
                )
            ]
            index = _capture_index(self._plan, entries)
            _index_bytes(
                self._log.parent / self._index_name, (_json(index) + "\n").encode()
            )
            canonical = json.loads(self._bridge.finish(self._index_name))
            self._completed = FamilyRun(
                self._family.canonical.terminal, tuple(self._captures), canonical
            )
        except BaseException as error:
            self._close(error)
        self._close()
        return self._completed

    def _close(self, primary=None):
        _require(
            threading.get_ident() == self._thread,
            "family experiment belongs to another thread",
        )
        errors = [] if primary is None else [primary]
        if self._failure is not None and (
            primary is None or _contains_error(primary, self._failure) is not True
        ):
            errors.insert(0, self._failure)
        if not self._closed:
            self._closed = True
            if self._owner_entered:
                self._owner_entered = False
                abort = (
                    ExperimentError("family experiment closed without canonical Finish")
                    if primary is None
                    else primary
                )
                try:
                    self._context.__exit__(
                        type(abort), abort, BaseException.__traceback__.__get__(abort)
                    )
                except BaseException as error:
                    if error is not abort:
                        errors.append(error)
                if primary is None:
                    errors.append(abort)
            for resource in (*self._journals, self._bridge):
                if resource is not None:
                    try:
                        resource.close()
                    except BaseException as error:
                        errors.append(error)
        if errors:
            self._failure = (
                errors[0]
                if len(errors) == 1
                else BaseExceptionGroup("family experiment and cleanup failed", errors)
            )
            raise self._failure

    def close(self):
        self._close()


@contextmanager
def owned_family_experiment(
    runtime,
    plan,
    log_path,
    index_name="family.capture.json",
    *,
    actor_id="prisoma.family_host",
):
    """Own an installed family; an explicit complete Finish is required before exit."""
    experiment = FamilyExperiment(
        runtime, plan, log_path, index_name, actor_id=actor_id
    )
    try:
        experiment.prepare()
        yield experiment
        _require(
            experiment._completed is not None,
            "finish the canonical family before leaving its context",
        )
    except BaseException as error:
        experiment._close(error)
    else:
        experiment.close()


@dataclass(frozen=True, slots=True)
class FamilyEvent:
    """Provisional reconstructed output; accept it only after inspection returns."""

    method: str
    slot: int
    payload: dict
    result: object
    capture_before: object
    capture_after: object


def _read_run(path):
    events = []
    canonical = json.loads(
        inspect_runlog(path, lambda encoded: events.append(json.loads(encoded)))
    )
    _require(
        len(events) >= 4
        and events[0]["type"] == "run_started"
        and events[1]["type"] == "config_logged",
        "family canonical prefix",
    )
    config = w.closed(
        events[1]["config"],
        {
            "schema",
            "run_id",
            "actor_id",
            "methods",
            "max_calls",
            "application",
            "clock",
        },
    )
    _require(
        config["schema"] == "prisoma.application_bridge.v1"
        and config["methods"] == list(METHODS)
        and _same(config["clock"], CLOCK),
        "family bridge configuration",
    )
    app = w.closed(
        config["application"], {"schema", "plan", "descriptor_sha256", "capture"}
    )
    plan = fc.freeze_plan(fc.decode("FamilyPlan", app["plan"]))
    budget = capture_budget(plan)
    capture = w.closed(app["capture"], {"index_name", *c.raw(budget)})
    index_name = capture["index_name"]
    names = _names(path, index_name, plan.limits.endpoint_count)
    _require(
        _same(app, _descriptor(plan, budget, index_name)),
        "family application descriptor mismatch",
    )
    _require(
        config["run_id"] == events[0]["run_id"] == plan.canonical_binding.run_id,
        "family canonical identity mismatch",
    )
    _require(
        type(config["max_calls"]) is int
        and config["max_calls"] == budget.max_calls
        and len(events) == 3 * budget.max_calls + 4,
        "complete family command roster",
    )
    calls = []
    for index, step in enumerate(_schedule(plan)):
        request, receipt, response = events[2 + 3 * index : 5 + 3 * index]
        method, slot, _ = step
        _require(
            [request["type"], receipt["type"], response["type"]]
            == ["bridge_request", "label_observed", "bridge_response"],
            "family canonical event order",
        )
        _require(
            request["request_id"]
            == response["request_id"]
            == f"application-{index + 1}"
            and request["method"] == method
            and request["step"] == receipt["step"] == response["step"] == index + 1,
            "family request join",
        )
        _require(
            request["actor"]
            == {
                "actor_type": "script",
                "actor_id": config["actor_id"],
                "session_id": None,
            },
            "family actor mismatch",
        )
        _require(
            receipt["name"] == "prisoma.application_result.v1"
            and receipt["metadata"]
            == {"record_role": "execution_receipt", "is_outcome_label": "false"}
            and receipt["timestamp_ns"] == response["timestamp_ns"]
            and response["ok"] is True,
            "family execution receipt role",
        )
        envelope = w.closed(receipt["value"], {"request_id", "method", "result"})
        _require(
            envelope["request_id"] == request["request_id"]
            and envelope["method"] == method
            and hash_object(_json(envelope["result"])) == response["result_hash"],
            "family result hash join",
        )
        result = w.closed(
            envelope["result"], {"schema", "slot", "span", "ncp", "result"}
        )
        _require(
            result["schema"] == RECEIPT_SCHEMA
            and type(result["slot"]) is int
            and result["slot"] == slot,
            "family receipt endpoint or schema",
        )
        span = w.closed(result["span"], {"before", "after"})
        before, after = _position(span["before"]), _position(span["after"])
        _require(
            0
            < after.exchange_pairs - before.exchange_pairs
            <= budget.max_call_exchanges,
            "family command span bound",
        )
        calls.append((step, request["payload"], result, before, after))
    artifact, ended = events[-2:]
    _require(
        artifact["type"] == "artifact_logged"
        and artifact["name"] == "application_capture"
        and artifact["kind"] == "application_supplied_bytes"
        and artifact["uri"] == index_name,
        "family capture index artifact",
    )
    _require(
        ended["type"] == "run_ended" and ended["status"] == "succeeded",
        "family canonical terminal",
    )
    identity = json.loads(artifact_identity(path, index_name))
    _require(
        identity["bytes"] <= MAX_INDEX_BYTES
        and artifact["sha256"] == identity["sha256"]
        and artifact["metadata"] == {"bytes": str(identity["bytes"])},
        "family capture index identity",
    )
    return canonical, plan, budget, index_name, names, identity, calls


def _read_index(path, name, plan, names, identity):
    raw = _index_bytes(path.parent / name)
    _require(
        len(raw) == identity["bytes"] and _sha(raw) == identity["sha256"],
        "family capture index changed",
    )
    index = w.closed(w.parse(raw), {"schema", "family_id", "plan_hash", "journals"})
    _require(
        index["schema"] == INDEX_SCHEMA
        and index["family_id"] == plan.family_id
        and index["plan_hash"] == hash_object(_json(c.raw(plan))),
        "family capture index plan join",
    )
    rows = index["journals"]
    _require(
        type(rows) is list and len(rows) == plan.limits.endpoint_count,
        "family journal roster size",
    )
    for slot, (row, peer, journal_name) in enumerate(
        zip(rows, _peers(plan), names, strict=True)
    ):
        w.closed(row, {"slot", "binding", "role", "name", "identity", "verification"})
        _require(
            type(row["slot"]) is int
            and row["slot"] == slot
            and row["name"] == journal_name
            and row["role"] == ("canonical" if slot == 0 else "evaluation")
            and _same(row["binding"], c.raw(peer.binding)),
            "family journal role or binding mismatch",
        )
        _require(
            row["identity"] == json.loads(artifact_identity(path, journal_name)),
            "family journal content mismatch",
        )
    return rows


class _Tape:
    def __init__(self, peer):
        self.peer, self.rows, self.primary = peer, deque(), None

    def exchange(self, request, reader, writer, *, deadline):
        _require(bool(self.rows), "missing family captured exchange")
        row = self.rows.popleft()
        _require(
            request == row.request,
            "family command does not reconstruct original request bytes",
        )
        primary = _primary(request, row.response, self.peer)
        if primary is not None:
            _require(self.primary is None, "extra family primary operation")
            self.primary = primary
        return row.response


def inspect_family_run(log_path, visit):
    """Reconstruct original observations without launching or contacting a producer.

    Visitor effects remain provisional until every child and parent terminal passes.
    The caller owns its retained observations and derived-output memory bounds.
    """
    _require(callable(visit), "family visitor must be callable")
    path = Path(log_path).absolute()
    initial = _file_metadata(path.lstat())
    canonical, plan, budget, index_name, names, identity, calls = _read_run(path)
    entries = _read_index(path, index_name, plan, names, identity)
    peers = _peers(plan)
    tapes = tuple(_Tape(peer) for peer in peers)
    stream = BytesIO()
    family = FamilySession(
        plan,
        [(stream, stream)] * len(peers),
        deadline=time.monotonic() + plan.limits.total_wall_seconds,
        close_endpoint=lambda slot: None,
        exchanges=tuple(tape.exchange for tape in tapes),
    )
    cursor, payload_count, raw_bytes = 0, 0, 0
    captures = [None] * len(peers)
    completed = None
    failures = []

    def inspect_slot(slot):
        nonlocal cursor, payload_count, raw_bytes, completed
        tape = tapes[slot]
        pending = []
        frame_bytes = 0

        def consume(row):
            nonlocal cursor, payload_count, raw_bytes, completed, frame_bytes
            _require(cursor < len(calls), "unassigned family exchange")
            step, payload, recorded, before, after = calls[cursor]
            method, expected_slot, next_slot = step
            _require(
                expected_slot == slot and row.peer.binding == peers[slot].binding,
                "family endpoint chronology",
            )
            if not pending:
                _require(row.before == before, "family span start mismatch")
            frame_bytes += len(row.request) + len(row.response)
            _require(
                len(pending) < budget.max_call_exchanges
                and frame_bytes <= budget.max_call_frame_bytes,
                "family captured command exceeds its bound",
            )
            pending.append(row)
            _require(row.after.records <= after.records, "family span end mismatch")
            if row.after != after:
                return
            tape.rows.extend(pending)
            tape.primary = None
            if method == METHODS[0]:
                _require(_same(payload, c.raw(plan)), "recorded family plan mismatch")
                observed = family.prepare()
            elif method == METHODS[9]:
                w.closed(payload, set())
                fields = w.closed(recorded["result"], {"terminal", "process_exit"})
                completed = family.finish()
                observed = {
                    "terminal": c.raw(completed),
                    "process_exit": _exit_receipt(fields["process_exit"], len(peers)),
                }
            else:
                observed = _execute(family, step, payload)
            _require(not tape.rows, "unexplained family captured exchanges")
            expected = _receipt(slot, before, after, tape.primary, observed)
            _require(_same(expected, recorded), "reconstructed family receipt mismatch")
            if type(observed) is FamilyObservation:
                payload_count += len(observed.observation.readings)
                raw_bytes += sum(
                    len(reading.payload) for reading in observed.observation.readings
                )
            visit(FamilyEvent(method, slot, payload, observed, before, after))
            cursor += 1
            pending.clear()
            frame_bytes = 0
            if method == METHODS[4]:
                # The suspended parent retains only its current parser/exchange
                # buffers. The complete child terminal precedes parent resumption.
                _require(
                    slot == 0 and captures[next_slot] is None,
                    "duplicate or nested branch capture",
                )
                inspect_slot(next_slot)

        capture = inspect(path.parent / names[slot], (peers[slot],), consume)
        _require(
            not pending and not tape.rows, "incomplete family command reconstruction"
        )
        _terminal(capture, budget.endpoints[slot])
        _require(
            _same(c.raw(capture), entries[slot]["verification"]),
            "family journal terminal mismatch",
        )
        _require(
            json.loads(artifact_identity(path, names[slot]))
            == entries[slot]["identity"],
            "family journal identity changed",
        )
        captures[slot] = capture

    try:
        inspect_slot(0)
        _require(
            cursor == len(calls) and completed is not None and all(captures),
            "incomplete family reconstruction",
        )
        for entry in entries:
            _require(
                json.loads(artifact_identity(path, entry["name"])) == entry["identity"],
                "family journal changed after inspection",
            )
        _require(
            json.loads(artifact_identity(path, index_name)) == identity,
            "family index changed after inspection",
        )
        _require(
            _file_metadata(path.lstat()) == initial,
            "canonical family log changed during inspection",
        )
        result = {
            "schema": "prisoma.crebain_family_run_verification.v1",
            "family_replayed": True,
            "family_id": plan.family_id,
            "endpoints": len(peers),
            "calls": cursor,
            "payload_count": payload_count,
            "raw_bytes": raw_bytes,
            "canonical": canonical,
            "captures": [c.raw(capture) for capture in captures],
            "capture_index": identity,
            "scientific_validation": False,
            "producer_process_retirement_verified": False,
        }
    except BaseException as primary:
        failures.append(primary)
    finally:
        for resource in (family, stream):
            try:
                resource.close()
            except BaseException as cleanup:
                failures.append(cleanup)
    if len(failures) == 1:
        raise failures[0]
    if failures:
        raise BaseExceptionGroup("family inspection and cleanup failed", failures)
    return result


def verify_family_run(log_path):
    """Verify the full canonical family and original bytes without retaining outputs."""
    return inspect_family_run(log_path, lambda event: None)
