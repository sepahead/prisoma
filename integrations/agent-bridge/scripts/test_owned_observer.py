"""Construction and real owned-child controls for the read-only Darwin observer.

Run beside the SDK-built darwin_observer binary. Native controls create only
their own bounded, cooperative process tree and retire it through pipe closure.
No signal is sent. These controls do not qualify an application process owner.
"""

from copy import deepcopy
import os
from pathlib import Path
import select
import subprocess
import sys
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from owned_observer import (
    MAX_EVENTS,
    MAX_IDENTITIES,
    MAX_OUTPUT_BYTES,
    MAX_ROSTER,
    Observer,
    ObserverError,
    OwnedTree,
    birth,
)


def row(pid=777, ppid=None, seconds=1790000000, **changes):
    return {
        "pid": pid,
        "ppid": os.getpid() if ppid is None else ppid,
        "uid": os.getuid(),
        "status": 2,
        "start_seconds": seconds,
        "start_microseconds": 1,
        **changes,
    }


def envelope(rows=None, unavailable=None):
    return {
        "schema": "local.darwin-process-observation.v1",
        "bsdinfo_bytes": 136,
        "rows": [row()] if rows is None else rows,
        "unavailable": [] if unavailable is None else unavailable,
    }


class FakeObserver(Observer):
    def __init__(self, value):
        self.value = value

    def command(self, *args):
        return deepcopy(self.value)


def tree(snapshot=None):
    return OwnedTree(
        None,
        SimpleNamespace(pid=777, returncode=None),
        ({777: row()}, set()) if snapshot is None else snapshot,
    )


class SchemaControls(unittest.TestCase):
    def test_exact_snapshot_and_selected_positive_boundaries(self):
        for status in range(1, 6):
            observed = row(status=status, start_microseconds=999999)
            self.assertEqual(
                FakeObserver(envelope([observed])).snapshot(),
                ({777: observed}, set()),
            )
        limit = row(
            pid=2**31 - 1,
            ppid=0,
            uid=2**32 - 1,
            seconds=2**64 - 1,
            start_microseconds=0,
        )
        self.assertEqual(
            FakeObserver(envelope([limit])).snapshot()[0][limit["pid"]], limit
        )
        selected = {"identity": row(), "executable_path_hex": b"/path/\xff".hex()}
        self.assertEqual(FakeObserver(selected).selected(birth(row())), selected)

    def test_snapshot_envelope_is_closed_and_exact(self):
        invalid = [None, [], True, {**envelope(), "extra": 1}]
        for key in envelope():
            value = envelope()
            del value[key]
            invalid.append(value)
        invalid.extend(
            {**envelope(), "bsdinfo_bytes": v} for v in (True, 136.0, "136", 135)
        )
        invalid.append({**envelope(), "schema": "other"})
        for value in invalid:
            with self.subTest(value=value), self.assertRaises(ObserverError):
                FakeObserver(value).snapshot()

    def test_rows_reject_wrong_types_bounds_and_members(self):
        invalid = [None, [], {**row(), "extra": 1}]
        for key in row():
            value = row()
            del value[key]
            invalid.append(value)
            for bad in (True, "1", 1.0, None):
                invalid.append({**row(), key: bad})
        for key, bad_values in {
            "pid": (0, -1, 2**31),
            "ppid": (-1, 2**31),
            "uid": (-1, 2**32),
            "status": (0, 6),
            "start_seconds": (0, -1, 2**64),
            "start_microseconds": (-1, 1000000),
        }.items():
            invalid.extend({**row(), key: value} for value in bad_values)
        for value in invalid:
            with self.subTest(value=value), self.assertRaises(ObserverError):
                FakeObserver(envelope([value])).snapshot()

    def test_rosters_reject_duplicates_overlap_and_wrong_types(self):
        invalid = [
            envelope([row(), row()]),
            envelope(unavailable=[777]),
            envelope(unavailable=[778, 778]),
            *[envelope(unavailable=[v]) for v in (True, 0, -1, 2**31, "777", None, [])],
            *[{**envelope(), "unavailable": v} for v in ({}, (), set(), None)],
            *[{**envelope(), "rows": v} for v in ({}, (), set(), None)],
        ]
        for value in invalid:
            with self.subTest(value=value), self.assertRaises(ObserverError):
                FakeObserver(value).snapshot()

    def test_total_roster_boundary_counts_unavailable_and_rows(self):
        unavailable = list(range(1, MAX_ROSTER + 1))
        rows, denied = FakeObserver(envelope([], unavailable)).snapshot()
        self.assertFalse(rows)
        self.assertEqual(len(denied), MAX_ROSTER)
        with self.assertRaises(ObserverError):
            FakeObserver(envelope([], unavailable + [MAX_ROSTER + 1])).snapshot()
        with self.assertRaises(ObserverError):
            FakeObserver(envelope([row(MAX_ROSTER + 1)], unavailable)).snapshot()

    def test_selected_observation_rejects_drift_and_malformed_paths(self):
        valid = {"identity": row(), "executable_path_hex": b"/path".hex()}
        invalid = [
            None,
            [],
            {**valid, "extra": True},
            {**valid, "identity": row(seconds=1790000001)},
        ]
        for path in (None, "", "0", "zz", "2F", "2f 61", "61", "2f00", "2f" * 4096):
            invalid.append({**valid, "executable_path_hex": path})
        for value in invalid:
            with self.subTest(value=value), self.assertRaises(ObserverError):
                FakeObserver(value).selected(birth(row()))
        for identity in ([777, 1, 1], (True, 1, 1), (777, 0, 1), (777, 1, 1000000)):
            with self.subTest(identity=identity), self.assertRaises(ObserverError):
                FakeObserver(valid).selected(identity)

    def test_duplicate_json_failed_invocation_and_byte_ceiling(self):
        observer = Observer(sys.executable)
        for raw, code in (
            (b'{"pid":1,"pid":2}', 0),
            (b"bad", 0),
            (b"{}", 1),
            (b"x" * (MAX_OUTPUT_BYTES + 1), 0),
        ):
            result = SimpleNamespace(returncode=code, stdout=raw)
            with (
                self.subTest(code=code, bytes=len(raw)),
                patch(
                    "owned_observer.subprocess.run",
                    return_value=result,
                ),
                self.assertRaises(ObserverError),
            ):
                observer.command("snapshot")
        with patch(
            "owned_observer.subprocess.run",
            return_value=SimpleNamespace(
                returncode=0,
                stdout=b'{"value":1}',
            ),
        ):
            self.assertEqual(observer.command("snapshot"), {"value": 1})


class TrackerControls(unittest.TestCase):
    def test_multilevel_discovery_ignores_unrelated_rows(self):
        root, child, grandchild = row(), row(778, 777), row(779, 778)
        snapshot = ({779: grandchild, 780: row(780), 778: child, 777: root}, set())
        tracked = tree(snapshot)
        self.assertEqual(
            set(tracked.known), {birth(root), birth(child), birth(grandchild)}
        )
        self.assertFalse(tracked.receipt(snapshot)["observed_identities_retired"])
        final = tracked.receipt(({}, set()))
        self.assertTrue(final["observed_identities_retired"])
        self.assertEqual(len(final["events"]), 6)

    def test_direct_child_join_and_raw_snapshot_validation(self):
        invalid = [
            ({777: row(ppid=0)}, set()),
            ({777: row(uid=os.getuid() + 1)}, set()),
            ({}, {777}),
            ({778: row()}, set()),
            ({777: row()}, {"777"}),
            [{777: row()}, set()],
        ]
        for snapshot in invalid:
            with self.subTest(snapshot=snapshot), self.assertRaises(ObserverError):
                tree(snapshot)
        for child in (
            SimpleNamespace(pid=777, returncode=0),
            SimpleNamespace(pid=True, returncode=None),
        ):
            with self.assertRaises(ObserverError):
                OwnedTree(None, child, ({777: row()}, set()))

    def test_zombie_identity_remains_active_until_absent(self):
        tracked = tree()
        self.assertFalse(
            tracked.receipt(({777: row(status=5)}, set()))[
                "observed_identities_retired"
            ]
        )
        self.assertTrue(tracked.receipt(({}, set()))["observed_identities_retired"])

    def test_denied_known_identity_permanently_prevents_retirement(self):
        tracked = tree()
        with self.assertRaisesRegex(ObserverError, "unobservable"):
            tracked.update(({}, {777}))
        with self.assertRaisesRegex(ObserverError, "prior observation failure"):
            tracked.receipt(({}, set()))

    def test_reappearance_fails_without_unbounded_event_growth(self):
        tracked = tree()
        tracked.update(({}, set()))
        with self.assertRaisesRegex(ObserverError, "reappeared"):
            tracked.update(({777: row()}, set()))
        for unused in range(10):
            with self.assertRaises(ObserverError):
                tracked.receipt(({}, set()))
        self.assertEqual(len(tracked.events), 2)

    def test_descendant_birth_and_uid_inconsistency_fail(self):
        for descendant in (
            row(778, 777, seconds=1),
            row(778, 777, uid=os.getuid() + 1),
        ):
            tracked = tree()
            with self.subTest(row=descendant), self.assertRaises(ObserverError):
                tracked.update(({777: row(), 778: descendant}, set()))
            with self.assertRaises(ObserverError):
                tracked.receipt(({}, set()))

    def test_exact_identity_and_event_capacity_and_overflow(self):
        rows = {777: row()}
        rows.update({1000 + i: row(1000 + i, 777) for i in range(MAX_IDENTITIES - 1)})
        tracked = tree((rows, set()))
        self.assertEqual(len(tracked.known), MAX_IDENTITIES)
        receipt = tracked.receipt(({}, set()))
        self.assertTrue(receipt["observed_identities_retired"])
        self.assertEqual(len(receipt["events"]), MAX_EVENTS)
        overflowing = tree()
        rows[2000] = row(2000, 777)
        with self.assertRaisesRegex(ObserverError, "capacity exceeded"):
            overflowing.update((rows, set()))
        with self.assertRaises(ObserverError):
            overflowing.receipt(({}, set()))

    def test_reused_descendant_pid_keeps_distinct_birth_knowledge(self):
        first = row(778, 777)
        tracked = tree(({777: row(), 778: first}, set()))
        second = row(778, 777, seconds=1790000001)
        active = tracked.update(({777: row(), 778: second}, set()))
        self.assertIn(birth(second), active)
        self.assertNotIn(birth(first), active)
        self.assertEqual(len(tracked.known), 3)
        receipt = tracked.receipt(({}, set()))
        self.assertEqual(len(receipt["events"]), 6)
        self.assertTrue(receipt["observed_identities_retired"])

    def test_receipt_mutation_cannot_change_retained_event_knowledge(self):
        tracked = tree()
        receipt = tracked.receipt(({}, set()))
        receipt["events"][0]["identity"]["pid"] = -1
        receipt["events"][1]["birth"][0] = -1
        retained = tracked.receipt(({}, set()))
        self.assertEqual(retained["events"][0]["identity"]["pid"], 777)
        self.assertEqual(retained["events"][1]["birth"][0], 777)


@unittest.skipUnless(sys.platform == "darwin", "native Darwin SDK observer required")
class NativeControls(unittest.TestCase):
    def test_owned_root_child_zombie_and_natural_retirement(self):
        observer = Observer(Path(__file__).with_name("darwin_observer"))
        script = """import subprocess, sys
child = subprocess.Popen([sys.executable, "-I", "-B", "-c", "import sys; sys.stdin.buffer.readline()"], stdin=subprocess.PIPE)
try:
    print(child.pid, flush=True)
    sys.stdin.buffer.readline()
    child.stdin.close()
    print("child_released", flush=True)
    sys.stdin.buffer.readline()
    child.wait(timeout=10)
    print("child_reaped", flush=True)
    sys.stdin.buffer.readline()
finally:
    if child.stdin is not None:
        child.stdin.close()
    child.wait(timeout=10)
"""
        owned = subprocess.Popen(
            [sys.executable, "-I", "-B", "-c", script],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        def line():
            ready, _, _ = select.select([owned.stdout], [], [], 10)
            self.assertTrue(ready, "owned control handshake timed out")
            value = owned.stdout.readline(256)
            self.assertTrue(
                value.endswith(b"\n"), "owned control handshake was incomplete"
            )
            return value.strip()

        def send():
            owned.stdin.write(b"continue\n")
            owned.stdin.flush()

        def until(predicate):
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                snapshot = observer.snapshot()
                if predicate(snapshot):
                    return snapshot
                time.sleep(0.01)
            self.fail("owned process state was not observed before the deadline")

        try:
            child_pid = int(line())
            snapshot = until(
                lambda value: owned.pid in value[0] and child_pid in value[0]
            )
            tracked = OwnedTree(observer, owned, snapshot)
            self.assertEqual(len(tracked.known), 2)
            selected = observer.selected(tracked.root)
            self.assertEqual(birth(selected["identity"]), tracked.root)
            with self.assertRaises(ObserverError):
                observer.selected(
                    (tracked.root[0], tracked.root[1] + 1, tracked.root[2])
                )
            send()
            self.assertEqual(line(), b"child_released")
            zombie = until(lambda value: value[0].get(child_pid, {}).get("status") == 5)
            self.assertIn(
                child_pid, {identity[0] for identity in tracked.update(zombie)}
            )
            self.assertFalse(tracked.receipt(zombie)["observed_identities_retired"])
            send()
            self.assertEqual(line(), b"child_reaped")
            reaped = until(
                lambda value: child_pid not in value[0] and child_pid not in value[1]
            )
            self.assertEqual(tracked.update(reaped), {tracked.root})
            send()
            root_zombie = until(
                lambda value: value[0].get(owned.pid, {}).get("status") == 5
            )
            self.assertEqual(tracked.update(root_zombie), {tracked.root})
            self.assertEqual(owned.wait(timeout=10), 0)
            final = tracked.receipt(observer.snapshot())
            self.assertTrue(final["observed_identities_retired"])
            self.assertEqual(final["observed_identity_count"], 2)
            self.assertEqual(len(final["events"]), 4)
            self.assertEqual(final["signals_sent_by_observer"], 0)
        finally:
            owned.stdin.close()
            self.assertEqual(owned.wait(timeout=15), 0)
            owned.stdout.close()
            owned.stderr.close()


if __name__ == "__main__":
    unittest.main(verbosity=2)
