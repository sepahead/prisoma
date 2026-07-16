"""Tests for the attribution ranking-sensitivity probe (numpy; torch optional)."""

from __future__ import annotations

import base64
import copy
import hashlib
import json
from dataclasses import replace
from pathlib import PurePosixPath

import numpy as np
import pytest

import experiments.attribution.runlog as attribution_runlog
import experiments.attribution.probe as attribution_probe
from experiments.attribution.__main__ import main as attribution_main
from experiments.attribution import (
    AttributionRecord,
    AttributionValidationCase,
    ProbeValidationCase,
    RankingSensitivityGate,
    SmallTransformer,
    bind_ranking_gate,
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
        "predictor_determinism_provenance": (
            "test predictor is a pure deterministic function of its array input"
        ),
        "selection_group_ids": ("selection-group",),
        "selection_unit_ids": ("selection-unit",),
        "alpha": 0.05,
        "min_groups": 5,
        "n_steps": 6,
        "n_random_rankings": 128,
        "seed": 19,
    }
    values.update(overrides)
    gate = RankingSensitivityGate(**values)
    try:
        return bind_ranking_gate(gate)
    except (OverflowError, ValueError):
        return gate


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


PRODUCER_EVIDENCE_METADATA_FIELDS = (
    "diagnostic",
    "gate_status",
    "gate_reason",
    "frozen_gate_id",
    "validation_split",
    "selection_split",
    "grouping_provenance",
    "baseline_provenance",
    "method_mean_absolute_deletion_sensitivity",
    "random_mean_absolute_deletion_sensitivity",
    "random_deletion_sensitivity_std",
    "group_win_binomial_p",
    "alpha",
    "validation_cases",
    "independent_groups",
    "winning_groups",
    "deletion_steps",
    "random_rankings_per_case",
    "random_reference_se_bound",
    "group_contrasts",
    "group_randomization_p_values",
    "ordered_group_ids",
    "representative_case_id",
    "validation_relevance_set_sha256",
    "validation_input_baseline_set_sha256",
    "baseline_may_be_out_of_distribution",
    "feature_dependence_unresolved",
    "causal_or_mechanistic_faithfulness_established",
    "method_implementation",
    "model_parameter_sha256",
    "gate_content_sha256",
    "probe_work_estimate_multiply_adds",
    "confirmatory_role",
    "multiplicity_policy",
)

EVIDENCE_CONFIG_BINDING_FIELDS = (
    "experiment",
    "n_records",
    "model",
    "model_parameter_sha256",
    "d_in",
    "d_model",
    "tokens",
    "validation_cases",
    "target_output",
    "modality",
    "layer",
    "baseline",
    "baseline_name",
    "diagnostic",
    "frozen_gate_id",
    "gate_content_sha256",
    "gate_manifest",
    "methods",
    "primary_method",
    "method_implementations",
    "validation_input_baseline_set_sha256",
    "case_set_sha256",
    "probe_work_estimate_multiply_adds",
    "seed",
)


@pytest.fixture(scope="module")
def passing_probe_batch():
    model, _ = _model_and_input(seed=28)
    records = run_attribution_probe(model, _probe_cases(28), gate=_gate())
    assert all(record.metadata["gate_status"] == "passed" for record in records)
    return tuple(records)


def _coherent_evidence_config(records):
    first = records[0]
    bundle = first.evidence_bundle
    assert bundle is not None
    model = bundle["model"]
    gate = bundle["gate"]
    cases = bundle["cases"]
    return {
        "experiment": "attribution_probe",
        "n_records": str(len(records)),
        "model": "small_transformer",
        "model_parameter_sha256": model["parameter_sha256"],
        "d_in": model["d_in"],
        "d_model": model["d_model"],
        "tokens": cases[0]["x"]["shape"][0],
        "validation_cases": len(cases),
        "target_output": first.target_output,
        "modality": first.modality or "not_declared",
        "layer": first.layer or "not_declared",
        "baseline": first.baseline,
        "baseline_name": first.baseline,
        "diagnostic": "deletion_ranking_sensitivity",
        "frozen_gate_id": bundle["frozen_gate_id"],
        "gate_content_sha256": bundle["gate_content_sha256"],
        "gate_manifest": gate,
        "methods": [record.method for record in records],
        "primary_method": bundle["primary_method"],
        "method_implementations": {
            record.method: record.metadata["method_implementation"]
            for record in records
        },
        "validation_input_baseline_set_sha256": bundle["case_set_sha256"],
        "case_set_sha256": bundle["case_set_sha256"],
        "probe_work_estimate_multiply_adds": bundle["work_estimate_multiply_adds"],
        "seed": gate["seed"],
    }


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


def test_finite_difference_uses_actual_representable_displacement_at_large_offset():
    model = SmallTransformer(d_in=1, d_model=2, seed=17)
    x = np.array([[1.0e11]], dtype=np.float64)
    gradient = finite_difference_gradient(model, x, h=1e-5)
    # With one token the model is exactly linear. Recover its coefficient from a
    # unit input independently of the finite-difference displacement.
    exact = model.forward(np.array([[1.0]], dtype=np.float64))
    assert gradient[0, 0] == pytest.approx(exact, rel=1e-10, abs=1e-12)


def test_group_level_ranking_sensitivity_passes_informative_linear_ranking():
    weights, cases = _linear_validation_cases()
    result = ranking_sensitivity_check(
        lambda value: float(np.dot(weights, value)), cases, gate=_gate()
    )
    assert result.passed
    assert result.status == "passed"
    assert result.group_win_binomial_p_value == pytest.approx(1 / 64)
    assert result.winning_groups == 6
    assert result.method_sensitivity > result.random_sensitivity


@pytest.mark.parametrize("kind", ["adversarial"])
def test_group_level_ranking_sensitivity_rejects_null_and_adversarial(kind):
    weights, cases = _linear_validation_cases(kind)
    result = faithfulness_check(
        lambda value: float(np.dot(weights, value)), cases, gate=_gate()
    )
    assert not result.passed
    assert result.status == "failed"


def test_group_level_ranking_sensitivity_abstains_on_exact_magnitude_ties():
    weights, cases = _linear_validation_cases("constant")
    result = faithfulness_check(
        lambda value: float(np.dot(weights, value)), cases, gate=_gate()
    )
    assert not result.passed
    assert result.status == "abstained"
    assert result.reason == "ranking_ties_unresolved:linear-case-0"


def test_partial_tie_counterexample_abstains_instead_of_false_pass():
    cases = [
        AttributionValidationCase(
            case_id=f"tie-case-{index}",
            group_id=f"tie-group-{index}",
            unit_ids=(f"tie-unit-{index}",),
            x=np.ones(4, dtype=np.float64),
            attribution=np.array([0.0, 0.0, 1.0, 0.0]),
            baseline=np.zeros(4, dtype=np.float64),
        )
        for index in range(6)
    ]
    result = ranking_sensitivity_check(
        lambda value: float(np.dot([1.0, 1.0, 2.0, 2.0], value)),
        cases,
        gate=_gate(n_steps=2),
    )
    assert result.status == "abstained"
    assert result.reason == "ranking_ties_unresolved:tie-case-0"
    assert result.group_win_binomial_p_value is None


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
    assert result.group_win_binomial_p_value is None


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
        (
            _gate(alpha=float(np.nextafter(0.0, 1.0))),
            "independent-group resource limit",
        ),
        (_gate(n_steps=1), "n_steps"),
        (_gate(selection_group_ids=()), "selection_group_ids must be nonempty"),
        (_gate(selection_unit_ids=()), "selection_unit_ids must be nonempty"),
        (
            replace(_gate(), frozen_gate_id="arbitrary-label"),
            "content-derived ranking gate identifier",
        ),
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


def test_group_level_gate_abstains_on_stateful_predictor():
    weights, cases = _linear_validation_cases()
    calls = 0

    def stateful(value):
        nonlocal calls
        calls += 1
        return float(np.dot(weights, value)) + calls * 1e-6

    result = ranking_sensitivity_check(stateful, cases, gate=_gate())
    assert result.status == "abstained"
    assert result.reason == "predictor_not_deterministic:linear-case-0"


def test_run_probe_emits_records_with_verdicts():
    model, _ = _model_and_input(seed=4)
    cases = _probe_cases(4)
    records = run_attribution_probe(
        model,
        cases,
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
        assert rec.faithfulness_passed == (
            rec.metadata["gate_status"] == "passed"
            and rec.metadata["confirmatory_role"] == "primary"
        )
        assert len(rec.metadata["validation_input_baseline_set_sha256"]) == 64
        assert len(rec.metadata["validation_relevance_set_sha256"]) == 64
        assert rec.evidence_bundle is not None
        assert (
            rec.evidence_bundle["model"]["parameter_sha256"] == model.parameter_sha256()
        )
        expected_work = attribution_probe._probe_work_estimate(
            model, tuple(cases), _gate(), (rec.method,)
        )
        assert rec.evidence_bundle["work_estimate_multiply_adds"] == expected_work
        assert rec.metadata["probe_work_estimate_multiply_adds"] == str(expected_work)


def test_probe_preflights_complete_composed_work_before_attribution(monkeypatch):
    model, _ = _model_and_input(seed=4)
    monkeypatch.setattr(attribution_probe, "MAX_PROBE_MULTIPLY_ADDS", 1)
    with pytest.raises(ValueError, match="complete attribution probe"):
        run_attribution_probe(model, _probe_cases(4), gate=_gate())


def test_probe_work_estimate_counts_the_full_lrp_reverse_pass():
    model = SmallTransformer(d_in=17, d_model=3, seed=5)
    gate = _gate(n_steps=2, n_random_rankings=400)
    case = ProbeValidationCase(
        case_id="case-wide",
        group_id="group-wide",
        unit_ids=("unit-wide",),
        x=np.ones((2, 17), dtype=np.float64),
        baseline=np.zeros((2, 17), dtype=np.float64),
    )
    forward = model.estimated_forward_multiply_adds(2)
    reverse = (
        2 * model.d_model
        + 4 * 2 * model.d_model * model.d_model
        + 2 * 2 * model.d_model
        + 2 * 2 * model.d_in * model.d_model
    )
    gate_forwards = 3 + (1 + gate.n_random_rankings) * gate.n_steps
    assert (
        attribution_probe._probe_work_estimate(model, (case,), gate, ("lrp_epsilon",))
        == forward + reverse + gate_forwards * forward
    )


def test_only_predeclared_primary_method_can_set_legacy_positive_flag():
    model, _ = _model_and_input(seed=4)
    records = run_attribution_probe(
        model,
        _probe_cases(4),
        gate=_gate(),
        primary_method="grad_x_input",
    )
    for record in records:
        if record.method == "lrp_epsilon":
            assert not record.faithfulness_passed
            assert record.metadata["confirmatory_role"] == "secondary"
        else:
            assert record.metadata["confirmatory_role"] == "primary"


def test_run_probe_rejects_insufficient_groups_before_attribution():
    model, _ = _model_and_input(seed=7)
    with pytest.raises(ValueError, match="independent validation groups"):
        run_attribution_probe(
            model, _probe_cases(7, count=4), gate=_gate(), methods=("lrp_epsilon",)
        )


def test_canonical_hash_matches_sorted_compact_json():
    cfg = {"experiment": "attribution_probe", "model": "small_transformer", "n": "3"}
    expected = canonical_hash(cfg)
    # Recompute the documented serialization independently.
    payload = json.dumps(cfg, sort_keys=True, separators=(",", ":")).encode()
    assert expected == hashlib.sha256(payload).hexdigest()


def test_cli_config_hash_binds_seed_model_gate_and_cases(tmp_path, capsys):
    hashes = []
    model_hashes = []
    for seed in (31, 32):
        directory = tmp_path / f"seed-{seed}"
        runlog = directory / "attr.jsonl"
        artifacts = directory / "artifacts"
        assert (
            attribution_main(
                [
                    "demo",
                    "--runlog",
                    str(runlog),
                    "--artifacts",
                    str(artifacts),
                    "--seed",
                    str(seed),
                    "--validation-cases",
                    "5",
                    "--random-rankings",
                    "128",
                    "--min-groups",
                    "5",
                ]
            )
            == 0
        )
        events = [json.loads(line) for line in runlog.read_text().splitlines()]
        config_event = next(
            event for event in events if event["type"] == "config_logged"
        )
        hashes.append(config_event["config_hash"])
        model_hashes.append(config_event["config"]["model_parameter_sha256"])
        assert config_event["config"]["seed"] == seed
        assert config_event["config"]["gate_manifest"]
        assert config_event["config"]["validation_input_baseline_set_sha256"]
    capsys.readouterr()
    assert len(set(hashes)) == 2
    assert len(set(model_hashes)) == 2


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
        config={"model": "small_transformer", "target_output": "scalar_target"},
        artifact_dir=tmp_path / "artifacts",
    )
    lines = [json.loads(line) for line in out.read_text().splitlines()]
    types = [e["type"] for e in lines]
    assert types[0] == "run_started"
    assert types[1] == "config_logged"
    assert types[-1] == "run_ended"
    assert types.count("attribution_logged") == len(records)
    assert types.count("artifact_logged") == 2 * len(records)
    # run_started and config_logged config_hash agree (validator requirement).
    assert lines[0]["config_hash"] == lines[1]["config_hash"]
    # config_hash equals the canonical hash of the logged config.
    assert lines[1]["config_hash"] == canonical_hash(lines[1]["config"])
    # Each attribution event carries the required fields and a confined, portable
    # NumPy v1.0 little-endian f64 C-order artifact that the converter can load.
    attribution_events = [e for e in lines if e["type"] == "attribution_logged"]
    assert sum(event["faithfulness_check"] for event in attribution_events) <= 1
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

        evidence_uri = event["metadata"]["evidence_bundle_uri"]
        evidence_path = out.parent.joinpath(*PurePosixPath(evidence_uri).parts)
        evidence_bytes = evidence_path.read_bytes()
        assert (
            hashlib.sha256(evidence_bytes).hexdigest()
            == event["metadata"]["evidence_bundle_sha256"]
        )
        evidence = json.loads(evidence_bytes)
        assert evidence["method"] == record.method
        assert evidence["layer"] == "not_declared"
        assert evidence["model"]["parameter_sha256"] == model.parameter_sha256()
        representative = evidence["cases"][0]["relevance"]
        reconstructed = np.frombuffer(
            base64.b64decode(representative["data_base64"]), dtype="<f8"
        ).reshape(representative["shape"])
        assert np.array_equal(reconstructed, record.relevance)

    artifact_events = [event for event in lines if event["type"] == "artifact_logged"]
    for event in artifact_events:
        artifact_path = out.parent.joinpath(*PurePosixPath(event["uri"]).parts)
        assert hashlib.sha256(artifact_path.read_bytes()).hexdigest() == event["sha256"]


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
        faithfulness_passed=False,
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
        faithfulness_passed=False,
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
        faithfulness_passed=False,
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
        faithfulness_passed=False,
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
        faithfulness_passed=False,
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
            faithfulness_passed=False,
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
        faithfulness_passed=False,
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
        (np.array([1.0 + 2.0j]), "real numeric"),
        (np.array([True]), "real numeric"),
    ],
    ids=["empty", "nonfinite", "too-many-values", "complex", "boolean"],
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
        faithfulness_passed=False,
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
        faithfulness_passed=False,
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
            faithfulness_passed=False,
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
            faithfulness_passed=False,
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
            faithfulness_passed=False,
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
        faithfulness_passed=False,
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
        faithfulness_passed=False,
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
        faithfulness_passed=False,
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
        faithfulness_passed=False,
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
        faithfulness_passed=False,
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
            faithfulness_passed=False,
        )
    with pytest.raises(ValueError):
        AttributionRecord(
            method="m",
            target_output="",
            relevance=np.zeros((2, 2)),
            faithfulness_passed=False,
        )


def test_writer_rejects_arbitrary_positive_legacy_flag_without_evidence(tmp_path):
    runlog_path = tmp_path / "attr.jsonl"
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
    )
    with pytest.raises(ValueError, match="positive recorded check"):
        write_attribution_runlog(
            runlog_path, [record], artifact_dir=tmp_path / "artifacts"
        )
    assert not runlog_path.exists()


def test_writer_rejects_minimal_forged_positive_evidence(tmp_path):
    runlog_path = tmp_path / "attr.jsonl"
    record = AttributionRecord(
        method="lrp_epsilon",
        target_output="action_dim_0",
        relevance=np.ones((2, 2)),
        faithfulness_passed=True,
        metadata={
            "diagnostic": "deletion_ranking_sensitivity",
            "gate_status": "passed",
            "gate_reason": "ranking_sensitivity_gate_passed",
            "confirmatory_role": "primary",
            "causal_or_mechanistic_faithfulness_established": "false",
        },
        evidence_bundle={"schema": "prisoma-attribution-evidence-v2"},
    )
    with pytest.raises(ValueError, match="omits required fields"):
        write_attribution_runlog(
            runlog_path, [record], artifact_dir=tmp_path / "artifacts"
        )
    assert not runlog_path.exists()
    assert not (tmp_path / "artifacts").exists()


@pytest.mark.parametrize(
    ("tamper", "message"),
    [
        ("gate", "gate hash"),
        ("model", "model hash"),
        ("case", "case-set hash"),
        ("relevance", "relevance-set hash"),
    ],
)
def test_writer_rejects_tampered_probe_commitments(tmp_path, tamper, message):
    model, _ = _model_and_input(seed=5)
    record = run_attribution_probe(
        model, _probe_cases(5), gate=_gate(), methods=("lrp_epsilon",)
    )[0]
    bundle = copy.deepcopy(record.evidence_bundle)
    assert bundle is not None
    if tamper == "gate":
        bundle["gate"]["seed"] += 1
    else:
        if tamper == "model":
            exact_array = bundle["model"]["parameters"]["w_embed"]
        elif tamper == "case":
            exact_array = bundle["cases"][0]["x"]
        else:
            exact_array = bundle["cases"][0]["relevance"]
        payload = bytearray(base64.b64decode(exact_array["data_base64"]))
        payload[0] ^= 1
        exact_array["data_base64"] = base64.b64encode(payload).decode("ascii")

    with pytest.raises(ValueError, match=message):
        write_attribution_runlog(
            tmp_path / f"{tamper}.jsonl",
            [replace(record, evidence_bundle=bundle)],
            artifact_dir=tmp_path / f"{tamper}-artifacts",
        )
    assert not (tmp_path / f"{tamper}.jsonl").exists()
    assert not (tmp_path / f"{tamper}-artifacts").exists()


def test_writer_recomputes_positive_evidence_before_publication(tmp_path):
    model, _ = _model_and_input(seed=28)
    record = run_attribution_probe(
        model, _probe_cases(28), gate=_gate(), methods=("lrp_epsilon",)
    )[0]
    assert record.faithfulness_passed

    published = write_attribution_runlog(
        tmp_path / "valid.jsonl",
        [record],
        artifact_dir=tmp_path / "valid-artifacts",
    )
    assert published.exists()

    forged_bundle = copy.deepcopy(record.evidence_bundle)
    assert forged_bundle is not None
    forged_bundle["decision"]["winning_groups"] -= 1
    forged_metadata = dict(record.metadata)
    forged_metadata["winning_groups"] = str(forged_bundle["decision"]["winning_groups"])
    with pytest.raises(ValueError, match="does not reproduce"):
        write_attribution_runlog(
            tmp_path / "forged.jsonl",
            [
                replace(
                    record,
                    metadata=forged_metadata,
                    evidence_bundle=forged_bundle,
                )
            ],
            artifact_dir=tmp_path / "forged-artifacts",
        )
    assert not (tmp_path / "forged.jsonl").exists()
    assert not (tmp_path / "forged-artifacts").exists()


def test_writer_recomputes_secondary_positive_evidence_before_publication(tmp_path):
    model, _ = _model_and_input(seed=5)
    records = run_attribution_probe(
        model, _probe_cases(5), gate=_gate(), primary_method="lrp_epsilon"
    )
    primary = next(
        record
        for record in records
        if record.metadata["confirmatory_role"] == "primary"
    )
    secondary = next(
        record
        for record in records
        if record.metadata["confirmatory_role"] == "secondary"
    )
    assert not secondary.faithfulness_passed

    forged_bundle = copy.deepcopy(secondary.evidence_bundle)
    assert forged_bundle is not None
    forged_bundle["decision"].update(
        {
            "status": "passed",
            "reason": "ranking_sensitivity_gate_passed",
            "passed": True,
            "winning_groups": 1,
        }
    )
    forged_metadata = dict(secondary.metadata)
    forged_metadata.update(
        {
            "gate_status": "passed",
            "gate_reason": "ranking_sensitivity_gate_passed",
            "winning_groups": "1",
        }
    )

    with pytest.raises(ValueError, match="does not reproduce"):
        write_attribution_runlog(
            tmp_path / "forged-secondary.jsonl",
            [
                primary,
                replace(
                    secondary,
                    metadata=forged_metadata,
                    evidence_bundle=forged_bundle,
                ),
            ],
            artifact_dir=tmp_path / "forged-secondary-artifacts",
        )
    assert not (tmp_path / "forged-secondary.jsonl").exists()
    assert not (tmp_path / "forged-secondary-artifacts").exists()


def test_writer_rejects_positive_evidence_over_aggregate_work_budget(tmp_path):
    model, _ = _model_and_input(seed=28)
    record = run_attribution_probe(
        model, _probe_cases(28), gate=_gate(), methods=("lrp_epsilon",)
    )[0]
    assert record.faithfulness_passed
    bundle = copy.deepcopy(record.evidence_bundle)
    assert bundle is not None
    bundle["gate"]["n_random_rankings"] = 100_000
    gate_payload = json.dumps(
        bundle["gate"],
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    gate_hash = hashlib.sha256(gate_payload).hexdigest()
    bundle["gate_content_sha256"] = gate_hash
    bundle["frozen_gate_id"] = f"sha256:{gate_hash}"
    metadata = dict(record.metadata)
    metadata["gate_content_sha256"] = gate_hash
    metadata["frozen_gate_id"] = bundle["frozen_gate_id"]

    with pytest.raises(ValueError, match="publication-recomputation resource budget"):
        write_attribution_runlog(
            tmp_path / "over-budget.jsonl",
            [replace(record, evidence_bundle=bundle, metadata=metadata)],
            artifact_dir=tmp_path / "over-budget-artifacts",
        )
    assert not (tmp_path / "over-budget.jsonl").exists()
    assert not (tmp_path / "over-budget-artifacts").exists()


def test_writer_rejects_aggregate_positive_work_before_recomputation(
    tmp_path, monkeypatch
):
    model, _ = _model_and_input(seed=28)
    records = run_attribution_probe(model, _probe_cases(28), gate=_gate())
    assert all(record.metadata["gate_status"] == "passed" for record in records)
    record_work = max(
        int(record.metadata["probe_work_estimate_multiply_adds"]) for record in records
    )
    monkeypatch.setattr(
        attribution_runlog,
        "MAX_EVIDENCE_RECOMPUTE_MULTIPLY_ADDS",
        record_work + 1,
    )
    recomputations = 0
    original_recompute = attribution_runlog._verify_positive_evidence_decision

    def counted_recompute(*args, **kwargs):
        nonlocal recomputations
        recomputations += 1
        return original_recompute(*args, **kwargs)

    monkeypatch.setattr(
        attribution_runlog,
        "_verify_positive_evidence_decision",
        counted_recompute,
    )

    with pytest.raises(ValueError, match="evidence batch exceeds"):
        write_attribution_runlog(
            tmp_path / "aggregate-over-budget.jsonl",
            records,
            artifact_dir=tmp_path / "aggregate-over-budget-artifacts",
        )
    assert recomputations == 0
    assert not (tmp_path / "aggregate-over-budget.jsonl").exists()
    assert not (tmp_path / "aggregate-over-budget-artifacts").exists()


def test_writer_rejects_typed_pass_without_reconstructable_evidence(tmp_path):
    model, _ = _model_and_input(seed=28)
    secondary = next(
        record
        for record in run_attribution_probe(model, _probe_cases(28), gate=_gate())
        if record.metadata["confirmatory_role"] == "secondary"
    )
    assert secondary.metadata["gate_status"] == "passed"
    assert not secondary.faithfulness_passed

    with pytest.raises(ValueError, match="typed passed attribution decision"):
        write_attribution_runlog(
            tmp_path / "passed-without-evidence.jsonl",
            [replace(secondary, evidence_bundle=None)],
            artifact_dir=tmp_path / "passed-without-evidence-artifacts",
        )
    assert not (tmp_path / "passed-without-evidence.jsonl").exists()
    assert not (tmp_path / "passed-without-evidence-artifacts").exists()


def test_writer_enforces_batch_primary_and_method_coherence(tmp_path):
    model, _ = _model_and_input(seed=28)
    records = run_attribution_probe(model, _probe_cases(28), gate=_gate())
    primary = next(
        record
        for record in records
        if record.metadata["confirmatory_role"] == "primary"
    )
    secondary = next(
        record
        for record in records
        if record.metadata["confirmatory_role"] == "secondary"
    )
    assert primary.faithfulness_passed
    assert secondary.metadata["gate_status"] == "passed"

    with pytest.raises(ValueError, match="unique methods"):
        write_attribution_runlog(
            tmp_path / "duplicate-method.jsonl",
            [primary, primary],
            artifact_dir=tmp_path / "duplicate-method-artifacts",
        )

    unbound_record = AttributionRecord(
        method="unbound_extra",
        target_output=primary.target_output,
        relevance=np.ones((2, 2), dtype=np.float64),
        faithfulness_passed=False,
    )
    with pytest.raises(ValueError, match="evidence for every record"):
        write_attribution_runlog(
            tmp_path / "mixed-evidence.jsonl",
            [primary, secondary, unbound_record],
            artifact_dir=tmp_path / "mixed-evidence-artifacts",
        )

    with pytest.raises(ValueError, match="exactly one primary evidence record"):
        write_attribution_runlog(
            tmp_path / "missing-primary.jsonl",
            [secondary],
            artifact_dir=tmp_path / "missing-primary-artifacts",
        )

    invalid_role_metadata = dict(secondary.metadata)
    invalid_role_metadata["confirmatory_role"] = "exploratory"
    with pytest.raises(ValueError, match="confirmatory_role"):
        write_attribution_runlog(
            tmp_path / "invalid-role.jsonl",
            [primary, replace(secondary, metadata=invalid_role_metadata)],
            artifact_dir=tmp_path / "invalid-role-artifacts",
        )

    conflicting_bundle = copy.deepcopy(secondary.evidence_bundle)
    assert conflicting_bundle is not None
    conflicting_bundle["primary_method"] = secondary.method
    conflicting_metadata = dict(secondary.metadata)
    conflicting_metadata["confirmatory_role"] = "primary"
    with pytest.raises(ValueError, match="same predeclared primary method"):
        write_attribution_runlog(
            tmp_path / "conflicting-primary.jsonl",
            [
                primary,
                replace(
                    secondary,
                    faithfulness_passed=True,
                    metadata=conflicting_metadata,
                    evidence_bundle=conflicting_bundle,
                ),
            ],
            artifact_dir=tmp_path / "conflicting-primary-artifacts",
        )

    for name in (
        "duplicate-method",
        "mixed-evidence",
        "missing-primary",
        "invalid-role",
        "conflicting-primary",
    ):
        assert not (tmp_path / f"{name}.jsonl").exists()
        assert not (tmp_path / f"{name}-artifacts").exists()


def test_writer_rejects_forged_work_estimate_commitment(tmp_path):
    model, _ = _model_and_input(seed=28)
    record = run_attribution_probe(
        model, _probe_cases(28), gate=_gate(), methods=("lrp_epsilon",)
    )[0]
    bundle = copy.deepcopy(record.evidence_bundle)
    assert bundle is not None
    bundle["work_estimate_multiply_adds"] += 1
    metadata = dict(record.metadata)
    metadata["probe_work_estimate_multiply_adds"] = str(
        bundle["work_estimate_multiply_adds"]
    )

    with pytest.raises(ValueError, match="recomputed per-record work"):
        write_attribution_runlog(
            tmp_path / "forged-work.jsonl",
            [replace(record, evidence_bundle=bundle, metadata=metadata)],
            artifact_dir=tmp_path / "forged-work-artifacts",
        )
    assert not (tmp_path / "forged-work.jsonl").exists()
    assert not (tmp_path / "forged-work-artifacts").exists()


@pytest.mark.parametrize(
    ("field", "message"),
    [
        ("software", "software provenance"),
        ("modality", "modality"),
    ],
)
def test_writer_binds_software_and_modality_provenance(tmp_path, field, message):
    model, _ = _model_and_input(seed=28)
    record = run_attribution_probe(
        model,
        _probe_cases(28),
        gate=_gate(),
        methods=("lrp_epsilon",),
        modality="vision",
    )[0]
    bundle = copy.deepcopy(record.evidence_bundle)
    assert bundle is not None
    if field == "software":
        bundle["software"]["numpy"] = "forged"
    else:
        bundle["modality"] = "language"

    with pytest.raises(ValueError, match=message):
        write_attribution_runlog(
            tmp_path / f"forged-{field}.jsonl",
            [replace(record, evidence_bundle=bundle)],
            artifact_dir=tmp_path / f"forged-{field}-artifacts",
        )
    assert not (tmp_path / f"forged-{field}.jsonl").exists()
    assert not (tmp_path / f"forged-{field}-artifacts").exists()


@pytest.mark.parametrize(
    ("field", "replacement", "message"),
    [
        ("baseline", "forged-production-baseline", "baseline"),
        ("layer", "forged.production.layer", "layer"),
    ],
)
def test_writer_binds_event_baseline_and_layer_to_evidence(
    tmp_path, field, replacement, message
):
    model, _ = _model_and_input(seed=28)
    record = run_attribution_probe(
        model,
        _probe_cases(28),
        gate=_gate(),
        methods=("lrp_epsilon",),
        layer="reference.input",
    )[0]
    forged = replace(record, **{field: replacement})

    with pytest.raises(ValueError, match=message):
        write_attribution_runlog(
            tmp_path / f"forged-{field}.jsonl",
            [forged],
            artifact_dir=tmp_path / f"forged-{field}-artifacts",
        )
    assert not (tmp_path / f"forged-{field}.jsonl").exists()
    assert not (tmp_path / f"forged-{field}-artifacts").exists()


def test_writer_rejects_cross_run_method_mixing(tmp_path):
    first_model, _ = _model_and_input(seed=28)
    second_model, _ = _model_and_input(seed=29)
    first_records = run_attribution_probe(first_model, _probe_cases(28), gate=_gate())
    second_records = run_attribution_probe(second_model, _probe_cases(29), gate=_gate())
    primary = next(
        record
        for record in first_records
        if record.metadata["confirmatory_role"] == "primary"
    )
    unrelated_secondary = next(
        record
        for record in second_records
        if record.metadata["confirmatory_role"] == "secondary"
    )

    with pytest.raises(ValueError, match="share one target, modality, layer"):
        write_attribution_runlog(
            tmp_path / "cross-run.jsonl",
            [primary, unrelated_secondary],
            artifact_dir=tmp_path / "cross-run-artifacts",
        )
    assert not (tmp_path / "cross-run.jsonl").exists()
    assert not (tmp_path / "cross-run-artifacts").exists()


@pytest.mark.parametrize(
    "metadata_field",
    PRODUCER_EVIDENCE_METADATA_FIELDS,
)
def test_writer_binds_every_producer_evidence_metadata_field(
    tmp_path, passing_probe_batch, metadata_field
):
    primary = next(
        record
        for record in passing_probe_batch
        if record.metadata["confirmatory_role"] == "primary"
    )
    assert set(primary.metadata) == set(PRODUCER_EVIDENCE_METADATA_FIELDS)
    forged_metadata = dict(primary.metadata)
    forged_metadata[metadata_field] = f"forged::{metadata_field}"

    with pytest.raises(ValueError):
        write_attribution_runlog(
            tmp_path / f"forged-metadata-{metadata_field}.jsonl",
            [replace(primary, metadata=forged_metadata)],
            artifact_dir=tmp_path / f"forged-metadata-{metadata_field}-artifacts",
        )
    assert not (tmp_path / f"forged-metadata-{metadata_field}.jsonl").exists()
    assert not (tmp_path / f"forged-metadata-{metadata_field}-artifacts").exists()


def test_writer_accepts_fully_coherent_evidence_config(tmp_path, passing_probe_batch):
    path = write_attribution_runlog(
        tmp_path / "coherent-config.jsonl",
        passing_probe_batch,
        config=_coherent_evidence_config(passing_probe_batch),
        artifact_dir=tmp_path / "coherent-config-artifacts",
    )
    assert path.exists()


@pytest.mark.parametrize("config_field", EVIDENCE_CONFIG_BINDING_FIELDS)
def test_writer_rejects_every_contradictory_evidence_config_field(
    tmp_path, passing_probe_batch, config_field
):
    config = copy.deepcopy(_coherent_evidence_config(passing_probe_batch))
    if config_field == "methods":
        config[config_field] = config[config_field][:-1]
    elif config_field == "method_implementations":
        config[config_field][config["methods"][0]] = "forged"
    elif config_field == "gate_manifest":
        config[config_field]["seed"] += 1
    elif isinstance(config[config_field], int):
        config[config_field] += 1
    else:
        config[config_field] = f"forged::{config_field}"

    with pytest.raises(ValueError, match="config"):
        write_attribution_runlog(
            tmp_path / f"forged-config-{config_field}.jsonl",
            passing_probe_batch,
            config=config,
            artifact_dir=tmp_path / f"forged-config-{config_field}-artifacts",
        )
    assert not (tmp_path / f"forged-config-{config_field}.jsonl").exists()
    assert not (tmp_path / f"forged-config-{config_field}-artifacts").exists()


def test_writer_rejects_promoting_a_reproducible_failed_decision(tmp_path):
    model, _ = _model_and_input(seed=5)
    record = run_attribution_probe(
        model, _probe_cases(5), gate=_gate(), methods=("lrp_epsilon",)
    )[0]
    assert not record.faithfulness_passed
    forged_bundle = copy.deepcopy(record.evidence_bundle)
    assert forged_bundle is not None
    forged_bundle["decision"].update(
        {
            "status": "passed",
            "reason": "ranking_sensitivity_gate_passed",
            "passed": True,
        }
    )
    forged_metadata = dict(record.metadata)
    forged_metadata.update(
        {
            "gate_status": "passed",
            "gate_reason": "ranking_sensitivity_gate_passed",
        }
    )
    forged_record = replace(
        record,
        faithfulness_passed=True,
        metadata=forged_metadata,
        evidence_bundle=forged_bundle,
    )

    with pytest.raises(ValueError, match="does not reproduce"):
        write_attribution_runlog(
            tmp_path / "forged.jsonl",
            [forged_record],
            artifact_dir=tmp_path / "forged-artifacts",
        )
    assert not (tmp_path / "forged.jsonl").exists()
    assert not (tmp_path / "forged-artifacts").exists()


def test_probe_evidence_requires_confined_artifact_publication(tmp_path):
    model, _ = _model_and_input(seed=12)
    records = run_attribution_probe(
        model, _probe_cases(12), gate=_gate(), methods=("lrp_epsilon",)
    )
    with pytest.raises(ValueError, match="require a confined artifact_dir"):
        write_attribution_runlog(tmp_path / "attr.jsonl", records)


def test_score_hash_commits_to_exact_f64_bytes(tmp_path):
    records = [
        AttributionRecord(
            method=f"method-{index}",
            target_output="target",
            relevance=np.array([value], dtype=np.float64),
            faithfulness_passed=False,
        )
        for index, value in enumerate((0.0, 1.0e-10, -0.0))
    ]
    path = write_attribution_runlog(tmp_path / "attr.jsonl", records)
    hashes = [
        event["score_hash"]
        for event in map(json.loads, path.read_text().splitlines())
        if event["type"] == "attribution_logged"
    ]
    assert len(set(hashes)) == 3


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
