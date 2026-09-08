"""Synthetic archive/HDF5 and owned-process controls; no real dataset authority."""

import json
import itertools
import errno
import os
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch
from types import SimpleNamespace

import numpy as np

from experiments.lewm import dataset_actions as reader
from experiments.lewm import supported_actions as actions
from experiments.lewm.assets import verify_runtime


class AdmissionControls(unittest.TestCase):
    def test_invalid_control_budget_rejects_before_optional_import(self):
        with patch.object(
            reader, "verify_runtime", side_effect=AssertionError("imported")
        ):
            for size in (0, True, reader.CONTROL_DECODED_BYTES + 1):
                with self.subTest(size=size), self.assertRaises(ValueError):
                    reader.fit_control_archive_scaler(
                        Path("absent"),
                        Path("absent"),
                        decoded_bytes=size,
                        source_id="control",
                    )

    def test_regular_file_admission_rejects_links_directories_and_fifo(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            file = root / "file"
            file.write_bytes(b"x")
            (root / "link").symlink_to(file)
            os.mkfifo(root / "fifo")
            for path in (root / "link", root, root / "fifo"):
                with (
                    self.subTest(path=path.name),
                    self.assertRaises((OSError, ValueError)),
                ):
                    reader._open_regular(path, 1024)
            descriptor, info = reader._open_regular(file, 1024)
            os.close(descriptor)
            self.assertEqual(info.st_size, 1)

    def test_permission_denied_does_not_mean_process_group_absence(self):
        with patch.object(
            reader.os, "killpg", side_effect=PermissionError("owned control")
        ):
            self.assertTrue(reader._group_alive(12345))
        with patch.object(reader.os, "killpg", side_effect=ProcessLookupError):
            self.assertFalse(reader._group_alive(12345))

    def test_copied_receipt_has_no_public_dataset_issuer(self):
        with self.assertRaises(TypeError):
            reader.fit_pusht_training_scaler(
                Path("absent"), Path("absent"), expected_sha256="a" * 64
            )
        forged = object.__new__(actions.FittedActionScaler)
        with self.assertRaisesRegex(ValueError, "owner-issued"):
            actions.prepare_candidates(
                forged, ("a", "b"), np.zeros((2, 25, 2), np.float32)
            )

    def test_cleanup_requires_real_absence_after_signal_permission_denial(self):
        process = Mock(pid=12345, returncode=None)

        def reap(**_kwargs):
            process.returncode = 0
            return 0

        process.wait.side_effect = reap
        signaled = False

        def observation(_pid, sig):
            nonlocal signaled
            if sig:
                signaled = True
                raise PermissionError("synthetic exiting-process signal denial")
            if signaled:
                raise ProcessLookupError

        with (
            patch.object(reader.os, "killpg", side_effect=observation),
        ):
            result = reader._cleanup(
                reader._OwnedDecoder(process, Mock(return_value=None))
            )
        self.assertEqual(result["status"], "confirmed")
        self.assertTrue(result["group_absent"] and result["leader_reaped"])
        self.assertIn("signal denial", result["errors"][0])
        process.returncode = None
        with (
            patch.object(
                reader.os, "killpg", side_effect=PermissionError("still present")
            ),
            patch.object(reader.time, "monotonic", side_effect=itertools.count(step=3)),
        ):
            unresolved = reader._cleanup(
                reader._OwnedDecoder(process, Mock(return_value=None))
            )
        self.assertEqual(unresolved["status"], "unresolved")
        self.assertFalse(unresolved["group_absent"])

    def test_reaped_or_lost_child_never_signals_a_reused_group(self):
        for already_reaped in (True, False):
            process = Mock(pid=12345, returncode=0 if already_reaped else None)
            process.wait.return_value = 0
            with (
                self.subTest(already_reaped=already_reaped),
                patch.object(reader.os, "killpg") as group,
            ):
                observation = Mock(
                    side_effect=ChildProcessError(errno.ECHILD, "ownership lost")
                )
                result = reader._cleanup(reader._OwnedDecoder(process, observation))
                self.assertEqual(result["status"], "unresolved")
                self.assertTrue(result["ownership_lost"])
                self.assertTrue(all(call.args[1] == 0 for call in group.call_args_list))
                process.poll.assert_not_called()

    def test_signal_authority_is_revoked_before_terminal_wait(self):
        process = Mock(pid=12345, returncode=None)
        observation = Mock(return_value=None)
        owner = reader._OwnedDecoder(process, observation)

        def reap(**_kwargs):
            self.assertFalse(owner.may_signal)
            process.returncode = 0
            return 0

        process.wait.side_effect = reap
        with (
            patch.object(reader.os, "killpg") as group,
        ):
            owner.signal(signal.SIGTERM)
            observation.assert_called_once_with(process.pid)
            owner.reap(1)
            with self.assertRaises(ChildProcessError):
                owner.signal(signal.SIGKILL)
            group.assert_called_once_with(process.pid, signal.SIGTERM)
        process.poll.assert_not_called()

    def test_wait_timeout_and_wrong_identity_cannot_restore_signal_authority(self):
        process = Mock(pid=12345, returncode=None)
        process.wait.side_effect = subprocess.TimeoutExpired("owned", 1)
        owner = reader._OwnedDecoder(process, Mock(return_value=None))
        with self.assertRaises(subprocess.TimeoutExpired):
            owner.reap(1)
        with patch.object(reader.os, "killpg") as group:
            with self.assertRaises(ChildProcessError):
                owner.signal(signal.SIGKILL)
            group.assert_not_called()
        wrong = SimpleNamespace(si_pid=12346, si_code=os.CLD_EXITED, si_status=0)
        with (
            patch.object(reader.os, "killpg") as group,
        ):
            with self.assertRaisesRegex(ChildProcessError, "identity"):
                reader._OwnedDecoder(process, Mock(return_value=wrong)).signal(
                    signal.SIGTERM
                )
            group.assert_not_called()

    def test_descriptor_close_errors_do_not_retry_or_skip_other_owners(self):
        first = OSError("uncertain first close")
        second = OSError("uncertain second close")
        with patch.object(reader.os, "close", side_effect=[first, second]) as close:
            errors = reader._close_descriptors((11, 12, None))
        self.assertEqual(errors, [first, second])
        self.assertEqual([call.args[0] for call in close.call_args_list], [11, 12])

    def test_missing_wait_capability_rejects_before_child_or_output_creation(self):
        with (
            patch.object(
                reader,
                "_prepare_waitid",
                side_effect=ValueError("missing-wait-control"),
            ),
            patch.object(reader.subprocess, "Popen") as launch,
            patch.object(reader.os, "open") as output,
        ):
            with self.assertRaisesRegex(ValueError, "missing-wait-control"):
                reader._decode(12345, Path("absent-output"), 1, {})
            launch.assert_not_called()
            output.assert_not_called()


class QualifiedArchiveControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        try:
            verify_runtime()
            reader._decoder_identity()
        except (ValueError, OSError, ModuleNotFoundError) as error:
            raise unittest.SkipTest(
                "Requires exact optional LeWM and local zstd runtimes"
            ) from error
        import h5py

        cls.h5py = h5py

    def setUp(self):
        retained = os.environ.get("LEWM_DATASET_CONTROL_ROOT")
        self.root = Path(
            tempfile.mkdtemp(prefix=f"{self._testMethodName}-", dir=retained)
        )
        if retained is None:
            import shutil

            self.addCleanup(shutil.rmtree, self.root)
        self.sequence = 0

    def archive(self, rows=None, *, build=None):
        self.sequence += 1
        path = self.root / f"control-{self.sequence}.h5"
        with self.h5py.File(path, "w") as file:
            if build is not None:
                build(file)
            else:
                if rows is None:
                    rows = np.array([[-2, 0], [0, 0], [np.nan, 9], [2, 0]], np.float64)
                file.create_dataset("action", data=rows)
                # Other columns are not needed by the exact upstream action-column fit.
                file["unrelated-image-link"] = self.h5py.ExternalLink(
                    "absent-image-file", "pixels"
                )
        archive = path.with_suffix(".h5.zst")
        subprocess.run(
            [str(reader.DECODER), "--quiet", "--check", "-o", str(archive), str(path)],
            check=True,
            capture_output=True,
            timeout=10,
            env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"},
        )
        return archive, path.stat().st_size

    def fit(self, archive, size, *, name="result"):
        return reader.fit_control_archive_scaler(
            archive, self.root / name, decoded_bytes=size, source_id="owned-control"
        )

    def test_actual_archive_rows_fit_mask_and_no_dataset_authority(self):
        from sklearn.preprocessing import StandardScaler

        archive, size = self.archive()
        fitted = self.fit(archive, size)
        expected = np.array([[-2, 0], [0, 0], [np.nan, 9], [2, 0]], np.float64)
        np.testing.assert_array_equal(fitted.fit_rows, expected)
        np.testing.assert_array_equal(
            fitted.retained_row_mask, [True, True, False, True]
        )
        reference = StandardScaler().fit(expected[[0, 1, 3]])
        for key, value in (
            ("mean", reference.mean_),
            ("variance", reference.var_),
            ("scale", reference.scale_),
        ):
            np.testing.assert_array_equal(fitted.statistics[key], value)
        for values in (fitted.fit_rows, fitted.retained_row_mask):
            with self.assertRaises(ValueError):
                values.setflags(write=True)
        self.assertEqual(fitted.receipt()["scope"], "synthetic_control_only")
        terminal = json.loads((self.root / "result/transaction.json").read_text())
        self.assertEqual(terminal["status"], "FIT_COMPLETED")
        self.assertFalse(terminal["dataset_authority"])
        self.assertEqual(terminal["decode"]["cleanup"]["status"], "confirmed")
        self.assertNotIn("torch", sys.modules)
        self.assertNotIn("hdf5plugin", sys.modules)
        proposed = np.linspace(-0.4, 0.4, 100, dtype=np.float32).reshape(2, 25, 2)
        self.assertEqual(
            actions.prepare_candidates(fitted, ("a", "b"), proposed).receipt()[
                "scaler_scope"
            ],
            "synthetic_control_only",
        )

    def test_float32_row_order_and_full_na_mask_are_retained(self):
        rows = np.array([[0.2, 0.8], [np.nan, 2], [-0.1, 0.3], [0.9, -0.5]], np.float32)
        archive, size = self.archive(rows[::-1])
        fitted = self.fit(archive, size)
        self.assertEqual(fitted.fit_rows.dtype, np.dtype("float32"))
        np.testing.assert_array_equal(fitted.fit_rows, rows[::-1])
        np.testing.assert_array_equal(
            fitted.retained_row_mask, [True, True, False, True]
        )

    def test_full_dataset_issuer_rejects_control_archive_before_decoder(self):
        archive, _size = self.archive()
        with patch.object(reader, "_decode", side_effect=AssertionError("decoded")):
            with self.assertRaises(reader.DatasetReadError) as caught:
                reader.fit_pusht_training_scaler(archive, self.root / "dataset-attempt")
        self.assertIn("complete frozen compressed extent", str(caught.exception))
        self.assertFalse((self.root / "dataset-attempt/decoded.h5").exists())

    def test_original_archive_mutation_after_snapshot_cannot_replace_rows(self):
        archive, size = self.archive()
        original = reader._decode

        def change_source(*args):
            archive.write_bytes(b"replaced original source")
            return original(*args)

        with patch.object(reader, "_decode", side_effect=change_source):
            fitted = self.fit(archive, size)
        self.assertEqual(fitted.receipt()["retained_rows"], 3)

    def test_wrong_compressed_digest_rejects_before_decoder(self):
        archive, size = self.archive()
        profile = reader._ArchiveProfile(
            "synthetic_control_only",
            "wrong-digest-control",
            archive.stat().st_size,
            "0" * 64,
            size,
        )
        with patch.object(reader, "_decode", side_effect=AssertionError("decoded")):
            with self.assertRaisesRegex(reader.DatasetReadError, "source digest"):
                reader._fit_archive(archive, self.root / "wrong-digest", profile)

    def test_runtime_drift_after_row_admission_cannot_issue_scaler(self):
        archive, size = self.archive()
        runtime = verify_runtime()
        with patch.object(
            reader, "verify_runtime", side_effect=[runtime, runtime, {"drift": True}]
        ):
            with self.assertRaisesRegex(
                reader.DatasetReadError, "Runtime or decoder identity changed"
            ):
                self.fit(archive, size)
        self.assertFalse((self.root / "result/scaler.json").exists())

    def test_mutation_during_snapshot_rejects_and_retains_failure(self):
        archive, size = self.archive()
        original = reader._write_all
        mutated = False

        def change_source(descriptor, data):
            nonlocal mutated
            original(descriptor, data)
            if not mutated:
                archive.write_bytes(b"changed")
                mutated = True

        with patch.object(reader, "_write_all", side_effect=change_source):
            with self.assertRaisesRegex(
                reader.DatasetReadError, "changed during snapshot"
            ):
                self.fit(archive, size)
        self.assertTrue((self.root / "result/failure.json").is_file())

    def test_checksum_corruption_cannot_issue_scaler(self):
        archive, size = self.archive()
        data = bytearray(archive.read_bytes())
        data[-1] ^= 1
        archive.write_bytes(data)
        with self.assertRaises(reader.DatasetReadError) as caught:
            self.fit(archive, size)
        self.assertEqual(
            caught.exception.receipt["decode"]["cleanup"]["status"], "confirmed"
        )
        self.assertFalse((self.root / "result/scaler.json").exists())

    def test_uncompressed_pass_through_is_rejected(self):
        archive, size = self.archive()
        archive.write_bytes(archive.with_suffix("").read_bytes())
        with self.assertRaisesRegex(
            reader.DatasetReadError, "not a standard Zstandard"
        ):
            self.fit(archive, size)

    def test_exact_decoded_extent_and_overflow_both_reject(self):
        archive, size = self.archive()
        for bound in (size - 1, size + 1):
            with (
                self.subTest(bound=bound),
                self.assertRaises(reader.DatasetReadError) as caught,
            ):
                self.fit(archive, bound, name=f"bound-{bound}")
            self.assertEqual(
                caught.exception.receipt["decode"]["cleanup"]["status"], "confirmed"
            )
        self.assertEqual(self.fit(archive, size).receipt()["retained_rows"], 3)

    def test_supported_builtin_filters_have_a_paired_valid_read(self):
        archive, size = self.archive(
            build=lambda file: file.create_dataset(
                "action",
                data=np.arange(8, dtype=np.float64).reshape(4, 2),
                chunks=(2, 2),
                compression="gzip",
                compression_opts=4,
                shuffle=True,
                fletcher32=True,
            )
        )
        fitted = self.fit(archive, size)
        self.assertEqual(
            [
                row["id"]
                for row in fitted.receipt()["transaction"]["action_read"]["filters"]
            ],
            [2, 1, 3],
        )

    def test_unsupported_shape_dtype_fill_filter_and_links_reject(self):
        h5py = self.h5py
        bad = {
            "missing": lambda f: f.create_group("other"),
            "group": lambda f: f.create_group("action"),
            "soft": lambda f: f.__setitem__("action", h5py.SoftLink("/absent")),
            "external-link": lambda f: f.__setitem__(
                "action", h5py.ExternalLink("absent", "/action")
            ),
            "shape": lambda f: f.create_dataset(
                "action", shape=(reader.MAX_FIT_ROWS + 1, 2), dtype="f4"
            ),
            "integer": lambda f: f.create_dataset(
                "action", data=np.ones((2, 2), dtype=np.int32)
            ),
            "endian": lambda f: f.create_dataset(
                "action", data=np.ones((2, 2), dtype=">f4")
            ),
            "vlen": lambda f: f.create_dataset(
                "action", shape=(2, 2), dtype=h5py.vlen_dtype(np.dtype("f8"))
            ),
            "fill-only": lambda f: f.create_dataset("action", shape=(2, 2), dtype="f4"),
            "scaleoffset": lambda f: f.create_dataset(
                "action", data=np.ones((2, 2)), scaleoffset=3
            ),
            "external-storage": lambda f: f.create_dataset(
                "action",
                data=np.ones((2, 2)),
                external=[(str(self.root / "external.bin"), 0, h5py.h5f.UNLIMITED)],
            ),
            "chunk": lambda f: f.create_dataset(
                "action",
                shape=(2, 2),
                maxshape=(None, 2),
                chunks=(reader.MAX_FIT_ROWS + 1, 2),
                dtype="f8",
            ),
            "virtual": lambda f: f.create_virtual_dataset(
                "action", h5py.VirtualLayout(shape=(2, 2), dtype="f4")
            ),
        }
        reasons = {
            "missing": "direct HDF5 hard link",
            "group": "not a dataset",
            "soft": "direct HDF5 hard link",
            "external-link": "direct HDF5 hard link",
            "shape": "bounded native",
            "integer": "bounded native",
            "endian": "bounded native",
            "vlen": "bounded native",
            "fill-only": "unallocated",
            "scaleoffset": "Unsupported HDF5 filter",
            "external-storage": "unsupported HDF5 storage",
            "chunk": "chunk exceeds",
            "virtual": "unsupported HDF5 storage",
        }
        for name, build in bad.items():
            archive, size = self.archive(build=build)
            with (
                self.subTest(name=name),
                self.assertRaisesRegex(reader.DatasetReadError, reasons[name]),
            ):
                self.fit(archive, size, name=name)
        archive, size = self.archive()
        self.assertEqual(self.fit(archive, size).receipt()["retained_rows"], 3)

    def test_infinity_even_on_nan_excluded_row_rejects(self):
        for index, rows in enumerate(
            (
                [[0, 0], [1, np.inf]],
                [[0, 0], [1, 1], [np.nan, np.inf]],
                [[0, 0], [np.nan, 1]],
            )
        ):
            archive, size = self.archive(np.array(rows, np.float64))
            reason = (
                "Infinite fit values" if index < 2 else "two complete finite fit rows"
            )
            with (
                self.subTest(index=index),
                self.assertRaisesRegex(reader.DatasetReadError, reason),
            ):
                self.fit(archive, size, name=f"bad-{index}")

    def test_space_and_decoder_identity_reject_before_child_start(self):
        archive, size = self.archive()
        with patch.object(reader, "_space", side_effect=ValueError("space control")):
            with self.assertRaisesRegex(reader.DatasetReadError, "space control"):
                self.fit(archive, size)
        changed = list(reader._DECODER_FILES)
        changed[0] = (*changed[0][:3], "0" * 64)
        with patch.object(reader, "_DECODER_FILES", tuple(changed)):
            with self.assertRaisesRegex(ValueError, "dependency bytes changed"):
                self.fit(archive, size, name="decoder-drift")

    def test_final_close_failure_relinquishes_both_descriptors_once(self):
        archive, size = self.archive()
        original = reader._close_descriptors
        real_read = reader._read_rows
        real_close = os.close
        attempts = []
        released = []
        rows_admitted = False

        def read_rows(descriptor):
            nonlocal rows_admitted
            result = real_read(descriptor)
            rows_admitted = True
            return result

        def close_owned(descriptors):
            if not rows_admitted:
                return original(descriptors)
            attempts.append(descriptors)
            failed = False

            def close(descriptor):
                nonlocal failed
                real_close(descriptor)
                released.append(descriptor)
                if not failed:
                    failed = True
                    raise OSError("uncertain-close-control")

            with patch.object(reader.os, "close", side_effect=close):
                return original(descriptors)

        with (
            patch.object(reader, "_close_descriptors", side_effect=close_owned),
            patch.object(reader, "_read_rows", side_effect=read_rows),
        ):
            with self.assertRaisesRegex(
                reader.DatasetReadError, "uncertain-close-control"
            ) as failure:
                self.fit(archive, size)
        self.assertEqual(attempts[1], (None, None))
        self.assertEqual(tuple(released), attempts[0])
        self.assertEqual(len(released), 2)
        self.assertEqual(len(failure.exception.receipt["descriptor_close_errors"]), 1)
        self.assertFalse((self.root / "result" / "transaction.json").exists())

    def test_row_failure_stays_primary_when_both_descriptor_closes_fail(self):
        archive, size = self.archive()
        original = reader._close_descriptors
        real_close = os.close
        released = []
        row_read_started = False

        def read_rows(_descriptor):
            nonlocal row_read_started
            row_read_started = True
            raise ValueError("primary-row-control")

        def close_owned(descriptors):
            if not row_read_started:
                return original(descriptors)

            def close(descriptor):
                real_close(descriptor)
                released.append(descriptor)
                raise OSError("secondary-close-control")

            with patch.object(reader.os, "close", side_effect=close):
                return original(descriptors)

        with (
            patch.object(reader, "_read_rows", side_effect=read_rows),
            patch.object(reader, "_close_descriptors", side_effect=close_owned),
            self.assertRaisesRegex(
                reader.DatasetReadError, "primary-row-control"
            ) as failure,
        ):
            self.fit(archive, size)
        self.assertEqual(len(released), 2)
        self.assertEqual(len(set(released)), 2)
        self.assertEqual(len(failure.exception.receipt["descriptor_close_errors"]), 2)

    def test_decoder_close_failure_cannot_issue_scaler(self):
        archive, size = self.archive()
        real_open, real_close = os.open, os.close
        target = None
        attempts = []

        def open_target(path, *args, **kwargs):
            nonlocal target
            descriptor = real_open(path, *args, **kwargs)
            if path == self.root / "result" / "decoded.h5":
                target = descriptor
            return descriptor

        def close(descriptor):
            real_close(descriptor)
            if descriptor == target:
                attempts.append(descriptor)
                raise OSError("target-close-control")

        with (
            patch.object(reader.os, "open", side_effect=open_target),
            patch.object(reader.os, "close", side_effect=close),
            patch.object(reader, "_fit_owned_rows") as issuer,
            self.assertRaisesRegex(
                reader.DatasetReadError, "target-close-control"
            ) as failure,
        ):
            self.fit(archive, size)
        issuer.assert_not_called()
        self.assertEqual(attempts, [target])
        decode = failure.exception.receipt["decode"]
        self.assertEqual(decode["decoded_bytes"], size)
        self.assertEqual(decode["cleanup"]["status"], "confirmed")
        self.assertEqual(decode["resource_close_errors"][0]["resource"], "target")
        self.assertFalse((self.root / "result" / "scaler.json").exists())
        self.assertFalse((self.root / "result" / "transaction.json").exists())
        self.assertEqual(
            self.fit(archive, size, name="valid-after-close-fault").receipt()[
                "retained_rows"
            ],
            3,
        )

    def snapshot_fault(self, operation, close_failures):
        archive, size = self.archive()
        original = reader._snapshot
        real_open, real_close, real_read = os.open, os.close, os.read
        real_write = reader._write_all
        active = False
        owned = {}
        attempts = []

        def snapshot(*args):
            nonlocal active
            active = True
            try:
                return original(*args)
            finally:
                active = False

        def open_owned(path, *args, **kwargs):
            descriptor = real_open(path, *args, **kwargs)
            if active:
                if path == archive:
                    owned["source"] = descriptor
                elif path == self.root / "result" / "verified.h5.zst":
                    owned["target"] = descriptor
            return descriptor

        def close(descriptor):
            real_close(descriptor)
            if active:
                name = next(key for key, value in owned.items() if value == descriptor)
                attempts.append(name)
                if name in close_failures:
                    raise OSError(f"snapshot-{name}-close-control")

        def read(descriptor, count):
            if active and operation == "read" and descriptor == owned.get("source"):
                raise OSError("primary-snapshot-read-control")
            return real_read(descriptor, count)

        def write(descriptor, data):
            if active and operation == "copy":
                raise OSError("primary-snapshot-copy-control")
            return real_write(descriptor, data)

        reason = (
            f"primary-snapshot-{operation}-control"
            if operation
            else "snapshot-target-close-control"
        )
        with (
            patch.object(reader, "_snapshot", side_effect=snapshot),
            patch.object(reader.os, "open", side_effect=open_owned),
            patch.object(reader.os, "close", side_effect=close),
            patch.object(reader.os, "read", side_effect=read),
            patch.object(reader, "_write_all", side_effect=write),
            patch.object(reader, "_decode") as decoder,
            self.assertRaisesRegex(reader.DatasetReadError, reason) as failure,
        ):
            self.fit(archive, size)
        decoder.assert_not_called()
        self.assertEqual(attempts, ["target", "source"])
        receipt = failure.exception.receipt["compressed_snapshot"]
        self.assertIn(reason, receipt["primary_failure"])
        self.assertEqual(len(receipt["descriptor_close_errors"]), len(close_failures))
        persisted = json.loads((self.root / "result" / "failure.json").read_text())
        self.assertEqual(persisted["compressed_snapshot"], receipt)
        self.assertIn(reason, persisted["primary_failure"])
        self.assertFalse((self.root / "result" / "decoded.h5").exists())
        self.assertEqual(
            self.fit(archive, size, name="valid-after-snapshot-fault").receipt()[
                "retained_rows"
            ],
            3,
        )

    def test_snapshot_read_error_preserves_both_close_errors(self):
        self.snapshot_fault("read", ("target", "source"))

    def test_snapshot_copy_error_preserves_both_close_errors(self):
        self.snapshot_fault("copy", ("target", "source"))

    def test_snapshot_lone_close_error_prevents_decode(self):
        self.snapshot_fault(None, ("target",))

    def test_retained_snapshot_probe_error_closes_its_descriptor(self):
        archive, size = self.archive()
        real_open, real_read, real_close = reader._open_regular, os.read, os.close
        retained = None
        attempts = []

        def open_retained(path, limit):
            nonlocal retained
            result = real_open(path, limit)
            if path == self.root / "result" / "verified.h5.zst":
                retained = result[0]
            return result

        def read(descriptor, count):
            if descriptor == retained:
                raise OSError("primary-retained-read-control")
            return real_read(descriptor, count)

        def close(descriptor):
            real_close(descriptor)
            if descriptor == retained:
                attempts.append(descriptor)
                raise OSError("retained-close-control")

        with (
            patch.object(reader, "_open_regular", side_effect=open_retained),
            patch.object(reader.os, "read", side_effect=read),
            patch.object(reader.os, "close", side_effect=close),
            patch.object(reader, "_decode") as decoder,
            self.assertRaisesRegex(
                reader.DatasetReadError, "primary-retained-read"
            ) as failure,
        ):
            self.fit(archive, size)
        decoder.assert_not_called()
        self.assertEqual(attempts, [retained])
        receipt = failure.exception.receipt["compressed_snapshot"]
        self.assertIn("primary-retained-read", receipt["primary_failure"])
        self.assertEqual(
            receipt["descriptor_close_errors"], ["OSError: retained-close-control"]
        )
        self.assertEqual(
            self.fit(archive, size, name="valid-after-probe-fault").receipt()[
                "retained_rows"
            ],
            3,
        )


@unittest.skipUnless(
    sys.platform == "darwin",
    "Actual lifecycle controls require the supported Darwin decoder profile",
)
class DecoderProcessControls(unittest.TestCase):
    """Actual owned process behavior with explicit synthetic producers, no HDF5 authority."""

    def setUp(self):
        retained = os.environ.get("LEWM_DATASET_CONTROL_ROOT")
        self.root = Path(
            tempfile.mkdtemp(prefix=f"{self._testMethodName}-", dir=retained)
        )
        if retained is None:
            import shutil

            self.addCleanup(shutil.rmtree, self.root)

    def run_producer(
        self, source, *, expected=4, timeout=0.25, writer=None, close_failures=()
    ):
        raw = self.root / "input"
        raw.write_bytes(b"synthetic process control")
        descriptor = os.open(raw, os.O_RDONLY)
        original = subprocess.Popen
        real_killpg = os.killpg
        real_open, real_close = os.open, os.close
        process = None
        target = None
        reap_started = False
        events = []

        class PipeCloseControl:
            def __init__(self, stream, name):
                self.stream, self.name = stream, name

            def __getattr__(self, name):
                return getattr(self.stream, name)

            def close(self):
                events.append({"operation": "close", "resource": self.name})
                self.stream.close()
                if self.name in close_failures:
                    raise OSError(f"{self.name}-close-control")

        def open_target(path, *args, **kwargs):
            nonlocal target
            descriptor = real_open(path, *args, **kwargs)
            if path == self.root / "decoded":
                target = descriptor
            return descriptor

        def close(descriptor):
            real_close(descriptor)
            if descriptor == target:
                events.append({"operation": "close", "resource": "target"})
                if "target" in close_failures:
                    raise OSError("target-close-control")

        def launch(_command, **kwargs):
            nonlocal process, reap_started
            process = original([sys.executable, "-c", source], **kwargs)
            process.stdout = PipeCloseControl(process.stdout, "stdout")
            process.stderr = PipeCloseControl(process.stderr, "stderr")
            process.poll = Mock(
                side_effect=AssertionError("poll would reap the owned leader")
            )
            real_wait = process.wait

            def reap(**arguments):
                nonlocal reap_started
                reap_started = True
                events.append({"operation": "terminal_wait"})
                return real_wait(**arguments)

            process.wait = reap
            return process

        def group_signal(pid, sig):
            if sig:
                self.assertFalse(reap_started, "A signal followed terminal wait")
                events.append({"operation": "signal", "signal": int(sig)})
            return real_killpg(pid, sig)

        receipt = {"scope": "synthetic_process_control_only", "process_events": events}
        try:
            with (
                patch.object(reader.subprocess, "Popen", side_effect=launch),
                patch.object(reader, "DECODE_SECONDS", timeout),
                patch.object(reader.os, "killpg", side_effect=group_signal),
                patch.object(reader.os, "open", side_effect=open_target),
                patch.object(reader.os, "close", side_effect=close),
            ):
                if writer:
                    with patch.object(reader, "_write_all", side_effect=writer):
                        result = reader._decode(
                            descriptor, self.root / "decoded", expected, receipt
                        )
                else:
                    result = reader._decode(
                        descriptor, self.root / "decoded", expected, receipt
                    )
            os.close(result)
        finally:
            os.close(descriptor)
            self.last_receipt = receipt
            (self.root / "process-receipt.json").write_text(
                json.dumps(receipt, indent=2) + "\n"
            )
            if process is not None:
                process.poll.assert_not_called()
                self.assertIsNotNone(process.returncode)
                self.assertFalse(reader._group_alive(process.pid))

    def test_unreaped_zombie_keeps_identity_until_terminal_cleanup(self):
        wait_child, identity = reader._prepare_waitid()
        process = subprocess.Popen(
            [sys.executable, "-c", "pass"],
            start_new_session=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        owner = reader._OwnedDecoder(process, wait_child)
        receipt = {"waitid_observer": identity}
        try:
            status = reader._wait_exited(owner, reader.time.monotonic() + 3)
            self.assertEqual(status.si_pid, process.pid)
            self.assertIsNone(process.returncode)
            self.assertEqual(owner.observe(), status)
            # EPERM is retained as group presence/unknown until the later reap.
            self.assertTrue(reader._group_alive(process.pid))
        finally:
            receipt.update(reader._cleanup(owner))
            (self.root / "zombie-receipt.json").write_text(
                json.dumps(receipt, indent=2) + "\n"
            )
        self.assertEqual(receipt["status"], "confirmed")
        self.assertTrue(receipt["group_absent"] and receipt["leader_reaped"])
        with patch.object(reader.os, "killpg") as group:
            with self.assertRaises(ChildProcessError):
                owner.signal(signal.SIGKILL)
            group.assert_not_called()

    def test_native_preparation_failures_cannot_create_a_child(self):
        for mutation in (
            patch.object(reader.ctypes, "CDLL", return_value=SimpleNamespace()),
            patch.object(reader.ctypes, "sizeof", return_value=4),
            patch.object(reader.signal, "getsignal", return_value=signal.SIG_IGN),
        ):
            with mutation, patch.object(reader.subprocess, "Popen") as launch:
                with self.assertRaises((ValueError, AttributeError)):
                    reader._decode(12345, self.root / "absent-output", 1, {})
                launch.assert_not_called()
                self.assertFalse((self.root / "absent-output").exists())
        observe, identity = reader._prepare_waitid()
        self.assertEqual(identity["api"], "darwin-arm64-libSystem-waitid-v1")
        with self.assertRaises(ChildProcessError):
            observe(os.getpid())

    def test_successful_output_reaps_owned_process(self):
        self.run_producer("import os; os.write(1,b'abcd')", timeout=3)

    def test_timeout_terminates_and_kills_term_ignoring_process(self):
        with self.assertRaises(TimeoutError):
            self.run_producer(
                "import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); time.sleep(30)"
            )
        self.assertEqual(self.last_receipt["cleanup"]["returncode"], -9)

    def test_descendant_cannot_outlive_successful_leader(self):
        source = "import os,time; p=os.fork(); os.close(1); os.close(2); time.sleep(30) if p==0 else None"
        with self.assertRaisesRegex(ValueError, "descendants"):
            self.run_producer(source, expected=1, timeout=3)

    def test_output_failure_preserves_primary_and_cleans_process(self):
        with self.assertRaisesRegex(OSError, "write-control"):
            self.run_producer(
                "import os,time; os.write(1,b'abcd'); time.sleep(30)",
                timeout=3,
                writer=OSError("write-control"),
            )

    def test_stderr_cap_fails_closed(self):
        with self.assertRaisesRegex(ValueError, "stderr"):
            self.run_producer("import os; os.write(2,b'x'*70000)", timeout=3)

    def assert_all_decoder_closes(self):
        events = self.last_receipt["process_events"]
        self.assertEqual(
            [row["resource"] for row in events if row["operation"] == "close"],
            ["stdout", "stderr", "target"],
        )
        self.assertEqual(self.last_receipt["cleanup"]["status"], "confirmed")
        for field in ("decoded_bytes", "decoded_sha256", "stderr", "elapsed_seconds"):
            self.assertIn(field, self.last_receipt)

    def test_stdout_close_failure_still_closes_stderr_and_target(self):
        with self.assertRaisesRegex(OSError, "stdout-close-control"):
            self.run_producer(
                "import os; os.write(1,b'abcd')", timeout=3, close_failures=("stdout",)
            )
        self.assert_all_decoder_closes()
        self.assertEqual(self.last_receipt["decoded_bytes"], 4)
        self.assertEqual(
            self.last_receipt["resource_close_errors"][0]["resource"], "stdout"
        )

    def test_stderr_close_failure_still_closes_target(self):
        with self.assertRaisesRegex(OSError, "stderr-close-control"):
            self.run_producer(
                "import os; os.write(1,b'abcd')", timeout=3, close_failures=("stderr",)
            )
        self.assert_all_decoder_closes()
        self.assertEqual(
            self.last_receipt["resource_close_errors"][0]["resource"], "stderr"
        )

    def test_target_close_failure_retains_primary_output_error(self):
        with self.assertRaisesRegex(OSError, "write-control"):
            self.run_producer(
                "import os,time; os.write(1,b'abcd'); time.sleep(30)",
                timeout=3,
                writer=OSError("write-control"),
                close_failures=("target",),
            )
        self.assert_all_decoder_closes()
        self.assertEqual(self.last_receipt["primary_failure"], "OSError: write-control")
        self.assertEqual(
            self.last_receipt["resource_close_errors"][0]["resource"], "target"
        )

    def test_all_close_errors_retain_primary_and_complete_observations(self):
        with self.assertRaisesRegex(OSError, "write-control"):
            self.run_producer(
                "import os,time; os.write(1,b'abcd'); time.sleep(30)",
                timeout=3,
                writer=OSError("write-control"),
                close_failures=("stdout", "stderr", "target"),
            )
        self.assert_all_decoder_closes()
        self.assertEqual(self.last_receipt["primary_failure"], "OSError: write-control")
        self.assertEqual(
            [row["resource"] for row in self.last_receipt["resource_close_errors"]],
            ["stdout", "stderr", "target"],
        )


if __name__ == "__main__":
    unittest.main()
