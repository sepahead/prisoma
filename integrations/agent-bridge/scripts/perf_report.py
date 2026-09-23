"""Reopen original timing rows and calculate prespecified paired summaries."""

import hashlib
import importlib.util
import json
from pathlib import Path
import sys

path = Path(__file__).with_name("perf_common.py")
spec = importlib.util.spec_from_file_location("perf_report_common", path)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


def measured_rows(case, result):
    return [
        {
            **row,
            "block": case["block"],
            "instrumentation": case["instrumentation"],
            "route_service_ns": row["route_complete_ns"] - row["start_ns"],
            "export_service_ns": row["export_complete_ns"] - row["route_complete_ns"],
            "route_offered_delay_ns": row["route_complete_ns"] - row["offer_ns"],
            "export_offered_delay_ns": row["export_complete_ns"] - row["offer_ns"],
            "start_lateness_ns": row["start_ns"] - row["offer_ns"],
        }
        for row in result["ticks"]
    ]


METRICS = (
    "route_service_ns",
    "export_service_ns",
    "route_offered_delay_ns",
    "export_offered_delay_ns",
    "start_lateness_ns",
)


def summarize(rows):
    return {
        "completed_samples": len(rows),
        "route_deadline_misses": sum(row["route_missed"] for row in rows),
        "export_deadline_misses": sum(row["export_missed"] for row in rows),
        "metrics_nanoseconds": {
            key: m.quantiles([row[key] for row in rows]) for key in METRICS
        },
    }


def report(campaign):
    terminal_path = campaign / "result.json"
    _, terminal = m.read_json(terminal_path)
    freeze_raw = m.selected_file(terminal["freeze"])
    freeze = m.parse_json(freeze_raw)
    m.require(
        terminal["status"] == "COMPLETE_ACCOUNTING"
        and len(terminal["completed"]) == len(freeze["cases"]) == 192
        and terminal["uncompleted_cases"] == []
        and terminal["failure"] is None,
        "only complete fixed cohorts have paired summaries",
    )
    m.require(
        m.identity(Path(__file__).resolve()) == freeze["tools"]["report"],
        "report tool identity",
    )
    groups, setup, spans, preparation_scopes = {}, {}, {}, {}
    for case, receipt in zip(freeze["cases"], terminal["completed"], strict=True):
        m.require(case == receipt["case"], "fixed complete case order")
        result = m.parse_json(m.selected_file(receipt["result"]))
        command = m.parse_json(m.selected_file(receipt["command"]))
        m.require(
            result["case"] == case
            and result["freeze_sha256"] == hashlib.sha256(freeze_raw).hexdigest(),
            "case identity",
        )
        m.require(
            result["planned_ticks"] == result["completed_ticks"] == 24,
            "complete case tick extent",
        )
        m.validate_lifecycle(
            result,
            clock_id=case["clock_id"],
            command_elapsed_seconds=command["elapsed_seconds"],
        )
        key = case["route"] + "/" + case["instrumentation"]
        if key in preparation_scopes:
            m.require(
                preparation_scopes[key] == result["preparation_scope"],
                "stable per-route preparation scope",
            )
        preparation_scopes[key] = result["preparation_scope"]
        groups.setdefault(key, []).extend(measured_rows(case, result))
        setup.setdefault(key, []).append(
            {
                "prepare_ns": result["prepare_end_ns"] - result["prepare_start_ns"],
                "finish_ns": result["finish_end_ns"] - result["finish_start_ns"],
                "context_retirement_ns": result["retired_ns"] - result["finish_end_ns"],
                "owned_command_seconds": command["elapsed_seconds"],
            }
        )
        # Inclusive spans are reported by label, never summed across nesting.
        for row in result["spans"]:
            m.require(
                type(row["start_ns"]) is int
                and type(row["end_ns"]) is int
                and row["end_ns"] >= row["start_ns"]
                and row["outcome"] == "returned",
                "complete finite span",
            )
            spans.setdefault(key, {}).setdefault(row["label"], []).append(
                row["end_ns"] - row["start_ns"]
            )
    warm = {
        key: [row for row in rows if row["tick"] > 6] for key, rows in groups.items()
    }
    m.require(
        all(len(rows) == 768 for rows in groups.values())
        and all(len(rows) == 576 for rows in warm.values()),
        "fixed full and warm extents",
    )
    paired = {}
    for level in ("minimal", "detailed"):
        for left, right in (
            ("body", "direct"),
            ("canonical", "body"),
            ("canonical", "direct"),
        ):
            key = left + "_minus_" + right + "/" + level
            paired[key] = {
                metric: m.quantiles(
                    m.paired_differences(
                        warm[left + "/" + level], warm[right + "/" + level], metric
                    )
                )
                for metric in METRICS
            }
    instrumentation = {}
    for route in ("direct", "body", "canonical"):
        left = [
            {**row, "instrumentation": "paired"} for row in warm[route + "/detailed"]
        ]
        right = [
            {**row, "instrumentation": "paired"} for row in warm[route + "/minimal"]
        ]
        instrumentation[route] = {
            metric: m.quantiles(m.paired_differences(left, right, metric))
            for metric in METRICS
        }
    result = {
        "schema": "local.m1-performance-report.v1",
        "status": "COMPLETE_COARSE_MEASUREMENT",
        "campaign_result": m.identity(terminal_path),
        "freeze": terminal["freeze"],
        "full_24_ticks": {key: summarize(rows) for key, rows in groups.items()},
        "warmed_ticks_7_through_24": {
            key: summarize(rows) for key, rows in warm.items()
        },
        "paired_whole_route_differences_nanoseconds": paired,
        "detailed_minus_minimal_observations_nanoseconds": instrumentation,
        "startup_and_retirement": {
            key: {
                metric: m.quantiles([row[metric] for row in rows]) for metric in rows[0]
            }
            for key, rows in setup.items()
        },
        "preparation_scopes": preparation_scopes,
        "inclusive_span_nanoseconds": {
            key: {label: m.quantiles(values) for label, values in labels.items()}
            for key, labels in spans.items()
        },
        "limits": [
            "Every sample retains its original 120-Hz offer and deadline",
            "At 576 warm samples per arm, p99.9 is the observed maximum, not a tail guarantee",
            "Whole-route differences are from separate executions, not isolated transport overhead",
            "Frame reads include remote service, scheduling, and backpressure",
            "Detailed/minimal differences include execution noise and cannot be subtracted as exact instrumentation cost",
            "Preparation spans include different route-specific verification work; compare their declared scopes before interpreting startup costs",
            "Native copies, allocation counts, GPU resources, long-run and contention qualification remain unmeasured",
            "Other authorized work was active on this machine; no dedicated-host claim",
        ],
        "authority": freeze["authority"],
    }
    return m.exclusive_json(campaign / "report.json", result)


if __name__ == "__main__":
    print(json.dumps(report(Path(sys.argv[1]))))
