"""Bounded clocks and accounting for a prospectively frozen local study."""

from __future__ import annotations

from contextlib import contextmanager
from functools import wraps
import importlib.util
import json
import math
from pathlib import Path
import time

MAX_TICKS = 1022
MAX_SPANS = 32768
MAX_JSON = 8 * 1024 * 1024

# Share the maintained file admission and metadata parser without installing Prisoma.
_FILE_SPEC = importlib.util.spec_from_file_location(
    "performance_file_owner", Path(__file__).with_name("m1_campaign.py")
)
_FILES = importlib.util.module_from_spec(_FILE_SPEC)
_FILE_SPEC.loader.exec_module(_FILES)


def parse_json(raw):
    value = _FILES.parse(raw)
    pending = [value]
    while pending:
        item = pending.pop()
        require(
            type(item) is not float or math.isfinite(item), "finite metadata numbers"
        )
        if type(item) is dict:
            pending.extend(item.values())
        elif type(item) is list:
            pending.extend(item)
    return value


def read_json(path):
    raw = _FILES.read(Path(path), maximum=MAX_JSON)
    return raw, parse_json(raw)


class MeasurementError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise MeasurementError(message)


def exact_int(value, low=0, high=2**63 - 1):
    return type(value) is int and low <= value <= high


def schedule_ns(origin, tick):
    require(exact_int(origin) and exact_int(tick, 1, MAX_TICKS), "schedule extent")
    # Round the offer upward so the projected instant is never before its rational time.
    offer = origin + ((tick - 1) * 1_000_000_000 + 119) // 120
    return offer, origin * 120 + tick * 1_000_000_000


def missed_deadline(end, deadline_numerator):
    require(exact_int(end) and exact_int(deadline_numerator), "deadline extent")
    return end * 120 > deadline_numerator


def nearest_rank(values, probability):
    require(
        type(probability) in (int, float)
        and math.isfinite(probability)
        and 0 < probability <= 1,
        "probability",
    )
    require(
        type(values) is list
        and bool(values)
        and all(type(x) in (int, float) and math.isfinite(x) for x in values),
        "finite nonempty sample",
    )
    return sorted(values)[math.ceil(probability * len(values)) - 1]


def quantiles(values):
    return {
        "n": len(values),
        "p50": nearest_rank(values, 0.5),
        "p99": nearest_rank(values, 0.99),
        "p99_9": nearest_rank(values, 0.999),
        "max": max(values),
        "method": "nearest_rank",
    }


def paired_differences(left, right, field):
    def indexed(rows):
        result = {}
        for row in rows:
            key = (row["block"], row["tick"], row["instrumentation"])
            require(key not in result, "duplicate pairing key")
            value = row[field]
            require(type(value) in (int, float) and math.isfinite(value), "finite pair")
            result[key] = value
        return result

    a, b = indexed(left), indexed(right)
    require(a.keys() == b.keys() and bool(a), "complete paired keys")
    return [a[key] - b[key] for key in sorted(a)]


def validate_tick_rows(rows, *, clock_id, origin_ns, planned_ticks, completed_ticks):
    require(
        type(rows) is list
        and len(rows) == completed_ticks
        and exact_int(planned_ticks, 1, MAX_TICKS)
        and exact_int(completed_ticks, 0, planned_ticks),
        "tick roster",
    )
    previous_export = origin_ns
    for tick, row in enumerate(rows, 1):
        require(
            exact_int(row["tick"], 1, planned_ticks)
            and row["tick"] == tick
            and row["clock_id"] == clock_id,
            "tick clock join",
        )
        offer, deadline = schedule_ns(origin_ns, tick)
        require(
            exact_int(row["offer_ns"])
            and exact_int(row["deadline_numerator_ns_x120"])
            and row["offer_ns"] == offer
            and row["deadline_numerator_ns_x120"] == deadline,
            "fixed offered schedule",
        )
        points = [
            row[key]
            for key in (
                "start_ns",
                "observation_ns",
                "route_complete_ns",
                "export_complete_ns",
            )
        ]
        require(
            all(exact_int(x) for x in points) and points == sorted(points),
            "timing order",
        )
        require(points[0] >= max(offer, previous_export), "sequential admitted start")
        require(
            type(row["route_missed"]) is bool
            and type(row["export_missed"]) is bool
            and row["route_missed"] == missed_deadline(points[2], deadline)
            and row["export_missed"] == missed_deadline(points[3], deadline),
            "deadline classification",
        )
        previous_export = points[3]


def validate_lifecycle(result, *, clock_id, command_elapsed_seconds):
    """Reject invalid complete-case intervals before calculating any duration."""
    require(
        type(clock_id) is str and bool(clock_id) and result["clock_id"] == clock_id,
        "lifecycle clock join",
    )
    keys = (
        "prepare_start_ns",
        "prepare_end_ns",
        "offered_origin_ns",
        "finish_start_ns",
        "finish_end_ns",
        "retired_ns",
    )
    points = [result[key] for key in keys]
    require(
        all(exact_int(value) for value in points)
        and points == sorted(points)
        and result["prepare_end_ns"] == result["offered_origin_ns"],
        "complete lifecycle timing order",
    )
    require(
        type(command_elapsed_seconds) in (int, float)
        and math.isfinite(command_elapsed_seconds)
        and command_elapsed_seconds >= 0,
        "finite nonnegative owned command duration",
    )
    validate_tick_rows(
        result["ticks"],
        clock_id=clock_id,
        origin_ns=result["offered_origin_ns"],
        planned_ticks=result["planned_ticks"],
        completed_ticks=result["completed_ticks"],
    )
    require(
        result["completed_ticks"] == result["planned_ticks"]
        and result["ticks"][-1]["export_complete_ns"] <= result["finish_start_ns"],
        "completed work precedes retirement",
    )


class Clock:
    def __init__(self, source=time.perf_counter_ns):
        self._source = source
        self.base = source()
        require(exact_int(self.base), "clock base")
        self.last = 0

    def now(self):
        value = self._source() - self.base
        require(exact_int(value) and value >= self.last, "clock regressed")
        self.last = value
        return value

    def wait_until(self, target):
        require(exact_int(target), "offer extent")
        while True:
            remaining = target - self.now()
            if remaining <= 0:
                return
            time.sleep(remaining / 1_000_000_000)


class Trace:
    """Synchronous, single-thread spans. Child processes have different clocks."""

    def __init__(self, clock, *, maximum=MAX_SPANS):
        require(exact_int(maximum, 1, MAX_SPANS), "span capacity")
        self.clock, self.maximum = clock, maximum
        self.rows, self.stack = [], []
        self.tick = 0
        self.overflowed = False

    @contextmanager
    def span(self, label):
        require(type(label) is str and 0 < len(label) <= 96, "span label")
        if len(self.rows) >= self.maximum:
            self.overflowed = True
            raise MeasurementError("span capacity exhausted before invocation")
        ordinal = len(self.rows)
        row = {
            "id": ordinal,
            "parent": self.stack[-1] if self.stack else None,
            "label": label,
            "tick": self.tick,
            "start_ns": self.clock.now(),
            "end_ns": None,
            "outcome": "pending",
        }
        self.rows.append(row)
        self.stack.append(ordinal)
        primary = None
        try:
            yield row
        except BaseException as error:
            primary = error
            row["outcome"] = "exception"
            raise
        else:
            row["outcome"] = "returned"
        finally:
            try:
                row["end_ns"] = self.clock.now()
                require(self.stack.pop() == ordinal, "span stack")
            except BaseException as error:
                if primary is not None:
                    raise BaseExceptionGroup(
                        "operation and timing cleanup failed", [primary, error]
                    )
                raise

    def wrapper(self, label, function):
        @wraps(function)
        def measured(*args, **kwargs):
            with self.span(label):
                return function(*args, **kwargs)

        return measured

    def validate(self):
        require(not self.stack and not self.overflowed, "complete span trace")
        for i, row in enumerate(self.rows):
            require(
                row["id"] == i and row["outcome"] in ("returned", "exception"),
                "span completion",
            )
            require(
                exact_int(row["start_ns"])
                and exact_int(row["end_ns"])
                and row["start_ns"] <= row["end_ns"],
                "span interval",
            )
            parent = row["parent"]
            if parent is not None:
                require(exact_int(parent, 0, i - 1), "span parent")
                enclosing = self.rows[parent]
                require(
                    enclosing["start_ns"]
                    <= row["start_ns"]
                    <= row["end_ns"]
                    <= enclosing["end_ns"],
                    "span nesting",
                )
        return True


def identity(path):
    return _FILES.file_identity(Path(path).absolute())


def selected_file(row, maximum=MAX_JSON):
    return _FILES.selected_file(row, maximum=maximum)


def exclusive_json(path, value):
    raw = (json.dumps(value, allow_nan=False, indent=2) + "\n").encode()
    require(len(raw) <= MAX_JSON, "result size")
    with Path(path).open("xb") as stream:
        stream.write(raw)
        stream.flush()
        import os

        os.fsync(stream.fileno())
    return identity(path)
