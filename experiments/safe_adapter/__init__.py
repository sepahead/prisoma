"""Adapt released SAFE VLA rollouts into this project's ``(V,L,D,A)`` contract.

See ``README.md`` in this directory for the honesty caveats about which variables
the released SAFE tensors actually provide (D, A, labels) versus which require
extra extraction (clean V, L). The public surface:

* :func:`~experiments.safe_adapter.rollouts.load_safe_rollout_dir` /
  :func:`~experiments.safe_adapter.rollouts.write_synthetic_safe_dir`
* :func:`~experiments.safe_adapter.convert.rollouts_to_dataset` and
  :class:`~experiments.safe_adapter.convert.MappingConfig`
* :func:`~experiments.safe_adapter.hook_probe.layerwise_physics_probe` (§9.1)
* :func:`~experiments.safe_adapter.verify.verify_contract_file`
"""

from __future__ import annotations

from .contract import VldaDataset, VldaSample
from .convert import MappingConfig, rollout_to_samples, rollouts_to_dataset
from .extract import VariableSpec, resolve_variable, text_hash_features
from .hook_probe import ProbeSweepResult, layerwise_physics_probe
from .rollouts import SafeRollout, load_safe_rollout_dir, write_synthetic_safe_dir
from .verify import ContractReport, verify_contract_file

__all__ = [
    "ContractReport",
    "MappingConfig",
    "ProbeSweepResult",
    "SafeRollout",
    "VariableSpec",
    "VldaDataset",
    "VldaSample",
    "layerwise_physics_probe",
    "load_safe_rollout_dir",
    "resolve_variable",
    "rollout_to_samples",
    "rollouts_to_dataset",
    "text_hash_features",
    "verify_contract_file",
    "write_synthetic_safe_dir",
]
