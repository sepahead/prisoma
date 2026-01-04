use numpy::{PyReadonlyArray2, ToPyArray};
use pid_core::geometry::{
    distance_concentration_stats, gromov_hyperbolicity, intrinsic_dimension_levina_bickel,
    DistanceConcentrationConfig, HyperbolicityConfig, IntrinsicDimConfig,
};
use pid_core::isx::{isx_redundancy, IsxConfig, IsxMethod};
use pid_core::ksg::{ksg_mi, KsgConfig};
use pid_core::matrix::MatRef;
use pid_core::metric::Metric;
use pyo3::prelude::*;

/// Convert a numpy array to MatRef.
/// Returns error if array is not C-contiguous or contains non-finite values.
fn array_to_matref<'a>(arr: &'a PyReadonlyArray2<f64>) -> PyResult<MatRef<'a>> {
    let arr_view = arr.as_array();
    if !arr_view.is_standard_layout() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Array must be C-contiguous",
        ));
    }
    let slice = arr_view.as_slice().ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err("Failed to get array slice (non-contiguous?)")
    })?;
    let (nrows, ncols) = (arr_view.shape()[0], arr_view.shape()[1]);
    
    MatRef::new(slice, nrows, ncols).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid data: {:?}", e))
    })
}

fn parse_metric(name: &str) -> PyResult<Metric> {
    match name.to_lowercase().as_str() {
        "chebyshev" | "linf" | "max" => Ok(Metric::Chebyshev),
        // Experimental research metrics (MI-only, not validated for ISX):
        "hyperbolic" | "lorentz" => Ok(Metric::HyperbolicLorentz),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown metric: '{}'. Valid metrics are: 'chebyshev' (aliases: 'linf', 'max'), \
             'hyperbolic' (alias: 'lorentz', experimental MI-only)",
            name
        ))),
    }
}

/// Compute KSG Mutual Information.
#[pyfunction]
#[pyo3(signature = (x, y, k=3, metric="chebyshev"))]
fn compute_mi(
    x: PyReadonlyArray2<f64>,
    y: PyReadonlyArray2<f64>,
    k: usize,
    metric: &str,
) -> PyResult<f64> {
    let x_mat = array_to_matref(&x)?;
    let y_mat = array_to_matref(&y)?;
    let metric_enum = parse_metric(metric)?;

    let cfg = KsgConfig {
        k,
        metric: metric_enum,
    };

    ksg_mi(x_mat, y_mat, &cfg)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{:?}", e)))
}

/// Compute continuous I_sx_intersect redundancy.
#[pyfunction]
#[pyo3(signature = (s1, s2, target, k=3, method="ehrlich_ksg"))]
fn compute_redundancy(
    s1: PyReadonlyArray2<f64>,
    s2: PyReadonlyArray2<f64>,
    target: PyReadonlyArray2<f64>,
    k: usize,
    method: &str,
) -> PyResult<f64> {
    let s1_mat = array_to_matref(&s1)?;
    let s2_mat = array_to_matref(&s2)?;
    let t_mat = array_to_matref(&target)?;

    let method_enum = match method.to_lowercase().as_str() {
        "ehrlich_ksg" | "continuous" => IsxMethod::EhrlichKsg,
        _ => return Err(pyo3::exceptions::PyValueError::new_err("Unknown method")),
    };

    let cfg = IsxConfig {
        k,
        method: method_enum,
    };

    isx_redundancy(s1_mat, s2_mat, t_mat, &cfg)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{:?}", e)))
}

/// Estimate intrinsic dimension using Levina-Bickel (kNN MLE).
#[pyfunction]
#[pyo3(signature = (x, k=10, metric="chebyshev"))]
fn estimate_intrinsic_dimension(
    x: PyReadonlyArray2<f64>,
    k: usize,
    metric: &str,
) -> PyResult<f64> {
    let x_mat = array_to_matref(&x)?;
    let metric_enum = parse_metric(metric)?;

    let cfg = IntrinsicDimConfig {
        k,
        metric: metric_enum,
    };

    intrinsic_dimension_levina_bickel(x_mat, &cfg)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{:?}", e)))
}

/// Estimate Gromov delta-hyperbolicity via 4-point sampling.
#[pyfunction]
#[pyo3(signature = (x, n_samples=1000, metric="chebyshev", seed=42))]
fn estimate_gromov_delta(
    x: PyReadonlyArray2<f64>,
    n_samples: usize,
    metric: &str,
    seed: u64,
) -> PyResult<f64> {
    let x_mat = array_to_matref(&x)?;
    let metric_enum = parse_metric(metric)?;

    let cfg = HyperbolicityConfig {
        n_samples,
        metric: metric_enum,
        seed,
    };

    gromov_hyperbolicity(x_mat, &cfg)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{:?}", e)))
}

/// Compute distance concentration statistics.
/// Returns a dict: {'pairwise_cv': float, 'nn_mean_ratio': float, ...}
#[pyfunction]
#[pyo3(signature = (x, metric="chebyshev"))]
fn distance_stats(x: PyReadonlyArray2<f64>, metric: &str) -> PyResult<std::collections::HashMap<String, f64>> {
    let x_mat = array_to_matref(&x)?;
    let metric_enum = parse_metric(metric)?;

    let cfg = DistanceConcentrationConfig {
        metric: metric_enum,
    };

    let stats = distance_concentration_stats(x_mat, &cfg)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{:?}", e)))?;

    let mut map = std::collections::HashMap::new();
    map.insert("pairwise_mean".to_string(), stats.pairwise_mean);
    map.insert("pairwise_std".to_string(), stats.pairwise_std);
    map.insert("pairwise_cv".to_string(), stats.pairwise_cv);
    map.insert("nn_mean".to_string(), stats.nn_mean);
    map.insert("nn_cv".to_string(), stats.nn_cv);
    map.insert("nn_over_pairwise_mean".to_string(), stats.nn_over_pairwise_mean);
    Ok(map)
}

#[pymodule]
fn pid_core_rs(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compute_mi, m)?)?;
    m.add_function(wrap_pyfunction!(compute_redundancy, m)?)?;
    m.add_function(wrap_pyfunction!(estimate_intrinsic_dimension, m)?)?;
    m.add_function(wrap_pyfunction!(estimate_gromov_delta, m)?)?;
    m.add_function(wrap_pyfunction!(distance_stats, m)?)?;
    Ok(())
}
