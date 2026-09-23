"""Actual direct-child lifecycle controls; these launch no simulator."""

import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

path = Path(__file__).resolve().parents[1] / "scripts" / "perf_campaign.py"
spec = importlib.util.spec_from_file_location("perf_campaign_test_subject", path)
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)
observer = os.environ.get("PRISOMA_M1_OBSERVER_BINARY")


class CampaignControls(unittest.TestCase):
    def capture(self, code, *, session=3, grace=1, stream=1024):
        with tempfile.TemporaryDirectory(prefix="perf-controls-") as temporary:
            directory = Path(temporary).resolve() / "command"
            result = p.capture(
                [sys.executable, "-I", "-B", "-c", code],
                {"PATH": "/usr/bin:/bin"},
                directory,
                observer or "/unselected-observer-not-invoked",
                {
                    "session_seconds": session,
                    "cleanup_seconds": grace,
                    "stream_bytes": stream,
                },
                time.monotonic() + 10,
            )
            result["observed_stdout"] = (directory / "stdout.bin").read_bytes()
            return result

    def test_complete_roster_fixed_order_unique_balanced_arms(self):
        rows = p.planned_cases()
        self.assertEqual(len(rows), 192)
        self.assertEqual(len({row["case_id"] for row in rows}), 192)
        for block in range(32):
            selected = rows[block * 6 : (block + 1) * 6]
            self.assertEqual(
                {(row["route"], row["instrumentation"]) for row in selected},
                {
                    (route, level)
                    for route in ("direct", "body", "canonical")
                    for level in ("minimal", "detailed")
                },
            )
        self.assertEqual(rows[36]["route"], "canonical")
        self.assertEqual(rows[36]["instrumentation"], "detailed")

    @unittest.skipUnless(observer, "select a separately qualified Darwin observer")
    def test_actual_child_output_exit_and_observed_retirement(self):
        result = self.capture('import time; time.sleep(.3); print("preserved output")')
        self.assertEqual(result["returncode"], 0)
        self.assertEqual(result["observed_stdout"], b"preserved output\n")
        self.assertIsNone(result["failure"])
        self.assertTrue(result["observation"]["observed_identities_retired"])
        self.assertEqual(result["observation"]["observed_identity_count"], 1)
        self.assertFalse(result["forced_kill"])
        self.assertEqual(result["signals_to_descendants"], 0)

    @unittest.skipUnless(observer, "select a separately qualified Darwin observer")
    def test_nonzero_exit_is_retained_and_never_relabelled_success(self):
        result = self.capture(
            'import sys,time; time.sleep(.3); print("failed input"); sys.exit(23)'
        )
        self.assertEqual(result["returncode"], 23)
        self.assertEqual(result["observed_stdout"], b"failed input\n")
        self.assertTrue(result["observation"]["observed_identities_retired"])

    @unittest.skipUnless(observer, "select a separately qualified Darwin observer")
    def test_actual_output_overflow_retains_prefix_and_stops_owned_child(self):
        result = self.capture(
            'import sys,time; time.sleep(.3); sys.stdout.write("x"*200); sys.stdout.flush(); time.sleep(5)',
            stream=32,
        )
        self.assertTrue(result["output_overflow"])
        self.assertEqual(result["observed_stdout"], b"x" * 32)
        self.assertEqual(result["failure"], "output_overflow")
        self.assertFalse(result["forced_kill"])

    @unittest.skipUnless(observer, "select a separately qualified Darwin observer")
    def test_timeout_and_force_are_separate_from_observed_cleanup(self):
        result = self.capture(
            "import signal,time; signal.signal(signal.SIGINT, signal.SIG_IGN); time.sleep(5)",
            session=0.5,
            grace=0.2,
        )
        self.assertTrue(result["timed_out"])
        self.assertTrue(result["forced_kill"])
        self.assertEqual(result["returncode"], -9)
        self.assertEqual(result["signals_to_descendants"], 0)
        self.assertTrue(result["observation"]["observed_identities_retired"])

    def test_artifact_scan_rejects_symlink_and_counts_original_bytes(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "original").write_bytes(b"12345")
            self.assertEqual(p.tree_size(directory), 5)
            (directory / "alias").symlink_to(directory / "original")
            with self.assertRaises(p.m.MeasurementError):
                p.tree_size(directory)

    def test_launch_failure_and_later_stream_fsync_failure_are_both_retained(self):
        class LaunchFailure(RuntimeError):
            pass

        class SyncFailure(OSError):
            pass

        original, later = LaunchFailure("original launch"), SyncFailure("later fsync")
        for fails_sync in (False, True):
            with (
                self.subTest(fails_sync=fails_sync),
                tempfile.TemporaryDirectory() as temporary,
            ):
                directory = Path(temporary).resolve() / "command"
                unused_observer = Path(temporary).resolve() / "unused-observer"
                unused_observer.write_bytes(
                    b"launch failure must precede observation\n"
                )
                with (
                    patch.object(p.subprocess, "Popen", side_effect=original),
                    patch.object(
                        p.o.Observer,
                        "command",
                        side_effect=AssertionError("observer must not execute"),
                    ) as observe,
                ):
                    context = (
                        patch.object(p.os, "fsync", side_effect=later)
                        if fails_sync
                        else patch.object(p.os, "fsync", wraps=p.os.fsync)
                    )
                    with context:
                        result = p.capture(
                            [sys.executable, "-I", "-B", "-c", "pass"],
                            {"PATH": "/usr/bin:/bin"},
                            directory,
                            unused_observer,
                            {
                                "session_seconds": 3,
                                "cleanup_seconds": 1,
                                "stream_bytes": 1024,
                            },
                            time.monotonic() + 10,
                        )
                    observe.assert_not_called()
                rendered = json.dumps(result["failure"])
                self.assertIn("LaunchFailure", rendered)
                self.assertIn("original launch", rendered)
                self.assertEqual("SyncFailure" in rendered, fails_sync)
                self.assertIsNone(result["returncode"])
                self.assertIsNone(result["observation"])


if __name__ == "__main__":
    unittest.main()
