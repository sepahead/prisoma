"""Shared metadata reader owns every descriptor and preserves cleanup failures."""

import errno
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "m1_campaign.py"
SPEC = importlib.util.spec_from_file_location("m1_file_subject", SCRIPT)
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


class FileControls(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="m1-file-controls-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.path = self.root / "regular"
        self.path.write_bytes(b"original bytes")
        self.opened = []
        self.original_open = os.open
        self.original_close = os.close
        self.original_fdopen = os.fdopen

    def tracked_open(self, *args, **kwargs):
        descriptor = self.original_open(*args, **kwargs)
        self.opened.append(descriptor)
        return descriptor

    def assert_closed(self):
        self.assertEqual(len(self.opened), 1)
        with self.assertRaises(OSError) as caught:
            os.fstat(self.opened[0])
        self.assertEqual(caught.exception.errno, errno.EBADF)

    def test_regular_bytes_are_unchanged_and_descriptor_closes(self):
        with patch.object(m.os, "open", side_effect=self.tracked_open):
            self.assertEqual(m.read(self.path), b"original bytes")
        self.assert_closed()

    def test_directory_construction_failure_closes_descriptor(self):
        with patch.object(m.os, "open", side_effect=self.tracked_open):
            with self.assertRaises(IsADirectoryError):
                m.read(self.root)
        self.assert_closed()

    def test_nonregular_admission_failure_closes_descriptor(self):
        pipe = self.root / "pipe"
        os.mkfifo(pipe, 0o600)
        with patch.object(m.os, "open", side_effect=self.tracked_open):
            with self.assertRaisesRegex(m.CampaignError, "file_bound"):
                m.read(pipe)
        self.assert_closed()

    def test_construction_cancellation_and_descriptor_failure_remain_originals(self):
        primary = KeyboardInterrupt("synthetic constructor cancellation")
        cleanup = OSError("synthetic descriptor cleanup")

        def close(descriptor):
            self.original_close(descriptor)
            raise cleanup

        with (
            patch.object(m.os, "open", side_effect=self.tracked_open),
            patch.object(m.os, "fdopen", side_effect=primary),
            patch.object(m.os, "close", side_effect=close),
        ):
            with self.assertRaises(BaseExceptionGroup) as caught:
                m.read(self.path)
        self.assertEqual(caught.exception.exceptions, (primary, cleanup))
        self.assert_closed()

    def test_read_and_both_close_failures_preserve_order_and_originals(self):
        primary = OSError("synthetic read failure")
        stream_error = OSError("synthetic stream cleanup")
        descriptor_error = OSError("synthetic descriptor cleanup")
        events = []

        class FailedStream:
            def __init__(self, stream):
                self.stream = stream

            def fileno(self):
                return self.stream.fileno()

            def read(self, maximum):
                events.append("read")
                raise primary

            def close(self):
                events.append("stream_close")
                self.stream.close()
                raise stream_error

        def fdopen(descriptor, *args, **kwargs):
            self.assertIs(kwargs["closefd"], False)
            return FailedStream(self.original_fdopen(descriptor, *args, **kwargs))

        def close(descriptor):
            events.append("descriptor_close")
            self.original_close(descriptor)
            raise descriptor_error

        with (
            patch.object(m.os, "open", side_effect=self.tracked_open),
            patch.object(m.os, "fdopen", side_effect=fdopen),
            patch.object(m.os, "close", side_effect=close),
        ):
            with self.assertRaises(BaseExceptionGroup) as caught:
                m.read(self.path)
        self.assertEqual(
            caught.exception.exceptions, (primary, stream_error, descriptor_error)
        )
        self.assertEqual(events, ["read", "stream_close", "descriptor_close"])
        self.assert_closed()

    def test_successful_read_cannot_hide_descriptor_cleanup_failure(self):
        cleanup = OSError("synthetic cleanup after successful read")

        def close(descriptor):
            self.original_close(descriptor)
            raise cleanup

        with (
            patch.object(m.os, "open", side_effect=self.tracked_open),
            patch.object(m.os, "close", side_effect=close),
        ):
            with self.assertRaises(OSError) as caught:
                m.read(self.path)
        self.assertIs(caught.exception, cleanup)
        self.assert_closed()


if __name__ == "__main__":
    unittest.main()
