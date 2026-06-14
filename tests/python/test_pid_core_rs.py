import numpy as np
import pytest

pid = pytest.importorskip("pid_core_rs")


def sample_arrays(n=80):
    x = np.linspace(-1.0, 1.0, n, dtype=np.float64).reshape(-1, 1)
    y = np.cos(np.linspace(0.0, 3.0, n, dtype=np.float64)).reshape(-1, 1)
    t = (x[:, 0] + 0.25 * y[:, 0]).reshape(-1, 1)
    return np.ascontiguousarray(x), np.ascontiguousarray(y), np.ascontiguousarray(t)


def test_compute_mi_exposes_estimator_options():
    x, y, _ = sample_arrays()
    mi = pid.compute_mi(x, y, k=3, tie_epsilon=0.0, negative_handling="allow")
    assert np.isfinite(mi)


def test_pid2_and_invariants_bindings():
    s1, s2, t = sample_arrays()
    out = pid.compute_pid2(s1, s2, t, k=3)
    assert {"redundancy", "unique_s1", "unique_s2", "synergy"} <= set(out)
    assert all(np.isfinite(v) for v in out.values())

    inv = pid.compute_invariants(s1, s2, t, k=3)
    assert {"mi_s1_t", "mi_s2_t", "mi_s1s2_t", "co_information", "r_bar", "v_bar"} <= set(inv)
    assert all(np.isfinite(v) for v in inv.values())


def test_redundancy_rejects_unvalidated_hyperbolic_metric():
    s1, s2, t = sample_arrays()
    with pytest.raises(RuntimeError):
        pid.compute_redundancy(s1, s2, t, metric="hyperbolic")


def test_compute_discrete_pid3_binding():
    # Discrete 3-source PID over the 18-atom lattice (Williams-Beer I_min).
    n = 120
    rng = np.random.default_rng(0)
    s0 = rng.standard_normal((n, 1))
    s1 = rng.standard_normal((n, 1))
    s2 = rng.standard_normal((n, 1))
    t = (s0[:, 0] + s1[:, 0]).reshape(-1, 1)
    out = pid.compute_discrete_pid3(
        np.ascontiguousarray(s0),
        np.ascontiguousarray(s1),
        np.ascontiguousarray(s2),
        np.ascontiguousarray(t),
        num_bins=4,
    )
    # 3-source SxPID lattice has 18 atoms; all finite.
    assert len(out) == 18
    assert all(np.isfinite(v) for v in out.values())
    # I_min atoms are non-negative on empirical distributions (grandplan 8.1.6).
    assert all(v >= -1e-9 for v in out.values())
