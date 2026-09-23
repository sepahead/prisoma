"""Controls for schedule, endpoints, pairing, and transparent measurement."""

import copy
import importlib.util
import pathlib
import unittest

p = pathlib.Path(__file__).resolve().parents[1] / "scripts" / "perf_common.py"
s = importlib.util.spec_from_file_location("perf_common", p)
m = importlib.util.module_from_spec(s)
s.loader.exec_module(m)


class Manual:
    def __init__(self):
        self.value = 0

    def __call__(self):
        return self.value


class Measurements(unittest.TestCase):
    def row(self, tick=1, start=0, observed=100, route=200, exported=300):
        offer, deadline = m.schedule_ns(0, tick)
        return {
            "tick": tick,
            "clock_id": "owned-clock",
            "offer_ns": offer,
            "deadline_numerator_ns_x120": deadline,
            "start_ns": start,
            "observation_ns": observed,
            "route_complete_ns": route,
            "export_complete_ns": exported,
            "route_missed": m.missed_deadline(route, deadline),
            "export_missed": m.missed_deadline(exported, deadline),
        }

    def validate(self, rows, planned=1):
        m.validate_tick_rows(
            rows,
            clock_id="owned-clock",
            origin_ns=0,
            planned_ticks=planned,
            completed_ticks=len(rows),
        )

    def test_rational_offer_and_deadline_boundaries(self):
        self.assertEqual(m.schedule_ns(0, 1), (0, 1_000_000_000))
        self.assertEqual(m.schedule_ns(0, 2), (8_333_334, 2_000_000_000))
        self.assertEqual(m.schedule_ns(0, 4), (25_000_000, 4_000_000_000))
        deadline = m.schedule_ns(0, 3)[1]
        self.assertFalse(m.missed_deadline(25_000_000, deadline))
        self.assertTrue(m.missed_deadline(25_000_001, deadline))
        for tick in (True, 0, 1023, float("nan"), 1.5):
            with self.subTest(tick=tick), self.assertRaises(m.MeasurementError):
                m.schedule_ns(0, tick)

    def test_delayed_owner_remains_in_route_cost_and_fixed_schedule(self):
        first = self.row(observed=20_000_000, route=21_000_000, exported=22_000_000)
        second = self.row(2, 22_000_000, 23_000_000, 24_000_000, 25_000_000)
        self.validate([first, second], 2)
        self.assertTrue(first["route_missed"])
        self.assertEqual(second["offer_ns"], 8_333_334)
        self.assertTrue(second["route_missed"])
        bad = copy.deepcopy(second)
        bad["offer_ns"] = 22_000_000
        with self.assertRaises(m.MeasurementError):
            self.validate([first, bad], 2)

    def test_delayed_export_separates_two_endpoints_and_next_backlog(self):
        first = self.row(route=1_000_000, exported=20_000_000)
        second = self.row(2, 20_000_000, 21_000_000, 22_000_000, 23_000_000)
        self.validate([first, second], 2)
        self.assertFalse(first["route_missed"])
        self.assertTrue(first["export_missed"])
        self.assertTrue(second["route_missed"])

    def test_invalid_rows_never_become_zero_cost(self):
        baseline = self.row()
        self.validate([baseline])
        variants = []
        for field, value in (
            ("clock_id", "foreign"),
            ("tick", True),
            ("observation_ns", -1),
            ("start_ns", float("nan")),
            ("route_complete_ns", 400),
            ("route_missed", 0),
            ("export_missed", True),
            ("offer_ns", 1),
            ("offer_ns", False),
            ("deadline_numerator_ns_x120", 1_000_000_000.0),
        ):
            item = copy.deepcopy(baseline)
            item[field] = value
            variants.append(item)
        for value in variants:
            with self.subTest(row=value), self.assertRaises(m.MeasurementError):
                self.validate([value])
        with self.assertRaises(m.MeasurementError):
            self.validate([baseline, baseline], 2)

    def test_missing_extra_and_nonsequential_ticks(self):
        rows = [self.row()]
        with self.assertRaises(m.MeasurementError):
            m.validate_tick_rows(
                rows,
                clock_id="owned-clock",
                origin_ns=0,
                planned_ticks=2,
                completed_ticks=2,
            )
        extra = [self.row(), self.row(2, 8_333_334, 8_333_400, 8_333_500, 8_333_600)]
        with self.assertRaises(m.MeasurementError):
            self.validate(extra, 1)
        with self.assertRaises(m.MeasurementError):
            self.validate([self.row(exported=9_000_000), extra[1]], 2)

    def test_quantiles_and_paired_differences(self):
        values = list(range(576))
        q = m.quantiles(values)
        self.assertEqual(
            (q["p50"], q["p99"], q["p99_9"], q["max"]), (287, 570, 575, 575)
        )
        left = [
            {"block": 0, "tick": i, "instrumentation": "minimal", "cost": x}
            for i, x in enumerate((1, 1000, 10), 1)
        ]
        right = [
            {"block": 0, "tick": i, "instrumentation": "minimal", "cost": x}
            for i, x in enumerate((0, 999, 0), 1)
        ]
        self.assertEqual(m.paired_differences(left, right, "cost"), [1, 1, 10])
        self.assertNotEqual(
            m.nearest_rank([1, 1, 10], 0.99),
            m.nearest_rank([1, 1000, 10], 0.99) - m.nearest_rank([0, 999, 0], 0.99),
        )
        for a, b in ((left, right[:-1]), (left + [left[0]], right)):
            with self.assertRaises(m.MeasurementError):
                m.paired_differences(a, b, "cost")
        for bad in ([], [float("nan")], [float("inf")], [True]):
            with self.assertRaises(m.MeasurementError):
                m.quantiles(bad)

    def test_complete_lifecycle_rejects_invalid_endpoints_and_clock(self):
        baseline = {
            "clock_id": "owned-clock",
            "prepare_start_ns": 0,
            "prepare_end_ns": 0,
            "offered_origin_ns": 0,
            "finish_start_ns": 300,
            "finish_end_ns": 301,
            "retired_ns": 302,
            "planned_ticks": 1,
            "completed_ticks": 1,
            "ticks": [self.row()],
        }
        m.validate_lifecycle(
            baseline, clock_id="owned-clock", command_elapsed_seconds=1.0
        )
        mutations = (
            ("clock_id", "foreign"),
            ("prepare_start_ns", 1),
            ("prepare_end_ns", 1),
            ("offered_origin_ns", False),
            ("finish_start_ns", 299),
            ("finish_end_ns", 299),
            ("retired_ns", 300),
            ("planned_ticks", 2),
            ("completed_ticks", True),
        )
        for field, value in mutations:
            case = copy.deepcopy(baseline)
            case[field] = value
            with (
                self.subTest(field=field, value=value),
                self.assertRaises(m.MeasurementError),
            ):
                m.validate_lifecycle(
                    case, clock_id="owned-clock", command_elapsed_seconds=1.0
                )
        for field in (
            "prepare_start_ns",
            "prepare_end_ns",
            "offered_origin_ns",
            "finish_start_ns",
            "finish_end_ns",
            "retired_ns",
        ):
            for value in (
                -1,
                float(baseline[field]),
                False,
                float("inf"),
                float("nan"),
            ):
                case = copy.deepcopy(baseline)
                case[field] = value
                with (
                    self.subTest(field=field, value=value),
                    self.assertRaises(m.MeasurementError),
                ):
                    m.validate_lifecycle(
                        case, clock_id="owned-clock", command_elapsed_seconds=1.0
                    )

    def test_complete_lifecycle_rejects_invalid_owned_command_duration(self):
        baseline = {
            "clock_id": "owned-clock",
            "prepare_start_ns": 0,
            "prepare_end_ns": 0,
            "offered_origin_ns": 0,
            "finish_start_ns": 300,
            "finish_end_ns": 301,
            "retired_ns": 302,
            "planned_ticks": 1,
            "completed_ticks": 1,
            "ticks": [self.row()],
        }
        for value in (0, 0.0, 1, 2.5):
            m.validate_lifecycle(
                baseline, clock_id="owned-clock", command_elapsed_seconds=value
            )
        for value in (-1, -0.1, True, float("inf"), float("nan")):
            with self.subTest(value=value), self.assertRaises(m.MeasurementError):
                m.validate_lifecycle(
                    baseline, clock_id="owned-clock", command_elapsed_seconds=value
                )

    def test_wrapper_forwards_exact_arguments_result_and_exception(self):
        source = Manual()
        trace = m.Trace(m.Clock(source))
        a, b, result, error = object(), object(), object(), RuntimeError("original")
        seen = []

        def target(x, *, y):
            seen.append((x, y))
            source.value += 10
            return result

        wrapped = trace.wrapper("public", target)
        self.assertIs(wrapped(a, y=b), result)
        self.assertEqual(seen, [(a, b)])

        def failing():
            raise error

        try:
            trace.wrapper("failing", failing)()
        except RuntimeError as observed:
            self.assertIs(observed, error)
        else:
            self.fail("original exception missing")
        self.assertTrue(trace.validate())
        self.assertEqual([v["outcome"] for v in trace.rows], ["returned", "exception"])

    def test_nested_span_and_capacity_controls(self):
        source = Manual()
        trace = m.Trace(m.Clock(source), maximum=2)
        with trace.span("parent"):
            source.value = 1
            with trace.span("child"):
                source.value = 2
            source.value = 3
        self.assertTrue(trace.validate())
        self.assertEqual(trace.rows[1]["parent"], 0)
        invoked = []
        with self.assertRaises(m.MeasurementError):
            trace.wrapper("overflow", lambda: invoked.append(True))()
        self.assertEqual(invoked, [])
        self.assertTrue(trace.overflowed)
        with self.assertRaises(m.MeasurementError):
            trace.validate()

    def test_clock_regression_retains_original_failure(self):
        source = Manual()
        clock = m.Clock(source)
        trace = m.Trace(clock)
        source.value = 5
        original = RuntimeError("operation")
        try:
            with trace.span("operation"):
                source.value = 4
                raise original
        except BaseExceptionGroup as error:
            self.assertIs(error.exceptions[0], original)
            self.assertIsInstance(error.exceptions[1], m.MeasurementError)
        else:
            self.fail("both failures must survive")


if __name__ == "__main__":
    unittest.main()
