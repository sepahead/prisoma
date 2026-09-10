"""Application joins on bounded synthetic NCP peers, separate from native evidence."""

from contextlib import contextmanager
from dataclasses import FrozenInstanceError, replace
import hashlib
import json
from pathlib import Path
import socket
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from crebain_ncp_sensors import SessionError, codec as c
from ncp_local import wire as framing
from ncp_local import modular_owner as o
from prisoma_agent_bridge import hash_object
from prisoma_agent_bridge.crebain import (
    ExperimentError,
    SensorExperiment,
    capture_budget,
    inspect_sensor_run,
    verify_sensor_run,
)

from synthetic_sensors import SyntheticSensors, binding, plan, target


@contextmanager
def channel(application):
    host = o.Owner(binding(), application, tuple(sorted(c.SENSOR_DIGESTS.values())))
    first, second = socket.socketpair()
    stream = first.makefile("rwb", buffering=0)
    peer = second.makefile("rwb", buffering=0)
    errors = []

    def serve():
        try:
            deadline = time.monotonic() + 30
            while (
                request := framing.read_local_frame(peer, deadline=deadline)
            ) is not None:
                framing.write_local_frame(
                    peer, bytes(host.process(request)), deadline=deadline
                )
        except (BrokenPipeError, ConnectionResetError):
            pass
        except BaseException as error:
            errors.append(error)
        finally:
            peer.close()
            second.close()

    thread = threading.Thread(target=serve)
    thread.start()
    try:
        yield stream, host
    finally:
        try:
            first.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass
        stream.close()
        first.close()
        thread.join(5)
        if thread.is_alive():
            raise AssertionError("owned synthetic peer did not retire")
        if errors:
            raise errors[0]


def selected_plan(rgb=2, acoustic=1, thermal=0):
    initial = plan(6)
    scene = initial.specification.scene
    cameras = tuple(
        replace(
            scene.rgbCameras[0],
            id=f"camera-{i}",
            periodTicks=i + 2,
            position=(float(i), 9.0, 5.0),
        )
        for i in range(rgb)
    )
    microphones = tuple(
        replace(scene.microphones[0], id=f"microphone-{i}") for i in range(acoustic)
    )
    infrared = tuple(
        replace(scene.thermalCameras[0], id=f"thermal-{i}") for i in range(thermal)
    )
    scene = replace(
        scene, rgbCameras=cameras, microphones=microphones, thermalCameras=infrared
    )
    return replace(initial, specification=replace(initial.specification, scene=scene))


def write_events(path, events):
    path.write_text(
        "".join(
            json.dumps(row, allow_nan=False, separators=(",", ":")) + "\n"
            for row in events
        )
    )


class SensorBridgeTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.log = Path(self.directory.name) / "run.jsonl"
        self.capture = self.log.with_name("capture.ncp")

    def execute(self, prepare=None):
        prepare = prepare or selected_plan()
        application = SyntheticSensors()
        observations = []
        with channel(application) as (stream, host):
            with SensorExperiment(
                self.log,
                self.capture,
                stream,
                stream,
                binding(),
                prepare,
                deadline=time.monotonic() + 30,
            ) as experiment:
                for tick in range(1, prepare.planned_ticks + 1):
                    observation = experiment.advance(target() if tick == 1 else None)
                    self.assertEqual(host.usage.live_slots, 0)
                    observations.append(observation)
                result = experiment.finish()
        self.assertEqual(application.advances, prepare.planned_ticks)
        self.assertTrue(application.finished)
        verified = verify_sensor_run(self.log)
        self.assertTrue(verified["sensor_session_replayed"])
        self.assertFalse(verified["scientific_validation"])
        self.assertFalse(verified["producer_process_retirement_verified"])
        self.assertEqual(
            verified["capture"]["exchange_pairs"],
            capture_budget(binding(), prepare).max_exchanges,
        )
        self.assertEqual(
            (verified["raw_bytes"], verified["payload_count"]),
            (result.session.raw_bytes, result.session.payload_count),
        )
        return observations, verified

    def test_two_rgb_sources_have_distinct_ids_cadences_and_receipts(self):
        observations, verified = self.execute()
        seen = {}
        for row in observations:
            for reading in row.readings:
                seen.setdefault(reading.manifest.sensor_id, []).append(
                    row.batch.body_tick
                )
        self.assertEqual(
            seen,
            {
                "pressure:microphone-0": [1, 2, 3, 4, 5, 6],
                "rgb:camera-0": [2, 4, 6],
                "rgb:camera-1": [3, 6],
            },
        )
        self.assertEqual(
            (verified["completed_ticks"], verified["payload_count"]), (6, 11)
        )
        self.assertEqual(verified["canonical"]["events"], 28)
        rgb = [
            row
            for row in observations[-1].readings
            if row.manifest.tensor.kind == "rgba8"
        ]
        self.assertEqual(len(rgb), 2)
        self.assertEqual(rgb[0].payload, rgb[1].payload)
        self.assertNotEqual(rgb[0].manifest.sensor_id, rgb[1].manifest.sensor_id)

    def test_a_camera_only_session_needs_no_acoustic_or_thermal_source(self):
        observations, verified = self.execute(selected_plan(rgb=1, acoustic=0))
        self.assertEqual(
            [row.batch.body_tick for row in observations if not row.readings], [1, 3, 5]
        )
        self.assertEqual(verified["payload_count"], 3)

    def test_multiple_microphones_and_optional_thermal_remain_separate(self):
        observations, verified = self.execute(
            selected_plan(rgb=1, acoustic=2, thermal=1)
        )
        for row in observations:
            self.assertIn(
                "pressure:microphone-0",
                {value.manifest.sensor_id for value in row.readings},
            )
            self.assertIn(
                "pressure:microphone-1",
                {value.manifest.sensor_id for value in row.readings},
            )
        self.assertEqual(verified["payload_count"], 17)

    def test_invalid_input_and_existing_capture_dispatch_nothing(self):
        self.capture.write_bytes(b"retain")
        application = SyntheticSensors()
        with channel(application) as (stream, _):
            with self.assertRaises(FileExistsError):
                SensorExperiment(
                    self.log,
                    self.capture,
                    stream,
                    stream,
                    binding(),
                    selected_plan(),
                    deadline=time.monotonic() + 30,
                )
        self.assertEqual(application.advances, 0)
        self.assertIsNone(application.prepare)
        self.assertFalse(self.log.exists())
        self.assertEqual(self.capture.read_bytes(), b"retain")

    def test_invalid_target_and_early_finish_leave_a_prepared_session_usable(self):
        application = SyntheticSensors()
        with channel(application) as (stream, _):
            with SensorExperiment(
                self.log,
                self.capture,
                stream,
                stream,
                binding(),
                selected_plan(),
                deadline=time.monotonic() + 30,
            ) as experiment:
                with self.assertRaises(ValueError):
                    experiment.advance(replace(target(), altitude_m=9.0))
                with self.assertRaises(ExperimentError):
                    experiment.finish()
                self.assertEqual(application.advances, 0)
                for tick in range(6):
                    experiment.advance(target() if tick == 0 else None)
                experiment.finish()
        verify_sensor_run(self.log)

    def test_capture_failure_after_a_body_step_prevents_observation_and_next_step(self):
        application = SyntheticSensors()
        with channel(application) as (stream, _):
            with SensorExperiment(
                self.log,
                self.capture,
                stream,
                stream,
                binding(),
                selected_plan(),
                deadline=time.monotonic() + 30,
            ) as experiment:
                original = experiment._journal.exchange

                def lose_reply(*args, **kwargs):
                    original(*args, **kwargs)
                    raise OSError("synthetic consumer failure after captured dispatch")

                with patch.object(
                    experiment._journal, "exchange", side_effect=lose_reply
                ):
                    with self.assertRaises(SessionError) as stopped:
                        experiment.advance(target())
                    self.assertIsInstance(stopped.exception.__cause__, OSError)
                self.assertEqual(application.advances, 1)
                with self.assertRaises(ExperimentError):
                    experiment.advance(target())
                self.assertEqual(application.advances, 1)
        with self.assertRaises(RuntimeError):
            verify_sensor_run(self.log)

    def test_changed_control_with_a_valid_canonical_hash_fails_ncp_reconstruction(self):
        self.execute()
        events = [json.loads(line) for line in self.log.read_text().splitlines()]
        request = events[5]
        request["payload"]["target"]["pitch_rad"] = 0.01
        request["payload_hash"] = hash_object(json.dumps(request["payload"]))
        write_events(self.log, events)
        with self.assertRaises(SessionError) as stopped:
            verify_sensor_run(self.log)
        self.assertEqual(stopped.exception.stage, "advance")
        self.assertIsInstance(stopped.exception.__cause__, ExperimentError)
        self.assertIn("original NCP request", str(stopped.exception.__cause__))

    def test_changed_execution_receipt_or_span_fails_even_after_response_rehash(self):
        self.execute()
        original = self.log.read_text()
        for field in ("observation", "span"):
            with self.subTest(field=field):
                events = [json.loads(line) for line in original.splitlines()]
                result = events[6]["value"]["result"]
                if field == "observation":
                    result["observation"]["readings"][0]["bytes"] += 8
                else:
                    result["span"]["after"]["chain_digest"] = "0" * 64
                events[7]["result_hash"] = hash_object(json.dumps(result))
                write_events(self.log, events)
                with self.assertRaisesRegex(ValueError, "receipt|span"):
                    verify_sensor_run(self.log)
        self.log.write_text(original)
        verify_sensor_run(self.log)

    def test_changed_capture_rejects_unchanged_and_forged_outer_artifact_hashes(self):
        self.execute()
        original = self.capture.read_bytes()
        changed = bytearray(original)
        changed[-1] ^= 1
        self.capture.write_bytes(changed)
        with self.assertRaisesRegex(ExperimentError, "artifact"):
            verify_sensor_run(self.log)
        events = [json.loads(line) for line in self.log.read_text().splitlines()]
        events[-2]["sha256"] = hashlib.sha256(changed).hexdigest()
        write_events(self.log, events)
        with self.assertRaisesRegex(ValueError, "record_digest"):
            verify_sensor_run(self.log)

    def test_inspection_returns_exact_observations_with_targets_and_capture_joins(self):
        observations, verified = self.execute()
        steps = []
        with (
            patch(
                "socket.socket", side_effect=AssertionError("replay opened a socket")
            ),
            patch(
                "subprocess.Popen", side_effect=AssertionError("replay started a child")
            ),
        ):
            inspected = inspect_sensor_run(self.log, steps.append)
        self.assertEqual(inspected, verified)
        self.assertEqual(repr([step.observation for step in steps]), repr(observations))
        self.assertEqual(
            [step.observation.batch.body_tick for step in steps], list(range(1, 7))
        )
        self.assertEqual(steps[0].requested_target, target())
        self.assertTrue(all(step.requested_target is None for step in steps[1:]))
        for step in steps:
            self.assertEqual(step.binding, binding())
            self.assertEqual(step.prepare, selected_plan())
            self.assertEqual(
                step.prepared.source_identity, step.observation.batch.source_identity
            )
            self.assertLess(step.capture_before.records, step.capture_after.records)
            self.assertTrue(
                all(type(row.payload) is bytes for row in step.observation.readings)
            )
        for left, right in zip(steps, steps[1:]):
            self.assertEqual(left.capture_after, right.capture_before)
        with self.assertRaises(FrozenInstanceError):
            steps[0].requested_target = None
        with self.assertRaises(FrozenInstanceError):
            steps[0].observation.batch.body_tick = 7

    def test_inspection_preserves_equal_byte_cameras_and_not_due_slots(self):
        self.execute()
        steps = []
        inspect_sensor_run(self.log, steps.append)
        final_rgb = [
            row
            for row in steps[-1].observation.readings
            if row.manifest.tensor.kind == "rgba8"
        ]
        self.assertEqual(final_rgb[0].payload, final_rgb[1].payload)
        self.assertNotEqual(
            final_rgb[0].manifest.sensor_id, final_rgb[1].manifest.sensor_id
        )
        self.assertEqual(
            [
                [
                    slot.sensor_id
                    for slot in step.observation.batch.slots
                    if slot.kind == "not_due"
                ]
                for step in steps
            ],
            [
                ["rgb:camera-0", "rgb:camera-1"],
                ["rgb:camera-1"],
                ["rgb:camera-0"],
                ["rgb:camera-1"],
                ["rgb:camera-0", "rgb:camera-1"],
                [],
            ],
        )

    def test_inspection_keeps_camera_only_empty_batches(self):
        observations, _ = self.execute(selected_plan(rgb=1, acoustic=0))
        steps = []
        inspect_sensor_run(self.log, steps.append)
        self.assertEqual([step.observation for step in steps], observations)
        self.assertEqual(
            [
                step.observation.batch.body_tick
                for step in steps
                if not step.observation.readings
            ],
            [1, 3, 5],
        )

    def test_inspection_keeps_multiple_microphones_and_thermal(self):
        observations, _ = self.execute(selected_plan(rgb=1, acoustic=2, thermal=1))
        steps = []
        inspect_sensor_run(self.log, steps.append)
        self.assertEqual(repr([step.observation for step in steps]), repr(observations))
        self.assertEqual(sum(len(step.observation.readings) for step in steps), 17)

    def test_inspection_requires_a_callable_before_reading(self):
        with patch("prisoma_agent_bridge.crebain._read_log") as read:
            with self.assertRaisesRegex(ExperimentError, "visitor must be callable"):
                inspect_sensor_run(self.log, None)
            read.assert_not_called()

    def test_inspection_callback_exception_propagates_and_stops_visits(self):
        _, verified = self.execute()
        failure = RuntimeError("derived output failed")
        visited = []

        def stop(step):
            visited.append(step.observation.batch.body_tick)
            raise failure

        with self.assertRaises(RuntimeError) as caught:
            inspect_sensor_run(self.log, stop)
        self.assertIs(caught.exception, failure)
        self.assertEqual(visited, [1])
        self.assertEqual(inspect_sensor_run(self.log, lambda _: None), verified)

    def test_inspection_rejects_a_rehashed_false_receipt_before_visiting_that_step(
        self,
    ):
        self.execute()
        events = [json.loads(line) for line in self.log.read_text().splitlines()]
        result = events[6]["value"]["result"]
        result["observation"]["readings"][0]["bytes"] += 8
        events[7]["result_hash"] = hash_object(json.dumps(result))
        write_events(self.log, events)
        visited = []
        with self.assertRaisesRegex(
            ExperimentError, "reconstructed execution receipt mismatch"
        ):
            inspect_sensor_run(self.log, visited.append)
        self.assertEqual(visited, [])

    def test_inspection_visits_remain_provisional_when_terminal_capture_is_corrupt(
        self,
    ):
        self.execute()
        original = self.capture.read_bytes()
        changed = bytearray(original)
        changed[-1] ^= 1
        self.capture.write_bytes(changed)
        events = [json.loads(line) for line in self.log.read_text().splitlines()]
        events[-2]["sha256"] = hashlib.sha256(changed).hexdigest()
        write_events(self.log, events)
        visited = []
        with self.assertRaisesRegex(ValueError, "record_digest"):
            inspect_sensor_run(self.log, visited.append)
        self.assertEqual(len(visited), 6)

    def test_inspection_rejects_canonical_mutation_during_a_callback(self):
        self.execute()
        original = self.log.read_bytes()

        def change(step):
            if step.observation.batch.body_tick == 1:
                self.log.write_bytes(original + b"\n")

        with self.assertRaisesRegex(ExperimentError, "canonical log changed"):
            inspect_sensor_run(self.log, change)
        self.log.write_bytes(original)
        self.assertTrue(
            inspect_sensor_run(self.log, lambda _: None)["sensor_session_replayed"]
        )

    def test_inspection_rejects_byte_identical_canonical_replacement(self):
        self.execute()
        original = self.log.read_bytes()

        def replace_file(step):
            if step.observation.batch.body_tick == 1:
                replacement = self.log.with_name("replacement.jsonl")
                replacement.write_bytes(original)
                replacement.chmod(0o600)
                replacement.replace(self.log)

        with self.assertRaisesRegex(ExperimentError, "canonical log changed"):
            inspect_sensor_run(self.log, replace_file)
        self.assertEqual(self.log.read_bytes(), original)
        self.assertTrue(
            inspect_sensor_run(self.log, lambda _: None)["sensor_session_replayed"]
        )


if __name__ == "__main__":
    unittest.main()
