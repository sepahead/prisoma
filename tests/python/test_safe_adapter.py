"""Tests for the SAFE -> (V,L,D,A) adapter (numpy only; no compiled extension)."""

from __future__ import annotations

import numpy as np
import pytest

from experiments.safe_adapter import (
    MappingConfig,
    VariableSpec,
    VldaDataset,
    VldaSample,
    layerwise_physics_probe,
    load_safe_rollout_dir,
    rollouts_to_dataset,
    text_hash_features,
    verify_contract_file,
    write_synthetic_safe_dir,
)
from experiments.safe_adapter.convert import rollout_to_samples
from experiments.safe_adapter.verify import verify_contract_obj


def _sample(sid: str, dim: int = 3, success: bool = True, split: str = "train") -> VldaSample:
    return VldaSample(
        sample_id=sid,
        v=[1.0] * dim,
        l=[2.0] * dim,
        d=[3.0] * dim,
        a=[4.0] * dim,
        success=success,
        episode_id=sid.split("--t")[0],
        metadata={"split": split},
    )


def test_contract_rejects_ragged_and_nonfinite():
    with pytest.raises(ValueError):
        VldaSample(sample_id="x", v=[float("nan")], l=[1.0], d=[1.0], a=[1.0], success=True)
    # Ragged across samples is caught by dataset.validate().
    s0 = _sample("e0--t0", dim=3)
    bad = VldaSample(
        sample_id="e0--t1", v=[1.0, 2.0], l=[2.0] * 3, d=[3.0] * 3, a=[4.0] * 3, success=False
    )
    ds = VldaDataset(samples=[s0, bad])
    issues = ds.validate()
    assert any("v has length" in issue for issue in issues)


def test_contract_round_trips_and_writes(tmp_path):
    # >= 8 samples: verify.py mirrors the Rust harness's hard minimum.
    ds = VldaDataset(
        samples=[
            _sample(f"e{i}--t0", success=(i % 2 == 0), split="train" if i < 6 else "test")
            for i in range(8)
        ],
        run_id="r",
        source="vla-safe/SAFE",
    )
    assert ds.validate() == []
    out = ds.write_json(tmp_path / "ds.json")
    report = verify_contract_file(out)
    assert report.ok
    assert report.dims == {"v": 3, "l": 3, "d": 3, "a": 3}


def test_text_hash_features_deterministic_and_normalized():
    a = text_hash_features("pick up the red block", 16)
    b = text_hash_features("pick up the red block", 16)
    assert np.allclose(a, b)
    assert a.shape == (16,)
    assert abs(np.linalg.norm(a) - 1.0) < 1e-9
    # Different text -> different features (overwhelmingly likely).
    c = text_hash_features("close the drawer", 16)
    assert not np.allclose(a, c)


def test_synth_load_convert_end_to_end(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "safe", n_tasks=4, episodes_per_task=4, n_steps=10, seed=1
    )
    rollouts = load_safe_rollout_dir(safe_dir, seen_task_ids={0, 1})
    assert len(rollouts) == 16
    # Raw per-token hidden states with token groups, so token slicing works.
    assert rollouts[0].hidden_states.ndim == 3
    assert rollouts[0].token_groups is not None

    dataset = rollouts_to_dataset(rollouts, MappingConfig())
    assert dataset.validate() == []
    # 16 rollouts * 10 steps.
    assert len(dataset.samples) == 160

    report = verify_contract_obj(dataset.to_json())
    assert report.ok
    # Both classes present in both splits, episodes disjoint across the split.
    assert report.heldout_class_coverage_ok()
    assert report.episode_disjoint_ok()
    # Provenance records the real source of each variable.
    md = dataset.samples[0].metadata
    assert md["v_provenance"].startswith("token_slice")
    assert md["d_provenance"].startswith("token_slice")
    assert md["label_provenance"] == "episode_success"


def test_seen_unseen_maps_to_train_heldout(tmp_path):
    safe_dir = write_synthetic_safe_dir(tmp_path / "safe2", n_tasks=3, episodes_per_task=2, seed=2)
    rollouts = load_safe_rollout_dir(safe_dir, seen_task_ids={0})
    dataset = rollouts_to_dataset(rollouts, MappingConfig())
    splits = {s.metadata["split"] for s in dataset.samples}
    assert splits == {"train", "test"}
    # Task 0 (seen) -> train; tasks 1,2 (unseen) -> held-out.
    for s in dataset.samples:
        task_id = int(s.metadata["task_id"])
        assert s.metadata["split"] == ("train" if task_id == 0 else "test")


def test_pooled_mode_requires_no_token_groups(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "safe3", n_tasks=2, episodes_per_task=2, seed=3, raw_token_states=False
    )
    rollouts = load_safe_rollout_dir(safe_dir, seen_task_ids={0})
    assert rollouts[0].hidden_states.ndim == 2
    # Token slicing must fail on pooled states; the pooled mapping must succeed.
    with pytest.raises(ValueError):
        rollout_to_samples(rollouts[0], MappingConfig())
    pooled = MappingConfig(
        v=VariableSpec("hidden_pool"),
        l=VariableSpec("text_hash", dim=8),
        d=VariableSpec("hidden_pool"),
    )
    samples = rollout_to_samples(rollouts[0], pooled)
    assert samples[0].metadata["l_provenance"] == "text_hash_proxy"
    assert len(samples[0].l) == 8


def test_verify_flags_class_coverage_and_episode_leak():
    # Held-out split missing the failure class -> coverage incomplete.
    # (8 samples: verify.py mirrors the harness's hard minimum, so a smaller
    # fixture would fail structural validity for the wrong reason.)
    ds = VldaDataset(
        samples=[
            *[_sample(f"tr{i}--t0", success=(i % 2 == 0), split="train") for i in range(6)],
            _sample("he0--t0", success=True, split="test"),
            _sample("he1--t0", success=True, split="test"),
        ]
    )
    report = verify_contract_obj(ds.to_json())
    assert report.ok  # structurally valid
    assert not report.heldout_class_coverage_ok()

    # Same episode id in train and held-out -> leak. (The episode-disjointness
    # verdict is independent of the sample-count structural issue.)
    leak = VldaDataset(
        samples=[
            VldaSample("shared--t0", [1.0], [1.0], [1.0], [1.0], True, "shared", {"split": "train"}),
            VldaSample("shared--t1", [1.0], [1.0], [1.0], [1.0], False, "shared", {"split": "test"}),
        ]
    )
    leak_report = verify_contract_obj(leak.to_json())
    assert not leak_report.episode_disjoint_ok()
    assert leak_report.shared_episode_ids == ["shared"]
    # And the under-minimum fixture is itself flagged, mirroring the harness.
    assert any("requires >= 8" in issue for issue in leak_report.issues)


def test_layerwise_physics_probe_finds_intermediate_peak():
    # Build 4 candidate layers where physics decodability peaks in the middle:
    # layer 0 = noise, layer 1 = strong signal, layer 2 = weaker, layer 3 = noise.
    rng = np.random.default_rng(0)
    n = 200
    target = rng.standard_normal((n, 1))
    train_mask = np.zeros(n, dtype=bool)
    train_mask[: n // 2] = True

    def layer(signal_weight: float) -> np.ndarray:
        feats = rng.standard_normal((n, 6))
        feats[:, 0] = signal_weight * target[:, 0] + (1 - signal_weight) * rng.standard_normal(n)
        return feats

    layers = [layer(0.0), layer(0.95), layer(0.6), layer(0.0)]
    result = layerwise_physics_probe(
        layers, {"object_speed": target}, train_mask, l2=1.0
    )
    assert result.peak_layer == 1
    assert result.best_layer_by_target["object_speed"] == 1
    # The intermediate-peak (emergence-zone) warning fires.
    assert any("emergence" in w for w in result.warnings)


def test_layerwise_physics_probe_boolean_target():
    rng = np.random.default_rng(1)
    n = 160
    y = (rng.standard_normal(n) > 0).astype(float)
    train_mask = np.zeros(n, dtype=bool)
    train_mask[: n // 2] = True
    informative = np.column_stack([y + 0.1 * rng.standard_normal(n), rng.standard_normal(n)])
    noise = rng.standard_normal((n, 2))
    result = layerwise_physics_probe(
        [noise, informative], {"contact": y}, train_mask, boolean_targets={"contact"}
    )
    assert result.peak_layer == 1
    informative_score = next(
        r.score for r in result.per_layer if r.layer_index == 1 and r.target_name == "contact"
    )
    assert informative_score > 0.8
