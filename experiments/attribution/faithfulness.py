"""Faithfulness check for an attribution: deletion AOPC vs a random control.

An attribution heatmap is only trustworthy if removing the features it calls
important actually changes the model output more than removing random features.
This module measures that directly (the deletion / AOPC protocol, in a sign-robust
form suitable for a signed regression target rather than a class probability):

1. rank input features by attribution magnitude;
2. progressively ablate the top-k features (replace with a baseline value) and
   record the target output after each removal;
3. AOPC = mean over k of ``|f(x) - f(x_ablated_topk)|`` — how much removing the
   most-important features perturbs the output. A faithful attribution has a
   *large* AOPC because it ablates the features that actually move the output.

The control repeats step 2 with **random** rankings (averaged over seeds). The
attribution **passes** only if its AOPC exceeds the random-control mean by the
caller's ``margin`` *plus* 3 standard errors of that mean — a win that is significant
relative to the control's sampling noise, not a floating-point one — so an
uninformative map (e.g. a constant attribution, whose ranking is arbitrary) reliably
fails. This is exactly the guard §14.7.1 requires before any PID-vs-attribution
comparison: a probe that fails its own faithfulness check cannot falsify or
corroborate a PID claim.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

import numpy as np

PredictFn = Callable[[np.ndarray], float]


@dataclass
class FaithfulnessResult:
    method_aopc: float
    random_aopc: float
    random_std: float
    margin: float
    passed: bool
    n_steps: int
    deletion_curve: list[float]
    random_curve: list[float]


def _deletion_curve(
    predict: PredictFn,
    x: np.ndarray,
    order: np.ndarray,
    baseline: np.ndarray,
    n_steps: int,
) -> list[float]:
    """Target output after ablating the first 1..n_steps features in ``order``."""
    flat = x.reshape(-1).copy()
    base_flat = baseline.reshape(-1)
    curve = []
    step_size = max(1, len(order) // n_steps)
    pos = 0
    for _ in range(n_steps):
        end = min(pos + step_size, len(order))
        for idx in order[pos:end]:
            flat[idx] = base_flat[idx]
        pos = end
        curve.append(predict(flat.reshape(x.shape)))
        if pos >= len(order):
            break
    return curve


def faithfulness_check(
    predict: PredictFn,
    x: np.ndarray,
    attribution: np.ndarray,
    *,
    baseline: np.ndarray | None = None,
    n_steps: int = 10,
    n_random: int = 8,
    margin: float = 0.0,
    seed: int = 0,
) -> FaithfulnessResult:
    """Run the deletion-AOPC faithfulness check for ``attribution`` on ``x``.

    ``baseline`` defaults to all zeros (a neutral reference). AOPC is computed as
    the mean drop ``f(x) - f(x_ablated)`` over deletion steps; the random control
    averages over ``n_random`` random orderings.
    """
    x = np.asarray(x, dtype=np.float64)
    attribution = np.asarray(attribution, dtype=np.float64)
    if attribution.shape != x.shape:
        raise ValueError("attribution shape must match x")
    baseline = np.zeros_like(x) if baseline is None else np.asarray(baseline, dtype=np.float64)

    f0 = predict(x)
    rng = np.random.default_rng(seed)

    # Random control: average the deletion AOPC over n_random random orderings.
    random_aopcs = []
    random_curve_acc = None
    for _ in range(n_random):
        perm = rng.permutation(x.size)
        curve = _deletion_curve(predict, x, perm, baseline, n_steps)
        random_aopcs.append(np.mean([abs(f0 - f) for f in curve]))
        arr = np.abs(f0 - np.asarray(curve))
        random_curve_acc = arr if random_curve_acc is None else random_curve_acc + arr
    random_aopc = float(np.mean(random_aopcs))
    random_std = float(np.std(random_aopcs))
    random_curve = (random_curve_acc / n_random).tolist() if random_curve_acc is not None else []

    # Method ordering: most-important-first by |attribution|, with ties broken at
    # random (seeded). Tied features carry no ranking signal, so when the attribution
    # has ties we average the method AOPC over n_random tie-break draws — a degenerate
    # attribution (e.g. constant) then collapses onto the random control instead of
    # "winning" on an arbitrary argsort tie order.
    mags = np.abs(attribution).reshape(-1)
    has_ties = np.unique(mags).size < mags.size
    n_order = n_random if has_ties else 1
    method_aopcs = []
    method_curve = None
    for _ in range(n_order):
        jitter = rng.random(mags.size)
        # lexsort uses the LAST key as primary: sort by -mags (desc), break ties by jitter.
        order = np.lexsort((jitter, -mags))
        curve = _deletion_curve(predict, x, order, baseline, n_steps)
        method_aopcs.append(np.mean([abs(f0 - f) for f in curve]))
        if method_curve is None:
            method_curve = curve  # representative curve for reporting
    method_aopc = float(np.mean(method_aopcs))

    # Pass requires beating the random-control MEAN by the caller's `margin` plus a
    # statistical floor of 3 standard errors of that mean (sem = std / sqrt(n_random)):
    # i.e. the win must be significant relative to the control's sampling noise, not a
    # float-epsilon artifact. Combined with the tie-break averaging above, an
    # uninformative attribution (which collapses onto the control mean) fails reliably,
    # while a genuinely faithful map (gap >> sem) still passes.
    sem = random_std / np.sqrt(max(n_random, 1))
    threshold = random_aopc + margin + 3.0 * sem
    passed = method_aopc > threshold
    return FaithfulnessResult(
        method_aopc=method_aopc,
        random_aopc=random_aopc,
        random_std=random_std,
        margin=margin,
        passed=passed,
        n_steps=len(method_curve),
        deletion_curve=method_curve,
        random_curve=random_curve,
    )
