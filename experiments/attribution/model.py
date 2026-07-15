"""A small self-attention model standing in for a transformer VLA's path to a scalar.

This is a deliberately minimal, dependency-light (numpy) transformer block:
``tokens -> embed -> single-head self-attention -> mean-pool -> linear head -> scalar``.
It exists so the attribution + ranking-sensitivity machinery (and run-log emission) can
be exercised on a *real* forward pass with a declared scalar target, without GPUs or
a multi-GB VLA checkpoint. For production VLAs, swap this for the real model and use
the LXT/AttnLRP library (`rachtibat/LRP-eXplains-Transformers`) — the validation
check, provenance, and run-log contract in this package are model-agnostic.

Linear layers are bias-free so that epsilon-LRP relevance is (approximately)
conserved, which the tests assert. Weights are deterministic given a seed.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

import numpy as np

MAX_INPUT_VALUES = 1_024
MAX_INPUT_DIMENSION = 1_024
MAX_MODEL_DIMENSION = 512
MAX_MODEL_PARAMETERS = 2_000_000
MAX_FORWARD_MULTIPLY_ADDS = 50_000_000
MAX_SEED = 2**64 - 1


def _exact_bounded_int(value: object, field: str, *, minimum: int, maximum: int) -> int:
    if isinstance(value, bool) or not isinstance(value, (int, np.integer)):
        raise ValueError(f"{field} must be an integer")
    parsed = int(value)
    if not minimum <= parsed <= maximum:
        raise ValueError(f"{field} must be between {minimum} and {maximum}")
    return parsed


def _require_finite(array: np.ndarray, context: str) -> np.ndarray:
    if not np.all(np.isfinite(array)):
        raise ValueError(f"{context} produced non-finite values")
    return array


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
        d_in = _exact_bounded_int(d_in, "d_in", minimum=1, maximum=MAX_INPUT_DIMENSION)
        d_model = _exact_bounded_int(
            d_model, "d_model", minimum=1, maximum=MAX_MODEL_DIMENSION
        )
        seed = _exact_bounded_int(seed, "seed", minimum=0, maximum=MAX_SEED)
        parameter_count = d_in * d_model + 4 * d_model * d_model + d_model
        if parameter_count > MAX_MODEL_PARAMETERS:
            raise ValueError(
                "reference-model parameter count exceeds the "
                f"{MAX_MODEL_PARAMETERS}-value resource budget"
            )

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

    def validate_input(self, x: object) -> np.ndarray:
        """Return a finite ``float64`` input inside the reference-model budget."""

        try:
            raw = np.asarray(x)
        except (TypeError, ValueError) as error:
            raise ValueError("x must be a rectangular numeric array") from error
        if raw.ndim != 2 or raw.shape[1] != self.d_in:
            raise ValueError(f"x must be (T, {self.d_in}), got {raw.shape}")
        if raw.shape[0] == 0:
            raise ValueError("x must contain at least one token")
        if raw.size > MAX_INPUT_VALUES:
            raise ValueError(
                f"x must contain at most {MAX_INPUT_VALUES} values, got {raw.size}"
            )
        forward_work = self.estimated_forward_multiply_adds(raw.shape[0])
        if forward_work > MAX_FORWARD_MULTIPLY_ADDS:
            raise ValueError(
                "reference-model forward pass exceeds the "
                f"{MAX_FORWARD_MULTIPLY_ADDS}-multiply-add resource budget"
            )
        if np.issubdtype(raw.dtype, np.bool_) or not np.issubdtype(
            raw.dtype, np.number
        ):
            raise ValueError("x must contain real numeric values")
        if np.issubdtype(raw.dtype, np.complexfloating):
            raise ValueError("x must contain real numeric values")
        try:
            validated = np.asarray(raw, dtype=np.float64)
        except (TypeError, ValueError, OverflowError) as error:
            raise ValueError("x must be representable as float64") from error
        if not np.all(np.isfinite(validated)):
            raise ValueError("x must contain only finite values")
        return validated

    def estimated_forward_multiply_adds(self, token_count: object) -> int:
        """Conservative dense-matmul work estimate for one forward pass."""

        token_count = _exact_bounded_int(
            token_count, "token_count", minimum=1, maximum=MAX_INPUT_VALUES
        )
        return (
            token_count * self.d_in * self.d_model
            + 4 * token_count * self.d_model * self.d_model
            + 2 * token_count * token_count * self.d_model
            + self.d_model
        )

    def validate_parameters(self) -> None:
        """Reject mutated model state that violates shape or finiteness invariants."""

        d_in = _exact_bounded_int(
            self.d_in, "d_in", minimum=1, maximum=MAX_INPUT_DIMENSION
        )
        d_model = _exact_bounded_int(
            self.d_model, "d_model", minimum=1, maximum=MAX_MODEL_DIMENSION
        )
        parameter_count = d_in * d_model + 4 * d_model * d_model + d_model
        if parameter_count > MAX_MODEL_PARAMETERS:
            raise ValueError(
                "reference-model parameter count exceeds the "
                f"{MAX_MODEL_PARAMETERS}-value resource budget"
            )
        expected = (
            ("w_embed", (d_in, d_model)),
            ("w_q", (d_model, d_model)),
            ("w_k", (d_model, d_model)),
            ("w_v", (d_model, d_model)),
            ("w_o", (d_model, d_model)),
            ("w_head", (d_model, 1)),
        )
        for name, shape in expected:
            value = getattr(self, name, None)
            if type(value) is not np.ndarray or value.shape != shape:
                raise ValueError(f"model parameter {name} must have shape {shape}")
            if not np.issubdtype(value.dtype, np.number) or np.issubdtype(
                value.dtype, np.complexfloating
            ):
                raise ValueError(f"model parameter {name} must be a real numeric array")
            if not np.all(np.isfinite(value)):
                raise ValueError(
                    f"model parameter {name} must contain only finite values"
                )

    def forward(self, x: np.ndarray) -> float:
        return self.forward_cache(x).target

    def forward_cache(self, x: np.ndarray) -> ForwardCache:
        x = self.validate_input(x)
        self.validate_parameters()
        try:
            with np.errstate(over="raise", divide="raise", invalid="raise"):
                embedded = _require_finite(
                    x @ self.w_embed, "input embedding"
                )  # (T, d_model)
                q = _require_finite(embedded @ self.w_q, "query projection")
                k = _require_finite(embedded @ self.w_k, "key projection")
                v = _require_finite(embedded @ self.w_v, "value projection")
                scores = _require_finite(
                    (q @ k.T) / math.sqrt(self.d_model), "attention scores"
                )
                attn_weights = _require_finite(
                    _softmax(scores, axis=-1), "attention softmax"
                )  # (T, T)
                attn_out = _require_finite(
                    attn_weights @ v, "attention output"
                )  # (T, d_model)
                projected = _require_finite(attn_out @ self.w_o, "output projection")
                pooled = _require_finite(
                    projected.mean(axis=0), "token pooling"
                )  # (d_model,)
                target = float((pooled @ self.w_head)[0])
        except FloatingPointError as error:
            raise ValueError("reference-model computation overflowed") from error
        if not math.isfinite(target):
            raise ValueError("reference-model target must be finite")
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
