"""Attribution methods: epsilon-LRP (AttnLRP-style) and gradient x input.

``lrp_epsilon`` implements layer-wise relevance propagation through
:class:`~experiments.attribution.model.SmallTransformer` using the epsilon rule for
linear layers and the **detached-softmax** rule for attention (attention weights are
treated as constant gates and relevance is routed through the value path). This is
the core simplification AttnLRP uses for the softmax-attention bilinear form; for
production transformers use the LXT library, which implements the full attention
rules — the relevance contract and faithfulness check here are identical.

``grad_times_input`` is the standard gradient x input attribution; the gradient is
computed by central finite differences so the implementation stays model-agnostic
and dependency-light (numpy only), which also makes it self-checking.

Both return a ``(T, d_in)`` relevance array aligned with the input tokens/features.
"""

from __future__ import annotations

import numpy as np

from .model import ForwardCache, SmallTransformer


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
    z = inputs @ weight  # pre-activation outputs (..., d_out)
    s = out_relevance / _eps_signed(z, eps)  # (..., d_out)
    c = s @ weight.T  # (..., d_in)
    return inputs * c


def lrp_epsilon(
    model: SmallTransformer, x: np.ndarray, *, eps: float = 1e-6
) -> np.ndarray:
    """Epsilon-LRP / AttnLRP-style relevance of the scalar target onto ``x``."""
    cache: ForwardCache = model.forward_cache(x)
    t = cache.x.shape[0]

    # Output relevance := the scalar target itself.
    r_target = np.array([cache.target], dtype=np.float64)  # (1,)

    # Head: pooled (d_model,) @ w_head (d_model,1) -> target.
    r_pooled = _lrp_linear(cache.pooled, model.w_head, r_target, eps)  # (d_model,)

    # Mean pool: distribute each feature's relevance over tokens proportionally to
    # the per-token activation (epsilon rule for a sum).
    col_sum = cache.projected.sum(axis=0)  # (d_model,)
    weights = cache.projected / _eps_signed(col_sum, eps)  # (T, d_model)
    r_projected = weights * r_pooled[None, :]  # (T, d_model)

    # Output projection: projected[t] = attn_out[t] @ w_o (per token).
    r_attn_out = _lrp_linear(cache.attn_out, model.w_o, r_projected, eps)  # (T, d_model)

    # Attention A @ V with A detached: attn_out[t,i] = sum_s A[t,s] v[s,i].
    # Route r_attn_out[t,i] to value rows proportional to A[t,s] v[s,i].
    r_values = np.zeros_like(cache.values)  # (T, d_model)
    denom = _eps_signed(cache.attn_out, eps)  # (T, d_model)
    s_mat = r_attn_out / denom  # (T, d_model)
    # contribution to value s, feature i: sum_t A[t,s] v[s,i] s_mat[t,i]
    # = v[s,i] * sum_t A[t,s] s_mat[t,i]
    r_values = cache.values * (cache.attn_weights.T @ s_mat)

    # Value projection: v = embedded @ w_v.
    r_embedded = _lrp_linear(cache.embedded, model.w_v, r_values, eps)  # (T, d_model)

    # Embedding: embedded = x @ w_embed.
    r_x = _lrp_linear(cache.x, model.w_embed, r_embedded, eps)  # (T, d_in)
    assert r_x.shape == (t, model.d_in)
    return r_x


def finite_difference_gradient(
    model: SmallTransformer, x: np.ndarray, *, h: float = 1e-5
) -> np.ndarray:
    """Central finite-difference gradient ``d target / d x`` of shape ``(T, d_in)``."""
    x = np.asarray(x, dtype=np.float64)
    grad = np.zeros_like(x)
    for t in range(x.shape[0]):
        for i in range(x.shape[1]):
            xp = x.copy()
            xm = x.copy()
            xp[t, i] += h
            xm[t, i] -= h
            grad[t, i] = (model.forward(xp) - model.forward(xm)) / (2.0 * h)
    return grad


def grad_times_input(
    model: SmallTransformer, x: np.ndarray, *, h: float = 1e-5
) -> np.ndarray:
    """Gradient x input attribution, gradient via central finite differences."""
    x = np.asarray(x, dtype=np.float64)
    return x * finite_difference_gradient(model, x, h=h)
