import math

import numpy as np
import pytest

pid = pytest.importorskip("pid_core_rs")


SUPPORT = "regular_full_dimensional_absolutely_continuous"
PROVENANCE = {
    "support_assertion": SUPPORT,
    "preprocessing_description": "consumer conformance fixture; no preprocessing",
    "observation_model_description": "synthetic continuous observations",
    "dependence_model_description": "rows treated as independent draws",
}


def sample_arrays(n=80):
    rng = np.random.default_rng(17)
    x = rng.normal(size=(n, 1))
    y = rng.normal(size=(n, 1))
    t = x + 0.25 * y
    return np.ascontiguousarray(x), np.ascontiguousarray(y), np.ascontiguousarray(t)


def repeated_gate(rows, repetitions=16):
    repeated = rows * repetitions
    return tuple(
        np.asarray([[row[index]] for row in repeated], dtype=np.int64)
        for index in range(len(rows[0]))
    )


def test_stable_v1_surface_excludes_legacy_scalar_calls():
    assert pid.__version__ == "1.0.0"
    assert pid.stable.compute_mi_report.__name__ == pid.compute_mi_report.__name__
    assert pid.diagnostics.diagnose_continuous_input is not None
    for removed in (
        "compute_mi",
        "compute_redundancy",
        "compute_pid2",
        "compute_discrete_pid3",
        "compute_invariants",
    ):
        assert not hasattr(pid, removed), removed


def test_compute_mi_report_requires_explicit_support_and_provenance():
    x, _, t = sample_arrays()
    report = pid.compute_mi_report(x, t, k=3, **PROVENANCE)
    assert isinstance(report, pid.MiReport)
    assert report.status == "conditional_continuous"
    assert report.support_assertion == SUPPORT
    assert report.n_samples == len(x)
    assert np.isfinite(report.value_nats)
    assert (
        report.provenance.preprocessing_description
        == PROVENANCE["preprocessing_description"]
    )

    with pytest.raises(TypeError):
        pid.compute_mi_report(x, t, k=3)


def test_categorical_sxpid2_returns_typed_measure_specific_atoms():
    s1, s2, target = repeated_gate([(0, 0, 0), (0, 1, 1), (1, 0, 1), (1, 1, 0)])
    result = pid.compute_categorical_sxpid2(s1, s2, target)
    assert isinstance(result, pid.SxPid2Result)
    atoms = (
        result.redundancy,
        result.unique_s1,
        result.unique_s2,
        result.synergy,
    )
    assert all(np.isfinite(atom.net_nats) for atom in atoms)
    assert sum(atom.net_nats for atom in atoms) == pytest.approx(
        result.mi_s1s2_t_nats, abs=1e-12
    )


def test_categorical_sxpid3_and_imin_remain_distinct_measures():
    s0, s1, s2, target = repeated_gate(
        [
            (0, 0, 0, 0),
            (0, 0, 1, 1),
            (0, 1, 0, 1),
            (0, 1, 1, 0),
            (1, 0, 0, 1),
            (1, 0, 1, 0),
            (1, 1, 0, 0),
            (1, 1, 1, 1),
        ]
    )
    lattice = pid.compute_categorical_sxpid3(s0, s1, s2, target)
    assert isinstance(lattice, pid.SxPidLatticeResult)
    assert len(lattice.entries) == 18
    assert all(np.isfinite(entry.atom.net_nats) for entry in lattice.entries)

    copy_s1, copy_s2, copy_target = repeated_gate(
        [(0, 0, 0), (0, 1, 1), (1, 0, 2), (1, 1, 3)]
    )
    sx = pid.compute_categorical_sxpid2(copy_s1, copy_s2, copy_target)
    imin = pid.compute_categorical_imin_pid2(copy_s1, copy_s2, copy_target)
    assert sx.redundancy.net_nats == pytest.approx(math.log(4.0 / 3.0), abs=1e-12)
    assert imin.redundancy_nats == pytest.approx(math.log(2.0), abs=1e-12)
    assert sx.redundancy.net_nats != pytest.approx(imin.redundancy_nats)
