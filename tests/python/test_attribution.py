"""Tests for the attribution ranking-sensitivity probe (numpy; torch optional)."""

from __future__ import annotations

import hashlib
import json
from dataclasses import replace
from pathlib import PurePosixPath

import numpy as np
import pytest

import experiments.attribution.runlog as attribution_runlog
from experiments.attribution import (
    AttributionRecord,
    AttributionValidationCase,
    ProbeValidationCase,
    RankingSensitivityGate,
    SmallTransformer,
    canonical_hash,
    faithfulness_check,
    finite_difference_gradient,
    grad_times_input,
    lrp_epsilon,
    ranking_sensitivity_check,
    run_attribution_probe,
    write_attribution_runlog,
)


def _model_and_input(seed: int = 0, tokens: int = 6, d_in: int = 5, d_model: int = 8):
    model = SmallTransformer(d_in=d_in, d_model=d_model, seed=seed)
    rng = np.random.default_rng(seed + 100)
    x = rng.standard_normal((tokens, d_in))
    return model, x


def _gate(**overrides):
    values = {
        "frozen_gate_id": "test-ranking-gate-v1",
        "baseline_name": "explicit_zero_test_baseline",
        "baseline_provenance": "test fixture constructs a shape-matched zero tensor",
        "validation_split": "heldout-test",
        "selection_split": "selection-test",
        "grouping_provenance": "one independent synthetic fixture per group",
        "selection_group_ids": ("selection-group",),
        "selection_unit_ids": ("selection-unit",),
        "alpha": 0.05,
        "min_groups": 5,
        "n_steps": 6,
        "n_random_rankings": 128,
        "seed": 19,
    }
    values.update(overrides)
    return RankingSensitivityGate(**values)


def _probe_cases(seed: int, count: int = 6):
    rng = np.random.default_rng(seed + 500)
    return [
        ProbeValidationCase(
            case_id=f"case-{index}",
            group_id=f"validation-group-{index}",
            unit_ids=(f"validation-unit-{index}",),
            x=rng.standard_normal((6, 5)),
            baseline=np.zeros((6, 5), dtype=np.float64),
        )
        for index in range(count)
    ]


def _linear_validation_cases(kind: str = "informative", count: int = 6):
    weights = np.array([16.0, 8.0, 4.0, 2.0, 1.0, 0.5])
    cases = []
    for index in range(count):
        x = np.ones(6, dtype=np.float64) * (1.0 + index / 20.0)
        informative = weights * x
        if kind == "informative":
            attribution = informative
        elif kind == "adversarial":
            attribution = informative[::-1]
        elif kind == "constant":
            attribution = np.ones_like(x)
        else:
            raise ValueError(f"unknown fixture kind: {kind}")
        cases.append(
            AttributionValidationCase(
                case_id=f"linear-case-{index}",
                group_id=f"linear-group-{index}",
                unit_ids=(f"linear-unit-{index}",),
                x=x,
                attribution=attribution,
                baseline=np.zeros_like(x),
            )
        )
    return weights, cases


def test_forward_is_deterministic_and_shaped():
    model, x = _model_and_input()
    a = model.forward(x)
    b = model.forward(x)
    assert a == b
    assert isinstance(a, float)
    cache = model.forward_cache(x)
    assert cache.attn_weights.shape == (x.shape[0], x.shape[0])
    # Attention rows are a softmax (sum to 1).
    assert np.allclose(cache.attn_weights.sum(axis=1), 1.0)


def test_lrp_epsilon_conserves_relevance():
    # Epsilon-LRP with bias-free layers conserves relevance: total input relevance
    # approximates the scalar target as eps -> 0.
    model, x = _model_and_input(seed=1)
    target = model.forward(x)
    relevance = lrp_epsilon(model, x, eps=1e-8)
    assert relevance.shape == x.shape
    assert abs(relevance.sum() - target) < 1e-3 * (1.0 + abs(target))


def test_grad_times_input_shape_and_linear_sanity():
    # On a single-token input the attention is trivial (softmax over one key = 1),
    # so the model is linear in x and grad-x-input sums to the target.
    model = SmallTransformer(d_in=4, d_model=6, seed=2)
    x = np.array([[0.4, -0.2, 0.1, 0.7]])
    gxi = grad_times_input(model, x)
    assert gxi.shape == x.shape
    assert abs(gxi.sum() - model.forward(x)) < 1e-4


def test_group_level_ranking_sensitivity_passes_informative_linear_ranking():
    weights, cases = _linear_validation_cases()
    result = ranking_sensitivity_check(
        lambda value: float(np.dot(weights, value)), cases, gate=_gate()
    )
    assert result.passed
    assert result.status == "passed"
    assert result.p_value == pytest.approx(1 / 64)
    assert result.positive_groups == 6
    assert result.method_aopc > result.random_aopc


@pytest.mark.parametrize("kind", ["constant", "adversarial"])
def test_group_level_ranking_sensitivity_rejects_null_and_adversarial(kind):
    weights, cases = _linear_validation_cases(kind)
    result = faithfulness_check(
        lambda value: float(np.dot(weights, value)), cases, gate=_gate()
    )
    assert not result.passed
    assert result.status == "failed"


def test_group_level_ranking_sensitivity_rejects_constant_predictor():
    _, cases = _linear_validation_cases()
    result = ranking_sensitivity_check(lambda _value: 1.0, cases, gate=_gate())
    assert not result.passed
    assert result.reason == "ranking_not_better_than_random_across_groups"


@pytest.mark.parametrize(
    ("gate", "reason"),
    [
        (
            _gate(selection_group_ids=("linear-group-2",)),
            "selection_validation_leakage",
        ),
        (
            _gate(selection_unit_ids=("linear-unit-3",)),
            "selection_validation_leakage",
        ),
        (
            _gate(selection_split="heldout-test"),
            "selection_validation_split_not_disjoint",
        ),
    ],
)
def test_group_level_gate_abstains_on_declared_selection_leakage(gate, reason):
    weights, cases = _linear_validation_cases()
    result = ranking_sensitivity_check(
        lambda value: float(np.dot(weights, value)), cases, gate=gate
    )
    assert result.status == "abstained"
    assert result.reason == reason
    assert not result.passed
    assert result.p_value is None


@pytest.mark.parametrize(
    ("replacement", "reason"),
    [
        ({"group_id": "linear-group-0"}, "validation_groups_not_disjoint"),
        ({"unit_ids": ("linear-unit-0",)}, "validation_units_not_disjoint"),
    ],
)
def test_group_level_gate_abstains_when_validation_partition_leaks(replacement, reason):
    weights, cases = _linear_validation_cases()
    cases[1] = replace(cases[1], **replacement)
    result = ranking_sensitivity_check(
        lambda value: float(np.dot(weights, value)), cases, gate=_gate()
    )
    assert result.status == "abstained"
    assert result.reason == reason


def test_group_level_gate_abstains_on_insufficient_independent_cases():
    weights, cases = _linear_validation_cases(count=4)
    result = ranking_sensitivity_check(
        lambda value: float(np.dot(weights, value)), cases, gate=_gate()
    )
    assert result.status == "abstained"
    assert result.reason == "insufficient_independent_validation_groups"


@pytest.mark.parametrize(
    ("case_change", "message"),
    [
        ({"attribution": np.ones(5)}, "attribution shape"),
        ({"baseline": np.ones(5)}, "baseline shape"),
        ({"x": np.array([1.0, 2.0, 3.0, 4.0, 5.0, np.nan])}, "finite"),
        ({"unit_ids": ()}, "nonempty tuple"),
    ],
)
def test_group_level_gate_rejects_invalid_case_inputs(case_change, message):
    weights, cases = _linear_validation_cases()
    cases[0] = replace(cases[0], **case_change)
    with pytest.raises(ValueError, match=message):
        ranking_sensitivity_check(
            lambda value: float(np.dot(weights, value)), cases, gate=_gate()
        )


@pytest.mark.parametrize(
    ("gate", "message"),
    [
        (_gate(baseline_name=""), "baseline_name"),
        (_gate(baseline_provenance=""), "baseline_provenance"),
        (_gate(n_random_rankings=99), "under-resolved"),
        (_gate(min_groups=4), "cannot attain alpha"),
        (_gate(alpha=float("nan")), "alpha"),
        (_gate(n_steps=1), "n_steps"),
    ],
)
def test_group_level_gate_rejects_invalid_frozen_parameters(gate, message):
    weights, cases = _linear_validation_cases()
    with pytest.raises(ValueError, match=message):
        ranking_sensitivity_check(
            lambda value: float(np.dot(weights, value)), cases, gate=gate
        )


def test_group_level_gate_rejects_nonfinite_predictor_output():
    _, cases = _linear_validation_cases()
    with pytest.raises(ValueError, match="non-finite"):
        ranking_sensitivity_check(lambda _value: float("nan"), cases, gate=_gate())


def test_run_probe_emits_records_with_verdicts():
    model, _ = _model_and_input(seed=4)
    records = run_attribution_probe(
        model,
        _probe_cases(4),
        gate=_gate(),
        target_output="action_dim_0",
        modality="vision",
    )
    assert {r.method for r in records} == {"lrp_epsilon", "grad_x_input"}
    for rec in records:
        assert rec.target_output == "action_dim_0"
        assert rec.modality == "vision"
        assert rec.baseline == "explicit_zero_test_baseline"
        assert rec.metadata["diagnostic"] == "deletion_ranking_sensitivity"
        assert rec.metadata["causal_or_mechanistic_faithfulness_established"] == "false"
        assert rec.metadata["independent_groups"] == "6"
        assert rec.faithfulness_passed == (rec.metadata["gate_status"] == "passed")
        assert len(rec.metadata["validation_input_baseline_set_sha256"]) == 64
        assert len(rec.metadata["validation_relevance_set_sha256"]) == 64


def test_run_probe_logs_insufficient_group_abstention_as_false():
    model, _ = _model_and_input(seed=7)
    records = run_attribution_probe(
        model, _probe_cases(7, count=4), gate=_gate(), methods=("lrp_epsilon",)
    )
    assert len(records) == 1
    assert not records[0].faithfulness_passed
    assert records[0].metadata["gate_status"] == "abstained"
    assert (
        records[0].metadata["gate_reason"]
        == "insufficient_independent_validation_groups"
    )


def test_canonical_hash_matches_sorted_compact_json():
    cfg = {"experiment": "attribution_probe", "model": "small_transformer", "n": "3"}
    expected = canonical_hash(cfg)
    # Recompute the documented serialization independently.
    payload = json.dumps(cfg, sort_keys=True, separators=(",", ":")).encode()
    assert expected == hashlib.sha256(payload).hexdigest()


def test_producer_limits_match_the_rerun_preparation_envelope():
    assert attribution_runlog.MAX_RERUN_EVENTS == 100_000
    assert attribution_runlog.MAX_RERUN_SERIALIZED_EVENT_BYTES == 64 * 1024 * 1024
    assert attribution_runlog.MAX_RERUN_PREPARED_ARTIFACT_BYTES == 8 * 1024 * 1024


def test_write_runlog_is_schema_shaped(tmp_path):
    model, _ = _model_and_input(seed=5)
    records = run_attribution_probe(model, _probe_cases(5), gate=_gate())
    out = write_attribution_runlog(
        tmp_path / "attr.jsonl",
        records,
        config={"model": "small_transformer", "target_output": "action_dim_0"},
        artifact_dir=tmp_path / "artifacts",
    )
    lines = [json.loads(line) for line in out.read_text().splitlines()]
    types = [e["type"] for e in lines]
    assert types[0] == "run_started"
    assert types[1] == "config_logged"
    assert types[-1] == "run_ended"
    assert types.count("attribution_logged") == len(records)
    # run_started and config_logged config_hash agree (validator requirement).
    assert lines[0]["config_hash"] == lines[1]["config_hash"]
    # config_hash equals the canonical hash of the logged config.
    assert lines[1]["config_hash"] == canonical_hash(lines[1]["config"])
    # Each attribution event carries the required fields and a confined, portable
    # NumPy v1.0 little-endian f64 C-order artifact that the converter can load.
    attribution_events = [e for e in lines if e["type"] == "attribution_logged"]
    for event, record in zip(attribution_events, records, strict=True):
        assert event["method"]
        assert event["target_output"]
        assert isinstance(event["faithfulness_check"], bool)
        assert event["score_hash"]

        artifact_uri = event["artifact_uri"]
        uri_path = PurePosixPath(artifact_uri)
        assert not uri_path.is_absolute()
        assert ".." not in uri_path.parts
        assert "\\" not in artifact_uri
        artifact_path = out.parent.joinpath(*uri_path.parts)
        artifact_bytes = artifact_path.read_bytes()
        assert artifact_bytes[:8] == b"\x93NUMPY\x01\x00"
        assert (
            event["metadata"]["artifact_sha256"]
            == hashlib.sha256(artifact_bytes).hexdigest()
        )
        assert artifact_path.name == f"{event['metadata']['artifact_sha256']}.npy"

        with artifact_path.open("rb") as handle:
            assert np.lib.format.read_magic(handle) == (1, 0)
            shape, fortran_order, dtype = np.lib.format.read_array_header_1_0(handle)
        assert shape == record.relevance.shape
        assert not fortran_order
        assert dtype.str == "<f8"

        loaded = np.load(artifact_path, allow_pickle=False)
        assert loaded.dtype.str == "<f8"
        assert loaded.flags.c_contiguous
        assert np.array_equal(loaded, record.relevance)


@pytest.mark.parametrize(
    "run_id",
    ["", " surrounding-space ", "e\u0301", 7],
    ids=["empty", "whitespace", "non-nfc", "non-string"],
)
def test_write_runlog_rejects_noncanonical_run_id_without_outputs(tmp_path, run_id):
    runlog_path = tmp_path / "run" / "attr.jsonl"
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )

    with pytest.raises(ValueError, match="run_id"):
        write_attribution_runlog(runlog_path, [record], run_id=run_id)

    assert not runlog_path.parent.exists()


@pytest.mark.parametrize(
    ("metadata", "message"),
    [
        ({1: "numeric", "1": "text"}, "exact strings"),
        ({"score": 1}, "exact strings"),
        ({"é": "canonical", "e\u0301": "decomposed"}, "normalization collision"),
        ({"relevance_shape": "forged"}, "reserved"),
        ({"artifact_sha256": "0" * 64}, "reserved"),
    ],
    ids=[
        "key-coercion-collision",
        "value-coercion",
        "unicode-key-collision",
        "shape-reserved",
        "digest-reserved",
    ],
)
def test_write_runlog_rejects_nonexact_metadata_without_outputs(
    tmp_path, metadata, message
):
    runlog_path = tmp_path / "run" / "attr.jsonl"
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
        metadata=metadata,
    )

    with pytest.raises(ValueError, match=message):
        write_attribution_runlog(
            runlog_path,
            [record],
            artifact_dir=runlog_path.parent / "artifacts",
        )

    assert not runlog_path.parent.exists()


def test_write_runlog_rejects_overlong_method_before_outputs(tmp_path):
    runlog_path = tmp_path / "run" / "attr.jsonl"
    record = AttributionRecord(
        method="m" * (attribution_runlog.MAX_RUNLOG_STRING_BYTES + 1),
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )

    with pytest.raises(ValueError, match="JSON string"):
        write_attribution_runlog(
            runlog_path,
            [record],
            artifact_dir=runlog_path.parent / "artifacts",
        )

    assert not runlog_path.parent.exists()


def test_write_runlog_rejects_excessive_json_nesting_before_outputs(tmp_path):
    runlog_path = tmp_path / "run" / "attr.jsonl"
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )
    nested: object = "leaf"
    for _ in range(attribution_runlog.MAX_RUNLOG_NESTING_DEPTH):
        nested = [nested]

    with pytest.raises(ValueError, match="nesting"):
        write_attribution_runlog(
            runlog_path,
            [record],
            config={"nested": nested},
            artifact_dir=runlog_path.parent / "artifacts",
        )

    assert not runlog_path.parent.exists()


@pytest.mark.parametrize(
    ("limit_name", "limit", "message"),
    [
        ("MAX_RERUN_EVENTS", 3, "event count"),
        (
            "MAX_RERUN_SERIALIZED_EVENT_BYTES",
            128,
            "serialized-event aggregate",
        ),
        ("MAX_RUNLOG_LINE_BYTES", 64, "line"),
        ("MAX_RUNLOG_FILE_BYTES", 128, "aggregate bytes"),
    ],
)
def test_write_runlog_enforces_canonical_aggregate_budgets_before_outputs(
    tmp_path, monkeypatch, limit_name, limit, message
):
    monkeypatch.setattr(attribution_runlog, limit_name, limit)
    runlog_path = tmp_path / limit_name / "attr.jsonl"
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )

    with pytest.raises(ValueError, match=message):
        write_attribution_runlog(
            runlog_path,
            [record],
            artifact_dir=runlog_path.parent / "artifacts",
        )

    assert not runlog_path.parent.exists()


def test_prepared_artifact_budget_counts_unique_hashes_before_outputs(
    tmp_path, monkeypatch
):
    monkeypatch.setattr(attribution_runlog, "MAX_RERUN_PREPARED_ARTIFACT_BYTES", 200)

    def record(value):
        return AttributionRecord(
            method="lrp_epsilon",
            target_output="action_dim_0",
            relevance=np.array([value], dtype=np.float64),
            faithfulness_passed=True,
        )

    rejected_log = tmp_path / "rejected" / "attr.jsonl"
    with pytest.raises(ValueError, match="unique prepared relevance artifacts"):
        write_attribution_runlog(
            rejected_log,
            [record(1.0), record(2.0)],
            artifact_dir=rejected_log.parent / "artifacts",
        )
    assert not rejected_log.parent.exists()

    accepted_log = tmp_path / "deduplicated" / "attr.jsonl"
    write_attribution_runlog(
        accepted_log,
        [record(1.0), record(1.0)],
        artifact_dir=accepted_log.parent / "artifacts",
    )
    assert accepted_log.is_file()
    assert len(list((accepted_log.parent / "artifacts").glob("*.npy"))) == 1


def test_bounded_serializer_enforces_array_and_object_limits(monkeypatch):
    monkeypatch.setattr(attribution_runlog, "MAX_RUNLOG_ARRAY_LEN", 1)
    with pytest.raises(ValueError, match="array length"):
        attribution_runlog._serialize_bounded_runlog(
            [{"type": "probe", "items": [1, 2]}]
        )

    monkeypatch.setattr(attribution_runlog, "MAX_RUNLOG_ARRAY_LEN", 1_000_000)
    monkeypatch.setattr(attribution_runlog, "MAX_RUNLOG_OBJECT_ENTRIES", 1)
    with pytest.raises(ValueError, match="object entries"):
        attribution_runlog._serialize_bounded_runlog([{"type": "probe", "value": 1}])


def test_write_runlog_rejects_external_artifact_dir_without_outputs(tmp_path):
    runlog_path = tmp_path / "run" / "attr.jsonl"
    artifact_dir = tmp_path / "external-artifacts"
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )

    with pytest.raises(ValueError, match="strict descendant"):
        write_attribution_runlog(
            runlog_path,
            [record],
            artifact_dir=artifact_dir,
        )

    assert not runlog_path.exists()
    assert not runlog_path.parent.exists()
    assert not artifact_dir.exists()


@pytest.mark.parametrize(
    ("bad_relevance", "message"),
    [
        (np.empty((0,), dtype=np.float64), "non-empty"),
        (np.array([np.nan], dtype=np.float64), "finite"),
        (np.zeros(1025, dtype=np.float64), "at most 1024"),
    ],
    ids=["empty", "nonfinite", "too-many-values"],
)
def test_invalid_later_record_leaves_no_partial_outputs(
    tmp_path, bad_relevance, message
):
    runlog_path = tmp_path / "run" / "attr.jsonl"
    artifact_dir = runlog_path.parent / "artifacts"
    valid = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )
    invalid = AttributionRecord(
        method="grad_x_input",
        target_output="action_dim_0",
        relevance=bad_relevance,
        faithfulness_passed=False,
    )

    with pytest.raises(ValueError, match=message):
        write_attribution_runlog(
            runlog_path,
            [valid, invalid],
            artifact_dir=artifact_dir,
        )

    assert not runlog_path.exists()
    assert not artifact_dir.exists()


def test_oversized_unsaved_relevance_is_rejected_before_outputs(tmp_path):
    runlog_path = tmp_path / "run" / "attr.jsonl"
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.zeros(1025, dtype=np.float64),
        faithfulness_passed=True,
    )

    with pytest.raises(ValueError, match="at most 1024"):
        write_attribution_runlog(runlog_path, [record])

    assert not runlog_path.exists()
    assert not runlog_path.parent.exists()


def test_changed_content_gets_new_uri_and_preserves_old_artifact(tmp_path):
    runlog_path = tmp_path / "attr.jsonl"
    artifact_dir = tmp_path / "artifacts"

    def write(relevance):
        record = AttributionRecord(
            method="lrp_epsilon",
            target_output="action_dim_0",
            relevance=np.asarray(relevance, dtype=np.float64),
            faithfulness_passed=True,
        )
        write_attribution_runlog(
            runlog_path,
            [record],
            artifact_dir=artifact_dir,
        )
        events = [json.loads(line) for line in runlog_path.read_text().splitlines()]
        event = next(e for e in events if e["type"] == "attribution_logged")
        path = runlog_path.parent / PurePosixPath(event["artifact_uri"])
        return event, path

    first_event, first_path = write([1.0, 2.0])
    first_bytes = first_path.read_bytes()
    second_event, second_path = write([3.0, 4.0])

    assert first_event["artifact_uri"] != second_event["artifact_uri"]
    assert first_path != second_path
    assert first_path.read_bytes() == first_bytes
    assert (
        hashlib.sha256(first_bytes).hexdigest()
        == first_event["metadata"]["artifact_sha256"]
    )
    assert np.array_equal(np.load(second_path, allow_pickle=False), [3.0, 4.0])


def test_default_run_id_isolated_across_logs_in_same_directory(tmp_path):
    artifact_dir = tmp_path / "artifacts"

    def publish(log_name, relevance):
        runlog_path = tmp_path / log_name
        record = AttributionRecord(
            method="lrp_epsilon",
            target_output="action_dim_0",
            relevance=np.asarray(relevance, dtype=np.float64),
            faithfulness_passed=True,
        )
        write_attribution_runlog(
            runlog_path,
            [record],
            artifact_dir=artifact_dir,
        )
        events = [json.loads(line) for line in runlog_path.read_text().splitlines()]
        event = next(e for e in events if e["type"] == "attribution_logged")
        artifact_path = runlog_path.parent / PurePosixPath(event["artifact_uri"])
        return runlog_path, event, artifact_path

    first_log, first_event, first_artifact = publish("first.jsonl", [1.0, 2.0])
    first_log_bytes = first_log.read_bytes()
    first_artifact_bytes = first_artifact.read_bytes()
    _, second_event, second_artifact = publish("second.jsonl", [3.0, 4.0])

    assert first_event["artifact_uri"] != second_event["artifact_uri"]
    assert first_artifact != second_artifact
    assert first_log.read_bytes() == first_log_bytes
    assert first_artifact.read_bytes() == first_artifact_bytes
    assert (
        hashlib.sha256(first_artifact_bytes).hexdigest()
        == first_event["metadata"]["artifact_sha256"]
    )


def test_later_artifact_install_failure_preserves_existing_publication(
    tmp_path, monkeypatch
):
    runlog_path = tmp_path / "attr.jsonl"
    artifact_dir = tmp_path / "artifacts"

    def record(method, values):
        return AttributionRecord(
            method=method,
            target_output="action_dim_0",
            relevance=np.asarray(values, dtype=np.float64),
            faithfulness_passed=True,
        )

    write_attribution_runlog(
        runlog_path,
        [record("lrp_epsilon", [1.0, 2.0]), record("grad_x_input", [3.0, 4.0])],
        artifact_dir=artifact_dir,
    )
    original_log_bytes = runlog_path.read_bytes()
    original_events = [
        event
        for event in map(json.loads, original_log_bytes.splitlines())
        if event["type"] == "attribution_logged"
    ]
    original_artifacts = {
        event["artifact_uri"]: (
            runlog_path.parent.joinpath(*PurePosixPath(event["artifact_uri"]).parts),
            event["metadata"]["artifact_sha256"],
        )
        for event in original_events
    }
    original_artifact_bytes = {
        uri: path.read_bytes() for uri, (path, _) in original_artifacts.items()
    }

    real_install = attribution_runlog._install_staged_artifact
    install_count = 0

    def fail_second_install(staged_path, artifact_path, artifact_bytes):
        nonlocal install_count
        install_count += 1
        if install_count == 2:
            raise OSError("injected later artifact install failure")
        real_install(staged_path, artifact_path, artifact_bytes)

    monkeypatch.setattr(
        attribution_runlog, "_install_staged_artifact", fail_second_install
    )
    with pytest.raises(OSError, match="injected later artifact install failure"):
        write_attribution_runlog(
            runlog_path,
            [
                record("lrp_epsilon", [10.0, 20.0]),
                record("grad_x_input", [30.0, 40.0]),
            ],
            artifact_dir=artifact_dir,
        )

    assert install_count == 2
    assert runlog_path.read_bytes() == original_log_bytes
    for uri, (artifact_path, expected_hash) in original_artifacts.items():
        artifact_bytes = artifact_path.read_bytes()
        assert artifact_bytes == original_artifact_bytes[uri]
        assert hashlib.sha256(artifact_bytes).hexdigest() == expected_hash
    assert not list(tmp_path.rglob(".attribution-stage-*.tmp"))


@pytest.mark.parametrize("invalid", [float("nan"), 0.1, None, -(2**63) - 1, 2**64])
def test_invalid_config_is_rejected_before_any_output(tmp_path, invalid):
    runlog_path = tmp_path / "run" / "attr.jsonl"
    artifact_dir = runlog_path.parent / "artifacts"
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )

    with pytest.raises(ValueError, match="canonical JSON value"):
        write_attribution_runlog(
            runlog_path,
            [record],
            config={"invalid": invalid},
            artifact_dir=artifact_dir,
        )

    assert not runlog_path.parent.exists()


@pytest.mark.parametrize("alias_kind", ["runlog-parent", "runlog-path"])
def test_write_runlog_rejects_artifact_topology_aliases(tmp_path, alias_kind):
    runlog_path = tmp_path / alias_kind / "attr.jsonl"
    artifact_dir = runlog_path.parent if alias_kind == "runlog-parent" else runlog_path
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )

    with pytest.raises(ValueError, match="strict descendant|must not alias"):
        write_attribution_runlog(
            runlog_path,
            [record],
            artifact_dir=artifact_dir,
        )

    assert not runlog_path.parent.exists()


def test_write_runlog_rejects_symlinked_publication_paths(tmp_path):
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )

    runlog_dir = tmp_path / "artifact-link-case"
    real_artifact_dir = runlog_dir / "real-artifacts"
    real_artifact_dir.mkdir(parents=True)
    linked_artifact_dir = runlog_dir / "linked-artifacts"
    linked_artifact_dir.symlink_to(real_artifact_dir, target_is_directory=True)
    linked_artifact_runlog = runlog_dir / "attr.jsonl"
    with pytest.raises(ValueError, match="must not contain symlinks"):
        write_attribution_runlog(
            linked_artifact_runlog,
            [record],
            artifact_dir=linked_artifact_dir,
        )
    assert not linked_artifact_runlog.exists()
    assert not list(real_artifact_dir.iterdir())

    target_runlog = tmp_path / "target.jsonl"
    target_runlog.write_text("existing target\n")
    linked_runlog = tmp_path / "linked.jsonl"
    linked_runlog.symlink_to(target_runlog)
    with pytest.raises(ValueError, match="must not contain symlinks"):
        write_attribution_runlog(
            linked_runlog,
            [record],
            artifact_dir=tmp_path / "other-artifacts",
        )
    assert target_runlog.read_text() == "existing target\n"
    assert not (tmp_path / "other-artifacts").exists()


def test_write_runlog_rejects_hard_linked_runlog_alias(tmp_path):
    target_runlog = tmp_path / "target.jsonl"
    target_runlog.write_text("existing target\n")
    linked_runlog = tmp_path / "linked.jsonl"
    linked_runlog.hardlink_to(target_runlog)
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )

    with pytest.raises(ValueError, match="hard-link aliases"):
        write_attribution_runlog(
            linked_runlog,
            [record],
            artifact_dir=tmp_path / "artifacts",
        )

    assert linked_runlog.read_text() == "existing target\n"
    assert target_runlog.read_text() == "existing target\n"
    assert not (tmp_path / "artifacts").exists()


@pytest.mark.parametrize("alias_kind", ["symlink", "hard-link"])
def test_write_runlog_rejects_artifact_destination_aliases(tmp_path, alias_kind):
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )
    probe_dir = tmp_path / "probe"
    probe_log = probe_dir / "attr.jsonl"
    write_attribution_runlog(
        probe_log,
        [record],
        artifact_dir=probe_dir / "artifacts",
    )
    probe_events = [json.loads(line) for line in probe_log.read_text().splitlines()]
    probe_event = next(
        event for event in probe_events if event["type"] == "attribution_logged"
    )
    artifact_name = PurePosixPath(probe_event["artifact_uri"]).name
    artifact_bytes = (
        probe_log.parent.joinpath(*PurePosixPath(probe_event["artifact_uri"]).parts)
    ).read_bytes()

    publication_dir = tmp_path / alias_kind
    artifact_dir = publication_dir / "artifacts"
    artifact_dir.mkdir(parents=True)
    alias_target = publication_dir / "alias-target.npy"
    alias_target.write_bytes(artifact_bytes)
    artifact_destination = artifact_dir / artifact_name
    if alias_kind == "symlink":
        artifact_destination.symlink_to(alias_target)
        expected_message = "regular non-symlink"
    else:
        artifact_destination.hardlink_to(alias_target)
        expected_message = "hard-link aliases"

    runlog_path = publication_dir / "attr.jsonl"
    with pytest.raises(ValueError, match=expected_message):
        write_attribution_runlog(
            runlog_path,
            [record],
            artifact_dir=artifact_dir,
        )

    assert not runlog_path.exists()
    assert alias_target.read_bytes() == artifact_bytes
    assert not list(publication_dir.rglob(".attribution-stage-*.tmp"))


def test_attribution_record_rejects_empty_fields():
    with pytest.raises(ValueError):
        AttributionRecord(
            method="",
            target_output="t",
            relevance=np.zeros((2, 2)),
            faithfulness_passed=True,
        )
    with pytest.raises(ValueError):
        AttributionRecord(
            method="m",
            target_output="",
            relevance=np.zeros((2, 2)),
            faithfulness_passed=True,
        )


def test_finite_difference_gradient_matches_torch_autograd():
    torch = pytest.importorskip("torch")
    model, x = _model_and_input(seed=6, tokens=4, d_in=3, d_model=5)
    fd = finite_difference_gradient(model, x, h=1e-5)

    # Rebuild the identical forward in torch and autograd the target wrt x.
    tx = torch.tensor(x, dtype=torch.float64, requires_grad=True)
    we = torch.tensor(model.w_embed, dtype=torch.float64)
    wq = torch.tensor(model.w_q, dtype=torch.float64)
    wk = torch.tensor(model.w_k, dtype=torch.float64)
    wv = torch.tensor(model.w_v, dtype=torch.float64)
    wo = torch.tensor(model.w_o, dtype=torch.float64)
    wh = torch.tensor(model.w_head, dtype=torch.float64)
    emb = tx @ we
    q, k, v = emb @ wq, emb @ wk, emb @ wv
    scores = (q @ k.T) / np.sqrt(model.d_model)
    attn = torch.softmax(scores, dim=-1)
    pooled = (attn @ v @ wo).mean(dim=0)
    target = (pooled @ wh)[0]
    target.backward()
    auto = tx.grad.numpy()
    assert np.allclose(fd, auto, atol=1e-5)
