"""Group-level deletion ranking-sensitivity diagnostic.

Deletion asks a deliberately narrow question: does an attribution's feature ranking
identify baseline replacements that change a declared scalar output sooner than a
uniformly random ranking?  Absolute output change is called *ranking sensitivity*
here.  It is not causal or mechanistic faithfulness: the baseline may be
out-of-distribution, features may be dependent, and replacement can create inputs the
model never encountered.

The gate is dataset-level and fail closed:

* every validation case declares its baseline, independent group, and underlying
  units;
* validation groups/units must be disjoint from each other and from all declared
  method-selection groups/units;
* the baseline name, provenance, grouping provenance, split names, and frozen gate
  identifier are mandatory;
* each case is compared with a deterministic Monte Carlo random-ranking reference;
* a group is a win only when its mean absolute deletion sensitivity exceeds the
  random-reference mean and
  its plus-one randomization-tail probability is below one half;
* the final p-value is a conservative one-sided binomial tail for the predeclared
  compound win rule across independent groups.

There is no effect-size ``margin`` or post-hoc standard-error multiplier.  ``alpha``
and the minimum group count belong to the caller's frozen gate.  Configuration
validation rejects a group count that could never attain ``alpha`` even if every
group won, and rejects a random reference whose worst-case binomial Monte Carlo
standard error exceeds ``alpha``.

The public ``faithfulness_check`` name is retained only because the canonical run-log
schema calls its boolean field ``faithfulness_check``.  New code should use
``ranking_sensitivity_check`` and report the result's diagnostic/status/reason.
"""

from __future__ import annotations

import hashlib
import json
import math
import unicodedata
from collections.abc import Callable, Sequence
from dataclasses import dataclass, replace
from typing import Literal

import numpy as np

PredictFn = Callable[[np.ndarray], float]
GateStatus = Literal["passed", "failed", "abstained"]

MAX_VALIDATION_CASES = 1_024
MAX_RANDOM_RANKINGS = 100_000
MAX_ABLATION_EVALUATIONS = 5_000_000
MAX_FEATURES_PER_CASE = 1_024
MAX_UNITS_PER_CASE = 1_024
MAX_SELECTION_IDENTIFIERS = 4_096
MAX_IDENTIFIER_BYTES = 1_024
MAX_PROVENANCE_BYTES = 16 * 1_024
MAX_SEED = 2**64 - 1


@dataclass(frozen=True)
class AttributionValidationCase:
    """One predeclared validation case and its attribution ranking.

    ``unit_ids`` identify the underlying episodes/objects/subjects.  A unit may not
    occur in two validation cases, even if their group labels differ; this prevents a
    caller from manufacturing independent-looking groups from repeated observations.
    """

    case_id: str
    group_id: str
    unit_ids: tuple[str, ...]
    x: np.ndarray
    attribution: np.ndarray
    baseline: np.ndarray


@dataclass(frozen=True)
class RankingSensitivityGate:
    """Frozen design/provenance for a deletion ranking-sensitivity decision."""

    frozen_gate_id: str
    baseline_name: str
    baseline_provenance: str
    validation_split: str
    selection_split: str
    grouping_provenance: str
    predictor_determinism_provenance: str
    selection_group_ids: tuple[str, ...]
    selection_unit_ids: tuple[str, ...]
    alpha: float
    min_groups: int
    n_steps: int
    n_random_rankings: int
    seed: int


@dataclass(frozen=True)
class FaithfulnessResult:
    """Result retained under the historical type name for schema compatibility."""

    diagnostic: str
    status: GateStatus
    reason: str
    passed: bool
    method_sensitivity: float | None
    random_sensitivity: float | None
    random_sensitivity_std: float | None
    group_win_binomial_p_value: float | None
    alpha: float
    n_steps: int
    n_cases: int
    n_groups: int
    winning_groups: int
    n_random_rankings: int
    random_reference_se_bound: float
    group_contrasts: tuple[float, ...]
    group_randomization_p_values: tuple[float, ...]
    # Both curves are |f(x) - f(x_ablated)| series.  They are descriptive means
    # across validation groups, not causal response curves or uncertainty bands.
    sensitivity_curve: list[float]
    random_sensitivity_curve: list[float]
    baseline_name: str
    baseline_provenance: str
    frozen_gate_id: str


def _canonical_identifier(
    value: object, field: str, *, max_bytes: int = MAX_IDENTIFIER_BYTES
) -> str:
    if type(value) is not str:
        raise ValueError(f"{field} must be a string")
    if not value or value != value.strip():
        raise ValueError(f"{field} must be nonempty without surrounding whitespace")
    if unicodedata.normalize("NFC", value) != value:
        raise ValueError(f"{field} must be NFC-normalized")
    if any(unicodedata.category(character) in {"Cc", "Cs"} for character in value):
        raise ValueError(f"{field} must not contain control or surrogate characters")
    if len(value.encode("utf-8")) > max_bytes:
        raise ValueError(f"{field} must contain at most {max_bytes} UTF-8 bytes")
    return value


def _validate_exact_int(
    value: object,
    field: str,
    *,
    minimum: int,
    maximum: int | None = None,
) -> int:
    if isinstance(value, bool) or not isinstance(value, (int, np.integer)):
        raise ValueError(f"{field} must be an integer")
    parsed = int(value)
    if parsed < minimum:
        raise ValueError(f"{field} must be >= {minimum}")
    if maximum is not None and parsed > maximum:
        raise ValueError(f"{field} must be <= {maximum}")
    return parsed


def _validate_gate(gate: RankingSensitivityGate) -> None:
    if not isinstance(gate, RankingSensitivityGate):
        raise ValueError("gate must be a RankingSensitivityGate")
    for field in (
        "frozen_gate_id",
        "baseline_name",
        "validation_split",
        "selection_split",
    ):
        _canonical_identifier(getattr(gate, field), field)
    for field in (
        "baseline_provenance",
        "grouping_provenance",
        "predictor_determinism_provenance",
    ):
        _canonical_identifier(
            getattr(gate, field), field, max_bytes=MAX_PROVENANCE_BYTES
        )

    if isinstance(gate.alpha, bool) or not isinstance(
        gate.alpha, (float, int, np.floating, np.integer)
    ):
        raise ValueError("alpha must be a finite number")
    alpha = float(gate.alpha)
    if not math.isfinite(alpha) or not 0.0 < alpha < 0.5:
        raise ValueError("alpha must be finite and strictly between 0 and 0.5")

    min_groups = _validate_exact_int(
        gate.min_groups,
        "min_groups",
        minimum=2,
        maximum=MAX_VALIDATION_CASES,
    )
    required_groups = math.ceil(-math.log2(alpha))
    if required_groups > MAX_VALIDATION_CASES:
        raise ValueError(
            "alpha cannot be attained within the independent-group resource limit"
        )
    if min_groups < required_groups:
        raise ValueError(
            "min_groups cannot attain alpha under the group-win binomial rule; "
            f"need at least {required_groups}"
        )

    _validate_exact_int(
        gate.n_steps, "n_steps", minimum=2, maximum=MAX_FEATURES_PER_CASE
    )
    n_random = _validate_exact_int(
        gate.n_random_rankings,
        "n_random_rankings",
        minimum=2,
        maximum=MAX_RANDOM_RANKINGS,
    )
    # For a Bernoulli tail estimate, sqrt(p(1-p)/n) <= 0.5/sqrt(n).  Requiring
    # this worst-case bound to be no wider than the frozen alpha makes the
    # resolution criterion explicit and scale-aware rather than another magic N.
    se_bound = 0.5 / math.sqrt(n_random)
    if se_bound > alpha:
        if alpha < 0.5 / math.sqrt(MAX_RANDOM_RANKINGS):
            raise ValueError(
                "random-ranking reference would require more than the supported "
                f"{MAX_RANDOM_RANKINGS} rankings"
            )
        required_random = math.ceil((0.5 / alpha) ** 2)
        raise ValueError(
            "random-ranking reference is under-resolved for alpha; "
            f"need at least {required_random} rankings"
        )
    _validate_exact_int(gate.seed, "seed", minimum=0, maximum=MAX_SEED)

    for field in ("selection_group_ids", "selection_unit_ids"):
        values = getattr(gate, field)
        if type(values) is not tuple:
            raise ValueError(f"{field} must be a tuple")
        if not values:
            raise ValueError(f"{field} must be nonempty")
        if len(values) > MAX_SELECTION_IDENTIFIERS:
            raise ValueError(
                f"{field} must contain at most {MAX_SELECTION_IDENTIFIERS} values"
            )
        normalized = [_canonical_identifier(value, field) for value in values]
        if len(set(normalized)) != len(normalized):
            raise ValueError(f"{field} must not contain duplicates")

    expected_gate_id = f"sha256:{ranking_gate_content_sha256(gate)}"
    if gate.frozen_gate_id != expected_gate_id:
        raise ValueError(
            "frozen_gate_id must be the content-derived ranking gate identifier "
            f"{expected_gate_id}"
        )


def ranking_gate_manifest(gate: RankingSensitivityGate) -> dict[str, object]:
    """Return the canonical gate content, excluding its derived identifier."""

    return {
        "schema": "prisoma-ranking-sensitivity-gate-v2",
        "baseline_name": gate.baseline_name,
        "baseline_provenance": gate.baseline_provenance,
        "validation_split": gate.validation_split,
        "selection_split": gate.selection_split,
        "grouping_provenance": gate.grouping_provenance,
        "predictor_determinism_provenance": gate.predictor_determinism_provenance,
        "selection_group_ids": list(gate.selection_group_ids),
        "selection_unit_ids": list(gate.selection_unit_ids),
        "alpha_f64": format(float(gate.alpha), ".17g"),
        "min_groups": int(gate.min_groups),
        "n_steps": int(gate.n_steps),
        "n_random_rankings": int(gate.n_random_rankings),
        "seed": int(gate.seed),
        "ranking_transform": "descending_absolute_magnitude",
        "tie_policy": "abstain_on_any_exact_magnitude_tie",
        "group_win_rule": (
            "method_mean_absolute_deletion_sensitivity_gt_random_mean_and_"
            "plus_one_randomization_tail_lt_half"
        ),
        "group_aggregation": "one_sided_binomial_tail_p0_half",
    }


def ranking_gate_content_sha256(gate: RankingSensitivityGate) -> str:
    payload = json.dumps(
        ranking_gate_manifest(gate),
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def bind_ranking_gate(gate: RankingSensitivityGate) -> RankingSensitivityGate:
    """Return ``gate`` with its content-derived frozen identifier installed."""

    if not isinstance(gate, RankingSensitivityGate):
        raise ValueError("gate must be a RankingSensitivityGate")
    return replace(gate, frozen_gate_id=f"sha256:{ranking_gate_content_sha256(gate)}")


def _predict_scalar(predict: PredictFn, x: np.ndarray, context: str) -> float:
    value = predict(x)
    if isinstance(value, (bool, np.bool_)):
        raise ValueError(f"predict returned a boolean for {context}")
    try:
        array = np.asarray(value)
    except (TypeError, ValueError) as error:
        raise ValueError(
            f"predict must return one numeric scalar for {context}"
        ) from error
    if array.shape != ():
        raise ValueError(f"predict must return one scalar for {context}")
    if not np.issubdtype(array.dtype, np.number) or np.issubdtype(
        array.dtype, np.complexfloating
    ):
        raise ValueError(f"predict must return one real numeric scalar for {context}")
    try:
        scalar = float(array)
    except (TypeError, ValueError) as error:
        raise ValueError(
            f"predict must return one numeric scalar for {context}"
        ) from error
    if not math.isfinite(scalar):
        raise ValueError(f"predict returned a non-finite scalar for {context}")
    return scalar


def _deletion_drop_curve(
    predict: PredictFn,
    x: np.ndarray,
    order: np.ndarray,
    baseline: np.ndarray,
    n_steps: int,
    f0: float,
    context: str,
) -> np.ndarray:
    """Absolute output change after each exhaustive deletion partition."""

    flat = x.reshape(-1).copy()
    base_flat = baseline.reshape(-1)
    drops: list[float] = []
    # array_split consumes every feature, unlike a floor-sized loop that can leave
    # a remainder unablated.
    for indices in np.array_split(order, n_steps):
        flat[indices] = base_flat[indices]
        output = _predict_scalar(predict, flat.reshape(x.shape), context)
        drop = abs(f0 - output)
        if not math.isfinite(drop):
            raise ValueError(f"output-change magnitude is non-finite for {context}")
        drops.append(drop)
    return np.asarray(drops, dtype=np.float64)


def _stream_seed(seed: int, case_id: str, stream: str) -> int:
    payload = f"{seed}\0{case_id}\0{stream}".encode()
    return int.from_bytes(hashlib.sha256(payload).digest()[:8], "little")


def _binomial_win_tail(wins: int, groups: int) -> float:
    numerator = sum(math.comb(groups, count) for count in range(wins, groups + 1))
    return float(numerator / (2**groups))


def _result_without_estimate(
    gate: RankingSensitivityGate,
    *,
    status: GateStatus,
    reason: str,
    n_cases: int,
    n_groups: int,
) -> FaithfulnessResult:
    return FaithfulnessResult(
        diagnostic="deletion_ranking_sensitivity",
        status=status,
        reason=reason,
        passed=False,
        method_sensitivity=None,
        random_sensitivity=None,
        random_sensitivity_std=None,
        group_win_binomial_p_value=None,
        alpha=float(gate.alpha),
        n_steps=int(gate.n_steps),
        n_cases=n_cases,
        n_groups=n_groups,
        winning_groups=0,
        n_random_rankings=int(gate.n_random_rankings),
        random_reference_se_bound=0.5 / math.sqrt(gate.n_random_rankings),
        group_contrasts=(),
        group_randomization_p_values=(),
        sensitivity_curve=[],
        random_sensitivity_curve=[],
        baseline_name=gate.baseline_name,
        baseline_provenance=gate.baseline_provenance,
        frozen_gate_id=gate.frozen_gate_id,
    )


def _numeric_array(value: object, context: str) -> np.ndarray:
    try:
        raw = np.asarray(value)
    except (TypeError, ValueError) as error:
        raise ValueError(f"{context} must be a rectangular numeric array") from error
    if raw.size == 0:
        raise ValueError(f"{context} must be nonempty")
    if raw.size > MAX_FEATURES_PER_CASE:
        raise ValueError(
            f"{context} must contain at most {MAX_FEATURES_PER_CASE} values"
        )
    if np.issubdtype(raw.dtype, np.bool_) or not np.issubdtype(raw.dtype, np.number):
        raise ValueError(f"{context} must contain real numeric values")
    if np.issubdtype(raw.dtype, np.complexfloating):
        raise ValueError(f"{context} must contain real numeric values")
    try:
        array = np.asarray(raw, dtype=np.float64)
    except (TypeError, ValueError, OverflowError) as error:
        raise ValueError(f"{context} must be representable as float64") from error
    if not np.all(np.isfinite(array)):
        raise ValueError(f"{context} must contain only finite values")
    return array


def _validated_case_arrays(
    case: AttributionValidationCase, n_steps: int
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    x = _numeric_array(case.x, f"case {case.case_id!r} x")
    attribution = _numeric_array(case.attribution, f"case {case.case_id!r} attribution")
    baseline = _numeric_array(case.baseline, f"case {case.case_id!r} baseline")
    if x.size < n_steps:
        raise ValueError(
            f"case {case.case_id!r} has fewer features than the frozen n_steps"
        )
    if attribution.shape != x.shape:
        raise ValueError(f"case {case.case_id!r} attribution shape must match x")
    if baseline.shape != x.shape:
        raise ValueError(f"case {case.case_id!r} baseline shape must match x")
    return x, attribution, baseline


def _stable_nonnegative_mean(values: np.ndarray) -> float:
    maximum = float(np.max(values))
    if maximum == 0.0:
        return 0.0
    result = maximum * float(np.mean(values / maximum))
    if not math.isfinite(result):
        raise ValueError("ranking-sensitivity mean is non-finite")
    return result


def _stable_nonnegative_column_mean(values: np.ndarray) -> np.ndarray:
    maxima = np.max(values, axis=0)
    scaled = np.divide(
        values,
        maxima,
        out=np.zeros_like(values),
        where=maxima[None, :] != 0.0,
    )
    result = maxima * np.mean(scaled, axis=0)
    if not np.all(np.isfinite(result)):
        raise ValueError("ranking-sensitivity curve mean is non-finite")
    return result


def _stable_sample_std(values: np.ndarray) -> float:
    maximum = float(np.max(np.abs(values)))
    if maximum == 0.0:
        return 0.0
    result = maximum * float(np.std(values / maximum, ddof=1))
    if not math.isfinite(result):
        raise ValueError("random-reference standard deviation is non-finite")
    return result


def ranking_sensitivity_check(
    predict: PredictFn,
    cases: Sequence[AttributionValidationCase],
    *,
    gate: RankingSensitivityGate,
) -> FaithfulnessResult:
    """Evaluate a frozen group-level deletion ranking-sensitivity gate.

    Malformed arrays, identifiers, parameters, or predictor outputs raise
    ``ValueError``.  Scientifically unusable but well-formed designs (insufficient
    groups, selection/validation leakage, or no ranking resolution) return a typed
    non-passing result so they can still be recorded in the run log.
    """

    if not callable(predict):
        raise ValueError("predict must be callable")
    _validate_gate(gate)
    if isinstance(cases, (str, bytes, np.ndarray)) or not isinstance(cases, Sequence):
        raise ValueError("cases must be a sequence of AttributionValidationCase values")
    if not cases:
        raise ValueError("cases must be nonempty")
    if len(cases) > MAX_VALIDATION_CASES:
        raise ValueError(f"cases must contain at most {MAX_VALIDATION_CASES} entries")

    case_ids: set[str] = set()
    group_ids: set[str] = set()
    validation_units: set[str] = set()
    prepared: list[
        tuple[AttributionValidationCase, np.ndarray, np.ndarray, np.ndarray]
    ] = []
    duplicate_group = False
    overlapping_unit = False
    for index, case in enumerate(cases):
        if not isinstance(case, AttributionValidationCase):
            raise ValueError(f"cases[{index}] must be an AttributionValidationCase")
        case_id = _canonical_identifier(case.case_id, f"cases[{index}].case_id")
        group_id = _canonical_identifier(case.group_id, f"cases[{index}].group_id")
        if case_id in case_ids:
            raise ValueError("case_id values must be unique")
        case_ids.add(case_id)
        if group_id in group_ids:
            duplicate_group = True
        group_ids.add(group_id)
        if type(case.unit_ids) is not tuple or not case.unit_ids:
            raise ValueError(f"case {case_id!r} unit_ids must be a nonempty tuple")
        if len(case.unit_ids) > MAX_UNITS_PER_CASE:
            raise ValueError(
                f"case {case_id!r} unit_ids must contain at most "
                f"{MAX_UNITS_PER_CASE} values"
            )
        local_units = {
            _canonical_identifier(unit, f"case {case_id!r} unit_ids")
            for unit in case.unit_ids
        }
        if len(local_units) != len(case.unit_ids):
            raise ValueError(f"case {case_id!r} unit_ids must not contain duplicates")
        if validation_units.intersection(local_units):
            overlapping_unit = True
        validation_units.update(local_units)
        x, attribution, baseline = _validated_case_arrays(case, gate.n_steps)
        prepared.append((case, x, attribution, baseline))

    if gate.validation_split == gate.selection_split:
        return _result_without_estimate(
            gate,
            status="abstained",
            reason="selection_validation_split_not_disjoint",
            n_cases=len(cases),
            n_groups=len(group_ids),
        )
    if duplicate_group:
        return _result_without_estimate(
            gate,
            status="abstained",
            reason="validation_groups_not_disjoint",
            n_cases=len(cases),
            n_groups=len(group_ids),
        )
    if overlapping_unit:
        return _result_without_estimate(
            gate,
            status="abstained",
            reason="validation_units_not_disjoint",
            n_cases=len(cases),
            n_groups=len(group_ids),
        )
    if group_ids.intersection(
        gate.selection_group_ids
    ) or validation_units.intersection(gate.selection_unit_ids):
        return _result_without_estimate(
            gate,
            status="abstained",
            reason="selection_validation_leakage",
            n_cases=len(cases),
            n_groups=len(group_ids),
        )
    if len(group_ids) < gate.min_groups:
        return _result_without_estimate(
            gate,
            status="abstained",
            reason="insufficient_independent_validation_groups",
            n_cases=len(cases),
            n_groups=len(group_ids),
        )

    evaluations = len(cases) * (3 + (1 + gate.n_random_rankings) * gate.n_steps)
    if evaluations > MAX_ABLATION_EVALUATIONS:
        raise ValueError(
            "frozen deletion design exceeds the ablation-evaluation resource budget"
        )

    for case, x, attribution, baseline in prepared:
        if np.unique(np.abs(attribution).reshape(-1)).size < attribution.size:
            return _result_without_estimate(
                gate,
                status="abstained",
                reason=f"ranking_ties_unresolved:{case.case_id}",
                n_cases=len(cases),
                n_groups=len(group_ids),
            )
        if np.array_equal(x, baseline):
            return _result_without_estimate(
                gate,
                status="failed",
                reason=f"baseline_does_not_perturb_case:{case.case_id}",
                n_cases=len(cases),
                n_groups=len(group_ids),
            )

    method_sensitivities: list[float] = []
    random_sensitivities_by_case: list[np.ndarray] = []
    method_curves: list[np.ndarray] = []
    random_curves: list[np.ndarray] = []

    for case, x, attribution, baseline in prepared:
        f0 = _predict_scalar(predict, x, f"case {case.case_id!r} original input")
        f0_repeat = _predict_scalar(
            predict, x, f"case {case.case_id!r} determinism repeat"
        )
        if f0_repeat != f0:
            return _result_without_estimate(
                gate,
                status="abstained",
                reason=f"predictor_not_deterministic:{case.case_id}",
                n_cases=len(cases),
                n_groups=len(group_ids),
            )
        magnitudes = np.abs(attribution).reshape(-1)
        order = np.argsort(-magnitudes, kind="stable")
        method_curve = _deletion_drop_curve(
            predict,
            x,
            order,
            baseline,
            gate.n_steps,
            f0,
            f"case {case.case_id!r} method deletion",
        )
        method_sensitivity = _stable_nonnegative_mean(method_curve)

        random_rng = np.random.default_rng(
            _stream_seed(gate.seed, case.case_id, "random-reference")
        )
        case_random_sensitivities = np.empty(gate.n_random_rankings, dtype=np.float64)
        random_curve_mean = np.zeros(gate.n_steps, dtype=np.float64)
        for draw in range(gate.n_random_rankings):
            order = random_rng.permutation(x.size)
            curve = _deletion_drop_curve(
                predict,
                x,
                order,
                baseline,
                gate.n_steps,
                f0,
                f"case {case.case_id!r} random deletion {draw}",
            )
            case_random_sensitivities[draw] = _stable_nonnegative_mean(curve)
            random_curve_mean += (curve - random_curve_mean) / (draw + 1)

        f0_final = _predict_scalar(
            predict, x, f"case {case.case_id!r} final determinism check"
        )
        if f0_final != f0:
            return _result_without_estimate(
                gate,
                status="abstained",
                reason=f"predictor_not_deterministic:{case.case_id}",
                n_cases=len(cases),
                n_groups=len(group_ids),
            )

        method_sensitivities.append(method_sensitivity)
        random_sensitivities_by_case.append(case_random_sensitivities)
        method_curves.append(method_curve)
        random_curves.append(random_curve_mean)

    method_array = np.asarray(method_sensitivities, dtype=np.float64)
    random_matrix = np.stack(random_sensitivities_by_case)
    random_means = np.asarray(
        [_stable_nonnegative_mean(row) for row in random_matrix], dtype=np.float64
    )
    contrasts = method_array - random_means
    group_tail_p = np.asarray(
        [
            (1 + int(np.count_nonzero(null_scores >= observed)))
            / (gate.n_random_rankings + 1)
            for observed, null_scores in zip(method_array, random_matrix, strict=True)
        ],
        dtype=np.float64,
    )
    wins = int(np.count_nonzero((contrasts > 0.0) & (group_tail_p < 0.5)))
    p_value = _binomial_win_tail(wins, len(cases))
    passed = bool(p_value <= gate.alpha)
    status: GateStatus = "passed" if passed else "failed"
    reason = (
        "ranking_sensitivity_gate_passed"
        if passed
        else "ranking_not_better_than_random_across_groups"
    )

    all_random = random_matrix.reshape(-1)
    return FaithfulnessResult(
        diagnostic="deletion_ranking_sensitivity",
        status=status,
        reason=reason,
        passed=passed,
        method_sensitivity=_stable_nonnegative_mean(method_array),
        random_sensitivity=_stable_nonnegative_mean(all_random),
        random_sensitivity_std=_stable_sample_std(all_random),
        group_win_binomial_p_value=p_value,
        alpha=float(gate.alpha),
        n_steps=int(gate.n_steps),
        n_cases=len(cases),
        n_groups=len(group_ids),
        winning_groups=wins,
        n_random_rankings=int(gate.n_random_rankings),
        random_reference_se_bound=0.5 / math.sqrt(gate.n_random_rankings),
        group_contrasts=tuple(float(value) for value in contrasts),
        group_randomization_p_values=tuple(float(value) for value in group_tail_p),
        sensitivity_curve=_stable_nonnegative_column_mean(
            np.stack(method_curves)
        ).tolist(),
        random_sensitivity_curve=_stable_nonnegative_column_mean(
            np.stack(random_curves)
        ).tolist(),
        baseline_name=gate.baseline_name,
        baseline_provenance=gate.baseline_provenance,
        frozen_gate_id=gate.frozen_gate_id,
    )


def faithfulness_check(
    predict: PredictFn,
    cases: Sequence[AttributionValidationCase],
    *,
    gate: RankingSensitivityGate,
) -> FaithfulnessResult:
    """Compatibility name for :func:`ranking_sensitivity_check`.

    The result measures deletion ranking sensitivity only.  It does not establish
    causal or mechanistic faithfulness.
    """

    return ranking_sensitivity_check(predict, cases, gate=gate)
