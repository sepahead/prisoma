"""One bounded file containing original NCP frames and a checked terminal record."""

from __future__ import annotations

import hashlib
import math
import os
from pathlib import Path
import stat
import struct
import threading
import time
from dataclasses import dataclass
from typing import BinaryIO

from ncp_local import modular_wire as w
from ncp_local import wire as framing
from ncp_local.modular_buffer import BufferBinding
from ncp_local.modular_client import Client

MAGIC = b"PNCPT\x00\x01\n"
PREFIX = struct.Struct(">BQI")
DOMAIN = b"prisoma.ncp.transcript.record.v1\0"
HEADER, REQUEST, RESPONSE, FINISH, ABORT = range(1, 6)
MAX_BYTES = 1024**3
MAX_EXCHANGES = 8190
MAX_PEERS = 16
TERMINAL_BYTES = 1024
RECORD_OVERHEAD = PREFIX.size + 32
PAIR_BYTES = 2 * (RECORD_OVERHEAD + 1 + w.FRAME_BYTES)


class CaptureError(ValueError):
    """A local capture failure with no submitted payload in its message."""

    def __init__(self, code: str) -> None:
        self.code = code
        super().__init__(f"NCP transcript: {code}")


def _require(condition: bool, code: str) -> None:
    if not condition:
        raise CaptureError(code)


@dataclass(frozen=True, slots=True)
class Peer:
    """One exact binding and its host-installed application contract."""

    binding: BufferBinding
    contract: type[w.Contract]


@dataclass(frozen=True, slots=True)
class Verification:
    """Transcript completion does not assert application or scientific validity."""

    store_completion: str
    exchange_pairs: int
    unanswered_request: bool
    pending_peer_operations: int
    finished_peers: int
    peer_count: int
    journal_bytes: int
    journal_digest: str
    application_completion_validated: bool = False
    scientific_validation: bool = False


class _Replay:
    """Use the public NCP client as the protocol-state authority."""

    def __init__(self, peers: tuple[Peer, ...]) -> None:
        _require(type(peers) is tuple and 1 <= len(peers) <= MAX_PEERS, "peers")
        _require(all(type(p) is Peer for p in peers), "peers")
        self.peers = peers
        self.clients = tuple(Client(p.binding, p.contract) for p in peers)
        _require(len({p.binding.endpoint_id for p in peers}) == len(peers), "peers")
        _require(len({p.binding.generation for p in peers}) == len(peers), "peers")
        _require(len({p.binding.run_id for p in peers}) == 1, "peers")
        self.prepared = [False] * len(peers)
        self.finished = [False] * len(peers)
        self.aborted = [False] * len(peers)
        self.pending: tuple[int, w.Command] | None = None
        self.pairs = 0

    def roster(self) -> list[dict]:
        return [
            {
                "binding": w.immutable_value(p.binding),
                "descriptor_sha256": hashlib.sha256(
                    p.contract.descriptor()
                ).hexdigest(),
            }
            for p in self.peers
        ]

    def request(self, index: int, payload: bytes) -> None:
        _require(self.pending is None and 0 <= index < len(self.peers), "request_order")
        peer, client = self.peers[index], self.clients[index]
        original = w.Request.decode(payload, peer.binding, peer.contract)
        command = original.command
        if type(command) is w.Execute:
            _require(
                not self.finished[index] and not self.aborted[index], "terminal_peer"
            )
            operation = command.operation
            if type(operation) is w.Prepare:
                _require(not self.prepared[index], "preparation_order")
            else:
                _require(self.prepared[index], "preparation_order")
            expected = client.begin(operation)
        elif type(command) is w.Ack:
            expected = client.acknowledgement()
        elif type(command) is w.Query:
            expected = client.result_query()
        else:
            raise CaptureError("request_kind")
        joined = w.Request.decode(expected, peer.binding, peer.contract)
        _require(joined.request_digest == original.request_digest, "request_join")
        self.pending = index, command

    def response(self, index: int, payload: bytes) -> None:
        _require(
            self.pending is not None and self.pending[0] == index, "response_order"
        )
        command = self.pending[1]
        client = self.clients[index]
        if type(command) is w.Execute:
            response = client.observe(payload)
        elif type(command) is w.Ack:
            response = client.observe_acknowledgement(payload)
        else:
            response = client.observe_query(payload)
        if response.outcome is w.Outcome.COMMITTED:
            if response.operation is w.Name.PREPARE:
                self.prepared[index] = True
            elif response.operation is w.Name.FINISH:
                self.finished[index] = True
            elif response.operation is w.Name.ABORT:
                self.aborted[index] = True
        self.pending = None
        self.pairs += 1

    def complete(self) -> bool:
        return (
            self.pending is None
            and all(self.finished)
            and all(
                c.pending_request is None and not c.is_retired for c in self.clients
            )
        )


def _header(replay: _Replay, maximum: int, quota: int) -> bytes:
    _require(w.integer(maximum, 1, MAX_EXCHANGES), "exchange_limit")
    _require(w.integer(quota, 1, MAX_BYTES), "quota")
    payload = w.encode(
        {
            "schema": "prisoma.ncp.transcript.v1",
            "max_exchanges": maximum,
            "quota_bytes": quota,
            "peers": replay.roster(),
        }
    )
    required = len(MAGIC) + RECORD_OVERHEAD + len(payload)
    required += maximum * PAIR_BYTES + RECORD_OVERHEAD + TERMINAL_BYTES
    _require(required <= quota, "quota")
    return payload


def _seal(
    ordinal: int, previous: bytes, kind: int, payload: bytes
) -> tuple[bytes, bytes]:
    _require(
        type(payload) is bytes and len(payload) <= w.FRAME_BYTES + 1, "record_size"
    )
    prefix = PREFIX.pack(kind, ordinal, len(payload))
    digest = hashlib.sha256(DOMAIN + previous + prefix + payload).digest()
    return prefix + payload + digest, digest


def _metadata(info: os.stat_result) -> tuple[int, ...]:
    return (
        info.st_dev,
        info.st_ino,
        info.st_mode,
        info.st_uid,
        info.st_nlink,
        info.st_size,
        info.st_mtime_ns,
        info.st_ctime_ns,
    )


def _private_regular(info: os.stat_result) -> None:
    _require(
        stat.S_ISREG(info.st_mode)
        and info.st_mode & 0o077 == 0
        and info.st_uid == os.getuid()
        and info.st_nlink == 1,
        "private_regular_file",
    )


class Journal:
    """Sequential capture on caller-owned streams; this class launches no peer.

    The fixed exchange quota includes one full request and response per slot.
    Pass each request through ``exchange`` before its producer can execute it.
    The caller must retain buffers until their captured reads succeed.
    ``close`` leaves an incomplete file unless ``finish`` or ``abort`` succeeded.
    """

    def __init__(
        self,
        path: str | Path,
        peers: tuple[Peer, ...],
        *,
        max_exchanges: int,
        quota_bytes: int,
    ) -> None:
        self._replay = _Replay(peers)
        payload = _header(self._replay, max_exchanges, quota_bytes)
        self._maximum, self._quota = max_exchanges, quota_bytes
        self._ordinal, self._previous, self._bytes = 0, bytes(32), 0
        self._thread = threading.get_ident()
        self._fd: int | None = None
        selected = Path(path)
        try:
            self._fd = os.open(
                selected,
                os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                0o600,
            )
            _private_regular(os.fstat(self._fd))
            self._write(MAGIC)
            self._append(HEADER, payload)
            parent = os.open(
                selected.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC
            )
            try:
                os.fsync(parent)
            finally:
                os.close(parent)
        except BaseException:
            self.close()
            raise

    def _active(self) -> None:
        _require(self._thread == threading.get_ident(), "single_caller")
        _require(self._fd is not None, "closed")

    def _write(self, payload: bytes) -> None:
        self._active()
        _require(len(payload) <= self._quota - self._bytes, "quota")
        view = memoryview(payload)
        while view:
            count = os.write(self._fd, view)
            _require(count > 0, "storage")
            view = view[count:]
        self._bytes += len(payload)

    def _append(self, kind: int, payload: bytes) -> None:
        raw, digest = _seal(self._ordinal, self._previous, kind, payload)
        self._write(raw)
        os.fsync(self._fd)
        self._ordinal += 1
        self._previous = digest

    def exchange(
        self,
        endpoint_id: str,
        request: bytes,
        reader: BinaryIO,
        writer: BinaryIO,
        *,
        deadline: float,
    ) -> bytes:
        """Sync a request, dispatch once, then sync its response before returning.

        The deadline covers the existing NCP transport calls. Local filesystem
        synchronization is blocking and has no hard real-time bound.
        Any failure retires this journal. The caller owns peer retirement.
        """
        self._active()
        try:
            _require(
                type(deadline) in (int, float)
                and math.isfinite(deadline)
                and deadline > time.monotonic(),
                "deadline",
            )
            _require(
                type(request) is bytes and 0 < len(request) <= w.FRAME_BYTES, "frame"
            )
            _require(self._replay.pairs < self._maximum, "exchange_limit")
            matches = [
                i
                for i, p in enumerate(self._replay.peers)
                if p.binding.endpoint_id == endpoint_id
            ]
            _require(len(matches) == 1, "peer")
            index = matches[0]
            self._replay.request(index, request)
            self._append(REQUEST, bytes([index]) + request)
            framing.write_local_frame(writer, request, deadline=deadline)
            response = framing.read_local_frame(reader, deadline=deadline)
            _require(response is not None, "response_missing")
            # Retain malformed received bytes as failed evidence, then close.
            self._append(RESPONSE, bytes([index]) + response)
            self._replay.response(index, response)
            return response
        except BaseException:
            self.close()
            raise

    def _terminal(self, kind: int) -> Verification:
        self._active()
        try:
            _require(kind == ABORT or self._replay.complete(), "unfinished_peers")
            payload = w.encode({"exchange_pairs": self._replay.pairs})
            _require(len(payload) <= TERMINAL_BYTES, "terminal_size")
            self._append(kind, payload)
            return _summary(self._replay, kind, self._bytes, self._previous)
        finally:
            self.close()

    def finish(self) -> Verification:
        """Finalize only after each peer's committed Finish has been acknowledged."""
        return self._terminal(FINISH)

    def abort(self) -> Verification:
        """Close an observed prefix without declaring the execution complete."""
        return self._terminal(ABORT)

    def close(self) -> None:
        if self._fd is not None:
            _require(self._thread == threading.get_ident(), "single_caller")
            fd, self._fd = self._fd, None
            os.close(fd)

    def __enter__(self) -> Journal:
        self._active()
        return self

    def __exit__(self, error_type, error, traceback) -> None:
        self.close()


def _summary(replay: _Replay, terminal: int, size: int, digest: bytes) -> Verification:
    return Verification(
        "complete" if terminal == FINISH else "aborted",
        replay.pairs,
        replay.pending is not None,
        sum(c.pending_request is not None for c in replay.clients),
        sum(replay.finished),
        len(replay.peers),
        size,
        digest.hex(),
    )


def _read_exact(file: BinaryIO, size: int) -> bytes:
    result = file.read(size)
    _require(len(result) == size, "truncated")
    return result


def verify(path: str | Path, peers: tuple[Peer, ...]) -> Verification:
    """Verify one stable private file with the exact installed peer contracts.

    This function dispatches nothing and never repairs an incomplete journal.
    A contract supplied by an untrusted caller is not an authority boundary.
    """
    replay = _Replay(peers)
    selected = Path(path)
    fd = os.open(selected, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as file:
        initial = os.fstat(file.fileno())
        _private_regular(initial)
        _require(len(MAGIC) < initial.st_size <= MAX_BYTES, "file_size")
        _require(_read_exact(file, len(MAGIC)) == MAGIC, "magic")
        ordinal, previous, total = 0, bytes(32), len(MAGIC)
        maximum, quota, terminal = None, None, None
        while total < initial.st_size:
            _require(terminal is None, "after_terminal")
            prefix = _read_exact(file, PREFIX.size)
            kind, sequence, size = PREFIX.unpack(prefix)
            _require(sequence == ordinal and size <= w.FRAME_BYTES + 1, "record_header")
            _require(ordinal <= 2 * MAX_EXCHANGES + 1, "record_count")
            payload = _read_exact(file, size)
            digest = _read_exact(file, 32)
            _require(
                digest == hashlib.sha256(DOMAIN + previous + prefix + payload).digest(),
                "record_digest",
            )
            total += PREFIX.size + size + 32
            if ordinal == 0:
                _require(kind == HEADER, "header_order")
                header = w.closed(
                    w.parse(payload),
                    {"schema", "max_exchanges", "quota_bytes", "peers"},
                )
                maximum, quota = header["max_exchanges"], header["quota_bytes"]
                expected = w.parse(_header(replay, maximum, quota))
                _require(header == expected, "header_join")
            elif kind in (REQUEST, RESPONSE):
                _require(1 < size <= w.FRAME_BYTES + 1, "frame")
                index, frame = payload[0], payload[1:]
                _require(replay.pairs < maximum, "exchange_limit")
                if kind == REQUEST:
                    replay.request(index, frame)
                else:
                    replay.response(index, frame)
            elif kind in (FINISH, ABORT):
                _require(size <= TERMINAL_BYTES, "terminal_size")
                count = w.closed(w.parse(payload), {"exchange_pairs"})["exchange_pairs"]
                _require(
                    w.integer(count, 0, maximum) and count == replay.pairs,
                    "terminal_count",
                )
                _require(kind == ABORT or replay.complete(), "unfinished_peers")
                terminal = kind
            else:
                raise CaptureError("record_kind")
            _require(total <= quota and total <= initial.st_size, "file_size")
            ordinal, previous = ordinal + 1, digest
        _require(terminal is not None, "terminal_missing")
        final = os.fstat(file.fileno())
        lexical = selected.lstat()
        _require(
            _metadata(initial) == _metadata(final) == _metadata(lexical), "file_changed"
        )
        return _summary(replay, terminal, total, previous)
