"""Faithfulness-checked attribution baseline (grandplan §6.10, §10.2; H4/exploratory).

A model-agnostic attribution + faithfulness + run-log toolchain, demonstrated on a
small numpy self-attention model so it runs without GPUs or a VLA checkpoint. The
faithfulness check, provenance/hashing, and ``attribution_logged`` run-log emission
are the reusable, production-relevant parts; for a real transformer VLA, swap the
model for the real one and the LRP method for the LXT/AttnLRP library
(``rachtibat/LRP-eXplains-Transformers``) — the contract here is unchanged.
"""

from __future__ import annotations

from .attribute import finite_difference_gradient, grad_times_input, lrp_epsilon
from .faithfulness import FaithfulnessResult, faithfulness_check
from .model import SmallTransformer
from .probe import METHODS, run_attribution_probe
from .runlog import AttributionRecord, canonical_hash, write_attribution_runlog

__all__ = [
    "METHODS",
    "AttributionRecord",
    "FaithfulnessResult",
    "SmallTransformer",
    "canonical_hash",
    "faithfulness_check",
    "finite_difference_gradient",
    "grad_times_input",
    "lrp_epsilon",
    "run_attribution_probe",
    "write_attribution_runlog",
]
