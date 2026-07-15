"""Attribution methods: epsilon-LRP (AttnLRP-style) and gradient x input.

``lrp_epsilon`` implements layer-wise relevance propagation through
:class:`~experiments.attribution.model.SmallTransformer` using the epsilon rule for
linear layers and the **detached-softmax** rule for attention (attention weights are
treated as constant gates and relevance is routed through the value path). This is
the core simplification AttnLRP uses for the softmax-attention bilinear form; for
production transformers use the LXT library, which implements the full attention
rules. The relevance implementation must be validated independently; the held-out
deletion ranking-sensitivity contract remains applicable.

``grad_times_input`` is the standard gradient x input attribution; the gradient is
computed by central finite differences so the implementation stays model-agnostic
and dependency-light (numpy only), which also makes it self-checking.

Both return a ``(T, d_in)`` relevance array aligned with the input tokens/features.
"""

from __future__ import annotations

import math

import numpy as np

from .model import ForwardCache, SmallTransformer


def _positive_finite(value: object, field: str) -> float:
    if isinstance(value, bool) or not isinstance(
        value, (float, int, np.floating, np.integer)
    ):
        raise ValueError(f"{field} must be a finite positive number")
    parsed = float(value)
    if not math.isfinite(parsed) or parsed <= 0.0:
        raise ValueError(f"{field} must be a finite positive number")
    if not math.isfinite(2.0 * parsed):
        raise ValueError(f"{field} is too large for a central difference")
    return parsed


def _finite(array: np.ndarray, context: str) -> np.ndarray:
    if not np.all(np.isfinite(array)):
        raise ValueError(f"{context} produced non-finite relevance values")
    return array


def _eps_signed(z: np.ndarray, eps: float) -> np.ndarray:
    """``z + eps*sign(z)`` with sign(0) := +1, the epsilon-LRP stabilizer."""
    sign = np.where(z >= 0.0, 1.0, -1.0)
    return z + eps * sign


def _lrp_linear(
    inputs: np.ndarray, weight: np.ndarray, out_relevance: np.ndarray, eps: float
) -> np.ndarray:
    """Epsilon-LRP through a bias-free linear map ``y = inputs @ weight``.

    ``inputs`` is ``(..., d_in)``, ``weight`` is ``(d_in, d_out)``, ``out_relevance``
    is ``(..., d_out)``. Returns input relevance ``(..., d_in)``.
    """
    try:
        with np.errstate(over="raise", divide="raise", invalid="raise"):
            z = _finite(
                inputs @ weight, "epsilon-LRP linear projection"
            )  # pre-activation outputs (..., d_out)
            s = _finite(
                out_relevance / _eps_signed(z, eps),
                "epsilon-LRP stabilized division",
            )  # (..., d_out)
            c = _finite(s @ weight.T, "epsilon-LRP backward projection")  # (..., d_in)
            return _finite(inputs * c, "epsilon-LRP linear propagation")
    except FloatingPointError as error:
        raise ValueError("epsilon-LRP numerical operation overflowed") from error


def lrp_epsilon(
    model: SmallTransformer, x: np.ndarray, *, eps: float = 1e-6
) -> np.ndarray:
    """Epsilon-LRP / AttnLRP-style relevance of the scalar target onto ``x``."""
    if not isinstance(model, SmallTransformer):
        raise ValueError("model must be a SmallTransformer")
    eps = _positive_finite(eps, "eps")
    cache: ForwardCache = model.forward_cache(x)
    t = cache.x.shape[0]

    # Output relevance := the scalar target itself.
    r_target = np.array([cache.target], dtype=np.float64)  # (1,)

    # Head: pooled (d_model,) @ w_head (d_model,1) -> target.
    r_pooled = _lrp_linear(cache.pooled, model.w_head, r_target, eps)  # (d_model,)

    # Mean pool: distribute each feature's relevance over tokens proportionally to
    # the per-token activation (epsilon rule for a sum).
    col_sum = cache.projected.sum(axis=0)  # (d_model,)
    try:
        with np.errstate(over="raise", divide="raise", invalid="raise"):
            weights = _finite(
                cache.projected / _eps_signed(col_sum, eps),
                "epsilon-LRP mean-pool division",
            )  # (T, d_model)
            r_projected = _finite(
                weights * r_pooled[None, :], "epsilon-LRP mean-pool propagation"
            )  # (T, d_model)
    except FloatingPointError as error:
        raise ValueError("epsilon-LRP mean-pool propagation overflowed") from error

    # Output projection: projected[t] = attn_out[t] @ w_o (per token).
    r_attn_out = _lrp_linear(
        cache.attn_out, model.w_o, r_projected, eps
    )  # (T, d_model)

    # Attention A @ V with A detached: attn_out[t,i] = sum_s A[t,s] v[s,i].
    # Route r_attn_out[t,i] to value rows proportional to A[t,s] v[s,i].
    try:
        with np.errstate(over="raise", divide="raise", invalid="raise"):
            denom = _eps_signed(cache.attn_out, eps)  # (T, d_model)
            s_mat = _finite(
                r_attn_out / denom, "epsilon-LRP attention division"
            )  # (T, d_model)
            # contribution to value s, feature i: sum_t A[t,s] v[s,i] s_mat[t,i]
            # = v[s,i] * sum_t A[t,s] s_mat[t,i]
            r_values = _finite(
                cache.values * (cache.attn_weights.T @ s_mat),
                "epsilon-LRP value propagation",
            )
    except FloatingPointError as error:
        raise ValueError("epsilon-LRP attention propagation overflowed") from error

    # Value projection: v = embedded @ w_v.
    r_embedded = _lrp_linear(cache.embedded, model.w_v, r_values, eps)  # (T, d_model)

    # Embedding: embedded = x @ w_embed.
    r_x = _lrp_linear(cache.x, model.w_embed, r_embedded, eps)  # (T, d_in)
    if r_x.shape != (t, model.d_in):
        raise RuntimeError(
            "epsilon-LRP produced an internal relevance shape inconsistent with "
            "the model input contract"
        )
    return _finite(r_x, "epsilon-LRP")


def finite_difference_gradient(
    model: SmallTransformer, x: np.ndarray, *, h: float = 1e-5
) -> np.ndarray:
    """Central finite-difference gradient ``d target / d x`` of shape ``(T, d_in)``."""
    if not isinstance(model, SmallTransformer):
        raise ValueError("model must be a SmallTransformer")
    h = _positive_finite(h, "h")
    x = model.validate_input(x)
    try:
        with np.errstate(over="raise", invalid="raise"):
            plus = x + h
            minus = x - h
    except FloatingPointError as error:
        raise ValueError("h perturbs x outside the finite float64 range") from error
    if not np.all(np.isfinite(plus)) or not np.all(np.isfinite(minus)):
        raise ValueError("h perturbs x outside the finite float64 range")
    if np.any(plus == x) or np.any(minus == x):
        raise ValueError("h is too small to perturb every input coordinate")
    grad = np.zeros_like(x)
    for t in range(x.shape[0]):
        for i in range(x.shape[1]):
            xp = x.copy()
            xm = x.copy()
            xp[t, i] += h
            xm[t, i] -= h
            numerator = model.forward(xp) - model.forward(xm)
            if not math.isfinite(numerator):
                raise ValueError("finite-difference numerator is non-finite")
            gradient = numerator / (2.0 * h)
            if not math.isfinite(gradient):
                raise ValueError("finite-difference gradient is non-finite")
            grad[t, i] = gradient
    return _finite(grad, "finite-difference gradient")


def grad_times_input(
    model: SmallTransformer, x: np.ndarray, *, h: float = 1e-5
) -> np.ndarray:
    """Gradient x input attribution, gradient via central finite differences."""
    if not isinstance(model, SmallTransformer):
        raise ValueError("model must be a SmallTransformer")
    x = model.validate_input(x)
    try:
        with np.errstate(over="raise", invalid="raise"):
            return _finite(
                x * finite_difference_gradient(model, x, h=h),
                "gradient-times-input",
            )
    except FloatingPointError as error:
        raise ValueError("gradient-times-input overflowed") from error
