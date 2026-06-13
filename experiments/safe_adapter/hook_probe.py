"""Layerwise physics-probe procedure for choosing the ``D_hidden[k]`` hook layer.

Implements the grandplan §7.6.3 "Physics Emergence Zone" hook-point prior
(arXiv:2602.07050): before committing to a hidden layer as ``D``, run cheap
layerwise linear probes for a few physical quantities (object speed / direction /
contact, here represented as caller-supplied physical targets) and hook ``D`` near
the probe-accuracy peak — *then* run the geometry gate there.

Two corollaries from §7.6.3 are surfaced as explicit warnings:

* late layers are expected to be action-formatted rather than world-informative, so
  a near-output layer with high task-decodability but *low physics decodability* is
  measuring output formatting, not a world model;
* the procedure reports per-layer decodability so a non-monotone (intermediate-peak)
  profile — the "emergence zone" — is visible rather than assumed.

Probes are ridge linear regression (for continuous targets) / ridge-classification
accuracy (for boolean targets), fit on a train split and scored on held-out, using
numpy only. This is a *diagnostic*, not a model: it ranks layers, it does not claim
the probe is the world model.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np


@dataclass
class LayerProbeResult:
    layer_index: int
    target_name: str
    metric: str  # "r2" or "accuracy"
    score: float
    n_train: int
    n_heldout: int


@dataclass
class ProbeSweepResult:
    per_layer: list[LayerProbeResult]
    # Best layer per target, and the overall argmax (physics-decodability peak).
    best_layer_by_target: dict[str, int]
    peak_layer: int
    peak_score: float
    warnings: list[str]


def _standardize_train(
    x: np.ndarray, train_mask: np.ndarray
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    mean = x[train_mask].mean(axis=0)
    std = x[train_mask].std(axis=0)
    std = np.where(std == 0.0, 1.0, std)
    return (x - mean) / std, mean, std


def _ridge_fit(x: np.ndarray, y: np.ndarray, l2: float) -> np.ndarray:
    """Closed-form ridge with an intercept column; returns coefficients."""
    n, d = x.shape
    xa = np.concatenate([np.ones((n, 1)), x], axis=1)
    reg = l2 * np.eye(d + 1)
    reg[0, 0] = 0.0  # do not penalize the intercept
    gram = xa.T @ xa + reg
    return np.linalg.solve(gram, xa.T @ y)


def _r2(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    ss_res = float(np.sum((y_true - y_pred) ** 2))
    ss_tot = float(np.sum((y_true - y_true.mean()) ** 2))
    if ss_tot == 0.0:
        return 0.0
    return 1.0 - ss_res / ss_tot


def probe_layer(
    states: np.ndarray,
    target: np.ndarray,
    train_mask: np.ndarray,
    *,
    is_boolean: bool,
    l2: float = 1.0,
) -> tuple[str, float, int, int]:
    """Probe one layer's ``(N, d)`` states for one target on a train/held-out split.

    Returns ``(metric_name, score, n_train, n_heldout)``. Continuous targets use
    held-out R²; boolean targets use held-out accuracy of a ridge classifier
    thresholded at the train-mean prediction.
    """
    held_mask = ~train_mask
    n_train = int(train_mask.sum())
    n_held = int(held_mask.sum())
    if n_train < 2 or n_held < 1:
        raise ValueError("need >=2 train and >=1 held-out rows to probe")

    xs, _, _ = _standardize_train(np.asarray(states, dtype=np.float64), train_mask)
    if is_boolean:
        y = np.asarray(target, dtype=np.float64).reshape(-1)
        coef = _ridge_fit(xs[train_mask], y[train_mask], l2)
        pred = np.concatenate([np.ones((xs.shape[0], 1)), xs], axis=1) @ coef
        threshold = 0.5  # targets coded as 0/1
        acc = float(((pred[held_mask] >= threshold) == (y[held_mask] >= 0.5)).mean())
        return "accuracy", acc, n_train, n_held
    y = np.asarray(target, dtype=np.float64)
    if y.ndim == 1:
        y = y.reshape(-1, 1)
    coef = _ridge_fit(xs[train_mask], y[train_mask], l2)
    pred = np.concatenate([np.ones((xs.shape[0], 1)), xs], axis=1) @ coef
    return "r2", _r2(y[held_mask], pred[held_mask]), n_train, n_held


def layerwise_physics_probe(
    layer_states: list[np.ndarray],
    physical_targets: dict[str, np.ndarray],
    train_mask: np.ndarray,
    *,
    boolean_targets: set[str] | None = None,
    l2: float = 1.0,
    near_output_fraction: float = 0.25,
) -> ProbeSweepResult:
    """Run the §7.6.3 layerwise physics probe over candidate hidden layers.

    ``layer_states[k]`` is the ``(N, d_k)`` hidden state at candidate layer ``k``;
    ``physical_targets`` maps a physical-quantity name to an ``(N,)`` (boolean) or
    ``(N, m)`` (continuous) target. The ``peak_layer`` is the layer maximizing the
    mean decodability across targets — the recommended ``D_hidden[k]`` hook point.
    """
    if not layer_states:
        raise ValueError("layer_states must be non-empty")
    if not physical_targets:
        raise ValueError("physical_targets must be non-empty")
    boolean_targets = boolean_targets or set()
    train_mask = np.asarray(train_mask, dtype=bool)

    per_layer: list[LayerProbeResult] = []
    # mean score per layer across targets, for the peak.
    layer_mean: list[float] = []
    best_layer_by_target: dict[str, tuple[int, float]] = {}

    for k, states in enumerate(layer_states):
        scores = []
        for name, target in physical_targets.items():
            metric, score, n_tr, n_he = probe_layer(
                states, target, train_mask, is_boolean=name in boolean_targets, l2=l2
            )
            per_layer.append(LayerProbeResult(k, name, metric, score, n_tr, n_he))
            scores.append(score)
            prev = best_layer_by_target.get(name)
            if prev is None or score > prev[1]:
                best_layer_by_target[name] = (k, score)
        layer_mean.append(float(np.mean(scores)))

    peak_layer = int(np.argmax(layer_mean))
    peak_score = layer_mean[peak_layer]

    warnings: list[str] = []
    n_layers = len(layer_states)
    near_output_start = int(np.ceil((1.0 - near_output_fraction) * n_layers))
    if peak_layer >= near_output_start and n_layers > 1:
        warnings.append(
            "physics-decodability peak is in the near-output layers; per §7.6.3 "
            "verify this is not just action formatting (check task-vs-physics "
            "decodability) before hooking D there."
        )
    if n_layers >= 3:
        # Flag a clear intermediate peak (the 'emergence zone').
        if 0 < peak_layer < n_layers - 1 and (
            layer_mean[peak_layer] > layer_mean[0]
            and layer_mean[peak_layer] > layer_mean[-1]
        ):
            warnings.append(
                "physics decodability peaks at an intermediate layer (emergence "
                "zone); prefer this hook over first/last layers."
            )

    return ProbeSweepResult(
        per_layer=per_layer,
        best_layer_by_target={k: v[0] for k, v in best_layer_by_target.items()},
        peak_layer=peak_layer,
        peak_score=peak_score,
        warnings=warnings,
    )
