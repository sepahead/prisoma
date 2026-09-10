"""Installed native adapter controls. No simulator or scientific result is inferred."""

import hashlib
import json
import os
from pathlib import Path
import tempfile
import threading
import unittest

from prisoma_agent_bridge import Bridge, artifact_identity, inspect_runlog


def encoded(value):
    return json.dumps(value, allow_nan=False, separators=(",", ":"))


def configuration(**changes):
    value = {
        "schema": "prisoma.application_bridge.v1",
        "run_id": "adapter-control",
        "actor_id": "installed-control",
        "methods": ["test.prepare", "test.advance"],
        "max_calls": 3,
        "application": {"schema": "test.application.v1"},
    }
    value.update(changes)
    return encoded(value)


class BridgeTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "run.jsonl"

    def bridge(self, **changes):
        value = Bridge(self.path, configuration(**changes))
        self.addCleanup(value.close)
        return value

    def read(self):
        events = []
        summary = json.loads(
            inspect_runlog(self.path, lambda value: events.append(json.loads(value)))
        )
        self.assertFalse(summary["application_completion_validated"])
        self.assertFalse(summary["scientific_validation"])
        self.assertEqual(summary["events"], len(events))
        return events

    def test_recorded_request_precedes_callback_and_receipt_precedes_response(self):
        bridge = self.bridge()
        original = {"camera_ids": ["rgb:left", "rgb:right"], "tick": 1}

        def callback(payload):
            self.assertEqual(json.loads(payload), original)
            prefix = [json.loads(line) for line in self.path.read_text().splitlines()]
            self.assertEqual(prefix[-1]["type"], "bridge_request")
            self.assertEqual(prefix[-1]["payload"], original)
            return '{"body_tick":1,"observed":true}'

        response = json.loads(
            bridge.dispatch("test.advance", encoded(original), callback)
        )
        self.assertTrue(response["ok"])
        result = json.loads(bridge.finish())
        events = self.read()
        self.assertEqual(
            [row["type"] for row in events],
            [
                "run_started",
                "config_logged",
                "bridge_request",
                "label_observed",
                "bridge_response",
                "run_ended",
            ],
        )
        self.assertEqual(events[3]["value"]["result"], response["result"])
        self.assertEqual(
            events[3]["metadata"],
            {"record_role": "execution_receipt", "is_outcome_label": "false"},
        )
        self.assertEqual(events[3]["timestamp_ns"], events[4]["timestamp_ns"])
        self.assertGreaterEqual(events[4]["timestamp_ns"], events[2]["timestamp_ns"])
        self.assertEqual(events[1]["config"]["clock"]["unit"], "ns")
        self.assertFalse(events[1]["config"]["clock"]["simulation_time"])
        self.assertEqual(
            (result["calls"], result["events"], result["bytes"]),
            (1, 6, self.path.stat().st_size),
        )
        with self.assertRaises(RuntimeError):
            bridge.dispatch("test.advance", "{}", callback)

    def test_input_rejections_do_not_execute_or_retire_a_healthy_bridge(self):
        bridge = self.bridge(max_calls=1)
        calls = []

        def callback(payload):
            calls.append(payload)
            return "{}"

        for method, payload, handler in (
            ("sim.step", "{}", callback),
            ("test.advance", '{"x":1,"x":2}', callback),
            ("test.advance", "[]", callback),
            ("test.advance", '{"x":NaN}', callback),
            ("test.advance", encoded({"x": "a" * 65_536}), callback),
            ("test.advance", "{}", None),
        ):
            with self.subTest(method=method, payload_bytes=len(payload)):
                with self.assertRaises(ValueError):
                    bridge.dispatch(method, payload, handler)
                self.assertEqual(calls, [])
        bridge.dispatch("test.advance", "{}", callback)
        with self.assertRaises(ValueError):
            bridge.dispatch("test.advance", "{}", callback)
        self.assertEqual(len(calls), 1)
        bridge.finish()
        self.read()

    def test_configuration_rejections_do_not_create_a_log(self):
        for changes in (
            {"schema": "unknown"},
            {"run_id": ""},
            {"actor_id": "with space"},
            {"max_calls": 0},
            {"max_calls": 1025},
            {"max_calls": True},
            {"methods": []},
            {"methods": ["same", "same"]},
            {"methods": [f"m{i}" for i in range(17)]},
            {"methods": ["x" * 65]},
            {"application": []},
            {"clock": "caller-selected"},
        ):
            with self.subTest(changes=changes):
                with self.assertRaises(ValueError):
                    Bridge(self.path, configuration(**changes))
                self.assertFalse(self.path.exists())

    def test_callback_exception_is_preserved_and_prevents_further_dispatch(self):
        bridge = self.bridge()
        marker = ValueError("retained callback failure")
        calls = []

        def fail(_):
            calls.append(1)
            raise marker

        with self.assertRaises(ValueError) as observed:
            bridge.dispatch("test.advance", "{}", fail)
        self.assertIs(observed.exception, marker)
        with self.assertRaises(RuntimeError):
            bridge.dispatch("test.advance", "{}", fail)
        self.assertEqual(calls, [1])
        bridge.close()
        prefix = [json.loads(line) for line in self.path.read_text().splitlines()]
        self.assertEqual(prefix[-1]["type"], "bridge_response")
        self.assertFalse(prefix[-1]["ok"])
        with self.assertRaises(RuntimeError):
            self.read()

    def test_malformed_callback_result_retires_after_one_effect(self):
        for index, result in enumerate(
            (None, [], "[]", '{"a":1,"a":2}', "{} " * 30_000)
        ):
            with self.subTest(index=index):
                path = self.path.with_name(f"invalid-{index}.jsonl")
                bridge = Bridge(path, configuration())
                self.addCleanup(bridge.close)
                calls = []

                def callback(_):
                    calls.append(1)
                    return result

                with self.assertRaises((RuntimeError, TypeError)):
                    bridge.dispatch("test.advance", "{}", callback)
                with self.assertRaises(RuntimeError):
                    bridge.dispatch("test.advance", "{}", callback)
                self.assertEqual(calls, [1])

    def test_foreign_thread_rejection_preserves_the_owner_session(self):
        bridge = self.bridge()
        failures, calls = [], []

        def attempt():
            try:
                bridge.dispatch("test.advance", "{}", lambda _: calls.append(1))
            except RuntimeError as error:
                failures.append(str(error))

        thread = threading.Thread(target=attempt)
        thread.start()
        thread.join(timeout=5)
        self.assertFalse(thread.is_alive())
        self.assertEqual(calls, [])
        self.assertEqual(len(failures), 1)
        bridge.dispatch("test.advance", "{}", lambda _: "{}")
        bridge.finish()
        self.read()

    def test_reentrant_dispatch_cannot_add_another_request(self):
        bridge = self.bridge()

        def callback(_):
            with self.assertRaises(RuntimeError):
                bridge.dispatch("test.advance", "{}", lambda _: "{}")
            return "{}"

        bridge.dispatch("test.advance", "{}", callback)
        bridge.finish()
        self.assertEqual(sum(row["type"] == "bridge_request" for row in self.read()), 1)

    def test_bad_log_permissions_before_dispatch_prevent_callback(self):
        bridge = self.bridge()
        self.path.chmod(0o644)
        calls = []
        with self.assertRaises(RuntimeError):
            bridge.dispatch("test.advance", "{}", lambda _: calls.append(1))
        self.assertEqual(calls, [])
        self.path.chmod(0o600)
        with self.assertRaises(RuntimeError):
            bridge.dispatch("test.advance", "{}", lambda _: "{}")

    def test_changed_log_after_callback_prevents_success_and_retry(self):
        bridge = self.bridge()
        calls = []

        def callback(_):
            calls.append(1)
            self.path.rename(self.path.with_name("retained-prefix.jsonl"))
            self.path.write_text("replacement")
            self.path.chmod(0o600)
            return "{}"

        with self.assertRaises(RuntimeError):
            bridge.dispatch("test.advance", "{}", callback)
        with self.assertRaises(RuntimeError):
            bridge.dispatch("test.advance", "{}", callback)
        self.assertEqual(calls, [1])

    def test_create_rejects_existing_files_and_symlinks(self):
        self.path.write_bytes(b"retain")
        with self.assertRaises(ValueError):
            Bridge(self.path, configuration())
        self.assertEqual(self.path.read_bytes(), b"retain")
        link = self.path.with_name("link.jsonl")
        link.symlink_to(self.path)
        with self.assertRaises(ValueError):
            Bridge(link, configuration())
        self.assertEqual(self.path.read_bytes(), b"retain")

    def test_private_sibling_artifact_is_bound_and_reopened(self):
        bridge = self.bridge()
        artifact = self.path.with_name("capture.bin")
        artifact.write_bytes(b"local observation")
        artifact.chmod(0o600)
        identity = json.loads(artifact_identity(self.path, artifact.name))
        self.assertEqual(
            identity,
            {"bytes": 17, "sha256": hashlib.sha256(artifact.read_bytes()).hexdigest()},
        )
        bridge.finish(artifact.name)
        event = self.read()[-2]
        self.assertEqual(event["sha256"], identity["sha256"])
        self.assertEqual(event["metadata"]["bytes"], str(identity["bytes"]))
        self.assertEqual(event["uri"], artifact.name)

    def test_artifact_rejects_parent_paths_symlinks_shared_files_and_self(self):
        bridge = self.bridge()
        artifact = self.path.with_name("capture.bin")
        artifact.write_bytes(b"private bytes")
        artifact.chmod(0o600)
        link = self.path.with_name("capture-link.bin")
        link.symlink_to(artifact)
        for name in (
            "../capture.bin",
            str(artifact),
            "",
            ".",
            "..",
            self.path.name,
            link.name,
        ):
            with self.subTest(name=name):
                with self.assertRaises((ValueError, RuntimeError)):
                    artifact_identity(self.path, name)
        artifact.chmod(0o644)
        with self.assertRaises(RuntimeError):
            artifact_identity(self.path, artifact.name)
        artifact.chmod(0o600)
        hard = self.path.with_name("hard.bin")
        os.link(artifact, hard)
        with self.assertRaises(RuntimeError):
            artifact_identity(self.path, artifact.name)
        hard.unlink()
        bridge.finish(artifact.name)
        self.read()

    def test_incomplete_corrupt_or_changed_inspection_never_returns_success(self):
        bridge = self.bridge()
        bridge.dispatch("test.advance", "{}", lambda _: "{}")
        with self.assertRaises(RuntimeError):
            self.read()
        bridge.finish()
        self.read()
        original = self.path.read_bytes()

        def mutate(_):
            value = self.path.stat()
            os.utime(self.path, ns=(value.st_atime_ns, value.st_mtime_ns + 1_000_000))

        with self.assertRaisesRegex(RuntimeError, "changed"):
            inspect_runlog(self.path, mutate)
        changed = [json.loads(line) for line in original.splitlines()]
        changed[2]["payload"]["changed_without_its_hash"] = True
        self.path.write_text("".join(encoded(row) + "\n" for row in changed))
        self.assertNotEqual(self.path.read_bytes(), original)
        with self.assertRaises(RuntimeError):
            self.read()
        self.path.write_bytes(original)
        self.read()

    def test_full_size_request_and_result_fit_the_admitted_call_budget(self):
        bridge = self.bridge(max_calls=1)
        payload = encoded({"x": "a" * (65_536 - 8)})
        self.assertEqual(len(payload.encode()), 65_536)
        response = json.loads(
            bridge.dispatch("test.advance", payload, lambda value: value)
        )
        self.assertEqual(response["result"], json.loads(payload))
        completed = json.loads(bridge.finish())
        self.assertLessEqual(completed["bytes"], 65_536 + 8_192 + 2 * 65_536 + 4_096)
        self.read()

    def test_visitor_failure_remains_a_failure_and_does_not_modify_evidence(self):
        self.bridge().finish()
        original = self.path.read_bytes()

        def fail(_):
            raise ValueError("visitor stopped")

        with self.assertRaisesRegex(ValueError, "visitor stopped"):
            inspect_runlog(self.path, fail)
        self.assertEqual(self.path.read_bytes(), original)
        self.read()


if __name__ == "__main__":
    unittest.main()
