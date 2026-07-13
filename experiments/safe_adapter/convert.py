"""Convert SAFE rollouts into the ``(V, L, D, A)`` + labels harness contract."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field

import numpy as np

from .contract import (
    MAX_CONTRACT_JSON_BYTES,
    VldaDataset,
    VldaSample,
    split_token,
)
from .extract import VariableSpec, resolve_variable
from .rollouts import SafeRollout


# Compact content-addressed ingress lineage copied into every emitted sample.
# File paths/sizes and the complete rights/split receipt remain in the hashed
# bundle manifest; these fields are sufficient to trace a sample back to it and
# to the exact per-episode source bytes without bloating each run-log event.
_INGRESS_METADATA_KEYS = (
    "ingest_format",
    "bundle_manifest_path",
    "bundle_manifest_locator_status",
    "bundle_manifest_sha256",
    "source_name",
    "source_revision",
    "rights_status",
    "rights_reference_sha256",
    "seen_split_receipt_sha256",
    "split_origin_sha256",
    "split_frozen_before_outcomes",
    "contamination_review_sha256",
    "seen_role",
    "heldout_role",
    "split_scientific_eligibility",
    "task_suite_name",
    "instruction_sha256",
    "model_id",
    "checkpoint_revision",
    "hook_id",
    "tensor_contract_sha256",
    "semantic_validation_status",
    "raw_csv_path",
    "raw_csv_sha256",
    "raw_arrays_path",
    "raw_arrays_sha256",
    "raw_metadata_path",
    "raw_metadata_sha256",
)

_REQUIRED_BOUND_LINEAGE_KEYS = (
    "ingest_format",
    "bundle_manifest_sha256",
    "source_name",
    "source_revision",
    "rights_status",
    "rights_reference_sha256",
    "seen_split_receipt_sha256",
    "split_origin_sha256",
    "split_frozen_before_outcomes",
    "contamination_review_sha256",
    "split_scientific_eligibility",
    "model_id",
    "checkpoint_revision",
    "hook_id",
    "tensor_contract_sha256",
    "semantic_validation_status",
    "raw_csv_sha256",
    "raw_arrays_sha256",
)

_REGIME_LINEAGE_KEYS = (
    "bundle_manifest_sha256",
    "source_name",
    "source_revision",
    "rights_status",
    "rights_reference_sha256",
    "seen_split_receipt_sha256",
    "split_origin_sha256",
    "split_frozen_before_outcomes",
    "contamination_review_sha256",
    "split_scientific_eligibility",
    "model_id",
    "checkpoint_revision",
    "hook_id",
    "tensor_contract_sha256",
    "semantic_validation_status",
)

_MAX_OUTPUT_SAMPLES = 250_000
_MAX_DERIVED_VLDA_ELEMENTS = 10_000_000
_ESTIMATED_BYTES_PER_ELEMENT = 32
_ESTIMATED_BYTES_PER_SAMPLE = 4_096


@dataclass
class _OutputBudget:
    samples: int = 0
    elements: int = 0
    estimated_json_bytes: int = 0

    def reserve(self, *, samples: int, elements: int) -> None:
        next_samples = self.samples + samples
        next_elements = self.elements + elements
        next_bytes = (
            self.estimated_json_bytes
            + elements * _ESTIMATED_BYTES_PER_ELEMENT
            + samples * _ESTIMATED_BYTES_PER_SAMPLE
        )
        if next_samples > _MAX_OUTPUT_SAMPLES:
            raise ValueError(
                f"derived dataset would have {next_samples} samples; "
                f"limit is {_MAX_OUTPUT_SAMPLES}"
            )
        if next_elements > _MAX_DERIVED_VLDA_ELEMENTS:
            raise ValueError(
                f"derived dataset would have {next_elements} VLDA elements; "
                f"limit is {_MAX_DERIVED_VLDA_ELEMENTS}"
            )
        if next_bytes > MAX_CONTRACT_JSON_BYTES:
            raise ValueError(
                f"derived dataset estimated JSON size {next_bytes} exceeds "
                f"the {MAX_CONTRACT_JSON_BYTES}-byte contract limit"
            )
        self.samples = next_samples
        self.elements = next_elements
        self.estimated_json_bytes = next_bytes


def _mapping_receipt(config: "MappingConfig") -> tuple[str, str]:
    def spec(value: VariableSpec) -> dict[str, object]:
        return {
            "mode": value.mode,
            "token_group": value.token_group,
            "dim": value.dim,
        }

    receipt = {
        "schema_version": 1,
        "adapter_contract": "safe_adapter_mapping_v1",
        "v": spec(config.v),
        "l": spec(config.l),
        "d": spec(config.d),
        "seen_role": "train" if config.train_if_seen else "test",
    }
    encoded = json.dumps(
        receipt,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    )
    return encoded, hashlib.sha256(encoded.encode("utf-8")).hexdigest()


@dataclass
class MappingConfig:
    """How to source each of V, L, D from a rollout, and the success label.

    Defaults assume *raw per-token* hidden states with named token groups (the
    structure of the synthetic fixture and of OpenVLA-style states when the raw
    per-token tensor is exported): ``V`` = vision token slice, ``L`` = language
    token slice, ``D`` = state token slice. When only pooled hidden states are
    available, switch ``v``/``l``/``d`` to ``hidden_pool`` (degenerate: V==L==D) or
    supply explicit ``vision_features`` / ``language_features`` per rollout and use
    ``explicit`` / ``text_hash``.
    """

    v: VariableSpec = field(
        default_factory=lambda: VariableSpec("token_slice", token_group="vision")
    )
    l: VariableSpec = field(
        default_factory=lambda: VariableSpec("token_slice", token_group="language")
    )
    d: VariableSpec = field(
        default_factory=lambda: VariableSpec("token_slice", token_group="state")
    )
    # Map SAFE seen/unseen -> train/held-out. SAFE evaluates zero-shot on unseen
    # tasks, so seen=train is the leakage-safe default.
    train_if_seen: bool = True


def _resolved_dim(
    spec: VariableSpec,
    *,
    rollout: SafeRollout,
    name: str,
) -> int:
    hidden = rollout.hidden_states
    if spec.mode == "hidden_pool":
        if hidden.ndim not in (2, 3):
            raise ValueError(f"{name}: hidden states must be 2-D or 3-D")
        return int(hidden.shape[-1])
    if spec.mode == "token_slice":
        if (
            hidden.ndim != 3
            or not rollout.token_groups
            or spec.token_group not in rollout.token_groups
        ):
            raise ValueError(
                f"{name}: token_slice requires a declared group on 3-D hidden states"
            )
        return int(hidden.shape[-1])
    if spec.mode == "explicit":
        features = (
            rollout.vision_features
            if name == "V"
            else rollout.language_features
            if name == "L"
            else None
        )
        if features is None or features.ndim != 2:
            raise ValueError(f"{name}: explicit mode requires 2-D features")
        return int(features.shape[1])
    if spec.mode == "text_hash":
        if name != "L" or isinstance(spec.dim, bool) or not isinstance(spec.dim, int):
            raise ValueError(f"{name}: text_hash requires an integer L dimension")
        return spec.dim
    raise ValueError(f"{name}: unknown mode {spec.mode!r}")


def rollout_to_samples(
    rollout: SafeRollout,
    config: MappingConfig,
    *,
    _budget: _OutputBudget | None = None,
) -> list[VldaSample]:
    """Resolve one rollout into per-step contract samples (one per timestep)."""
    n = rollout.n_steps
    budget = _budget or _OutputBudget()
    resolved_dims = [
        _resolved_dim(config.v, rollout=rollout, name="V"),
        _resolved_dim(config.l, rollout=rollout, name="L"),
        _resolved_dim(config.d, rollout=rollout, name="D"),
        int(rollout.actions.shape[1]),
    ]
    budget.reserve(samples=n, elements=n * sum(resolved_dims))
    v, v_prov = resolve_variable(
        config.v,
        name="V",
        hidden_states=rollout.hidden_states,
        n_steps=n,
        instruction=rollout.task_description,
        token_groups=rollout.token_groups,
        explicit_features=rollout.vision_features,
    )
    l, l_prov = resolve_variable(
        config.l,
        name="L",
        hidden_states=rollout.hidden_states,
        n_steps=n,
        instruction=rollout.task_description,
        token_groups=rollout.token_groups,
        explicit_features=rollout.language_features,
    )
    d, d_prov = resolve_variable(
        config.d,
        name="D",
        hidden_states=rollout.hidden_states,
        n_steps=n,
        instruction=rollout.task_description,
        token_groups=rollout.token_groups,
        explicit_features=None,
    )
    a = np.asarray(rollout.actions, dtype=np.float64)
    # `n` IS `actions.shape[0]`, so comparing `a.shape[0]` to `n` is a tautology
    # that never fires. The real alignment risk is the hidden-state-derived
    # V/L/D disagreeing with the action count (a SAFE pickle can carry an extra
    # pre-action state, or a truncated tail): validate those against `n` so a
    # misaligned rollout raises a clear contract error instead of silently
    # truncating in the per-step loop below or dying with a raw IndexError.
    for name, arr in (("V", v), ("L", l), ("D", d)):
        if arr.shape[0] != n:
            raise ValueError(
                f"{name}/action step-count mismatch in {rollout.episode_id()}: "
                f"{arr.shape[0]} rows vs {n} actions"
            )

    split = split_token(rollout.seen, train_if_seen=config.train_if_seen)
    episode_id = rollout.episode_id()
    ingress_metadata = {}
    for key in _INGRESS_METADATA_KEYS:
        if key not in rollout.extra:
            continue
        value = rollout.extra[key]
        if value is None:
            continue
        ingress_metadata[key] = (
            str(value).lower() if isinstance(value, bool) else str(value)
        )
    samples: list[VldaSample] = []
    for step in range(n):
        samples.append(
            VldaSample(
                sample_id=f"{episode_id}--t{step}",
                v=v[step].tolist(),
                l=l[step].tolist(),
                d=d[step].tolist(),
                a=a[step].tolist(),
                success=rollout.episode_success,
                episode_id=episode_id,
                metadata={
                    "split": split,
                    "task_id": str(rollout.task_id),
                    "step": str(step),
                    "v_provenance": v_prov,
                    "l_provenance": l_prov,
                    "d_provenance": d_prov,
                    "a_provenance": "action_vector",
                    "label_provenance": "episode_success",
                    **ingress_metadata,
                },
            )
        )
    return samples


def rollouts_to_dataset(
    rollouts: list[SafeRollout],
    config: MappingConfig | None = None,
    *,
    run_id: str | None = None,
    model: str | None = None,
    task: str | None = None,
) -> VldaDataset:
    """Convert a list of SAFE rollouts into a contract :class:`VldaDataset`.

    Raises if the resulting dataset is not contract-valid (e.g. ragged dimensions
    from rollouts with differing hidden-state widths).
    """
    config = config or MappingConfig()
    if not rollouts:
        raise ValueError("cannot convert an empty rollout list")
    for index, rollout in enumerate(rollouts):
        missing = [
            key
            for key in _REQUIRED_BOUND_LINEAGE_KEYS
            if key not in rollout.extra
            or rollout.extra[key] is None
            or rollout.extra[key] == ""
        ]
        if missing:
            raise ValueError(
                f"rollout {index} lacks required content-addressed ingress lineage: {missing}"
            )
        unresolved = [
            key
            for key in ("model_id", "checkpoint_revision", "hook_id")
            if str(rollout.extra[key]).strip().lower() == "unresolved"
        ]
        if unresolved:
            raise ValueError(
                f"rollout {index} has unresolved capture lineage: {unresolved}"
            )
    for key in _REGIME_LINEAGE_KEYS:
        values = {rollout.extra[key] for rollout in rollouts}
        if len(values) != 1:
            raise ValueError(f"rollouts mix regime-defining ingress receipt {key!r}")
    if not config.train_if_seen and any(
        rollout.extra.get("seen_role") == "train" for rollout in rollouts
    ):
        raise ValueError(
            "cannot invert a content-addressed split receipt: seen tasks are frozen as train"
        )
    declared_source = str(rollouts[0].extra["source_name"])
    declared_model = str(rollouts[0].extra["model_id"])
    if model is not None and model != declared_model:
        raise ValueError(
            f"requested model {model!r} conflicts with manifest model {declared_model!r}"
        )
    mapping_json, mapping_sha256 = _mapping_receipt(config)
    samples: list[VldaSample] = []
    output_budget = _OutputBudget()
    for rollout in rollouts:
        samples.extend(rollout_to_samples(rollout, config, _budget=output_budget))
    for sample in samples:
        sample.metadata["mapping_config"] = mapping_json
        sample.metadata["mapping_config_sha256"] = mapping_sha256
        sample.metadata["adapter_contract_version"] = "safe_adapter_mapping_v1"
    dataset = VldaDataset(
        samples=samples,
        run_id=run_id or "safe_adapter",
        source=declared_source,
        model=declared_model,
        task=task,
    )
    issues = dataset.validate()
    if issues:
        raise ValueError(
            "converted dataset is not contract-valid:\n  " + "\n  ".join(issues)
        )
    return dataset
