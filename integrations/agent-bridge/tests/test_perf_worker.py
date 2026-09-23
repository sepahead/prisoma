"""Actual framing controls for the maintained timing wrapper; no simulator launch."""

import importlib.util
import io
from pathlib import Path
import struct
import unittest

from ncp_local import wire

p = Path(__file__).resolve().parents[1] / "scripts" / "perf_worker.py"
s = importlib.util.spec_from_file_location("perf_worker", p)
w = importlib.util.module_from_spec(s)
s.loader.exec_module(w)


class ProbeControls(unittest.TestCase):
    def test_minimal_and_detailed_keep_original_framing(self):
        for detailed in (False, True):
            with self.subTest(detailed=detailed):
                trace = w.m.Trace(w.m.Clock())
                trace.tick = 7
                probe = w.FrameProbe(trace, 2, detailed)
                writer = io.BytesIO()
                response = b"original response"
                reader = io.BytesIO(struct.pack(">I", len(response)) + response)
                request = b"original request"
                probe.install()
                try:
                    result = wire.write_local_frame(writer, request)
                    self.assertIsNone(result)
                    self.assertEqual(wire.read_local_frame(reader), response)
                    self.assertEqual(
                        writer.getvalue(), struct.pack(">I", len(request)) + request
                    )
                    self.assertIs(probe.frames[0][2], request)
                    self.assertEqual(probe.frames[1], ("response", 7, response))
                    before = writer.getvalue()
                    with self.assertRaises(w.m.MeasurementError):
                        wire.write_local_frame(writer, b"excess")
                    self.assertEqual(writer.getvalue(), before)
                finally:
                    probe.restore()
                self.assertTrue(trace.validate())
                self.assertEqual(len(trace.rows), 2 if detailed else 0)

    def test_original_exception_identity_survives(self):
        original = wire.read_local_frame
        failure = OSError("read failure")

        def failing(reader, *, deadline=None):
            raise failure

        wire.read_local_frame = failing
        trace = w.m.Trace(w.m.Clock())
        probe = w.FrameProbe(trace, 2, True)
        try:
            probe.install()
            try:
                wire.read_local_frame(io.BytesIO(), deadline=12)
            except OSError as error:
                self.assertIs(error, failure)
            else:
                self.fail("original failure missing")
            self.assertTrue(trace.validate())
            self.assertEqual(trace.rows[0]["outcome"], "exception")
        finally:
            probe.restore()
            wire.read_local_frame = original

    def test_eof_remains_absent_and_does_not_become_a_frame(self):
        trace = w.m.Trace(w.m.Clock())
        probe = w.FrameProbe(trace, 2, True)
        probe.install()
        try:
            wire.write_local_frame(io.BytesIO(), b"request")
            self.assertIsNone(wire.read_local_frame(io.BytesIO()))
        finally:
            probe.restore()
        self.assertIsNone(probe.frames[-1][2])

    def test_invalid_frame_still_rejects_through_installed_framer(self):
        probe = w.FrameProbe(w.m.Trace(w.m.Clock()), 2, False)
        probe.install()
        try:
            with self.assertRaises(Exception):
                wire.write_local_frame(io.BytesIO(), b"")
            with self.assertRaises(Exception):
                wire.read_local_frame(io.BytesIO(struct.pack(">I", 65537)))
        finally:
            probe.restore()

    def test_byte_capacity_reserves_response_before_request_effects(self):
        probe = w.FrameProbe(w.m.Trace(w.m.Clock()), 8, False, byte_limit=65537)
        writer = io.BytesIO()
        probe.install()
        try:
            wire.write_local_frame(writer, b"a")
            wire.read_local_frame(io.BytesIO(struct.pack(">I", 1) + b"b"))
            before = writer.getvalue()
            with self.assertRaises(w.m.MeasurementError):
                wire.write_local_frame(writer, b"c")
            self.assertEqual(writer.getvalue(), before)
            self.assertEqual(probe.retained_bytes, 2)
            self.assertEqual(len(probe.frames), 2)
        finally:
            probe.restore()


if __name__ == "__main__":
    unittest.main()
