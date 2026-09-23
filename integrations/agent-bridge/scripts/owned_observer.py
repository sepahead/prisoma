"""Read-only birth-bound observations of a directly owned Darwin process tree.

The observer proves only disappearance of the identities it actually observed.
Polling cannot establish hostile-process containment or discover every short
lived descendant. Application retirement receipts remain separate obligations.
The caller exclusively owns reaping of the root and selects the reviewed,
SDK-built C helper. Its fixed output bound precedes Python's capture operation.
An inconsistent observation permanently prevents this tracker from certifying
retirement. A fresh tracker cannot adopt an earlier tracker's evidence.
"""

from copy import deepcopy
import json
import os
from pathlib import Path
import subprocess

MAX_ROSTER = 32767
MAX_IDENTITIES = 256
MAX_EVENTS = 2 * MAX_IDENTITIES
MAX_OUTPUT_BYTES = 8 * 1024**2
ROW_FIELDS = {"pid", "ppid", "uid", "status", "start_seconds", "start_microseconds"}


def birth(row):
    return row["pid"], row["start_seconds"], row["start_microseconds"]


class ObserverError(RuntimeError):
    pass


def _integer(value, low, high):
    return type(value) is int and low <= value <= high


def _row(row):
    if (
        type(row) is not dict
        or set(row) != ROW_FIELDS
        or not _integer(row["pid"], 1, 2**31 - 1)
        or not _integer(row["ppid"], 0, 2**31 - 1)
        or not _integer(row["uid"], 0, 2**32 - 1)
        or not _integer(row["status"], 1, 5)
        or not _integer(row["start_seconds"], 1, 2**64 - 1)
        or not _integer(row["start_microseconds"], 0, 999999)
    ):
        raise ObserverError("invalid observer identity row")
    return dict(row)


def _snapshot_parts(snapshot):
    if type(snapshot) is not tuple or len(snapshot) != 2:
        raise ObserverError("invalid observer snapshot")
    rows, unavailable = snapshot
    if (
        type(rows) is not dict
        or type(unavailable) is not set
        or len(rows) + len(unavailable) > MAX_ROSTER
        or any(not _integer(pid, 1, 2**31 - 1) for pid in unavailable)
    ):
        raise ObserverError("invalid observer roster")
    checked = {}
    for pid, value in rows.items():
        row = _row(value)
        if not _integer(pid, 1, 2**31 - 1) or pid != row["pid"] or pid in unavailable:
            raise ObserverError("observer roster identity differs")
        checked[pid] = row
    return checked, set(unavailable)


def _json_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ObserverError("duplicate observer JSON member")
        result[key] = value
    return result


class Observer:
    def __init__(self, executable):
        self.executable = Path(executable).resolve(strict=True)

    def command(self, *args):
        completed = subprocess.run(
            [str(self.executable), *map(str, args)],
            capture_output=True,
            timeout=5,
            check=False,
        )
        if completed.returncode or len(completed.stdout) > MAX_OUTPUT_BYTES:
            raise ObserverError("bounded observer invocation failed")
        try:
            return json.loads(completed.stdout, object_pairs_hook=_json_object)
        except (UnicodeDecodeError, ValueError, RecursionError) as error:
            raise ObserverError("invalid observer JSON") from error

    def snapshot(self):
        value = self.command("snapshot")
        if (
            type(value) is not dict
            or set(value) != {"schema", "bsdinfo_bytes", "rows", "unavailable"}
            or value["schema"] != "local.darwin-process-observation.v1"
            or type(value["bsdinfo_bytes"]) is not int
            or value["bsdinfo_bytes"] != 136
        ):
            raise ObserverError("unqualified observer ABI")
        rows, unavailable = value["rows"], value["unavailable"]
        if (
            type(rows) is not list
            or type(unavailable) is not list
            or len(rows) + len(unavailable) > MAX_ROSTER
            or any(not _integer(pid, 1, 2**31 - 1) for pid in unavailable)
            or len(set(unavailable)) != len(unavailable)
        ):
            raise ObserverError("invalid observer roster")
        checked = {}
        for value in rows:
            row = _row(value)
            if row["pid"] in checked:
                raise ObserverError("duplicate observer identity")
            checked[row["pid"]] = row
        return _snapshot_parts((checked, set(unavailable)))

    def selected(self, identity):
        if (
            type(identity) is not tuple
            or len(identity) != 3
            or not _integer(identity[0], 1, 2**31 - 1)
            or not _integer(identity[1], 1, 2**64 - 1)
            or not _integer(identity[2], 0, 999999)
        ):
            raise ObserverError("invalid selected process identity")
        result = self.command("selected", identity[0])
        if type(result) is not dict or set(result) != {
            "identity",
            "executable_path_hex",
        }:
            raise ObserverError("invalid selected process observation")
        if birth(_row(result["identity"])) != identity:
            raise ObserverError("selected process birth differs")
        path = result["executable_path_hex"]
        if (
            type(path) is not str
            or not 2 <= len(path) <= 8190
            or len(path) % 2
            or any(c not in "0123456789abcdef" for c in path)
        ):
            raise ObserverError("invalid selected process path")
        decoded = bytes.fromhex(path)
        if not decoded.startswith(b"/") or b"\0" in decoded:
            raise ObserverError("invalid selected process path")
        return result


class OwnedTree:
    def __init__(self, observer, child, snapshot):
        if child.returncode is not None or not _integer(child.pid, 1, 2**31 - 1):
            raise ObserverError("an unreaped direct child is required")
        rows, unavailable = _snapshot_parts(snapshot)
        root = rows.get(child.pid)
        if (
            child.pid in unavailable
            or root is None
            or root["ppid"] != os.getpid()
            or root["uid"] != os.getuid()
        ):
            raise ObserverError("direct child identity could not be established")
        self.observer = observer
        self.root = birth(root)
        self.known = {self.root: dict(root)}
        self.events = [{"kind": "observed", "identity": dict(root)}]
        self.absent = set()
        self.failed = False
        self.update(snapshot)

    def update(self, snapshot):
        if self.failed:
            raise ObserverError("a prior observation failure prevents retirement proof")
        try:
            return self._update(snapshot)
        except ObserverError:
            self.failed = True
            raise

    def _event(self, event):
        if len(self.events) >= MAX_EVENTS:
            raise ObserverError("observed event capacity exceeded")
        self.events.append(event)

    def _update(self, snapshot):
        rows, unavailable = _snapshot_parts(snapshot)
        if any(identity[0] in unavailable for identity in self.known):
            raise ObserverError("a known process became unobservable")
        active = {
            identity
            for identity in self.known
            if identity[0] in rows and birth(rows[identity[0]]) == identity
        }
        if active & self.absent:
            raise ObserverError("a previously absent process identity reappeared")
        changed = True
        while changed:
            changed = False
            for row in rows.values():
                identity = birth(row)
                if identity in self.known:
                    continue
                parent = rows.get(row["ppid"])
                if parent is None or birth(parent) not in active:
                    continue
                if row["uid"] != os.getuid() or identity[1:] < birth(parent)[1:]:
                    raise ObserverError("descendant ownership or birth is inconsistent")
                if len(self.known) >= MAX_IDENTITIES:
                    raise ObserverError("observed descendant capacity exceeded")
                self.known[identity] = dict(row)
                self._event({"kind": "observed", "identity": dict(row)})
                active.add(identity)
                changed = True
        missing = set(self.known) - active
        for identity in sorted(missing - self.absent):
            self._event({"kind": "identity_absent", "birth": list(identity)})
        self.absent.update(missing)
        return active

    def receipt(self, snapshot):
        active = self.update(snapshot)
        return {
            "schema": "local.observed-owned-tree.v1",
            "observed_identity_count": len(self.known),
            "observed_identities_retired": not active,
            "remaining_births": [list(identity) for identity in sorted(active)],
            "events": deepcopy(self.events),
            "hostile_process_containment": False,
            "signals_sent_by_observer": 0,
            "independent_application_receipt": False,
        }
