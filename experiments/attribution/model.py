"""A small self-attention model standing in for a transformer VLA's path to a scalar.

This is a deliberately minimal, dependency-light (numpy) transformer block:
``tokens -> embed -> single-head self-attention -> mean-pool -> linear head -> scalar``.
It exists so the attribution + faithfulness machinery (and the run-log emission) can
be exercised on a *real* forward pass with a declared scalar target, without GPUs or
a multi-GB VLA checkpoint. For production VLAs, swap this for the real model and use
the LXT/AttnLRP library (`rachtibat/LRP-eXplains-Transformers`) — the faithfulness
check, provenance, and run-log contract in this package are model-agnostic.

Linear layers are bias-free so that epsilon-LRP relevance is (approximately)
conserved, which the tests assert. Weights are deterministic given a seed.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np


def _softmax(z: np.ndarray, axis: int = -1) -> np.ndarray:
    z = z - z.max(axis=axis, keepdims=True)
    e = np.exp(z)
    return e / e.sum(axis=axis, keepdims=True)


@dataclass
class ForwardCache:
    """Intermediate activations captured for relevance propagation."""

    x: np.ndarray  # (T, d_in) input features
    embedded: np.ndarray  # (T, d_model)
    attn_weights: np.ndarray  # (T, T) softmax attention
    values: np.ndarray  # (T, d_model)
    attn_out: np.ndarray  # (T, d_model) = A @ V
    projected: np.ndarray  # (T, d_model) after output projection
    pooled: np.ndarray  # (d_model,) mean over tokens
    target: float  # scalar head output


class SmallTransformer:
    """Single-head self-attention block + linear head producing a scalar target."""

    def __init__(self, d_in: int, d_model: int, *, seed: int = 0) -> None:
        rng = np.random.default_rng(seed)
        scale = 1.0 / np.sqrt(d_model)
        self.d_in = d_in
        self.d_model = d_model
        self.w_embed = rng.standard_normal((d_in, d_model)) * scale
        self.w_q = rng.standard_normal((d_model, d_model)) * scale
        self.w_k = rng.standard_normal((d_model, d_model)) * scale
        self.w_v = rng.standard_normal((d_model, d_model)) * scale
        self.w_o = rng.standard_normal((d_model, d_model)) * scale
        self.w_head = rng.standard_normal((d_model, 1)) * scale

    def forward(self, x: np.ndarray) -> float:
        return self.forward_cache(x).target

    def forward_cache(self, x: np.ndarray) -> ForwardCache:
        x = np.asarray(x, dtype=np.float64)
        if x.ndim != 2 or x.shape[1] != self.d_in:
            raise ValueError(f"x must be (T, {self.d_in}), got {x.shape}")
        embedded = x @ self.w_embed  # (T, d_model)
        q = embedded @ self.w_q
        k = embedded @ self.w_k
        v = embedded @ self.w_v
        scores = (q @ k.T) / np.sqrt(self.d_model)
        attn_weights = _softmax(scores, axis=-1)  # (T, T)
        attn_out = attn_weights @ v  # (T, d_model)
        projected = attn_out @ self.w_o
        pooled = projected.mean(axis=0)  # (d_model,)
        target = float((pooled @ self.w_head)[0])
        return ForwardCache(
            x=x,
            embedded=embedded,
            attn_weights=attn_weights,
            values=v,
            attn_out=attn_out,
            projected=projected,
            pooled=pooled,
            target=target,
        )
