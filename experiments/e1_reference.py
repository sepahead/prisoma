"""Bounded E1 numerical reference. This module owns no simulator capability.

Content identities describe values supplied to this module. They do not establish
canonical event order, checkpoint authority, native label ancestry, or loaded code.
The experiment owner must establish those facts before using operational results.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass, fields
import hashlib
import json
import math
import re
import struct
import sys
from typing import Any

import numpy as np


EXPERIMENT_ID = "NCP-E1-ACTION-CONDITIONED-PRESSURE-FORECAST"
SEED_DOMAIN = "prisoma.e1.reference-seed-roster.v1"
SPLITS = (("qualification", 8), ("train", 64), ("development", 16), ("heldout", 32))
LAMBDAS = (1e-6, 1e-3, 1.0, 1e3)
ACTIONS = (
    "neutral",
    "positive_pitch",
    "negative_pitch",
    "positive_roll",
    "negative_roll",
)
ANGLES = ((0.0, 0.0), (0.0, 0.03), (0.0, -0.03), (0.03, 0.0), (-0.03, 0.0))
OBSERVATIONS = (
    "A_R",
    "A_G",
    "A_B",
    "B_R",
    "B_G",
    "B_B",
    "pressure_mean_pa",
    "pressure_rms_pa",
)
FEATURES = (
    OBSERVATIONS
    + ("roll_rad", "pitch_rad")
    + tuple(
        f"{observation}*{angle}"
        for observation in OBSERVATIONS
        for angle in ("roll_rad", "pitch_rad")
    )
)
TARGET_FUNCTION_SHA256 = (
    "2587d58265dfec6decf9d2255a410ca195a337e1bbf1020ccaac4f37ee6c1505"
)
BOOTSTRAP_INDICES_SHA256 = (
    "2f2e3aa4f7869010334213e1ac4bd7b0e25d2b7892ef3c80cff6208fccb1d203"
)
RCOND = 1e-12
NORMAL_RESIDUAL_FACTOR = 1e-10


class ReferenceError(ValueError):
    """A named rejection of an input or numerical contract."""

    def __init__(self, code: str, detail: str):
        super().__init__(f"{code}: {detail}")
        self.code = code


def _require(condition: bool, code: str, detail: str) -> None:
    if not condition:
        raise ReferenceError(code, detail)


def _finite(value: float, code: str = "nonfinite") -> float:
    _require(type(value) in (int, float), code, "finite binary64 value required")
    try:
        result = float(value)
    except OverflowError as error:
        raise ReferenceError(code, "value exceeds binary64 range") from error
    _require(math.isfinite(result), code, "finite binary64 value required")
    return result


def _digest(value: str) -> str:
    _require(
        type(value) is str and re.fullmatch(r"[0-9a-f]{64}", value) is not None,
        "digest",
        "lowercase SHA-256 required",
    )
    return value


def canonical(value: Any) -> bytes:
    """Encode a bounded, finite JSON value for content identity."""
    try:
        encoded = json.dumps(
            value,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
            allow_nan=False,
        ).encode()
    except (TypeError, ValueError, OverflowError, RecursionError) as error:
        raise ReferenceError("json_value", "finite JSON value required") from error
    _require(len(encoded) <= 1_048_576, "json_size", "reference record exceeds 1 MiB")
    return encoded


def content_digest(value: Any) -> str:
    return hashlib.sha256(canonical(value)).hexdigest()


def _record_bytes(payload: bytes, expected_sha256: str) -> dict[str, Any]:
    _digest(expected_sha256)
    _require(
        type(payload) is bytes and len(payload) <= 1_048_576,
        "json_size",
        "bounded original canonical bytes required",
    )
    _require(
        hashlib.sha256(payload).hexdigest() == expected_sha256,
        "inference_identity",
        "selected artifact bytes changed",
    )

    def unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result = {}
        for key, value in pairs:
            _require(key not in result, "json_value", "duplicate object key")
            result[key] = value
        return result

    try:
        value = json.loads(payload, object_pairs_hook=unique)
    except (ValueError, UnicodeError, RecursionError) as error:
        if isinstance(error, ReferenceError):
            raise
        raise ReferenceError("json_value", "canonical finite JSON required") from error
    _require(
        type(value) is dict and canonical(value) == payload,
        "json_value",
        "original canonical object bytes required",
    )
    return value


def _numerical_runtime() -> None:
    _require(
        sys.implementation.name == "cpython"
        and sys.version_info[:2] == (3, 11)
        and np.__version__ == "2.4.6",
        "numerical_runtime",
        "E1 selects CPython 3.11 and locked NumPy 2.4.6",
    )


def project_position(seed: int) -> tuple[float, float, float]:
    """Project three native uint32 LCG draws, including the fixed-y draw."""
    _require(
        type(seed) is int and 0 <= seed < 2**32,
        "seed_type_range",
        "uint32 seed required",
    )
    words = _position_words(seed)
    return (-0.5 + words[0] / 2**32, 8.0, -0.5 + words[2] / 2**32)


def _position_words(seed: int) -> tuple[int, int, int]:
    words = []
    for _ in range(3):
        seed = (1664525 * seed + 1013904223) & 0xFFFFFFFF
        words.append(seed)
    return tuple(words)


def seed_roster() -> dict[str, Any]:
    """Reconstruct the complete prospective roster without ambient randomness."""
    used = {104729}
    episodes = []
    for split, count in SPLITS:
        for index in range(count):
            episode_id = f"e1-{split}-{index:03d}"
            seeds, derivations = {}, {}
            for role in ("placement", "runtime", "acoustic"):
                nonce = 0
                while True:
                    preimage = f"{SEED_DOMAIN}\n{episode_id}\n{role}\n{nonce}\n".encode(
                        "ascii"
                    )
                    digest = hashlib.sha256(preimage).digest()
                    seed = int.from_bytes(digest[:4], "big")
                    if seed not in used:
                        break
                    nonce += 1
                used.add(seed)
                seeds[role] = seed
                derivations[role] = {"nonce": nonce, "preimage_sha256": digest.hex()}
            episodes.append(
                {
                    "episode_id": episode_id,
                    "split": split,
                    "seeds": seeds,
                    "derivation": derivations,
                    "placement_lcg_words_xyz": list(
                        _position_words(seeds["placement"])
                    ),
                    "explicit_position_m": list(project_position(seeds["placement"])),
                }
            )
    return {
        "schema": SEED_DOMAIN,
        "status": "PROSPECTIVE_DESIGN_NOT_OPERATIONAL_AUTHORITY",
        "experiment_id": EXPERIMENT_ID,
        "selection": {
            "domain": SEED_DOMAIN,
            "hash": "sha256",
            "encoding": "ASCII domain LF episode_id LF role LF decimal_nonce LF",
            "seed_word": "first_four_digest_bytes_unsigned_big_endian",
            "collision_rule": "increment_nonce_until_globally_unused_uint32",
            "excluded_words": [104729],
            "role_order": ["placement", "runtime", "acoustic"],
            "split_order_and_counts": [[name, count] for name, count in SPLITS],
            "outcomes_inspected": False,
            "seed_replacement_allowed": False,
        },
        "placement": {
            "generator": "CREBAIN_uint32_LCG_projection_v1",
            "multiplier": 1664525,
            "increment": 1013904223,
            "modulus": 2**32,
            "draws_per_position": 3,
            "axis_order": ["x", "y", "z"],
            "minimum_m": [-0.5, 8.0, -0.5],
            "maximum_m": [0.5, 8.0, 0.5],
            "fixed_y_draw_consumed": True,
            "distribution_scope": "finite_pseudorandom_design_not_literal_continuous_independence",
            "runtime_rng_consumption_claimed": False,
        },
        "branch_rule": "Restored siblings intentionally reuse the qualified episode checkpoint RNG state; every owner/run/generation remains unique.",
        "bootstrap": {
            "bit_generator": "numpy.random.PCG64",
            "seed": 104729,
            "resamples": 10000,
            "episode_count": 32,
        },
        "episodes": episodes,
    }


def validate_seed_roster(value: dict[str, Any]) -> None:
    _require(
        canonical(value) == canonical(seed_roster()),
        "seed_roster_identity",
        "the complete prospective roster must match",
    )


def study_contract() -> dict[str, Any]:
    """Return the explicit bounded study, including stage-specific policies."""
    return {
        "schema": "prisoma.e1-reference-study.v1",
        "experiment_id": EXPERIMENT_ID,
        "scope": "small_learned_reference_not_advertised_product_model_qualification",
        "splits": dict(SPLITS),
        "seed_roster_semantic_sha256": content_digest(seed_roster()),
        "clock_hz": 120,
        "scene": {
            "bodies": 1,
            "solids": 0,
            "frame": "Y_up_Z_forward",
            "camera_positions_m": [[0, 9, 5], [5, 9, 0]],
            "camera_target_m": [0, 8, 0],
            "camera_fov_degrees": 60,
            "camera_period_ticks": 3,
            "microphone_position_m": [0, 2, 10],
            "neutral_through_tick": 12,
            "workload_identity": "separate_prospective_native_freeze_required",
        },
        "landmark_tick": 12,
        "target_ticks": [34, 35, 36],
        "samples_per_window": 400,
        "actions": [
            {
                "id": name,
                "armed": True,
                "kind": "force_attitude_height",
                "roll_rad": roll,
                "pitch_rad": pitch,
                "heading_rad": 0.0,
                "altitude_m": 8.0,
                "apply_tick": 13,
                "hold_through_tick": 36,
            }
            for name, (roll, pitch) in zip(ACTIONS, ANGLES, strict=True)
        ],
        "feature_order": list(FEATURES),
        "feature_units": ["1"] * 6 + ["Pa", "Pa"] + ["rad"] * 14 + ["Pa rad"] * 4,
        "observation_units": ["1"] * 6 + ["Pa", "Pa"],
        "action_unit": "rad",
        "intercept_separate": True,
        "rgb": {
            "width": 160,
            "height": 120,
            "channels": "RGBA",
            "kind": "rgba8",
            "dtype": "u8",
            "layout": "c_contiguous",
            "encoding": "rgba8-srgb",
            "row_origin": "bottom-left",
            "normalized_by": 255,
        },
        "pressure": {
            "kind": "pressure",
            "dtype": "f64le",
            "layout": "c_contiguous",
            "unit": "pascal",
            "sample_rate_hz": 16000,
            "availability": "source_tick_equals_available_tick",
            "target_function_sha256": TARGET_FUNCTION_SHA256,
        },
        "transform": {
            "fit_split": "train",
            "rows": 320,
            "drop_rule": "exact_numeric_constant",
            "representation": "maximum_absolute_value_then_unit_mean_and_population_scale",
            "nonconstant_zero_scale": "reject",
        },
        "solver": {
            "library": "numpy",
            "version": "2.4.6",
            "python": "3.11",
            "dtype": "float64",
            "method": "numpy.linalg.lstsq_augmented_ridge",
            "rcond": RCOND,
            "normal_residual_relative_bound": NORMAL_RESIDUAL_FACTOR,
            "objective": "SSE_plus_lambda_squared_coefficient_norm",
            "intercept_penalized": False,
            "target_scale": "positive_training_maximum_or_one",
            "lambdas": list(LAMBDAS),
            "development_tie": "exact_larger_lambda",
            "retrain_after_selection": False,
        },
        "forecast": {
            "negative_clip": 0.0,
            "selection": "smallest_value_then_declared_action_order",
        },
        "stage_policy": {
            "train": {"forecasts": ["persistence"], "collection_policy": "neutral"},
            "development": {
                "forecasts": [
                    "all_four_frozen_training_ridges",
                    "constant",
                    "persistence",
                ],
                "collection_policy": "neutral",
            },
            "heldout": {
                "forecasts": ["selected_frozen_ridge", "constant", "persistence"],
                "collection_policy": "selected_ridge",
            },
        },
        "evaluation": {
            "metric": "episode_mean_absolute_error_pa_across_five_actions",
            "direction": "baseline_minus_reference",
            "bootstrap": {
                "bit_generator": "PCG64",
                "seed": 104729,
                "resamples": 10000,
                "unit": "episode",
                "episode_count": 32,
                "indices_le_u32_sha256": BOOTSTRAP_INDICES_SHA256,
                "quantiles": [0.025, 0.975],
                "method": "linear",
            },
            "useful_margin": "0.05_times_training_mean_target_pa",
            "benefit_requires": "both_baseline_interval_lower_endpoints_exceed_same_positive_margin",
            "no_action_variation": "all_five_targets_equal_within_every_heldout_episode",
            "missing_or_failed_episode": "retain_failure_and_reject_complete_report",
        },
        "authority": {
            "native_label_ancestry_verified_here": False,
            "canonical_order_verified_here": False,
            "loaded_bytes_attested": False,
            "release_qualified": False,
        },
    }


def _compensated(values: tuple[float, ...] | list[float]) -> float:
    total = correction = 0.0
    for value in values:
        following = _finite(total + value)
        correction = _finite(
            correction
            + (
                ((total - following) + value)
                if abs(total) >= abs(value)
                else ((value - following) + total)
            )
        )
        total = following
    return _finite(total + correction)


def _moment(values: tuple[float, ...] | list[float], *, rms: bool) -> float:
    _require(
        type(values) in (tuple, list) and 1 <= len(values) <= 400,
        "sample_count",
        "one to 400 ordered samples required",
    )
    _require(
        all(type(value) is float and math.isfinite(value) for value in values),
        "pressure_numeric",
        "finite binary64 samples required",
    )
    maximum = max(abs(value) for value in values)
    if maximum == 0:
        return 0.0
    ratios = [_finite(value / maximum) for value in values]
    terms = [_finite(value * value) for value in ratios] if rms else ratios
    average = _finite(_compensated(terms) / len(values))
    _require(
        not rms or average >= 0, "pressure_numeric", "negative normalized square mean"
    )
    return _finite(maximum * (math.sqrt(average) if rms else average))


def pressure_mean(samples: tuple[float, ...] | list[float]) -> float:
    _require(
        type(samples) in (tuple, list) and len(samples) == 400,
        "sample_count",
        "exactly 400 pressure samples required",
    )
    return _moment(samples, rms=False)


def pressure_rms(samples: tuple[float, ...] | list[float]) -> float:
    _require(
        type(samples) in (tuple, list) and len(samples) == 400,
        "sample_count",
        "exactly 400 pressure samples required",
    )
    return _moment(samples, rms=True)


@dataclass(frozen=True, slots=True)
class RGBFrame:
    source_id: str
    source_tick: int
    available_tick: int
    payload: bytes
    kind: str = "rgba8"
    dtype: str = "u8"
    layout: str = "c_contiguous"
    encoding: str = "rgba8-srgb"
    row_origin: str = "bottom-left"
    shape: tuple[int, int, int] = (120, 160, 4)


@dataclass(frozen=True, slots=True)
class PressureBlock:
    source_id: str
    source_tick: int
    available_tick: int
    sample_start: int
    sample_end: int
    payload: bytes
    shape: tuple[int]
    kind: str = "pressure"
    dtype: str = "f64le"
    layout: str = "c_contiguous"
    sample_rate_hz: int = 16000
    unit: str = "pascal"


def pressure_window(
    blocks: tuple[PressureBlock, ...], *, first_tick: int, source_id: str
) -> tuple[tuple[float, ...], str]:
    """Decode one declared window. Native branch ancestry is the caller's gate."""
    _require(
        type(source_id) is str and 0 < len(source_id) <= 256,
        "pressure_window",
        "bounded declared source identity required",
    )
    _require(
        first_tick in (10, 34) and type(first_tick) is int,
        "pressure_window",
        "E1 landmark or target window required",
    )
    _require(
        type(blocks) is tuple and len(blocks) == 3,
        "pressure_window",
        "three ordered complete blocks required",
    )
    payloads = []
    for tick, block in zip(range(first_tick, first_tick + 3), blocks, strict=True):
        _require(
            type(block) is PressureBlock,
            "pressure_encoding",
            "typed pressure block required",
        )
        _require(
            all(
                type(x) is int
                for x in (
                    block.source_tick,
                    block.available_tick,
                    block.sample_start,
                    block.sample_end,
                )
            ),
            "pressure_window",
            "exact integer ticks and sample indices required",
        )
        start, end = (tick - 1) * 16000 // 120, tick * 16000 // 120
        _require(
            block.source_id == source_id
            and block.source_tick == tick
            and block.available_tick == tick
            and block.sample_start == start
            and block.sample_end == end,
            "pressure_window",
            "source, availability, and complete contiguous interval must match",
        )
        _require(
            block.kind == "pressure"
            and block.dtype == "f64le"
            and block.layout == "c_contiguous"
            and type(block.shape) is tuple
            and block.shape == (end - start,)
            and type(block.shape[0]) is int
            and type(block.sample_rate_hz) is int
            and block.sample_rate_hz == 16000
            and block.unit == "pascal"
            and type(block.payload) is bytes
            and len(block.payload) == 8 * (end - start),
            "pressure_encoding",
            "original little-endian binary64 payload required",
        )
        payloads.append(block.payload)
    payload = b"".join(payloads)
    _require(
        len(payload) == 3200, "sample_count", "exactly 400 pressure samples required"
    )
    values = struct.unpack("<400d", payload)
    _require(
        all(math.isfinite(value) for value in values),
        "pressure_numeric",
        "nonfinite pressure sample",
    )
    return values, hashlib.sha256(payload).hexdigest()


@dataclass(frozen=True, slots=True)
class Landmark:
    values: tuple[float, ...]
    input_sha256: str

    def __post_init__(self) -> None:
        _require(
            type(self.values) is tuple and len(self.values) == 8,
            "feature_roster",
            "eight observation features required",
        )
        object.__setattr__(
            self,
            "values",
            tuple(_finite(value, "feature_numeric") for value in self.values),
        )
        _require(
            all(0 <= value <= 1 for value in self.values[:6]) and self.values[7] >= 0,
            "feature_numeric",
            "encoded RGB and pressure RMS bounds",
        )
        _digest(self.input_sha256)


def landmark_features(
    frames: tuple[RGBFrame, RGBFrame],
    blocks: tuple[PressureBlock, ...],
    *,
    camera_ids: tuple[str, str],
    microphone_id: str,
) -> Landmark:
    """Extract only the two declared tick-12 frames and ticks 10–12 pressure."""
    _require(
        type(camera_ids) is tuple
        and len(camera_ids) == 2
        and all(type(x) is str and 0 < len(x) <= 256 for x in camera_ids)
        and camera_ids[0] != camera_ids[1],
        "rgb_contract",
        "two distinct declared camera identities required",
    )
    _require(
        type(frames) is tuple and len(frames) == 2,
        "rgb_contract",
        "two ordered RGB frames required",
    )
    values, inputs = [], []
    for frame, source_id in zip(frames, camera_ids, strict=True):
        _require(
            type(frame) is RGBFrame
            and frame.source_id == source_id
            and type(frame.shape) is tuple
            and frame.shape == (120, 160, 4)
            and all(type(x) is int for x in frame.shape),
            "rgb_contract",
            "declared source and exact RGBA shape required",
        )
        _require(
            type(frame.source_tick) is int
            and type(frame.available_tick) is int
            and frame.source_tick == frame.available_tick == 12,
            "landmark_availability",
            "due frame must be available at tick 12",
        )
        _require(
            frame.kind == "rgba8"
            and frame.dtype == "u8"
            and frame.layout == "c_contiguous"
            and frame.encoding == "rgba8-srgb"
            and frame.row_origin == "bottom-left"
            and type(frame.payload) is bytes
            and len(frame.payload) == 76800,
            "rgb_contract",
            "declared original RGBA8 layout required",
        )
        values.extend(
            sum(frame.payload[channel::4]) / (19200 * 255) for channel in range(3)
        )
        inputs.append(
            {
                "source_id": source_id,
                "source_tick": frame.source_tick,
                "available_tick": frame.available_tick,
                "shape": list(frame.shape),
                "kind": frame.kind,
                "dtype": frame.dtype,
                "layout": frame.layout,
                "encoding": frame.encoding,
                "row_origin": frame.row_origin,
                "payload_sha256": hashlib.sha256(frame.payload).hexdigest(),
            }
        )
    pressure, payload_digest = pressure_window(
        blocks, first_tick=10, source_id=microphone_id
    )
    values.extend((pressure_mean(pressure), pressure_rms(pressure)))
    pressure_inputs = [
        {
            "source_id": block.source_id,
            "source_tick": block.source_tick,
            "available_tick": block.available_tick,
            "sample_start": block.sample_start,
            "sample_end": block.sample_end,
            "shape": list(block.shape),
            "kind": block.kind,
            "dtype": block.dtype,
            "layout": block.layout,
            "sample_rate_hz": block.sample_rate_hz,
            "unit": block.unit,
            "payload_sha256": hashlib.sha256(block.payload).hexdigest(),
        }
        for block in blocks
    ]
    return Landmark(
        tuple(values),
        content_digest(
            {
                "rgb": inputs,
                "pressure": pressure_inputs,
                "ordered_pressure_payload_sha256": payload_digest,
            }
        ),
    )


def candidate_features(observation: Landmark, action: str) -> tuple[float, ...]:
    _require(
        type(observation) is Landmark and action in ACTIONS,
        "feature_roster",
        "declared observation and action required",
    )
    roll, pitch = ANGLES[ACTIONS.index(action)]
    result = (
        observation.values
        + (roll, pitch)
        + tuple(
            _finite(value * angle)
            for value in observation.values
            for angle in (roll, pitch)
        )
    )
    _require(len(result) == 26, "feature_roster", "26 non-intercept features required")
    return result


@dataclass(frozen=True, slots=True)
class Episode:
    """Numerical rows whose native source qualification remains external."""

    episode_id: str
    observation: Landmark
    targets_pa: tuple[float, ...]
    label_set_sha256: str
    target_function_sha256: str = TARGET_FUNCTION_SHA256
    action_order: tuple[str, ...] = ACTIONS

    def __post_init__(self) -> None:
        _require(
            type(self.episode_id) is str and type(self.observation) is Landmark,
            "episode",
            "typed episode identity and landmark required",
        )
        _require(
            type(self.targets_pa) is tuple
            and len(self.targets_pa) == 5
            and self.action_order == ACTIONS,
            "branch_roster",
            "all five targets in declared action order required",
        )
        object.__setattr__(
            self,
            "targets_pa",
            tuple(_finite(value, "target_numeric") for value in self.targets_pa),
        )
        _require(
            all(_finite(value, "target_numeric") >= 0 for value in self.targets_pa),
            "target_numeric",
            "pressure RMS targets must be nonnegative",
        )
        _digest(self.label_set_sha256)
        _require(
            self.target_function_sha256 == TARGET_FUNCTION_SHA256,
            "target_function",
            "selected owner RMS contract required",
        )


def _episode_ids(split: str) -> tuple[str, ...]:
    counts = dict(SPLITS)
    _require(split in counts, "split_roster", "unknown episode split")
    return tuple(f"e1-{split}-{index:03d}" for index in range(counts[split]))


def _complete_split(rows: tuple[Any, ...], split: str) -> None:
    _require(
        type(rows) is tuple
        and all(
            type(row) is (Episode if split == "train" else EpisodeEvaluation)
            for row in rows
        )
        and tuple(row.episode_id for row in rows) == _episode_ids(split),
        "split_roster",
        "complete ordered split required; no replacement or omitted episode",
    )


@dataclass(frozen=True, slots=True)
class Transform:
    maximum: float
    unit_mean: float
    unit_scale: float
    constant: float | None

    def __post_init__(self) -> None:
        for name in ("maximum", "unit_mean", "unit_scale"):
            object.__setattr__(self, name, _finite(getattr(self, name), "transform"))
        if self.constant is None:
            _require(
                self.maximum > 0 and self.unit_scale > 0,
                "transform",
                "retained column needs positive finite scales",
            )
        else:
            object.__setattr__(self, "constant", _finite(self.constant, "transform"))
            _require(
                self.maximum == self.unit_mean == self.unit_scale == 0.0,
                "transform",
                "dropped column has no transform",
            )


def _fit_transform(matrix: np.ndarray) -> tuple[np.ndarray, tuple[Transform, ...]]:
    transforms, columns = [], []
    for index in range(26):
        raw = [float(value) for value in matrix[:, index]]
        if min(raw) == max(raw):
            transforms.append(Transform(0.0, 0.0, 0.0, raw[0]))
            continue
        maximum = max(abs(value) for value in raw)
        unit = [value / maximum for value in raw]
        mean = _moment(unit, rms=False)
        deviations = [_finite(value - mean) for value in unit]
        scale = _moment(deviations, rms=True)
        _require(
            scale > 0,
            "scale_rule",
            "nonconstant training column has unrepresentable scale",
        )
        transforms.append(Transform(maximum, mean, scale, None))
        columns.append([_finite(value / scale) for value in deviations])
    normalized = (
        np.asarray(columns, dtype=np.float64).T
        if columns
        else np.empty((320, 0), dtype=np.float64)
    )
    return normalized, tuple(transforms)


@dataclass(frozen=True, slots=True)
class Reference:
    penalty: float
    transforms: tuple[Transform, ...]
    coefficients: tuple[float, ...]
    target_scale: float
    training_mean_pa: float
    training_action_variation: bool
    training_sha256: str
    source_sha256: str
    runtime_inventory_sha256: str

    def __post_init__(self) -> None:
        _require(
            type(self.penalty) is float and self.penalty in LAMBDAS,
            "lambda_grid",
            "selected ridge grid value required",
        )
        _require(
            type(self.transforms) is tuple
            and len(self.transforms) == 26
            and all(type(x) is Transform for x in self.transforms),
            "transform",
            "all 26 training column decisions required",
        )
        _require(
            type(self.coefficients) is tuple
            and len(self.coefficients)
            == 1 + sum(x.constant is None for x in self.transforms),
            "coefficients",
            "unpenalized intercept plus retained columns required",
        )
        object.__setattr__(
            self,
            "coefficients",
            tuple(_finite(value, "coefficients") for value in self.coefficients),
        )
        for name in ("target_scale", "training_mean_pa"):
            object.__setattr__(
                self, name, _finite(getattr(self, name), "training_summary")
            )
        _require(
            _finite(self.target_scale) > 0
            and 0 <= _finite(self.training_mean_pa) <= self.target_scale
            and type(self.training_action_variation) is bool,
            "training_summary",
            "finite training summaries required",
        )
        for value in (
            self.training_sha256,
            self.source_sha256,
            self.runtime_inventory_sha256,
        ):
            _digest(value)

    def record(self) -> dict[str, Any]:
        return {
            "schema": "prisoma.e1-ridge-reference.v1",
            "contract_sha256": content_digest(study_contract()),
            "feature_order": list(FEATURES),
            "numerical_library": {
                "python": "3.11",
                "numpy": "2.4.6",
                "dtype": "float64",
                "rcond": RCOND,
                "normal_residual_factor": NORMAL_RESIDUAL_FACTOR,
            },
            **asdict(self),
        }

    @property
    def sha256(self) -> str:
        return content_digest(self.record())


def load_reference(payload: bytes, *, expected_sha256: str) -> Reference:
    """Reopen exact finite data, without pickle or executable model loaders."""
    value = _record_bytes(payload, expected_sha256)
    names = {field.name for field in fields(Reference)}
    _require(
        set(value)
        == names | {"schema", "contract_sha256", "feature_order", "numerical_library"},
        "reference_record",
        "closed reference record required",
    )
    _require(
        type(value["transforms"]) is list
        and len(value["transforms"]) == 26
        and all(
            type(row) is dict
            and set(row) == {field.name for field in fields(Transform)}
            for row in value["transforms"]
        )
        and type(value["coefficients"]) is list,
        "reference_record",
        "typed transform and coefficient records required",
    )
    parameters = {name: value[name] for name in names}
    parameters["transforms"] = tuple(Transform(**row) for row in value["transforms"])
    parameters["coefficients"] = tuple(value["coefficients"])
    reference = Reference(**parameters)
    _require(
        canonical(reference.record()) == payload,
        "reference_record",
        "schema, feature order, study and numerical contract must match",
    )
    return reference


def _ridge(
    design: np.ndarray, labels: np.ndarray, penalty: float, target_scale: float
) -> tuple[float, ...]:
    width = design.shape[1]
    augmented = np.vstack((design, np.diag([0.0] + [math.sqrt(penalty)] * (width - 1))))
    target = np.concatenate((labels / target_scale, np.zeros(width, dtype=np.float64)))
    try:
        coefficients, _, rank, singular = np.linalg.lstsq(
            augmented, target, rcond=RCOND
        )
    except np.linalg.LinAlgError as error:
        raise ReferenceError(
            "solver_contract", "least squares did not converge"
        ) from error
    _require(
        rank == width
        and np.isfinite(coefficients).all()
        and np.isfinite(singular).all(),
        "solver_contract",
        "finite full-rank augmented fit required",
    )
    residual = design @ coefficients - labels / target_scale
    gradient = design.T @ residual + penalty * np.concatenate(([0.0], coefficients[1:]))
    penalized_norm = (
        float(np.linalg.norm(coefficients[1:], np.inf)) if width > 1 else 0.0
    )
    bound = _finite(
        NORMAL_RESIDUAL_FACTOR
        * (
            float(np.linalg.norm(design, 1))
            * (
                float(np.linalg.norm(design, np.inf))
                * float(np.linalg.norm(coefficients, np.inf))
                + float(np.linalg.norm(labels / target_scale, np.inf))
            )
            + penalty * penalized_norm
        ),
        "solver_contract",
    )
    _require(
        np.isfinite(gradient).all()
        and float(np.linalg.norm(gradient, np.inf)) <= bound,
        "solver_contract",
        "normal residual exceeds selected relative bound",
    )
    return tuple(float(value) for value in coefficients)


def fit_reference(
    training: tuple[Episode, ...], *, source_sha256: str, runtime_inventory_sha256: str
) -> tuple[Reference, ...]:
    """Fit the four training-only artifacts; no development labels are accepted."""
    _numerical_runtime()
    _complete_split(training, "train")
    _require(
        all(type(row) is Episode for row in training),
        "episode",
        "typed complete episodes required",
    )
    matrix = np.asarray(
        [
            candidate_features(row.observation, action)
            for row in training
            for action in ACTIONS
        ],
        dtype=np.float64,
    )
    labels_list = [float(value) for row in training for value in row.targets_pa]
    labels = np.asarray(labels_list, dtype=np.float64)
    transformed, transforms = _fit_transform(matrix)
    design = np.column_stack((np.ones(320, dtype=np.float64), transformed))
    target_scale = max(labels_list) or 1.0
    training_mean = _moment(labels_list, rms=False)
    training_identity = content_digest([asdict(row) for row in training])
    varying = any(len(set(row.targets_pa)) > 1 for row in training)
    return tuple(
        Reference(
            penalty,
            transforms,
            _ridge(design, labels, penalty, target_scale),
            target_scale,
            training_mean,
            varying,
            training_identity,
            source_sha256,
            runtime_inventory_sha256,
        )
        for penalty in LAMBDAS
    )


def forecast(reference: Reference, observation: Landmark) -> tuple[float, ...]:
    _numerical_runtime()
    _require(
        type(reference) is Reference and type(observation) is Landmark,
        "inference_identity",
        "typed frozen reference and observation required",
    )
    predictions = []
    for action in ACTIONS:
        features = candidate_features(observation, action)
        normalized = [1.0]
        for value, transform in zip(features, reference.transforms, strict=True):
            if transform.constant is None:
                normalized.append(
                    _finite(
                        (_finite(value / transform.maximum) - transform.unit_mean)
                        / transform.unit_scale,
                        "feature_numeric",
                    )
                )
        prediction = _finite(
            _compensated(
                [
                    _finite(value * coefficient)
                    for value, coefficient in zip(
                        normalized, reference.coefficients, strict=True
                    )
                ]
            )
            * reference.target_scale,
            "forecast_numeric",
        )
        predictions.append(max(0.0, prediction))
    return tuple(predictions)


@dataclass(frozen=True, slots=True)
class ForecastVector:
    predictor_id: str
    values_pa: tuple[float, ...]

    def __post_init__(self) -> None:
        _require(
            type(self.predictor_id) is str
            and 0 < len(self.predictor_id) <= 80
            and type(self.values_pa) is tuple
            and len(self.values_pa) == 5,
            "forecast_selection",
            "five identified forecasts required",
        )
        object.__setattr__(
            self,
            "values_pa",
            tuple(_finite(value, "forecast_numeric") for value in self.values_pa),
        )
        _require(
            all(_finite(x, "forecast_numeric") >= 0 for x in self.values_pa),
            "forecast_numeric",
            "finite nonnegative point forecasts required",
        )

    @property
    def recommendation(self) -> str:
        return ACTIONS[min(range(5), key=self.values_pa.__getitem__)]


@dataclass(frozen=True, slots=True)
class Commitment:
    """A closed proposed commitment, requiring separate canonical publication."""

    episode_id: str
    stage: str
    input_sha256: str
    forecasts: tuple[ForecastVector, ...]
    collection_policy: str
    selected_action: str
    model_selection_sha256: str | None

    def __post_init__(self) -> None:
        _require(
            self.stage in ("train", "development", "heldout")
            and self.episode_id in _episode_ids(self.stage),
            "split_roster",
            "episode must belong to its declared stage",
        )
        _digest(self.input_sha256)
        _require(
            type(self.forecasts) is tuple
            and all(type(row) is ForecastVector for row in self.forecasts),
            "forecast_selection",
            "typed forecast vectors required",
        )
        names = tuple(row.predictor_id for row in self.forecasts)
        _require(
            len(names) == len(set(names)),
            "forecast_selection",
            "duplicate predictor identity",
        )
        _require(
            all(
                len(set(row.values_pa)) == 1
                for row in self.forecasts
                if row.predictor_id in ("constant", "persistence")
            ),
            "baseline_shape",
            "each no-model baseline repeats one value across all five actions",
        )
        _require(
            self.selected_action in ACTIONS,
            "forecast_selection",
            "declared selected action required",
        )
        if self.stage == "train":
            _require(
                names == ("persistence",)
                and self.collection_policy == "neutral"
                and self.selected_action == "neutral"
                and self.model_selection_sha256 is None,
                "training_order",
                "training collects neutral under persistence, before fitted artifacts",
            )
        elif self.stage == "development":
            _require(
                len(names) == 6
                and names[-2:] == ("constant", "persistence")
                and all(re.fullmatch(r"[0-9a-f]{64}", x) for x in names[:4])
                and self.collection_policy == "neutral"
                and self.selected_action == "neutral"
                and self.model_selection_sha256 is None,
                "development_order",
                "all four frozen candidates commit while actual collection remains neutral",
            )
        else:
            _require(
                len(names) == 3
                and names[-2:] == ("constant", "persistence")
                and re.fullmatch(r"[0-9a-f]{64}", names[0]) is not None
                and self.collection_policy == names[0]
                and self.selected_action == self.forecasts[0].recommendation,
                "holdout_lock",
                "held-out collection follows the selected frozen ridge",
            )
            _digest(self.model_selection_sha256)

    def record(self) -> dict[str, Any]:
        return {
            "schema": "prisoma.e1-forecast-commitment.v1",
            "contract_sha256": content_digest(study_contract()),
            "action_order": ACTIONS,
            **asdict(self),
        }

    @property
    def sha256(self) -> str:
        return content_digest(self.record())


def load_commitment(payload: bytes, *, expected_sha256: str) -> Commitment:
    """Reopen proposed forecast content; canonical publication remains external."""
    value = _record_bytes(payload, expected_sha256)
    names = {field.name for field in fields(Commitment)}
    _require(
        set(value) == names | {"schema", "contract_sha256", "action_order"}
        and type(value.get("forecasts")) is list
        and 1 <= len(value["forecasts"]) <= 6
        and all(
            type(row) is dict
            and set(row) == {"predictor_id", "values_pa"}
            and type(row["values_pa"]) is list
            for row in value["forecasts"]
        ),
        "commitment_record",
        "closed proposed commitment required",
    )
    parameters = {name: value[name] for name in names}
    parameters["forecasts"] = tuple(
        ForecastVector(row["predictor_id"], tuple(row["values_pa"]))
        for row in value["forecasts"]
    )
    commitment = Commitment(**parameters)
    _require(
        canonical(commitment.record()) == payload,
        "commitment_record",
        "study and action roster must match",
    )
    return commitment


def _grid(references: tuple[Reference, ...]) -> None:
    _require(
        type(references) is tuple
        and all(type(row) is Reference for row in references)
        and tuple(row.penalty for row in references) == LAMBDAS,
        "lambda_grid",
        "all four ordered training artifacts required",
    )
    common = [
        (
            row.transforms,
            row.target_scale,
            row.training_mean_pa,
            row.training_action_variation,
            row.training_sha256,
            row.source_sha256,
            row.runtime_inventory_sha256,
        )
        for row in references
    ]
    _require(
        all(value == common[0] for value in common),
        "training_only_transform",
        "candidate grid must share one training source and transform",
    )


def commit_collection(
    stage: str,
    episode_id: str,
    observation: Landmark,
    references: tuple[Reference, ...] = (),
) -> Commitment:
    """Build train/dev pre-label content. This function publishes no event."""
    _require(type(observation) is Landmark, "feature_roster", "typed landmark required")
    persistence = ForecastVector("persistence", (observation.values[7],) * 5)
    if stage == "train":
        _require(
            not references,
            "training_order",
            "training cannot claim a fitted ridge policy",
        )
        rows = (persistence,)
    else:
        _require(
            stage == "development",
            "development_order",
            "use selected commitment for held-out collection",
        )
        _grid(references)
        rows = tuple(
            ForecastVector(row.sha256, forecast(row, observation)) for row in references
        ) + (
            ForecastVector("constant", (references[0].training_mean_pa,) * 5),
            persistence,
        )
    return Commitment(
        episode_id, stage, observation.input_sha256, rows, "neutral", "neutral", None
    )


@dataclass(frozen=True, slots=True)
class EpisodeEvaluation:
    commitment: Commitment
    episode: Episode
    mae_pa: tuple[float, ...]

    def __post_init__(self) -> None:
        _require(
            type(self.commitment) is Commitment
            and type(self.episode) is Episode
            and type(self.mae_pa) is tuple
            and len(self.mae_pa) == len(self.commitment.forecasts),
            "episode_metric",
            "complete typed episode score required",
        )
        object.__setattr__(
            self, "mae_pa", tuple(_finite(x, "episode_metric") for x in self.mae_pa)
        )
        _require(
            all(x >= 0 for x in self.mae_pa),
            "episode_metric",
            "nonnegative MAE required",
        )

    @property
    def episode_id(self) -> str:
        return self.episode.episode_id


def evaluate_episode(commitment: Commitment, episode: Episode) -> EpisodeEvaluation:
    """Score complete numerical rows; supplied identity is not native ancestry."""
    _require(
        type(commitment) is Commitment
        and type(episode) is Episode
        and commitment.episode_id == episode.episode_id
        and commitment.input_sha256 == episode.observation.input_sha256,
        "episode_join",
        "forecast and labels must share the episode and permitted input identity",
    )
    errors = tuple(
        _moment(
            [
                abs(_finite(prediction - target))
                for prediction, target in zip(
                    row.values_pa, episode.targets_pa, strict=True
                )
            ],
            rms=False,
        )
        for row in commitment.forecasts
    )
    return EpisodeEvaluation(commitment, episode, errors)


@dataclass(frozen=True, slots=True)
class Selection:
    reference: Reference
    grid_sha256: tuple[str, ...]
    development_sha256: str

    def __post_init__(self) -> None:
        _require(
            type(self.reference) is Reference
            and type(self.grid_sha256) is tuple
            and all(type(value) is str for value in self.grid_sha256)
            and len(self.grid_sha256) == len(set(self.grid_sha256)) == 4
            and self.reference.sha256 in self.grid_sha256,
            "development_selection",
            "selected artifact must belong to the complete frozen grid",
        )
        for value in (*self.grid_sha256, self.development_sha256):
            _digest(value)

    def record(self) -> dict[str, Any]:
        return {
            "schema": "prisoma.e1-development-selection.v1",
            "reference_sha256": self.reference.sha256,
            "grid_sha256": self.grid_sha256,
            "development_sha256": self.development_sha256,
            "criterion": "episode_mean_mae_exact_larger_lambda_tie",
        }

    @property
    def sha256(self) -> str:
        return content_digest(self.record())


def load_selection(
    payload: bytes, *, expected_sha256: str, references: tuple[Reference, ...]
) -> Selection:
    """Rejoin a previously selected record to the same four frozen artifacts."""
    _grid(references)
    value = _record_bytes(payload, expected_sha256)
    _require(
        set(value)
        == {
            "schema",
            "reference_sha256",
            "grid_sha256",
            "development_sha256",
            "criterion",
        }
        and value["grid_sha256"] == [row.sha256 for row in references],
        "development_selection",
        "selected complete ordered grid required",
    )
    matched = [row for row in references if row.sha256 == value["reference_sha256"]]
    _require(
        len(matched) == 1, "development_selection", "one selected reference required"
    )
    selection = Selection(
        matched[0], tuple(value["grid_sha256"]), value["development_sha256"]
    )
    _require(
        canonical(selection.record()) == payload,
        "development_selection",
        "selected criterion must match",
    )
    return selection


def choose_lambda(
    references: tuple[Reference, ...], development: tuple[EpisodeEvaluation, ...]
) -> Selection:
    _grid(references)
    _complete_split(development, "development")
    identities = tuple(row.sha256 for row in references)
    for row in development:
        expected = commit_collection(
            "development", row.episode_id, row.episode.observation, references
        )
        _require(
            row.commitment.sha256 == expected.sha256
            and row == evaluate_episode(row.commitment, row.episode),
            "development_selection",
            "every candidate forecast must match its frozen artifact and score",
        )
    scores = tuple(
        _moment([row.mae_pa[index] for row in development], rms=False)
        for index in range(4)
    )
    selected = min(
        range(4), key=lambda index: (scores[index], -references[index].penalty)
    )
    return Selection(
        references[selected],
        identities,
        content_digest([asdict(row) for row in development]),
    )


def commit_selected(
    selection: Selection, episode_id: str, observation: Landmark
) -> Commitment:
    _require(
        type(selection) is Selection, "holdout_lock", "typed frozen selection required"
    )
    reference = selection.reference
    primary = ForecastVector(reference.sha256, forecast(reference, observation))
    rows = (
        primary,
        ForecastVector("constant", (reference.training_mean_pa,) * 5),
        ForecastVector("persistence", (observation.values[7],) * 5),
    )
    return Commitment(
        episode_id,
        "heldout",
        observation.input_sha256,
        rows,
        primary.predictor_id,
        primary.recommendation,
        selection.sha256,
    )


def _paired_interval(
    differences: tuple[float, ...], indices: np.ndarray
) -> dict[str, Any]:
    means = np.asarray(
        [
            _moment([differences[int(index)] for index in row], rms=False)
            for row in indices
        ],
        dtype=np.float64,
    )
    interval = np.quantile(means, [0.025, 0.975], method="linear")
    _require(
        np.isfinite(interval).all(),
        "bootstrap_numeric",
        "finite percentile interval required",
    )
    return {
        "episode_differences_pa": list(differences),
        "mean_difference_pa": _moment(differences, rms=False),
        "interval_95_pa": [float(value) for value in interval],
    }


def paired_episode_report(
    selection: Selection, heldout: tuple[EpisodeEvaluation, ...]
) -> dict[str, Any]:
    """Report paired forecast arithmetic, without claiming native policy benefit."""
    _numerical_runtime()
    _complete_split(heldout, "heldout")
    for row in heldout:
        expected = commit_selected(selection, row.episode_id, row.episode.observation)
        _require(
            row.commitment.sha256 == expected.sha256
            and row == evaluate_episode(row.commitment, row.episode),
            "holdout_lock",
            "selected artifact and complete episode scores must remain fixed",
        )
    indices = np.random.Generator(np.random.PCG64(104729)).integers(
        0, 32, size=(10000, 32), dtype=np.uint32, endpoint=False
    )
    index_digest = hashlib.sha256(indices.astype("<u4").tobytes(order="C")).hexdigest()
    _require(
        index_digest == BOOTSTRAP_INDICES_SHA256,
        "bootstrap_identity",
        "selected numerical runtime changed prospective resampling",
    )
    comparisons = {
        name: _paired_interval(
            tuple(_finite(row.mae_pa[column] - row.mae_pa[0]) for row in heldout),
            indices,
        )
        for name, column in (("constant", 1), ("persistence", 2))
    }
    margin = _finite(0.05 * selection.reference.training_mean_pa)
    variation = any(len(set(row.episode.targets_pa)) > 1 for row in heldout)
    reasons = []
    if selection.reference.training_mean_pa == 0 or margin == 0:
        reasons.append("zero_or_unrepresentable_useful_margin")
    if not variation:
        reasons.append("no_observed_action_relevant_variation")
    conclusion = (
        "uninformative"
        if reasons
        else "benefit_established"
        if all(row["interval_95_pa"][0] > margin for row in comparisons.values())
        else "null_or_inconclusive"
    )
    return {
        "schema": "prisoma.e1-reference-forecast-report.v1",
        "selection_sha256": selection.sha256,
        "contract_sha256": content_digest(study_contract()),
        "heldout_sha256": content_digest([asdict(row) for row in heldout]),
        "episodes": 32,
        "action_rows": 160,
        "bootstrap_indices_sha256": index_digest,
        "useful_margin_pa": margin,
        "comparisons": comparisons,
        "conclusion": conclusion,
        "uninformative_reasons": reasons,
        "training_action_variation": selection.reference.training_action_variation,
        "policy_comparison": "requires_separate_qualified_actual_selected_and_restored_neutral_execution_join",
        "authority": {
            "canonical_order_verified": False,
            "native_label_ancestry_verified": False,
            "scientific_validation": False,
            "calibrated_posterior": False,
            "release_qualified": False,
        },
    }
