use pid_core::{intrinsic_dimension_levina_bickel, IntrinsicDimConfig, MatRef, Metric, PidError};

mod common;

use common::Rng64;

#[test]
fn intrinsic_dimension_increases_with_embedding_dimension() {
    let mut rng = Rng64::new(0xD1A7_2026);
    let n = 350usize;

    // 1D Gaussian.
    let mut x1 = Vec::with_capacity(n);
    for _ in 0..n {
        x1.push(rng.normal());
    }
    let x1 = MatRef::new(&x1, n, 1).unwrap();

    // 3D Gaussian (independent coords).
    let mut x3 = Vec::with_capacity(n * 3);
    for _ in 0..n {
        x3.push(rng.normal());
        x3.push(rng.normal());
        x3.push(rng.normal());
    }
    let x3 = MatRef::new(&x3, n, 3).unwrap();

    let cfg = IntrinsicDimConfig {
        k: 10,
        metric: Metric::Chebyshev,
    };

    let d1 = intrinsic_dimension_levina_bickel(x1, &cfg).unwrap();
    let d3 = intrinsic_dimension_levina_bickel(x3, &cfg).unwrap();

    assert!(d1.is_finite() && d1 > 0.0, "d1={d1}");
    assert!(d3.is_finite() && d3 > 0.0, "d3={d3}");
    assert!(d3 > d1 + 0.5, "expected d3>d1, got d1={d1} d3={d3}");
}

#[test]
fn intrinsic_dimension_errors_on_duplicate_points() {
    let n = 50usize;
    let x = vec![0.0f64; n];
    let x = MatRef::new(&x, n, 1).unwrap();
    let cfg = IntrinsicDimConfig::default();

    let err = intrinsic_dimension_levina_bickel(x, &cfg).unwrap_err();
    assert!(
        matches!(err, PidError::NumericalInstability { .. }),
        "unexpected error: {err:?}"
    );
}
