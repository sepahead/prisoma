"""Deletion ranking-sensitivity attribution diagnostic (H4/exploratory).

A model-agnostic attribution + group-level deletion diagnostic + run-log toolchain,
demonstrated on a small numpy self-attention model.  Deletion ranking sensitivity is
not causal or mechanistic faithfulness; the explicit baseline may be OOD and feature
dependence remains unresolved.  The run-log's historical ``faithfulness_check``
boolean is true only when the stronger frozen validation gate passes.
"""

from __future__ import annotations

from .attribute import finite_difference_gradient, grad_times_input, lrp_epsilon
from .faithfulness import (
    AttributionValidationCase,
    FaithfulnessResult,
    RankingSensitivityGate,
    bind_ranking_gate,
    faithfulness_check,
    ranking_gate_content_sha256,
    ranking_gate_manifest,
    ranking_sensitivity_check,
)
from .model import SmallTransformer
from .probe import METHODS, ProbeValidationCase, run_attribution_probe
from .runlog import AttributionRecord, canonical_hash, write_attribution_runlog

__all__ = [
    "METHODS",
    "AttributionRecord",
    "AttributionValidationCase",
    "FaithfulnessResult",
    "ProbeValidationCase",
    "RankingSensitivityGate",
    "SmallTransformer",
    "bind_ranking_gate",
    "canonical_hash",
    "faithfulness_check",
    "finite_difference_gradient",
    "grad_times_input",
    "lrp_epsilon",
    "ranking_sensitivity_check",
    "ranking_gate_content_sha256",
    "ranking_gate_manifest",
    "run_attribution_probe",
    "write_attribution_runlog",
]
