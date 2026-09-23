"""Exercise the fault helper only against directly owned waiting children."""

import importlib.util
import errno
import json
import os
from pathlib import Path
import resource
import signal
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "owned_observer", ROOT / "owned_observer.py"
)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class FaultControls(unittest.TestCase):
    def test_kernel_rejects_wrong_version_before_owned_positive_control(self):
        result = subprocess.run(
            [str(ROOT / "audit_token_probe")], capture_output=True, timeout=10
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(receipt["wrong_version_signal_result"], errno.ESRCH)
        for key in (
            "token_pid_matches",
            "child_survived_wrong_version",
            "waited",
            "terminated_by_sigterm",
        ):
            self.assertIs(receipt[key], True)
        self.assertEqual(receipt["signal_result"], 0)

    def test_identity_rejection_and_exact_owned_signal(self):
        observer = module.Observer(ROOT / "darwin_observer")
        child = subprocess.Popen(
            [sys.executable, "-I", "-B", "-c", "import sys; sys.stdin.buffer.read()"],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        try:
            row = observer.snapshot()[0][child.pid]
            selected = observer.selected(module.birth(row))
            executable = os.fsdecode(bytes.fromhex(selected["executable_path_hex"]))
            args = [
                str(ROOT / "darwin_fault"),
                "terminate",
                str(child.pid),
                str(row["start_seconds"]),
                str(row["start_microseconds"]),
                str(row["ppid"]),
                executable,
            ]
            for index, replacement, expected in [
                (2, "-1", 10),
                (3, str(row["start_seconds"] + 1), 20),
                (4, "1000000", 10),
                (5, str(row["ppid"] + 1), 20),
                (6, "/not/the/owned/executable", 23),
            ]:
                with self.subTest(index=index):
                    wrong = args.copy()
                    wrong[index] = replacement
                    result = subprocess.run(wrong, capture_output=True, timeout=5)
                    self.assertEqual(result.returncode, expected, result.stderr)
                    self.assertIsNone(child.poll())
            result = subprocess.run(args, capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stderr)
            receipt = json.loads(result.stdout)
            self.assertEqual(receipt["schema"], "local.darwin-version-bound-fault.v1")
            self.assertEqual(
                (
                    receipt["pid"],
                    receipt["start_seconds"],
                    receipt["start_microseconds"],
                ),
                module.birth(row),
            )
            self.assertEqual(receipt["ppid"], row["ppid"])
            self.assertEqual(receipt["signal_result"], 0)
            self.assertEqual(receipt["signal"], 15)
            self.assertEqual(child.wait(timeout=5), -15)
        finally:
            child.stdin.close()
            child.wait(timeout=5)

    def test_failed_output_after_signal_is_an_uncertain_command_result(self):
        observer = module.Observer(ROOT / "darwin_observer")
        child = subprocess.Popen(
            [sys.executable, "-I", "-B", "-c", "import sys; sys.stdin.buffer.read()"],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

        def zero_file_limit():
            signal.signal(signal.SIGXFSZ, signal.SIG_IGN)
            resource.setrlimit(resource.RLIMIT_FSIZE, (0, 0))

        try:
            row = observer.snapshot()[0][child.pid]
            selected = observer.selected(module.birth(row))
            executable = os.fsdecode(bytes.fromhex(selected["executable_path_hex"]))
            args = [
                str(ROOT / "darwin_fault"),
                "terminate",
                str(child.pid),
                str(row["start_seconds"]),
                str(row["start_microseconds"]),
                str(row["ppid"]),
                executable,
            ]
            with tempfile.TemporaryFile() as target:
                result = subprocess.run(
                    args,
                    stdout=target,
                    stderr=subprocess.PIPE,
                    preexec_fn=zero_file_limit,
                    timeout=5,
                )
                target.seek(0)
                self.assertEqual(target.read(), b"")
            self.assertEqual(result.returncode, 25)
            # No retry follows the ambiguous command result. The direct owner waits.
            self.assertEqual(child.wait(timeout=5), -signal.SIGTERM)
        finally:
            child.stdin.close()
            child.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
