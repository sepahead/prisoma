"""Owned-context lifecycle controls with synthetic peers, without native processes."""

from contextlib import contextmanager
from dataclasses import replace
import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from crebain_ncp_sensors import BodySession, SessionError
from ncp_local import modular_wire as w
from prisoma_agent_bridge import crebain as bridge

from synthetic_sensors import SyntheticSensors, binding, target
from test_crebain import channel, selected_plan


def leaves(error):
    if isinstance(error, BaseExceptionGroup):
        return [leaf for child in error.exceptions for leaf in leaves(child)]
    return [error]


class CountingSensors(SyntheticSensors):
    def __init__(self):
        super().__init__()
        self.operations = []

    def execute(self, operation, permit):
        self.operations.append(type(operation))
        return super().execute(operation, permit)


class SyntheticOwner:
    """Mirror body_session's normal auto-Finish and exceptional cleanup paths."""

    def __init__(self, log):
        self.log = log
        self.application = CountingSensors()
        self.entries = []
        self.exits = []
        self.entry_error = self.cleanup_error = None

    @contextmanager
    def __call__(self, runtime, prepare, **options):
        prefix = [json.loads(row) for row in self.log.read_text().splitlines()]
        self.entries.append((runtime, prepare, options, prefix))
        with channel(self.application) as (stream, host):
            self.host = host
            session = BodySession(
                stream,
                stream,
                options["binding"],
                prepare,
                deadline=time.monotonic() + 30,
                exchange=options["exchange"],
            )
            primary = None
            try:
                session.prepare()
                if self.entry_error is not None:
                    raise self.entry_error
                yield session
                session.finish()
            except BaseException as error:
                primary = error
            finally:
                self.exits.append(primary)
                session.close()
                session.process_exit = {"synthetic": True, "healthy": primary is None}
                session.diagnostics = b"synthetic bounded output"
                session.diagnostics_truncated = True
            if self.cleanup_error is not None:
                errors = (
                    [self.cleanup_error]
                    if primary is None
                    else [primary, self.cleanup_error]
                )
                raise BaseExceptionGroup("synthetic owner cleanup", errors)
            if primary is not None:
                raise primary


class OwnedSensorTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.log = Path(self.directory.name) / "run.jsonl"
        self.capture = self.log.with_name("capture.ncp")
        self.prepare = selected_plan(rgb=0, acoustic=1)
        self.runtime = object()
        self.owner = SyntheticOwner(self.log)

    def experiment(self, **options):
        return bridge.owned_sensor_experiment(
            self.runtime,
            self.prepare,
            self.log,
            self.capture,
            binding=binding(),
            **options,
        )

    def advance_all(self, experiment):
        for tick in range(self.prepare.planned_ticks):
            experiment.advance(target() if tick == 0 else None)
            self.assertEqual(self.owner.host.usage.live_slots, 0)

    def assert_unfinished(self):
        self.assertEqual(self.owner.application.operations.count(w.Finish), 0)
        self.assertFalse(self.owner.application.finished)
        with self.assertRaises((RuntimeError, ValueError)):
            bridge.verify_sensor_run(self.log)

    def test_explicit_finish_records_once_and_retains_owner_observations(self):
        with patch.object(bridge, "body_session", self.owner):
            with self.experiment(timeout_s=37, actor_id="owned-control") as experiment:
                self.assertIsNone(experiment.process_exit)
                with self.assertRaises(bridge.ExperimentError):
                    experiment.prepare()
                with self.assertRaises(bridge.ExperimentError):
                    experiment.finish()
                with self.assertRaises(ValueError):
                    experiment.advance(replace(target(), altitude_m=9.0))
                self.advance_all(experiment)
                result = experiment.finish()
        self.assertEqual(self.owner.exits, [None])
        self.assertEqual(self.owner.application.operations.count(w.Prepare), 1)
        self.assertEqual(self.owner.application.operations.count(w.Finish), 1)
        runtime, prepare, options, prefix = self.owner.entries[0]
        self.assertIs(runtime, self.runtime)
        self.assertIs(prepare, self.prepare)
        self.assertEqual(options["timeout_s"], 37)
        self.assertEqual(options["binding"], binding())
        self.assertEqual(prefix[-1]["type"], "bridge_request")
        self.assertEqual(prefix[-1]["method"], "crebain.prepare")
        self.assertEqual(prefix[-1]["actor"]["actor_id"], "owned-control")
        self.assertEqual(experiment.process_exit, {"synthetic": True, "healthy": True})
        self.assertEqual(experiment.diagnostics, b"synthetic bounded output")
        self.assertTrue(experiment.diagnostics_truncated)
        report = bridge.verify_sensor_run(self.log)
        self.assertEqual(report["completed_ticks"], self.prepare.planned_ticks)
        self.assertEqual(report["raw_bytes"], result.session.raw_bytes)
        self.assertFalse(report["scientific_validation"])
        self.assertFalse(report["producer_process_retirement_verified"])

    def test_missing_binding_uses_the_public_binding_factory(self):
        with (
            patch.object(bridge, "body_session", self.owner),
            patch.object(bridge, "new_binding", return_value=binding()) as create,
        ):
            with bridge.owned_sensor_experiment(
                self.runtime, self.prepare, self.log, self.capture
            ) as experiment:
                self.advance_all(experiment)
                experiment.finish()
        create.assert_called_once_with()
        self.assertEqual(self.owner.exits, [None])

    def test_complete_ticks_without_explicit_finish_abort(self):
        with patch.object(bridge, "body_session", self.owner):
            with self.assertRaises(bridge.ExperimentError) as stopped:
                with self.experiment() as experiment:
                    self.advance_all(experiment)
        self.assertEqual(self.owner.exits, [stopped.exception])
        self.assertFalse(experiment.process_exit["healthy"])
        self.assert_unfinished()

    def test_explicit_close_before_completion_aborts(self):
        with patch.object(bridge, "body_session", self.owner):
            with self.assertRaises(bridge.ExperimentError) as stopped:
                with self.experiment() as experiment:
                    experiment.close()
                    with self.assertRaises(bridge.ExperimentError):
                        experiment.advance(target())
        self.assertEqual(self.owner.exits, [stopped.exception])
        self.assert_unfinished()

    def test_caller_exception_is_preserved_and_skips_owner_finish(self):
        original = RuntimeError("caller stopped")
        with patch.object(bridge, "body_session", self.owner):
            with self.assertRaises(RuntimeError) as stopped:
                with self.experiment():
                    raise original
        self.assertIs(stopped.exception, original)
        self.assertEqual(self.owner.exits, [original])
        self.assert_unfinished()

    def test_failure_before_prepare_callback_never_enters_owner(self):
        original = OSError("canonical request sync failed")
        real_bridge = bridge.Bridge

        class FailingBridge:
            def __init__(self, *args):
                self.inner = real_bridge(*args)

            def dispatch(self, *args):
                raise original

            def close(self):
                self.inner.close()

        with (
            patch.object(bridge, "body_session", self.owner),
            patch.object(bridge, "Bridge", FailingBridge),
        ):
            with self.assertRaises(OSError) as stopped:
                with self.experiment():
                    self.fail("failed Prepare must not yield")
        self.assertIs(stopped.exception, original)
        self.assertEqual(self.owner.entries, [])
        self.assertEqual(self.owner.exits, [])

    def test_prepare_receipt_failure_closes_entered_owner_once(self):
        original = ValueError("canonical receipt failed")
        with (
            patch.object(bridge, "body_session", self.owner),
            patch.object(bridge, "_receipt", side_effect=original),
        ):
            with self.assertRaises(ValueError) as stopped:
                with self.experiment():
                    self.fail("failed Prepare must not yield")
        self.assertIs(stopped.exception, original)
        self.assertEqual(self.owner.exits, [original])
        self.assertEqual(self.owner.application.operations.count(w.Prepare), 1)
        self.assert_unfinished()

    def test_owner_entry_failure_is_not_exited_twice(self):
        original = ValueError("source identity mismatch before owner yield")
        self.owner.entry_error = original
        with patch.object(bridge, "body_session", self.owner):
            with self.assertRaises(ValueError) as stopped:
                with self.experiment():
                    self.fail("failed owner entry must not yield")
        self.assertIs(stopped.exception, original)
        self.assertEqual(self.owner.exits, [original])
        self.assert_unfinished()

    def test_caught_capture_failure_cannot_become_a_normal_owner_exit(self):
        with patch.object(bridge, "body_session", self.owner):
            with self.assertRaises(SessionError) as stopped:
                with self.experiment() as experiment:
                    exchange = experiment._journal.exchange

                    def lose_reply(*args, **kwargs):
                        exchange(*args, **kwargs)
                        raise OSError("captured mutation lost its result")

                    with patch.object(
                        experiment._journal, "exchange", side_effect=lose_reply
                    ):
                        with self.assertRaises(SessionError) as caught:
                            experiment.advance(target())
                    with self.assertRaises(bridge.ExperimentError):
                        experiment.advance(target())
        self.assertIs(stopped.exception, caught.exception)
        self.assertEqual(self.owner.exits, [caught.exception])
        self.assertEqual(caught.exception.last_validated_tick, 0)
        self.assertEqual(caught.exception.attempted_tick, 1)
        self.assertEqual(self.owner.application.advances, 1)
        self.assert_unfinished()

    def test_later_caller_error_does_not_replace_caught_dispatch_failure(self):
        first, later = ValueError("receipt stopped"), OSError("later caller stopped")
        with patch.object(bridge, "body_session", self.owner):
            with self.assertRaises(BaseExceptionGroup) as stopped:
                with self.experiment() as experiment:
                    with patch.object(bridge, "_receipt", side_effect=first):
                        with self.assertRaises(ValueError) as caught:
                            experiment.advance(target())
                    self.assertIs(caught.exception, first)
                    raise later
        self.assertEqual(leaves(stopped.exception), [first, later])
        self.assertEqual(self.owner.exits, [stopped.exception])
        self.assert_unfinished()

    def test_journal_finalization_failure_after_wire_finish_aborts_owner(self):
        original = OSError("capture finalization failed")
        with patch.object(bridge, "body_session", self.owner):
            with self.assertRaises(OSError) as stopped:
                with self.experiment() as experiment:
                    self.advance_all(experiment)
                    with patch.object(
                        experiment._journal, "finish", side_effect=original
                    ):
                        with self.assertRaises(OSError):
                            experiment.finish()
        self.assertIs(stopped.exception, original)
        self.assertEqual(self.owner.exits, [original])
        self.assertEqual(self.owner.application.operations.count(w.Finish), 1)
        with self.assertRaises((RuntimeError, ValueError)):
            bridge.verify_sensor_run(self.log)

    def test_canonical_finalization_failure_after_capture_aborts_owner(self):
        original = OSError("canonical finalization failed")
        real_bridge = bridge.Bridge

        class FailingBridge:
            def __init__(self, *args):
                self.inner = real_bridge(*args)

            def __getattr__(self, name):
                return getattr(self.inner, name)

            def finish(self, *args):
                raise original

        with (
            patch.object(bridge, "body_session", self.owner),
            patch.object(bridge, "Bridge", FailingBridge),
        ):
            with self.assertRaises(OSError) as stopped:
                with self.experiment() as experiment:
                    self.advance_all(experiment)
                    with self.assertRaises(OSError):
                        experiment.finish()
        self.assertIs(stopped.exception, original)
        self.assertEqual(self.owner.exits, [original])
        self.assertEqual(self.owner.application.operations.count(w.Finish), 1)
        with self.assertRaises((RuntimeError, ValueError)):
            bridge.verify_sensor_run(self.log)

    def test_primary_evidence_cleanup_and_owner_cleanup_errors_all_survive(self):
        first, evidence, owner = (
            ValueError("receipt stopped"),
            OSError("capture close stopped"),
            RuntimeError("owner cleanup stopped"),
        )
        self.owner.cleanup_error = owner
        with patch.object(bridge, "body_session", self.owner):
            with self.assertRaises(BaseExceptionGroup) as stopped:
                with self.experiment() as experiment:
                    close = experiment._journal.close

                    def failing_close():
                        close()
                        raise evidence

                    with (
                        patch.object(bridge, "_receipt", side_effect=first),
                        patch.object(
                            experiment._journal, "close", side_effect=failing_close
                        ) as closed,
                    ):
                        experiment.advance(target())
        self.assertEqual(leaves(stopped.exception), [first, evidence, owner])
        self.assertEqual(leaves(self.owner.exits[0]), [first, evidence])
        self.assertEqual(len(self.owner.exits), 1)
        closed.assert_called_once_with()
        self.assertFalse(experiment.process_exit["healthy"])
        self.assert_unfinished()

    def test_invalid_runtime_reaches_public_admission_without_spawning(self):
        with patch("crebain_ncp_sensors.owned._Process") as process:
            with self.assertRaises(ValueError):
                with self.experiment():
                    self.fail("invalid runtime must not yield")
        process.assert_not_called()
        self.assertTrue(self.log.exists())
        self.assertTrue(self.capture.exists())


if __name__ == "__main__":
    unittest.main()
