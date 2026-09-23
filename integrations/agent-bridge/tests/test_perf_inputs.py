"""Explicit performance input admission with synthetic metadata, no native owner."""

import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import uuid

from crebain_ncp_sensors import new_binding
from ncp_local.modular_buffer import BufferError

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "perf_campaign.py"
SPEC = importlib.util.spec_from_file_location("performance_input_subject", SCRIPT)
p = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(p)


class InputControls(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(
            prefix="performance-input-controls-"
        )
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.prior = self.root / "prior.json"
        self.design = self.root / "design.md"
        self.review = self.root / "review.json"
        self.design.write_bytes(b"Synthetic prospective design, no measurement.\n")
        self.review.write_bytes(b'{"synthetic":true,"execution_authorized":false}\n')
        executable = self.root / "unexecuted-binary"
        executable.write_bytes(b"synthetic file, never executable")
        manifest = self.root / "runtime.json"
        manifest.write_text(
            json.dumps(
                {"bun": p.m.identity(executable), "node": p.m.identity(executable)}
            )
        )
        self.runtime = SimpleNamespace(
            prefix=str(self.root),
            manifest_sha256=p.c.digest(manifest.read_bytes()),
            source_identity="a" * 64,
            environment={},
            bun=executable,
            node=executable,
        )
        self.old = {
            "schema": "prisoma.m1-freeze.v1",
            "runtime": {
                "prefix": str(self.root),
                "manifest_sha256": self.runtime.manifest_sha256,
                "source_identity": self.runtime.source_identity,
            },
            "tools": {
                "observer_source": p.m.identity(executable),
                "observer_binary": p.m.identity(executable),
            },
            "workload": p.m.identity(
                Path(__file__).with_name("fixtures") / "m1.workload.v1.json"
            ),
            "environments": {},
        }
        self.prior.write_text(json.dumps(self.old))

    def freeze(self, output, **changes):
        values = {
            "m1_freeze": self.prior,
            "design": self.design,
            "review": self.review,
            "campaign": output,
        }
        values.update(changes)
        return p.freeze_study(**values)

    def test_valid_inputs_bind_exact_files_and_existing_helpers(self):
        output = self.root / "accepted"
        with patch(
            "crebain_ncp_sensors.runtime.InstalledRuntime.open",
            return_value=self.runtime,
        ) as opened:
            identity = self.freeze(output)
        opened.assert_called_once_with(str(self.root))
        result = p.c.parse(p.c.selected_file(identity))
        self.assertEqual(result["original_m1_freeze"], p.m.identity(self.prior))
        self.assertEqual(result["tools"]["design"], p.m.identity(self.design))
        self.assertEqual(result["tools"]["design_closure"], p.m.identity(self.review))
        self.assertEqual(
            result["tools"]["m1_helpers"],
            p.m.identity(SCRIPT.with_name("m1_campaign.py")),
        )
        self.assertEqual(
            result["tools"]["observer_python"],
            p.m.identity(SCRIPT.with_name("owned_observer.py")),
        )
        self.assertEqual(len(result["cases"]), 192)
        for case in result["cases"]:
            binding = new_binding(run_id=case["run_id"])
            self.assertEqual(binding.run_id, case["run_id"])
            self.assertEqual(str(uuid.UUID(case["run_id"])), case["run_id"])
        self.assertFalse(result["authority"]["release_qualified"])
        self.assertFalse(result["authority"]["real_time_qualified"])
        self.assertFalse(json.loads(self.review.read_bytes())["execution_authorized"])
        p.validate_freeze(result, (output / "freeze.json").read_bytes())
        with patch("crebain_ncp_sensors.runtime.InstalledRuntime.open") as opened:
            with self.assertRaisesRegex(p.m.MeasurementError, "fresh campaign"):
                self.freeze(output)
            opened.assert_not_called()
        self.assertEqual(p.m.identity(output / "freeze.json"), identity)

    def test_legacy_run_id_rejects_before_runtime_and_execution_marker(self):
        output = self.root / "legacy-run-id"
        with patch(
            "crebain_ncp_sensors.runtime.InstalledRuntime.open",
            return_value=self.runtime,
        ):
            self.freeze(output)
        selected = output / "freeze.json"
        result = json.loads(selected.read_bytes())
        # Change the selected bytes themselves, without relying on a stale outer hash.
        result["cases"][0]["run_id"] = uuid.UUID(result["cases"][0]["run_id"]).hex
        inventory = self.root / "inventory.json"
        inventory.write_bytes(b"{}")
        result["environments"] = {
            "canonical": {
                "prefix": str(Path(sys.prefix).resolve()),
                "inventory": p.m.identity(inventory),
                "wheels": {},
            }
        }
        selected.write_text(json.dumps(result))
        with (
            patch("crebain_ncp_sensors.runtime.InstalledRuntime.open") as opened,
            patch.object(p, "capture") as capture,
        ):
            with self.assertRaises(BufferError) as rejected:
                p.run_study(selected)
            self.assertEqual(rejected.exception.code, "binding")
            opened.assert_not_called()
            capture.assert_not_called()
        self.assertFalse((output / "execution-start.json").exists())
        self.assertEqual(list((output / "runs").iterdir()), [])
        self.assertEqual(list((output / "commands").iterdir()), [])

    def test_each_input_rejects_size_symlink_relative_and_empty_before_runtime(self):
        oversized = self.root / "oversized"
        with oversized.open("wb") as stream:
            stream.seek(p.m.MAX_JSON)
            stream.write(b"x")
        empty = self.root / "empty"
        empty.write_bytes(b"")
        symlink = self.root / "alias"
        symlink.symlink_to(self.design)
        variants = [
            (oversized, p.c.CampaignError, "file_bound"),
            (symlink, p.c.CampaignError, "direct_path"),
            (Path("relative"), p.c.CampaignError, "direct_path"),
            (empty, p.m.MeasurementError, "nonempty selected input"),
        ]
        for field in ("m1_freeze", "design", "review"):
            for index, (path, exception, reason) in enumerate(variants):
                with self.subTest(field=field, path=path):
                    output = self.root / f"rejected-{field}-{index}"
                    with patch(
                        "crebain_ncp_sensors.runtime.InstalledRuntime.open"
                    ) as opened:
                        with self.assertRaisesRegex(exception, reason):
                            self.freeze(output, **{field: path})
                        opened.assert_not_called()
                    self.assertFalse(output.exists())

    def test_prior_json_rejects_duplicate_nonfinite_and_foreign_profile_before_runtime(
        self,
    ):
        for index, (raw, exception, reason) in enumerate(
            [
                (b'{"runtime":{},"runtime":{}}', ValueError, "duplicate_key"),
                (b'{"runtime":NaN}', ValueError, "nonfinite"),
                (
                    b'{"schema":"foreign-profile"}',
                    p.m.MeasurementError,
                    "original M1 freeze schema",
                ),
            ]
        ):
            self.prior.write_bytes(raw)
            output = self.root / f"invalid-json-{index}"
            with (
                self.subTest(raw=raw),
                patch("crebain_ncp_sensors.runtime.InstalledRuntime.open") as opened,
            ):
                with self.assertRaisesRegex(exception, reason):
                    self.freeze(output)
                opened.assert_not_called()
            self.assertFalse(output.exists())

    def test_outer_json_preserves_finite_values_and_rejects_duplicate_or_overflow(self):
        path = self.root / "outer.json"
        raw = b'{"value":1.25,"exponent":1e-6,"negative_zero":-0.0}'
        path.write_bytes(raw)
        selected, decoded = p.m.read_json(path)
        self.assertEqual(selected, raw)
        self.assertEqual(decoded, json.loads(raw))
        for raw, reason in (
            (b'{"x":1,"x":2}', "duplicate_key"),
            (b'{"x":1,"\\u0078":2}', "duplicate_key"),
            (b'{"x":NaN}', "nonfinite"),
            (b'{"x":1e400}', "finite metadata numbers"),
        ):
            path.write_bytes(raw)
            with self.subTest(raw=raw), self.assertRaisesRegex(ValueError, reason):
                p.m.read_json(path)
        path.write_bytes(b"0" + b" " * (p.m.MAX_JSON - 1))
        self.assertEqual(p.m.read_json(path)[1], 0)
        with path.open("ab") as stream:
            stream.write(b" ")
        with self.assertRaisesRegex(ValueError, "file_bound"):
            p.m.read_json(path)
        path.unlink()
        path.mkdir()
        with self.assertRaises(IsADirectoryError):
            p.m.read_json(path)

    def test_outer_file_growth_rejects_before_json_acceptance(self):
        path = self.root / "changing.json"
        path.write_bytes(b'{"value":1}')
        original = p.m._FILES.os.fdopen

        class GrowingReader:
            def __init__(self, stream):
                self.stream = stream

            def close(self):
                self.stream.close()

            def fileno(self):
                return self.stream.fileno()

            def read(self, maximum):
                raw = self.stream.read(maximum)
                with path.open("ab") as changed:
                    changed.write(b" ")
                return raw

        def growing(fd, *args, **kwargs):
            return GrowingReader(original(fd, *args, **kwargs))

        with patch.object(p.m._FILES.os, "fdopen", side_effect=growing):
            with self.assertRaisesRegex(ValueError, "file_changed"):
                p.m.read_json(path)
        self.assertEqual(path.read_bytes(), b'{"value":1} ')

    def test_input_clis_preserve_absolute_and_symlink_rejections(self):
        selected = self.root / "selected.json"
        selected.write_bytes(b'{"schema":"foreign-profile"}')
        alias = self.root / "alias.json"
        alias.symlink_to(selected)
        for path, reason in (
            (selected, "freeze schema"),
            (alias, "direct_path"),
            (Path("selected.json"), "direct_path"),
        ):
            with self.subTest(operation="run", path=path):
                command = subprocess.run(
                    [sys.executable, "-I", "-B", str(SCRIPT), "run", str(path)],
                    cwd=self.root,
                    capture_output=True,
                    timeout=10,
                )
                self.assertNotEqual(command.returncode, 0)
                self.assertIn(reason, command.stderr.decode())
                self.assertFalse((self.root / "execution-start.json").exists())
        (self.root / "result.json").write_bytes(b'{"freeze":{}}')
        directory_alias = self.root / "directory-alias"
        directory_alias.symlink_to(self.root, target_is_directory=True)
        for path, reason in (
            (self.root, "file_shape"),
            (directory_alias, "direct_path"),
            (Path("."), "direct_path"),
        ):
            with self.subTest(operation="report", path=path):
                command = subprocess.run(
                    [
                        sys.executable,
                        "-I",
                        "-B",
                        str(SCRIPT.with_name("perf_report.py")),
                        str(path),
                    ],
                    cwd=self.root,
                    capture_output=True,
                    timeout=10,
                )
                self.assertNotEqual(command.returncode, 0)
                self.assertIn(reason, command.stderr.decode())
                self.assertFalse((self.root / "report.json").exists())


if __name__ == "__main__":
    unittest.main()
