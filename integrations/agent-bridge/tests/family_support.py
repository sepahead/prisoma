"""Actual SDK/socket controls using CREBAIN's explicitly synthetic native fixture."""

from contextlib import contextmanager
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import uuid

from crebain_ncp_sensors import (
    codec as c,
    family_contract as fc,
    family_types as f,
    types as t,
)
from crebain_ncp_sensors.family import FamilySession


def plan(branches=2, *, ticks=6, landmark=3, image_side=8):
    workload = json.loads(
        (Path(__file__).parent / "fixtures/m1.workload.v1.json").read_bytes()
    )
    raw = workload["specification"]
    raw["scene"]["thermalCameras"] = []
    for camera in raw["scene"]["rgbCameras"]:
        camera.update(width=image_side, height=image_side, periodTicks=3)
    body = c.decode(
        "Prepare",
        {
            "specification": raw,
            "planned_ticks": ticks,
            "composition_digest": fc.COMPOSITION_DIGEST,
        },
    )
    target = t.SetTarget("set_target", True, 0.0, 0.0, 0.0, 8.0)
    return f.FamilyPlan(
        str(uuid.uuid4()),
        fc.new_binding(),
        body,
        landmark,
        tuple(
            f.BranchPlan(
                slot,
                f"case-{slot}",
                "label" if slot == 1 else "same_action_control",
                fc.new_binding(),
                target,
            )
            for slot in range(1, branches + 1)
        ),
        f.PressureWindow(
            "scaled_compensated_pressure_rms400_v1",
            "pressure:mic-a",
            ticks - 2,
            ticks,
            400,
            "pascal",
            fc.TARGET_DIGEST,
        ),
        f.FamilyLimits(120, branches + 1, 2, 1, 1, 3200),
    )


@contextmanager
def synthetic_owner(runtime, selected, *, exchanges):
    """Use real owned sockets and process exit, without a renderer or physics claim."""
    channels = [socket.socketpair() for _ in range(selected.limits.endpoint_count)]
    streams = [
        (host.makefile("rb", buffering=0), host.makefile("wb", buffering=0))
        for host, _ in channels
    ]
    ordinary = Path(os.environ["CREBAIN_SENSOR_BRIDGE"])
    command = [
        os.environ["CREBAIN_FAMILY_PRODUCER"],
        "--bun",
        os.environ["CREBAIN_SENSOR_BUN"],
        "--node",
        os.environ["CREBAIN_SENSOR_NODE"],
        "--bridge",
        str(ordinary.with_name("family-process.test-support.ts")),
        "--source-identity",
        "a" * 64,
        "--family-plan-json",
        json.dumps(c.raw(selected), separators=(",", ":")),
        "--service-fds-json",
        json.dumps([service.fileno() for _, service in channels]),
    ]
    process = session = None
    errors, forced = [], False
    with tempfile.TemporaryFile() as diagnostics:
        try:
            process = subprocess.Popen(
                command,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=diagnostics,
                pass_fds=tuple(service.fileno() for _, service in channels),
                close_fds=True,
            )
            for _, service in channels:
                service.close()
            session = FamilySession(
                selected,
                streams,
                deadline=time.monotonic() + selected.limits.total_wall_seconds,
                close_endpoint=lambda slot: channels[slot][0].shutdown(
                    socket.SHUT_RDWR
                ),
                source_identity="a" * 64,
                exchanges=exchanges,
            )
            session.prepare()
            yield session
            session.finish()
        except BaseException as error:
            errors.append(error)
        finally:
            if session is not None:
                try:
                    session.close()
                except BaseException as error:
                    errors.append(error)
            for host, service in channels:
                try:
                    host.shutdown(socket.SHUT_RDWR)
                except OSError:
                    pass
                host.close()
                service.close()
            for pair in streams:
                for stream in pair:
                    stream.close()
            if process is not None:
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    # This is only the test's direct, unreaped child.
                    forced = True
                    process.kill()
                    process.wait(timeout=5)
                    errors.append(
                        AssertionError("synthetic family child failed to retire")
                    )
                diagnostics.seek(0, 2)
                size = diagnostics.tell()
                diagnostics.seek(0)
                output = diagnostics.read(131072)
                if not errors and process.returncode != 0:
                    errors.append(AssertionError(output.decode(errors="replace")))
                if session is not None:
                    session.process_exit = {
                        "schema": "crebain.family-process-exit.v1",
                        "pid": process.pid,
                        "returncode": process.returncode,
                        "reason": "producer_exit",
                        "forced": forced,
                        "diagnostics_bytes": size,
                        "diagnostics_truncated": size > 131072,
                        "cleanup_confirmed": not forced and process.returncode == 0,
                        "endpoint_count": selected.limits.endpoint_count,
                    }
                    session.diagnostics, session.diagnostics_truncated = (
                        output,
                        size > 131072,
                    )
    if len(errors) == 1:
        raise errors[0]
    if errors:
        raise BaseExceptionGroup("synthetic family and cleanup failed", errors)


def complete(experiment, selected, forecast=b'{"scores":[1,2]}'):
    target = selected.branches[0].target
    observations = []
    for tick in range(1, selected.landmark_tick + 1):
        observations.append(experiment.advance(target if tick == 1 else None))
    experiment.checkpoint()
    experiment.commit_decision(forecast, selected.branches[0].case_id)
    for tick in range(selected.landmark_tick + 1, selected.body.planned_ticks + 1):
        observations.append(
            experiment.advance(target if tick == selected.landmark_tick + 1 else None)
        )
    for branch in selected.branches:
        experiment.reserve(branch.case_id)
        experiment.restore(branch.slot)
        for tick in range(selected.landmark_tick + 1, selected.body.planned_ticks + 1):
            observations.append(
                experiment.advance(
                    branch.target if tick == selected.landmark_tick + 1 else None,
                    slot=branch.slot,
                )
            )
        experiment.evaluate(branch.slot)
        experiment.branch_finish(branch.slot)
    experiment.release_checkpoint()
    return experiment.finish(), observations
