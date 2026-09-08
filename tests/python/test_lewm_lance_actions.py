"""Paired snapshot controls; actual sklearn fitting requires its optional runtime."""

from dataclasses import replace
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import numpy as np

from experiments.lewm import lance_actions as reader
from experiments.lewm.assets import verify_runtime


class _Inputs:
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source = self.root / "arbitrary-name.data"
        self.rows = np.array([[-2, 0], [0, 0], [3, 1]], dtype="<f4")
        self.data = self.rows.tobytes()
        self.source.write_bytes(self.data)
        self.profile = reader._Column(
            3,
            hashlib.sha256(self.data).hexdigest(),
            "synthetic_control_only",
            {"source_id": "snapshot-control"},
        )

    def read(self):
        return reader._read_column(self.source, self.profile)


class SnapshotControls(_Inputs, unittest.TestCase):
    def test_exact_bytes_and_non_dataset_filename(self):
        with patch.object(reader.os, "close", wraps=os.close) as close:
            self.assertEqual(self.read(), self.data)
        self.assertEqual(close.call_count, 1)

    def test_shape_budget_precedes_open(self):
        for rows in (0, 1, 8_388_609, 2**100):
            with (
                self.subTest(rows=rows),
                patch.object(reader.os, "open", side_effect=AssertionError("opened")),
                self.assertRaises(ValueError),
            ):
                reader._read_column(self.source, replace(self.profile, rows=rows))
        self.assertEqual(self.read(), self.data)

    def test_short_long_and_same_length_changed_bytes_reject(self):
        for data in (self.data[:-1], self.data + b"x", b"x" + self.data[1:]):
            self.source.write_bytes(data)
            with self.subTest(size=len(data)), self.assertRaises(ValueError):
                self.read()
        self.source.write_bytes(self.data)
        self.assertEqual(self.read(), self.data)

    def test_non_regular_and_final_symlink_inputs_reject(self):
        alias = self.root / "alias"
        alias.symlink_to(self.source)
        directory = self.root / "directory"
        directory.mkdir()
        fifo = self.root / "fifo"
        os.mkfifo(fifo)
        for path in (alias, directory, fifo):
            with self.subTest(path=path.name), self.assertRaises((OSError, ValueError)):
                reader._read_column(path, self.profile)
        self.assertEqual(self.read(), self.data)

    def test_growth_after_admission_rejects(self):
        original = os.read
        changed = False

        def grow(descriptor, size):
            nonlocal changed
            if not changed:
                changed = True
                with self.source.open("ab") as stream:
                    stream.write(b"x")
            return original(descriptor, size)

        with (
            patch.object(reader.os, "read", grow),
            self.assertRaisesRegex(ValueError, "grew"),
        ):
            self.read()
        self.source.write_bytes(self.data)
        self.assertEqual(self.read(), self.data)

    def test_source_identity_change_rejects_even_with_original_bytes(self):
        original = os.read
        before = self.source.stat()

        def touch(descriptor, size):
            block = original(descriptor, size)
            if not block:
                os.utime(
                    self.source,
                    ns=(before.st_atime_ns, before.st_mtime_ns + 1_000_000),
                )
            return block

        with (
            patch.object(reader.os, "read", touch),
            self.assertRaisesRegex(ValueError, "changed during"),
        ):
            self.read()
        self.assertEqual(self.read(), self.data)

    def test_deadline_rejects_with_same_valid_input_counterpart(self):
        with patch.object(reader.time, "monotonic", side_effect=[0.0, 61.0]):
            with self.assertRaises(TimeoutError):
                self.read()
        self.assertEqual(self.read(), self.data)

    def test_failed_close_preserves_primary_and_is_not_retried(self):
        original_close = os.close
        primary = OSError("selected read failure")

        def close(descriptor):
            original_close(descriptor)
            raise OSError("selected close failure")

        with (
            patch.object(reader.os, "read", side_effect=primary),
            patch.object(reader.os, "close", side_effect=close) as calls,
            self.assertRaises(OSError) as caught,
        ):
            self.read()
        self.assertIs(caught.exception, primary)
        self.assertEqual(calls.call_count, 1)
        self.assertIn("close also failed", primary.__notes__[0])
        self.assertEqual(self.read(), self.data)

    def test_failed_close_after_success_does_not_return_bytes(self):
        original_close = os.close

        def close(descriptor):
            original_close(descriptor)
            raise OSError("selected close failure")

        with (
            patch.object(reader.os, "close", close),
            self.assertRaisesRegex(OSError, "selected close failure"),
        ):
            self.read()
        self.assertEqual(self.read(), self.data)

    def test_public_issuer_rejects_control_bytes_before_fitting(self):
        output = self.root / "rejected-public"
        with (
            patch.object(reader, "_fit_owned_rows", side_effect=AssertionError("fit")),
            self.assertRaisesRegex(ValueError, "exact regular-file extent"),
        ):
            reader.fit_pusht_lance_snapshot(self.source, output)
        self.assertEqual(list(output.iterdir()), [])
        self.assertEqual(self.read(), self.data)

    def test_existing_output_is_preserved(self):
        output = self.root / "existing"
        output.mkdir()
        marker = output / "keep"
        marker.write_text("untouched")
        with self.assertRaises(FileExistsError):
            reader._fit_snapshot(self.source, output, self.profile)
        self.assertEqual(marker.read_text(), "untouched")


class QualifiedSklearnControls(_Inputs, unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        try:
            verify_runtime()
        except (ValueError, ModuleNotFoundError) as error:
            raise unittest.SkipTest(
                "Requires the exact optional LeWM runtime"
            ) from error

    def test_actual_fit_retains_source_scope_and_complete_rows(self):
        output = self.root / "fit"
        fitted = reader._fit_snapshot(self.source, output, self.profile)
        self.assertEqual(fitted.fit_rows.tobytes(), self.data)
        self.assertEqual(fitted.receipt()["scope"], "synthetic_control_only")
        self.assertFalse(fitted.receipt()["original_hdf5_equivalence"])
        self.assertFalse(fitted.receipt()["training_normalization_qualified"])
        self.assertEqual(fitted.receipt()["retained_rows"], 3)
        np.testing.assert_allclose(fitted.statistics["mean"], [1 / 3, 1 / 3])
        self.assertEqual((output / "actions.f32le").read_bytes(), self.data)
        self.assertEqual(
            json.loads((output / "scaler.json").read_text()), fitted.receipt()
        )
        self.assertEqual(
            json.loads((output / "transaction.json").read_text())["scaler_sha256"],
            fitted.sha256,
        )
        self.source.write_bytes(b"x" * len(self.data))
        self.assertEqual(fitted.fit_rows.tobytes(), self.data)

    def test_terminal_write_failure_issues_no_returned_handle(self):
        output = self.root / "failed-fit"
        original = reader._save
        primary = OSError("selected terminal write failure")
        writes = 0

        def save(path, data):
            nonlocal writes
            writes += 1
            if writes == 3:
                raise primary
            original(path, data)

        with patch.object(reader, "_save", save), self.assertRaises(OSError) as caught:
            reader._fit_snapshot(self.source, output, self.profile)
        self.assertIs(caught.exception, primary)
        self.assertFalse((output / "transaction.json").exists())
        self.assertEqual((output / "actions.f32le").read_bytes(), self.data)
        fitted = reader._fit_snapshot(
            self.source, self.root / "valid-fit", self.profile
        )
        self.assertEqual(fitted.fit_rows.tobytes(), self.data)


if __name__ == "__main__":
    unittest.main()
