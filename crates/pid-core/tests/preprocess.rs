use pid_core::{HashProjector, Jitter, MatRef};

fn mat_equal(a: MatRef<'_>, b: MatRef<'_>, tol: f64) -> bool {
    if a.nrows() != b.nrows() || a.ncols() != b.ncols() {
        return false;
    }
    for i in 0..a.nrows() {
        for (&av, &bv) in a.row(i).iter().zip(b.row(i).iter()) {
            if (av - bv).abs() > tol {
                return false;
            }
        }
    }
    true
}

#[test]
fn hash_projector_is_deterministic() {
    let n = 4;
    let d = 7;
    let x: Vec<f64> = (0..(n * d)).map(|i| (i as f64) * 0.01).collect();
    let x = MatRef::new(&x, n, d).unwrap();

    let p1 = HashProjector::new(d, 3, 123).unwrap();
    let p2 = HashProjector::new(d, 3, 123).unwrap();

    let y1 = p1.transform(x).unwrap();
    let y2 = p2.transform(x).unwrap();

    assert!(mat_equal(y1.as_ref(), y2.as_ref(), 0.0));
}

#[test]
fn hash_projector_shapes_and_finite() {
    let n = 3;
    let d = 5;
    let x: Vec<f64> = (0..(n * d)).map(|i| (i as f64) - 3.0).collect();
    let x = MatRef::new(&x, n, d).unwrap();

    let p = HashProjector::new(d, 2, 7).unwrap();
    let y = p.transform(x).unwrap();
    let y = y.as_ref();

    assert_eq!(y.nrows(), n);
    assert_eq!(y.ncols(), 2);
    for i in 0..n {
        assert!(y.row(i).iter().all(|v| v.is_finite()));
    }
}

#[test]
fn jitter_std_zero_is_identity() {
    let n = 2;
    let d = 4;
    let x: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, -1.0, -2.0, -3.0, -4.0];
    let x = MatRef::new(&x, n, d).unwrap();

    let j = Jitter::new(0.0, 999).unwrap();
    let y = j.apply(x).unwrap();
    assert!(mat_equal(x, y.as_ref(), 0.0));
}

#[test]
fn jitter_is_deterministic_given_seed() {
    let n = 2;
    let d = 6;
    let x: Vec<f64> = (0..(n * d)).map(|i| (i as f64) * 0.1).collect();
    let x = MatRef::new(&x, n, d).unwrap();

    let j1 = Jitter::new(0.01, 2026).unwrap();
    let j2 = Jitter::new(0.01, 2026).unwrap();
    let y1 = j1.apply(x).unwrap();
    let y2 = j2.apply(x).unwrap();

    assert!(mat_equal(y1.as_ref(), y2.as_ref(), 0.0));
}

#[test]
fn preprocess_rejects_invalid_configs() {
    assert!(HashProjector::new(0, 2, 1).is_err());
    assert!(HashProjector::new(2, 0, 1).is_err());
    assert!(Jitter::new(-1.0, 0).is_err());
    assert!(Jitter::new(f64::NAN, 0).is_err());
}
