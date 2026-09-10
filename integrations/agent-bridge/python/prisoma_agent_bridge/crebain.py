"""Canonical sensor-session commands with read-only reconstruction from NCP bytes."""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass
import hashlib
from io import BytesIO
import json
from pathlib import Path
import threading
import time
from typing import BinaryIO

from crebain_ncp_sensors import SensorContract, SensorSession, codec as c, types as t
from ncp_local import modular_wire as w
from ncp_local.modular_buffer import BufferBinding, CHUNK_BYTES
from prisoma_ncp_transcript import (
    Exchange,
    Journal,
    Peer,
    Position,
    Verification,
    capacity_bytes,
    inspect,
)

from . import Bridge, artifact_identity, hash_object, inspect_runlog

SCHEMA = "prisoma.crebain_sensor_bridge.v1"
RECEIPT_SCHEMA = "prisoma.crebain_execution_receipt.v1"
METHODS = ("crebain.prepare", "crebain.advance", "crebain.finish")
CLOCK = {
    "origin": "bridge_creation",
    "unit": "ns",
    "kind": "host_elapsed_monotonic",
    "simulation_time": False,
}


class ExperimentError(ValueError):
    """An application execution or evidence join failed. No retry is authorized."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ExperimentError(message)


def _json(value: object) -> str:
    return json.dumps(value, allow_nan=False, separators=(",", ":"))


def _sha(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _same(left: object, right: object) -> bool:
    # These values are parsed JSON projections. The frozen-record adapter is
    # reserved for dataclass inputs and intentionally rejects mutable mappings.
    return w.typed_digest(w.PROFILE_DOMAIN, left) == w.typed_digest(
        w.PROFILE_DOMAIN, right
    )


@dataclass(frozen=True, slots=True)
class CaptureBudget:
    max_exchanges: int
    quota_bytes: int
    max_call_exchanges: int
    max_call_frame_bytes: int


def capture_budget(binding: BufferBinding, prepare: t.Prepare) -> CaptureBudget:
    """Bound the installed 120-Hz sensor contract before any producer operation."""
    SensorContract.check_input(w.Prepare(prepare))
    _require(prepare.planned_ticks + 2 <= 1024, "application call budget")
    scene = prepare.specification.scene
    counts = [2, 2]  # Prepare/ACK and Finish/ACK.
    for tick in range(1, prepare.planned_ticks + 1):
        sizes = [
            camera.width * camera.height * 4
            for camera in (*scene.rgbCameras, *scene.thermalCameras)
            if tick % camera.periodTicks == 0
        ]
        samples = tick * 16_000 // 120 - (tick - 1) * 16_000 // 120
        sizes.extend([8 * samples] * len(scene.microphones))
        # Each advance, chunk read, and release has one acknowledgement.
        counts.append(
            2 + sum(2 * ((size + CHUNK_BYTES - 1) // CHUNK_BYTES) + 2 for size in sizes)
        )
    maximum = sum(counts)
    peers = (Peer(binding, SensorContract),)
    return CaptureBudget(
        maximum,
        capacity_bytes(peers, max_exchanges=maximum),
        max(counts),
        2 * w.FRAME_BYTES * max(counts),
    )


def _primary(request: bytes, response: bytes, binding: BufferBinding) -> dict | None:
    decoded = w.Request.decode(request, binding, SensorContract)
    if type(decoded.command) is not w.Execute or type(
        decoded.command.operation
    ) not in (w.Prepare, w.Application, w.Finish):
        return None
    result = w.Response.decode(response, binding, SensorContract)
    _require(result.outcome is w.Outcome.COMMITTED, "primary operation did not commit")
    return {
        "request_sha256": _sha(request),
        "response_sha256": _sha(response),
        "request_digest": decoded.request_digest,
        "result_digest": result.result_digest,
        "operation": result.operation.value,
    }


def _observation(value: t.Prepared | t.BatchObservation | t.SessionResult) -> dict:
    if type(value) is not t.BatchObservation:
        return c.raw(value)
    return {
        "body_tick": value.batch.body_tick,
        "batch_digest": value.batch.batch_digest,
        "readings": [
            {
                "sensor_id": row.manifest.sensor_id,
                "bytes": len(row.payload),
                "sha256": _sha(row.payload),
            }
            for row in value.readings
        ],
    }


def _receipt(
    before: Position, after: Position, primary: dict | None, value: object
) -> dict:
    _require(primary is not None, "missing primary NCP exchange")
    return {
        "schema": RECEIPT_SCHEMA,
        "span": {"before": c.raw(before), "after": c.raw(after)},
        "ncp": primary,
        "observation": _observation(value),
    }


def _target(payload: dict, prepare: t.Prepare, next_tick: int) -> t.SetTarget | None:
    row = w.closed(payload, {"tick", "target"})
    _require(
        type(row["tick"]) is int and row["tick"] == next_tick, "recorded tick mismatch"
    )
    target = None if row["target"] is None else c.decode("SetTarget", row["target"])
    if target is not None:
        c.validate_target(target, prepare.specification.controller)
    return target


@dataclass(frozen=True, slots=True)
class SensorRun:
    session: t.SessionResult
    capture: Verification
    run_log: dict


class SensorExperiment:
    """One host-owned CREBAIN session. The host retains process and stream ownership."""

    def __init__(
        self,
        log_path: str | Path,
        capture_path: str | Path,
        reader: BinaryIO,
        writer: BinaryIO,
        binding: BufferBinding,
        prepare: t.Prepare,
        *,
        deadline: float,
        actor_id: str = "prisoma.sensor_host",
    ) -> None:
        log, capture = Path(log_path).absolute(), Path(capture_path).absolute()
        _require(
            log.parent.resolve(strict=True) == capture.parent.resolve(strict=True),
            "capture must be a log sibling",
        )
        _require(
            log.name != capture.name and len(capture.name.encode()) <= 128,
            "distinct bounded capture name required",
        )
        budget = capture_budget(binding, prepare)
        self._owner = threading.get_ident()
        self._retired = False
        self._prepared = False
        self._binding, self._plan, self._budget = binding, prepare, budget
        self._primary = None
        self._journal = self._bridge = None
        self._capture_name = capture.name
        self._session = SensorSession(
            reader, writer, binding, prepare, deadline=deadline, exchange=self._exchange
        )
        config = {
            "schema": "prisoma.application_bridge.v1",
            "run_id": binding.run_id,
            "actor_id": actor_id,
            "methods": list(METHODS),
            "max_calls": prepare.planned_ticks + 2,
            "application": {
                "schema": SCHEMA,
                "binding": c.raw(binding),
                "prepare": c.raw(prepare),
                "descriptor_sha256": _sha(SensorContract.descriptor()),
                "capture": {"name": capture.name, **c.raw(budget)},
            },
        }
        try:
            self._journal = Journal(
                capture,
                (Peer(binding, SensorContract),),
                max_exchanges=budget.max_exchanges,
                quota_bytes=budget.quota_bytes,
            )
            self._bridge = Bridge(log, _json(config))
        except BaseException:
            self.close()
            raise

    def _active(self) -> None:
        _require(
            threading.get_ident() == self._owner, "experiment belongs to another thread"
        )
        _require(not self._retired, "experiment is retired")

    def _exchange(
        self, request: bytes, reader: BinaryIO, writer: BinaryIO, *, deadline: float
    ) -> bytes:
        response = self._journal.exchange(
            self._binding.endpoint_id, request, reader, writer, deadline=deadline
        )
        primary = _primary(request, response, self._binding)
        if primary is not None:
            _require(
                self._primary is None, "multiple primary operations in one command"
            )
            self._primary = primary
        return response

    def _call(self, method: str, payload: dict, invoke) -> object:
        self._active()
        before = self._journal.position()
        self._primary = None
        observed = None

        def callback(recorded: str) -> str:
            nonlocal observed
            observed = invoke(json.loads(recorded))
            after = self._journal.position()
            _require(
                after.exchange_pairs - before.exchange_pairs
                <= self._budget.max_call_exchanges,
                "call exchange budget",
            )
            return _json(_receipt(before, after, self._primary, observed))

        try:
            self._bridge.dispatch(method, _json(payload), callback)
        except BaseException:
            self.close()
            raise
        return observed

    def prepare(self) -> t.Prepared:
        self._active()
        _require(not self._prepared, "session is already prepared")

        def invoke(payload):
            decoded = SensorContract.decode_prepare(payload)
            _require(
                _same(c.raw(decoded), c.raw(self._plan)),
                "recorded preparation mismatch",
            )
            return self._session.prepare()

        result = self._call(METHODS[0], c.raw(self._plan), invoke)
        self._prepared = True
        return result

    def advance(self, target: t.SetTarget | None = None) -> t.BatchObservation:
        self._active()
        _require(self._prepared, "prepare the session before advancing")
        payload = {
            "tick": self._session.next_tick,
            "target": None if target is None else c.raw(target),
        }
        _target(payload, self._plan, self._session.next_tick)

        def invoke(recorded):
            selected = _target(recorded, self._plan, self._session.next_tick)
            with self._session.advance(selected) as batch:
                observed = batch.observation
            return observed

        return self._call(METHODS[1], payload, invoke)

    def finish(self) -> SensorRun:
        self._active()
        _require(self._prepared, "prepare the session before finishing")
        _require(
            self._session.validated_ticks == self._plan.planned_ticks,
            "complete every planned tick before finishing",
        )

        def invoke(recorded):
            row = w.closed(recorded, {"completed_ticks"})
            _require(
                type(row["completed_ticks"]) is int
                and row["completed_ticks"] == self._session.validated_ticks,
                "recorded finish mismatch",
            )
            return self._session.finish()

        session = self._call(
            METHODS[2], {"completed_ticks": self._session.validated_ticks}, invoke
        )
        try:
            capture = self._journal.finish()
            _require(
                capture.exchange_pairs == self._budget.max_exchanges,
                "complete exchange roster mismatch",
            )
            run_log = json.loads(self._bridge.finish(self._capture_name))
            return SensorRun(session, capture, run_log)
        finally:
            self.close()

    def close(self) -> None:
        _require(
            threading.get_ident() == self._owner, "experiment belongs to another thread"
        )
        self._retired = True
        self._session.close()
        if self._journal is not None:
            self._journal.close()
        if self._bridge is not None:
            self._bridge.close()

    def __enter__(self) -> SensorExperiment:
        self.prepare()
        return self

    def __exit__(self, error_type, error, traceback) -> None:
        self.close()


def _position(value: dict) -> Position:
    row = w.closed(
        value, {"records", "journal_bytes", "chain_digest", "exchange_pairs"}
    )
    _require(
        all(
            w.integer(row[key], 0)
            for key in ("records", "journal_bytes", "exchange_pairs")
        )
        and w.digest_valid(row["chain_digest"]),
        "invalid capture position",
    )
    return Position(**row)


class _Tape:
    """One bounded call's original frames. This object has no producer streams."""

    def __init__(self, binding: BufferBinding):
        self.binding = binding
        self.rows: deque[Exchange] = deque()
        self.primary = None

    def exchange(self, request: bytes, reader, writer, *, deadline: float) -> bytes:
        _require(bool(self.rows), "captured call is missing an exchange")
        row = self.rows.popleft()
        _require(
            request == row.request,
            "recorded control does not reconstruct the original NCP request",
        )
        primary = _primary(request, row.response, self.binding)
        if primary is not None:
            _require(self.primary is None, "extra captured primary operation")
            self.primary = primary
        return row.response


def _read_log(path: Path):
    events = []
    canonical = json.loads(
        inspect_runlog(path, lambda event: events.append(json.loads(event)))
    )
    _require(
        len(events) >= 13
        and events[0]["type"] == "run_started"
        and events[1]["type"] == "config_logged",
        "application run prefix",
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
        "application bridge configuration",
    )
    app = w.closed(
        config["application"],
        {"schema", "binding", "prepare", "descriptor_sha256", "capture"},
    )
    _require(
        app["schema"] == SCHEMA
        and app["descriptor_sha256"] == _sha(SensorContract.descriptor()),
        "installed application descriptor mismatch",
    )
    binding = BufferBinding(
        **w.closed(
            app["binding"],
            {
                "profile_digest",
                "application_digest",
                "run_id",
                "endpoint_id",
                "generation",
            },
        )
    )
    binding.validate()
    _require(
        config["run_id"] == binding.run_id == events[0]["run_id"],
        "application run identity mismatch",
    )
    prepare = SensorContract.decode_prepare(app["prepare"])
    budget = capture_budget(binding, prepare)
    capture = w.closed(
        app["capture"],
        {
            "name",
            "max_exchanges",
            "quota_bytes",
            "max_call_exchanges",
            "max_call_frame_bytes",
        },
    )
    _require(
        _same({key: capture[key] for key in c.raw(budget)}, c.raw(budget)),
        "capture budget mismatch",
    )
    count = prepare.planned_ticks + 2
    _require(
        type(config["max_calls"]) is int
        and config["max_calls"] == count
        and len(events) == 3 * count + 4,
        "complete application call roster",
    )
    calls = []
    for index in range(count):
        request, receipt, response = events[2 + 3 * index : 5 + 3 * index]
        method = (
            METHODS[0]
            if index == 0
            else METHODS[2]
            if index == count - 1
            else METHODS[1]
        )
        _require(
            [request["type"], receipt["type"], response["type"]]
            == ["bridge_request", "label_observed", "bridge_response"],
            "application event order",
        )
        _require(
            request["request_id"]
            == response["request_id"]
            == f"application-{index + 1}"
            and request["method"] == method
            and request["step"] == response["step"] == receipt["step"] == index + 1,
            "application request join",
        )
        _require(
            request["actor"]
            == {
                "actor_type": "script",
                "actor_id": config["actor_id"],
                "session_id": None,
            },
            "application actor mismatch",
        )
        _require(
            receipt["name"] == "prisoma.application_result.v1"
            and receipt["metadata"]
            == {"record_role": "execution_receipt", "is_outcome_label": "false"}
            and receipt["timestamp_ns"] == response["timestamp_ns"]
            and response["ok"] is True,
            "execution receipt role or response",
        )
        envelope = w.closed(receipt["value"], {"request_id", "method", "result"})
        _require(
            envelope["request_id"] == request["request_id"]
            and envelope["method"] == method
            and hash_object(_json(envelope["result"])) == response["result_hash"],
            "execution result hash join",
        )
        result = w.closed(envelope["result"], {"schema", "span", "ncp", "observation"})
        _require(result["schema"] == RECEIPT_SCHEMA, "execution receipt schema")
        span = w.closed(result["span"], {"before", "after"})
        before, after = _position(span["before"]), _position(span["after"])
        _require(
            0
            < after.exchange_pairs - before.exchange_pairs
            <= budget.max_call_exchanges,
            "execution span bound",
        )
        calls.append((method, request["payload"], result, before, after))
    artifact, ended = events[-2:]
    _require(
        artifact["type"] == "artifact_logged"
        and artifact["name"] == "application_capture"
        and artifact["kind"] == "application_supplied_bytes"
        and artifact["uri"] == capture["name"],
        "terminal capture artifact",
    )
    _require(
        ended["type"] == "run_ended" and ended["status"] == "succeeded",
        "application run did not complete",
    )
    identity = json.loads(artifact_identity(path, capture["name"]))
    _require(
        artifact["sha256"] == identity["sha256"]
        and artifact["metadata"] == {"bytes": str(identity["bytes"])},
        "capture artifact content mismatch",
    )
    return canonical, binding, prepare, budget, capture["name"], identity, calls


def verify_sensor_run(log_path: str | Path) -> dict:
    """Reconstruct commands and sensor bytes without starting or contacting a producer."""
    path = Path(log_path).absolute()
    canonical, binding, prepare, budget, capture_name, identity, calls = _read_log(path)
    tape = _Tape(binding)
    stream = BytesIO()
    session = SensorSession(
        stream,
        stream,
        binding,
        prepare,
        deadline=time.monotonic() + 300,
        exchange=tape.exchange,
    )
    pending: list[Exchange] = []
    frame_bytes, cursor = 0, 0
    completed = None

    def visit(row: Exchange):
        nonlocal cursor, frame_bytes, completed
        _require(cursor < len(calls), "unassigned NCP exchange")
        method, payload, recorded, before, after = calls[cursor]
        _require(row.peer.binding == binding, "captured peer mismatch")
        if not pending:
            _require(row.before == before, "execution span start mismatch")
        frame_bytes += len(row.request) + len(row.response)
        _require(
            len(pending) < budget.max_call_exchanges
            and frame_bytes <= budget.max_call_frame_bytes,
            "captured call exceeds bound",
        )
        pending.append(row)
        _require(row.after.records <= after.records, "execution span end mismatch")
        if row.after != after:
            return
        tape.rows.extend(pending)
        tape.primary = None
        if method == METHODS[0]:
            decoded = SensorContract.decode_prepare(payload)
            _require(
                _same(c.raw(decoded), c.raw(prepare)), "recorded preparation mismatch"
            )
            observed = session.prepare()
        elif method == METHODS[1]:
            target = _target(payload, prepare, session.next_tick)
            with session.advance(target) as batch:
                observed = batch.observation
        else:
            final = w.closed(payload, {"completed_ticks"})
            _require(
                type(final["completed_ticks"]) is int
                and final["completed_ticks"] == session.validated_ticks,
                "recorded finish mismatch",
            )
            observed = session.finish()
            completed = observed
        _require(not tape.rows, "captured call has unexplained exchanges")
        expected = _receipt(before, after, tape.primary, observed)
        _require(_same(expected, recorded), "reconstructed execution receipt mismatch")
        cursor += 1
        pending.clear()
        frame_bytes = 0

    try:
        capture = inspect(
            path.parent / capture_name, (Peer(binding, SensorContract),), visit
        )
        _require(
            cursor == len(calls) and not pending and completed is not None,
            "incomplete application reconstruction",
        )
        _require(
            capture.store_completion == "complete"
            and capture.exchange_pairs == budget.max_exchanges,
            "incomplete terminal capture",
        )
        _require(
            json.loads(artifact_identity(path, capture_name)) == identity,
            "capture identity changed",
        )
        return {
            "schema": "prisoma.crebain_sensor_run_verification.v1",
            "sensor_session_replayed": True,
            "planned_ticks": prepare.planned_ticks,
            "completed_ticks": completed.terminal.completed_ticks,
            "payload_count": completed.payload_count,
            "raw_bytes": completed.raw_bytes,
            "canonical": canonical,
            "capture": c.raw(capture),
            "artifact": identity,
            "scientific_validation": False,
            "producer_process_retirement_verified": False,
        }
    finally:
        session.close()
        stream.close()
