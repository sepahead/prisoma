"""Owned compressed archive -> HDF5 action rows -> actual sklearn fit.

The public dataset issuer accepts no expected digest or verification receipt.
Controls always retain control-only scope. This is an ordinary trusted local
Python workflow, not arbitrary-code isolation or loaded-code attestation.
"""

from __future__ import annotations

import hashlib
import ctypes
import json
import math
import os
import platform
import re
import selectors
import shutil
import signal
import stat
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace
from typing import Callable

import numpy as np

from .assets import verify_runtime
from .supported_actions import (
    MAX_FIT_ROWS,
    LEGACY_ROW_PROFILE,
    FittedActionScaler,
    _admit_row_shape,
    _fit_owned_rows,
    _row_byte_limit,
)

CHUNK_BYTES = 1024 * 1024
ACTION_BYTES = MAX_FIT_ROWS * 2 * 8
CONTROL_ARCHIVE_BYTES = 4 * 1024 * 1024
CONTROL_DECODED_BYTES = 8 * 1024 * 1024
DISK_FLOOR_BYTES = 24 * 1024**3
SNAPSHOT_SECONDS = 900
DECODE_SECONDS = 1800
STDERR_BYTES = 65536
ZSTD_MEMORY_MIB = 256
HDF5_READER_SHA256 = "afe836ec706acc6c55450f3c936a5898b57ed65a999695af25b4b59cd14d10f3"
DECODER = Path("/opt/homebrew/Cellar/zstd/1.5.7_1/bin/zstd")
_DECODER_FILES = (
    (
        str(DECODER),
        str(DECODER),
        205776,
        "aff8169fb421bb925fb16c44a7e0143fa2c7a941dc45cce76b15062a2ce54917",
    ),
    (
        "/opt/homebrew/Cellar/zstd/1.5.7_1/lib/libzstd.1.dylib",
        "/opt/homebrew/Cellar/zstd/1.5.7_1/lib/libzstd.1.5.7.dylib",
        649648,
        "e2847c4613b386683c234913ae3b7b04299254096caf7616e3b3cd9bb97a39ab",
    ),
    (
        "/opt/homebrew/opt/xz/lib/liblzma.5.dylib",
        "/opt/homebrew/Cellar/xz/5.8.3/lib/liblzma.5.dylib",
        184512,
        "3d5bfa2f097c31463642b1daab5e662b44368bb4da368f85e412e7f9adcbaa10",
    ),
    (
        "/opt/homebrew/opt/lz4/lib/liblz4.1.dylib",
        "/opt/homebrew/Cellar/lz4/1.10.0/lib/liblz4.1.10.0.dylib",
        178048,
        "be3414171e667ff38a542d6dcdef1e2f0f04028e4a6080f2c164afff38b0d9b7",
    ),
)


@dataclass(frozen=True)
class _ArchiveProfile:
    scope: str
    source_id: str
    compressed_bytes: int
    compressed_sha256: str
    decoded_bytes: int


_TRAINING = _ArchiveProfile(
    "verified_frozen_training_archive_rows",
    "quentinll-lewm-pusht-655cd446b9929369d7d406001da85c15d1457850",
    13136247974,
    "7cfbd6d90fa2f27876379a5ff169715a36ed82edbda64f9e5b5bfa34d212f318",
    46300921856,
)


class DatasetReadError(RuntimeError):
    """Retain the original failure and independently observed cleanup evidence."""

    def __init__(self, primary: BaseException, receipt: dict):
        super().__init__(f"Owned action-row transaction failed: {primary}")
        self.primary = primary
        self.receipt = json.loads(json.dumps(receipt))


def _identity(info: os.stat_result) -> tuple:
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns)


def _open_regular(path: Path, limit: int) -> tuple[int, os.stat_result]:
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or not 0 < info.st_size <= limit:
            raise ValueError("Source is not a nonempty bounded regular file")
        return descriptor, info
    except BaseException:
        os.close(descriptor)
        raise


def _hash_descriptor(descriptor: int, size: int) -> str:
    os.lseek(descriptor, 0, os.SEEK_SET)
    digest, count = hashlib.sha256(), 0
    while block := os.read(descriptor, min(CHUNK_BYTES, size - count + 1)):
        count += len(block)
        if count > size:
            raise ValueError("File grew beyond its admitted extent")
        digest.update(block)
    if count != size:
        raise ValueError("File ended before its admitted extent")
    os.lseek(descriptor, 0, os.SEEK_SET)
    return digest.hexdigest()


def _decoder_identity() -> dict:
    if any(name.startswith("DYLD_") for name in os.environ):
        raise ValueError("DYLD configuration is outside the decoder profile")
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise ValueError("Decoder profile requires Darwin arm64")
    observed = []
    for path, realpath, size, digest in _DECODER_FILES:
        if str(Path(path).resolve(strict=True)) != realpath:
            raise ValueError("Decoder dependency path changed")
        descriptor, before = _open_regular(Path(realpath), size)
        try:
            actual = _hash_descriptor(descriptor, size)
            if actual != digest or _identity(os.fstat(descriptor)) != _identity(before):
                raise ValueError("Decoder dependency bytes changed")
        finally:
            os.close(descriptor)
        observed.append(
            {"path": path, "realpath": realpath, "bytes": size, "sha256": actual}
        )
    build = (
        subprocess.run(
            ["/usr/bin/sw_vers", "-buildVersion"],
            check=True,
            capture_output=True,
            timeout=5,
            env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"},
        )
        .stdout.decode("ascii")
        .strip()
    )
    return {
        "files": observed,
        "macos_version": platform.mac_ver()[0],
        "macos_build": build,
        "system_linkage": ["/usr/lib/libz.1.dylib", "/usr/lib/libSystem.B.dylib"],
        "scope": "local_non_system_bytes_and_system_build_observation_not_loaded_byte_attestation",
    }


def _space(directory: Path, remaining: int = 0) -> None:
    if shutil.disk_usage(directory).free < DISK_FLOOR_BYTES + remaining:
        raise ValueError("Insufficient free space for the frozen transaction")


def _write_all(descriptor: int, data: bytes) -> None:
    view = memoryview(data)
    while view:
        written = os.write(descriptor, view)
        if written <= 0:
            raise OSError("Private output write made no progress")
        view = view[written:]


def _save(path: Path, value: dict) -> None:
    with path.open("x", encoding="utf-8") as stream:
        os.chmod(path, 0o600)
        json.dump(value, stream, sort_keys=True, indent=2, allow_nan=False)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())


def _snapshot(
    archive: Path, output: Path, profile: _ArchiveProfile, receipt: dict
) -> tuple[int, dict]:
    source = target = retained = None
    primary = None
    close_errors = []
    try:
        source, before = _open_regular(archive, profile.compressed_bytes)
        if before.st_size != profile.compressed_bytes:
            raise ValueError("The complete frozen compressed extent is unavailable")
        target = os.open(output, os.O_RDWR | os.O_CREAT | os.O_EXCL, 0o600)
        count, digest, deadline = (
            0,
            hashlib.sha256(),
            time.monotonic() + SNAPSHOT_SECONDS,
        )
        while block := os.read(
            source, min(CHUNK_BYTES, profile.compressed_bytes - count + 1)
        ):
            if time.monotonic() >= deadline:
                raise TimeoutError("Compressed snapshot deadline")
            count += len(block)
            if count > profile.compressed_bytes:
                raise ValueError("Compressed source grew beyond its frozen extent")
            _space(
                output.parent,
                profile.compressed_bytes - count + profile.decoded_bytes,
            )
            _write_all(target, block)
            digest.update(block)
        if _identity(os.fstat(source)) != _identity(before):
            raise ValueError("Compressed source changed during snapshot")
        if (
            count != profile.compressed_bytes
            or digest.hexdigest() != profile.compressed_sha256
        ):
            raise ValueError("Complete compressed source digest or extent mismatch")
        os.fsync(target)
        os.fchmod(target, 0o400)
        private_identity = _identity(os.fstat(target))
        closing = (target, source)
        target = source = None
        close_errors.extend(_close_descriptors(closing))
        if close_errors:
            raise close_errors[0]
        retained, observed = _open_regular(output, profile.compressed_bytes)
        if _identity(observed) != private_identity:
            raise ValueError("Private compressed snapshot was replaced")
        magic = os.read(retained, 4)
        os.lseek(retained, 0, os.SEEK_SET)
        if magic != b"\x28\xb5\x2f\xfd":
            raise ValueError("The admitted stream is not a standard Zstandard frame")
        receipt.update(
            bytes=count,
            sha256=digest.hexdigest(),
            source_identity=list(_identity(before)),
        )
    except BaseException as error:
        primary = error
    finally:
        closing = (target, source, retained if primary is not None else None)
        target = source = None
        if primary is not None:
            retained = None
        close_errors.extend(_close_descriptors(closing))
        if close_errors:
            receipt["descriptor_close_errors"] = [
                f"{type(error).__name__}: {error}" for error in close_errors
            ]
        if primary is not None:
            receipt["primary_failure"] = f"{type(primary).__name__}: {primary}"
    if primary is not None:
        raise primary
    return retained, receipt


def _group_alive(pid: int) -> bool:
    try:
        os.killpg(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def _prepare_waitid() -> tuple[Callable, dict]:
    """Prepare the reviewed Darwin ABI before creating any owned process."""
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise ValueError("Decoder lifecycle profile requires Darwin arm64")
    if signal.getsignal(signal.SIGCHLD) != signal.SIG_DFL:
        raise ValueError("Decoder requires default SIGCHLD and exclusive child waiting")

    class Sigval(ctypes.Union):
        _fields_ = [("integer", ctypes.c_int), ("pointer", ctypes.c_void_p)]

    class Siginfo(ctypes.Structure):
        _fields_ = [
            ("signo", ctypes.c_int),
            ("error", ctypes.c_int),
            ("code", ctypes.c_int),
            ("pid", ctypes.c_int),
            ("uid", ctypes.c_uint),
            ("status", ctypes.c_int),
            ("addr", ctypes.c_void_p),
            ("value", Sigval),
            ("band", ctypes.c_long),
            ("pad", ctypes.c_ulong * 7),
        ]

    # Independently joined to SDK sizeof/offsetof and a compiled native control.
    layout = {name: getattr(Siginfo, name).offset for name, _ in Siginfo._fields_}
    expected = dict(
        signo=0,
        error=4,
        code=8,
        pid=12,
        uid=16,
        status=20,
        addr=24,
        value=32,
        band=40,
        pad=48,
    )
    constants = {
        name: getattr(os, name)
        for name in (
            "P_PID",
            "WEXITED",
            "WNOHANG",
            "WNOWAIT",
            "CLD_EXITED",
            "CLD_KILLED",
            "CLD_DUMPED",
        )
    }
    if (
        layout != expected
        or ctypes.sizeof(Siginfo) != 104
        or ctypes.alignment(Siginfo) != 8
        or ctypes.sizeof(ctypes.c_uint) != 4
        or tuple(constants.values()) != (1, 4, 1, 32, 1, 2, 3)
    ):
        raise ValueError("Darwin waitid ABI differs from the reviewed profile")
    library = ctypes.CDLL("/usr/lib/libSystem.B.dylib", use_errno=True)
    native = library.waitid
    native.argtypes = [
        ctypes.c_int,
        ctypes.c_uint,
        ctypes.POINTER(Siginfo),
        ctypes.c_int,
    ]
    native.restype = ctypes.c_int

    def observe(pid: int):
        row = Siginfo()
        ctypes.set_errno(0)
        result = native(
            os.P_PID, pid, ctypes.byref(row), os.WEXITED | os.WNOHANG | os.WNOWAIT
        )
        if result != 0:
            number = ctypes.get_errno()
            raise OSError(number, os.strerror(number))
        if row.pid == 0:
            return None
        return SimpleNamespace(si_pid=row.pid, si_code=row.code, si_status=row.status)

    # The current process cannot be its own child. Verify the API before launch.
    try:
        observe(os.getpid())
    except ChildProcessError:
        pass
    else:
        raise ValueError("Darwin waitid did not reject a non-child identity")

    return observe, {
        "api": "darwin-arm64-libSystem-waitid-v1",
        "library": "/usr/lib/libSystem.B.dylib",
        "siginfo_bytes": 104,
        "siginfo_alignment": 8,
        "offsets": layout,
        "constants": constants,
        "scope": "local_system_api_and_reviewed_abi_not_loaded_byte_attestation",
    }


@dataclass
class _OwnedDecoder:
    """Exclusive unreaped-child owner; ordinary trusted Python, not thread isolation."""

    process: subprocess.Popen
    wait_child: Callable
    may_signal: bool = True

    def observe(self):
        if not self.may_signal or self.process.returncode is not None:
            self.may_signal = False
            raise ChildProcessError("Decoder child ownership was relinquished")
        try:
            status = self.wait_child(self.process.pid)
        except BaseException:
            self.may_signal = False
            raise
        if status is not None and (
            status.si_pid != self.process.pid
            or status.si_code not in (os.CLD_EXITED, os.CLD_KILLED, os.CLD_DUMPED)
        ):
            self.may_signal = False
            raise ChildProcessError("Decoder wait observation changed child identity")
        return status

    def signal(self, sig: int) -> None:
        self.observe()
        os.killpg(self.process.pid, sig)

    def reap(self, timeout: float) -> int:
        # Revoke before wait, including a timeout or error. No later signal is legal.
        self.may_signal = False
        return self.process.wait(timeout=timeout)


def _wait_exited(owner: _OwnedDecoder, deadline: float):
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("Zstandard decode deadline")
        status = owner.observe()
        if status is not None:
            return status
        time.sleep(min(0.02, remaining))


def _cleanup(owner: _OwnedDecoder) -> dict:
    process = owner.process
    errors = []
    ownership_lost = False
    try:
        owner.observe()
        for sig in (signal.SIGTERM, signal.SIGKILL):
            if not _group_alive(process.pid):
                break
            try:
                owner.signal(sig)
            except ProcessLookupError:
                pass
            except ChildProcessError:
                raise
            except OSError as error:
                errors.append(str(error))
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                owner.observe()
                if not _group_alive(process.pid):
                    break
                time.sleep(0.02)
    except BaseException as error:
        ownership_lost = True
        errors.append(f"{type(error).__name__}: {error}")
    try:
        owner.reap(timeout=1)
    except BaseException as error:
        errors.append(f"{type(error).__name__}: {error}")
    # After reap this is observation only. Even a reused PGID receives no signal.
    absent = not _group_alive(process.pid)
    reaped = process.returncode is not None
    return {
        # A transient signal denial is retained. Only later actual group absence
        # and a reaped leader confirm cleanup; neither denial nor exit suffices.
        "status": "confirmed"
        if absent and reaped and not ownership_lost
        else "unresolved",
        "group_absent": absent,
        "leader_reaped": reaped,
        "signal_authority_revoked": not owner.may_signal,
        "ownership_lost": ownership_lost,
        "returncode": process.returncode,
        "errors": errors,
    }


def _decode(compressed: int, output: Path, expected_bytes: int, receipt: dict) -> int:
    wait_child, wait_identity = _prepare_waitid()
    target = os.open(output, os.O_RDWR | os.O_CREAT | os.O_EXCL, 0o600)
    command = [
        str(DECODER),
        "--decompress",
        "--stdout",
        "--check",
        "--no-pass-through",
        "--no-progress",
        "--no-asyncio",
        f"-M{ZSTD_MEMORY_MIB * 1024**2}",
        "-",
    ]
    process = owner = None
    primary = None
    close_errors = []
    count, digest, stderr = 0, hashlib.sha256(), bytearray()
    started = time.monotonic()
    receipt.update(
        command=command,
        timeout_seconds=DECODE_SECONDS,
        zstd_window_memory_mib=ZSTD_MEMORY_MIB,
        decoded_cap_bytes=expected_bytes,
        waitid_observer=wait_identity,
    )
    try:
        process = subprocess.Popen(
            command,
            stdin=compressed,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
            env={"PATH": "/usr/bin:/bin", "LC_ALL": "C", "LANG": "C"},
        )
        owner = _OwnedDecoder(process, wait_child)
        receipt["pid"] = process.pid
        with selectors.DefaultSelector() as selector:
            for stream, name in (
                (process.stdout, "stdout"),
                (process.stderr, "stderr"),
            ):
                os.set_blocking(stream.fileno(), False)
                selector.register(stream, selectors.EVENT_READ, name)
            while selector.get_map():
                remaining = DECODE_SECONDS - (time.monotonic() - started)
                if remaining <= 0:
                    raise TimeoutError("Zstandard decode deadline")
                _space(output.parent, expected_bytes - count)
                for key, _events in selector.select(min(0.25, remaining)):
                    capacity = (
                        expected_bytes - count
                        if key.data == "stdout"
                        else STDERR_BYTES - len(stderr)
                    )
                    block = os.read(key.fd, min(CHUNK_BYTES, capacity + 1))
                    if not block:
                        selector.unregister(key.fileobj)
                    elif key.data == "stdout":
                        if count + len(block) > expected_bytes:
                            raise ValueError(
                                "Decoded stdout exceeded the frozen byte cap"
                            )
                        _write_all(target, block)
                        count += len(block)
                        digest.update(block)
                    else:
                        stderr.extend(block)
                        if len(stderr) > STDERR_BYTES:
                            raise ValueError("Decoder stderr exceeded its byte cap")
            status = _wait_exited(owner, started + DECODE_SECONDS)
            code = (
                status.si_status
                if status.si_code == os.CLD_EXITED
                else -status.si_status
            )
        if code != 0:
            raise ValueError(f"Zstandard did not terminate successfully: {code}")
        # Darwin's unreaped zombie can yield EPERM. It is not proof of absence.
        # A successful probe after leader exit records an unexpected live group.
        try:
            os.killpg(process.pid, 0)
        except PermissionError:
            receipt["group_after_leader_exit"] = "permission_denied_unknown"
        except ProcessLookupError:
            receipt["group_after_leader_exit"] = "absent"
        else:
            receipt["group_after_leader_exit"] = "present"
            raise ValueError("Decoder descendants outlived the direct process")
        if count != expected_bytes:
            raise ValueError("Decoded extent differs from the frozen complete size")
        os.fsync(target)
        os.fchmod(target, 0o400)
        identity = _identity(os.fstat(target))
    except BaseException as error:
        primary = error
    finally:
        if process is not None:
            try:
                receipt["cleanup"] = _cleanup(owner)
            except BaseException as cleanup:
                receipt["cleanup"] = {
                    "status": "unresolved",
                    "errors": [f"{type(cleanup).__name__}: {cleanup}"],
                }
            streams = (process.stdout, process.stderr)
            process.stdout = process.stderr = None
            for name, stream in zip(("stdout", "stderr"), streams):
                try:
                    stream.close()
                except BaseException as error:
                    close_errors.append((name, error))
        else:
            receipt["cleanup"] = {"status": "not_started", "errors": []}
        closing, target = target, None
        try:
            os.close(closing)
        except BaseException as error:
            close_errors.append(("target", error))
        receipt.update(
            decoded_bytes=count,
            decoded_sha256=digest.hexdigest(),
            stderr=bytes(stderr).decode("utf-8", errors="replace"),
            elapsed_seconds=time.monotonic() - started,
        )
        if close_errors:
            receipt["resource_close_errors"] = [
                {"resource": name, "error": f"{type(error).__name__}: {error}"}
                for name, error in close_errors
            ]
        if primary is not None:
            receipt["primary_failure"] = f"{type(primary).__name__}: {primary}"
    if primary is not None:
        raise primary
    if receipt["cleanup"]["status"] != "confirmed":
        raise ValueError("Decoder cleanup remains unresolved")
    if close_errors:
        raise close_errors[0][1]
    retained, observed = _open_regular(output, expected_bytes)
    if _identity(observed) != identity:
        os.close(retained)
        raise ValueError("Decoded private output was replaced")
    return retained


def _read_rows(
    descriptor: int, *, row_profile: str = LEGACY_ROW_PROFILE
) -> tuple[np.ndarray, dict]:
    _row_byte_limit(row_profile)
    # This imports neither the general stable-worldmodel reader nor hdf5plugin/Torch.
    import h5py

    before = _identity(os.fstat(descriptor))
    with os.fdopen(os.dup(descriptor), "rb") as stream:
        stream.seek(0)
        with h5py.File(
            stream, "r", driver="fileobj", rdcc_nbytes=4 * 1024 * 1024
        ) as file:
            if not isinstance(file.get("action", getlink=True), h5py.HardLink):
                raise ValueError("Root action must be a direct HDF5 hard link")
            dataset = file["action"]
            if not isinstance(dataset, h5py.Dataset):
                raise ValueError("Root action is not a dataset")
            shape, dtype = dataset.shape, dataset.dtype
            if (
                shape is None
                or len(shape) != 2
                or shape[1] != 2
                or dtype.kind != "f"
                or dtype.itemsize not in (4, 8)
                or not dtype.isnative
                or dtype.fields is not None
                or dtype.subdtype is not None
                or dtype.metadata is not None
            ):
                raise ValueError(
                    "Action requires bounded native float32/float64 shape [N,2]"
                )
            try:
                admission = _admit_row_shape(shape, dtype.itemsize, row_profile)
            except ValueError as error:
                raise ValueError(
                    f"Action requires bounded native float32/float64 rows: {error}"
                ) from error
            properties = dataset.id.get_create_plist()
            if (
                dataset.is_virtual
                or properties.get_external_count()
                or properties.get_layout()
                not in (h5py.h5d.CONTIGUOUS, h5py.h5d.CHUNKED)
            ):
                raise ValueError("Indirect or unsupported HDF5 storage")
            if (
                dataset.chunks
                and math.prod(dataset.chunks) * dtype.itemsize > ACTION_BYTES
            ):
                raise ValueError("HDF5 chunk exceeds the numeric allocation bound")
            if properties.get_nfilters() > 3:
                raise ValueError("Unsupported HDF5 filter pipeline")
            filters = []
            for index in range(properties.get_nfilters()):
                identifier, flags, options, name = properties.get_filter(index)
                if not (
                    identifier == 1
                    and len(options) == 1
                    and 0 <= options[0] <= 9
                    or identifier == 2
                    and tuple(options) == (dtype.itemsize,)
                    or identifier == 3
                    and not options
                ):
                    raise ValueError("Unsupported HDF5 filter")
                filters.append(
                    {
                        "id": identifier,
                        "flags": flags,
                        "options": list(options),
                        "name": name.decode("ascii", errors="replace"),
                    }
                )
            if dataset.id.get_space_status() != h5py.h5d.SPACE_STATUS_ALLOCATED:
                raise ValueError("Action storage contains unallocated fill-only space")
            # This is the pinned wheel's root column operation, after allocation checks.
            data = dataset[:]
            if (
                type(data) is not np.ndarray
                or data.shape != shape
                or data.dtype != dtype
            ):
                raise ValueError("Action read changed its admitted dtype or shape")
            # HDF5 may retain an explicit native endian marker in its dtype object.
            # Use NumPy's native scalar descriptor without changing any row bytes.
            native_dtype = np.float32 if dtype.itemsize == 4 else np.float64
            owned = np.frombuffer(data.tobytes(order="C"), dtype=native_dtype).reshape(
                shape
            )
            observation = {
                "column": "/action",
                "row_order": "root_action_full_slice_original_order",
                "dtype": dtype.str,
                "shape": list(shape),
                "filters": filters,
                "chunks": list(dataset.chunks) if dataset.chunks else None,
                "hdf5_version": h5py.version.hdf5_version,
                "reader_source_sha256": HDF5_READER_SHA256,
            }
            if row_profile != LEGACY_ROW_PROFILE:
                observation["row_admission"] = admission
    if _identity(os.fstat(descriptor)) != before:
        raise ValueError("Decoded HDF5 changed while reading action rows")
    return owned, observation


def _close_descriptors(descriptors: tuple) -> list[BaseException]:
    """Close relinquished descriptors once; never retry an uncertain numeric FD."""
    errors = []
    for descriptor in descriptors:
        if descriptor is not None:
            try:
                os.close(descriptor)
            except BaseException as error:
                errors.append(error)
    return errors


def _fit_archive(
    archive: Path,
    output: Path,
    profile: _ArchiveProfile,
    *,
    row_profile: str = LEGACY_ROW_PROFILE,
) -> FittedActionScaler:
    row_bytes = _row_byte_limit(row_profile)
    runtime = verify_runtime()
    decoder = _decoder_identity()
    output.mkdir(mode=0o700)
    compressed = decoded = None
    receipt = {
        "schema": "prisoma.lewm.owned-action-row-transaction.v1",
        "status": "PREPARING",
        "scope": profile.scope,
        "source_id": profile.source_id,
        "runtime": runtime,
        "decoder_identity": decoder,
        "decode": {},
        "row_limit": MAX_FIT_ROWS,
        "numeric_allocation_bytes": ACTION_BYTES,
    }
    if row_profile != LEGACY_ROW_PROFILE:
        del receipt["row_limit"]
        receipt.update(
            row_profile=row_profile,
            row_limits_by_dtype={
                "float32": row_bytes // (2 * 4),
                "float64": row_bytes // (2 * 8),
            },
            numeric_allocation_bytes=row_bytes,
            maximum_chunk_bytes=ACTION_BYTES,
        )
    try:
        _space(output, profile.compressed_bytes + profile.decoded_bytes)
        receipt["compressed_snapshot"] = {}
        compressed, _snapshot_receipt = _snapshot(
            archive, output / "verified.h5.zst", profile, receipt["compressed_snapshot"]
        )
        _save(output / "input.json", receipt)
        compressed_identity = _identity(os.fstat(compressed))
        decoded = _decode(
            compressed, output / "decoded.h5", profile.decoded_bytes, receipt["decode"]
        )
        if (
            _identity(os.fstat(compressed)) != compressed_identity
            or _hash_descriptor(compressed, profile.compressed_bytes)
            != profile.compressed_sha256
        ):
            raise ValueError("Compressed private snapshot changed during decode")
        decoded_identity = _identity(os.fstat(decoded))
        rows, row_observation = _read_rows(decoded, row_profile=row_profile)
        receipt["action_read"] = row_observation
        if (
            _identity(os.fstat(decoded)) != decoded_identity
            or _hash_descriptor(decoded, profile.decoded_bytes)
            != receipt["decode"]["decoded_sha256"]
        ):
            raise ValueError("Decoded private snapshot changed during action admission")
        if verify_runtime() != runtime or _decoder_identity() != decoder:
            raise ValueError("Runtime or decoder identity changed during transaction")
        closing = (decoded, compressed)
        decoded = compressed = None
        close_errors = _close_descriptors(closing)
        if close_errors:
            receipt["descriptor_close_errors"] = [
                f"{type(error).__name__}: {error}" for error in close_errors
            ]
            raise close_errors[0]
        receipt["status"] = "ROWS_ADMITTED"
        fitted = _fit_owned_rows(
            rows,
            {
                "schema": "prisoma.lewm.action-scaler-owned-archive.v1",
                "scope": profile.scope,
                "source_id": profile.source_id,
                "row_order": row_observation["row_order"],
                "transaction": receipt,
            },
            row_profile=row_profile,
        )
        _save(output / "scaler.json", fitted.receipt())
        _save(
            output / "transaction.json",
            {
                **receipt,
                "status": "FIT_COMPLETED",
                "scaler_sha256": fitted.sha256,
                "dataset_authority": profile is _TRAINING,
                "raw_actions_executed": False,
                "scientific_qualification": "NOT_RUN",
            },
        )
        return fitted
    except BaseException as error:
        closing = (decoded, compressed)
        decoded = compressed = None
        close_errors = _close_descriptors(closing)
        if close_errors:
            receipt.setdefault("descriptor_close_errors", []).extend(
                f"{type(close).__name__}: {close}" for close in close_errors
            )
        receipt.update(
            status="FAILED", primary_failure=f"{type(error).__name__}: {error}"
        )
        try:
            _save(output / "failure.json", receipt)
        except OSError as persistence:
            receipt["failure_persistence_error"] = str(persistence)
        raise DatasetReadError(error, receipt) from error


def fit_pusht_training_scaler(
    archive: Path, output: Path, *, row_profile: str = LEGACY_ROW_PROFILE
) -> FittedActionScaler:
    """Fit only the fixed complete 655cd446 training archive; never a caller hash."""
    return _fit_archive(Path(archive), Path(output), _TRAINING, row_profile=row_profile)


def fit_control_archive_scaler(
    archive: Path,
    output: Path,
    *,
    decoded_bytes: int,
    source_id: str,
    row_profile: str = LEGACY_ROW_PROFILE,
) -> FittedActionScaler:
    """Exercise the same transaction on a tiny archive, always with control scope."""
    _row_byte_limit(row_profile)
    if (
        type(decoded_bytes) is not int
        or not 1 <= decoded_bytes <= CONTROL_DECODED_BYTES
        or type(source_id) is not str
        or re.fullmatch(r"[a-zA-Z0-9_-]{1,64}", source_id) is None
    ):
        raise ValueError("A bounded control extent and source identifier are required")
    verify_runtime()
    descriptor, before = _open_regular(Path(archive), CONTROL_ARCHIVE_BYTES)
    try:
        digest = _hash_descriptor(descriptor, before.st_size)
        if _identity(os.fstat(descriptor)) != _identity(before):
            raise ValueError("Control archive changed before admission")
    finally:
        os.close(descriptor)
    profile = _ArchiveProfile(
        "synthetic_control_only", source_id, before.st_size, digest, decoded_bytes
    )
    return _fit_archive(Path(archive), Path(output), profile, row_profile=row_profile)
