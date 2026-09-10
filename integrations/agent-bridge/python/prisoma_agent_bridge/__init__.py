"""Optional canonical control. Application adapters own their completion checks."""

from ._native import Bridge, artifact_identity, hash_object, inspect_runlog

__all__ = ["Bridge", "artifact_identity", "hash_object", "inspect_runlog"]
