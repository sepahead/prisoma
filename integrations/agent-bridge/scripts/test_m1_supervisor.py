"""Checkpoint, publication, and owned-child supervisor controls.

Set PRISOMA_M1_OBSERVER_BINARY to the separately qualified Darwin observer for
native controls. These synthetic workers perform no simulator operation.
"""

import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "m1_supervisor", Path(__file__).with_name("m1_supervisor.py")
)
s = importlib.util.module_from_spec(spec)
spec.loader.exec_module(s)


def checkpoint():
    return {
        "schema": "prisoma.m1-checkpoint.v1",
        "freeze_sha256": "a" * 64,
        "case_id": "case",
        "stage": "prepared",
        "run_id": "run",
        "endpoint_id": "endpoint",
        "generation": 1,
    }


class SupervisorControls(unittest.TestCase):
    def test_checkpoint_accepts_exact_join_and_rejects_every_changed_member(self):
        selected = checkpoint()
        binding = {
            "binding": {
                key: selected[key] for key in ("run_id", "endpoint_id", "generation")
            }
        }
        raw = s.c.canonical(selected) + b"\n"
        result = s.checkpoint_message(raw, binding, "a" * 64, "case", "prepared")
        self.assertEqual(result, {**selected, "schema": "prisoma.m1-continue.v1"})
        for key in selected:
            changed = {**selected, key: "different"}
            with self.subTest(key=key), self.assertRaises(s.c.CampaignError):
                s.checkpoint_message(
                    s.c.canonical(changed) + b"\n",
                    binding,
                    "a" * 64,
                    "case",
                    "prepared",
                )
        for changed in (
            {**selected, "generation": True},
            {**selected, "generation": 1.0},
            {**selected, "extra": 0},
        ):
            with self.assertRaises(s.c.CampaignError):
                s.checkpoint_message(
                    s.c.canonical(changed) + b"\n",
                    binding,
                    "a" * 64,
                    "case",
                    "prepared",
                )

    def test_checkpoint_rejects_partial_multiline_duplicate_and_overflow(self):
        selected = checkpoint()
        binding = {"binding": selected}
        raw = s.c.canonical(selected)
        for invalid in (
            raw,
            raw + b"\n\n",
            raw + b"\n" + raw + b"\n",
            b" " * 4096 + b"\n",
            b'{"schema":"bad",' + raw[1:] + b"\n",
        ):
            with self.subTest(raw=invalid[:40]), self.assertRaises(s.c.CampaignError):
                s.checkpoint_message(invalid, binding, "a" * 64, "case", "prepared")

    def test_publication_is_exclusive_and_rejects_symlink(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            root.chmod(0o700)
            identity = s.publish_json(root, "receipt.json", {"ok": True})
            self.assertEqual(s.c.selected_file(identity, base=root), b'{"ok":true}\n')
            with self.assertRaises(FileExistsError):
                s.publish_json(root, "receipt.json", {"ok": False})
            (root / "link.json").symlink_to(root / "receipt.json")
            with self.assertRaises(FileExistsError):
                s.publish_json(root, "link.json", {"ok": False})
            self.assertEqual((root / "receipt.json").read_bytes(), b'{"ok":true}\n')
            with self.assertRaises(s.c.CampaignError):
                s.publish_json(root, "../escape.json", {})

    def test_renderer_selection_requires_one_browser_and_its_exact_active_parent(self):
        rows = {
            2: {
                "pid": 2,
                "status": 2,
                "ppid": 3,
                "start_seconds": 12,
                "start_microseconds": 0,
            },
            3: {
                "pid": 3,
                "status": 2,
                "ppid": 1,
                "start_seconds": 11,
                "start_microseconds": 0,
            },
        }
        tree = SimpleNamespace(
            root=(1, 10, 0), update=lambda _: {(1, 10, 0), (2, 12, 0), (3, 11, 0)}
        )
        paths = {2: "/browser", 3: "/node"}
        observer = SimpleNamespace(
            selected=lambda birth: {
                "executable_path_hex": os.fsencode(paths[birth[0]]).hex()
            }
        )

        def select(executable="/browser", parent="/node"):
            return s.renderer_target(observer, tree, (rows, set()), executable, parent)

        self.assertEqual(select(), (rows[2], rows[3]))
        with self.assertRaises(s.c.CampaignError):
            select("/missing")
        with self.assertRaises(s.c.CampaignError):
            select(parent="/other-node")
        paths[3] = "/browser"
        with self.assertRaises(s.c.CampaignError):
            select()
        paths[3] = "/node"
        rows[3]["status"] = 5
        with self.assertRaises(s.c.CampaignError):
            select()
        rows[3]["status"] = 2
        rows[2]["ppid"] = 99
        with self.assertRaises(s.c.CampaignError):
            select()
        rows[2]["ppid"] = 3
        self.assertEqual(select(), (rows[2], rows[3]))

    def test_failed_fsync_cannot_publish_a_terminal_name(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            root.chmod(0o700)
            with patch.object(s.os, "fsync", side_effect=OSError("injected fsync")):
                with self.assertRaises(OSError):
                    s.publish_json(root, "owner.json", {"ok": True})
            self.assertFalse((root / "owner.json").exists())
            self.assertTrue((root / ".owner.json.pending").exists())

    def test_primary_and_cleanup_failure_objects_are_both_retained(self):
        primary, cleanup = ValueError("primary"), OSError("cleanup")
        self.assertIs(s.joined_failures([primary]), primary)
        joined = s.joined_failures([primary, cleanup])
        self.assertIs(joined.exceptions[0], primary)
        self.assertIs(joined.exceptions[1], cleanup)
        self.assertIsNone(s.joined_failures([]))


FAKE_WORKER = r"""
import argparse,json,os,pathlib,sys
p=argparse.ArgumentParser()
p.add_argument('worker'); p.add_argument('--freeze'); p.add_argument('--case'); p.add_argument('--output')
p.add_argument('--progress-fd',type=int); p.add_argument('--control-fd',type=int)
a=p.parse_args(); f=json.loads(pathlib.Path(a.freeze).read_bytes()); case=f['cases'][0]
out=pathlib.Path(a.output); out.mkdir(mode=0o700); i=out.stat()
b={'schema':'prisoma.m1-case-binding.v1','freeze_sha256':'a'*64,'case':case,'source':f['source'],
   'tools':f['tools'],'runtime':f['runtime'],'workload_sha256':f['workload']['sha256'],
   'environment':f['environments'][case['arm']],'output_path':str(out),
   'output_owner':{'device':i.st_dev,'inode':i.st_ino,'uid':i.st_uid,'mode':0o700},
   'binding':{'run_id':'run','endpoint_id':'endpoint','generation':1}}
(out/'binding.json').write_text(json.dumps(b))
with os.fdopen(a.control_fd,'rb',buffering=0) as control:
 for stage in ('prepared','tick1'):
  m={'schema':'prisoma.m1-checkpoint.v1','freeze_sha256':'a'*64,'case_id':a.case,'stage':stage,**b['binding']}
  if a.case=='bad-checkpoint': m['generation']=True
  os.write(a.progress_fd,json.dumps(m).encode()+b'\n')
  reply=b''
  while not reply.endswith(b'\n'):
   chunk=control.read(1)
   if not chunk: sys.exit(3)
   reply+=chunk
  assert json.loads(reply)=={**m,'schema':'prisoma.m1-continue.v1'}
if a.case=='missing-result': sys.exit(0)
(out/'result.json').write_text('{"synthetic":true}')
"""


@unittest.skipUnless(
    os.environ.get("PRISOMA_M1_OBSERVER_BINARY"), "requires qualified Darwin observer"
)
class NativeOwnerControls(unittest.TestCase):
    def case(self, name, fault="none", fail_journal=False):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            root.chmod(0o700)
            worker = root / "worker.py"
            worker.write_text(FAKE_WORKER)
            interpreter = root / "python-prefix/bin"
            interpreter.mkdir(parents=True)
            (interpreter / "python").symlink_to(Path(sys.executable).resolve())
            freeze = {
                "output_root": str(root),
                "cases": [{"case_id": name, "arm": "body", "fault": fault}],
                "source": {},
                "runtime": {},
                "workload": {"sha256": "b" * 64},
                "environments": {"body": {"prefix": str(interpreter.parent)}},
                "limits": {"session_seconds": 10, "checkpoint_seconds": 2},
                "tools": {
                    "worker": s.c.file_identity(worker),
                    "supervisor": s.c.file_identity(Path(s.__file__).resolve()),
                    "observer_python": s.c.file_identity(
                        Path(s.observations.__file__).resolve()
                    ),
                    "observer_binary": s.c.file_identity(
                        Path(os.environ["PRISOMA_M1_OBSERVER_BINARY"])
                    ),
                },
            }
            selected = root / "freeze.json"
            selected.write_text(json.dumps(freeze))
            isolated = SimpleNamespace(
                flags=SimpleNamespace(isolated=1), dont_write_bytecode=True
            )
            original_write = s.c.Output.write
            secondary = OSError("injected supervisor journal failure")

            def selected_write(writer, name, raw):
                if fail_journal and name == "worker.stdout":
                    raise secondary
                return original_write(writer, name, raw)

            with (
                patch.object(s.c, "load_freeze", return_value=(freeze, "a" * 64)),
                patch.object(s, "sys", isolated),
                patch.object(s.c.Output, "write", selected_write),
            ):
                if fail_journal:
                    expected = (
                        BaseExceptionGroup if name == "bad-checkpoint" else OSError
                    )
                    with self.assertRaises(expected) as caught:
                        s.supervise(selected, name)
                    if expected is BaseExceptionGroup:
                        self.assertIsInstance(
                            caught.exception.exceptions[0], s.c.CampaignError
                        )
                        self.assertIn(secondary, caught.exception.exceptions)
                    else:
                        self.assertIs(caught.exception, secondary)
                    self.assertFalse((root / name / "owner.json").exists())
                elif name in {"bad-checkpoint", "missing-result"}:
                    with self.assertRaises(s.c.CampaignError):
                        s.supervise(selected, name)
                    self.assertFalse((root / name / "owner.json").exists())
                else:
                    s.supervise(selected, name)
                    owner = json.loads((root / name / "owner.json").read_bytes())
                    self.assertEqual(owner["observed_births"], owner["retired_births"])
                    self.assertFalse(owner["emergency_cleanup"])
                    self.assertEqual(
                        owner["worker_returncode"], -9 if fault == "caller_loss" else 0
                    )
                    self.assertEqual(
                        owner["result_sha256"] is None, fault == "caller_loss"
                    )
                terminal = json.loads(
                    (
                        root / (name + ".supervisor") / "supervisor-result.json"
                    ).read_bytes()
                )
                self.assertTrue(
                    terminal["observed_tree"]["observed_identities_retired"]
                )

    def test_healthy_owned_worker_and_natural_retirement(self):
        self.case("healthy")

    def test_caller_fault_preserves_missing_result_and_natural_retirement(self):
        self.case("caller-loss", "caller_loss")

    def test_bad_checkpoint_aborts_without_an_owner_acceptance(self):
        self.case("bad-checkpoint")

    def test_zero_exit_without_result_cannot_publish_healthy_owner(self):
        self.case("missing-result")

    def test_late_journal_failure_prevents_owner_publication(self):
        self.case("healthy", fail_journal=True)

    def test_primary_failure_survives_later_journal_failure(self):
        self.case("bad-checkpoint", fail_journal=True)


if __name__ == "__main__":
    unittest.main()
