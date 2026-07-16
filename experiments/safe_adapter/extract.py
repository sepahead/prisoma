"""Derive ``(V, L, D, A)`` per-step features from a SAFE rollout, with provenance.

Honesty about what the released SAFE rollouts actually contain
-------------------------------------------------------------

The SAFE rollout datasets (``vla-safe/SAFE``) cleanly provide, per step:

* ``A`` — the action vector (e.g. OpenVLA's 7-D ``dx,dy,dz,droll,dpitch,dyaw,dgripper``);
* ``D`` — a declared policy-backbone hidden-state site (``hidden_states``), kept
  semantically neutral until architecture evidence and held-out probes justify a
  stronger world/plan interpretation;
* the episode success/failure outcome, task id, and episode id.

They do **not** ship a clean pre-fusion vision embedding ``V`` or a text embedding
``L`` as separate tensors. This module is explicit about that: every variable it
produces carries a provenance string, and it refuses to fabricate ``V``. The
supported, non-fabricated sources are:

* **token slicing** — if the *raw* per-token hidden states ``(T, n_token, d)`` and
  the token-group index ranges are available, slice declared vision / language /
  state groups. Those names are provenance claims only until token-mask ancestry
  and held-out probes validate their semantics;
* **explicit vision features** — a separately extracted ``(T, d_v)`` array (e.g.
  from running a vision encoder over the rollout frames);
* **text featurization** — a deterministic hashing featurization of the
  instruction text for ``L`` (a transparent *proxy*, clearly labelled as such; a
  real sentence encoder is preferred and can be supplied as ``language_features``).

If a variable can only be a proxy and the caller has not opted in, extraction
raises rather than silently emitting placeholder data.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass

import numpy as np

MAX_TEXT_HASH_DIM = 65_536
MAX_TEXT_HASH_CHARS = 16_384


@dataclass
class VariableSpec:
    """How to source one of V/L/D for a rollout.

    ``mode`` is one of:
      * ``"hidden_pool"`` — pool the (raw or pooled) hidden states to ``(T, d)``;
      * ``"token_slice"`` — slice a named token group from raw per-token hidden
        states (requires ``token_group``);
      * ``"explicit"`` — use a separately supplied ``(T, d)`` feature array;
      * ``"text_hash"`` — deterministic hashing featurization of the instruction
        (proxy; ``L`` only).
    """

    mode: str
    token_group: str | None = None
    dim: int | None = None  # required for text_hash


def _stable_token_mean(hidden_states: np.ndarray) -> np.ndarray:
    """Mean-pool token rows without overflowing on extreme finite values."""
    # Preserve the historical NumPy reduction bit-for-bit for ordinary inputs.
    # Only take the scaled fallback when that reduction actually overflows.
    with np.errstate(over="ignore", invalid="ignore"):
        pooled = hidden_states.mean(axis=1)
    if np.isfinite(pooled).all():
        return pooled

    scale = np.max(np.abs(hidden_states), axis=1, keepdims=True)
    normalized = np.divide(
        hidden_states,
        scale,
        out=np.zeros_like(hidden_states),
        where=scale != 0.0,
    )
    pooled = normalized.mean(axis=1) * scale[:, 0, :]
    if not np.isfinite(pooled).all():
        raise ValueError("hidden-state pooling produced a non-finite value")
    return pooled


def pool_tokens(hidden_states: np.ndarray, *, reduction: str = "mean") -> np.ndarray:
    """Collapse raw ``(T, n_token, d)`` hidden states to ``(T, d)``.

    Accepts already-pooled ``(T, d)`` input unchanged. ``reduction`` is ``"mean"``
    or ``"last"`` (last token), mirroring common VLA pooling choices.
    """
    hs = np.asarray(hidden_states, dtype=np.float64)
    if not np.isfinite(hs).all():
        raise ValueError("hidden_states contains a non-finite value")
    if hs.ndim == 2:
        return hs
    if hs.ndim != 3:
        raise ValueError(f"hidden_states must be 2-D or 3-D, got shape {hs.shape}")
    if reduction == "mean":
        return _stable_token_mean(hs)
    if reduction == "last":
        return hs[:, -1, :]
    raise ValueError(f"unknown reduction: {reduction!r}")


def slice_token_group(
    hidden_states: np.ndarray, token_group: tuple[int, int]
) -> np.ndarray:
    """Mean-pool a contiguous token-index range from raw ``(T, n_token, d)`` states."""
    hs = np.asarray(hidden_states, dtype=np.float64)
    if not np.isfinite(hs).all():
        raise ValueError("hidden_states contains a non-finite value")
    if hs.ndim != 3:
        raise ValueError(
            "token slicing requires raw per-token hidden states (T, n_token, d); "
            f"got shape {hs.shape}"
        )
    start, end = token_group
    if not (0 <= start < end <= hs.shape[1]):
        raise ValueError(
            f"token group {token_group} out of range for n_token={hs.shape[1]}"
        )
    return _stable_token_mean(hs[:, start:end, :])


def text_hash_features(text: str, dim: int) -> np.ndarray:
    """Deterministic hashing featurization of ``text`` into a length-``dim`` vector.

    This is a transparent **proxy** for a real sentence encoder: it hashes word and
    character-trigram tokens into ``dim`` buckets (signed hashing trick) and
    L2-normalizes. It is reproducible and language-agnostic, but it is *not* a
    learned semantic embedding — prefer supplying real ``language_features`` when a
    text encoder is available.
    """
    if not isinstance(text, str) or len(text) > MAX_TEXT_HASH_CHARS:
        raise ValueError(
            f"text must be a string of at most {MAX_TEXT_HASH_CHARS} characters"
        )
    if (
        isinstance(dim, bool)
        or not isinstance(dim, int)
        or dim <= 0
        or dim > MAX_TEXT_HASH_DIM
    ):
        raise ValueError(f"dim must be in 1..{MAX_TEXT_HASH_DIM}")
    vec = np.zeros(dim, dtype=np.float64)
    tokens = list(text.lower().split())
    trigrams = [text[i : i + 3] for i in range(max(0, len(text) - 2))]
    for token in tokens + trigrams:
        # SHA-1 is retained solely as the versioned, deterministic bucket mapping for
        # this non-security hashing trick; it is never used for integrity or trust.
        digest = hashlib.sha1(token.encode("utf-8"), usedforsecurity=False).digest()
        bucket = int.from_bytes(digest[:4], "big") % dim
        sign = 1.0 if digest[4] & 1 else -1.0
        vec[bucket] += sign
    norm = np.linalg.norm(vec)
    if norm > 0:
        vec /= norm
    return vec


def _broadcast_steps(vec: np.ndarray, n_steps: int) -> np.ndarray:
    """Repeat a single per-rollout vector across ``n_steps`` rows."""
    return np.tile(vec.reshape(1, -1), (n_steps, 1))


def resolve_variable(
    spec: VariableSpec,
    *,
    name: str,
    hidden_states: np.ndarray,
    n_steps: int,
    instruction: str,
    token_groups: dict[str, tuple[int, int]] | None,
    explicit_features: np.ndarray | None,
) -> tuple[np.ndarray, str]:
    """Resolve one variable to a ``(T, d)`` array plus a provenance string."""
    if spec.mode == "explicit":
        if explicit_features is None:
            raise ValueError(f"{name}: mode 'explicit' requires a feature array")
        feats = np.asarray(explicit_features, dtype=np.float64)
        if feats.ndim != 2 or feats.shape[0] != n_steps:
            raise ValueError(
                f"{name}: explicit features must be (T={n_steps}, d), got {feats.shape}"
            )
        if feats.shape[1] == 0 or not np.isfinite(feats).all():
            raise ValueError(f"{name}: explicit features must be nonempty and finite")
        return feats, "explicit_features"
    if spec.mode == "hidden_pool":
        resolved = pool_tokens(hidden_states)
        if resolved.shape[0] != n_steps or resolved.shape[1] == 0:
            raise ValueError(
                f"{name}: hidden-state pool must be (T={n_steps}, d>0), got {resolved.shape}"
            )
        return resolved, "hidden_state_pool"
    if spec.mode == "token_slice":
        if not token_groups or spec.token_group not in token_groups:
            raise ValueError(
                f"{name}: mode 'token_slice' requires token_groups[{spec.token_group!r}]"
            )
        resolved = slice_token_group(hidden_states, token_groups[spec.token_group])
        if resolved.shape[0] != n_steps or resolved.shape[1] == 0:
            raise ValueError(
                f"{name}: token slice must be (T={n_steps}, d>0), got {resolved.shape}"
            )
        return resolved, f"token_slice:{spec.token_group}"
    if spec.mode == "text_hash":
        if spec.dim is None:
            raise ValueError(f"{name}: mode 'text_hash' requires dim")
        vec = text_hash_features(instruction, spec.dim)
        return _broadcast_steps(vec, n_steps), "text_hash_proxy"
    raise ValueError(f"{name}: unknown mode {spec.mode!r}")
