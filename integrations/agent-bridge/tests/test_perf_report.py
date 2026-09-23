"""Complete synthetic report cohorts and separately rehashed invalid endpoints."""

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1] / "scripts"
SPEC = importlib.util.spec_from_file_location(
    "performance_report_subject", SCRIPTS / "perf_report.py"
)
r = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(r)


class ReportControls(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(
            prefix="performance-report-controls-"
        )
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.baseline = self.root / "baseline"
        self.baseline.mkdir()
        self.cases = [
            dict(
                case_id=f"b{block:02d}-{route}-{level}",
                block=block,
                route=route,
                instrumentation=level,
                clock_id=f"clock-{block}-{route}-{level}",
            )
            for block in range(32)
            for route in ("direct", "body", "canonical")
            for level in ("minimal", "detailed")
        ]
        frozen = self.save(
            self.baseline / "freeze.json",
            {
                "cases": self.cases,
                "tools": {"report": r.m.identity(SCRIPTS / "perf_report.py")},
                "authority": {
                    "synthetic_control_only": True,
                    "native_execution": False,
                },
            },
        )
        completed = []
        for index, case in enumerate(self.cases):
            ticks = []
            for tick in range(1, 25):
                offer, deadline = r.m.schedule_ns(0, tick)
                ticks.append(
                    dict(
                        tick=tick,
                        clock_id=case["clock_id"],
                        offer_ns=offer,
                        deadline_numerator_ns_x120=deadline,
                        start_ns=offer,
                        observation_ns=offer + 1,
                        route_complete_ns=offer + 2,
                        export_complete_ns=offer + 3,
                        route_missed=False,
                        export_missed=False,
                    )
                )
            end = ticks[-1]["export_complete_ns"]
            result = {
                "case": case,
                "clock_id": case["clock_id"],
                "freeze_sha256": frozen["sha256"],
                "planned_ticks": 24,
                "completed_ticks": 24,
                "preparation_scope": "synthetic " + case["route"],
                "offered_origin_ns": 0,
                "prepare_start_ns": 0,
                "prepare_end_ns": 0,
                "finish_start_ns": end + 1,
                "finish_end_ns": end + 2,
                "retired_ns": end + 3,
                "ticks": ticks,
                "spans": [],
            }
            completed.append(
                {
                    "case": case,
                    "result": self.save(self.baseline / f"{index}.result.json", result),
                    "command": self.save(
                        self.baseline / f"{index}.command.json",
                        {"elapsed_seconds": 1.0},
                    ),
                }
            )
        self.terminal = {
            "status": "COMPLETE_ACCOUNTING",
            "freeze": frozen,
            "completed": completed,
            "uncompleted_cases": [],
            "failure": None,
        }
        self.save(self.baseline / "result.json", self.terminal)

    @staticmethod
    def save(path, value):
        with path.open("x") as stream:
            json.dump(value, stream, allow_nan=False)
            stream.write("\n")
        return r.m.identity(path)

    def test_complete_cohort_retains_all_arms_and_declared_preparation_scopes(self):
        identity = r.report(self.baseline)
        value = json.loads(r.m.selected_file(identity))
        self.assertEqual(len(value["full_24_ticks"]), 6)
        self.assertTrue(
            all(
                row["completed_samples"] == 768
                for row in value["full_24_ticks"].values()
            )
        )
        self.assertTrue(
            all(
                row["completed_samples"] == 576
                for row in value["warmed_ticks_7_through_24"].values()
            )
        )
        self.assertEqual(len(value["preparation_scopes"]), 6)
        self.assertFalse(value["authority"]["native_execution"])

    def test_rehashed_invalid_intervals_and_clock_reject_before_report_publication(
        self,
    ):
        selected = self.terminal["completed"][0]
        original = json.loads(r.m.selected_file(selected["result"]))
        end = original["ticks"][-1]["export_complete_ns"]
        variants = [
            ({"prepare_start_ns": 1}, {}, "complete lifecycle timing order"),
            ({"finish_end_ns": end}, {}, "complete lifecycle timing order"),
            ({"retired_ns": end + 1}, {}, "complete lifecycle timing order"),
            ({"finish_start_ns": end - 1}, {}, "completed work precedes retirement"),
            (
                {},
                {"elapsed_seconds": -10.0},
                "finite nonnegative owned command duration",
            ),
            (
                {},
                {"elapsed_seconds": True},
                "finite nonnegative owned command duration",
            ),
            ({"clock_id": "foreign"}, {}, "lifecycle clock join"),
            ({"prepare_start_ns": 0.0}, {}, "complete lifecycle timing order"),
        ]
        for index, (change, duration, reason) in enumerate(variants):
            directory = self.root / f"rejected-{index}"
            directory.mkdir()
            terminal = copy.deepcopy(self.terminal)
            terminal["completed"][0]["result"] = self.save(
                directory / "changed.result.json", {**original, **change}
            )
            terminal["completed"][0]["command"] = self.save(
                directory / "changed.command.json", {"elapsed_seconds": 1.0, **duration}
            )
            self.save(directory / "result.json", terminal)
            with (
                self.subTest(index=index),
                self.assertRaisesRegex(r.m.MeasurementError, reason),
            ):
                r.report(directory)
            self.assertFalse((directory / "report.json").exists())
        self.assertEqual(json.loads(r.m.selected_file(selected["result"])), original)
        self.assertEqual(
            json.loads(r.m.selected_file(selected["command"])), {"elapsed_seconds": 1.0}
        )


if __name__ == "__main__":
    unittest.main()
