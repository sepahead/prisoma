"""Run one predeclared M1 timing case through unchanged installed Python owners."""

from __future__ import annotations
import argparse
from contextlib import ExitStack
import importlib.util
import json
import os
from pathlib import Path
import struct
import sys

MAX_RETAINED_FRAME_BYTES = 16 * 1024 * 1024
MAX_EXPORT_BYTES = 48 * 1024 * 1024


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    sys.modules[name] = result
    spec.loader.exec_module(result)
    return result


m = module("perf_common", Path(__file__).with_name("perf_common.py"))


class FrameProbe:
    """Retain immutable original frame objects; no request or result is rewritten."""

    def __init__(
        self, trace, maximum, detailed, *, byte_limit=MAX_RETAINED_FRAME_BYTES
    ):
        self.trace, self.maximum, self.detailed = trace, maximum, detailed
        m.require(
            m.exact_int(byte_limit, 65537, MAX_RETAINED_FRAME_BYTES),
            "frame byte capacity",
        )
        self.byte_limit, self.retained_bytes = byte_limit, 0
        self.frames = []
        self.originals = None

    def install(self):
        from ncp_local import wire

        self.originals = wire.write_local_frame, wire.read_local_frame
        original_write, original_read = self.originals

        def write(writer, payload, *, deadline=None):
            m.require(len(self.frames) < self.maximum, "frame probe capacity")
            # Reserve a complete legal response before the request can have effects.
            size = len(payload) if type(payload) is bytes else 0
            m.require(
                self.retained_bytes + size + 65536 <= self.byte_limit,
                "frame byte capacity exhausted before invocation",
            )
            self.frames.append(("request", self.trace.tick, payload))
            self.retained_bytes += size
            if self.detailed:
                with self.trace.span("frame.write_inclusive"):
                    return original_write(writer, payload, deadline=deadline)
            return original_write(writer, payload, deadline=deadline)

        def read(reader, *, deadline=None):
            m.require(len(self.frames) < self.maximum, "frame probe capacity")
            m.require(
                self.retained_bytes + 65536 <= self.byte_limit,
                "frame byte capacity exhausted before read",
            )
            if self.detailed:
                with self.trace.span("frame.read_inclusive_remote_service"):
                    value = original_read(reader, deadline=deadline)
            else:
                value = original_read(reader, deadline=deadline)
            self.frames.append(("response", self.trace.tick, value))
            if value is not None:
                m.require(
                    type(value) is bytes and len(value) <= 65536,
                    "installed framer result extent",
                )
                self.retained_bytes += len(value)
            return value

        wire.write_local_frame, wire.read_local_frame = write, read

    def restore(self):
        from ncp_local import wire

        if self.originals:
            wire.write_local_frame, wire.read_local_frame = self.originals

    def export(self, output):
        rows, byte_count = [], 0
        tail = []
        with output.open("wire.bin") as stream:
            for i in range(0, len(self.frames), 2):
                request = self.frames[i]
                response = self.frames[i + 1] if i + 1 < len(self.frames) else None
                if response is None or response[2] is None:
                    if type(request[2]) is bytes:
                        tail.append(
                            {
                                "role": "request_input_transmission_unproven",
                                "tick": request[1],
                                "artifact": output.write(
                                    "unpaired-request.bin", request[2]
                                ),
                            }
                        )
                    break
                m.require(
                    request[0] == "request"
                    and response[0] == "response"
                    and request[1] == response[1],
                    "actual frame pairing",
                )
                q, r = request[2], response[2]
                m.require(
                    type(q) is bytes
                    and type(r) is bytes
                    and 0 < len(q) <= 65536
                    and 0 < len(r) <= 65536,
                    "actual frame shape",
                )
                output.append(stream, struct.pack(">II", len(q), len(r)) + q + r)
                rows.append(
                    {
                        "exchange": len(rows),
                        "tick": request[1],
                        "request_bytes": len(q),
                        "response_bytes": len(r),
                    }
                )
                byte_count += len(q) + len(r) + 8
        return {
            "exchanges": len(rows),
            "framed_bytes": byte_count,
            "rows": rows,
            "observed_frame_calls": len(self.frames),
            "unpaired": tail,
            "retained_raw_frame_bytes": self.retained_bytes,
            "retained_raw_frame_byte_limit": self.byte_limit,
        }


def study_output(c, path):
    class StudyOutput(c.Output):
        def append(self, stream, raw):
            m.require(
                type(raw) is bytes and self.total + len(raw) <= MAX_EXPORT_BYTES,
                "study export byte capacity",
            )
            super().append(stream, raw)

    return StudyOutput(path)


def instrument(trace, detailed, canonical):
    restores = []
    if not detailed:
        return restores
    from ncp_local.modular_client import Client
    from crebain_ncp_sensors import codec

    targets = [
        (Client, name, "sdk." + name)
        for name in ("begin", "observe", "acknowledgement", "observe_acknowledgement")
    ]
    targets += [(codec, "validate_batch", "application.validate_batch")]
    targets += [(os, "fsync", "python.fsync")]
    if canonical:
        from prisoma_ncp_transcript.transcript import Journal

        targets += [
            (Journal, "exchange", "capture.exchange_inclusive"),
            (Journal, "position", "capture.position"),
            (Journal, "finish", "capture.finish"),
        ]
    for obj, name, label in targets:
        original = getattr(obj, name)
        setattr(obj, name, trace.wrapper(label, original))
        restores.append((obj, name, original))
    return restores


def export_payloads(c, output, stream, observation, counter):
    payloads = []
    for reading in observation.readings:
        identity = output.write(f"payload-{counter:05d}.bin", reading.payload)
        counter += 1
        payloads.append({**identity, "sensor_id": reading.manifest.sensor_id})
    output.append(
        stream,
        c.canonical({"tick": observation.batch.body_tick, "payloads": payloads})
        + b"\n",
    )
    os.fsync(stream.fileno())
    return counter


def main(freeze_path, case_id):
    m.require(sys.flags.isolated and sys.dont_write_bytecode, "isolated interpreter")
    raw, freeze = m.read_json(Path(freeze_path))
    m.require(freeze["schema"] == "local.m1-performance-freeze.v1", "freeze schema")
    cases = [x for x in freeze["cases"] if x["case_id"] == case_id]
    m.require(len(cases) == 1, "exact case")
    case = cases[0]
    m.require(case["route"] in ("body", "canonical"), "python route")
    for row in freeze["tools"].values():
        m.selected_file(row)
    m.require(
        m.identity(Path(__file__).resolve()) == freeze["tools"]["python_worker"],
        "worker identity",
    )
    c = module("perf_m1_helpers", Path(freeze["tools"]["m1_helpers"]["path"]))
    from crebain_ncp_sensors import codec, body_session, new_binding
    from crebain_ncp_sensors.runtime import InstalledRuntime

    m.require(type(case["run_id"]) is str, "selected run identity")
    # Pure preflight leaves actual binding creation in the measured preparation.
    new_binding(run_id=case["run_id"])
    environment = freeze["environments"][case["route"]]
    m.require(str(Path(sys.prefix).resolve()) == environment["prefix"], "environment")
    selected_inventory = m.parse_json(m.selected_file(environment["inventory"]))
    m.require(c.inventory() == selected_inventory, "installed inventory drift")
    for name, wheel in environment["wheels"].items():
        c.wheel_installation(name, wheel, selected_inventory)
    prepare, targets, expected = c.workload(freeze["workload"])
    clock = m.Clock()
    trace = m.Trace(clock)
    output = study_output(c, Path(freeze["output_root"]) / case_id)
    output.json(
        "case.json",
        {
            "freeze_sha256": c.digest(raw),
            "case": case,
            "clock_domain": "process-local-perf_counter_ns",
            "clock_id": case["clock_id"],
            "environment": environment,
            "runtime": freeze["runtime"],
        },
    )
    rows = []
    observations = []
    session = completed = primary = None
    origin = None
    prepare_start = clock.now()
    prepare_end = finish_start = finish_end = retired = None
    payload_count = 0
    observed_ticks = released_ticks = 0
    incomplete_tick = None
    wire = None
    detailed = case["instrumentation"] == "detailed"
    canonical = case["route"] == "canonical"
    probe = FrameProbe(trace, expected["exchanges"] * 2, detailed)
    restores = []
    try:
        with trace.span("runtime_verification"):
            runtime = InstalledRuntime.open(freeze["runtime"]["prefix"])
            m.require(
                runtime.manifest_sha256 == freeze["runtime"]["manifest_sha256"]
                and runtime.source_identity == freeze["runtime"]["source_identity"],
                "runtime identity",
            )
        binding = new_binding(run_id=case["run_id"])
        # The binding is fresh; its complete identity is durable before owner admission.
        output.json("binding.json", codec.raw(binding))
        if canonical:
            from prisoma_agent_bridge.crebain import owned_sensor_experiment

            owner = owned_sensor_experiment(
                runtime,
                prepare,
                output.path / "run.jsonl",
                output.path / "capture.ncp",
                binding=binding,
                timeout_s=freeze["limits"]["session_seconds"],
            )
        else:
            owner = body_session(
                runtime,
                prepare,
                binding=binding,
                timeout_s=freeze["limits"]["session_seconds"],
            )
        probe.install()
        restores = instrument(trace, detailed, canonical)
        with output.open("steps.jsonl") as steps:
            with ExitStack() as stack:
                with trace.span("owner.prepare_inclusive"):
                    session = stack.enter_context(owner)
                prepare_end = clock.now()
                origin = prepare_end
                for tick in range(1, prepare.planned_ticks + 1):
                    trace.tick = tick
                    offer, deadline = m.schedule_ns(origin, tick)
                    clock.wait_until(offer)
                    start = clock.now()
                    incomplete_tick = {
                        "tick": tick,
                        "offer_ns": offer,
                        "deadline_numerator_ns_x120": deadline,
                        "start_ns": start,
                    }
                    with trace.span("route.advance_inclusive"):
                        if canonical:
                            observation = session.advance(targets.get(tick))
                            observation_ns = clock.now()
                            observed_ticks = observation.batch.body_tick
                            released_ticks = observed_ticks
                        else:
                            pending = session.advance(targets.get(tick))
                            observation = pending.observation
                            observation_ns = clock.now()
                            observed_ticks = observation.batch.body_tick
                            with trace.span("route.release"):
                                pending.release()
                            released_ticks = observed_ticks
                    route_complete = clock.now()
                    incomplete_tick.update(
                        observation_ns=observation_ns, route_complete_ns=route_complete
                    )
                    with trace.span("benchmark.hash_and_durable_export"):
                        payload_count = export_payloads(
                            c, output, steps, observation, payload_count
                        )
                    exported = clock.now()
                    rows.append(
                        {
                            "tick": tick,
                            "clock_id": case["clock_id"],
                            "offer_ns": offer,
                            "deadline_numerator_ns_x120": deadline,
                            "start_ns": start,
                            "observation_ns": observation_ns,
                            "route_complete_ns": route_complete,
                            "export_complete_ns": exported,
                            "route_missed": m.missed_deadline(route_complete, deadline),
                            "export_missed": m.missed_deadline(exported, deadline),
                        }
                    )
                    incomplete_tick = None
                    observations.append(
                        {
                            "tick": tick,
                            "batch": codec.raw(observation.batch),
                            "readings": [
                                {
                                    "manifest": codec.raw(x.manifest),
                                    "byte_manifest": codec.raw(x.byte_manifest),
                                }
                                for x in observation.readings
                            ],
                        }
                    )
                trace.tick = prepare.planned_ticks + 1
                finish_start = clock.now()
                with trace.span("route.finish_and_selected_finalization"):
                    completed = session.finish()
                finish_end = clock.now()
            retired = clock.now()
        trace.validate()
        m.validate_tick_rows(
            rows,
            clock_id=case["clock_id"],
            origin_ns=origin,
            planned_ticks=prepare.planned_ticks,
            completed_ticks=len(rows),
        )
        m.require(len(probe.frames) == 2 * expected["exchanges"], "exchange roster")
        m.require(payload_count == expected["payloads"], "payload roster")
        process_exit = session.process_exit
        m.require(
            type(process_exit) is dict
            and process_exit.get("returncode") == 0
            and process_exit.get("forced") is False
            and process_exit.get("cleanup_confirmed") is True,
            "owned process exit",
        )
        if canonical:
            from prisoma_agent_bridge.crebain import verify_sensor_run

            verification = verify_sensor_run(output.path / "run.jsonl")
            output.json("canonical-verification.json", verification)
    except BaseException as error:
        primary = error
    finally:
        for obj, name, original in reversed(restores):
            setattr(obj, name, original)
        probe.restore()

    try:
        wire = probe.export(output)
        output.json("observations.json", observations)
    except BaseException as error:
        primary = (
            error
            if primary is None
            else BaseExceptionGroup(
                "operation and frame export failed", [primary, error]
            )
        )

    # Readbacks and result synchronization do not masquerade as operation timings.
    result = {
        "schema": "local.m1-performance-case.v1",
        "case": case,
        "freeze_sha256": c.digest(raw),
        "status": "complete" if primary is None else "failed",
        "scope": "coarse instrumented local timing; no real-time or release qualification",
        "clock_id": case["clock_id"],
        "clock_domain": "process-local-perf_counter_ns",
        "planned_ticks": prepare.planned_ticks,
        "completed_ticks": len(rows),
        "completed_ticks_meaning": "complete route and benchmark-export measurements",
        "last_observed_tick": observed_ticks,
        "last_released_tick": released_ticks,
        "uncompleted_measurement_ticks": list(
            range(len(rows) + 1, prepare.planned_ticks + 1)
        ),
        "incomplete_tick": incomplete_tick,
        "prepare_start_ns": prepare_start,
        "prepare_end_ns": prepare_end,
        "preparation_scope": "InstalledRuntime.open complete verification, fresh binding publication, and installed Python owner admission",
        "offered_origin_ns": origin,
        "finish_start_ns": finish_start,
        "finish_end_ns": finish_end,
        "retired_ns": retired,
        "ticks": rows,
        "spans": trace.rows,
        "span_overflow": trace.overflowed,
        "wire": wire,
        "payload_count": payload_count,
        "process_exit": None if session is None else session.process_exit,
        "terminal": None
        if completed is None
        else codec.raw(completed.session if canonical else completed),
        "failure": None if primary is None else c.diagnostic(primary),
    }
    try:
        if session is not None:
            output.write("diagnostics.bin", session.diagnostics)
        output.json("result.json", result)
    except BaseException as error:
        if primary is not None:
            raise BaseExceptionGroup(
                "operation and terminal publication failed", [primary, error]
            )
        raise
    if primary is not None:
        raise primary
    print(
        json.dumps(
            {
                "case": case_id,
                "status": "complete",
                "ticks": len(rows),
                "route_deadline_misses": sum(x["route_missed"] for x in rows),
                "export_deadline_misses": sum(x["export_missed"] for x in rows),
            }
        )
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--freeze", required=True)
    parser.add_argument("--case", required=True)
    args = parser.parse_args()
    main(args.freeze, args.case)
