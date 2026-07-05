"""SAFE rollout intermediate representation, loader, and a synthetic generator.

The real ``vla-safe/SAFE`` rollouts are stored as one ``task{N}--ep{M}--succ{0/1}.csv``
(per-step actions + hand-crafted uncertainty metrics) plus a matching ``.pkl`` dict
with keys ``hidden_states``, ``task_suite_name``, ``task_id``, ``task_description``,
``episode_idx`` (or the upstream ``eposide_idx`` typo) and ``episode_success``
(see ``failure_prob/data/openvla.py`` upstream). This module normalizes that into a
[`SafeRollout`][] and also provides a synthetic generator that writes the *same*
on-disk layout, so the whole adapter is testable without the multi-GB downloads.
"""

from __future__ import annotations

import pickle
import re
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

_FILENAME_RE = re.compile(r"task(\d+)--ep(\d+)--succ([01])\.csv$")

# OpenVLA-style 7-D action column order used by the SAFE CSVs.
ACTION_COLUMNS = (
    "action/dx",
    "action/dy",
    "action/dz",
    "action/droll",
    "action/dpitch",
    "action/dyaw",
    "action/dgripper",
)


@dataclass
class SafeRollout:
    """One normalized SAFE rollout episode."""

    task_id: int
    episode_idx: int
    task_description: str
    episode_success: bool
    actions: np.ndarray  # (T, d_a)
    hidden_states: np.ndarray  # (T, d_h) pooled, or (T, n_token, d_h) raw
    seen: bool = True
    vision_features: np.ndarray | None = None  # (T, d_v), if separately extracted
    language_features: np.ndarray | None = None  # (T, d_l), if a text encoder was run
    token_groups: dict[str, tuple[int, int]] | None = None
    extra: dict = field(default_factory=dict)

    @property
    def n_steps(self) -> int:
        return int(self.actions.shape[0])

    def episode_id(self) -> str:
        return f"task{self.task_id}--ep{self.episode_idx}"


def parse_safe_filename(name: str) -> tuple[int, int, bool]:
    """Parse ``task{N}--ep{M}--succ{0/1}.csv`` -> (task_id, episode_idx, success)."""
    match = _FILENAME_RE.search(name)
    if not match:
        raise ValueError(f"unrecognised SAFE rollout filename: {name!r}")
    return int(match.group(1)), int(match.group(2)), bool(int(match.group(3)))


def _read_action_csv(path: Path) -> np.ndarray:
    """Read the 7 action columns from a SAFE rollout CSV into ``(T, d_a)``."""
    import csv

    with path.open(newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise ValueError(f"{path}: empty CSV")
        missing = [c for c in ACTION_COLUMNS if c not in reader.fieldnames]
        if missing:
            raise ValueError(f"{path}: missing action columns {missing}")
        rows = [[float(row[c]) for c in ACTION_COLUMNS] for row in reader]
    if not rows:
        raise ValueError(f"{path}: no rows")
    return np.asarray(rows, dtype=np.float64)


def _hidden_states_to_array(value: object) -> np.ndarray:
    """Coerce the pickled ``hidden_states`` (numpy, list, or torch tensor) to numpy."""
    if isinstance(value, list):
        value = np.stack([np.asarray(v) for v in value], axis=0)
    # numpy arrays and (CPU) torch tensors both satisfy np.asarray via __array__.
    return np.asarray(value, dtype=np.float64)


def load_safe_rollout_dir(
    directory: str | Path,
    *,
    seen_task_ids: Iterable[int] | None = None,
) -> list[SafeRollout]:
    """Load all ``*.csv`` + ``*.pkl`` rollout pairs under ``directory``.

    ``seen_task_ids`` marks which task ids are "seen" (train); unseen tasks become
    the held-out split. If ``None``, every task is treated as seen (the caller is
    then expected to define a split another way).
    """
    directory = Path(directory)
    seen = set(seen_task_ids) if seen_task_ids is not None else None
    rollouts: list[SafeRollout] = []
    for csv_path in sorted(directory.glob("*.csv")):
        task_id, episode_idx, success = parse_safe_filename(csv_path.name)
        actions = _read_action_csv(csv_path)
        pkl_path = csv_path.with_suffix(".pkl")
        if not pkl_path.exists():
            raise FileNotFoundError(f"missing rollout pickle: {pkl_path}")
        try:
            with pkl_path.open("rb") as handle:
                meta = pickle.load(handle)  # noqa: S301 - trusted local rollout data
        except (pickle.UnpicklingError, EOFError, ValueError) as exc:
            raise ValueError(
                f"corrupt or unreadable rollout pickle {pkl_path}: {exc}"
            ) from exc
        if not isinstance(meta, dict):
            raise ValueError(
                f"rollout pickle {pkl_path} must contain a dict, got {type(meta).__name__}"
            )
        if "eposide_idx" in meta and "episode_idx" not in meta:
            meta["episode_idx"] = meta.pop("eposide_idx")
        if "hidden_states" not in meta:
            raise ValueError(
                f"rollout pickle {pkl_path} is missing required key 'hidden_states' "
                f"(keys: {sorted(meta)})"
            )
        hidden = _hidden_states_to_array(meta["hidden_states"])
        rollouts.append(
            SafeRollout(
                task_id=int(meta.get("task_id", task_id)),
                episode_idx=int(meta.get("episode_idx", episode_idx)),
                task_description=str(meta.get("task_description", f"task {task_id}")),
                episode_success=bool(meta.get("episode_success", success)),
                actions=actions,
                hidden_states=hidden,
                seen=(seen is None) or (int(meta.get("task_id", task_id)) in seen),
                vision_features=_optional_array(meta.get("vision_features")),
                language_features=_optional_array(meta.get("language_features")),
                token_groups=meta.get("token_groups"),
            )
        )
    if not rollouts:
        raise ValueError(f"no SAFE rollouts found under {directory}")
    return rollouts


def _optional_array(value: object) -> np.ndarray | None:
    return None if value is None else np.asarray(value, dtype=np.float64)


def write_synthetic_safe_dir(
    directory: str | Path,
    *,
    n_tasks: int = 4,
    episodes_per_task: int = 4,
    n_steps: int = 12,
    n_tokens: int = 6,
    d_hidden: int = 8,
    d_action: int = 7,
    seed: int = 0,
    raw_token_states: bool = True,
) -> Path:
    """Write a synthetic SAFE-format rollout directory (CSV + PKL) for testing.

    The generated data has a *learnable* structure: a latent per-episode "skill"
    variable drives both the hidden states and the success outcome, so downstream
    PID/baselines/probes see real (not random) signal. With ``raw_token_states`` the
    pickles store raw ``(T, n_token, d)`` hidden states so token slicing is testable.
    """
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(seed)

    for task_id in range(n_tasks):
        for episode_idx in range(episodes_per_task):
            # Latent skill in [-1, 1]; higher skill -> more likely success.
            skill = rng.uniform(-1.0, 1.0)
            success = bool(skill + 0.2 * rng.standard_normal() > 0.0)

            # Actions: smooth trajectory modulated by skill + noise.
            t = np.linspace(0.0, 1.0, n_steps)
            base = np.outer(t, np.ones(d_action)) * (0.5 + 0.5 * skill)
            actions = base + 0.05 * rng.standard_normal((n_steps, d_action))

            if raw_token_states:
                hidden = rng.standard_normal((n_steps, n_tokens, d_hidden)) * 0.3
                # Vision tokens (first third) encode skill; language tokens (middle)
                # encode task identity; state tokens (last third) mix both.
                v_end = n_tokens // 3
                l_end = 2 * n_tokens // 3
                hidden[:, :v_end, 0] += skill
                hidden[:, v_end:l_end, 1] += task_id / n_tasks
                hidden[:, l_end:, 2] += skill + task_id / n_tasks
                token_groups = {
                    "vision": [0, max(1, v_end)],
                    "language": [max(1, v_end), max(2, l_end)],
                    "state": [max(2, l_end), n_tokens],
                }
            else:
                hidden = rng.standard_normal((n_steps, d_hidden)) * 0.3
                hidden[:, 0] += skill
                token_groups = None

            stem = f"task{task_id}--ep{episode_idx}--succ{int(success)}"
            _write_action_csv(directory / f"{stem}.csv", actions)
            meta = {
                "hidden_states": hidden,
                "task_suite_name": "synthetic",
                "task_id": task_id,
                "task_description": f"synthetic task {task_id}: move object {task_id}",
                "episode_idx": episode_idx,
                "episode_success": int(success),
                "token_groups": token_groups,
            }
            with (directory / f"{stem}.pkl").open("wb") as handle:
                pickle.dump(meta, handle)
    return directory


def _write_action_csv(path: Path, actions: np.ndarray) -> None:
    import csv

    with path.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(ACTION_COLUMNS)
        for row in actions:
            writer.writerow(f"{v:.8g}" for v in row)
