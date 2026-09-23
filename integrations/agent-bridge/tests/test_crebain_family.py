"""Canonical family controls; synthetic native values supply no scientific evidence."""

from dataclasses import replace
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import shutil
import tempfile
import threading
import unittest
from unittest.mock import patch

from crebain_ncp_sensors import codec as c, family_contract as fc
from ncp_local import modular_wire as w
from prisoma_agent_bridge import hash_object
from prisoma_agent_bridge import crebain_family as a
from prisoma_agent_bridge.crebain import ExperimentError, _contains_error
from prisoma_ncp_transcript import CaptureError, Journal

from family_support import complete, plan, synthetic_owner


class FamilyAdmissionTests(unittest.TestCase):
    def test_exact_e1_capacity_matches_independent_payload_and_command_counts(self):
        for branches, exchanges, calls in ((5, 1818, 181), (15, 4618, 461)):
            selected = plan(branches, ticks=36, landmark=12)
            scene = selected.body.specification.scene
            camera = replace(scene.rgbCameras[0], width=160, height=120, periodTicks=3)
            scene = replace(
                scene, rgbCameras=(camera, replace(camera, id="second-camera"))
            )
            selected = replace(
                selected,
                body=replace(
                    selected.body,
                    specification=replace(selected.body.specification, scene=scene),
                ),
            )
            budget = a.capture_budget(selected)
            self.assertEqual(
                (budget.max_exchanges, budget.max_calls), (exchanges, calls)
            )
            self.assertEqual(budget.endpoints[0].max_exchanges, 418 + 2 * branches)
            self.assertEqual(
                [row.max_exchanges for row in budget.endpoints[1:]], [278] * branches
            )
            self.assertEqual(budget.max_call_frame_bytes, 2_883_584)
            self.assertEqual(len(a._schedule(selected)), calls)
            self.assertEqual(
                budget.quota_bytes, sum(row.quota_bytes for row in budget.endpoints)
            )
        with self.assertRaises((w.ModularError, ExperimentError)):
            a.capture_budget(plan(15, ticks=120, landmark=3))

    def test_distinct_family_run_ids_require_separate_journals(self):
        selected = plan()
        self.assertEqual(len({peer.binding.run_id for peer in a._peers(selected)}), 3)
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(CaptureError):
                Journal(
                    Path(directory) / "invalid",
                    a._peers(selected),
                    max_exchanges=100,
                    quota_bytes=20_000_000,
                )
            journals = []
            try:
                for slot, (peer, budget) in enumerate(
                    zip(
                        a._peers(selected),
                        a.capture_budget(selected).endpoints,
                        strict=True,
                    )
                ):
                    journals.append(
                        Journal(
                            Path(directory) / str(slot),
                            (peer,),
                            max_exchanges=budget.max_exchanges,
                            quota_bytes=budget.quota_bytes,
                        )
                    )
                self.assertEqual(len(journals), 3)
            finally:
                for journal in journals:
                    journal.close()

    def test_forecast_preserves_arbitrary_original_bytes_and_rejects_bad_encoding(self):
        import base64

        original = b' \x00{"scores": [1, 2]}\n'
        payload = {
            "forecast_base64": base64.b64encode(original).decode(),
            "selected_case_id": "case-1",
        }
        self.assertEqual(a._forecast(payload), (original, "case-1"))
        for encoded in ("", "_", "YQ==\n", "YR==", "A" * 65536):
            with (
                self.subTest(encoded=encoded[:20]),
                self.assertRaises((ValueError, w.ModularError)),
            ):
                a._forecast({**payload, "forecast_base64": encoded})

    def test_index_names_and_existing_index_fail_before_any_owner(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "run.jsonl"
            for name in (
                "../capture",
                "a/b",
                "a\\b",
                ".",
                "..",
                "\0",
                "x" * 97,
                log.name,
            ):
                with (
                    self.subTest(name=name),
                    patch.object(a, "family_session") as owner,
                    self.assertRaises(ExperimentError),
                ):
                    a.FamilyExperiment(None, plan(), log, name)
                owner.assert_not_called()
                self.assertFalse(log.exists())
            existing = log.parent / "family.capture.json"
            existing.write_bytes(b"unrelated")
            with (
                patch.object(a, "family_session") as owner,
                self.assertRaises(ExperimentError),
            ):
                a.FamilyExperiment(None, plan(), log)
            owner.assert_not_called()
            self.assertEqual(existing.read_bytes(), b"unrelated")
            self.assertFalse(log.exists())

    def test_private_index_rejects_directory_symlink_fifo_and_oversize(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            valid = root / "index"
            a._index_bytes(valid, b'{"valid":true}')
            self.assertEqual(a._index_bytes(valid), b'{"valid":true}')
            with self.assertRaises(FileExistsError):
                a._index_bytes(valid, b"replacement")
            link = root / "link"
            link.symlink_to(valid)
            fifo = root / "fifo"
            os.mkfifo(fifo, 0o600)
            large = root / "large"
            large.write_bytes(b"a" * (a.MAX_INDEX_BYTES + 1))
            large.chmod(0o600)
            for path in (root, link, fifo, large):
                with (
                    self.subTest(path=path.name),
                    self.assertRaises((OSError, ExperimentError)),
                ):
                    a._index_bytes(path)

    def test_index_primary_and_descriptor_cleanup_failures_are_both_retained(self):
        primary, cleanup = (
            KeyboardInterrupt("synthetic read cancellation"),
            OSError("synthetic close failure"),
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "index"
            a._index_bytes(path, b"{}")
            real_close = os.close
            closed = []

            def close(fd):
                real_close(fd)
                closed.append(fd)
                raise cleanup

            with (
                patch.object(os, "read", side_effect=primary),
                patch.object(os, "close", side_effect=close),
                self.assertRaises(BaseExceptionGroup) as raised,
            ):
                a._index_bytes(path)
            self.assertEqual(raised.exception.exceptions, (primary, cleanup))
            self.assertEqual(len(closed), 1)
            with self.assertRaises(OSError):
                os.fstat(closed[0])
            self.assertEqual(a._index_bytes(path), b"{}")

    def test_wrong_role_and_bool_slot_are_rejected_before_dispatch(self):
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(a, "family_session") as owner,
        ):
            experiment = a.FamilyExperiment(None, plan(), Path(directory) / "run.jsonl")
            for invoke in (
                lambda: experiment.restore(True),
                lambda: experiment.evaluate(0),
                lambda: experiment.reserve("case-1"),
            ):
                with self.assertRaises(ExperimentError):
                    invoke()
            owner.assert_not_called()
            experiment.close()


class FamilyFailureTests(unittest.TestCase):
    def test_pre_yield_owner_failure_preserves_original_and_incomplete_request(self):
        primary = OSError("synthetic installed owner entry failure")

        @contextmanager
        def unavailable(*args, **kwargs):
            raise primary
            yield

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "entry.jsonl"
            with (
                patch.object(a, "family_session", unavailable),
                self.assertRaises(OSError) as raised,
            ):
                with a.owned_family_experiment(None, plan(), path):
                    self.fail("Failed owner entry yielded an experiment")
            self.assertIs(raised.exception, primary)
            events = [json.loads(line) for line in path.read_text().splitlines()]
            self.assertTrue(any(row["type"] == "bridge_request" for row in events))
            self.assertFalse(any(row["type"] == "run_ended" for row in events))
            self.assertFalse((path.parent / "family.capture.json").exists())

    def test_journal_cleanup_failure_does_not_discard_owner_entry_error(self):
        primary, cleanup = (
            OSError("synthetic entry failure"),
            OSError("synthetic journal close failure"),
        )
        real_close = a.Journal.close
        closed = []

        @contextmanager
        def unavailable(*args, **kwargs):
            raise primary
            yield

        def close(journal):
            real_close(journal)
            closed.append(journal)
            raise cleanup

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "entry.jsonl"
            with (
                patch.object(a, "family_session", unavailable),
                patch.object(a.Journal, "close", close),
                self.assertRaises(BaseExceptionGroup) as raised,
            ):
                with a.owned_family_experiment(None, plan(), path):
                    self.fail("Failed owner entry yielded an experiment")
            self.assertIs(_contains_error(raised.exception, primary), True)
            self.assertIs(_contains_error(raised.exception, cleanup), True)
            self.assertEqual(len(closed), 3)

    def test_exception_diagnostics_cannot_prevent_owner_or_resource_retirement(self):
        class HostileError(RuntimeError):
            def __bool__(self):
                raise AssertionError("Exception truthiness must not execute")

            @property
            def __traceback__(self):
                raise AssertionError("Exception traceback property must not execute")

        for primary in (RuntimeError("ordinary"), HostileError("hostile")):
            events = []

            class Context:
                def __exit__(self, kind, error, traceback):
                    events.append(("owner", kind, error, traceback))

            class Resource:
                def __init__(self, name):
                    self.name = name

                def close(self):
                    events.append(self.name)

            experiment = a.FamilyExperiment.__new__(a.FamilyExperiment)
            experiment._thread = threading.get_ident()
            experiment._closed = False
            experiment._failure = None
            experiment._owner_entered = True
            experiment._context = Context()
            experiment._journals = [Resource("parent"), Resource("child")]
            experiment._bridge = Resource("bridge")
            with self.assertRaises(type(primary)) as raised:
                experiment._close(primary)
            self.assertIs(raised.exception, primary)
            self.assertEqual(events[0][:3], ("owner", type(primary), primary))
            self.assertIsNone(events[0][3])
            self.assertEqual(events[1:], ["parent", "child", "bridge"])
            self.assertTrue(experiment._closed)
            self.assertFalse(experiment._owner_entered)


@unittest.skipUnless(
    os.environ.get("CREBAIN_FAMILY_PRODUCER"),
    "explicit constructed SDK and synthetic family fixture required",
)
class FamilySocketTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.storage = tempfile.TemporaryDirectory()
        cls.root = Path(cls.storage.name)
        cls.selected = plan()
        cls.log = cls.root / "run.jsonl"
        with patch.object(a, "family_session", synthetic_owner):
            with a.owned_family_experiment(None, cls.selected, cls.log) as experiment:
                cls.completed_run, cls.observations = complete(experiment, cls.selected)
                cls.exit = experiment.process_exit

    @classmethod
    def tearDownClass(cls):
        cls.storage.cleanup()

    def clone(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        for source in self.root.iterdir():
            shutil.copy2(source, Path(temp.name) / source.name)
        return Path(temp.name) / self.log.name

    def events(self, path):
        return [json.loads(line) for line in path.read_text().splitlines()]

    def write_events(self, path, events):
        path.write_text(
            "".join(
                json.dumps(row, allow_nan=False, separators=(",", ":")) + "\n"
                for row in events
            )
        )

    def mutate_index(self, path, change):
        index_path = path.parent / "family.capture.json"
        index = json.loads(index_path.read_bytes())
        change(index)
        raw = (
            json.dumps(index, allow_nan=False, separators=(",", ":")) + "\n"
        ).encode()
        index_path.write_bytes(raw)
        events = self.events(path)
        artifact = events[-2]
        artifact["sha256"] = hashlib.sha256(raw).hexdigest()
        artifact["metadata"] = {"bytes": str(len(raw))}
        self.write_events(path, events)

    def test_complete_original_bytes_and_one_global_timeline_replay(self):
        observed = []
        with patch.object(
            a, "family_session", side_effect=AssertionError("readback cannot launch")
        ):
            report = a.inspect_family_run(self.log, observed.append)
        self.assertTrue(report["family_replayed"])
        self.assertFalse(report["scientific_validation"])
        self.assertFalse(report["producer_process_retirement_verified"])
        self.assertTrue(self.exit["cleanup_confirmed"])
        self.assertEqual(report["calls"], a.capture_budget(self.selected).max_calls)
        replayed = [
            event.result
            for event in observed
            if type(event.result) is a.FamilyObservation
        ]
        self.assertEqual(replayed, self.observations)
        self.assertEqual(
            [row.exchange_pairs for row in self.completed_run.captures],
            [row.max_exchanges for row in a.capture_budget(self.selected).endpoints],
        )
        decision = next(
            i for i, event in enumerate(observed) if event.method == a.METHODS[3]
        )
        final = next(
            i
            for i, event in enumerate(observed)
            if type(event.result) is a.FamilyObservation
            and event.slot == 0
            and event.result.response.canonical_final_state is not None
        )
        labels = [i for i, event in enumerate(observed) if event.method == a.METHODS[6]]
        self.assertTrue(all(decision < final < label for label in labels))

    def test_maximum_sixteen_endpoints_use_distinct_original_journals(self):
        with tempfile.TemporaryDirectory() as directory:
            selected = plan(15)
            path = Path(directory) / "maximum.jsonl"
            with (
                patch.object(a, "family_session", synthetic_owner),
                a.owned_family_experiment(None, selected, path) as experiment,
            ):
                complete(experiment, selected)
            report = a.verify_family_run(path)
            self.assertEqual(report["endpoints"], 16)
            self.assertEqual(len(report["captures"]), 16)

    def test_rehashed_missing_extra_reordered_or_foreign_journals_are_rejected(self):
        changes = [
            lambda value: value["journals"].pop(),
            lambda value: value["journals"].append(value["journals"][-1]),
            lambda value: value["journals"].reverse(),
            lambda value: value["journals"][1].update(binding=c.raw(fc.new_binding())),
            lambda value: value.update(plan_hash="f" * 64),
        ]
        for change in changes:
            path = self.clone()
            self.mutate_index(path, change)
            with self.assertRaises((ExperimentError, ValueError)):
                a.verify_family_run(path)

    def test_late_parent_corruption_keeps_child_visits_provisional(self):
        path = self.clone()
        parent = path.parent / "family.capture.json.00.ncp"
        original = parent.read_bytes()
        altered = original[:-1] + bytes((original[-1] ^ 1,))
        parent.write_bytes(altered)
        self.mutate_index(
            path,
            lambda value: value["journals"][0].update(
                identity={
                    "bytes": len(altered),
                    "sha256": hashlib.sha256(altered).hexdigest(),
                }
            ),
        )
        observed = []
        with self.assertRaises(CaptureError):
            a.inspect_family_run(path, observed.append)
        self.assertTrue(
            any(event.method == a.METHODS[7] and event.slot == 2 for event in observed)
        )

    def test_changed_forecast_bytes_cannot_reconstruct_original_ncp_commitment(self):
        path = self.clone()
        events = self.events(path)
        request = next(
            row
            for row in events
            if row["type"] == "bridge_request" and row["method"] == a.METHODS[3]
        )
        request["payload"]["forecast_base64"] = "Y2hhbmdlZA=="
        request["payload_hash"] = hash_object(json.dumps(request["payload"]))
        self.write_events(path, events)
        with self.assertRaisesRegex(ExperimentError, "original request bytes"):
            a.verify_family_run(path)

    def test_rehashed_foreign_receipt_role_cannot_move_an_evaluation_to_parent(self):
        path = self.clone()
        events = self.events(path)
        index = next(
            i
            for i, row in enumerate(events)
            if row["type"] == "label_observed"
            and row["value"]["method"] == a.METHODS[6]
        )
        result = events[index]["value"]["result"]
        result["slot"] = 0
        events[index + 1]["result_hash"] = hash_object(
            json.dumps(result, separators=(",", ":"))
        )
        self.write_events(path, events)
        with self.assertRaisesRegex(ExperimentError, "endpoint or schema"):
            a.verify_family_run(path)

    def test_visitor_cancellation_remains_primary_when_local_close_also_fails(self):
        primary, cleanup = (
            KeyboardInterrupt("visitor cancellation"),
            OSError("local close failed"),
        )
        with (
            patch.object(a.FamilySession, "close", side_effect=cleanup),
            self.assertRaises(BaseExceptionGroup) as raised,
        ):
            a.inspect_family_run(self.log, lambda event: (_ for _ in ()).throw(primary))
        self.assertEqual(raised.exception.exceptions, (primary, cleanup))
        self.assertTrue(a.verify_family_run(self.log)["family_replayed"])

    def test_early_context_exit_preserves_incomplete_prefix_without_automatic_finish(
        self,
    ):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "early.jsonl"
            with (
                patch.object(a, "family_session", synthetic_owner),
                self.assertRaises(ExperimentError),
            ):
                with a.owned_family_experiment(None, plan(), path):
                    pass
            self.assertFalse((path.parent / "family.capture.json").exists())
            events = self.events(path)
            self.assertFalse(any(row["type"] == "run_ended" for row in events))
            with self.assertRaises((ValueError, RuntimeError)):
                a.verify_family_run(path)

    def test_caught_callback_failure_still_prevents_successful_context_completion(self):
        primary = OSError("synthetic collector callback failed")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "callback.jsonl"
            with (
                patch.object(a, "family_session", synthetic_owner),
                self.assertRaises(BaseException) as raised,
            ):
                with a.owned_family_experiment(None, plan(), path) as experiment:
                    with (
                        patch.object(a, "_execute", side_effect=primary),
                        self.assertRaises(OSError) as immediate,
                    ):
                        experiment.advance()
                    self.assertIs(immediate.exception, primary)
            self.assertIs(_contains_error(raised.exception, primary), True)
            self.assertFalse((path.parent / "family.capture.json").exists())

    def test_recording_failure_after_effect_keeps_original_error_and_ends_owner(self):
        primary = OSError("synthetic recording failure after accepted response")
        real_bridge = a.Bridge

        class FailingRecorder:
            def __init__(self, *args):
                self.inner = real_bridge(*args)

            def dispatch(self, method, payload, callback):
                result = self.inner.dispatch(method, payload, callback)
                if method == a.METHODS[1]:
                    raise primary
                return result

            def close(self):
                self.inner.close()

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "recording.jsonl"
            selected = plan()
            with (
                patch.object(a, "family_session", synthetic_owner),
                patch.object(a, "Bridge", FailingRecorder),
                self.assertRaises(OSError) as raised,
            ):
                with a.owned_family_experiment(None, selected, path) as experiment:
                    experiment.advance(selected.branches[0].target)
            self.assertIs(raised.exception, primary)
            events = self.events(path)
            advances = [
                row
                for row in events
                if row["type"] == "label_observed"
                and row["value"]["method"] == a.METHODS[1]
            ]
            self.assertEqual(len(advances), 1)
            self.assertEqual(
                advances[0]["value"]["result"]["result"]["response"]["body"]["tick"], 1
            )
            self.assertIsNotNone(experiment.process_exit)
            self.assertFalse(any(row["type"] == "run_ended" for row in events))

    def test_finish_cleanup_failure_cannot_publish_index_or_canonical_success(self):
        cleanup = OSError("synthetic owner cleanup failure after actual retirement")

        @contextmanager
        def failed_exit(*args, **kwargs):
            with synthetic_owner(*args, **kwargs) as session:
                yield session
            raise cleanup

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "cleanup.jsonl"
            selected = plan()
            with (
                patch.object(a, "family_session", failed_exit),
                self.assertRaises(OSError) as raised,
            ):
                with a.owned_family_experiment(None, selected, path) as experiment:
                    complete(experiment, selected)
            self.assertIs(raised.exception, cleanup)
            self.assertTrue(experiment.process_exit["cleanup_confirmed"])
            self.assertFalse((path.parent / "family.capture.json").exists())
            self.assertFalse(
                any(row["type"] == "run_ended" for row in self.events(path))
            )


if __name__ == "__main__":
    unittest.main()
