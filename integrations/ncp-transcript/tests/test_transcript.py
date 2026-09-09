"""Synthetic protocol controls; no simulator or scientific outcome is inferred."""

from dataclasses import dataclass, replace
import hashlib
import os
from pathlib import Path
import socket
import struct
import tempfile
import threading
import time
import unittest
from unittest.mock import patch
from uuid import UUID

from ncp_local import modular_wire as w
from ncp_local import wire as framing
from ncp_local.modular_buffer import BufferBinding
from ncp_local.modular_client import Client
from ncp_local.modular_owner import profile_digest

from prisoma_ncp_transcript import CaptureError, Journal, Peer, verify
from prisoma_ncp_transcript import transcript as t


@dataclass(frozen=True, slots=True)
class Payload:
    text: str


class Contract:
    """An explicitly synthetic installed application, with closed payloads."""

    @staticmethod
    def descriptor():
        return b'{"schema":"prisoma.transcript.synthetic.v1"}'

    @staticmethod
    def allows(operation):
        return operation in (
            w.Name.PREPARE,
            w.Name.APPLICATION,
            w.Name.FINISH,
            w.Name.ABORT,
        )

    @staticmethod
    def decode(value):
        text = w.closed(value, {"text"})["text"]
        if type(text) is not str or len(text) > 50_000:
            raise w.ModularError("wire")
        return Payload(text)

    decode_prepare = decode_command = decode_finish = decode_result = (
        decode_terminal
    ) = decode

    @staticmethod
    def reject(_):
        raise w.ModularError("wire")

    decode_import = decode_metadata = decode_imported = reject

    @staticmethod
    def check_input(operation):
        if type(operation) is not w.Abort:
            Contract.decode(w.immutable_value(operation.data))

    @staticmethod
    def check_response(operation, body, context):
        pass

    @staticmethod
    def check_import_metadata(descriptor, metadata):
        raise w.ModularError("wire")


def peer(index=0, run=1):
    return Peer(
        BufferBinding(
            profile_digest(),
            w.typed_digest(w.PROFILE_DOMAIN, w.parse(Contract.descriptor())),
            str(UUID(int=run, version=4)),
            str(UUID(int=index + 200, version=4)),
            str(UUID(int=index + 100, version=4)),
        ),
        Contract,
    )


class Producer:
    """Construct responses from the submitted operation, using the public codec."""

    def __init__(self, selected):
        self.peer = selected
        self.client = Client(selected.binding, selected.contract)
        self.frames = []

    def answer(self, raw, outcome=w.Outcome.COMMITTED):
        request = w.Request.decode(raw, self.peer.binding, self.peer.contract)
        command = request.command
        if type(command) is w.Ack:
            response = w.Response(
                request.binding,
                request.sequence,
                w.Name.ACK,
                request.request_digest,
                w.Outcome.ACKNOWLEDGED,
                w.Code.RELEASED,
                w.Body(
                    w.BodyKind.ACKNOWLEDGED,
                    w.AckStamp(
                        request.sequence,
                        command.original_request_digest,
                        command.result_digest,
                    ),
                ),
            )
        else:
            op = command.operation
            kind = {
                w.Prepare: w.BodyKind.PREPARED,
                w.Application: w.BodyKind.APPLICATION,
                w.Finish: w.BodyKind.FINISHED,
                w.Abort: w.BodyKind.ABORTED,
            }[type(op)]
            data = None if type(op) is w.Abort else op.data
            code = w.Code.OK
            if outcome is w.Outcome.REJECTED:
                kind, data, code = w.BodyKind.REJECTED, None, w.Code.INVALID
            if outcome is w.Outcome.INDETERMINATE:
                kind, data, code = w.BodyKind.INDETERMINATE, "backend", w.Code.UNKNOWN
            response = w.Response(
                request.binding,
                request.sequence,
                w.operation_name(op),
                request.request_digest,
                outcome,
                code,
                w.Body(kind, data),
            )
        return response.encode()

    def exchange(self, journal, raw, response=None):
        reply = self.answer(raw) if response is None else response
        with (
            patch.object(framing, "write_local_frame") as send,
            patch.object(framing, "read_local_frame", return_value=reply),
        ):
            actual = journal.exchange(
                self.peer.binding.endpoint_id,
                raw,
                None,
                None,
                deadline=time.monotonic() + 10,
            )
            send.assert_called_once()
        self.frames.extend((raw, actual))
        return actual

    def operation(
        self, journal, operation, *, acknowledge=True, outcome=w.Outcome.COMMITTED
    ):
        raw = self.client.begin(operation)
        result = self.exchange(journal, raw, self.answer(raw, outcome))
        self.client.observe(result)
        if acknowledge and outcome is w.Outcome.COMMITTED:
            self.ack(journal)
        return raw, result

    def ack(self, journal):
        raw = self.client.acknowledgement()
        result = self.exchange(journal, raw)
        self.client.observe_acknowledgement(result)


def records(path):
    """Independent framing reader used only to inspect test output bytes."""
    raw = path.read_bytes()
    offset = 8
    rows = []
    while offset < len(raw):
        kind, ordinal, length = struct.unpack(">BQI", raw[offset : offset + 13])
        rows.append((kind, ordinal, raw[offset + 13 : offset + 13 + length]))
        offset += 13 + length + 32
    return rows


def rewrite(path, rows):
    raw = bytearray(b"PNCPT\x00\x01\n")
    previous = bytes(32)
    for ordinal, (kind, _, payload) in enumerate(rows):
        prefix = struct.pack(">BQI", kind, ordinal, len(payload))
        previous = hashlib.sha256(
            b"prisoma.ncp.transcript.record.v1\0" + previous + prefix + payload
        ).digest()
        raw.extend(prefix + payload + previous)
    path.write_bytes(raw)


class TranscriptTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / "capture.ncp"
        self.peers = (peer(),)

    def journal(self, maximum=32, quota=8 * 1024**2, peers=None):
        journal = Journal(
            self.path,
            self.peers if peers is None else peers,
            max_exchanges=maximum,
            quota_bytes=quota,
        )
        self.addCleanup(journal.close)
        return journal

    def completed(self, *, peers=None, large=False):
        selected = self.peers if peers is None else peers
        journal = self.journal(maximum=64, quota=16 * 1024**2, peers=selected)
        originals = []
        for p in selected:
            producer = Producer(p)
            producer.operation(journal, w.Prepare(Payload("start")))
            if large:
                for _ in range(8):
                    producer.operation(journal, w.Application(Payload("A" * 50_000)))
            producer.operation(journal, w.Finish(Payload("done")))
            originals.extend(producer.frames)
        result = journal.finish()
        self.assertEqual(verify(self.path, selected), result)
        self.assertFalse(result.application_completion_validated)
        self.assertFalse(result.scientific_validation)
        return result, originals

    def test_one_peer_needs_no_other_project(self):
        result, _ = self.completed()
        self.assertEqual(
            (result.peer_count, result.finished_peers, result.exchange_pairs), (1, 1, 4)
        )

    def test_sixteen_optional_peers_have_distinct_terminal_chains(self):
        result, _ = self.completed(peers=tuple(peer(i) for i in range(16)))
        self.assertEqual(
            (result.peer_count, result.finished_peers, result.exchange_pairs),
            (16, 16, 64),
        )

    def test_large_payload_sequence_preserves_every_original_frame(self):
        result, originals = self.completed(large=True)
        stored = [
            payload[1:] for kind, _, payload in records(self.path) if kind in (2, 3)
        ]
        self.assertEqual(stored, originals)
        self.assertGreater(result.journal_bytes, 800_000)

    def test_peer_roster_rejects_empty_duplicate_cross_run_and_seventeenth_peer(self):
        for peers in (
            (),
            (peer(), peer()),
            (peer(), peer(1, 2)),
            tuple(peer(i) for i in range(17)),
        ):
            with (
                self.subTest(size=len(peers)),
                self.assertRaises((CaptureError, w.ModularError)),
            ):
                self.journal(peers=peers)
            self.assertFalse(self.path.exists())

    def test_invalid_quota_and_exchange_limit_leave_no_file(self):
        for maximum, quota in (
            (True, 8 * 1024**2),
            (0, 8 * 1024**2),
            (8191, t.MAX_BYTES),
            (1, True),
            (1, 100),
            (1, t.MAX_BYTES + 1),
        ):
            with (
                self.subTest(maximum=maximum, quota=quota),
                self.assertRaises(CaptureError),
            ):
                self.journal(maximum, quota)
            self.assertFalse(self.path.exists())

    def test_maximum_slot_arithmetic_has_no_off_by_one(self):
        replay = t._Replay(self.peers)
        upper = t._header(replay, 4, t.MAX_BYTES)
        # Quota digits can shorten the header, so solve its exact serialized fixed point.
        quota = 8 + 45 + len(upper) + 4 * (2 * (45 + 1 + 65_536)) + 45 + 1024
        for _ in range(4):
            payload = w.encode(
                {
                    "schema": "prisoma.ncp.transcript.v1",
                    "max_exchanges": 4,
                    "quota_bytes": quota,
                    "peers": replay.roster(),
                }
            )
            quota = 8 + 45 + len(payload) + 4 * 131_164 + 1069
        with self.assertRaises(CaptureError):
            self.journal(4, quota - 1)
        journal = self.journal(4, quota)
        producer = Producer(self.peers[0])
        producer.operation(journal, w.Prepare(Payload("start")))
        producer.operation(journal, w.Finish(Payload("done")))
        self.assertEqual(journal.finish(), verify(self.path, self.peers))

    def test_header_and_terminal_checks_survive_rehashed_corruption(self):
        self.completed()
        original = records(self.path)
        for value in (True, 4.0, 3, 5, None):
            changed = original.copy()
            changed[-1] = (4, changed[-1][1], w.encode({"exchange_pairs": value}))
            rewrite(self.path, changed)
            with (
                self.subTest(value=value),
                self.assertRaises((CaptureError, w.ModularError)),
            ):
                verify(self.path, self.peers)
        rewrite(self.path, original)
        self.assertEqual(verify(self.path, self.peers).exchange_pairs, 4)

    def test_missing_ack_and_reordered_frames_fail_even_when_rehashed(self):
        self.completed()
        original = records(self.path)
        for changed in (
            original[:3] + original[5:],
            original[:1] + original[2:3] + original[1:2] + original[3:],
        ):
            rewrite(self.path, changed)
            with self.assertRaises((CaptureError, w.ModularError)):
                verify(self.path, self.peers)

    def test_truncation_and_trailing_record_never_look_complete(self):
        self.completed()
        raw = self.path.read_bytes()
        cuts = {0, 1, 7, 8, 9, len(raw) - 1, len(raw) - 32}
        offset = 8
        for _, _, payload in records(self.path):
            cuts.update((offset, offset + 1, offset + 13, offset + 13 + len(payload)))
            offset += 45 + len(payload)
        for cut in sorted(cuts):
            self.path.write_bytes(raw[:cut])
            with (
                self.subTest(cut=cut),
                self.assertRaises((CaptureError, w.ModularError)),
            ):
                verify(self.path, self.peers)
        self.path.write_bytes(raw + b"x")
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)

    def test_bit_corruption_is_detected_in_each_record(self):
        self.completed()
        raw = self.path.read_bytes()
        offset = 8
        for _, _, payload in records(self.path):
            for delta in (0, 1, 9, 13, 44 + len(payload)):
                changed = bytearray(raw)
                changed[offset + delta] ^= 1
                self.path.write_bytes(changed)
                with self.assertRaises((CaptureError, w.ModularError)):
                    verify(self.path, self.peers)
            offset += 45 + len(payload)

    def test_unbounded_length_is_rejected_before_payload_read(self):
        self.completed()
        with self.path.open("r+b") as file:
            file.seek(8 + 9)
            file.write(struct.pack(">I", 0xFFFFFFFF))
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)

    def test_verifier_requires_same_installed_descriptor(self):
        self.completed()
        changed = replace(
            self.peers[0],
            binding=replace(
                self.peers[0].binding, generation=str(UUID(int=900, version=4))
            ),
        )
        with self.assertRaises((CaptureError, w.ModularError)):
            verify(self.path, (changed,))

    def test_filesystem_admission_rejects_existing_links_fifo_and_shared_file(self):
        self.path.write_text("keep")
        with self.assertRaises(FileExistsError):
            self.journal()
        self.assertEqual(self.path.read_text(), "keep")
        link = self.path.with_name("link")
        link.symlink_to(self.path)
        with self.assertRaises(OSError):
            Journal(link, self.peers, max_exchanges=4, quota_bytes=1024**2)
        with self.assertRaises(OSError):
            verify(link, self.peers)
        fifo = self.path.with_name("fifo")
        os.mkfifo(fifo, 0o600)
        with self.assertRaises(CaptureError):
            verify(fifo, self.peers)
        self.path.unlink()
        self.completed()
        os.chmod(self.path, 0o644)
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)
        os.chmod(self.path, 0o600)
        hardlink = self.path.with_name("hardlink")
        os.link(self.path, hardlink)
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)
        hardlink.unlink()
        self.assertEqual(verify(self.path, self.peers).store_completion, "complete")

    def test_abort_and_plain_close_have_different_meanings(self):
        journal = self.journal()
        result = journal.abort()
        self.assertEqual(result.store_completion, "aborted")
        self.assertEqual(result, verify(self.path, self.peers))
        self.path.unlink()
        self.journal().close()
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)

    def test_finish_without_acknowledgement_is_incomplete(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        producer.operation(journal, w.Prepare(Payload("start")))
        producer.operation(journal, w.Finish(Payload("done")), acknowledge=False)
        with self.assertRaises(CaptureError):
            journal.finish()
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)

    def test_rejected_command_is_retained_and_does_not_advance_sequence(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        producer.operation(journal, w.Prepare(Payload("start")))
        before = producer.client.next_sequence
        producer.operation(
            journal, w.Application(Payload("reject")), outcome=w.Outcome.REJECTED
        )
        self.assertEqual(producer.client.next_sequence, before)
        producer.operation(journal, w.Finish(Payload("done")))
        self.assertEqual(journal.finish().exchange_pairs, 5)
        self.assertEqual(verify(self.path, self.peers).exchange_pairs, 5)

    def test_result_query_retains_the_original_committed_response(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        _, response = producer.operation(
            journal, w.Prepare(Payload("start")), acknowledge=False
        )
        raw = producer.client.result_query()
        result = producer.exchange(journal, raw, response)
        producer.client.observe_query(result)
        producer.ack(journal)
        producer.operation(journal, w.Finish(Payload("done")))
        self.assertEqual(journal.finish().exchange_pairs, 5)
        self.assertEqual(verify(self.path, self.peers).exchange_pairs, 5)

    def test_peer_abort_cannot_be_followed_by_another_execute(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        producer.operation(journal, w.Prepare(Payload("start")))
        producer.operation(journal, w.Abort())
        with self.assertRaises(CaptureError):
            producer.operation(journal, w.Finish(Payload("impossible")))
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)

    def test_indeterminate_result_is_retained_as_aborted_evidence(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        producer.operation(journal, w.Prepare(Payload("start")))
        producer.operation(
            journal, w.Application(Payload("unknown")), outcome=w.Outcome.INDETERMINATE
        )
        result = journal.abort()
        self.assertEqual(result.store_completion, "aborted")
        self.assertEqual(verify(self.path, self.peers), result)

    def test_request_sync_failure_prevents_any_dispatch(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        raw = producer.client.begin(w.Prepare(Payload("start")))
        with (
            patch.object(os, "fsync", side_effect=OSError("synthetic sync failure")),
            patch.object(framing, "write_local_frame") as send,
        ):
            with self.assertRaises(OSError):
                journal.exchange(
                    self.peers[0].binding.endpoint_id,
                    raw,
                    None,
                    None,
                    deadline=time.monotonic() + 10,
                )
            send.assert_not_called()
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)

    def test_response_sync_failure_returns_no_success(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        raw = producer.client.begin(w.Prepare(Payload("start")))
        original_sync = os.fsync
        calls = 0

        def sync(fd):
            nonlocal calls
            calls += 1
            if calls == 2:
                raise OSError("synthetic response sync failure")
            original_sync(fd)

        with (
            patch.object(os, "fsync", side_effect=sync),
            patch.object(framing, "write_local_frame") as send,
            patch.object(
                framing, "read_local_frame", return_value=producer.answer(raw)
            ),
        ):
            with self.assertRaises(OSError):
                journal.exchange(
                    self.peers[0].binding.endpoint_id,
                    raw,
                    None,
                    None,
                    deadline=time.monotonic() + 10,
                )
            send.assert_called_once()
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)

    def test_partial_write_failure_preserves_failed_prefix(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        raw = producer.client.begin(w.Prepare(Payload("start")))
        original_write = os.write
        calls = 0

        def write(fd, data):
            nonlocal calls
            calls += 1
            if calls == 1:
                return original_write(fd, data[:5])
            raise OSError("synthetic partial write")

        before = self.path.stat().st_size
        with (
            patch.object(os, "write", side_effect=write),
            patch.object(framing, "write_local_frame") as send,
        ):
            with self.assertRaises(OSError):
                journal.exchange(
                    self.peers[0].binding.endpoint_id,
                    raw,
                    None,
                    None,
                    deadline=time.monotonic() + 10,
                )
            send.assert_not_called()
        self.assertEqual(self.path.stat().st_size, before + 5)
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)

    def test_malformed_response_bytes_are_retained_without_completion(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        raw = producer.client.begin(w.Prepare(Payload("start")))
        with self.assertRaises((CaptureError, w.ModularError)):
            producer.exchange(journal, raw, b'{"malformed":true}')
        self.assertEqual(records(self.path)[-1][2][1:], b'{"malformed":true}')
        with self.assertRaises((CaptureError, w.ModularError)):
            verify(self.path, self.peers)

    def test_expired_deadline_and_foreign_thread_dispatch_nothing(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        raw = producer.client.begin(w.Prepare(Payload("start")))
        failures = []

        def foreign():
            try:
                journal.exchange(
                    self.peers[0].binding.endpoint_id,
                    raw,
                    None,
                    None,
                    deadline=time.monotonic() + 10,
                )
            except CaptureError as error:
                failures.append(error.code)

        worker = threading.Thread(target=foreign)
        worker.start()
        worker.join(timeout=5)
        self.assertFalse(worker.is_alive())
        self.assertEqual(failures, ["single_caller"])
        with patch.object(framing, "write_local_frame") as send:
            with self.assertRaises(CaptureError):
                journal.exchange(
                    self.peers[0].binding.endpoint_id,
                    raw,
                    None,
                    None,
                    deadline=time.monotonic() - 1,
                )
            send.assert_not_called()

    def test_live_private_streams_sync_before_dispatch_and_response_return(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        left, right = socket.socketpair()
        self.addCleanup(left.close)
        self.addCleanup(right.close)
        reader, writer = (
            left.makefile("rb", buffering=0),
            left.makefile("wb", buffering=0),
        )
        remote_reader, remote_writer = (
            right.makefile("rb", buffering=0),
            right.makefile("wb", buffering=0),
        )
        for stream in (reader, writer, remote_reader, remote_writer):
            self.addCleanup(stream.close)
        failures = []

        def serve():
            try:
                for _ in range(4):
                    raw = framing.read_local_frame(
                        remote_reader, deadline=time.monotonic() + 10
                    )
                    self.assertEqual(records(self.path)[-1][2][1:], raw)
                    response = producer.answer(raw)
                    framing.write_local_frame(
                        remote_writer, response, deadline=time.monotonic() + 10
                    )
            except BaseException as error:
                failures.append(error)

        worker = threading.Thread(target=serve)
        worker.start()
        sync_count = 0
        original_sync = os.fsync

        def sync(fd):
            nonlocal sync_count
            original_sync(fd)
            sync_count += 1

        try:
            with patch.object(os, "fsync", side_effect=sync):
                for operation in (
                    w.Prepare(Payload("start")),
                    w.Finish(Payload("done")),
                ):
                    raw = producer.client.begin(operation)
                    before = sync_count
                    response = journal.exchange(
                        self.peers[0].binding.endpoint_id,
                        raw,
                        reader,
                        writer,
                        deadline=time.monotonic() + 10,
                    )
                    self.assertEqual(sync_count, before + 2)
                    producer.client.observe(response)
                    ack = producer.client.acknowledgement()
                    response = journal.exchange(
                        self.peers[0].binding.endpoint_id,
                        ack,
                        reader,
                        writer,
                        deadline=time.monotonic() + 10,
                    )
                    producer.client.observe_acknowledgement(response)
        finally:
            worker.join(timeout=10)
        self.assertFalse(worker.is_alive())
        self.assertEqual(failures, [])
        self.assertEqual(journal.finish(), verify(self.path, self.peers))

    def test_exchange_limit_rejects_before_an_additional_dispatch(self):
        journal = self.journal(maximum=1, quota=1024**2)
        producer = Producer(self.peers[0])
        producer.operation(journal, w.Prepare(Payload("start")), acknowledge=False)
        size = self.path.stat().st_size
        with patch.object(framing, "write_local_frame") as send:
            with self.assertRaises(CaptureError):
                journal.exchange(
                    self.peers[0].binding.endpoint_id,
                    producer.client.acknowledgement(),
                    None,
                    None,
                    deadline=time.monotonic() + 10,
                )
            send.assert_not_called()
        self.assertEqual(self.path.stat().st_size, size)

    def test_transport_eof_does_not_retry_or_finalize(self):
        journal = self.journal()
        producer = Producer(self.peers[0])
        raw = producer.client.begin(w.Prepare(Payload("start")))
        with (
            patch.object(framing, "write_local_frame") as send,
            patch.object(framing, "read_local_frame", return_value=None),
        ):
            with self.assertRaises(CaptureError):
                journal.exchange(
                    self.peers[0].binding.endpoint_id,
                    raw,
                    None,
                    None,
                    deadline=time.monotonic() + 10,
                )
            send.assert_called_once()
        self.assertEqual(records(self.path)[-1][0], 2)
        with self.assertRaises(CaptureError):
            verify(self.path, self.peers)

    def test_file_replacement_during_verification_is_detected(self):
        self.completed()
        replacement = self.path.with_name("replacement")
        replacement.write_bytes(self.path.read_bytes())
        os.chmod(replacement, 0o600)
        original = t._Replay.response
        replaced = False

        def response(replay, index, payload):
            nonlocal replaced
            original(replay, index, payload)
            if not replaced:
                os.replace(replacement, self.path)
                replaced = True

        with (
            patch.object(t._Replay, "response", response),
            self.assertRaises(CaptureError),
        ):
            verify(self.path, self.peers)
        self.assertEqual(verify(self.path, self.peers).store_completion, "complete")

    def test_closed_application_contract_rejects_rehashed_unknown_input(self):
        self.completed()
        original = records(self.path)
        changed = original.copy()
        kind, ordinal, payload = changed[1]
        request = w.parse(payload[1:])
        request["command"]["operation"]["data"]["unknown"] = True
        request["request_digest"] = w.typed_digest(
            w.REQUEST_SCHEMA, request, "request_digest"
        )
        changed[1] = kind, ordinal, payload[:1] + w.encode(request)
        rewrite(self.path, changed)
        with self.assertRaises((CaptureError, w.ModularError)):
            verify(self.path, self.peers)
        rewrite(self.path, original)
        self.assertEqual(verify(self.path, self.peers).store_completion, "complete")


if __name__ == "__main__":
    unittest.main()
