"""Command receipt controls use synthetic children, never a native simulator."""

from contextlib import contextmanager
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
import zipfile


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "m1_run.py"
SPEC = importlib.util.spec_from_file_location("m1_run_controls", SCRIPT)
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)

SUPERVISOR = """
import argparse, json, os, pathlib, sys
parser = argparse.ArgumentParser()
parser.add_argument('--freeze', required=True)
parser.add_argument('--case', required=True)
args = parser.parse_args()
freeze = json.loads(pathlib.Path(args.freeze).read_bytes())
output = pathlib.Path(freeze['output_root']) / args.case
output.mkdir(mode=0o700)
print(json.dumps({'isolated': sys.flags.isolated,
                  'bytecode': sys.dont_write_bytecode,
                  'executable': sys.executable}), flush=True)
OWNER
TAIL
"""


class CaptureTests(unittest.TestCase):
    def capture(self, source, *, bound=4096, wall=2, grace=0.2):
        stdout, stderr = io.BytesIO(), io.BytesIO()
        previous = signal.getsignal(signal.SIGINT)
        state = m.capture(
            [sys.executable, "-I", "-B", "-c", source],
            os.environ.copy(),
            {"wall_seconds": wall, "grace_seconds": grace, "stream_bytes": bound},
            stdout,
            stderr,
        )
        self.assertIs(signal.getsignal(signal.SIGINT), previous)
        self.assertIs(type(state["returncode"]), int)
        return state, stdout.getvalue(), stderr.getvalue()

    def test_normal_exit_and_both_streams(self):
        state, stdout, stderr = self.capture(
            "import os; os.write(1,b'out'); os.write(2,b'err')"
        )
        self.assertEqual((state["returncode"], state["failure"]), (0, None))
        self.assertEqual((stdout, stderr), (b"out", b"err"))
        self.assertFalse(
            any(
                state[key]
                for key in (
                    "timed_out",
                    "output_overflow",
                    "interrupted",
                    "forced_kill",
                )
            )
        )

    def test_nonzero_exit_retains_output_and_exact_returncode(self):
        state, stdout, _ = self.capture("import os; os.write(1,b'detail'); os._exit(7)")
        self.assertEqual(state["returncode"], 7)
        self.assertEqual(state["failure"], "supervisor_returncode")
        self.assertEqual(stdout, b"detail")

    def test_stream_bounds_include_exact_boundary_and_one_extra_byte(self):
        for descriptor in (1, 2):
            for size in (31, 32, 33):
                with self.subTest(descriptor=descriptor, size=size):
                    state, stdout, stderr = self.capture(
                        f"import os; os.write({descriptor}, b'x' * {size})", bound=32
                    )
                    selected = stdout if descriptor == 1 else stderr
                    self.assertEqual(selected, b"x" * min(32, size))
                    self.assertEqual(state["output_overflow"], size > 32)
                    self.assertEqual(state["failure"] is None, size <= 32)

    def test_timeout_allows_owned_supervisor_to_handle_interrupt(self):
        state, stdout, _ = self.capture(
            "import signal,time,sys; "
            "signal.signal(signal.SIGINT, lambda *_: sys.exit(0)); "
            "print('ready', flush=True); time.sleep(20)",
            wall=0.3,
        )
        self.assertEqual(stdout, b"ready\n")
        self.assertTrue(state["timed_out"])
        self.assertFalse(state["forced_kill"])
        self.assertEqual(
            (state["returncode"], state["failure"]), (0, "supervisor_timeout")
        )

    def test_timeout_kills_only_the_unreaped_direct_child_after_grace(self):
        started = time.monotonic()
        state, stdout, _ = self.capture(
            "import signal,time; signal.signal(signal.SIGINT, signal.SIG_IGN); "
            "print('ready', flush=True); time.sleep(20)",
            wall=0.3,
        )
        self.assertEqual(stdout, b"ready\n")
        self.assertTrue(state["timed_out"])
        self.assertTrue(state["forced_kill"])
        self.assertEqual(state["returncode"], -signal.SIGKILL)
        self.assertLess(time.monotonic() - started, 3)

    def test_injected_interruption_is_recorded_as_command_failure(self):
        @contextmanager
        def interrupted(state):
            state["interrupted"] = True
            yield

        with patch.object(m, "_interrupts", interrupted):
            state, _, _ = self.capture("import time; time.sleep(20)")
        self.assertTrue(state["interrupted"])
        self.assertEqual(state["failure"], "runner_interrupted")

    def test_io_failure_reaps_the_owned_child(self):
        class BrokenOutput:
            def write(self, _raw):
                raise OSError("injected write failure")

        state = m.capture(
            [
                sys.executable,
                "-I",
                "-B",
                "-c",
                "import time; print('ready', flush=True); time.sleep(20)",
            ],
            os.environ.copy(),
            {"wall_seconds": 2, "grace_seconds": 0.2, "stream_bytes": 4096},
            BrokenOutput(),
            io.BytesIO(),
        )
        self.assertIs(type(state["returncode"]), int)
        self.assertEqual(state["failure"], "supervisor_io")

    def test_operation_and_cleanup_failures_preserve_both_original_objects(self):
        operation_error = OSError("injected operation failure")
        cleanup_error = RuntimeError("injected cleanup failure")
        children = []
        real_popen = subprocess.Popen

        class BrokenOutput:
            def write(self, _raw):
                raise operation_error

        def own(*args, **kwargs):
            child = real_popen(*args, **kwargs)
            children.append(child)
            return child

        try:
            with (
                patch.object(m.subprocess, "Popen", side_effect=own),
                patch.object(m, "_retire_owned", side_effect=cleanup_error) as retire,
            ):
                with self.assertRaises(BaseExceptionGroup) as observed:
                    m.capture(
                        [
                            sys.executable,
                            "-I",
                            "-B",
                            "-c",
                            "import time; print('ready', flush=True); time.sleep(20)",
                        ],
                        os.environ.copy(),
                        {"wall_seconds": 2, "grace_seconds": 0.2, "stream_bytes": 4096},
                        BrokenOutput(),
                        io.BytesIO(),
                    )
            self.assertEqual(retire.call_count, 1)
            self.assertEqual(
                observed.exception.exceptions, (operation_error, cleanup_error)
            )
        finally:
            # The injected failed cleanup supplies no retirement evidence.
            # This test separately retains and reaps its actual direct child.
            for child in children:
                if child.poll() is None:
                    child.kill()
                child.wait(timeout=5)

    def test_hostile_exception_diagnostics_cannot_prevent_owned_retirement(self):
        class HostileError(RuntimeError):
            @property
            def __class__(self):
                raise AssertionError("exception diagnostics ran before cleanup")

        operation_error = HostileError("injected operation failure")
        children = []
        real_popen = subprocess.Popen

        class BrokenOutput:
            def write(self, _raw):
                raise operation_error

        def own(*args, **kwargs):
            child = real_popen(*args, **kwargs)
            children.append(child)
            return child

        try:
            with patch.object(m.subprocess, "Popen", side_effect=own):
                with self.assertRaises(RuntimeError) as observed:
                    m.capture(
                        [
                            sys.executable,
                            "-I",
                            "-B",
                            "-c",
                            "import time; print('ready', flush=True); time.sleep(20)",
                        ],
                        os.environ.copy(),
                        {"wall_seconds": 2, "grace_seconds": 0.2, "stream_bytes": 4096},
                        BrokenOutput(),
                        io.BytesIO(),
                    )
            self.assertIs(observed.exception, operation_error)
            self.assertEqual(len(children), 1)
            self.assertIsNotNone(children[0].poll())
        finally:
            for child in children:
                if child.poll() is None:
                    child.kill()
                child.wait(timeout=5)

    @unittest.skipUnless(hasattr(os, "fork"), "POSIX inherited-pipe control")
    def test_reaped_root_with_open_descendant_pipe_does_not_authorize_a_signal(self):
        with tempfile.TemporaryDirectory() as temporary:
            marker = Path(temporary) / "natural-retirement"
            source = (
                "import os,time,pathlib; child=os.fork(); "
                "os._exit(0) if child else None; time.sleep(0.8); "
                f"pathlib.Path({str(marker)!r}).write_text('natural'); os._exit(0)"
            )
            with patch.object(m, "_signal_owned", wraps=m._signal_owned) as observed:
                state, _, _ = self.capture(source, wall=0.2, grace=0.1)
            self.assertEqual(state["returncode"], 0)
            self.assertTrue(state["timed_out"])
            self.assertFalse(state["forced_kill"])
            self.assertTrue(observed.called)
            self.assertTrue(
                all(call.args[0].returncode == 0 for call in observed.call_args_list)
            )
            deadline = time.monotonic() + 3
            while not marker.exists() and time.monotonic() < deadline:
                time.sleep(0.02)
            self.assertEqual(marker.read_text(), "natural")


class CommandTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.source = self.root / "source"
        self.source.mkdir()
        self.output = self.root / "output"
        self.output.mkdir(mode=0o700)
        for name in ("m1_run.py", "m1_campaign.py"):
            shutil.copyfile(SCRIPT.with_name(name), self.source / name)
        self.supervisor = self.source / "synthetic_supervisor.py"
        self.supervisor.write_text(
            SUPERVISOR.replace(
                "OWNER",
                "(output / 'owner.json').write_bytes(b'{\"synthetic\":true}\\n')",
            ).replace("TAIL", "")
        )
        payload = self.source / "selected.bin"
        payload.write_bytes(b"synthetic selection only")
        self.freeze_path = self.root / "freeze.json"
        self.freeze = {
            "schema": "prisoma.m1-freeze.v1",
            "campaign_id": "runner-controls",
            "workload": m.c.file_identity(payload),
            "source": {"root": str(self.source)},
            "tools": {
                "worker": m.c.file_identity(self.source / "m1_campaign.py"),
                "runner": m.c.file_identity(self.source / "m1_run.py"),
                "supervisor": m.c.file_identity(self.supervisor),
                **{
                    name: m.c.file_identity(payload)
                    for name in (
                        "observer_source",
                        "observer_binary",
                        "observer_python",
                        "fault_source",
                        "fault_binary",
                    )
                },
            },
            "runtime": {
                "prefix": str(self.root),
                "manifest_sha256": "a" * 64,
                "source_identity": "b" * 64,
            },
            "renderer": m.c.file_identity(payload),
            "renderer_parent": m.c.file_identity(payload),
            "limits": {"session_seconds": 2, "checkpoint_seconds": 1},
            "output_root": str(self.output),
            "cases": [
                {"case_id": "body", "arm": "body", "fault": "none"},
                {"case_id": "canonical", "arm": "canonical", "fault": "none"},
            ],
            "environments": {},
        }
        wheels, distributions, installed_files = {}, {}, {}
        site_packages = f"lib/python{sys.version_info.major}.{sys.version_info.minor}/site-packages/"
        for package in sorted(m.c.CANONICAL_DISTRIBUTIONS):
            distribution = package.replace("-", "_")
            members = {
                distribution + "-0.dist-info/METADATA": (
                    f"Name: {package}\nVersion: 0\n".encode()
                ),
                distribution + "/__init__.py": b"# synthetic command fixture\n",
            }
            wheel = self.root / (package + ".whl")
            with zipfile.ZipFile(wheel, "w") as archive:
                for name, raw in members.items():
                    archive.writestr(name, raw)
            wheels[package] = m.c.file_identity(wheel)
            installed_files[package] = {
                site_packages + name: raw for name, raw in members.items()
            }
            distributions[package] = {
                "version": "0",
                "files": {
                    name: {"bytes": len(raw), "sha256": m.c.digest(raw)}
                    for name, raw in installed_files[package].items()
                },
            }
        for arm, packages in (
            ("body", m.c.SHARED_DISTRIBUTIONS),
            ("canonical", m.c.CANONICAL_DISTRIBUTIONS),
        ):
            prefix = self.root / arm
            (prefix / "bin").mkdir(parents=True)
            (prefix / "bin" / "python").symlink_to(Path(sys.executable).resolve())
            (prefix / "pyvenv.cfg").write_text(
                f"home = {Path(sys.executable).resolve().parent}\n"
                "include-system-site-packages = false\n"
            )
            for package in packages:
                for name, raw in installed_files[package].items():
                    path = prefix / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(raw)
            inventory_path = self.root / (arm + ".inventory.json")
            inventory_path.write_bytes(
                m.c.canonical(
                    {
                        "schema": "prisoma.m1-installed-inventory.v1",
                        "prefix": str(prefix),
                        "python": m.c.file_identity(Path(sys.executable).resolve()),
                        "distributions": {
                            package: distributions[package] for package in packages
                        },
                        "modules": {},
                    }
                )
            )
            self.freeze["environments"][arm] = {
                "prefix": str(prefix),
                "inventory": m.c.file_identity(inventory_path),
                "wheels": {package: wheels[package] for package in packages},
            }
        self.git("init", "-q", "-b", "main")

    def git(self, *arguments):
        environment = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith("GIT_")
        }
        environment.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
        return (
            subprocess.check_output(
                [
                    "git",
                    "-C",
                    str(self.source),
                    "-c",
                    "user.name=Synthetic Control",
                    "-c",
                    "user.email=synthetic@example.invalid",
                    *arguments,
                ],
                env=environment,
                stderr=subprocess.STDOUT,
            )
            .decode()
            .strip()
        )

    def freeze_selection(self, *, owner=True, tail=""):
        self.supervisor.write_text(
            SUPERVISOR.replace(
                "OWNER",
                "(output / 'owner.json').write_bytes(b'{\"synthetic\":true}\\n')"
                if owner
                else "",
            ).replace("TAIL", tail)
        )
        self.freeze["tools"]["supervisor"] = m.c.file_identity(self.supervisor)
        self.git("add", ".")
        self.git("commit", "-q", "-m", "Freeze synthetic command control")
        self.freeze["source"].update(
            commit=self.git("rev-parse", "HEAD"),
            tree=self.git("rev-parse", "HEAD^{tree}"),
        )
        self.write_freeze()

    def write_freeze(self):
        self.freeze_path.write_bytes(m.c.canonical(self.freeze) + b"\n")

    def invoke(self, *, isolated=True):
        argv = [sys.executable, "-I", "-B"] if isolated else [sys.executable, "-B"]
        result = subprocess.run(
            [
                *argv,
                str(self.source / "m1_run.py"),
                "--freeze",
                str(self.freeze_path),
                "--case",
                "body",
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=15,
        )
        path = self.output / "body.command.json"
        receipt = m.c.parse(path.read_bytes()) if path.exists() else None
        return result, receipt

    def test_cli_success_binds_exact_command_owner_and_atomic_commit(self):
        self.freeze_selection()
        result, receipt = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(
            receipt.keys(),
            {
                "schema",
                "freeze",
                "case",
                "runner",
                "supervisor",
                "argv",
                "limits",
                "returncode",
                "timed_out",
                "output_overflow",
                "interrupted",
                "forced_kill",
                "failure",
                "stdout",
                "stderr",
                "owner",
            },
        )
        self.assertEqual(receipt["schema"], "prisoma.m1-command.v1")
        self.assertEqual(receipt["freeze"], m.c.file_identity(self.freeze_path))
        self.assertEqual(receipt["case"], self.freeze["cases"][0])
        self.assertEqual(receipt["runner"], self.freeze["tools"]["runner"])
        self.assertEqual(receipt["supervisor"], self.freeze["tools"]["supervisor"])
        self.assertEqual(
            receipt["argv"],
            m.command_argv(self.freeze, self.freeze_path, self.freeze["cases"][0]),
        )
        self.assertEqual(
            receipt["limits"],
            {
                "wall_seconds": 34,
                "grace_seconds": 1,
                "stream_bytes": 77824,
            },
        )
        stdout = m.c.selected_file(receipt["stdout"])
        observed = json.loads(stdout)
        self.assertEqual(
            observed,
            {
                "isolated": 1,
                "bytecode": True,
                "executable": self.freeze["environments"]["body"]["prefix"]
                + "/bin/python",
            },
        )
        self.assertEqual(m.c.selected_file(receipt["stderr"]), b"")
        self.assertEqual(m.c.selected_file(receipt["owner"]), b'{"synthetic":true}\n')
        self.assertEqual(receipt["returncode"], 0)
        self.assertIsNone(receipt["failure"])
        for key in ("timed_out", "output_overflow", "interrupted", "forced_kill"):
            self.assertIs(receipt[key], False)
        committed = (self.output / "body.command.json").stat()
        staged = (self.output / ".body.command.json.pending").stat()
        self.assertEqual(
            (committed.st_dev, committed.st_ino), (staged.st_dev, staged.st_ino)
        )

    def test_owner_identity_is_observed_after_actual_supervisor_exit(self):
        self.freeze_selection(
            tail=(
                "import time; time.sleep(0.1); "
                "(output / 'owner.json').write_bytes(b'{\"final\":true}\\n')"
            )
        )
        result, receipt = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(m.c.selected_file(receipt["owner"]), b'{"final":true}\n')

    def test_owner_publication_followed_by_nonzero_exit_is_not_success(self):
        self.freeze_selection(tail="raise SystemExit(7)")
        result, receipt = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(receipt["returncode"], 7)
        self.assertEqual(receipt["failure"], "supervisor_returncode")
        self.assertIsNotNone(receipt["owner"])

    def test_zero_exit_without_owner_is_not_success(self):
        self.freeze_selection(owner=False)
        result, receipt = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(receipt["returncode"], 0)
        self.assertEqual(receipt["failure"], "owner_missing")
        self.assertIsNone(receipt["owner"])

    def test_symlink_owner_is_not_a_selected_byte_identity(self):
        self.freeze_selection(
            owner=False, tail="(output / 'owner.json').symlink_to(args.freeze)"
        )
        result, receipt = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(receipt["failure"], "owner_read")
        self.assertIsNone(receipt["owner"])

    def test_post_exit_freeze_drift_preserves_original_selection_and_fails(self):
        self.freeze_selection(
            tail="pathlib.Path(args.freeze).write_bytes(pathlib.Path(args.freeze).read_bytes()+b' ')"
        )
        before = m.c.file_identity(self.freeze_path)
        result, receipt = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(receipt["freeze"], before)
        self.assertEqual(receipt["failure"], "selection_changed")

    def test_post_exit_tool_drift_fails_without_rewriting_observed_returncode(self):
        self.freeze_selection(
            tail="pathlib.Path(__file__).write_text('# changed after execution\\n')"
        )
        result, receipt = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertEqual(receipt["returncode"], 0)
        self.assertEqual(receipt["failure"], "selection_changed")

    def test_cli_output_overflow_is_bounded_and_cannot_become_success(self):
        self.freeze_selection(tail="os.write(1, b'x' * 100000)")
        result, receipt = self.invoke()
        self.assertEqual(result.returncode, 1)
        self.assertTrue(receipt["output_overflow"])
        self.assertEqual(receipt["failure"], "supervisor_output_bound")
        self.assertEqual(receipt["stdout"]["bytes"], 77824)
        m.c.selected_file(receipt["stdout"])

    def test_existing_receipt_prevents_launch_and_is_never_replaced(self):
        self.freeze_selection()
        existing = self.output / "body.command.json"
        existing.write_bytes(b'{"prior":true}\n')
        result, receipt = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(receipt, {"prior": True})
        self.assertFalse((self.output / "body").exists())
        self.assertFalse((self.output / "body.command").exists())

    def test_runner_identity_mismatch_prevents_launch(self):
        self.freeze_selection()
        self.freeze["tools"]["runner"] = copy.deepcopy(
            self.freeze["tools"]["supervisor"]
        )
        self.write_freeze()
        result, receipt = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"runner_identity", result.stderr)
        self.assertIsNone(receipt)
        self.assertFalse((self.output / "body").exists())

    def test_selected_python_binary_mismatch_prevents_launch(self):
        self.freeze_selection()
        environment = self.freeze["environments"]["body"]
        path = Path(environment["inventory"]["path"])
        inventory = json.loads(path.read_bytes())
        inventory["python"] = copy.deepcopy(self.freeze["tools"]["supervisor"])
        path.write_bytes(m.c.canonical(inventory))
        environment["inventory"] = m.c.file_identity(path)
        self.write_freeze()
        result, receipt = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"python_identity", result.stderr)
        self.assertIsNone(receipt)
        self.assertFalse((self.output / "body").exists())

    def test_nonisolated_invocation_prevents_launch(self):
        self.freeze_selection()
        result, receipt = self.invoke(isolated=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"isolated_python", result.stderr)
        self.assertIsNone(receipt)


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()

    def test_reopen_failure_leaves_staging_without_a_completed_receipt(self):
        with patch.object(m.c, "read", return_value=b"changed"):
            with self.assertRaisesRegex(m.c.CampaignError, "receipt_reopen"):
                m.publish(self.root, "case.command.json", {"synthetic": True})
        self.assertFalse((self.root / "case.command.json").exists())
        self.assertTrue((self.root / ".case.command.json.pending").exists())

    def test_fsync_failure_cannot_publish_a_completed_receipt(self):
        with patch.object(m.os, "fsync", side_effect=OSError("injected fsync")):
            with self.assertRaises(OSError):
                m.publish(self.root, "case.command.json", {"synthetic": True})
        self.assertFalse((self.root / "case.command.json").exists())

    def test_destination_collision_preserves_prior_bytes(self):
        target = self.root / "case.command.json"
        target.write_bytes(b"prior")
        with self.assertRaises(FileExistsError):
            m.publish(self.root, "case.command.json", {"synthetic": True})
        self.assertEqual(target.read_bytes(), b"prior")


if __name__ == "__main__":
    unittest.main()
