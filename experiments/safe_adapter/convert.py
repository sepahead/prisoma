"""Convert SAFE rollouts into the ``(V, L, D, A)`` + labels harness contract."""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np

from .contract import VldaDataset, VldaSample, split_token
from .extract import VariableSpec, resolve_variable
from .rollouts import SafeRollout


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


def rollout_to_samples(
    rollout: SafeRollout, config: MappingConfig
) -> list[VldaSample]:
    """Resolve one rollout into per-step contract samples (one per timestep)."""
    n = rollout.n_steps
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
    samples: list[VldaSample] = []
    for rollout in rollouts:
        samples.extend(rollout_to_samples(rollout, config))
    dataset = VldaDataset(
        samples=samples,
        run_id=run_id or "safe_adapter",
        source="vla-safe/SAFE",
        model=model,
        task=task,
    )
    issues = dataset.validate()
    if issues:
        raise ValueError("converted dataset is not contract-valid:\n  " + "\n  ".join(issues))
    return dataset
