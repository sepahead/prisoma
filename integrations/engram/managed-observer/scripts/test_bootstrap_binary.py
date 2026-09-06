#!/usr/bin/env python3
"""Filesystem controls for portable bootstrap materialization; no model execution."""

from __future__ import annotations

import errno
import importlib.util
import os
import stat
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from source_provenance import snapshot_regular_file

SPEC = importlib.util.spec_from_file_location(
    "bootstrap_binary", Path(__file__).with_name("prepare-bootstrap-observer.py")
)
assert SPEC is not None and SPEC.loader is not None
BOOTSTRAP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOOTSTRAP)


class BootstrapBinaryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.binary = self.root / "crates/observer/target/release/observer"
        self.binary.parent.mkdir(parents=True)
        self.payload = b"bounded synthetic executable bytes\n"
        self.binary.write_bytes(self.payload)
        self.binary.chmod(0o755)
        for name, value in (
            ("ROOT", self.root),
            ("RELEASE_BINARY", self.binary),
            ("source_observation", lambda: {"fixture_context": "unchanged"}),
        ):
            p = patch.object(BOOTSTRAP, name, value)
            p.start()
            self.addCleanup(p.stop)

    def assert_no_temporary(self) -> None:
        self.assertEqual(list(self.binary.parent.glob(".*.bootstrap-*")), [])

    def test_single_link_produces_exact_independent_private_inode(self) -> None:
        before = self.binary.stat()
        result = BOOTSTRAP.prepare_binary(self.binary)
        after = self.binary.stat()
        self.assertNotEqual(
            (before.st_dev, before.st_ino), (after.st_dev, after.st_ino)
        )
        self.assertEqual(after.st_nlink, 1)
        self.assertEqual(stat.S_IMODE(after.st_mode), 0o700)
        self.assertEqual(snapshot_regular_file(self.binary, 1024), self.payload)
        self.assertEqual(result["source_identity_before"][3], 1)
        self.assertFalse(result["compiled_source_attested"])
        self.assertFalse(result["observed_build_authority"])
        self.assertFalse(result["package_authority"])
        self.assert_no_temporary()

    def test_cargo_alias_is_preserved_and_cannot_mutate_staged_bytes(self) -> None:
        counterpart = self.binary.parent / "cargo-deps-counterpart"
        os.link(self.binary, counterpart)
        original = counterpart.stat()
        with self.assertRaises(ValueError):
            snapshot_regular_file(self.binary, 1024)
        result = BOOTSTRAP.prepare_binary(self.binary)
        self.assertEqual(result["source_identity_before"][3], 2)
        self.assertEqual(counterpart.stat().st_ino, original.st_ino)
        self.assertEqual(counterpart.read_bytes(), self.payload)
        self.assertEqual(counterpart.stat().st_nlink, 1)
        self.assertEqual(stat.S_IMODE(counterpart.stat().st_mode), 0o755)
        counterpart.write_bytes(b"a later Cargo build changes the retained old inode\n")
        self.assertEqual(snapshot_regular_file(self.binary, 1024), self.payload)
        self.assert_no_temporary()

    def test_wrong_fixed_output_rejects_without_mutation(self) -> None:
        other = self.binary.with_name("other")
        other.write_bytes(self.payload)
        with self.assertRaises(ValueError):
            BOOTSTRAP.prepare_binary(other)
        self.assertEqual(other.read_bytes(), self.payload)
        self.assertEqual(self.binary.read_bytes(), self.payload)

    def test_symlink_output_rejects(self) -> None:
        other = self.binary.with_name("real")
        self.binary.rename(other)
        self.binary.symlink_to(other)
        with self.assertRaises(ValueError):
            BOOTSTRAP.prepare_binary(self.binary)
        self.assertTrue(self.binary.is_symlink())
        self.assertEqual(other.read_bytes(), self.payload)

    def test_symlink_parent_rejects(self) -> None:
        parent = self.binary.parent
        moved = parent.with_name("moved-release")
        parent.rename(moved)
        parent.symlink_to(moved, target_is_directory=True)
        with self.assertRaises(ValueError):
            BOOTSTRAP.prepare_binary(self.binary)

    def test_intermediate_directory_swap_at_open_rejects(self) -> None:
        self.check_directory_swap_at_open(link=False)

    def test_intermediate_symlink_swap_at_open_rejects(self) -> None:
        self.check_directory_swap_at_open(link=True)

    def check_directory_swap_at_open(self, *, link: bool) -> None:
        opened = os.open
        target = self.binary.parent.parent
        retained = target.with_name("retained-target")
        replacement = target.with_name("replacement-target")
        replacement.joinpath("release").mkdir(parents=True)
        alternate = replacement / "release/observer"
        alternate.write_bytes(b"a different parent owns these bytes")
        alternate.chmod(0o755)
        changed = False

        def swap(path, flags, mode=0o777, *, dir_fd=None):
            nonlocal changed
            if (
                path in ("target", self.binary.parent)
                and flags & os.O_DIRECTORY
                and not changed
            ):
                changed = True
                target.rename(retained)
                if link:
                    target.symlink_to(replacement, target_is_directory=True)
                else:
                    replacement.rename(target)
            return opened(path, flags, mode, dir_fd=dir_fd)

        with patch.object(BOOTSTRAP.os, "open", side_effect=swap):
            with self.assertRaises((OSError, ValueError)):
                BOOTSTRAP.prepare_binary(self.binary)
        self.assertTrue(changed)
        self.assertEqual(
            retained.joinpath("release/observer").read_bytes(), self.payload
        )
        self.assertEqual(
            self.binary.read_bytes(), b"a different parent owns these bytes"
        )
        self.assertEqual(list(self.root.rglob(".*.bootstrap-*")), [])

    def test_nonregular_outputs_reject_before_blocking_open(self) -> None:
        self.binary.unlink()
        self.binary.mkdir()
        with self.assertRaises(ValueError):
            BOOTSTRAP.prepare_binary(self.binary)
        self.binary.rmdir()
        os.mkfifo(self.binary, 0o700)
        with self.assertRaises(ValueError):
            BOOTSTRAP.prepare_binary(self.binary)

    def test_shared_writable_output_rejects(self) -> None:
        self.binary.chmod(0o777)
        with self.assertRaises(ValueError):
            BOOTSTRAP.prepare_binary(self.binary)
        self.assertEqual(stat.S_IMODE(self.binary.stat().st_mode), 0o777)

    def test_nonexecutable_output_rejects(self) -> None:
        self.binary.chmod(0o600)
        with self.assertRaises(ValueError):
            BOOTSTRAP.prepare_binary(self.binary)

    def test_foreign_owner_rejects(self) -> None:
        with self.binary.open("rb") as source:
            with patch.object(BOOTSTRAP.os, "geteuid", return_value=os.geteuid() + 1):
                with self.assertRaises(ValueError):
                    BOOTSTRAP.read_cargo_output(source.fileno())

    def test_shared_writable_parent_rejects(self) -> None:
        self.binary.parent.chmod(0o777)
        with self.assertRaises(ValueError):
            BOOTSTRAP.prepare_binary(self.binary)

    def test_size_bound_rejects_before_copy(self) -> None:
        with patch.object(BOOTSTRAP, "MAX_BINARY_BYTES", len(self.payload) - 1):
            with self.assertRaises(ValueError):
                BOOTSTRAP.prepare_binary(self.binary)
        self.assert_no_temporary()

    def test_growth_during_capture_rejects(self) -> None:
        read = os.read
        changed = False

        def grow(descriptor: int, count: int) -> bytes:
            nonlocal changed
            value = read(descriptor, count)
            if not changed:
                changed = True
                with self.binary.open("ab") as writer:
                    writer.write(b"later bytes")
            return value

        with patch.object(BOOTSTRAP.os, "read", side_effect=grow):
            with self.assertRaises(ValueError):
                BOOTSTRAP.prepare_binary(self.binary)
        self.assert_no_temporary()

    def test_named_source_replacement_rejects(self) -> None:
        read = os.read
        changed = False

        def replace(descriptor: int, count: int) -> bytes:
            nonlocal changed
            value = read(descriptor, count)
            if not changed:
                changed = True
                replacement = self.binary.with_name("replacement")
                replacement.write_bytes(b"new named inode")
                replacement.chmod(0o700)
                replacement.replace(self.binary)
            return value

        with patch.object(BOOTSTRAP.os, "read", side_effect=replace):
            with self.assertRaises(ValueError):
                BOOTSTRAP.prepare_binary(self.binary)
        self.assertEqual(self.binary.read_bytes(), b"new named inode")
        self.assert_no_temporary()

    def test_changed_copy_rejects_before_publication(self) -> None:
        sync = os.fsync

        def alter(descriptor: int) -> None:
            sync(descriptor)
            os.pwrite(descriptor, b"X", 0)

        with patch.object(BOOTSTRAP.os, "fsync", side_effect=alter):
            with self.assertRaises(ValueError):
                BOOTSTRAP.prepare_binary(self.binary)
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assert_no_temporary()

    def test_name_collision_does_not_remove_foreign_temporary(self) -> None:
        occupied = self.binary.parent / f".{self.binary.name}.bootstrap-fixed"
        occupied.write_bytes(b"retained other file")
        with patch.object(BOOTSTRAP.secrets, "token_hex", return_value="fixed"):
            with self.assertRaises(FileExistsError):
                BOOTSTRAP.prepare_binary(self.binary)
        self.assertEqual(occupied.read_bytes(), b"retained other file")

    def test_close_failure_attempts_every_owned_cleanup_once(self) -> None:
        self.check_close_failure(primary_failure=False)

    def test_close_failure_preserves_primary_rejection(self) -> None:
        self.check_close_failure(primary_failure=True)

    def check_close_failure(self, *, primary_failure: bool) -> None:
        opened = os.open
        closed = os.close
        owned: list[int] = []
        attempts: list[int] = []
        failed = False

        def record(path, flags, mode=0o777, *, dir_fd=None):
            descriptor = opened(path, flags, mode, dir_fd=dir_fd)
            if flags & os.O_DIRECTORY or (
                isinstance(path, str)
                and (path == self.binary.name or ".bootstrap-" in path)
            ):
                owned.append(descriptor)
            return descriptor

        def fail_once(descriptor):
            nonlocal failed
            closed(descriptor)
            if descriptor in owned:
                attempts.append(descriptor)
                if not failed:
                    failed = True
                    raise OSError(errno.EIO, "injected ambiguous close failure")

        context = [{"fixture_context": "unchanged"}] * 3
        if primary_failure:
            context[1] = {"fixture_context": "changed"}
        before = self.binary.stat().st_ino
        with (
            patch.object(BOOTSTRAP.os, "open", side_effect=record),
            patch.object(BOOTSTRAP.os, "close", side_effect=fail_once),
            patch.object(BOOTSTRAP, "source_observation", side_effect=context),
        ):
            with self.assertRaises(
                ValueError if primary_failure else OSError
            ) as raised:
                BOOTSTRAP.prepare_binary(self.binary)
        self.assertTrue(failed)
        self.assertEqual(attempts, list(reversed(owned)))
        for descriptor in owned:
            with self.assertRaises(OSError):
                os.fstat(descriptor)
        if primary_failure:
            self.assertIn("source observation changed", str(raised.exception))
            self.assertIn("cleanup also failed", raised.exception.__notes__[0])
            self.assertEqual(self.binary.stat().st_ino, before)
        else:
            # Replacement occurred, but cleanup failed: no successful receipt exists.
            self.assertNotEqual(self.binary.stat().st_ino, before)
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assert_no_temporary()

    def test_source_observation_drift_rejects(self) -> None:
        with patch.object(
            BOOTSTRAP,
            "source_observation",
            side_effect=[{"index": "a"}, {"index": "b"}],
        ):
            with self.assertRaises(ValueError):
                BOOTSTRAP.prepare_binary(self.binary)
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assert_no_temporary()


if __name__ == "__main__":
    unittest.main()
