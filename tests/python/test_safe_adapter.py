"""Tests for the SAFE -> (V,L,D,A) adapter (numpy only; no compiled extension)."""

from __future__ import annotations

import json
import os
import pickle

import numpy as np
import pytest

from experiments.safe_adapter import (
    MappingConfig,
    IngressLimits,
    SafeRollout,
    VariableSpec,
    VldaDataset,
    VldaSample,
    layerwise_physics_probe,
    load_safe_rollout_dir,
    rollouts_to_dataset,
    text_hash_features,
    verify_contract_file,
    write_safe_bundle_manifest,
    write_synthetic_safe_dir,
)
from experiments.safe_adapter.contract import load_dataset_json
from experiments.safe_adapter.convert import rollout_to_samples
from experiments.safe_adapter.rollouts import ACTION_COLUMNS
from experiments.safe_adapter.verify import verify_contract_obj


def _sample(
    sid: str, dim: int = 3, success: bool = True, split: str = "train"
) -> VldaSample:
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


def _write_legacy_bundle(path, meta: dict, *, rights_status: str = "verified"):
    path.mkdir()
    stem = "task0--ep0--succ1"
    (path / f"{stem}.csv").write_text(
        ",".join(ACTION_COLUMNS)
        + "\n"
        + ",".join("0.1" for _ in ACTION_COLUMNS)
        + "\n",
        encoding="utf-8",
    )
    with (path / f"{stem}.pkl").open("wb") as handle:
        pickle.dump(meta, handle, protocol=pickle.HIGHEST_PROTOCOL)
    write_safe_bundle_manifest(
        path,
        source_name="test/legacy",
        source_revision="fixture-v1",
        rights_status=rights_status,
        rights_reference="unit-test fixture",
        seen_task_ids={0},
        split_origin="unit-test fixture",
        split_frozen_before_outcomes=True,
        contamination_review="unit-test isolated task ids",
    )
    return path


def test_contract_rejects_ragged_and_nonfinite():
    with pytest.raises(ValueError):
        VldaSample(
            sample_id="x", v=[float("nan")], l=[1.0], d=[1.0], a=[1.0], success=True
        )
    # Ragged across samples is caught by dataset.validate().
    s0 = _sample("e0--t0", dim=3)
    bad = VldaSample(
        sample_id="e0--t1",
        v=[1.0, 2.0],
        l=[2.0] * 3,
        d=[3.0] * 3,
        a=[4.0] * 3,
        success=False,
    )
    ds = VldaDataset(samples=[s0, bad])
    issues = ds.validate()
    assert any("v has length" in issue for issue in issues)


@pytest.mark.parametrize(
    ("kwargs", "error"),
    [
        ({"sample_id": 1}, ValueError),
        ({"v": [True]}, TypeError),
        ({"success": "false"}, TypeError),
        ({"episode_id": 7}, TypeError),
        ({"metadata": {7: "x"}}, TypeError),
    ],
)
def test_contract_public_api_rejects_coercive_types(kwargs, error):
    values = {
        "sample_id": "s",
        "v": [1.0],
        "l": [1.0],
        "d": [1.0],
        "a": [1.0],
        "success": True,
        "episode_id": "e",
        "metadata": {"split": "train"},
    }
    values.update(kwargs)
    with pytest.raises(error):
        VldaSample(**values)


def test_contract_round_trips_and_writes(tmp_path):
    # >= 8 samples: verify.py mirrors the Rust harness's hard minimum.
    ds = VldaDataset(
        samples=[
            _sample(
                f"e{i}--t0", success=(i % 2 == 0), split="train" if i < 6 else "test"
            )
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
    assert (safe_dir / "safe_bundle_manifest.json").is_file()
    assert not list(safe_dir.glob("*.pkl"))
    assert rollouts[0].extra["ingest_format"] == "canonical_npz_json_v1"
    assert len(rollouts[0].extra["bundle_manifest_sha256"]) == 64
    # Raw per-token hidden states with token groups, so token slicing works.
    assert rollouts[0].hidden_states.ndim == 3
    assert rollouts[0].token_groups is not None

    dataset = rollouts_to_dataset(rollouts, MappingConfig())
    assert dataset.validate() == []
    assert dataset.source == "prisoma/synthetic-safe"
    assert dataset.model == "synthetic-generator"
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
    assert md["raw_csv_sha256"] == rollouts[0].extra["raw_csv_sha256"]
    assert md["bundle_manifest_sha256"] == rollouts[0].extra["bundle_manifest_sha256"]
    assert md["bundle_manifest_locator_status"] == "external_not_archived_by_converter"
    assert len(md["instruction_sha256"]) == 64
    assert len(md["mapping_config_sha256"]) == 64
    assert md["semantic_validation_status"] == "unvalidated"
    assert md["split_frozen_before_outcomes"] == "true"


def test_legacy_pickle_is_default_off_and_restricted_opt_in_accepts_numpy(tmp_path):
    legacy = _write_legacy_bundle(
        tmp_path / "legacy",
        {
            "hidden_states": np.asarray([[1.0, 2.0]]),
            "task_id": 0,
            "episode_idx": 0,
            "episode_success": True,
        },
    )

    with pytest.raises(ValueError, match="disabled by default"):
        load_safe_rollout_dir(legacy, seen_task_ids={0})

    rollouts = load_safe_rollout_dir(
        legacy,
        seen_task_ids={0},
        allow_legacy_pickle=True,
    )
    assert rollouts[0].extra["ingest_format"] == "legacy_numpy_pickle_v1"


def test_restricted_legacy_pickle_rejects_code_execution_gadget(tmp_path):
    marker = tmp_path / "pickle-ran"

    class Gadget:
        def __reduce__(self):
            return os.system, (f"touch {marker}",)

    legacy = _write_legacy_bundle(
        tmp_path / "malicious",
        {
            "hidden_states": Gadget(),
            "task_id": 0,
            "episode_idx": 0,
            "episode_success": True,
        },
    )

    with pytest.raises(ValueError, match="forbidden pickle global"):
        load_safe_rollout_dir(
            legacy,
            seen_task_ids={0},
            allow_legacy_pickle=True,
        )
    assert not marker.exists()


def test_manifest_detects_byte_drift_before_loading(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "drift", n_tasks=2, episodes_per_task=1, n_steps=2, seed=7
    )
    csv_path = next(safe_dir.glob("*.csv"))
    with csv_path.open("a", encoding="utf-8") as handle:
        handle.write("0,0,0,0,0,0,0\n")

    with pytest.raises(ValueError, match="content receipt mismatch"):
        load_safe_rollout_dir(safe_dir, seen_task_ids={0})


def test_bundle_rejects_unlisted_directories_and_schema_bool(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "coverage", n_tasks=2, episodes_per_task=1, n_steps=2, seed=11
    )
    (safe_dir / "unlisted").mkdir()
    with pytest.raises(ValueError, match="unsupported non-file"):
        load_safe_rollout_dir(safe_dir, seen_task_ids={0})
    (safe_dir / "unlisted").rmdir()

    manifest_path = safe_dir / "safe_bundle_manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["schema_version"] = True
    manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
    with pytest.raises(ValueError, match="schema_version"):
        load_safe_rollout_dir(safe_dir, seen_task_ids={0})


@pytest.mark.skipif(not hasattr(os, "mkfifo"), reason="FIFO test requires POSIX")
def test_nonregular_inputs_are_rejected_without_blocking(tmp_path):
    fifo_bundle = tmp_path / "fifo-bundle"
    fifo_bundle.mkdir()
    os.mkfifo(fifo_bundle / "safe_bundle_manifest.json")
    with pytest.raises(ValueError, match="regular file"):
        load_safe_rollout_dir(fifo_bundle)

    contract_fifo = tmp_path / "contract.json"
    os.mkfifo(contract_fifo)
    with pytest.raises(ValueError, match="regular file"):
        load_dataset_json(contract_fifo)

    safe_dir = write_synthetic_safe_dir(
        tmp_path / "payload-fifo", n_tasks=2, episodes_per_task=1, n_steps=2
    )
    csv_path = next(safe_dir.glob("*.csv"))
    csv_path.unlink()
    os.mkfifo(csv_path)
    with pytest.raises(ValueError, match="regular file"):
        load_safe_rollout_dir(safe_dir, seen_task_ids={0})


def test_legacy_pickle_rejects_trailing_objects(tmp_path):
    legacy = _write_legacy_bundle(
        tmp_path / "trailing",
        {
            "hidden_states": np.asarray([[1.0, 2.0]]),
            "task_id": 0,
            "episode_idx": 0,
            "episode_success": True,
        },
    )
    pickle_path = next(legacy.glob("*.pkl"))
    with pickle_path.open("ab") as handle:
        pickle.dump({"second": "object"}, handle)
    write_safe_bundle_manifest(
        legacy,
        source_name="test/legacy",
        source_revision="fixture-v2",
        rights_status="verified",
        rights_reference="unit-test fixture",
        seen_task_ids={0},
        overwrite=True,
        split_origin="unit-test fixture",
        split_frozen_before_outcomes=True,
        contamination_review="unit-test isolated task ids",
    )
    with pytest.raises(ValueError, match="trailing bytes"):
        load_safe_rollout_dir(
            legacy,
            seen_task_ids={0},
            allow_legacy_pickle=True,
        )


def test_derived_output_budget_runs_before_text_hash_allocation():
    rollout = SafeRollout(
        task_id=0,
        episode_idx=0,
        task_description="bounded instruction",
        episode_success=True,
        actions=np.ones((200, 7)),
        hidden_states=np.ones((200, 3, 2)),
        token_groups={"vision": (0, 1), "language": (1, 2), "state": (2, 3)},
    )
    config = MappingConfig(l=VariableSpec("text_hash", dim=65_536))

    with pytest.raises(ValueError, match="derived dataset would have"):
        rollout_to_samples(rollout, config)


def test_synthetic_generation_preflights_work_before_creating_directory(tmp_path):
    destination = tmp_path / "too-many"
    with pytest.raises(ValueError, match="rollout count"):
        write_synthetic_safe_dir(
            destination,
            n_tasks=4_097,
            episodes_per_task=1,
        )
    assert not destination.exists()

    oversized_dimension = tmp_path / "too-wide"
    with pytest.raises(ValueError, match="array dimension"):
        write_synthetic_safe_dir(
            oversized_dimension,
            n_tasks=1,
            episodes_per_task=1,
            n_steps=1,
            n_tokens=3,
            d_hidden=1_000_001,
        )
    assert not oversized_dimension.exists()


def test_metadata_cannot_override_filename_identity_or_outcome(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "identity", n_tasks=2, episodes_per_task=1, n_steps=2, seed=8
    )
    metadata_path = next(safe_dir.glob("*.metadata.json"))
    metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
    metadata["episode_success"] = not metadata["episode_success"]
    metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
    write_safe_bundle_manifest(
        safe_dir,
        source_name="prisoma/synthetic-safe",
        source_revision="identity-conflict-fixture",
        rights_status="synthetic_generated",
        rights_reference="unit-test fixture",
        seen_task_ids={0},
        overwrite=True,
        split_origin="unit-test fixture",
        split_frozen_before_outcomes=True,
        contamination_review="unit-test isolated task ids",
    )

    with pytest.raises(ValueError, match="conflicts with filename"):
        load_safe_rollout_dir(safe_dir, seen_task_ids={0})


def test_resource_limits_reject_rows_and_tensor_shape_before_conversion(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "bounded", n_tasks=2, episodes_per_task=1, n_steps=3, seed=9
    )
    with pytest.raises(ValueError, match="CSV row limit"):
        load_safe_rollout_dir(
            safe_dir,
            seen_task_ids={0},
            limits=IngressLimits(max_csv_rows=2),
        )
    with pytest.raises(ValueError, match="elements exceed"):
        load_safe_rollout_dir(
            safe_dir,
            seen_task_ids={0},
            limits=IngressLimits(max_tensor_elements=10),
        )


def test_directory_entry_enumeration_respects_rollout_bound(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "entry-bound", n_tasks=1, episodes_per_task=1, n_steps=2
    )
    (safe_dir / "junk-a").write_bytes(b"")
    (safe_dir / "junk-b").write_bytes(b"")

    with pytest.raises(ValueError, match="finite 3-file limit"):
        load_safe_rollout_dir(
            safe_dir,
            seen_task_ids=set(),
            limits=IngressLimits(max_rollouts=1),
        )


def test_unverified_rights_and_split_drift_fail_closed(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "rights", n_tasks=2, episodes_per_task=1, n_steps=2, seed=10
    )
    write_safe_bundle_manifest(
        safe_dir,
        source_name="vla-safe/SAFE",
        source_revision="test-revision",
        rights_status="unverified",
        rights_reference="no license receipt yet",
        seen_task_ids={0},
        overwrite=True,
        split_origin="unit-test fixture",
        split_frozen_before_outcomes=True,
        contamination_review="unit-test isolated task ids",
    )
    with pytest.raises(ValueError, match="rights status is unverified"):
        load_safe_rollout_dir(safe_dir, seen_task_ids={0})
    with pytest.raises(ValueError, match="split receipt"):
        load_safe_rollout_dir(
            safe_dir,
            seen_task_ids={1},
            allow_unverified_rights=True,
        )
    assert (
        len(
            load_safe_rollout_dir(
                safe_dir,
                seen_task_ids={0},
                allow_unverified_rights=True,
            )
        )
        == 2
    )


def test_unfrozen_split_override_remains_machine_readably_blocked(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "unfrozen",
        n_tasks=4,
        episodes_per_task=4,
        n_steps=2,
        seed=1,
    )
    write_safe_bundle_manifest(
        safe_dir,
        source_name="synthetic/safe_adapter",
        source_revision="unfrozen-fixture",
        rights_status="synthetic_generated",
        rights_reference="unit-test fixture",
        seen_task_ids={0, 1},
        overwrite=True,
        split_origin="post-outcome audit partition",
        split_frozen_before_outcomes=False,
        contamination_review="not_assessed",
        model_id="unit-test-model",
        checkpoint_revision="unit-test-checkpoint",
        hook_id="unit-test-hook",
        tensor_contract_sha256="a" * 64,
    )

    with pytest.raises(ValueError, match="split was not frozen"):
        load_safe_rollout_dir(safe_dir, seen_task_ids={0, 1})

    rollouts = load_safe_rollout_dir(
        safe_dir,
        seen_task_ids={0, 1},
        allow_unfrozen_split=True,
    )
    report = verify_contract_obj(rollouts_to_dataset(rollouts).to_json())
    assert not report.ok
    assert any("scientific eligibility is blocked" in issue for issue in report.issues)


def test_contract_json_read_is_bounded_and_write_is_atomic(tmp_path):
    oversized = tmp_path / "oversized.json"
    oversized.write_text('{"samples": []}', encoding="utf-8")
    with pytest.raises(ValueError, match="limit is 4"):
        load_dataset_json(oversized, max_bytes=4)

    dataset = VldaDataset(samples=[_sample("e0--t0")])
    destination = tmp_path / "dataset.json"
    destination.write_text("old", encoding="utf-8")
    with pytest.raises(FileExistsError, match="refusing to overwrite"):
        dataset.write_json(destination)
    dataset.write_json(destination, overwrite=True)

    assert load_dataset_json(destination)["samples"][0]["sample_id"] == "e0--t0"
    assert not list(tmp_path.glob(".dataset.json.*.tmp"))


def test_seen_unseen_maps_to_train_heldout(tmp_path):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / "safe2", n_tasks=3, episodes_per_task=2, seed=2
    )
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
        tmp_path / "safe3",
        n_tasks=2,
        episodes_per_task=2,
        seed=3,
        raw_token_states=False,
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
            *[
                _sample(f"tr{i}--t0", success=(i % 2 == 0), split="train")
                for i in range(6)
            ],
            _sample("he0--t0", success=True, split="test"),
            _sample("he1--t0", success=True, split="test"),
        ]
    )
    report = verify_contract_obj(ds.to_json())
    assert not report.ok
    assert not report.heldout_class_coverage_ok()
    assert any("success and failure" in issue for issue in report.issues)

    # Same episode id in train and held-out -> leak. (The episode-disjointness
    # verdict is independent of the sample-count structural issue.)
    leak = VldaDataset(
        samples=[
            VldaSample(
                "shared--t0",
                [1.0],
                [1.0],
                [1.0],
                [1.0],
                True,
                "shared",
                {"split": "train"},
            ),
            VldaSample(
                "shared--t1",
                [1.0],
                [1.0],
                [1.0],
                [1.0],
                False,
                "shared",
                {"split": "test"},
            ),
        ]
    )
    leak_report = verify_contract_obj(leak.to_json())
    assert not leak_report.episode_disjoint_ok()
    assert leak_report.shared_episode_ids == ["shared"]
    assert any("both train and held-out" in issue for issue in leak_report.issues)
    # And the under-minimum fixture is itself flagged, mirroring the harness.
    assert any("requires >= 8" in issue for issue in leak_report.issues)


def test_verify_rejects_types_the_rust_contract_rejects():
    dataset = VldaDataset(samples=[_sample(f"e{i}--t0") for i in range(8)]).to_json()
    dataset["samples"][0]["sample_id"] = 7
    dataset["samples"][1]["v"] = [True]
    dataset["samples"][2]["labels"]["success"] = "false"
    dataset["samples"][3]["metadata"] = []
    dataset["samples"][4]["episode_id"] = 9
    dataset["samples"][5]["metadata"]["split"] = "bogus"
    dataset["samples"][6].pop("episode_id")
    dataset["samples"][7]["labels"] = []

    report = verify_contract_obj(dataset)

    assert not report.ok
    assert any("sample_id" in issue for issue in report.issues)
    assert any("non-numeric" in issue for issue in report.issues)
    assert any("labels.success" in issue for issue in report.issues)
    assert any("metadata" in issue for issue in report.issues)
    assert any("episode_id" in issue for issue in report.issues)
    assert any("recognized train or held-out" in issue for issue in report.issues)
    assert sum("labels must be an object" in issue for issue in report.issues) == 1


def test_verify_rejects_unknown_and_mixed_split_eligibility():
    dataset = VldaDataset(
        samples=[
            _sample(
                f"e{i}--t0",
                success=(i % 2 == 0),
                split="train" if i < 4 else "test",
            )
            for i in range(8)
        ]
    ).to_json()
    dataset["samples"][0]["metadata"]["split_scientific_eligibility"] = "typo"
    report = verify_contract_obj(dataset)
    assert not report.ok
    assert any(
        "unknown split scientific eligibility" in issue for issue in report.issues
    )

    for sample in dataset["samples"]:
        sample["metadata"]["split_scientific_eligibility"] = "structural_split_ready"
    dataset["samples"][-1]["metadata"]["split_scientific_eligibility"] = (
        "blocked_unfrozen_or_unreviewed"
    )
    report = verify_contract_obj(dataset)
    assert not report.ok
    assert any("statuses are inconsistent" in issue for issue in report.issues)

    for sample in dataset["samples"]:
        sample["metadata"]["split_scientific_eligibility"] = "structural_split_ready"
    dataset["samples"][-1]["metadata"].pop("split_scientific_eligibility")
    report = verify_contract_obj(dataset)
    assert not report.ok
    assert any("stamped consistently" in issue for issue in report.issues)


def test_dataset_conversion_requires_bound_lineage():
    rollout = SafeRollout(
        task_id=0,
        episode_idx=0,
        task_description="unbound in-memory rollout",
        episode_success=True,
        actions=np.ones((2, 7)),
        hidden_states=np.ones((2, 3, 2)),
        token_groups={"vision": (0, 1), "language": (1, 2), "state": (2, 3)},
    )
    with pytest.raises(ValueError, match="required content-addressed ingress lineage"):
        rollouts_to_dataset([rollout])


@pytest.mark.parametrize(
    "receipt_key",
    [
        "source_revision",
        "checkpoint_revision",
        "seen_split_receipt_sha256",
        "rights_reference_sha256",
    ],
)
def test_dataset_conversion_rejects_mixed_regime_receipts(tmp_path, receipt_key):
    safe_dir = write_synthetic_safe_dir(
        tmp_path / receipt_key, n_tasks=2, episodes_per_task=2, n_steps=2
    )
    rollouts = load_safe_rollout_dir(safe_dir, seen_task_ids={0})
    rollouts[0].extra[receipt_key] = f"different-{receipt_key}"

    with pytest.raises(
        ValueError, match=f"regime-defining ingress receipt '{receipt_key}'"
    ):
        rollouts_to_dataset(rollouts)


def test_layerwise_physics_probe_finds_intermediate_peak():
    # Build 4 candidate layers where physics decodability peaks in the middle:
    # layer 0 = noise, layer 1 = strong signal, layer 2 = weaker, layer 3 = noise.
    rng = np.random.default_rng(0)
    n = 200
    target = rng.standard_normal((n, 1))
    fit_mask = np.zeros(n, dtype=bool)
    selection_mask = np.zeros(n, dtype=bool)
    evaluation_mask = np.zeros(n, dtype=bool)
    fit_mask[:80] = True
    selection_mask[80:140] = True
    evaluation_mask[140:] = True

    def layer(signal_weight: float) -> np.ndarray:
        feats = rng.standard_normal((n, 6))
        feats[:, 0] = signal_weight * target[:, 0] + (
            1 - signal_weight
        ) * rng.standard_normal(n)
        return feats

    layers = [layer(0.0), layer(0.95), layer(0.6), layer(0.0)]
    result = layerwise_physics_probe(
        layers,
        {"object_speed": target},
        fit_mask,
        selection_mask=selection_mask,
        evaluation_mask=evaluation_mask,
        group_ids=np.arange(n),
        probe_components=4,
        l2=1.0,
    )
    assert result.peak_layer == 1
    assert result.best_layer_by_target["object_speed"] == 1
    assert result.evaluation_results[0].split == "evaluation"
    assert result.evaluation_results[0].layer_index == result.peak_layer
    assert result.evaluation_score > 0.5
    # The intermediate-peak (emergence-zone) warning fires.
    assert any("emergence" in w for w in result.warnings)


def test_layerwise_physics_probe_boolean_target():
    rng = np.random.default_rng(1)
    n = 160
    y = (rng.standard_normal(n) > 0).astype(float)
    fit_mask = np.zeros(n, dtype=bool)
    selection_mask = np.zeros(n, dtype=bool)
    evaluation_mask = np.zeros(n, dtype=bool)
    fit_mask[:60] = True
    selection_mask[60:110] = True
    evaluation_mask[110:] = True
    informative = np.column_stack(
        [y + 0.1 * rng.standard_normal(n), rng.standard_normal(n)]
    )
    noise = rng.standard_normal((n, 2))
    result = layerwise_physics_probe(
        [noise, informative],
        {"contact": y},
        fit_mask,
        selection_mask=selection_mask,
        evaluation_mask=evaluation_mask,
        group_ids=np.arange(n),
        probe_components=2,
        boolean_targets={"contact"},
    )
    assert result.peak_layer == 1
    informative_score = next(
        r.score
        for r in result.per_layer
        if r.layer_index == 1 and r.target_name == "contact"
    )
    assert informative_score > 0.8
    assert result.evaluation_score > 0.8


def test_layer_probe_keeps_evaluation_out_of_layer_selection():
    """A final-holdout reversal is reported; it cannot switch the chosen layer."""

    rng = np.random.default_rng(91)
    n = 180
    target = rng.standard_normal(n)
    fit_mask = np.zeros(n, dtype=bool)
    selection_mask = np.zeros(n, dtype=bool)
    evaluation_mask = np.zeros(n, dtype=bool)
    fit_mask[:60] = True
    selection_mask[60:120] = True
    evaluation_mask[120:] = True

    selection_only = rng.standard_normal((n, 2))
    selection_only[:120, 0] = target[:120]
    stable = rng.standard_normal((n, 2))
    stable[:, 0] = 0.65 * target + 0.35 * rng.standard_normal(n)

    result = layerwise_physics_probe(
        [selection_only, stable],
        {"speed": target},
        fit_mask,
        selection_mask=selection_mask,
        evaluation_mask=evaluation_mask,
        group_ids=np.arange(n),
        probe_components=2,
    )

    assert result.peak_layer == 0
    assert {item.layer_index for item in result.evaluation_results} == {0}
    assert result.selection_score > 0.9
    assert result.evaluation_score < 0.2


def test_layer_probe_rejects_cross_partition_group_leakage():
    rng = np.random.default_rng(92)
    n = 18
    states = [rng.standard_normal((n, 2))]
    target = rng.standard_normal(n)
    fit_mask = np.zeros(n, dtype=bool)
    selection_mask = np.zeros(n, dtype=bool)
    evaluation_mask = np.zeros(n, dtype=bool)
    fit_mask[:6] = True
    selection_mask[6:12] = True
    evaluation_mask[12:] = True
    groups = np.arange(n)
    groups[6] = groups[0]

    with pytest.raises(ValueError, match="disjoint across partitions"):
        layerwise_physics_probe(
            states,
            {"speed": target},
            fit_mask,
            selection_mask=selection_mask,
            evaluation_mask=evaluation_mask,
            group_ids=groups,
            probe_components=2,
        )
