"""Orchestrate the §6.10 attribution probe: attribute, faithfulness-check, log.

This is the attribution baseline (H4/exploratory) end to end on the small reference model: for each requested
attribution method, compute the relevance of a declared scalar target, run the
deletion-AOPC faithfulness check against a random control, and assemble
schema-conformant ``attribution_logged`` records (written via
:mod:`~experiments.attribution.runlog`). Attributions that fail their faithfulness
check are still logged — with ``faithfulness_check=false`` — so a downstream
PID-vs-attribution comparison can exclude them, exactly as §6.10 requires.
"""

from __future__ import annotations

from collections.abc import Sequence

import numpy as np

from .attribute import grad_times_input, lrp_epsilon
from .faithfulness import faithfulness_check
from .model import SmallTransformer
from .runlog import AttributionRecord

METHODS = {
    "lrp_epsilon": lrp_epsilon,
    "grad_x_input": grad_times_input,
}


def run_attribution_probe(
    model: SmallTransformer,
    x: np.ndarray,
    *,
    target_output: str = "scalar_target",
    methods: Sequence[str] = ("lrp_epsilon", "grad_x_input"),
    modality: str | None = None,
    n_steps: int = 10,
    n_random: int = 8,
    seed: int = 0,
) -> list[AttributionRecord]:
    """Run each method, faithfulness-check it, and return loggable records."""
    x = np.asarray(x, dtype=np.float64)
    records: list[AttributionRecord] = []
    for name in methods:
        if name not in METHODS:
            raise ValueError(f"unknown attribution method: {name!r}")
        relevance = METHODS[name](model, x)
        result = faithfulness_check(
            model.forward,
            x,
            relevance,
            n_steps=n_steps,
            n_random=n_random,
            seed=seed,
        )
        records.append(
            AttributionRecord(
                method=name,
                target_output=target_output,
                relevance=relevance,
                faithfulness_passed=result.passed,
                modality=modality,
                baseline="zero",
                metadata={
                    "method_aopc": f"{result.method_aopc:.6f}",
                    "random_aopc": f"{result.random_aopc:.6f}",
                    "faithfulness_steps": str(result.n_steps),
                },
            )
        )
    return records
