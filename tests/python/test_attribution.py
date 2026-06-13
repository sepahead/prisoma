"""Tests for the faithfulness-checked attribution probe (numpy; torch optional)."""

from __future__ import annotations

import json

import numpy as np
import pytest

from experiments.attribution import (
    AttributionRecord,
    SmallTransformer,
    canonical_hash,
    faithfulness_check,
    finite_difference_gradient,
    grad_times_input,
    lrp_epsilon,
    run_attribution_probe,
    write_attribution_runlog,
)


def _model_and_input(seed: int = 0, tokens: int = 6, d_in: int = 5, d_model: int = 8):
    model = SmallTransformer(d_in=d_in, d_model=d_model, seed=seed)
    rng = np.random.default_rng(seed + 100)
    x = rng.standard_normal((tokens, d_in))
    return model, x


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


def test_faithfulness_passes_real_and_rejects_uninformative():
    model, x = _model_and_input(seed=3)
    real = lrp_epsilon(model, x)
    real_result = faithfulness_check(model.forward, x, real, n_steps=8, n_random=16)
    assert real_result.passed
    assert real_result.method_aopc > real_result.random_aopc

    # A constant "attribution" carries no ranking information; its deletion order is
    # arbitrary, so it must not beat the random control.
    constant = np.ones_like(x)
    bad_result = faithfulness_check(model.forward, x, constant, n_steps=8, n_random=16)
    assert not bad_result.passed


def test_run_probe_emits_records_with_verdicts():
    model, x = _model_and_input(seed=4)
    records = run_attribution_probe(model, x, target_output="action_dim_0", modality="vision")
    assert {r.method for r in records} == {"lrp_epsilon", "grad_x_input"}
    for rec in records:
        assert rec.target_output == "action_dim_0"
        assert rec.modality == "vision"
        assert "method_aopc" in rec.metadata


def test_canonical_hash_matches_sorted_compact_json():
    cfg = {"experiment": "attribution_probe", "model": "small_transformer", "n": "3"}
    expected = canonical_hash(cfg)
    # Recompute the documented serialization independently.
    import hashlib

    payload = json.dumps(cfg, sort_keys=True, separators=(",", ":")).encode()
    assert expected == hashlib.sha256(payload).hexdigest()


def test_write_runlog_is_schema_shaped(tmp_path):
    model, x = _model_and_input(seed=5)
    records = run_attribution_probe(model, x)
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
    # Each attribution event carries the required non-empty fields + faithfulness.
    for event in lines:
        if event["type"] == "attribution_logged":
            assert event["method"]
            assert event["target_output"]
            assert isinstance(event["faithfulness_check"], bool)
            assert event["score_hash"]
            assert "artifact_sha256" in event["metadata"]


def test_attribution_record_rejects_empty_fields():
    with pytest.raises(ValueError):
        AttributionRecord(method="", target_output="t", relevance=np.zeros((2, 2)), faithfulness_passed=True)
    with pytest.raises(ValueError):
        AttributionRecord(method="m", target_output="", relevance=np.zeros((2, 2)), faithfulness_passed=True)


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
