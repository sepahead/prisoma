"""Leakage-controlled layerwise physics probes for choosing ``D_hidden[k]``.

The hook-point sweep is a diagnostic selection procedure, not evidence that a
representation is a world model.  It uses three caller-frozen, group-disjoint
partitions:

* ``fit`` fits preprocessing and the linear probes;
* ``selection`` chooses the candidate layer; and
* ``evaluation`` reports one final score for that already-chosen layer.

Every layer is projected to the same preregistered number of train-fit principal
components and uses the same fixed ridge penalty.  This controls the most obvious
capacity difference between unequal-width layers.  It does not make representation
comparisons causal, identify a unique semantic layer, or remove all probe-selection
bias.  The independent evaluation partition must therefore remain untouched until
the layer and all probe settings are frozen.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import numpy as np


ProbeSplit = Literal["selection", "evaluation"]


@dataclass(frozen=True)
class LayerProbeResult:
    """One target score for one layer and one named scoring partition."""

    layer_index: int
    target_name: str
    metric: str  # ``r2`` or ``balanced_accuracy_skill``
    score: float
    split: ProbeSplit
    n_fit: int
    n_scored: int


@dataclass(frozen=True)
class ProbeSweepResult:
    """Selection trace plus the untouched evaluation of the selected layer."""

    selection_results: list[LayerProbeResult]
    evaluation_results: list[LayerProbeResult]
    best_layer_by_target: dict[str, int]
    peak_layer: int
    selection_score: float
    evaluation_score: float
    probe_components: int
    target_weights: dict[str, float]
    warnings: list[str]

    @property
    def per_layer(self) -> list[LayerProbeResult]:
        """Compatibility alias for the development-only layer sweep."""

        return self.selection_results

    @property
    def peak_score(self) -> float:
        """Compatibility alias; this is a selection, never evaluation, score."""

        return self.selection_score


def _as_mask(mask: np.ndarray, *, name: str, n_rows: int) -> np.ndarray:
    raw = np.asarray(mask)
    if raw.dtype != np.bool_ or raw.ndim != 1 or raw.shape[0] != n_rows:
        raise ValueError(f"{name} must be a boolean vector with one value per row")
    return raw


def _validate_partitions(
    fit_mask: np.ndarray,
    selection_mask: np.ndarray,
    evaluation_mask: np.ndarray,
    group_ids: np.ndarray,
) -> None:
    masks = {
        "fit": fit_mask,
        "selection": selection_mask,
        "evaluation": evaluation_mask,
    }
    for name, mask in masks.items():
        if int(mask.sum()) < 2:
            raise ValueError(f"{name} must contain at least two rows")
    membership = fit_mask.astype(np.uint8)
    membership += selection_mask.astype(np.uint8)
    membership += evaluation_mask.astype(np.uint8)
    if not np.all(membership == 1):
        raise ValueError(
            "fit, selection, and evaluation masks must be disjoint and cover every row"
        )

    groups = np.asarray(group_ids)
    if groups.ndim != 1 or groups.shape[0] != fit_mask.shape[0]:
        raise ValueError("group_ids must have one value per row")
    # Object values can have surprising equality/hash behavior.  Canonical strings
    # give a deterministic comparison and reject ambiguous non-scalar containers.
    canonical: list[str] = []
    for value in groups.tolist():
        if isinstance(value, (str, int, np.integer)) and not isinstance(value, bool):
            text = str(value)
        else:
            raise ValueError("group_ids values must be strings or non-boolean integers")
        if not text:
            raise ValueError("group_ids values must be nonempty")
        canonical.append(text)
    canonical_groups = np.asarray(canonical, dtype=object)
    group_sets = {
        name: set(canonical_groups[mask].tolist()) for name, mask in masks.items()
    }
    for left, right in (
        ("fit", "selection"),
        ("fit", "evaluation"),
        ("selection", "evaluation"),
    ):
        overlap = group_sets[left] & group_sets[right]
        if overlap:
            example = min(overlap)
            raise ValueError(
                f"group_ids must be disjoint across partitions; {example!r} occurs in "
                f"both {left} and {right}"
            )


def _standardize_and_project(
    x: np.ndarray, fit_mask: np.ndarray, n_components: int
) -> np.ndarray:
    """Apply a fit-only standardization and fixed-rank PCA to every row."""

    fit = x[fit_mask]
    mean = fit.mean(axis=0)
    std = fit.std(axis=0)
    std = np.where(std > 0.0, std, 1.0)
    standardized = (x - mean) / std
    # SVD is fit exclusively on the named fit partition.  Equal component count,
    # rather than raw layer width, fixes the linear probe's input dimension.
    _, _, vt = np.linalg.svd(standardized[fit_mask], full_matrices=False)
    return standardized @ vt[:n_components].T


def _ridge_fit(x: np.ndarray, y: np.ndarray, l2: float) -> np.ndarray:
    """Closed-form ridge with an unpenalized intercept."""

    n_rows, n_features = x.shape
    xa = np.concatenate([np.ones((n_rows, 1)), x], axis=1)
    reg = l2 * np.eye(n_features + 1)
    reg[0, 0] = 0.0
    gram = xa.T @ xa + reg
    try:
        return np.linalg.solve(gram, xa.T @ y)
    except np.linalg.LinAlgError as exc:
        raise ValueError(
            "ridge system is singular; increase the fixed l2 penalty"
        ) from exc


def _predict(x: np.ndarray, coefficients: np.ndarray) -> np.ndarray:
    xa = np.concatenate([np.ones((x.shape[0], 1)), x], axis=1)
    return xa @ coefficients


def _r2(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    """Uniform-average multi-output R², excluding constant target columns."""

    y_true = np.asarray(y_true, dtype=np.float64)
    y_pred = np.asarray(y_pred, dtype=np.float64)
    if y_true.ndim == 1:
        y_true = y_true.reshape(-1, 1)
        y_pred = y_pred.reshape(-1, 1)
    ss_res = np.sum((y_true - y_pred) ** 2, axis=0)
    ss_tot = np.sum((y_true - y_true.mean(axis=0)) ** 2, axis=0)
    valid = ss_tot > 0.0
    if not np.any(valid):
        raise ValueError("continuous target is constant on the scoring partition")
    return float(np.mean(1.0 - ss_res[valid] / ss_tot[valid]))


def _balanced_accuracy_skill(y_true: np.ndarray, y_score: np.ndarray) -> float:
    """Return ``2 * balanced_accuracy - 1`` (chance=0, perfect=1)."""

    truth = np.asarray(y_true, dtype=np.float64).reshape(-1) >= 0.5
    predicted = np.asarray(y_score, dtype=np.float64).reshape(-1) >= 0.5
    positives = truth
    negatives = ~truth
    if not positives.any() or not negatives.any():
        raise ValueError(
            "boolean targets need both classes on every fit/selection/evaluation partition"
        )
    sensitivity = float((predicted[positives] == truth[positives]).mean())
    specificity = float((predicted[negatives] == truth[negatives]).mean())
    return sensitivity + specificity - 1.0


def _validate_target(
    name: str,
    target: np.ndarray,
    *,
    n_rows: int,
    is_boolean: bool,
    masks: tuple[np.ndarray, np.ndarray, np.ndarray],
) -> np.ndarray:
    y = np.asarray(target, dtype=np.float64)
    if y.ndim not in (1, 2) or y.shape[0] != n_rows:
        raise ValueError(f"target {name!r} must be an (N,) or (N,m) array")
    if y.ndim == 2 and y.shape[1] == 0:
        raise ValueError(f"target {name!r} must have at least one column")
    if not np.isfinite(y).all():
        raise ValueError(f"target {name!r} contains a non-finite value")
    if is_boolean:
        if y.ndim != 1 or not np.isin(y, (0.0, 1.0)).all():
            raise ValueError(
                f"boolean target {name!r} must be a binary (N,) 0/1 vector"
            )
        for mask in masks:
            values = y[mask]
            if np.unique(values).size != 2:
                raise ValueError(
                    f"boolean target {name!r} needs both classes in every partition"
                )
    return y


def _fit_and_score(
    projected: np.ndarray,
    target: np.ndarray,
    fit_mask: np.ndarray,
    score_mask: np.ndarray,
    *,
    is_boolean: bool,
    l2: float,
) -> tuple[str, float]:
    coefficients = _ridge_fit(projected[fit_mask], target[fit_mask], l2)
    predictions = _predict(projected[score_mask], coefficients)
    if is_boolean:
        return (
            "balanced_accuracy_skill",
            _balanced_accuracy_skill(target[score_mask], predictions),
        )
    return "r2", _r2(target[score_mask], predictions)


def _normalized_weights(
    target_names: list[str], target_weights: dict[str, float] | None
) -> dict[str, float]:
    if target_weights is None:
        weight = 1.0 / len(target_names)
        return {name: weight for name in target_names}
    if set(target_weights) != set(target_names):
        raise ValueError("target_weights keys must exactly match physical_targets")
    checked: dict[str, float] = {}
    for name in target_names:
        value = target_weights[name]
        if isinstance(value, bool) or not isinstance(value, (int, float, np.number)):
            raise ValueError("target weights must be finite positive numbers")
        weight = float(value)
        if not np.isfinite(weight) or weight <= 0.0:
            raise ValueError("target weights must be finite positive numbers")
        checked[name] = weight
    # Normalize by the largest weight first. Summing raw finite weights can
    # overflow to infinity (for example two weights near f64::MAX), silently
    # turning every normalized weight into zero. The scaled sum is bounded by
    # the number of targets and preserves the intended relative weights.
    scale = max(checked.values())
    scaled = {name: value / scale for name, value in checked.items()}
    if any(not np.isfinite(value) or value <= 0.0 for value in scaled.values()):
        raise ValueError(
            "target weight dynamic range is too large for finite positive normalization"
        )
    total = sum(scaled.values())
    normalized = {name: value / total for name, value in scaled.items()}
    if any(not np.isfinite(value) or value <= 0.0 for value in normalized.values()):
        raise ValueError("target weights could not be normalized safely")
    return normalized


def layerwise_physics_probe(
    layer_states: list[np.ndarray],
    physical_targets: dict[str, np.ndarray],
    fit_mask: np.ndarray,
    *,
    selection_mask: np.ndarray,
    evaluation_mask: np.ndarray,
    group_ids: np.ndarray,
    probe_components: int,
    boolean_targets: set[str] | None = None,
    target_weights: dict[str, float] | None = None,
    l2: float = 1.0,
    near_output_fraction: float = 0.25,
) -> ProbeSweepResult:
    """Select a hook layer without evaluating every candidate on the final holdout.

    ``probe_components``, ``l2``, targets, weights, masks, and groups must be frozen
    before looking at selection or evaluation outcomes.  Selection scores every
    candidate layer.  Evaluation scores only the selected layer, after refitting its
    fixed probe pipeline on the combined fit+selection development data.

    Boolean targets use balanced-accuracy skill (``2*BA-1``), which puts chance at
    zero like R²'s no-skill reference.  A weighted mean across heterogeneous targets
    is still a policy choice, so ``target_weights`` is returned in the result and must
    be preregistered for confirmatory use.
    """

    if not layer_states:
        raise ValueError("layer_states must be non-empty")
    if not physical_targets:
        raise ValueError("physical_targets must be non-empty")
    first = np.asarray(layer_states[0], dtype=np.float64)
    if first.ndim != 2 or first.shape[0] < 6 or first.shape[1] == 0:
        raise ValueError("every layer must be a nonempty (N,d) array with N >= 6")
    n_rows = first.shape[0]
    checked_layers: list[np.ndarray] = []
    for index, states in enumerate(layer_states):
        checked = np.asarray(states, dtype=np.float64)
        if checked.ndim != 2 or checked.shape[0] != n_rows or checked.shape[1] == 0:
            raise ValueError(f"layer {index} must be a nonempty (N,d) array")
        if not np.isfinite(checked).all():
            raise ValueError(f"layer {index} contains a non-finite value")
        checked_layers.append(checked)

    fit = _as_mask(fit_mask, name="fit_mask", n_rows=n_rows)
    selection = _as_mask(selection_mask, name="selection_mask", n_rows=n_rows)
    evaluation = _as_mask(evaluation_mask, name="evaluation_mask", n_rows=n_rows)
    _validate_partitions(fit, selection, evaluation, group_ids)

    if isinstance(probe_components, bool) or not isinstance(
        probe_components, (int, np.integer)
    ):
        raise ValueError("probe_components must be a positive integer")
    probe_components = int(probe_components)
    max_components = min(
        int(fit.sum()) - 1, *(layer.shape[1] for layer in checked_layers)
    )
    if not 1 <= probe_components <= max_components:
        raise ValueError(
            f"probe_components must be in [1, {max_components}] for this fit split"
        )
    if isinstance(l2, bool) or not np.isfinite(l2) or l2 <= 0.0:
        raise ValueError("l2 must be a finite positive preregistered penalty")
    if (
        isinstance(near_output_fraction, bool)
        or not np.isfinite(near_output_fraction)
        or not 0.0 < near_output_fraction <= 1.0
    ):
        raise ValueError("near_output_fraction must be finite and in (0, 1]")

    boolean = set() if boolean_targets is None else set(boolean_targets)
    unknown_boolean = boolean - set(physical_targets)
    if unknown_boolean:
        raise ValueError(
            f"boolean_targets contains unknown names: {sorted(unknown_boolean)!r}"
        )
    names = list(physical_targets)
    weights = _normalized_weights(names, target_weights)
    targets = {
        name: _validate_target(
            name,
            physical_targets[name],
            n_rows=n_rows,
            is_boolean=name in boolean,
            masks=(fit, selection, evaluation),
        )
        for name in names
    }

    selection_results: list[LayerProbeResult] = []
    layer_scores: list[float] = []
    target_layer_scores: dict[str, list[float]] = {name: [] for name in names}
    for layer_index, states in enumerate(checked_layers):
        projected = _standardize_and_project(states, fit, probe_components)
        aggregate = 0.0
        for name in names:
            metric, score = _fit_and_score(
                projected,
                targets[name],
                fit,
                selection,
                is_boolean=name in boolean,
                l2=float(l2),
            )
            if not np.isfinite(score):
                raise ValueError(
                    f"probe score for layer {layer_index}, target {name!r} is non-finite"
                )
            selection_results.append(
                LayerProbeResult(
                    layer_index=layer_index,
                    target_name=name,
                    metric=metric,
                    score=score,
                    split="selection",
                    n_fit=int(fit.sum()),
                    n_scored=int(selection.sum()),
                )
            )
            target_layer_scores[name].append(score)
            aggregate += weights[name] * score
        layer_scores.append(aggregate)

    peak_layer = int(np.argmax(np.asarray(layer_scores)))
    development = fit | selection
    projected_selected = _standardize_and_project(
        checked_layers[peak_layer], development, probe_components
    )
    evaluation_results: list[LayerProbeResult] = []
    evaluation_score = 0.0
    for name in names:
        metric, score = _fit_and_score(
            projected_selected,
            targets[name],
            development,
            evaluation,
            is_boolean=name in boolean,
            l2=float(l2),
        )
        if not np.isfinite(score):
            raise ValueError(f"evaluation score for target {name!r} is non-finite")
        evaluation_results.append(
            LayerProbeResult(
                layer_index=peak_layer,
                target_name=name,
                metric=metric,
                score=score,
                split="evaluation",
                n_fit=int(development.sum()),
                n_scored=int(evaluation.sum()),
            )
        )
        evaluation_score += weights[name] * score

    warnings: list[str] = []
    n_layers = len(checked_layers)
    near_output_start = int(np.ceil((1.0 - near_output_fraction) * n_layers))
    if peak_layer >= near_output_start and n_layers > 1:
        warnings.append(
            "selection chose a near-output layer; test action-formatting alternatives "
            "before interpreting the representation as physical state"
        )
    if n_layers >= 3 and 0 < peak_layer < n_layers - 1:
        if (
            layer_scores[peak_layer] > layer_scores[0]
            and layer_scores[peak_layer] > layer_scores[-1]
        ):
            warnings.append(
                "selection decodability peaks at an intermediate layer; this is a "
                "hook-point diagnostic, not evidence of an emergence mechanism"
            )
    if evaluation_score < 0.0:
        warnings.append(
            "the selected layer failed to beat the no-skill reference on untouched evaluation"
        )

    return ProbeSweepResult(
        selection_results=selection_results,
        evaluation_results=evaluation_results,
        best_layer_by_target={
            name: int(np.argmax(np.asarray(scores)))
            for name, scores in target_layer_scores.items()
        },
        peak_layer=peak_layer,
        selection_score=layer_scores[peak_layer],
        evaluation_score=evaluation_score,
        probe_components=probe_components,
        target_weights=weights,
        warnings=warnings,
    )
