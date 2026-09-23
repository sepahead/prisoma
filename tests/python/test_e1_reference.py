"""Synthetic E1 arithmetic and content controls; no simulator or causal evidence."""

from __future__ import annotations

import ast
from dataclasses import FrozenInstanceError, replace
from decimal import Decimal, localcontext
import hashlib
import json
from pathlib import Path
import struct
import sys

import numpy as np
import pytest

from experiments import e1_reference as e1


FIXTURES = Path(__file__).parents[1] / "fixtures" / "e1"


def digest(value):
    return hashlib.sha256(str(value).encode()).hexdigest()


def rejection(code):
    return pytest.raises(e1.ReferenceError, match=f"^{code}:")


def observation(index):
    x = (index % 17) / 16
    return e1.Landmark(
        (x, 0.25, 0.5, 1 - x, x / 2, 0.0, x - 0.5, 2 + x),
        digest(("synthetic-input", index)),
    )


def episode(split, index, targets=None):
    # This known relation tests numerical fitting only. It is not a world model.
    obs = observation(index)
    if targets is None:
        x = obs.values[0]
        targets = tuple(
            8 + 2 * x + 30 * roll + 60 * pitch + 10 * x * pitch
            for roll, pitch in e1.ANGLES
        )
    return e1.Episode(
        f"e1-{split}-{index:03d}",
        obs,
        targets,
        digest(("synthetic-labels", split, index, targets)),
    )


@pytest.fixture(scope="module")
def training():
    return tuple(episode("train", index) for index in range(64))


@pytest.fixture(scope="module")
def grid(training):
    return e1.fit_reference(
        training,
        source_sha256=digest("synthetic-source"),
        runtime_inventory_sha256=digest("synthetic-runtime"),
    )


def development_rows(grid):
    result = []
    for index in range(16):
        row = episode("development", index)
        commitment = e1.commit_collection(
            "development", row.episode_id, row.observation, grid
        )
        result.append(e1.evaluate_episode(commitment, row))
    return tuple(result)


@pytest.fixture(scope="module")
def selection(grid):
    return e1.choose_lambda(grid, development_rows(grid))


def heldout_rows(selection, targets=None):
    result = []
    for index in range(32):
        row = episode("heldout", index, targets)
        result.append(
            e1.evaluate_episode(
                e1.commit_selected(selection, row.episode_id, row.observation), row
            )
        )
    return tuple(result)


def blocks(first_tick=10, samples=(1.0,) * 400):
    payload = struct.pack("<400d", *samples)
    rows, offset = [], 0
    for tick in range(first_tick, first_tick + 3):
        start, end = (tick - 1) * 16000 // 120, tick * 16000 // 120
        count = end - start
        rows.append(
            e1.PressureBlock(
                "microphone",
                tick,
                tick,
                start,
                end,
                payload[offset : offset + count * 8],
                (count,),
            )
        )
        offset += count * 8
    return tuple(rows)


def frames():
    return (
        e1.RGBFrame("A", 12, 12, bytes((0, 127, 255, 0)) * 19200),
        e1.RGBFrame("B", 12, 12, bytes((255, 0, 127, 255)) * 19200),
    )


def test_prospective_roster_and_study_are_exact():
    roster = json.loads((FIXTURES / "seed-roster.v1.json").read_bytes())
    e1.validate_seed_roster(roster)
    assert e1.study_contract() == json.loads((FIXTURES / "study.v1.json").read_bytes())
    rows = roster["episodes"]
    assert [sum(row["split"] == split for row in rows) for split, _ in e1.SPLITS] == [
        8,
        64,
        16,
        32,
    ]
    seeds = [seed for row in rows for seed in row["seeds"].values()]
    assert len(seeds) == len(set(seeds)) == 360 and 104729 not in seeds
    roster["episodes"][-1]["seeds"]["runtime"] = seeds[0]
    with rejection("seed_roster_identity"):
        e1.validate_seed_roster(roster)


def test_native_lcg_draws_fixed_y_and_rejects_wrong_seeds():
    # Independently published first three LCG words from seed zero.
    assert e1.project_position(0) == (
        1013904223 / 2**32 - 0.5,
        8.0,
        3519870697 / 2**32 - 0.5,
    )
    assert e1.project_position(0)[2] != 1196435762 / 2**32 - 0.5
    for wrong in (-1, 2**32, True, 1.0, None):
        with rejection("seed_type_range"):
            e1.project_position(wrong)


@pytest.mark.parametrize(
    "name,values,mean_bits,rms_bits",
    [
        ("zero", (0.0,) * 400, "0000000000000000", "0000000000000000"),
        ("negative_zero", (-0.0,) * 400, "0000000000000000", "0000000000000000"),
        (
            "maximum_positive",
            (sys.float_info.max,) * 400,
            "ffffffffffffef7f",
            "ffffffffffffef7f",
        ),
        (
            "maximum_negative",
            (-sys.float_info.max,) * 400,
            "ffffffffffffefff",
            "ffffffffffffef7f",
        ),
        (
            "maximum_cancellation",
            (sys.float_info.max, -sys.float_info.max) * 200,
            "0000000000000000",
            "ffffffffffffef7f",
        ),
        (
            "mixed_scale",
            (sys.float_info.max, -sys.float_info.max, 1.0, -1.0) * 100,
            "0000000000000000",
            "cc3b7f669ea0e67f",
        ),
        (
            "minimum_subnormal",
            (float.fromhex("0x0.0000000000001p-1022"),) * 400,
            "0100000000000000",
            "0100000000000000",
        ),
        (
            "ordinary",
            tuple(((i % 17) - 8) / 100 for i in range(400)),
            "92cb7f48bf7d4dbf",
            "c4d948072d11a93f",
        ),
    ],
)
def test_frozen_ordered_pressure_vectors(name, values, mean_bits, rms_bits):
    # Frozen before implementation in design-vector SHA 8f65efff…45a8a57.
    assert struct.pack("<d", e1.pressure_mean(values)).hex() == mean_bits, name
    assert struct.pack("<d", e1.pressure_rms(values)).hex() == rms_bits, name


def test_pressure_reference_and_invalid_inputs():
    values = tuple(((i % 17) - 8) / 100 for i in range(400))
    with localcontext() as context:
        context.prec = 80
        exact = [Decimal.from_float(x) for x in values]
        mean = float(sum(exact) / 400)
        rms = float((sum(x * x for x in exact) / 400).sqrt())
    assert e1.pressure_mean(values) == pytest.approx(mean, rel=2e-15)
    assert e1.pressure_rms(values) == pytest.approx(rms, rel=2e-15)
    for bad in (None, (), values[:-1], values + (0.0,)):
        with rejection("sample_count"):
            e1.pressure_rms(bad)
    for bad in (float("nan"), float("inf"), 1, True):
        with rejection("pressure_numeric"):
            e1.pressure_mean((bad,) * 400)
    with rejection("nonfinite"):
        e1._finite(10**400)


def test_pressure_source_windows_metadata_and_original_hash():
    for first in (10, 34):
        admitted = blocks(first, (-0.0,) * 400)
        values, raw_sha = e1.pressure_window(
            admitted, first_tick=first, source_id="microphone"
        )
        assert (
            len(values) == 400
            and raw_sha == hashlib.sha256(struct.pack("<400d", *values)).hexdigest()
        )
        assert raw_sha != hashlib.sha256(bytes(3200)).hexdigest()
        for change, code in (
            ({"available_tick": first + 1}, "pressure_window"),
            ({"source_id": "foreign"}, "pressure_window"),
            ({"source_tick": float(first)}, "pressure_window"),
            ({"sample_end": admitted[0].sample_end + 1}, "pressure_window"),
            ({"dtype": "f64be"}, "pressure_encoding"),
            ({"unit": "normalized"}, "pressure_encoding"),
            ({"shape": (132,)}, "pressure_encoding"),
            ({"sample_rate_hz": 16000.0}, "pressure_encoding"),
            ({"payload": bytes(8)}, "pressure_encoding"),
        ):
            with rejection(code):
                e1.pressure_window(
                    (replace(admitted[0], **change), *admitted[1:]),
                    first_tick=first,
                    source_id="microphone",
                )


def test_encoded_rgb_features_and_negative_metadata():
    admitted = frames()
    obs = e1.landmark_features(
        admitted, blocks(), camera_ids=("A", "B"), microphone_id="microphone"
    )
    assert obs.values == (0.0, 127 / 255, 1.0, 1.0, 0.0, 127 / 255, 1.0, 1.0)
    changed_alpha = (
        replace(admitted[0], payload=bytes((0, 127, 255, 255)) * 19200),
        admitted[1],
    )
    alpha_obs = e1.landmark_features(
        changed_alpha, blocks(), camera_ids=("A", "B"), microphone_id="microphone"
    )
    assert alpha_obs.values == obs.values and alpha_obs.input_sha256 != obs.input_sha256
    for changes, code in (
        ({"encoding": "linear-rgba8"}, "rgb_contract"),
        ({"row_origin": "top-left"}, "rgb_contract"),
        ({"dtype": "f32"}, "rgb_contract"),
        ({"layout": "planar"}, "rgb_contract"),
        ({"shape": (120, 160, 3)}, "rgb_contract"),
        ({"available_tick": 13}, "landmark_availability"),
        ({"source_tick": True}, "landmark_availability"),
    ):
        with rejection(code):
            e1.landmark_features(
                (replace(admitted[0], **changes), admitted[1]),
                blocks(),
                camera_ids=("A", "B"),
                microphone_id="microphone",
            )


def test_exact_feature_order_units_and_no_future_inputs():
    obs = observation(3)
    actual = e1.candidate_features(obs, "positive_roll")
    assert len(actual) == len(e1.FEATURES) == 26
    assert actual[:10] == obs.values + (0.03, 0.0)
    assert actual[10:] == tuple(v * a for v in obs.values for a in (0.03, 0.0))
    assert len(e1.study_contract()["feature_units"]) == 26
    with rejection("feature_roster"):
        e1.candidate_features(obs, "raw_pusht_action")
    with rejection("feature_numeric"):
        replace(obs, values=obs.values[:7] + (float("inf"),))
    tree = ast.parse(Path(e1.__file__).read_text())
    imports = {
        node.module.split(".")[0]
        for node in ast.walk(tree)
        if isinstance(node, ast.ImportFrom)
    }
    imports |= {
        alias.name.split(".")[0]
        for node in ast.walk(tree)
        if isinstance(node, ast.Import)
        for alias in node.names
    }
    assert imports <= {
        "__future__",
        "dataclasses",
        "hashlib",
        "json",
        "math",
        "re",
        "struct",
        "sys",
        "typing",
        "numpy",
    }


def test_fit_train_only_scaling_and_grid(training, grid):
    assert tuple(row.penalty for row in grid) == e1.LAMBDAS
    assert len(set(row.training_sha256 for row in grid)) == 1
    assert len(set(row.transforms for row in grid)) == 1
    assert grid[0].training_mean_pa == pytest.approx(
        np.mean([row.targets_pa for row in training])
    )
    assert any(t.constant is not None for t in grid[0].transforms)
    original = tuple(row.sha256 for row in grid)
    e1.forecast(grid[0], replace(observation(3), values=(1.0,) * 8))
    assert original == tuple(row.sha256 for row in grid)
    for wrong in (
        training[:-1],
        training[::-1],
        tuple(episode("heldout", i) for i in range(64)),
        (None,) * 64,
    ):
        with rejection("split_roster"):
            e1.fit_reference(
                wrong,
                source_sha256=digest("source"),
                runtime_inventory_sha256=digest("runtime"),
            )
    with rejection("lambda_grid"):
        e1.commit_collection(
            "development", "e1-development-000", observation(0), grid[:3]
        )
    with rejection("training_only_transform"):
        e1.commit_collection(
            "development",
            "e1-development-000",
            observation(0),
            (replace(grid[0], training_sha256=digest("other")), *grid[1:]),
        )


def test_unpenalized_intercept_sse_and_all_dropped_columns():
    # X'X=diag(4,4), X'y=(12,8), so intercept=3 and slope=8/(4+lambda).
    design = np.array(
        [[1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [1.0, 1.0]], dtype=np.float64
    )
    labels = np.array([1.0, 1.0, 5.0, 5.0], dtype=np.float64)
    fitted = e1._ridge(design, labels, 1.0, 5.0)
    assert np.array(fitted) * 5 == pytest.approx([3.0, 1.6], abs=2e-14)
    assert not np.allclose(np.array(fitted) * 5, [12 / 5, 1.0])
    normalized, decisions = e1._fit_transform(np.full((320, 26), 7.0, dtype=np.float64))
    assert normalized.shape == (320, 0) and all(
        row.constant == 7.0 for row in decisions
    )
    assert e1._ridge(
        np.ones((320, 1)), np.full(320, 7.0), 1000.0, 7.0
    ) == pytest.approx((1.0,))


def test_small_variance_retained_and_collapsed_scale_rejects(monkeypatch):
    matrix = np.zeros((320, 26), dtype=np.float64)
    matrix[::2, 0] = np.nextafter(0.0, 1.0)
    result, decisions = e1._fit_transform(matrix)
    assert result.shape == (320, 1) and decisions[0].constant is None
    original = e1._moment
    monkeypatch.setattr(
        e1,
        "_moment",
        lambda values, *, rms: 0.0 if rms else original(values, rms=False),
    )
    with rejection("scale_rule"):
        e1._fit_transform(matrix)


def test_solver_runtime_rank_finiteness_and_residual_failures(training, monkeypatch):
    keywords = {
        "source_sha256": digest("source"),
        "runtime_inventory_sha256": digest("runtime"),
    }
    monkeypatch.setattr(np, "__version__", "future-unqualified")
    with rejection("numerical_runtime"):
        e1.fit_reference(training, **keywords)
    monkeypatch.setattr(np, "__version__", "2.4.6")
    design, labels = np.ones((320, 1)), np.ones(320)
    for coefficients, rank in (
        (np.array([0.0]), 1),
        (np.array([1.0]), 0),
        (np.array([np.nan]), 1),
    ):
        monkeypatch.setattr(
            np.linalg, "lstsq", lambda *a, **k: (coefficients, None, rank, np.ones(1))
        )
        with rejection("solver_contract"):
            e1._ridge(design, labels, 1.0, 1.0)


def test_reference_artifact_exact_identity_and_closed_loader(grid):
    reference = grid[0]
    payload = e1.canonical(reference.record())
    assert e1.load_reference(payload, expected_sha256=reference.sha256) == reference
    with pytest.raises(FrozenInstanceError):
        reference.penalty = 1.0
    for field, value in (
        ("source_sha256", digest("other")),
        ("training_sha256", digest("other")),
        ("target_scale", reference.target_scale * 2),
    ):
        changed = reference.record()
        changed[field] = value
        with rejection("inference_identity"):
            e1.load_reference(e1.canonical(changed), expected_sha256=reference.sha256)
    for field, value in (
        ("numerical_library", {"dtype": "float32"}),
        ("feature_order", list(reversed(e1.FEATURES))),
        ("contract_sha256", digest("other")),
    ):
        changed = reference.record()
        changed[field] = value
        payload = e1.canonical(changed)
        with rejection("reference_record"):
            e1.load_reference(
                payload, expected_sha256=hashlib.sha256(payload).hexdigest()
            )
    duplicate = b'{"schema":1,"schema":2}'
    with rejection("json_value"):
        e1.load_reference(
            duplicate, expected_sha256=hashlib.sha256(duplicate).hexdigest()
        )


def test_prediction_clip_and_fixed_tie_order(grid):
    negative = replace(
        grid[0], coefficients=(-1.0,) + (0.0,) * (len(grid[0].coefficients) - 1)
    )
    assert e1.forecast(negative, observation(0)) == (0.0,) * 5
    assert e1.ForecastVector("synthetic", (0.0,) * 5).recommendation == "neutral"
    assert (
        e1.ForecastVector("synthetic", (2.0, 1.0, 1.0, 3.0, 4.0)).recommendation
        == "positive_pitch"
    )
    with rejection("forecast_numeric"):
        e1.ForecastVector("synthetic", (float("nan"),) * 5)
    with rejection("forecast_selection"):
        e1.ForecastVector("synthetic", (0.0,) * 4)


def test_stage_commitments_and_round_trip(grid, selection):
    train = e1.commit_collection("train", "e1-train-000", observation(0))
    assert (
        train.selected_action == "neutral"
        and train.forecasts[0].predictor_id == "persistence"
    )
    with rejection("training_order"):
        e1.commit_collection("train", "e1-train-000", observation(0), grid)
    dev = e1.commit_collection(
        "development", "e1-development-000", observation(0), grid
    )
    assert tuple(row.predictor_id for row in dev.forecasts[:4]) == tuple(
        row.sha256 for row in grid
    )
    assert dev.collection_policy == dev.selected_action == "neutral"
    with rejection("development_order"):
        replace(dev, collection_policy=grid[0].sha256)
    held = e1.commit_selected(selection, "e1-heldout-000", observation(0))
    assert held.selected_action == held.forecasts[0].recommendation
    with rejection("holdout_lock"):
        replace(held, selected_action="positive_roll")
    for row in (train, dev, held):
        assert (
            e1.load_commitment(e1.canonical(row.record()), expected_sha256=row.sha256)
            == row
        )
    assert (
        e1.load_selection(
            e1.canonical(selection.record()),
            expected_sha256=selection.sha256,
            references=grid,
        )
        == selection
    )
    with rejection("development_selection"):
        e1.load_selection(
            e1.canonical(selection.record()),
            expected_sha256=selection.sha256,
            references=tuple(
                replace(row, source_sha256=digest("other")) for row in grid
            ),
        )


def test_complete_episode_score_and_development_reconstruction(grid):
    rows = development_rows(grid)
    row = rows[0]
    assert row.mae_pa[0] == pytest.approx(
        sum(
            abs(a - b)
            for a, b in zip(
                row.commitment.forecasts[0].values_pa, row.episode.targets_pa
            )
        )
        / 5
    )
    assert e1.choose_lambda(grid, rows).reference.penalty in e1.LAMBDAS
    with rejection("episode_join"):
        e1.evaluate_episode(
            row.commitment, replace(row.episode, observation=observation(44))
        )
    with rejection("branch_roster"):
        replace(row.episode, targets_pa=row.episode.targets_pa[:-1])
    with rejection("target_function"):
        replace(row.episode, target_function_sha256=digest("different-target"))
    with rejection("development_selection"):
        e1.choose_lambda(grid, (replace(row, mae_pa=(123.0,) * 6), *rows[1:]))
    with rejection("split_roster"):
        e1.choose_lambda(grid, rows[:-1])


def test_loader_rejects_rehashed_action_varying_baselines(grid, selection):
    commitments = (
        e1.commit_collection("train", "e1-train-000", observation(0)),
        e1.commit_collection("development", "e1-development-000", observation(0), grid),
        e1.commit_selected(selection, "e1-heldout-000", observation(0)),
    )
    for commitment in commitments:
        assert (
            e1.load_commitment(
                e1.canonical(commitment.record()), expected_sha256=commitment.sha256
            )
            == commitment
        )
        for index, row in enumerate(commitment.forecasts):
            if row.predictor_id not in ("constant", "persistence"):
                continue
            changed = json.loads(e1.canonical(commitment.record()))
            changed["forecasts"][index]["values_pa"][1] += 1.0
            raw = e1.canonical(changed)
            with rejection("baseline_shape"):
                e1.load_commitment(raw, expected_sha256=hashlib.sha256(raw).hexdigest())


def test_exact_development_tie_prefers_larger_lambda(grid):
    # Controlled immutable candidates with identical coefficients, then one ulp-sized difference.
    tied = tuple(
        replace(row, coefficients=(1.0,) + (0.0,) * (len(row.coefficients) - 1))
        for row in grid
    )
    assert e1.choose_lambda(tied, development_rows(tied)).reference.penalty == 1000.0
    changed = (
        replace(tied[0], coefficients=(0.999999999999,) + tied[0].coefficients[1:]),
        *tied[1:],
    )
    # Predictions exceed every synthetic target, so this strictly smaller intercept wins.
    assert (
        e1.choose_lambda(changed, development_rows(changed)).reference.penalty == 1e-6
    )


def test_paired_episode_report_and_fixed_population(selection):
    rows = heldout_rows(selection)
    result = e1.paired_episode_report(selection, rows)
    assert result["episodes"] == 32 and result["action_rows"] == 160
    assert result["bootstrap_indices_sha256"] == e1.BOOTSTRAP_INDICES_SHA256
    assert set(result["comparisons"]) == {"constant", "persistence"}
    assert (
        result["conclusion"] == "benefit_established"
    )  # Synthetic known relation only.
    assert all(value is False for value in result["authority"].values())
    assert result["policy_comparison"].startswith("requires_separate_qualified")
    with rejection("split_roster"):
        e1.paired_episode_report(selection, rows[:-1])
    foreign = replace(selection, development_sha256=digest("post-holdout-change"))
    with rejection("holdout_lock"):
        e1.paired_episode_report(foreign, rows)


def test_grouped_bootstrap_null_sign_and_uninformative_outcomes(grid):
    indices = np.random.Generator(np.random.PCG64(104729)).integers(
        0, 32, size=(10000, 32), dtype=np.uint32
    )
    assert (
        hashlib.sha256(indices.astype("<u4").tobytes()).hexdigest()
        == e1.BOOTSTRAP_INDICES_SHA256
    )
    assert e1._paired_interval((0.0,) * 32, indices)["interval_95_pa"] == [0.0, 0.0]
    assert e1._paired_interval((2.0,) * 32, indices)["interval_95_pa"] == [2.0, 2.0]
    for mean, expected_reason in (
        (0.0, "zero_or_unrepresentable_useful_margin"),
        (
            float.fromhex("0x0.0000000000001p-1022"),
            "zero_or_unrepresentable_useful_margin",
        ),
        (1.0, "no_observed_action_relevant_variation"),
    ):
        altered = tuple(replace(row, training_mean_pa=mean) for row in grid)
        selected = e1.choose_lambda(altered, development_rows(altered))
        result = e1.paired_episode_report(selected, heldout_rows(selected, (1.0,) * 5))
        assert result["conclusion"] == "uninformative"
        assert expected_reason in result["uninformative_reasons"]


def test_null_result_keeps_variation_and_both_unfavorable_comparators(selection):
    rows = heldout_rows(selection, (2.0, 2.01, 2.02, 2.03, 2.04))
    result = e1.paired_episode_report(selection, rows)
    assert result["conclusion"] == "null_or_inconclusive"
    assert result["uninformative_reasons"] == []
    assert result["comparisons"]["persistence"]["interval_95_pa"][1] < 0
    assert result["useful_margin_pa"] == 0.05 * selection.reference.training_mean_pa


def test_zero_training_is_valid_numerics_and_not_a_benefit():
    grid = e1.fit_reference(
        tuple(episode("train", i, (0.0,) * 5) for i in range(64)),
        source_sha256=digest("synthetic-zero"),
        runtime_inventory_sha256=digest("synthetic-runtime"),
    )
    assert all(row.target_scale == 1.0 and row.training_mean_pa == 0.0 for row in grid)
    selected = e1.choose_lambda(grid, development_rows(grid))
    assert selected.reference.penalty == 1000.0
    result = e1.paired_episode_report(selected, heldout_rows(selected, (0.0,) * 5))
    assert result["conclusion"] == "uninformative"
    assert result["comparisons"]["constant"]["interval_95_pa"] == [0.0, 0.0]


def test_failed_or_changed_population_and_tampered_scores(selection):
    rows = heldout_rows(selection)
    with rejection("split_roster"):
        e1.paired_episode_report(selection, rows[:-1] + (rows[0],))
    with rejection("holdout_lock"):
        e1.paired_episode_report(
            selection, (replace(rows[0], mae_pa=(0.0,) * 3), *rows[1:])
        )
