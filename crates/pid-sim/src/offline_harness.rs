use anyhow::{bail, Context, Result};
use pid_core::{
    bootstrap_rows_stats, co_information_pairwise, concat_horiz, discrete_mi,
    distance_concentration_stats, gromov_hyperbolicity, intrinsic_dimension_levina_bickel,
    permutation_rows_pvalue, pid2_isx, quantize_equal_width, BootstrapConfig,
    DistanceConcentrationConfig, HyperbolicityConfig, IntrinsicDimConfig, IsxConfig, KsgConfig,
    LogisticRegression, LogisticRegressionConfig, MatOwned, MatRef, Metric, NegativeHandling,
    Pid2Config, PlsProjector, RowResampleScheme, Standardizer,
};
use pid_runlog::{
    EmbeddingVariableContract, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;

const OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION: f64 = 20.0;
const OFFLINE_GEOMETRY_MIN_PAIRWISE_CV: f64 = 0.1;
const OFFLINE_GEOMETRY_MIN_DELTA_REL: f64 = 0.1;
const OFFLINE_GEOMETRY_INTRINSIC_K: usize = 10;
const OFFLINE_GEOMETRY_HYPERBOLICITY_SAMPLES: usize = 500;
const OFFLINE_HELDOUT_SPLIT_METADATA_KEY: &str = "split";
const OFFLINE_CENTROID_SUCCESS_SCORE: &str =
    "distance_to_failure_centroid_minus_distance_to_success_centroid";
/// Unique-joint-bin fraction above which discrete plug-in MI is treated as
/// saturated (grandplan §8.1.6): estimates pinned near entropy ceilings (~ln n)
/// reflect small-sample artifacts, not dependence.
const OFFLINE_DISCRETE_SATURATION_UNIQUE_FRACTION_MAX: f64 = 0.8;

/// PID estimator mode: continuous (KSG-based kNN), discrete (quantization + counting),
/// or discrete-pls (PLS projection + discrete PID).
///
/// Measure identity (grandplan §8.1.6): continuous mode estimates the
/// shared-exclusions `I^sx_∩`; the discrete modes estimate a Williams–Beer-style
/// `I_min` redundancy. Cross-mode comparisons are cross-measure comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PidMode {
    /// Continuous PID using KSG kNN mutual information and shared-exclusions redundancy.
    #[default]
    Continuous,
    /// Discrete PID using equal-width quantization and counting-based entropy
    /// (`I_min`-style redundancy, not discrete `i^sx_∩`).
    Discrete,
    /// PLS supervised projection toward `A` followed by discrete PID (escape hatch
    /// for high-dimensional embeddings; projection is fitted on the samples given
    /// to each screen, so the train-split screen fits on train samples only).
    DiscretePls,
}

/// Options for the offline VLDA harness.
#[derive(Debug, Clone)]
pub struct OfflineVldaHarnessOptions {
    /// PID estimator mode (continuous, discrete, or discrete-pls).
    pub pid_mode: PidMode,
    /// Number of quantization bins when `pid_mode == Discrete` or `DiscretePls`.
    pub discrete_bins: usize,
    /// Number of PLS latent components when `pid_mode == DiscretePls`.
    pub pls_components: usize,
}

impl Default for OfflineVldaHarnessOptions {
    fn default() -> Self {
        Self {
            pid_mode: PidMode::Continuous,
            discrete_bins: 10,
            pls_components: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaDataset {
    pub run_id: Option<String>,
    pub source: Option<String>,
    pub model: Option<String>,
    pub task: Option<String>,
    pub samples: Vec<OfflineVldaSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaSample {
    pub sample_id: String,
    pub episode_id: Option<String>,
    pub v: Vec<f64>,
    pub l: Vec<f64>,
    pub d: Vec<f64>,
    pub a: Vec<f64>,
    #[serde(default)]
    pub labels: BTreeMap<String, Value>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineVldaDims {
    pub samples: usize,
    pub v: usize,
    pub l: usize,
    pub d: usize,
    pub a: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaMetrics {
    pub mi_v_action: f64,
    pub mi_l_action: f64,
    pub mi_d_action: f64,
    pub mi_vl_action: f64,
    pub co_information_v_l_action: f64,
    pub redundancy_v_l_action: f64,
    pub unique_v_action: f64,
    pub unique_l_action: f64,
    pub synergy_v_l_action: f64,
    pub success_rate: Option<f64>,
    pub majority_success_accuracy: Option<f64>,
    pub loo_nn_v_success_accuracy: Option<f64>,
    pub loo_nn_l_success_accuracy: Option<f64>,
    pub loo_nn_d_success_accuracy: Option<f64>,
    pub loo_nn_a_success_accuracy: Option<f64>,
    pub loo_nn_vlda_success_accuracy: Option<f64>,
    pub episode_loo_majority_success_accuracy: Option<f64>,
    pub episode_loo_nn_v_success_accuracy: Option<f64>,
    pub episode_loo_nn_l_success_accuracy: Option<f64>,
    pub episode_loo_nn_d_success_accuracy: Option<f64>,
    pub episode_loo_nn_a_success_accuracy: Option<f64>,
    pub episode_loo_nn_vlda_success_accuracy: Option<f64>,
    pub heldout_majority_success_accuracy: Option<f64>,
    pub heldout_majority_success_balanced_accuracy: Option<f64>,
    pub heldout_nn_v_success_accuracy: Option<f64>,
    pub heldout_nn_l_success_accuracy: Option<f64>,
    pub heldout_nn_d_success_accuracy: Option<f64>,
    pub heldout_nn_a_success_accuracy: Option<f64>,
    pub heldout_nn_vlda_success_accuracy: Option<f64>,
    pub heldout_nn_v_success_balanced_accuracy: Option<f64>,
    pub heldout_nn_l_success_balanced_accuracy: Option<f64>,
    pub heldout_nn_d_success_balanced_accuracy: Option<f64>,
    pub heldout_nn_a_success_balanced_accuracy: Option<f64>,
    pub heldout_nn_vlda_success_balanced_accuracy: Option<f64>,
    pub heldout_centroid_v_success_accuracy: Option<f64>,
    pub heldout_centroid_l_success_accuracy: Option<f64>,
    pub heldout_centroid_d_success_accuracy: Option<f64>,
    pub heldout_centroid_a_success_accuracy: Option<f64>,
    pub heldout_centroid_vlda_success_accuracy: Option<f64>,
    pub heldout_centroid_v_success_balanced_accuracy: Option<f64>,
    pub heldout_centroid_l_success_balanced_accuracy: Option<f64>,
    pub heldout_centroid_d_success_balanced_accuracy: Option<f64>,
    pub heldout_centroid_a_success_balanced_accuracy: Option<f64>,
    pub heldout_centroid_vlda_success_balanced_accuracy: Option<f64>,
    pub heldout_centroid_v_success_auroc: Option<f64>,
    pub heldout_centroid_l_success_auroc: Option<f64>,
    pub heldout_centroid_d_success_auroc: Option<f64>,
    pub heldout_centroid_a_success_auroc: Option<f64>,
    pub heldout_centroid_vlda_success_auroc: Option<f64>,
    /// SAFE-class internal-feature failure detector: L2-regularized logistic
    /// regression on the pooled, train-standardized `(V,L,D,A)` features, fit on
    /// the train split and evaluated on the held-out split (leakage-safe). This is
    /// the strong learned baseline H1 must beat (grandplan §3.4 / §14.1.1).
    pub heldout_logreg_vlda_success_accuracy: Option<f64>,
    pub heldout_logreg_vlda_success_balanced_accuracy: Option<f64>,
    pub heldout_logreg_vlda_success_auroc: Option<f64>,
    pub pid_pairs: BTreeMap<String, OfflineVldaPidPairMetrics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPidScreenMetrics {
    pub mi_v_action: f64,
    pub mi_l_action: f64,
    pub mi_d_action: f64,
    pub mi_vl_action: f64,
    pub co_information_v_l_action: f64,
    pub redundancy_v_l_action: f64,
    pub unique_v_action: f64,
    pub unique_l_action: f64,
    pub synergy_v_l_action: f64,
    pub pid_pairs: BTreeMap<String, OfflineVldaPidPairMetrics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPidPairMetrics {
    pub source_1: String,
    pub source_2: String,
    pub target: String,
    pub mi_source_1_action: f64,
    pub mi_source_2_action: f64,
    pub mi_joint_action: f64,
    pub co_information: f64,
    pub redundancy: f64,
    pub unique_source_1: f64,
    pub unique_source_2: f64,
    pub synergy: f64,
    /// Discrete-mode saturation diagnostics (grandplan §8.1.6); `None` in continuous mode.
    #[serde(default)]
    pub discrete_saturation: Option<OfflineVldaDiscreteSaturation>,
}

/// Saturation diagnostics for discrete (quantized) PID screens.
///
/// When almost every sample occupies its own joint bin, plug-in entropies hit
/// the `ln n` ceiling and MI estimates measure sample size, not dependence
/// (grandplan §8.1.6). Treat pairs with `saturation_warning == true` as invalid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaDiscreteSaturation {
    pub unique_fraction_source_1: f64,
    pub unique_fraction_source_2: f64,
    pub unique_fraction_target: f64,
    pub unique_fraction_joint: f64,
    pub saturation_warning: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaTrainSplitPidReport {
    pub split_metadata_key: String,
    pub split: String,
    pub train_values: Vec<String>,
    pub heldout_values: Vec<String>,
    pub status: String,
    pub samples: usize,
    pub heldout_samples_excluded: usize,
    pub train_sample_ids: Vec<String>,
    pub preprocessing: Option<OfflineVldaPreprocessingReport>,
    pub metrics: Option<OfflineVldaPidScreenMetrics>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPreprocessingReport {
    pub strategy: String,
    pub variables: BTreeMap<String, OfflineVldaPreprocessingVariable>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPreprocessingVariable {
    pub input_dim: usize,
    pub output_dim: usize,
    pub zero_variance_dims: usize,
    pub mean_sha256: String,
    pub inv_std_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaGeometryReport {
    pub space: String,
    pub metric: String,
    pub intrinsic_k: usize,
    pub hyperbolicity_samples: usize,
    pub gates: OfflineVldaGeometryGates,
    pub variables: BTreeMap<String, OfflineVldaGeometryVariable>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaGeometryGates {
    pub status: String,
    pub max_intrinsic_dimension: f64,
    pub min_pairwise_cv: f64,
    pub min_delta_rel: f64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaGeometryVariable {
    pub dims: Vec<usize>,
    pub intrinsic_dimension: Option<f64>,
    pub intrinsic_dimension_error: Option<String>,
    pub pairwise_count: Option<u64>,
    pub pairwise_min: Option<f64>,
    pub pairwise_max: Option<f64>,
    pub pairwise_mean: Option<f64>,
    pub pairwise_cv: Option<f64>,
    pub nn_mean: Option<f64>,
    pub nn_over_pairwise_mean: Option<f64>,
    pub distance_concentration_error: Option<String>,
    pub gromov_delta: Option<f64>,
    pub gromov_delta_rel: Option<f64>,
    pub gromov_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineVldaHeldoutSplitReport {
    pub metadata_key: String,
    pub train_values: Vec<String>,
    pub heldout_values: Vec<String>,
    pub train_samples: usize,
    pub heldout_samples: usize,
    pub value_counts: BTreeMap<String, usize>,
    pub train_sample_ids: Vec<String>,
    pub heldout_sample_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineVldaHeldoutClassCoverageReport {
    pub metadata_key: String,
    pub status: String,
    pub train_successes: usize,
    pub train_failures: usize,
    pub heldout_successes: usize,
    pub heldout_failures: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineVldaHeldoutEpisodeDisjointReport {
    pub split_metadata_key: String,
    pub episode_key: String,
    pub status: String,
    pub train_episodes: usize,
    pub heldout_episodes: usize,
    pub shared_episodes: usize,
    pub missing_episode_samples: usize,
    pub shared_episode_ids: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaHeldoutPredictionRecord {
    pub sample_id: String,
    pub episode_id: Option<String>,
    pub split_value: String,
    pub classifier: String,
    pub variable: Option<String>,
    pub true_success: bool,
    pub predicted_success: bool,
    pub correct: bool,
    pub score: Option<f64>,
    pub score_name: Option<String>,
    pub nearest_train_sample_id: Option<String>,
    pub squared_distance: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaHeldoutFailureDiagnostics {
    pub classifier: String,
    pub variable: Option<String>,
    pub samples: usize,
    pub true_failures: usize,
    pub true_successes: usize,
    pub predicted_failures: usize,
    pub predicted_successes: usize,
    pub failure_true_positives: usize,
    pub failure_false_positives: usize,
    pub failure_true_negatives: usize,
    pub failure_false_negatives: usize,
    pub failure_precision: Option<f64>,
    pub failure_recall: Option<f64>,
    pub failure_specificity: Option<f64>,
    pub failure_f1: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaReport {
    pub run_id: String,
    pub config_hash: String,
    pub config: Value,
    pub dims: OfflineVldaDims,
    pub label_counts: BTreeMap<String, usize>,
    pub preprocessing: OfflineVldaPreprocessingReport,
    pub geometry: OfflineVldaGeometryReport,
    pub train_split_pid: Option<OfflineVldaTrainSplitPidReport>,
    pub heldout_split: Option<OfflineVldaHeldoutSplitReport>,
    pub heldout_class_coverage: Option<OfflineVldaHeldoutClassCoverageReport>,
    pub heldout_episode_disjoint: Option<OfflineVldaHeldoutEpisodeDisjointReport>,
    pub heldout_predictions: Vec<OfflineVldaHeldoutPredictionRecord>,
    pub heldout_failure_diagnostics: Vec<OfflineVldaHeldoutFailureDiagnostics>,
    /// Per-axis provenance honesty: aggregates the provenance markers the capture
    /// adapter stamps on each sample — `l_source`/`d_source` (live `ncp-observer` tap)
    /// and `{v,l,d,a}_provenance` (offline `safe_adapter`) — so a PID atom computed
    /// from a *fabricated* `L` (`absent_zeroed`), a *recency-misaligned* `D`
    /// (`recency_fallback`), or a *hash-proxy* feature (`text_hash_proxy`) is surfaced
    /// as degraded rather than silently reported as trustworthy. Empty when no sample
    /// carries provenance markers (e.g. a pure synthetic dataset).
    pub axis_provenance: Vec<OfflineVldaAxisProvenance>,
    pub metrics: OfflineVldaMetrics,
}

/// Provenance summary for one `(V,L,D,A)` axis, aggregated across samples. `status`
/// is `"degraded"` when any sample carries a known-bad provenance value for the axis
/// (a fabricated/zeroed `L`, or a recency-misaligned/absent `D`) — in which case the
/// PID atoms that involve this axis must be treated as not trustworthy for the
/// affected samples (M5 honesty: never present a fabricated/misaligned axis's atoms
/// as clean).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineVldaAxisProvenance {
    pub marker: String,
    pub axis: String,
    pub sources: BTreeMap<String, usize>,
    pub degraded_samples: usize,
    pub total_samples: usize,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OfflineVldaRunlogOptions {
    pub require_geometry_pass: bool,
    pub require_success_labels: bool,
    pub require_heldout_split: bool,
    pub require_heldout_class_coverage: bool,
    pub require_heldout_episode_disjoint: bool,
    /// Fail the run if any *stamped* V/L/D/A provenance marker is degraded (a
    /// `text_hash_proxy` / `absent_zeroed` / `recency_fallback` … value), AND fail
    /// if NO marker was stamped at all — so the gate cannot pass vacuously on a
    /// dataset that carries no provenance (positive attestation). NB: this checks
    /// only the axes that actually carry a marker; it does **not** (yet) require all
    /// four axes to be independently attested. A capture that stamps a subset (e.g.
    /// `ncp-observer` stamps `l_source`/`d_source` but nothing for V or A) passes as
    /// long as the stamped axes are honest; the `safe_adapter` path stamps all four
    /// (`{v,l,d,a}_provenance`). Requiring per-axis coverage of all four is tracked
    /// follow-up. See [`offline_vlda_axis_provenance_failure_messages`].
    pub require_axis_provenance_honest: bool,
}

pub fn read_offline_vlda_dataset(path: impl AsRef<Path>) -> Result<OfflineVldaDataset> {
    let file = std::fs::File::open(path.as_ref())
        .with_context(|| format!("failed to open {}", path.as_ref().display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("failed to parse {}", path.as_ref().display()))
}

pub fn run_offline_vlda_harness(
    dataset: OfflineVldaDataset,
    input_uri: Option<String>,
    input_sha256: Option<String>,
) -> Result<OfflineVldaReport> {
    run_offline_vlda_harness_with_options(
        dataset,
        input_uri,
        input_sha256,
        &OfflineVldaHarnessOptions::default(),
    )
}

/// Run the offline VLDA harness with explicit options (PID mode, bin count, etc.).
pub fn run_offline_vlda_harness_with_options(
    dataset: OfflineVldaDataset,
    input_uri: Option<String>,
    input_sha256: Option<String>,
    options: &OfflineVldaHarnessOptions,
) -> Result<OfflineVldaReport> {
    let dims = validate_dataset(&dataset)?;
    let label_counts = label_counts(&dataset.samples);
    let analysis = compute_analysis(
        &dataset.samples,
        &dims,
        options.pid_mode,
        options.discrete_bins,
        options.pls_components,
    )?;
    let run_id = dataset
        .run_id
        .clone()
        .unwrap_or_else(|| "offline-vlda-run".to_string());
    let config = json!({
        "harness": "offline_vlda",
        "source": dataset.source,
        "model": dataset.model,
        "task": dataset.task,
        "input_uri": input_uri,
        "input_sha256": input_sha256,
        "dims": dims,
        "samples": dataset.samples.len(),
        "metric_pipeline": {
            "mi": match options.pid_mode {
                PidMode::Continuous => "ksg",
                PidMode::Discrete | PidMode::DiscretePls => "discrete",
            },
            "pid": match options.pid_mode {
                PidMode::Continuous => "isx_ehrlich_ksg",
                PidMode::Discrete => "discrete_imin",
                PidMode::DiscretePls => "pls_discrete_imin",
            },
            "pid_mode": options.pid_mode,
            "discrete_bins": options.discrete_bins,
            "pls_components": options.pls_components,
            "pid_pairs": [["V", "L"], ["V", "D"], ["L", "D"]],
            "pid_sample_scopes": if analysis.train_split_pid.as_ref().and_then(|report| report.metrics.as_ref()).is_some() {
                vec!["all_samples", "metadata_split_train"]
            } else {
                vec!["all_samples"]
            },
            "target": "A",
            "shared_source_metrics": ["mi_v_action", "mi_l_action", "mi_d_action"],
            "preprocessing": {
                "pid_geometry_space": analysis.preprocessing.strategy.clone(),
                "standardizer": "per_variable_center_scale_population_std",
                "full_sample_pid_fit_scope": "all_samples",
                "train_split_pid_fit_scope": analysis.train_split_pid.as_ref().and_then(|report| report.metrics.as_ref()).map(|_| "metadata_split_train")
            },
            "geometry": {
                "metric": analysis.geometry.metric.clone(),
                "intrinsic_k": analysis.geometry.intrinsic_k,
                "hyperbolicity_samples": analysis.geometry.hyperbolicity_samples,
                "max_intrinsic_dimension": OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION,
                "min_pairwise_cv": OFFLINE_GEOMETRY_MIN_PAIRWISE_CV,
                "min_delta_rel": OFFLINE_GEOMETRY_MIN_DELTA_REL
            },
            "baselines": [
                "majority_success_accuracy",
                "loo_nn_v_success_accuracy",
                "loo_nn_l_success_accuracy",
                "loo_nn_d_success_accuracy",
                "loo_nn_a_success_accuracy",
                "loo_nn_vlda_success_accuracy",
                "episode_loo_majority_success_accuracy",
                "episode_loo_nn_v_success_accuracy",
                "episode_loo_nn_l_success_accuracy",
                "episode_loo_nn_d_success_accuracy",
                "episode_loo_nn_a_success_accuracy",
                "episode_loo_nn_vlda_success_accuracy",
                "heldout_majority_success_accuracy",
                "heldout_majority_success_balanced_accuracy",
                "heldout_nn_v_success_accuracy",
                "heldout_nn_l_success_accuracy",
                "heldout_nn_d_success_accuracy",
                "heldout_nn_a_success_accuracy",
                "heldout_nn_vlda_success_accuracy",
                "heldout_nn_v_success_balanced_accuracy",
                "heldout_nn_l_success_balanced_accuracy",
                "heldout_nn_d_success_balanced_accuracy",
                "heldout_nn_a_success_balanced_accuracy",
                "heldout_nn_vlda_success_balanced_accuracy",
                "heldout_centroid_v_success_accuracy",
                "heldout_centroid_l_success_accuracy",
                "heldout_centroid_d_success_accuracy",
                "heldout_centroid_a_success_accuracy",
                "heldout_centroid_vlda_success_accuracy",
                "heldout_centroid_v_success_balanced_accuracy",
                "heldout_centroid_l_success_balanced_accuracy",
                "heldout_centroid_d_success_balanced_accuracy",
                "heldout_centroid_a_success_balanced_accuracy",
                "heldout_centroid_vlda_success_balanced_accuracy",
                "heldout_centroid_v_success_auroc",
                "heldout_centroid_l_success_auroc",
                "heldout_centroid_d_success_auroc",
                "heldout_centroid_a_success_auroc",
                "heldout_centroid_vlda_success_auroc",
                "heldout_logreg_vlda_success_accuracy",
                "heldout_logreg_vlda_success_balanced_accuracy",
                "heldout_logreg_vlda_success_auroc",
                "heldout_failure_true_positive_count",
                "heldout_failure_false_positive_count",
                "heldout_failure_true_negative_count",
                "heldout_failure_false_negative_count",
                "heldout_failure_precision",
                "heldout_failure_recall",
                "heldout_failure_specificity",
                "heldout_failure_f1",
                "heldout_class_coverage_pass",
                "heldout_class_coverage_train_success_count",
                "heldout_class_coverage_train_failure_count",
                "heldout_class_coverage_heldout_success_count",
                "heldout_class_coverage_heldout_failure_count",
                "heldout_episode_disjoint_pass",
                "heldout_episode_disjoint_train_episode_count",
                "heldout_episode_disjoint_heldout_episode_count",
                "heldout_episode_disjoint_shared_episode_count",
                "heldout_episode_disjoint_missing_episode_sample_count",
                "heldout_prediction_correct",
                "heldout_prediction_score",
                "heldout_prediction_squared_distance"
            ],
            "heldout_split": analysis.heldout_split.clone(),
            "train_split_pid": analysis.train_split_pid.as_ref().map(|report| json!({
                "status": report.status,
                "split_metadata_key": report.split_metadata_key,
                "split": report.split,
                "samples": report.samples,
                "heldout_samples_excluded": report.heldout_samples_excluded,
                "preprocessing_available": report.preprocessing.is_some(),
                "metrics_available": report.metrics.is_some()
            })),
            "heldout_class_coverage": analysis.heldout_class_coverage.clone(),
            "heldout_episode_disjoint": analysis.heldout_episode_disjoint.clone(),
            "prediction_records": [
                "heldout_train_split_majority",
                "heldout_train_split_1nn",
                "heldout_train_split_nearest_centroid"
            ],
            "negative_handling": "allow"
        }
    });
    let config_hash = pid_runlog::canonical_json_hash(&config)?;
    Ok(OfflineVldaReport {
        run_id,
        config_hash,
        config,
        dims,
        label_counts,
        preprocessing: analysis.preprocessing,
        geometry: analysis.geometry,
        train_split_pid: analysis.train_split_pid,
        heldout_split: analysis.heldout_split,
        heldout_class_coverage: analysis.heldout_class_coverage,
        heldout_episode_disjoint: analysis.heldout_episode_disjoint,
        heldout_predictions: analysis.heldout_predictions,
        heldout_failure_diagnostics: analysis.heldout_failure_diagnostics,
        axis_provenance: axis_provenance(&dataset.samples),
        metrics: analysis.metrics,
    })
}

fn train_split_pid_report(
    samples: &[OfflineVldaSample],
    dims: &OfflineVldaDims,
    split: &OfflineVldaHeldoutSplitPlan,
    pid_mode: PidMode,
    discrete_bins: usize,
    pls_components: usize,
) -> OfflineVldaTrainSplitPidReport {
    let train_samples = samples
        .iter()
        .zip(&split.roles)
        .filter_map(|(sample, role)| {
            (*role == OfflineVldaSplitRole::Train).then_some(sample.clone())
        })
        .collect::<Vec<_>>();
    let train_dims = OfflineVldaDims {
        samples: train_samples.len(),
        v: dims.v,
        l: dims.l,
        d: dims.d,
        a: dims.a,
    };
    let result = (|| -> Result<(OfflineVldaPreprocessingReport, OfflineVldaPidScreenMetrics)> {
        let prepared = prepare_standardized_embeddings(&train_samples, &train_dims)?;
        let metrics =
            compute_pid_screen_metrics(&prepared, pid_mode, discrete_bins, pls_components)?;
        Ok((prepared.preprocessing, metrics))
    })();
    let (status, preprocessing, metrics, error) = match result {
        Ok((preprocessing, metrics)) => (
            "available".to_string(),
            Some(preprocessing),
            Some(metrics),
            None,
        ),
        Err(err) => ("error".to_string(), None, None, Some(format!("{err:#}"))),
    };
    OfflineVldaTrainSplitPidReport {
        split_metadata_key: split.report.metadata_key.clone(),
        split: "metadata_split_train".to_string(),
        train_values: split.report.train_values.clone(),
        heldout_values: split.report.heldout_values.clone(),
        status,
        samples: train_samples.len(),
        heldout_samples_excluded: split.report.heldout_samples,
        train_sample_ids: split.report.train_sample_ids.clone(),
        preprocessing,
        metrics,
        error,
    }
}

fn compute_pid_screen_metrics(
    prepared: &PreparedVldaMatrices,
    pid_mode: PidMode,
    discrete_bins: usize,
    pls_components: usize,
) -> Result<OfflineVldaPidScreenMetrics> {
    let v = prepared.v.as_ref();
    let l = prepared.l.as_ref();
    let d = prepared.d.as_ref();
    let a = prepared.a.as_ref();

    // DiscretePls: project each source toward A with PLS fitted on the samples
    // given to this screen (train-only in the train-split path; in-sample for the
    // all-samples screen, which the metric_pipeline provenance records). The
    // target A stays unprojected.
    let pls_projected = match pid_mode {
        PidMode::DiscretePls => {
            let v_proj = PlsProjector::fit(v, a, pls_components)?.transform(v)?;
            let l_proj = PlsProjector::fit(l, a, pls_components)?.transform(l)?;
            let d_proj = PlsProjector::fit(d, a, pls_components)?.transform(d)?;
            Some((v_proj, l_proj, d_proj))
        }
        PidMode::Continuous | PidMode::Discrete => None,
    };
    let (v_eff, l_eff, d_eff) = match &pls_projected {
        Some((v_proj, l_proj, d_proj)) => (v_proj.as_ref(), l_proj.as_ref(), d_proj.as_ref()),
        None => (v, l, d),
    };

    // Compute per-source MI with A.
    let (mi_v_action, mi_l_action, mi_d_action) = match pid_mode {
        PidMode::Continuous => {
            let ksg = KsgConfig {
                negative_handling: NegativeHandling::Allow,
                ..Default::default()
            };
            (
                pid_core::ksg_mi(v_eff, a, &ksg)?,
                pid_core::ksg_mi(l_eff, a, &ksg)?,
                pid_core::ksg_mi(d_eff, a, &ksg)?,
            )
        }
        PidMode::Discrete | PidMode::DiscretePls => {
            let v_bins = quantize_equal_width(v_eff, discrete_bins)?;
            let l_bins = quantize_equal_width(l_eff, discrete_bins)?;
            let d_bins = quantize_equal_width(d_eff, discrete_bins)?;
            let a_bins = quantize_equal_width(a, discrete_bins)?;
            (
                discrete_mi(&v_bins, &a_bins, discrete_bins)?,
                discrete_mi(&l_bins, &a_bins, discrete_bins)?,
                discrete_mi(&d_bins, &a_bins, discrete_bins)?,
            )
        }
    };

    let v_source = OfflineVldaSourceMatrix {
        name: "V",
        matrix: v_eff,
        mi_action: mi_v_action,
    };
    let l_source = OfflineVldaSourceMatrix {
        name: "L",
        matrix: l_eff,
        mi_action: mi_l_action,
    };
    let d_source = OfflineVldaSourceMatrix {
        name: "D",
        matrix: d_eff,
        mi_action: mi_d_action,
    };
    let action_target = OfflineVldaTargetMatrix {
        name: "A",
        matrix: a,
    };

    let (vl_pair, vd_pair, ld_pair) = match pid_mode {
        PidMode::Continuous => {
            let ksg = KsgConfig {
                negative_handling: NegativeHandling::Allow,
                ..Default::default()
            };
            let pid_cfg = Pid2Config {
                ksg: ksg.clone(),
                isx: IsxConfig {
                    k: ksg.k,
                    metric: ksg.metric,
                    tie_epsilon: ksg.tie_epsilon,
                    ..Default::default()
                },
            };
            (
                compute_pid_pair_metrics(v_source, l_source, action_target, &ksg, &pid_cfg)?,
                compute_pid_pair_metrics(v_source, d_source, action_target, &ksg, &pid_cfg)?,
                compute_pid_pair_metrics(l_source, d_source, action_target, &ksg, &pid_cfg)?,
            )
        }
        PidMode::Discrete | PidMode::DiscretePls => (
            compute_pid_pair_metrics_discrete(v_source, l_source, action_target, discrete_bins)?,
            compute_pid_pair_metrics_discrete(v_source, d_source, action_target, discrete_bins)?,
            compute_pid_pair_metrics_discrete(l_source, d_source, action_target, discrete_bins)?,
        ),
    };

    let pid_pairs = [
        ("VL".to_string(), vl_pair.clone()),
        ("VD".to_string(), vd_pair),
        ("LD".to_string(), ld_pair),
    ]
    .into_iter()
    .collect();
    Ok(OfflineVldaPidScreenMetrics {
        mi_v_action,
        mi_l_action,
        mi_d_action,
        mi_vl_action: vl_pair.mi_joint_action,
        co_information_v_l_action: vl_pair.co_information,
        redundancy_v_l_action: vl_pair.redundancy,
        unique_v_action: vl_pair.unique_source_1,
        unique_l_action: vl_pair.unique_source_2,
        synergy_v_l_action: vl_pair.synergy,
        pid_pairs,
    })
}

// ── Opt-in PID-screen uncertainty (subsample bootstrap + permutation nulls) ──

/// Configuration for [`compute_offline_pid_uncertainty`].
#[derive(Debug, Clone, PartialEq)]
pub struct OfflineVldaUncertaintyConfig {
    /// Number of subsample-bootstrap resamples (0 disables CIs).
    pub n_boot: usize,
    /// Number of permutations for single-source unique-atom nulls (0 disables them).
    pub n_perm: usize,
    /// Moving-block length for the resamplers (1 = i.i.d.).
    pub block_size: usize,
    /// Significance level for the percentile CIs.
    pub alpha: f64,
    /// Base seed for the resamplers.
    pub seed: u64,
}

impl Default for OfflineVldaUncertaintyConfig {
    fn default() -> Self {
        Self {
            n_boot: 0,
            n_perm: 0,
            block_size: 1,
            alpha: 0.05,
            seed: 0xC0FFEE,
        }
    }
}

impl OfflineVldaUncertaintyConfig {
    pub fn enabled(&self) -> bool {
        self.n_boot > 0 || self.n_perm > 0
    }
}

/// Percentile CI for one PID atom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaAtomCi {
    pub point: f64,
    pub ci_low: f64,
    pub ci_high: f64,
    pub n_valid: usize,
}

/// Bootstrap CIs + permutation p-values for one two-source pair → A.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPairUncertainty {
    pub pair: String,
    pub redundancy: Option<OfflineVldaAtomCi>,
    pub unique_s1: Option<OfflineVldaAtomCi>,
    pub unique_s2: Option<OfflineVldaAtomCi>,
    pub synergy: Option<OfflineVldaAtomCi>,
    /// One-sided permutation p-value for `unique_s1` (shuffling source 1).
    pub unique_s1_perm_p: Option<f64>,
    /// One-sided permutation p-value for `unique_s2` (shuffling source 2).
    pub unique_s2_perm_p: Option<f64>,
    pub perm_n_valid_s1: usize,
    pub perm_n_valid_s2: usize,
}

/// Result of [`compute_offline_pid_uncertainty`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPidUncertainty {
    /// `"continuous"` when CIs were computed, or `"skipped:<reason>"`.
    pub mode: String,
    pub n_boot: usize,
    pub n_perm: usize,
    pub block_size: usize,
    pub subsample_len: usize,
    pub alpha: f64,
    pub resample_scheme: String,
    pub pairs: Vec<OfflineVldaPairUncertainty>,
}

/// Compute subsample-bootstrap CIs and single-source permutation p-values for the
/// three two-source `(V,L)→A` / `(V,D)→A` / `(L,D)→A` PID screens.
///
/// This is the analysis-side complement to the Exp0 uncertainty gate: it quantifies
/// uncertainty on the **continuous `I^sx_∩`** atoms (the primary atom-level measure;
/// discrete `I_min` modes are a different measure — Warning 6 — and are reported as
/// `skipped`). Resampling is Politis–Romano subsampling without replacement, which
/// is safe for the KSG kNN estimator (a with-replacement bootstrap is not — see
/// `pid_core::RowResampleScheme`); CIs are correspondingly conservative.
///
/// It is intentionally self-contained and written to a dedicated file by the
/// binary, so it never perturbs the canonical run-log / summary metric counts.
pub fn compute_offline_pid_uncertainty(
    dataset: &OfflineVldaDataset,
    pid_mode: PidMode,
    config: &OfflineVldaUncertaintyConfig,
) -> Result<OfflineVldaPidUncertainty> {
    if pid_mode != PidMode::Continuous {
        return Ok(OfflineVldaPidUncertainty {
            mode: format!("skipped:non_continuous_mode_is_a_different_measure ({pid_mode:?})"),
            n_boot: config.n_boot,
            n_perm: config.n_perm,
            block_size: config.block_size,
            subsample_len: 0,
            alpha: config.alpha,
            resample_scheme: "politis_romano_subsample".to_string(),
            pairs: Vec::new(),
        });
    }
    if config.block_size == 0 {
        bail!("uncertainty block_size must be >= 1");
    }

    let dims = validate_dataset(dataset)?;
    let prepared = prepare_standardized_embeddings(&dataset.samples, &dims)?;
    let v = prepared.v.as_ref();
    let l = prepared.l.as_ref();
    let d = prepared.d.as_ref();
    let a = prepared.a.as_ref();
    let n = v.nrows();

    // Subsample length: half the rows in whole blocks (the conservative
    // Politis–Romano regime); clamp so there is at least one block.
    let subsample_len = (((n / 2) / config.block_size).max(1)) * config.block_size;

    let ksg = KsgConfig {
        negative_handling: NegativeHandling::Allow,
        ..Default::default()
    };
    let pid_cfg = Pid2Config {
        ksg: ksg.clone(),
        isx: IsxConfig {
            k: ksg.k,
            metric: ksg.metric,
            tie_epsilon: ksg.tie_epsilon,
            ..Default::default()
        },
    };

    let pairs_spec: [(&str, MatRef<'_>, MatRef<'_>); 3] =
        [("VL", v, l), ("VD", v, d), ("LD", l, d)];
    let mut pairs = Vec::with_capacity(3);
    for (name, s1, s2) in pairs_spec {
        let mats = [s1, s2, a];

        let (redundancy, unique_s1, unique_s2, synergy) = if config.n_boot > 0 {
            let boot_cfg = BootstrapConfig {
                n_boot: config.n_boot,
                block_size: config.block_size,
                seed: config.seed,
                alpha: config.alpha,
            };
            let scheme = RowResampleScheme::Subsample { subsample_len };
            let res = bootstrap_rows_stats(&mats, &boot_cfg, scheme, |m| {
                let r = pid2_isx(m[0], m[1], m[2], &pid_cfg)?;
                Ok(vec![r.redundancy, r.unique_s1, r.unique_s2, r.synergy])
            })
            .map_err(|e| anyhow::anyhow!("pid2 bootstrap failed for {name}: {e}"))?;
            let to_ci = |idx: usize| {
                let s = &res.stats[idx];
                Some(OfflineVldaAtomCi {
                    point: s.point_estimate,
                    ci_low: s.ci_low,
                    ci_high: s.ci_high,
                    n_valid: s.n_valid,
                })
            };
            (to_ci(0), to_ci(1), to_ci(2), to_ci(3))
        } else {
            (None, None, None, None)
        };

        let (unique_s1_perm_p, perm_n_valid_s1, unique_s2_perm_p, perm_n_valid_s2) =
            if config.n_perm > 0 {
                // Shuffle source 1 → null for its unique atom; likewise source 2.
                let p1 = permutation_rows_pvalue(&mats, 0, config.n_perm, config.seed, |m| {
                    Ok(pid2_isx(m[0], m[1], m[2], &pid_cfg)?.unique_s1)
                })
                .map_err(|e| anyhow::anyhow!("pid2 permutation (s1) failed for {name}: {e}"))?;
                let p2 = permutation_rows_pvalue(
                    &mats,
                    1,
                    config.n_perm,
                    config.seed.wrapping_add(1),
                    |m| Ok(pid2_isx(m[0], m[1], m[2], &pid_cfg)?.unique_s2),
                )
                .map_err(|e| anyhow::anyhow!("pid2 permutation (s2) failed for {name}: {e}"))?;
                (
                    p1.p_value.is_finite().then_some(p1.p_value),
                    p1.n_valid,
                    p2.p_value.is_finite().then_some(p2.p_value),
                    p2.n_valid,
                )
            } else {
                (None, 0, None, 0)
            };

        pairs.push(OfflineVldaPairUncertainty {
            pair: name.to_string(),
            redundancy,
            unique_s1,
            unique_s2,
            synergy,
            unique_s1_perm_p,
            unique_s2_perm_p,
            perm_n_valid_s1,
            perm_n_valid_s2,
        });
    }

    Ok(OfflineVldaPidUncertainty {
        mode: "continuous".to_string(),
        n_boot: config.n_boot,
        n_perm: config.n_perm,
        block_size: config.block_size,
        subsample_len,
        alpha: config.alpha,
        resample_scheme: "politis_romano_subsample".to_string(),
        pairs,
    })
}

/// Write a [`OfflineVldaPidUncertainty`] to a JSON file.
pub fn write_offline_pid_uncertainty(
    path: impl AsRef<Path>,
    uncertainty: &OfflineVldaPidUncertainty,
) -> Result<()> {
    ensure_parent(path.as_ref())?;
    pid_runlog::write_json_file(path, uncertainty)
}

pub fn write_offline_vlda_summary(
    path: impl AsRef<Path>,
    report: &OfflineVldaReport,
) -> Result<()> {
    ensure_parent(path.as_ref())?;
    pid_runlog::write_json_file(path, report)
}

pub fn write_offline_vlda_runlog(
    path: impl AsRef<Path>,
    summary_path: Option<&Path>,
    input_path: Option<&Path>,
    dataset: &OfflineVldaDataset,
    report: &OfflineVldaReport,
) -> Result<()> {
    write_offline_vlda_runlog_with_options(
        path,
        summary_path,
        input_path,
        dataset,
        report,
        OfflineVldaRunlogOptions::default(),
    )
}

pub fn write_offline_vlda_runlog_with_options(
    path: impl AsRef<Path>,
    summary_path: Option<&Path>,
    input_path: Option<&Path>,
    dataset: &OfflineVldaDataset,
    report: &OfflineVldaReport,
    options: OfflineVldaRunlogOptions,
) -> Result<()> {
    ensure_parent(path.as_ref())?;
    let mut writer = RunLogWriter::create(path.as_ref())?;
    let summary_sha256 = summary_path.and_then(|path| pid_runlog::sha256_file(path).ok());
    let input_uri = input_path
        .map(|path| path.display().to_string())
        .or_else(|| {
            report
                .config
                .get("input_uri")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    let input_sha256 = input_path
        .and_then(|path| pid_runlog::sha256_file(path).ok())
        .or_else(|| {
            report
                .config
                .get("input_sha256")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: report.run_id.clone(),
        timestamp_ns: 0,
        config_hash: report.config_hash.clone(),
        metadata: [
            ("source".to_string(), "pid-offline-harness".to_string()),
            (
                "strict_geometry_gate".to_string(),
                options.require_geometry_pass.to_string(),
            ),
            (
                "strict_success_labels".to_string(),
                options.require_success_labels.to_string(),
            ),
            (
                "strict_heldout_split".to_string(),
                options.require_heldout_split.to_string(),
            ),
            (
                "strict_heldout_class_coverage".to_string(),
                options.require_heldout_class_coverage.to_string(),
            ),
            (
                "strict_heldout_episode_disjoint".to_string(),
                options.require_heldout_episode_disjoint.to_string(),
            ),
            (
                "strict_axis_provenance_honest".to_string(),
                options.require_axis_provenance_honest.to_string(),
            ),
            (
                "geometry_gate_status".to_string(),
                report.geometry.gates.status.clone(),
            ),
            (
                "success_label_status".to_string(),
                offline_vlda_success_label_status(report).to_string(),
            ),
            (
                "heldout_split_status".to_string(),
                offline_vlda_heldout_split_status(report).to_string(),
            ),
            (
                "train_split_pid_status".to_string(),
                offline_vlda_train_split_pid_status(report).to_string(),
            ),
            (
                "heldout_class_coverage_status".to_string(),
                offline_vlda_heldout_class_coverage_status(report).to_string(),
            ),
            (
                "heldout_episode_disjoint_status".to_string(),
                offline_vlda_heldout_episode_disjoint_status(report).to_string(),
            ),
            (
                "task".to_string(),
                dataset
                    .task
                    .clone()
                    .unwrap_or_else(|| "offline_vlda".to_string()),
            ),
        ]
        .into_iter()
        .collect(),
    })?;
    writer.append(&RunLogEvent::ConfigLogged {
        timestamp_ns: 0,
        config_hash: report.config_hash.clone(),
        config: report.config.clone(),
    })?;
    for (idx, sample) in dataset.samples.iter().enumerate() {
        let step = idx as u64;
        let timestamp_ns = step * 1_000_000;
        let mut metadata = sample.metadata.clone();
        metadata.insert("sample_id".to_string(), sample.sample_id.clone());
        if let Some(episode_id) = &sample.episode_id {
            metadata.insert("episode_id".to_string(), episode_id.clone());
        }
        writer.append(&RunLogEvent::FrameObserved {
            step,
            timestamp_ns,
            observation_hash: Some(pid_runlog::canonical_json_hash(sample)?),
            metadata,
        })?;
        for (label, value) in &sample.labels {
            writer.append(&RunLogEvent::LabelObserved {
                step,
                timestamp_ns,
                name: format!("offline_vlda.{label}"),
                value: value.clone(),
                metadata: [("sample_id".to_string(), sample.sample_id.clone())]
                    .into_iter()
                    .collect(),
            })?;
        }
    }

    let embedding_timestamp_base = dataset.samples.len() as u64 * 1_000_000 + 1_000_000;
    writer.append(&RunLogEvent::EmbeddingContract {
        timestamp_ns: embedding_timestamp_base,
        name: "offline_vlda.vlda_contract".to_string(),
        variables: [
            ("V", "offline_vlda.V", report.dims.v),
            ("L", "offline_vlda.L", report.dims.l),
            ("D", "offline_vlda.D", report.dims.d),
            ("A", "offline_vlda.A", report.dims.a),
        ]
        .into_iter()
        .map(|(variable, source, dim)| EmbeddingVariableContract {
            variable: variable.to_string(),
            source: source.to_string(),
            dims: vec![report.dims.samples, dim],
            artifact_uri: input_uri.clone(),
            sha256: input_sha256.clone(),
        })
        .collect(),
        metadata: [
            ("source".to_string(), "pid-offline-harness".to_string()),
            ("decomposition".to_string(), "(V,L,D,A)".to_string()),
            (
                "pid_geometry_space".to_string(),
                report.preprocessing.strategy.clone(),
            ),
            (
                "geometry_metric".to_string(),
                report.geometry.metric.clone(),
            ),
        ]
        .into_iter()
        .collect(),
    })?;
    for (idx, (name, dim)) in [
        ("offline_vlda.V", report.dims.v),
        ("offline_vlda.L", report.dims.l),
        ("offline_vlda.D", report.dims.d),
        ("offline_vlda.A", report.dims.a),
    ]
    .into_iter()
    .enumerate()
    {
        writer.append(&RunLogEvent::EmbeddingCaptured {
            step: report.dims.samples as u64,
            timestamp_ns: embedding_timestamp_base + idx as u64 + 1,
            name: name.to_string(),
            dims: vec![report.dims.samples, dim],
            artifact_uri: input_uri.clone(),
            sha256: input_sha256.clone(),
            metadata: [
                ("source".to_string(), "offline_vlda_dataset".to_string()),
                ("analysis_space".to_string(), "raw_capture".to_string()),
                (
                    "pid_geometry_space".to_string(),
                    report.preprocessing.strategy.clone(),
                ),
            ]
            .into_iter()
            .collect(),
        })?;
    }

    let metric_timestamp_base = embedding_timestamp_base + 10_000;
    // Metric events are stamped metric_timestamp_base + i for i in 0..count,
    // and count scales with the dataset (≈21 events per labeled held-out
    // sample). Everything appended after them must continue from the RETURNED
    // count — a fixed offset would be overtaken on realistic capture sizes and
    // the log would fail pid-runlog's nondecreasing-timestamp validation.
    let metric_events = write_metric_events(&mut writer, report, metric_timestamp_base)?;
    let mut next_timestamp_ns = metric_timestamp_base + metric_events;
    if let Some(input_path) = input_path {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns: next_timestamp_ns,
            name: "offline_vlda_input_json".to_string(),
            kind: "dataset_json".to_string(),
            uri: input_path.display().to_string(),
            sha256: input_sha256,
            metadata: BTreeMap::new(),
        })?;
        next_timestamp_ns += 1;
    }
    if let Some(summary_path) = summary_path {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns: next_timestamp_ns,
            name: "offline_vlda_summary_json".to_string(),
            kind: "summary_json".to_string(),
            uri: summary_path.display().to_string(),
            sha256: summary_sha256,
            metadata: BTreeMap::new(),
        })?;
        next_timestamp_ns += 1;
    }
    let failures = offline_vlda_required_failures(dataset, report, options);
    let run_failed = !failures.is_empty();
    let run_message = if run_failed {
        failures.join("; ")
    } else {
        format!(
            "offline VLDA harness complete: {} samples",
            report.dims.samples
        )
    };
    for failure in failures.iter() {
        writer.append(&RunLogEvent::ErrorLogged {
            step: Some(report.dims.samples as u64),
            timestamp_ns: next_timestamp_ns,
            message: failure.clone(),
            recoverable: false,
        })?;
        next_timestamp_ns += 1;
    }
    writer.append(&RunLogEvent::RunEnded {
        run_id: report.run_id.clone(),
        timestamp_ns: next_timestamp_ns,
        status: if run_failed {
            RunStatus::Failed
        } else {
            RunStatus::Succeeded
        },
        message: Some(run_message),
    })?;
    writer.flush()?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct OfflineVldaPidMetricEventScope<'a> {
    prefix: &'static str,
    train_pid: Option<&'a OfflineVldaTrainSplitPidReport>,
}

pub fn offline_vlda_geometry_gate_failure_message(report: &OfflineVldaReport) -> String {
    format!(
        "offline VLDA geometry gate {}: {} warning(s)",
        report.geometry.gates.status,
        report.geometry.gates.warnings.len()
    )
}

pub fn offline_vlda_success_label_failure_message(
    dataset: &OfflineVldaDataset,
    report: &OfflineVldaReport,
) -> String {
    let boolean_success_labels = dataset
        .samples
        .iter()
        .filter(|sample| {
            sample
                .labels
                .get("success")
                .and_then(Value::as_bool)
                .is_some()
        })
        .count();
    format!(
        "offline VLDA success labels unavailable: {boolean_success_labels}/{} samples have boolean success labels",
        report.dims.samples
    )
}

pub fn offline_vlda_success_label_status(report: &OfflineVldaReport) -> &'static str {
    if report.metrics.success_rate.is_some() {
        "available"
    } else {
        "missing"
    }
}

pub fn offline_vlda_heldout_split_failure_message(
    dataset: &OfflineVldaDataset,
    report: &OfflineVldaReport,
) -> String {
    let split = heldout_split_diagnostics(dataset);
    let boolean_success_labels = dataset
        .samples
        .iter()
        .filter(|sample| {
            sample
                .labels
                .get("success")
                .and_then(Value::as_bool)
                .is_some()
        })
        .count();
    format!(
        "offline VLDA held-out split unavailable: metadata.{} train={} heldout={} missing={} unrecognized={} boolean_success_labels={}/{}",
        OFFLINE_HELDOUT_SPLIT_METADATA_KEY,
        split.train_samples,
        split.heldout_samples,
        split.missing_samples,
        split.unrecognized_samples,
        boolean_success_labels,
        report.dims.samples
    )
}

pub fn offline_vlda_heldout_split_status(report: &OfflineVldaReport) -> &'static str {
    if report.metrics.heldout_majority_success_accuracy.is_some() {
        "available"
    } else if report.heldout_split.is_some() {
        "missing_success_labels"
    } else {
        "missing"
    }
}

pub fn offline_vlda_train_split_pid_status(report: &OfflineVldaReport) -> &'static str {
    match report.train_split_pid.as_ref() {
        Some(train_pid) if train_pid.metrics.is_some() => "available",
        Some(_) => "error",
        None => "missing",
    }
}

pub fn offline_vlda_heldout_class_coverage_failure_message(report: &OfflineVldaReport) -> String {
    match &report.heldout_class_coverage {
        Some(coverage) => format!(
            "offline VLDA held-out class coverage {}: train_successes={} train_failures={} heldout_successes={} heldout_failures={} warning(s)={}",
            coverage.status,
            coverage.train_successes,
            coverage.train_failures,
            coverage.heldout_successes,
            coverage.heldout_failures,
            coverage.warnings.len()
        ),
        None => "offline VLDA held-out class coverage unavailable".to_string(),
    }
}

pub fn offline_vlda_heldout_class_coverage_status(report: &OfflineVldaReport) -> &'static str {
    match report.heldout_class_coverage.as_ref() {
        Some(coverage) if coverage.status == "pass" => "pass",
        Some(_) => "warn",
        None => "missing",
    }
}

pub fn offline_vlda_heldout_episode_disjoint_failure_message(report: &OfflineVldaReport) -> String {
    match &report.heldout_episode_disjoint {
        Some(disjoint) => format!(
            "offline VLDA held-out episode disjointness {}: train_episodes={} heldout_episodes={} shared_episodes={} missing_episode_samples={} warning(s)={}",
            disjoint.status,
            disjoint.train_episodes,
            disjoint.heldout_episodes,
            disjoint.shared_episodes,
            disjoint.missing_episode_samples,
            disjoint.warnings.len()
        ),
        None => "offline VLDA held-out episode disjointness unavailable".to_string(),
    }
}

pub fn offline_vlda_heldout_episode_disjoint_status(report: &OfflineVldaReport) -> &'static str {
    match report.heldout_episode_disjoint.as_ref() {
        Some(disjoint) if disjoint.status == "pass" => "pass",
        Some(_) => "warn",
        None => "missing",
    }
}

/// Gate messages for `--require-axis-provenance-honest`. Returns a failure for every
/// V/L/D/A axis whose provenance is `degraded` (a PID atom computed from a
/// fabricated / recency-misaligned / hash-proxy axis is not trustworthy), AND — the
/// key hardening — a single failure when NO provenance markers were stamped at all:
/// honesty cannot be *attested* from a dataset that carries no provenance, so the
/// gate fails closed rather than passing vacuously (positive attestation). Returns an
/// empty vec only when at least one marker is present and none is degraded.
pub fn offline_vlda_axis_provenance_failure_messages(
    axis_provenance: &[OfflineVldaAxisProvenance],
) -> Vec<String> {
    if axis_provenance.is_empty() {
        return vec![
            "offline VLDA axis-provenance gate: no axis-provenance markers were stamped, so \
             V/L/D/A honesty cannot be attested (positive attestation required)"
                .to_string(),
        ];
    }
    axis_provenance
        .iter()
        .filter(|p| p.status == "degraded")
        .map(|p| {
            format!(
                "offline VLDA axis-provenance gate: axis {} ({}) is degraded — {} sample(s) carry \
                 a non-honest marker",
                p.axis, p.marker, p.degraded_samples
            )
        })
        .collect()
}

fn offline_vlda_required_failures(
    dataset: &OfflineVldaDataset,
    report: &OfflineVldaReport,
    options: OfflineVldaRunlogOptions,
) -> Vec<String> {
    let mut failures = Vec::new();
    if options.require_geometry_pass && report.geometry.gates.status != "pass" {
        failures.push(offline_vlda_geometry_gate_failure_message(report));
    }
    if options.require_success_labels && report.metrics.success_rate.is_none() {
        failures.push(offline_vlda_success_label_failure_message(dataset, report));
    }
    if options.require_heldout_split && report.metrics.heldout_majority_success_accuracy.is_none() {
        failures.push(offline_vlda_heldout_split_failure_message(dataset, report));
    }
    if options.require_heldout_class_coverage
        && offline_vlda_heldout_class_coverage_status(report) != "pass"
    {
        failures.push(offline_vlda_heldout_class_coverage_failure_message(report));
    }
    if options.require_heldout_episode_disjoint
        && offline_vlda_heldout_episode_disjoint_status(report) != "pass"
    {
        failures.push(offline_vlda_heldout_episode_disjoint_failure_message(
            report,
        ));
    }
    if options.require_axis_provenance_honest {
        failures.extend(offline_vlda_axis_provenance_failure_messages(
            &report.axis_provenance,
        ));
    }
    failures
}

fn validate_dataset(dataset: &OfflineVldaDataset) -> Result<OfflineVldaDims> {
    if dataset.samples.len() < 8 {
        bail!("offline VLDA dataset must contain at least 8 samples");
    }
    let first = dataset.samples.first().expect("checked nonempty");
    let dims = OfflineVldaDims {
        samples: dataset.samples.len(),
        v: first.v.len(),
        l: first.l.len(),
        d: first.d.len(),
        a: first.a.len(),
    };
    if dims.v == 0 || dims.l == 0 || dims.d == 0 || dims.a == 0 {
        bail!("v/l/d/a vectors must be nonempty");
    }
    let mut sample_ids = BTreeSet::new();
    for sample in &dataset.samples {
        if sample.sample_id.is_empty() {
            bail!("sample_id must not be empty");
        }
        if !sample_ids.insert(sample.sample_id.clone()) {
            bail!("sample_id values must be unique");
        }
        if sample.v.len() != dims.v
            || sample.l.len() != dims.l
            || sample.d.len() != dims.d
            || sample.a.len() != dims.a
        {
            bail!("all v/l/d/a vectors must have consistent dimensions");
        }
        for value in sample
            .v
            .iter()
            .chain(&sample.l)
            .chain(&sample.d)
            .chain(&sample.a)
        {
            if !value.is_finite() {
                bail!("v/l/d/a vectors must contain only finite values");
            }
        }
        for (label, value) in &sample.labels {
            if label.is_empty() {
                bail!("label names must not be empty");
            }
            if value.is_null() {
                bail!("label values must not be null");
            }
        }
        if sample.metadata.keys().any(|key| key.is_empty()) {
            bail!("metadata keys must not be empty");
        }
    }
    Ok(dims)
}

fn label_counts(samples: &[OfflineVldaSample]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for sample in samples {
        for label in sample.labels.keys() {
            *counts.entry(label.clone()).or_insert(0) += 1;
        }
    }
    counts
}

/// Aggregate the per-sample axis-provenance markers into a per-axis honesty summary
/// (see [`OfflineVldaAxisProvenance`]). A `(marker, axis, degraded_values)` is
/// reported only when at least one sample carries that marker; the axis is `degraded`
/// when any sample carries a known-bad value for it.
fn axis_provenance(samples: &[OfflineVldaSample]) -> Vec<OfflineVldaAxisProvenance> {
    // Markers stamped by the capture adapters, with the values that mean "this axis
    // is not trustworthy for this sample". Two capture conventions are recognized:
    //   - ncp-observer (live Engram/NEST tap): `l_source` / `d_source`.
    //   - safe_adapter (offline VLA rollouts): `{v,l,d,a}_provenance`, where
    //     `text_hash_proxy` is a hash surrogate for a missing real feature (degraded),
    //     while `explicit_features` / `hidden_state_pool` / `token_slice:*` /
    //     `action_vector` are honest.
    const DEGRADED_PROV: &[&str] = &["text_hash_proxy", "absent_zeroed", "zeroed", "absent"];
    const MARKERS: &[(&str, &str, &[&str])] = &[
        ("l_source", "L", &["absent_zeroed"]),
        ("d_source", "D", &["recency_fallback", "absent"]),
        ("v_provenance", "V", DEGRADED_PROV),
        ("l_provenance", "L", DEGRADED_PROV),
        ("d_provenance", "D", DEGRADED_PROV),
        ("a_provenance", "A", DEGRADED_PROV),
    ];
    let mut out = Vec::new();
    for &(marker, axis, degraded_values) in MARKERS {
        let mut sources: BTreeMap<String, usize> = BTreeMap::new();
        let mut degraded_samples = 0usize;
        let mut total_samples = 0usize;
        for sample in samples {
            if let Some(value) = sample.metadata.get(marker) {
                *sources.entry(value.clone()).or_insert(0) += 1;
                total_samples += 1;
                if degraded_values.contains(&value.as_str()) {
                    degraded_samples += 1;
                }
            }
        }
        if total_samples == 0 {
            continue; // marker absent (e.g. a synthetic or SAFE-sourced dataset)
        }
        let (status, note) = if degraded_samples > 0 {
            (
                "degraded".to_string(),
                Some(format!(
                    "{degraded_samples}/{total_samples} samples carry a degraded {axis} axis \
                     ({}); PID atoms involving {axis} are NOT trustworthy for those samples",
                    degraded_values.join("/")
                )),
            )
        } else {
            ("ok".to_string(), None)
        };
        out.push(OfflineVldaAxisProvenance {
            marker: marker.to_string(),
            axis: axis.to_string(),
            sources,
            degraded_samples,
            total_samples,
            status,
            note,
        });
    }
    out
}

struct OfflineVldaAnalysis {
    metrics: OfflineVldaMetrics,
    preprocessing: OfflineVldaPreprocessingReport,
    geometry: OfflineVldaGeometryReport,
    train_split_pid: Option<OfflineVldaTrainSplitPidReport>,
    heldout_split: Option<OfflineVldaHeldoutSplitReport>,
    heldout_class_coverage: Option<OfflineVldaHeldoutClassCoverageReport>,
    heldout_episode_disjoint: Option<OfflineVldaHeldoutEpisodeDisjointReport>,
    heldout_predictions: Vec<OfflineVldaHeldoutPredictionRecord>,
    heldout_failure_diagnostics: Vec<OfflineVldaHeldoutFailureDiagnostics>,
}

struct PreparedVldaMatrices {
    v: MatOwned,
    l: MatOwned,
    d: MatOwned,
    a: MatOwned,
    vl: MatOwned,
    vlda: MatOwned,
    preprocessing: OfflineVldaPreprocessingReport,
}

fn compute_analysis(
    samples: &[OfflineVldaSample],
    dims: &OfflineVldaDims,
    pid_mode: PidMode,
    discrete_bins: usize,
    pls_components: usize,
) -> Result<OfflineVldaAnalysis> {
    let prepared = prepare_standardized_embeddings(samples, dims)?;
    let heldout_split = heldout_split_plan(samples);
    if heldout_split.is_none() {
        // `heldout_split_plan` is all-or-nothing: a single sample missing the split
        // key or carrying an unrecognized value voids the ENTIRE plan. If the dataset
        // nonetheless carries recognized split values, that silent void is almost
        // certainly a data error, so surface it instead of dropping all held-out
        // analysis without a word (pass --require-heldout-split to fail hard).
        let mut recognized = 0usize;
        let mut missing = 0usize;
        let mut unrecognized = 0usize;
        for sample in samples {
            match sample.metadata.get(OFFLINE_HELDOUT_SPLIT_METADATA_KEY) {
                None => missing += 1,
                Some(value) => {
                    if split_role(&normalize_split_value(value)).is_some() {
                        recognized += 1;
                    } else {
                        unrecognized += 1;
                    }
                }
            }
        }
        if recognized > 0 {
            eprintln!(
                "[pid-offline-harness] WARNING: held-out split disabled despite {recognized} \
                 sample(s) with a recognized '{}' value — the plan needs both a train and a \
                 held-out class and every sample must carry a recognized value ({missing} missing \
                 the key, {unrecognized} unrecognized). ALL held-out analysis is skipped; fix the \
                 split metadata or pass --require-heldout-split to fail hard.",
                OFFLINE_HELDOUT_SPLIT_METADATA_KEY
            );
        }
    }
    let success_labels = success_labels(samples);
    let heldout_class_coverage = heldout_split
        .as_ref()
        .zip(success_labels.as_deref())
        .map(|(split, labels)| heldout_class_coverage_report(labels, &split.roles));
    let heldout_episode_disjoint = heldout_split
        .as_ref()
        .map(|split| heldout_episode_disjoint_report(samples, &split.roles));
    let metrics = compute_metrics(
        samples,
        &prepared,
        heldout_split.as_ref(),
        pid_mode,
        discrete_bins,
        pls_components,
    )?;
    let train_split_pid = heldout_split.as_ref().map(|split| {
        train_split_pid_report(
            samples,
            dims,
            split,
            pid_mode,
            discrete_bins,
            pls_components,
        )
    });
    let heldout_predictions = heldout_prediction_records(samples, heldout_split.as_ref());
    let heldout_failure_diagnostics = heldout_failure_diagnostics(&heldout_predictions);
    let geometry = compute_geometry_report(&prepared);
    Ok(OfflineVldaAnalysis {
        metrics,
        preprocessing: prepared.preprocessing,
        geometry,
        train_split_pid,
        heldout_split: heldout_split.map(|split| split.report),
        heldout_class_coverage,
        heldout_episode_disjoint,
        heldout_predictions,
        heldout_failure_diagnostics,
    })
}

fn prepare_standardized_embeddings(
    samples: &[OfflineVldaSample],
    dims: &OfflineVldaDims,
) -> Result<PreparedVldaMatrices> {
    let n = samples.len();
    let mut variables = BTreeMap::new();
    let v = flatten(samples, dims.v, |sample| &sample.v);
    let l = flatten(samples, dims.l, |sample| &sample.l);
    let d = flatten(samples, dims.d, |sample| &sample.d);
    let a = flatten(samples, dims.a, |sample| &sample.a);
    let v = standardize_embedding("V", &v, n, dims.v, &mut variables)?;
    let l = standardize_embedding("L", &l, n, dims.l, &mut variables)?;
    let d = standardize_embedding("D", &d, n, dims.d, &mut variables)?;
    let a = standardize_embedding("A", &a, n, dims.a, &mut variables)?;
    let vl = concat_horiz(v.as_ref(), l.as_ref())?;
    let vld = concat_horiz(vl.as_ref(), d.as_ref())?;
    let vlda = concat_horiz(vld.as_ref(), a.as_ref())?;
    Ok(PreparedVldaMatrices {
        v,
        l,
        d,
        a,
        vl,
        vlda,
        preprocessing: OfflineVldaPreprocessingReport {
            strategy: "per_variable_standardized".to_string(),
            variables,
        },
    })
}

fn standardize_embedding(
    name: &str,
    data: &[f64],
    n: usize,
    dim: usize,
    variables: &mut BTreeMap<String, OfflineVldaPreprocessingVariable>,
) -> Result<MatOwned> {
    let raw = MatRef::new(data, n, dim)?;
    let (standardized, standardizer) = Standardizer::fit_transform(raw)?;
    variables.insert(
        name.to_string(),
        OfflineVldaPreprocessingVariable {
            input_dim: dim,
            output_dim: dim,
            zero_variance_dims: zero_variance_dims(data, n, dim),
            mean_sha256: pid_runlog::canonical_json_hash(&standardizer.mean().to_vec())?,
            inv_std_sha256: pid_runlog::canonical_json_hash(&standardizer.inv_std().to_vec())?,
        },
    );
    Ok(standardized)
}

fn zero_variance_dims(data: &[f64], n: usize, dim: usize) -> usize {
    (0..dim)
        .filter(|col| {
            let first = data[*col];
            (1..n).all(|row| data[row * dim + *col] == first)
        })
        .count()
}

fn compute_metrics(
    samples: &[OfflineVldaSample],
    prepared: &PreparedVldaMatrices,
    heldout_split: Option<&OfflineVldaHeldoutSplitPlan>,
    pid_mode: PidMode,
    discrete_bins: usize,
    pls_components: usize,
) -> Result<OfflineVldaMetrics> {
    let pid_screen = compute_pid_screen_metrics(prepared, pid_mode, discrete_bins, pls_components)?;
    let success_labels = success_labels(samples);
    let (success_rate, majority_success_accuracy) = success_metrics(&success_labels);
    let loo_nn_v_success_accuracy = success_labels
        .as_deref()
        .map(|labels| loo_nn_success_accuracy(samples, labels, |sample| sample.v.clone()));
    let loo_nn_l_success_accuracy = success_labels
        .as_deref()
        .map(|labels| loo_nn_success_accuracy(samples, labels, |sample| sample.l.clone()));
    let loo_nn_d_success_accuracy = success_labels
        .as_deref()
        .map(|labels| loo_nn_success_accuracy(samples, labels, |sample| sample.d.clone()));
    let loo_nn_a_success_accuracy = success_labels
        .as_deref()
        .map(|labels| loo_nn_success_accuracy(samples, labels, |sample| sample.a.clone()));
    let loo_nn_vlda_success_accuracy = success_labels.as_deref().map(|labels| {
        loo_nn_success_accuracy(samples, labels, |sample| {
            let mut values = Vec::with_capacity(
                sample.v.len() + sample.l.len() + sample.d.len() + sample.a.len(),
            );
            values.extend_from_slice(&sample.v);
            values.extend_from_slice(&sample.l);
            values.extend_from_slice(&sample.d);
            values.extend_from_slice(&sample.a);
            values
        })
    });
    let episode_ids = episode_ids(samples);
    let episode_loo_majority_success_accuracy = success_labels
        .as_deref()
        .zip(episode_ids.as_deref())
        .map(|(labels, episode_ids)| episode_loo_majority_success_accuracy(labels, episode_ids));
    let episode_loo_nn_v_success_accuracy = success_labels
        .as_deref()
        .zip(episode_ids.as_deref())
        .map(|(labels, episode_ids)| {
            episode_loo_nn_success_accuracy(samples, labels, episode_ids, |sample| sample.v.clone())
        });
    let episode_loo_nn_l_success_accuracy = success_labels
        .as_deref()
        .zip(episode_ids.as_deref())
        .map(|(labels, episode_ids)| {
            episode_loo_nn_success_accuracy(samples, labels, episode_ids, |sample| sample.l.clone())
        });
    let episode_loo_nn_d_success_accuracy = success_labels
        .as_deref()
        .zip(episode_ids.as_deref())
        .map(|(labels, episode_ids)| {
            episode_loo_nn_success_accuracy(samples, labels, episode_ids, |sample| sample.d.clone())
        });
    let episode_loo_nn_a_success_accuracy = success_labels
        .as_deref()
        .zip(episode_ids.as_deref())
        .map(|(labels, episode_ids)| {
            episode_loo_nn_success_accuracy(samples, labels, episode_ids, |sample| sample.a.clone())
        });
    let episode_loo_nn_vlda_success_accuracy = success_labels
        .as_deref()
        .zip(episode_ids.as_deref())
        .map(|(labels, episode_ids)| {
            episode_loo_nn_success_accuracy(samples, labels, episode_ids, |sample| {
                let mut values = Vec::with_capacity(
                    sample.v.len() + sample.l.len() + sample.d.len() + sample.a.len(),
                );
                values.extend_from_slice(&sample.v);
                values.extend_from_slice(&sample.l);
                values.extend_from_slice(&sample.d);
                values.extend_from_slice(&sample.a);
                values
            })
        });
    let heldout_majority_success_metrics = success_labels
        .as_deref()
        .zip(heldout_split)
        .map(|(labels, split)| heldout_majority_success_metrics(labels, &split.roles));
    let heldout_majority_success_accuracy =
        heldout_majority_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_majority_success_balanced_accuracy =
        heldout_majority_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_nn_v_success_metrics =
        success_labels
            .as_deref()
            .zip(heldout_split)
            .map(|(labels, split)| {
                heldout_nn_success_metrics(samples, labels, &split.roles, |sample| sample.v.clone())
            });
    let heldout_nn_l_success_metrics =
        success_labels
            .as_deref()
            .zip(heldout_split)
            .map(|(labels, split)| {
                heldout_nn_success_metrics(samples, labels, &split.roles, |sample| sample.l.clone())
            });
    let heldout_nn_d_success_metrics =
        success_labels
            .as_deref()
            .zip(heldout_split)
            .map(|(labels, split)| {
                heldout_nn_success_metrics(samples, labels, &split.roles, |sample| sample.d.clone())
            });
    let heldout_nn_a_success_metrics =
        success_labels
            .as_deref()
            .zip(heldout_split)
            .map(|(labels, split)| {
                heldout_nn_success_metrics(samples, labels, &split.roles, |sample| sample.a.clone())
            });
    let heldout_nn_vlda_success_metrics =
        success_labels
            .as_deref()
            .zip(heldout_split)
            .map(|(labels, split)| {
                heldout_nn_success_metrics(samples, labels, &split.roles, |sample| {
                    let mut values = Vec::with_capacity(
                        sample.v.len() + sample.l.len() + sample.d.len() + sample.a.len(),
                    );
                    values.extend_from_slice(&sample.v);
                    values.extend_from_slice(&sample.l);
                    values.extend_from_slice(&sample.d);
                    values.extend_from_slice(&sample.a);
                    values
                })
            });
    let heldout_nn_v_success_accuracy =
        heldout_nn_v_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_nn_l_success_accuracy =
        heldout_nn_l_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_nn_d_success_accuracy =
        heldout_nn_d_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_nn_a_success_accuracy =
        heldout_nn_a_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_nn_vlda_success_accuracy =
        heldout_nn_vlda_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_nn_v_success_balanced_accuracy =
        heldout_nn_v_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_nn_l_success_balanced_accuracy =
        heldout_nn_l_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_nn_d_success_balanced_accuracy =
        heldout_nn_d_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_nn_a_success_balanced_accuracy =
        heldout_nn_a_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_nn_vlda_success_balanced_accuracy =
        heldout_nn_vlda_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_centroid_v_success_metrics =
        success_labels
            .as_deref()
            .zip(heldout_split)
            .and_then(|(labels, split)| {
                heldout_centroid_success_metrics(samples, labels, &split.roles, |sample| {
                    sample.v.clone()
                })
            });
    let heldout_centroid_l_success_metrics =
        success_labels
            .as_deref()
            .zip(heldout_split)
            .and_then(|(labels, split)| {
                heldout_centroid_success_metrics(samples, labels, &split.roles, |sample| {
                    sample.l.clone()
                })
            });
    let heldout_centroid_d_success_metrics =
        success_labels
            .as_deref()
            .zip(heldout_split)
            .and_then(|(labels, split)| {
                heldout_centroid_success_metrics(samples, labels, &split.roles, |sample| {
                    sample.d.clone()
                })
            });
    let heldout_centroid_a_success_metrics =
        success_labels
            .as_deref()
            .zip(heldout_split)
            .and_then(|(labels, split)| {
                heldout_centroid_success_metrics(samples, labels, &split.roles, |sample| {
                    sample.a.clone()
                })
            });
    let heldout_centroid_vlda_success_metrics = success_labels
        .as_deref()
        .zip(heldout_split)
        .and_then(|(labels, split)| {
            heldout_centroid_success_metrics(samples, labels, &split.roles, |sample| {
                let mut values = Vec::with_capacity(
                    sample.v.len() + sample.l.len() + sample.d.len() + sample.a.len(),
                );
                values.extend_from_slice(&sample.v);
                values.extend_from_slice(&sample.l);
                values.extend_from_slice(&sample.d);
                values.extend_from_slice(&sample.a);
                values
            })
        });
    let heldout_centroid_v_success_accuracy =
        heldout_centroid_v_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_centroid_l_success_accuracy =
        heldout_centroid_l_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_centroid_d_success_accuracy =
        heldout_centroid_d_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_centroid_a_success_accuracy =
        heldout_centroid_a_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_centroid_vlda_success_accuracy =
        heldout_centroid_vlda_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_centroid_v_success_balanced_accuracy =
        heldout_centroid_v_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_centroid_l_success_balanced_accuracy =
        heldout_centroid_l_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_centroid_d_success_balanced_accuracy =
        heldout_centroid_d_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_centroid_a_success_balanced_accuracy =
        heldout_centroid_a_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_centroid_vlda_success_balanced_accuracy =
        heldout_centroid_vlda_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_centroid_v_success_auroc =
        heldout_centroid_v_success_metrics.and_then(|metrics| metrics.auroc);
    let heldout_centroid_l_success_auroc =
        heldout_centroid_l_success_metrics.and_then(|metrics| metrics.auroc);
    let heldout_centroid_d_success_auroc =
        heldout_centroid_d_success_metrics.and_then(|metrics| metrics.auroc);
    let heldout_centroid_a_success_auroc =
        heldout_centroid_a_success_metrics.and_then(|metrics| metrics.auroc);
    let heldout_centroid_vlda_success_auroc =
        heldout_centroid_vlda_success_metrics.and_then(|metrics| metrics.auroc);
    // SAFE-class internal-feature failure detector (logistic regression on pooled
    // train-standardized VLDA features; fit on train, scored on held-out).
    let heldout_logreg_vlda_success_metrics = success_labels
        .as_deref()
        .zip(heldout_split)
        .and_then(|(labels, split)| {
            heldout_logreg_success_metrics(samples, labels, &split.roles, |sample| {
                let mut values = Vec::with_capacity(
                    sample.v.len() + sample.l.len() + sample.d.len() + sample.a.len(),
                );
                values.extend_from_slice(&sample.v);
                values.extend_from_slice(&sample.l);
                values.extend_from_slice(&sample.d);
                values.extend_from_slice(&sample.a);
                values
            })
        });
    let heldout_logreg_vlda_success_accuracy =
        heldout_logreg_vlda_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_logreg_vlda_success_balanced_accuracy =
        heldout_logreg_vlda_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_logreg_vlda_success_auroc =
        heldout_logreg_vlda_success_metrics.and_then(|metrics| metrics.auroc);
    Ok(OfflineVldaMetrics {
        mi_v_action: pid_screen.mi_v_action,
        mi_l_action: pid_screen.mi_l_action,
        mi_d_action: pid_screen.mi_d_action,
        mi_vl_action: pid_screen.mi_vl_action,
        co_information_v_l_action: pid_screen.co_information_v_l_action,
        redundancy_v_l_action: pid_screen.redundancy_v_l_action,
        unique_v_action: pid_screen.unique_v_action,
        unique_l_action: pid_screen.unique_l_action,
        synergy_v_l_action: pid_screen.synergy_v_l_action,
        success_rate,
        majority_success_accuracy,
        loo_nn_v_success_accuracy,
        loo_nn_l_success_accuracy,
        loo_nn_d_success_accuracy,
        loo_nn_a_success_accuracy,
        loo_nn_vlda_success_accuracy,
        episode_loo_majority_success_accuracy,
        episode_loo_nn_v_success_accuracy,
        episode_loo_nn_l_success_accuracy,
        episode_loo_nn_d_success_accuracy,
        episode_loo_nn_a_success_accuracy,
        episode_loo_nn_vlda_success_accuracy,
        heldout_majority_success_accuracy,
        heldout_majority_success_balanced_accuracy,
        heldout_nn_v_success_accuracy,
        heldout_nn_l_success_accuracy,
        heldout_nn_d_success_accuracy,
        heldout_nn_a_success_accuracy,
        heldout_nn_vlda_success_accuracy,
        heldout_nn_v_success_balanced_accuracy,
        heldout_nn_l_success_balanced_accuracy,
        heldout_nn_d_success_balanced_accuracy,
        heldout_nn_a_success_balanced_accuracy,
        heldout_nn_vlda_success_balanced_accuracy,
        heldout_centroid_v_success_accuracy,
        heldout_centroid_l_success_accuracy,
        heldout_centroid_d_success_accuracy,
        heldout_centroid_a_success_accuracy,
        heldout_centroid_vlda_success_accuracy,
        heldout_centroid_v_success_balanced_accuracy,
        heldout_centroid_l_success_balanced_accuracy,
        heldout_centroid_d_success_balanced_accuracy,
        heldout_centroid_a_success_balanced_accuracy,
        heldout_centroid_vlda_success_balanced_accuracy,
        heldout_centroid_v_success_auroc,
        heldout_centroid_l_success_auroc,
        heldout_centroid_d_success_auroc,
        heldout_centroid_a_success_auroc,
        heldout_centroid_vlda_success_auroc,
        heldout_logreg_vlda_success_accuracy,
        heldout_logreg_vlda_success_balanced_accuracy,
        heldout_logreg_vlda_success_auroc,
        pid_pairs: pid_screen.pid_pairs,
    })
}

#[derive(Debug, Clone, Copy)]
struct OfflineVldaSourceMatrix<'a> {
    name: &'static str,
    matrix: MatRef<'a>,
    mi_action: f64,
}

#[derive(Debug, Clone, Copy)]
struct OfflineVldaTargetMatrix<'a> {
    name: &'static str,
    matrix: MatRef<'a>,
}

fn compute_pid_pair_metrics(
    source_1: OfflineVldaSourceMatrix<'_>,
    source_2: OfflineVldaSourceMatrix<'_>,
    target: OfflineVldaTargetMatrix<'_>,
    ksg: &KsgConfig,
    pid_cfg: &Pid2Config,
) -> Result<OfflineVldaPidPairMetrics> {
    let pid = pid2_isx(source_1.matrix, source_2.matrix, target.matrix, pid_cfg)?;
    Ok(OfflineVldaPidPairMetrics {
        source_1: source_1.name.to_string(),
        source_2: source_2.name.to_string(),
        target: target.name.to_string(),
        mi_source_1_action: source_1.mi_action,
        mi_source_2_action: source_2.mi_action,
        mi_joint_action: pid_core::ksg_mi_concat_xy(
            source_1.matrix,
            source_2.matrix,
            target.matrix,
            ksg,
        )?,
        co_information: co_information_pairwise(
            source_1.matrix,
            source_2.matrix,
            target.matrix,
            ksg,
        )?,
        redundancy: pid.redundancy,
        unique_source_1: pid.unique_s1,
        unique_source_2: pid.unique_s2,
        synergy: pid.synergy,
        discrete_saturation: None,
    })
}

/// Fraction of rows whose bin pattern is unique (1.0 = every sample in its own bin).
fn unique_row_fraction(bins: &[Vec<usize>]) -> f64 {
    if bins.is_empty() {
        return 0.0;
    }
    let unique: std::collections::HashSet<&Vec<usize>> = bins.iter().collect();
    unique.len() as f64 / bins.len() as f64
}

/// Discrete-mode PID pair metrics: quantization + counting-based entropy instead of kNN.
///
/// Redundancy is the Williams–Beer-style `I_min` functional (grandplan §8.1.6), not
/// discrete `i^sx_∩`. Saturation diagnostics flag regimes where plug-in MI is pinned
/// to entropy ceilings by unique-bin sparsity.
fn compute_pid_pair_metrics_discrete(
    source_1: OfflineVldaSourceMatrix<'_>,
    source_2: OfflineVldaSourceMatrix<'_>,
    target: OfflineVldaTargetMatrix<'_>,
    num_bins: usize,
) -> Result<OfflineVldaPidPairMetrics> {
    let pid = pid_core::discrete_pid2(source_1.matrix, source_2.matrix, target.matrix, num_bins)?;
    // Compute MI(S1,S2;T) for the co-information term.
    let s1s2 = concat_horiz(source_1.matrix, source_2.matrix)?;
    let s1s2_bins = quantize_equal_width(s1s2.as_ref(), num_bins)?;
    let t_bins = quantize_equal_width(target.matrix, num_bins)?;
    let mi_s1s2_t = discrete_mi(&s1s2_bins, &t_bins, num_bins)?;
    // Co-information: MI(S1;T) + MI(S2;T) - MI(S1,S2;T)
    let co_information = pid.mi_s1_t + pid.mi_s2_t - mi_s1s2_t;
    // Saturation diagnostics (grandplan §8.1.6).
    let s1_bins = quantize_equal_width(source_1.matrix, num_bins)?;
    let s2_bins = quantize_equal_width(source_2.matrix, num_bins)?;
    let joint_bins: Vec<Vec<usize>> = s1s2_bins
        .iter()
        .zip(&t_bins)
        .map(|(st, t)| {
            let mut row = st.clone();
            row.extend_from_slice(t);
            row
        })
        .collect();
    let unique_fraction_source_1 = unique_row_fraction(&s1_bins);
    let unique_fraction_source_2 = unique_row_fraction(&s2_bins);
    let unique_fraction_target = unique_row_fraction(&t_bins);
    let unique_fraction_joint = unique_row_fraction(&joint_bins);
    let saturation_warning = [
        unique_fraction_source_1,
        unique_fraction_source_2,
        unique_fraction_target,
        unique_fraction_joint,
    ]
    .iter()
    .any(|&fraction| fraction > OFFLINE_DISCRETE_SATURATION_UNIQUE_FRACTION_MAX);
    Ok(OfflineVldaPidPairMetrics {
        source_1: source_1.name.to_string(),
        source_2: source_2.name.to_string(),
        target: target.name.to_string(),
        mi_source_1_action: pid.mi_s1_t,
        mi_source_2_action: pid.mi_s2_t,
        mi_joint_action: mi_s1s2_t,
        co_information,
        redundancy: pid.redundancy,
        unique_source_1: pid.unique_s1,
        unique_source_2: pid.unique_s2,
        synergy: pid.synergy,
        discrete_saturation: Some(OfflineVldaDiscreteSaturation {
            unique_fraction_source_1,
            unique_fraction_source_2,
            unique_fraction_target,
            unique_fraction_joint,
            saturation_warning,
        }),
    })
}

fn compute_geometry_report(prepared: &PreparedVldaMatrices) -> OfflineVldaGeometryReport {
    let metric = Metric::Chebyshev;
    let intrinsic_cfg = IntrinsicDimConfig {
        k: OFFLINE_GEOMETRY_INTRINSIC_K,
        metric,
    };
    let distance_cfg = DistanceConcentrationConfig { metric };
    let hyperbolicity_cfg = HyperbolicityConfig {
        n_samples: OFFLINE_GEOMETRY_HYPERBOLICITY_SAMPLES,
        metric,
        seed: 0x2026_0509,
    };
    let mut variables = BTreeMap::new();
    for (name, matrix) in [
        ("V", prepared.v.as_ref()),
        ("L", prepared.l.as_ref()),
        ("D", prepared.d.as_ref()),
        ("A", prepared.a.as_ref()),
        ("VL", prepared.vl.as_ref()),
        ("VLDA", prepared.vlda.as_ref()),
    ] {
        variables.insert(
            name.to_string(),
            compute_geometry_variable(matrix, &intrinsic_cfg, &distance_cfg, &hyperbolicity_cfg),
        );
    }
    let gates = compute_geometry_gates(&variables, &prepared.preprocessing);
    OfflineVldaGeometryReport {
        space: "per_variable_standardized".to_string(),
        metric: "chebyshev".to_string(),
        intrinsic_k: OFFLINE_GEOMETRY_INTRINSIC_K,
        hyperbolicity_samples: OFFLINE_GEOMETRY_HYPERBOLICITY_SAMPLES,
        gates,
        variables,
    }
}

fn compute_geometry_variable(
    matrix: MatRef<'_>,
    intrinsic_cfg: &IntrinsicDimConfig,
    distance_cfg: &DistanceConcentrationConfig,
    hyperbolicity_cfg: &HyperbolicityConfig,
) -> OfflineVldaGeometryVariable {
    let (intrinsic_dimension, intrinsic_dimension_error) =
        match intrinsic_dimension_levina_bickel(matrix, intrinsic_cfg) {
            Ok(value) if value.is_finite() => (Some(value), None),
            Ok(_) => (None, Some("intrinsic dimension was non-finite".to_string())),
            Err(err) => (None, Some(format!("{err}"))),
        };
    let (
        pairwise_count,
        pairwise_min,
        pairwise_max,
        pairwise_mean,
        pairwise_cv,
        nn_mean,
        nn_over_pairwise_mean,
        distance_concentration_error,
    ) = match distance_concentration_stats(matrix, distance_cfg) {
        Ok(stats) => (
            Some(stats.pairwise_count),
            finite_option(stats.pairwise_min),
            finite_option(stats.pairwise_max),
            finite_option(stats.pairwise_mean),
            finite_option(stats.pairwise_cv),
            finite_option(stats.nn_mean),
            finite_option(stats.nn_over_pairwise_mean),
            None,
        ),
        Err(err) => (
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(format!("{err}")),
        ),
    };
    let (gromov_delta, gromov_error) = match gromov_hyperbolicity(matrix, hyperbolicity_cfg) {
        Ok(value) if value.is_finite() => (Some(value), None),
        Ok(_) => (None, Some("gromov delta was non-finite".to_string())),
        Err(err) => (None, Some(format!("{err}"))),
    };
    let gromov_delta_rel = match (gromov_delta, pairwise_max) {
        (Some(delta), Some(diameter)) if diameter > 0.0 => finite_option((2.0 * delta) / diameter),
        _ => None,
    };
    OfflineVldaGeometryVariable {
        dims: vec![matrix.nrows(), matrix.ncols()],
        intrinsic_dimension,
        intrinsic_dimension_error,
        pairwise_count,
        pairwise_min,
        pairwise_max,
        pairwise_mean,
        pairwise_cv,
        nn_mean,
        nn_over_pairwise_mean,
        distance_concentration_error,
        gromov_delta,
        gromov_delta_rel,
        gromov_error,
    }
}

fn finite_option(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn compute_geometry_gates(
    variables: &BTreeMap<String, OfflineVldaGeometryVariable>,
    preprocessing: &OfflineVldaPreprocessingReport,
) -> OfflineVldaGeometryGates {
    let mut warnings = Vec::new();
    // Degenerate-axis guard: a variable whose every dimension is constant has zero
    // variance, hence zero mutual information with anything by construction, so every
    // PID atom that involves it is invalid (not merely small). This reuses the
    // already-computed `zero_variance_dims` so an all-zeroed channel (e.g. a fabricated
    // all-zero L from an absent language channel — see NCP_DEV_PROMPT Gap 2) is flagged
    // loudly rather than silently passed through the gates.
    for (name, variable) in &preprocessing.variables {
        if variable.input_dim > 0 && variable.zero_variance_dims == variable.input_dim {
            warnings.push(format!(
                "geometry {name} is all-constant (zero_variance_dims == input_dim == {}): \
                 zero variance implies zero mutual information by construction, so every \
                 PID atom involving {name} is degenerate/invalid",
                variable.input_dim
            ));
        }
    }
    for (name, variable) in variables {
        match variable.intrinsic_dimension {
            Some(value) if value > OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION => warnings.push(
                format!(
                    "geometry {name} intrinsic_dimension {value:.4} exceeds {OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION:.4}"
                ),
            ),
            Some(_) => {}
            None => warnings.push(format!(
                "geometry {name} intrinsic_dimension unavailable: {}",
                variable
                    .intrinsic_dimension_error
                    .as_deref()
                    .unwrap_or("unknown error")
            )),
        }
        match variable.pairwise_cv {
            Some(value) if value < OFFLINE_GEOMETRY_MIN_PAIRWISE_CV => warnings.push(format!(
                "geometry {name} pairwise_cv {value:.4} is below {OFFLINE_GEOMETRY_MIN_PAIRWISE_CV:.4}"
            )),
            Some(_) => {}
            None => warnings.push(format!(
                "geometry {name} distance concentration unavailable: {}",
                variable
                    .distance_concentration_error
                    .as_deref()
                    .unwrap_or("unknown error")
            )),
        }
        match variable.gromov_delta_rel {
            Some(value) if value < OFFLINE_GEOMETRY_MIN_DELTA_REL => warnings.push(format!(
                "geometry {name} delta_rel {value:.4} is below {OFFLINE_GEOMETRY_MIN_DELTA_REL:.4}"
            )),
            Some(_) => {}
            None => warnings.push(format!(
                "geometry {name} delta_rel unavailable: {}",
                variable
                    .gromov_error
                    .as_deref()
                    .unwrap_or("missing diameter")
            )),
        }
    }
    OfflineVldaGeometryGates {
        status: if warnings.is_empty() {
            "pass".to_string()
        } else {
            "warn".to_string()
        },
        max_intrinsic_dimension: OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION,
        min_pairwise_cv: OFFLINE_GEOMETRY_MIN_PAIRWISE_CV,
        min_delta_rel: OFFLINE_GEOMETRY_MIN_DELTA_REL,
        warnings,
    }
}

fn flatten<F>(samples: &[OfflineVldaSample], dim: usize, values: F) -> Vec<f64>
where
    F: Fn(&OfflineVldaSample) -> &[f64],
{
    let mut out = Vec::with_capacity(samples.len() * dim);
    for sample in samples {
        out.extend_from_slice(values(sample));
    }
    out
}

#[derive(Debug, Clone)]
struct OfflineVldaHeldoutSplitPlan {
    report: OfflineVldaHeldoutSplitReport,
    roles: Vec<OfflineVldaSplitRole>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OfflineVldaSplitRole {
    Train,
    Heldout,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct OfflineVldaHeldoutSplitDiagnostics {
    train_samples: usize,
    heldout_samples: usize,
    missing_samples: usize,
    unrecognized_samples: usize,
}

fn heldout_split_plan(samples: &[OfflineVldaSample]) -> Option<OfflineVldaHeldoutSplitPlan> {
    let mut roles = Vec::with_capacity(samples.len());
    let mut value_counts = BTreeMap::new();
    let mut train_sample_ids = Vec::new();
    let mut heldout_sample_ids = Vec::new();
    for sample in samples {
        let value = sample.metadata.get(OFFLINE_HELDOUT_SPLIT_METADATA_KEY)?;
        let normalized = normalize_split_value(value);
        let role = split_role(&normalized)?;
        *value_counts.entry(normalized).or_insert(0) += 1;
        match role {
            OfflineVldaSplitRole::Train => train_sample_ids.push(sample.sample_id.clone()),
            OfflineVldaSplitRole::Heldout => heldout_sample_ids.push(sample.sample_id.clone()),
        }
        roles.push(role);
    }
    (!train_sample_ids.is_empty() && !heldout_sample_ids.is_empty()).then_some(
        OfflineVldaHeldoutSplitPlan {
            report: OfflineVldaHeldoutSplitReport {
                metadata_key: OFFLINE_HELDOUT_SPLIT_METADATA_KEY.to_string(),
                train_values: vec!["train".to_string(), "training".to_string()],
                heldout_values: vec![
                    "test".to_string(),
                    "validation".to_string(),
                    "val".to_string(),
                    "eval".to_string(),
                    "evaluation".to_string(),
                    "heldout".to_string(),
                    "holdout".to_string(),
                    "held_out".to_string(),
                    "hold_out".to_string(),
                ],
                train_samples: train_sample_ids.len(),
                heldout_samples: heldout_sample_ids.len(),
                value_counts,
                train_sample_ids,
                heldout_sample_ids,
            },
            roles,
        },
    )
}

fn heldout_split_diagnostics(dataset: &OfflineVldaDataset) -> OfflineVldaHeldoutSplitDiagnostics {
    let mut diagnostics = OfflineVldaHeldoutSplitDiagnostics::default();
    for sample in &dataset.samples {
        let Some(value) = sample.metadata.get(OFFLINE_HELDOUT_SPLIT_METADATA_KEY) else {
            diagnostics.missing_samples += 1;
            continue;
        };
        match split_role(&normalize_split_value(value)) {
            Some(OfflineVldaSplitRole::Train) => diagnostics.train_samples += 1,
            Some(OfflineVldaSplitRole::Heldout) => diagnostics.heldout_samples += 1,
            None => diagnostics.unrecognized_samples += 1,
        }
    }
    diagnostics
}

fn heldout_class_coverage_report(
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
) -> OfflineVldaHeldoutClassCoverageReport {
    let mut train_successes = 0;
    let mut train_failures = 0;
    let mut heldout_successes = 0;
    let mut heldout_failures = 0;
    for (label, role) in labels.iter().zip(roles) {
        match (role, label) {
            (OfflineVldaSplitRole::Train, true) => train_successes += 1,
            (OfflineVldaSplitRole::Train, false) => train_failures += 1,
            (OfflineVldaSplitRole::Heldout, true) => heldout_successes += 1,
            (OfflineVldaSplitRole::Heldout, false) => heldout_failures += 1,
        }
    }
    let mut warnings = Vec::new();
    if train_successes == 0 {
        warnings.push("train split has no success=true samples".to_string());
    }
    if train_failures == 0 {
        warnings.push("train split has no success=false samples".to_string());
    }
    if heldout_successes == 0 {
        warnings.push("held-out split has no success=true samples".to_string());
    }
    if heldout_failures == 0 {
        warnings.push("held-out split has no success=false samples".to_string());
    }
    OfflineVldaHeldoutClassCoverageReport {
        metadata_key: OFFLINE_HELDOUT_SPLIT_METADATA_KEY.to_string(),
        status: if warnings.is_empty() {
            "pass".to_string()
        } else {
            "warn".to_string()
        },
        train_successes,
        train_failures,
        heldout_successes,
        heldout_failures,
        warnings,
    }
}

fn heldout_episode_disjoint_report(
    samples: &[OfflineVldaSample],
    roles: &[OfflineVldaSplitRole],
) -> OfflineVldaHeldoutEpisodeDisjointReport {
    let mut train_episode_ids = BTreeSet::new();
    let mut heldout_episode_ids = BTreeSet::new();
    let mut missing_episode_samples = 0;
    for (sample, role) in samples.iter().zip(roles) {
        let Some(episode_id) = &sample.episode_id else {
            missing_episode_samples += 1;
            continue;
        };
        match role {
            OfflineVldaSplitRole::Train => {
                train_episode_ids.insert(episode_id.clone());
            }
            OfflineVldaSplitRole::Heldout => {
                heldout_episode_ids.insert(episode_id.clone());
            }
        }
    }
    let shared_episode_ids = train_episode_ids
        .intersection(&heldout_episode_ids)
        .cloned()
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    if missing_episode_samples > 0 {
        warnings.push(format!(
            "{missing_episode_samples} sample(s) missing episode_id for split leakage audit"
        ));
    }
    if !shared_episode_ids.is_empty() {
        warnings.push(format!(
            "{} episode_id(s) appear in both train and held-out splits",
            shared_episode_ids.len()
        ));
    }
    OfflineVldaHeldoutEpisodeDisjointReport {
        split_metadata_key: OFFLINE_HELDOUT_SPLIT_METADATA_KEY.to_string(),
        episode_key: "episode_id".to_string(),
        status: if warnings.is_empty() {
            "pass".to_string()
        } else {
            "warn".to_string()
        },
        train_episodes: train_episode_ids.len(),
        heldout_episodes: heldout_episode_ids.len(),
        shared_episodes: shared_episode_ids.len(),
        missing_episode_samples,
        shared_episode_ids,
        warnings,
    }
}

fn normalize_split_value(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('-', "_")
}

fn split_role(value: &str) -> Option<OfflineVldaSplitRole> {
    match value {
        "train" | "training" => Some(OfflineVldaSplitRole::Train),
        "test" | "validation" | "val" | "eval" | "evaluation" | "heldout" | "holdout"
        | "held_out" | "hold_out" => Some(OfflineVldaSplitRole::Heldout),
        _ => None,
    }
}

fn success_labels(samples: &[OfflineVldaSample]) -> Option<Vec<bool>> {
    let labels = samples
        .iter()
        .filter_map(|sample| sample.labels.get("success").and_then(Value::as_bool))
        .collect::<Vec<_>>();
    if labels.len() != samples.len() {
        None
    } else {
        Some(labels)
    }
}

fn success_metrics(labels: &Option<Vec<bool>>) -> (Option<f64>, Option<f64>) {
    let Some(labels) = labels else {
        return (None, None);
    };
    let successes = labels.iter().filter(|value| **value).count();
    let success_rate = successes as f64 / labels.len() as f64;
    let majority = success_rate >= 0.5;
    let majority_success_accuracy =
        labels.iter().filter(|value| **value == majority).count() as f64 / labels.len() as f64;
    (Some(success_rate), Some(majority_success_accuracy))
}

fn heldout_prediction_records(
    samples: &[OfflineVldaSample],
    split: Option<&OfflineVldaHeldoutSplitPlan>,
) -> Vec<OfflineVldaHeldoutPredictionRecord> {
    let Some(labels) = success_labels(samples) else {
        return Vec::new();
    };
    let Some(split) = split else {
        return Vec::new();
    };
    let mut records = Vec::new();
    append_heldout_majority_prediction_records(&mut records, samples, &labels, &split.roles);
    append_heldout_nn_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "V",
        |sample| sample.v.clone(),
    );
    append_heldout_nn_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "L",
        |sample| sample.l.clone(),
    );
    append_heldout_nn_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "D",
        |sample| sample.d.clone(),
    );
    append_heldout_nn_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "A",
        |sample| sample.a.clone(),
    );
    append_heldout_nn_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "VLDA",
        vlda_values,
    );
    append_heldout_centroid_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "V",
        |sample| sample.v.clone(),
    );
    append_heldout_centroid_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "L",
        |sample| sample.l.clone(),
    );
    append_heldout_centroid_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "D",
        |sample| sample.d.clone(),
    );
    append_heldout_centroid_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "A",
        |sample| sample.a.clone(),
    );
    append_heldout_centroid_prediction_records(
        &mut records,
        samples,
        &labels,
        &split.roles,
        "VLDA",
        vlda_values,
    );
    records
}

fn heldout_failure_diagnostics(
    records: &[OfflineVldaHeldoutPredictionRecord],
) -> Vec<OfflineVldaHeldoutFailureDiagnostics> {
    let mut diagnostics = Vec::new();
    for record in records {
        let idx =
            diagnostics
                .iter()
                .position(|diagnostic: &OfflineVldaHeldoutFailureDiagnostics| {
                    diagnostic.classifier == record.classifier
                        && diagnostic.variable.as_deref() == record.variable.as_deref()
                });
        let diagnostic_idx = match idx {
            Some(idx) => idx,
            None => {
                diagnostics.push(OfflineVldaHeldoutFailureDiagnostics {
                    classifier: record.classifier.clone(),
                    variable: record.variable.clone(),
                    samples: 0,
                    true_failures: 0,
                    true_successes: 0,
                    predicted_failures: 0,
                    predicted_successes: 0,
                    failure_true_positives: 0,
                    failure_false_positives: 0,
                    failure_true_negatives: 0,
                    failure_false_negatives: 0,
                    failure_precision: None,
                    failure_recall: None,
                    failure_specificity: None,
                    failure_f1: None,
                });
                diagnostics.len() - 1
            }
        };
        let diagnostic = &mut diagnostics[diagnostic_idx];
        diagnostic.samples += 1;
        if record.true_success {
            diagnostic.true_successes += 1;
        } else {
            diagnostic.true_failures += 1;
        }
        if record.predicted_success {
            diagnostic.predicted_successes += 1;
        } else {
            diagnostic.predicted_failures += 1;
        }
        match (record.true_success, record.predicted_success) {
            (false, false) => diagnostic.failure_true_positives += 1,
            (true, false) => diagnostic.failure_false_positives += 1,
            (true, true) => diagnostic.failure_true_negatives += 1,
            (false, true) => diagnostic.failure_false_negatives += 1,
        }
    }
    for diagnostic in &mut diagnostics {
        diagnostic.failure_precision = nonzero_ratio(
            diagnostic.failure_true_positives,
            diagnostic.predicted_failures,
        );
        diagnostic.failure_recall =
            nonzero_ratio(diagnostic.failure_true_positives, diagnostic.true_failures);
        diagnostic.failure_specificity =
            nonzero_ratio(diagnostic.failure_true_negatives, diagnostic.true_successes);
        diagnostic.failure_f1 = nonzero_ratio(
            2 * diagnostic.failure_true_positives,
            2 * diagnostic.failure_true_positives
                + diagnostic.failure_false_positives
                + diagnostic.failure_false_negatives,
        );
    }
    diagnostics
}

fn nonzero_ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then_some(numerator as f64 / denominator as f64)
}

struct OfflineVldaHeldoutPredictionInput<'a> {
    classifier: &'a str,
    variable: Option<&'a str>,
    predicted_success: bool,
    score: Option<f64>,
    score_name: Option<String>,
    nearest_train_sample_id: Option<String>,
    squared_distance: Option<f64>,
}

fn append_heldout_majority_prediction_records(
    records: &mut Vec<OfflineVldaHeldoutPredictionRecord>,
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
) {
    let mut train_successes = 0;
    let mut train_total = 0;
    for (label, role) in labels.iter().zip(roles) {
        if *role == OfflineVldaSplitRole::Train {
            train_total += 1;
            if *label {
                train_successes += 1;
            }
        }
    }
    let prediction = train_successes * 2 >= train_total;
    for idx in heldout_indices(roles) {
        records.push(heldout_prediction_record(
            samples,
            labels,
            idx,
            OfflineVldaHeldoutPredictionInput {
                classifier: "train_split_majority",
                variable: None,
                predicted_success: prediction,
                score: None,
                score_name: None,
                nearest_train_sample_id: None,
                squared_distance: None,
            },
        ));
    }
}

fn append_heldout_nn_prediction_records<F>(
    records: &mut Vec<OfflineVldaHeldoutPredictionRecord>,
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    variable: &str,
    values: F,
) where
    F: Fn(&OfflineVldaSample) -> Vec<f64>,
{
    let features = samples.iter().map(values).collect::<Vec<_>>();
    for idx in heldout_indices(roles) {
        let (nearest_idx, squared_distance) =
            nearest_neighbor_in_train(samples, &features, &features[idx], roles);
        records.push(heldout_prediction_record(
            samples,
            labels,
            idx,
            OfflineVldaHeldoutPredictionInput {
                classifier: "train_split_1nn",
                variable: Some(variable),
                predicted_success: labels[nearest_idx],
                score: None,
                score_name: None,
                nearest_train_sample_id: Some(samples[nearest_idx].sample_id.clone()),
                squared_distance: Some(squared_distance),
            },
        ));
    }
}

fn append_heldout_centroid_prediction_records<F>(
    records: &mut Vec<OfflineVldaHeldoutPredictionRecord>,
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    variable: &str,
    values: F,
) where
    F: Fn(&OfflineVldaSample) -> Vec<f64>,
{
    let Some(model) = train_standardized_centroid_model(samples, labels, roles, values) else {
        return;
    };
    for idx in heldout_indices(roles) {
        let false_distance = squared_euclidean(&model.features[idx], &model.centroids[0]);
        let true_distance = squared_euclidean(&model.features[idx], &model.centroids[1]);
        let score = false_distance - true_distance;
        records.push(heldout_prediction_record(
            samples,
            labels,
            idx,
            OfflineVldaHeldoutPredictionInput {
                classifier: "train_split_nearest_centroid",
                variable: Some(variable),
                predicted_success: score > 0.0,
                score: Some(score),
                score_name: Some(OFFLINE_CENTROID_SUCCESS_SCORE.to_string()),
                nearest_train_sample_id: None,
                squared_distance: None,
            },
        ));
    }
}

fn heldout_prediction_record(
    samples: &[OfflineVldaSample],
    labels: &[bool],
    idx: usize,
    input: OfflineVldaHeldoutPredictionInput<'_>,
) -> OfflineVldaHeldoutPredictionRecord {
    OfflineVldaHeldoutPredictionRecord {
        sample_id: samples[idx].sample_id.clone(),
        episode_id: samples[idx].episode_id.clone(),
        split_value: samples[idx]
            .metadata
            .get(OFFLINE_HELDOUT_SPLIT_METADATA_KEY)
            .map(|value| normalize_split_value(value))
            .unwrap_or_default(),
        classifier: input.classifier.to_string(),
        variable: input.variable.map(str::to_string),
        true_success: labels[idx],
        predicted_success: input.predicted_success,
        correct: input.predicted_success == labels[idx],
        score: input.score,
        score_name: input.score_name,
        nearest_train_sample_id: input.nearest_train_sample_id,
        squared_distance: input.squared_distance,
    }
}

fn heldout_indices(roles: &[OfflineVldaSplitRole]) -> impl Iterator<Item = usize> + '_ {
    roles
        .iter()
        .enumerate()
        .filter_map(|(idx, role)| (*role == OfflineVldaSplitRole::Heldout).then_some(idx))
}

fn vlda_values(sample: &OfflineVldaSample) -> Vec<f64> {
    let mut values =
        Vec::with_capacity(sample.v.len() + sample.l.len() + sample.d.len() + sample.a.len());
    values.extend_from_slice(&sample.v);
    values.extend_from_slice(&sample.l);
    values.extend_from_slice(&sample.d);
    values.extend_from_slice(&sample.a);
    values
}

fn episode_ids(samples: &[OfflineVldaSample]) -> Option<Vec<String>> {
    let episode_ids = samples
        .iter()
        .map(|sample| sample.episode_id.clone())
        .collect::<Option<Vec<_>>>()?;
    (episode_ids.iter().collect::<BTreeSet<_>>().len() >= 2).then_some(episode_ids)
}

fn episode_loo_majority_success_accuracy(labels: &[bool], episode_ids: &[String]) -> f64 {
    let correct = labels
        .iter()
        .enumerate()
        .filter(|(idx, label)| {
            let mut successes = 0;
            let mut total = 0;
            for (candidate_idx, candidate_label) in labels.iter().enumerate() {
                if episode_ids[candidate_idx] == episode_ids[*idx] {
                    continue;
                }
                total += 1;
                if *candidate_label {
                    successes += 1;
                }
            }
            let majority = successes * 2 >= total;
            majority == **label
        })
        .count();
    correct as f64 / labels.len() as f64
}

fn loo_nn_success_accuracy<F>(samples: &[OfflineVldaSample], labels: &[bool], values: F) -> f64
where
    F: Fn(&OfflineVldaSample) -> Vec<f64>,
{
    let features = samples.iter().map(values).collect::<Vec<_>>();
    let correct = features
        .iter()
        .enumerate()
        .filter(|(idx, feature)| {
            let nearest = nearest_neighbor_idx(samples, &features, *idx, feature);
            labels[nearest] == labels[*idx]
        })
        .count();
    correct as f64 / labels.len() as f64
}

fn episode_loo_nn_success_accuracy<F>(
    samples: &[OfflineVldaSample],
    labels: &[bool],
    episode_ids: &[String],
    values: F,
) -> f64
where
    F: Fn(&OfflineVldaSample) -> Vec<f64>,
{
    let features = samples.iter().map(values).collect::<Vec<_>>();
    let correct = features
        .iter()
        .enumerate()
        .filter(|(idx, feature)| {
            let nearest = nearest_neighbor_idx_excluding_episode(
                samples,
                &features,
                *idx,
                feature,
                episode_ids,
            );
            labels[nearest] == labels[*idx]
        })
        .count();
    correct as f64 / labels.len() as f64
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OfflineVldaHeldoutClassifierMetrics {
    accuracy: f64,
    balanced_accuracy: Option<f64>,
    auroc: Option<f64>,
}

struct OfflineVldaCentroidModel {
    features: Vec<Vec<f64>>,
    centroids: [Vec<f64>; 2],
}

fn heldout_majority_success_metrics(
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
) -> OfflineVldaHeldoutClassifierMetrics {
    let mut train_successes = 0;
    let mut train_total = 0;
    for (label, role) in labels.iter().zip(roles) {
        if *role == OfflineVldaSplitRole::Train {
            train_total += 1;
            if *label {
                train_successes += 1;
            }
        }
    }
    let majority = train_successes * 2 >= train_total;
    heldout_prediction_metrics(labels, roles, |_| majority)
}

fn heldout_nn_success_metrics<F>(
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    values: F,
) -> OfflineVldaHeldoutClassifierMetrics
where
    F: Fn(&OfflineVldaSample) -> Vec<f64>,
{
    let features = samples.iter().map(values).collect::<Vec<_>>();
    heldout_prediction_metrics(labels, roles, |idx| {
        let nearest = nearest_neighbor_idx_in_train(samples, &features, &features[idx], roles);
        labels[nearest]
    })
}

fn heldout_centroid_success_metrics<F>(
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    values: F,
) -> Option<OfflineVldaHeldoutClassifierMetrics>
where
    F: Fn(&OfflineVldaSample) -> Vec<f64>,
{
    let model = train_standardized_centroid_model(samples, labels, roles, values)?;
    Some(heldout_scored_prediction_metrics(labels, roles, |idx| {
        let false_distance = squared_euclidean(&model.features[idx], &model.centroids[0]);
        let true_distance = squared_euclidean(&model.features[idx], &model.centroids[1]);
        let score = false_distance - true_distance;
        (score > 0.0, score)
    }))
}

fn train_standardized_centroid_model<F>(
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    values: F,
) -> Option<OfflineVldaCentroidModel>
where
    F: Fn(&OfflineVldaSample) -> Vec<f64>,
{
    let features = samples.iter().map(values).collect::<Vec<_>>();
    let dim = features.first()?.len();
    let train_total = roles
        .iter()
        .filter(|role| **role == OfflineVldaSplitRole::Train)
        .count();
    let mut mean = vec![0.0; dim];
    for (feature, role) in features.iter().zip(roles) {
        if *role == OfflineVldaSplitRole::Train {
            for (sum, value) in mean.iter_mut().zip(feature) {
                *sum += *value;
            }
        }
    }
    for value in &mut mean {
        *value /= train_total as f64;
    }
    let mut variance = vec![0.0; dim];
    for (feature, role) in features.iter().zip(roles) {
        if *role == OfflineVldaSplitRole::Train {
            for ((sum, value), mean) in variance.iter_mut().zip(feature).zip(&mean) {
                let delta = value - mean;
                *sum += delta * delta;
            }
        }
    }
    let inv_std = variance
        .into_iter()
        .map(|sum| {
            if sum == 0.0 {
                1.0
            } else {
                (train_total as f64 / sum).sqrt()
            }
        })
        .collect::<Vec<_>>();
    let features = features
        .iter()
        .map(|feature| {
            feature
                .iter()
                .zip(&mean)
                .zip(&inv_std)
                .map(|((value, mean), inv_std)| (value - mean) * inv_std)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut centroids = [vec![0.0; dim], vec![0.0; dim]];
    let mut counts = [0usize, 0usize];
    for (idx, feature) in features.iter().enumerate() {
        if roles[idx] != OfflineVldaSplitRole::Train {
            continue;
        }
        let class = usize::from(labels[idx]);
        counts[class] += 1;
        for (sum, value) in centroids[class].iter_mut().zip(feature) {
            *sum += *value;
        }
    }
    if counts.contains(&0) {
        return None;
    }
    for (centroid, count) in centroids.iter_mut().zip(counts) {
        for value in centroid {
            *value /= count as f64;
        }
    }
    Some(OfflineVldaCentroidModel {
        features,
        centroids,
    })
}

/// SAFE-class internal-feature failure-detector baseline: fit an L2-regularized
/// logistic regression on the train split (features standardized with train-only
/// statistics) and score the held-out split. Leakage-safe: both the standardizer
/// and the classifier see train rows only. Returns `None` if either class is
/// absent from the train split or the fit fails.
fn heldout_logreg_success_metrics<F>(
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    values: F,
) -> Option<OfflineVldaHeldoutClassifierMetrics>
where
    F: Fn(&OfflineVldaSample) -> Vec<f64>,
{
    // Reuse the train-only standardization machinery (and its both-classes guard)
    // from the centroid baseline; we only need its standardized `features`.
    let model = train_standardized_centroid_model(samples, labels, roles, values)?;
    let dim = model.features.first()?.len();
    if dim == 0 {
        return None;
    }

    // Assemble the train design matrix + labels (standardized features, train rows).
    let mut train_rows = Vec::new();
    let mut train_labels = Vec::new();
    for (idx, role) in roles.iter().enumerate() {
        if *role == OfflineVldaSplitRole::Train {
            train_rows.extend_from_slice(&model.features[idx]);
            train_labels.push(labels[idx]);
        }
    }
    let n_train = train_labels.len();
    if n_train == 0 {
        return None;
    }
    let x_train = MatOwned::new(train_rows, n_train, dim).ok()?;
    // The SAFE-class logistic baseline is the H1 comparison point — if the fit
    // fails, the metric must not vanish without a trace.
    let logreg = match LogisticRegression::fit(
        x_train.as_ref(),
        &train_labels,
        &LogisticRegressionConfig::default(),
    ) {
        Ok(model) => model,
        Err(err) => {
            eprintln!(
                "[pid-offline-harness] heldout_logreg_vlda baseline dropped: \
                 logistic fit failed: {err}"
            );
            return None;
        }
    };

    Some(heldout_scored_prediction_metrics(labels, roles, |idx| {
        // Decision-function logit on the (train-standardized) held-out features.
        let logit = logreg.intercept()
            + model.features[idx]
                .iter()
                .zip(logreg.weights())
                .map(|(a, b)| a * b)
                .sum::<f64>();
        (logit >= 0.0, logit)
    }))
}

fn heldout_prediction_metrics<F>(
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    mut predict: F,
) -> OfflineVldaHeldoutClassifierMetrics
where
    F: FnMut(usize) -> bool,
{
    let mut correct = 0;
    let mut total = 0;
    let mut class_correct = [0usize; 2];
    let mut class_total = [0usize; 2];
    for (idx, label) in labels.iter().enumerate() {
        if roles[idx] != OfflineVldaSplitRole::Heldout {
            continue;
        }
        let class = usize::from(*label);
        let prediction = predict(idx);
        total += 1;
        class_total[class] += 1;
        if prediction == *label {
            correct += 1;
            class_correct[class] += 1;
        }
    }
    let balanced_accuracy = (class_total[0] > 0 && class_total[1] > 0).then_some(
        (class_correct[0] as f64 / class_total[0] as f64
            + class_correct[1] as f64 / class_total[1] as f64)
            / 2.0,
    );
    OfflineVldaHeldoutClassifierMetrics {
        accuracy: correct as f64 / total as f64,
        balanced_accuracy,
        auroc: None,
    }
}

fn heldout_scored_prediction_metrics<F>(
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    mut predict: F,
) -> OfflineVldaHeldoutClassifierMetrics
where
    F: FnMut(usize) -> (bool, f64),
{
    let mut correct = 0;
    let mut total = 0;
    let mut class_correct = [0usize; 2];
    let mut class_total = [0usize; 2];
    let mut scores = Vec::new();
    for (idx, label) in labels.iter().enumerate() {
        if roles[idx] != OfflineVldaSplitRole::Heldout {
            continue;
        }
        let class = usize::from(*label);
        let (prediction, score) = predict(idx);
        total += 1;
        class_total[class] += 1;
        scores.push((score, *label));
        if prediction == *label {
            correct += 1;
            class_correct[class] += 1;
        }
    }
    let balanced_accuracy = (class_total[0] > 0 && class_total[1] > 0).then_some(
        (class_correct[0] as f64 / class_total[0] as f64
            + class_correct[1] as f64 / class_total[1] as f64)
            / 2.0,
    );
    OfflineVldaHeldoutClassifierMetrics {
        accuracy: correct as f64 / total as f64,
        balanced_accuracy,
        auroc: heldout_auroc(&scores),
    }
}

fn heldout_auroc(scores: &[(f64, bool)]) -> Option<f64> {
    let positives = scores
        .iter()
        .filter_map(|(score, label)| (*label).then_some(*score))
        .collect::<Vec<_>>();
    let negatives = scores
        .iter()
        .filter_map(|(score, label)| (!*label).then_some(*score))
        .collect::<Vec<_>>();
    if positives.is_empty() || negatives.is_empty() {
        return None;
    }
    let mut wins = 0.0;
    for positive in &positives {
        for negative in &negatives {
            match positive.total_cmp(negative) {
                Ordering::Greater => wins += 1.0,
                Ordering::Equal => wins += 0.5,
                Ordering::Less => {}
            }
        }
    }
    Some(wins / (positives.len() * negatives.len()) as f64)
}

fn nearest_neighbor_idx(
    samples: &[OfflineVldaSample],
    features: &[Vec<f64>],
    idx: usize,
    feature: &[f64],
) -> usize {
    let mut best_idx: Option<usize> = None;
    let mut best_distance = f64::INFINITY;
    for (candidate_idx, candidate) in features.iter().enumerate() {
        if candidate_idx == idx {
            continue;
        }
        let distance = squared_euclidean(feature, candidate);
        let replace = match best_idx {
            None => true,
            Some(current_idx) => match distance.total_cmp(&best_distance) {
                Ordering::Less => true,
                Ordering::Equal => {
                    samples[candidate_idx].sample_id.as_str()
                        < samples[current_idx].sample_id.as_str()
                }
                Ordering::Greater => false,
            },
        };
        if replace {
            best_idx = Some(candidate_idx);
            best_distance = distance;
        }
    }
    best_idx.expect("validated dataset has at least two samples")
}

fn nearest_neighbor_idx_in_train(
    samples: &[OfflineVldaSample],
    features: &[Vec<f64>],
    feature: &[f64],
    roles: &[OfflineVldaSplitRole],
) -> usize {
    nearest_neighbor_in_train(samples, features, feature, roles).0
}

fn nearest_neighbor_in_train(
    samples: &[OfflineVldaSample],
    features: &[Vec<f64>],
    feature: &[f64],
    roles: &[OfflineVldaSplitRole],
) -> (usize, f64) {
    let mut best_idx: Option<usize> = None;
    let mut best_distance = f64::INFINITY;
    for (candidate_idx, candidate) in features.iter().enumerate() {
        if roles[candidate_idx] != OfflineVldaSplitRole::Train {
            continue;
        }
        let distance = squared_euclidean(feature, candidate);
        let replace = match best_idx {
            None => true,
            Some(current_idx) => match distance.total_cmp(&best_distance) {
                Ordering::Less => true,
                Ordering::Equal => {
                    samples[candidate_idx].sample_id.as_str()
                        < samples[current_idx].sample_id.as_str()
                }
                Ordering::Greater => false,
            },
        };
        if replace {
            best_idx = Some(candidate_idx);
            best_distance = distance;
        }
    }
    (
        best_idx.expect("validated held-out split has at least one train sample"),
        best_distance,
    )
}

fn nearest_neighbor_idx_excluding_episode(
    samples: &[OfflineVldaSample],
    features: &[Vec<f64>],
    idx: usize,
    feature: &[f64],
    episode_ids: &[String],
) -> usize {
    let mut best_idx: Option<usize> = None;
    let mut best_distance = f64::INFINITY;
    for (candidate_idx, candidate) in features.iter().enumerate() {
        if episode_ids[candidate_idx] == episode_ids[idx] {
            continue;
        }
        let distance = squared_euclidean(feature, candidate);
        let replace = match best_idx {
            None => true,
            Some(current_idx) => match distance.total_cmp(&best_distance) {
                Ordering::Less => true,
                Ordering::Equal => {
                    samples[candidate_idx].sample_id.as_str()
                        < samples[current_idx].sample_id.as_str()
                }
                Ordering::Greater => false,
            },
        };
        if replace {
            best_idx = Some(candidate_idx);
            best_distance = distance;
        }
    }
    best_idx.expect("validated episode ids include at least two episodes")
}

fn squared_euclidean(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            let delta = left - right;
            delta * delta
        })
        .sum()
}

/// Writes every metric event at `timestamp_base_ns + i` and returns the number
/// of events written, so the caller can continue the timeline from there.
fn write_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    timestamp_base_ns: u64,
) -> Result<u64> {
    let metrics = [
        ("offline_vlda.pid.mi_v_action", report.metrics.mi_v_action),
        ("offline_vlda.pid.mi_l_action", report.metrics.mi_l_action),
        ("offline_vlda.pid.mi_d_action", report.metrics.mi_d_action),
        ("offline_vlda.pid.mi_vl_action", report.metrics.mi_vl_action),
        (
            "offline_vlda.pid.co_information_v_l_action",
            report.metrics.co_information_v_l_action,
        ),
        (
            "offline_vlda.pid.redundancy_v_l_action",
            report.metrics.redundancy_v_l_action,
        ),
        (
            "offline_vlda.pid.unique_v_action",
            report.metrics.unique_v_action,
        ),
        (
            "offline_vlda.pid.unique_l_action",
            report.metrics.unique_l_action,
        ),
        (
            "offline_vlda.pid.synergy_v_l_action",
            report.metrics.synergy_v_l_action,
        ),
    ];
    for (idx, (name, value)) in metrics.into_iter().enumerate() {
        writer.append(&RunLogEvent::PidMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + idx as u64,
            name: name.to_string(),
            value,
            metadata: offline_vlda_pid_metric_metadata(report, name, None),
        })?;
    }
    let mut idx = metrics.len() as u64;
    for pair in ["VD", "LD"] {
        if let Some(pair_metrics) = report.metrics.pid_pairs.get(pair) {
            write_pid_pair_metric_events(
                writer,
                report,
                pair,
                pair_metrics,
                OfflineVldaPidMetricEventScope {
                    prefix: "offline_vlda.pid",
                    train_pid: None,
                },
                timestamp_base_ns,
                &mut idx,
            )?;
        }
    }
    write_train_split_pid_metric_events(writer, report, timestamp_base_ns, &mut idx)?;
    write_geometry_metric_events(writer, report, timestamp_base_ns, &mut idx)?;
    if let Some(value) = report.metrics.success_rate {
        writer.append(&RunLogEvent::EvaluationMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + idx,
            name: "offline_vlda.labels.success_rate".to_string(),
            value,
            metadata: [("category".to_string(), "label".to_string())]
                .into_iter()
                .collect(),
        })?;
        idx += 1;
    }
    if let Some(value) = report.metrics.majority_success_accuracy {
        writer.append(&RunLogEvent::EvaluationMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + idx,
            name: "offline_vlda.baseline.majority_success_accuracy".to_string(),
            value,
            metadata: [("category".to_string(), "baseline".to_string())]
                .into_iter()
                .collect(),
        })?;
        idx += 1;
    }
    for (name, value) in [
        (
            "offline_vlda.baseline.loo_nn_v_success_accuracy",
            report.metrics.loo_nn_v_success_accuracy,
        ),
        (
            "offline_vlda.baseline.loo_nn_l_success_accuracy",
            report.metrics.loo_nn_l_success_accuracy,
        ),
        (
            "offline_vlda.baseline.loo_nn_d_success_accuracy",
            report.metrics.loo_nn_d_success_accuracy,
        ),
        (
            "offline_vlda.baseline.loo_nn_a_success_accuracy",
            report.metrics.loo_nn_a_success_accuracy,
        ),
        (
            "offline_vlda.baseline.loo_nn_vlda_success_accuracy",
            report.metrics.loo_nn_vlda_success_accuracy,
        ),
    ] {
        if let Some(value) = value {
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: name.to_string(),
                value,
                metadata: [
                    ("category".to_string(), "baseline".to_string()),
                    ("classifier".to_string(), "leave_one_out_1nn".to_string()),
                    ("distance".to_string(), "raw_euclidean".to_string()),
                ]
                .into_iter()
                .collect(),
            })?;
            idx += 1;
        }
    }
    if let Some(value) = report.metrics.episode_loo_majority_success_accuracy {
        writer.append(&RunLogEvent::EvaluationMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + idx,
            name: "offline_vlda.baseline.episode_loo_majority_success_accuracy".to_string(),
            value,
            metadata: [
                ("category".to_string(), "baseline".to_string()),
                (
                    "classifier".to_string(),
                    "leave_one_episode_out_majority".to_string(),
                ),
                ("split".to_string(), "leave_one_episode_out".to_string()),
                ("group_key".to_string(), "episode_id".to_string()),
            ]
            .into_iter()
            .collect(),
        })?;
        idx += 1;
    }
    for (name, value) in [
        (
            "offline_vlda.baseline.episode_loo_nn_v_success_accuracy",
            report.metrics.episode_loo_nn_v_success_accuracy,
        ),
        (
            "offline_vlda.baseline.episode_loo_nn_l_success_accuracy",
            report.metrics.episode_loo_nn_l_success_accuracy,
        ),
        (
            "offline_vlda.baseline.episode_loo_nn_d_success_accuracy",
            report.metrics.episode_loo_nn_d_success_accuracy,
        ),
        (
            "offline_vlda.baseline.episode_loo_nn_a_success_accuracy",
            report.metrics.episode_loo_nn_a_success_accuracy,
        ),
        (
            "offline_vlda.baseline.episode_loo_nn_vlda_success_accuracy",
            report.metrics.episode_loo_nn_vlda_success_accuracy,
        ),
    ] {
        if let Some(value) = value {
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: name.to_string(),
                value,
                metadata: [
                    ("category".to_string(), "baseline".to_string()),
                    (
                        "classifier".to_string(),
                        "leave_one_episode_out_1nn".to_string(),
                    ),
                    ("distance".to_string(), "raw_euclidean".to_string()),
                    ("split".to_string(), "leave_one_episode_out".to_string()),
                    ("group_key".to_string(), "episode_id".to_string()),
                ]
                .into_iter()
                .collect(),
            })?;
            idx += 1;
        }
    }
    if let Some(value) = report.metrics.heldout_majority_success_accuracy {
        writer.append(&RunLogEvent::EvaluationMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + idx,
            name: "offline_vlda.baseline.heldout_majority_success_accuracy".to_string(),
            value,
            metadata: offline_vlda_heldout_split_metric_metadata(
                report,
                "train_split_majority",
                None,
                "accuracy",
            ),
        })?;
        idx += 1;
    }
    if let Some(value) = report.metrics.heldout_majority_success_balanced_accuracy {
        writer.append(&RunLogEvent::EvaluationMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + idx,
            name: "offline_vlda.baseline.heldout_majority_success_balanced_accuracy".to_string(),
            value,
            metadata: offline_vlda_heldout_split_metric_metadata(
                report,
                "train_split_majority",
                None,
                "balanced_accuracy",
            ),
        })?;
        idx += 1;
    }
    for (name, value) in [
        (
            "offline_vlda.baseline.heldout_nn_v_success_accuracy",
            report.metrics.heldout_nn_v_success_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_nn_l_success_accuracy",
            report.metrics.heldout_nn_l_success_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_nn_d_success_accuracy",
            report.metrics.heldout_nn_d_success_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_nn_a_success_accuracy",
            report.metrics.heldout_nn_a_success_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_nn_vlda_success_accuracy",
            report.metrics.heldout_nn_vlda_success_accuracy,
        ),
    ] {
        if let Some(value) = value {
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: name.to_string(),
                value,
                metadata: offline_vlda_heldout_split_metric_metadata(
                    report,
                    "train_split_1nn",
                    Some("raw_euclidean"),
                    "accuracy",
                ),
            })?;
            idx += 1;
        }
    }
    for (name, value) in [
        (
            "offline_vlda.baseline.heldout_nn_v_success_balanced_accuracy",
            report.metrics.heldout_nn_v_success_balanced_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_nn_l_success_balanced_accuracy",
            report.metrics.heldout_nn_l_success_balanced_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_nn_d_success_balanced_accuracy",
            report.metrics.heldout_nn_d_success_balanced_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_nn_a_success_balanced_accuracy",
            report.metrics.heldout_nn_a_success_balanced_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_nn_vlda_success_balanced_accuracy",
            report.metrics.heldout_nn_vlda_success_balanced_accuracy,
        ),
    ] {
        if let Some(value) = value {
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: name.to_string(),
                value,
                metadata: offline_vlda_heldout_split_metric_metadata(
                    report,
                    "train_split_1nn",
                    Some("raw_euclidean"),
                    "balanced_accuracy",
                ),
            })?;
            idx += 1;
        }
    }
    for (name, value) in [
        (
            "offline_vlda.baseline.heldout_centroid_v_success_accuracy",
            report.metrics.heldout_centroid_v_success_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_l_success_accuracy",
            report.metrics.heldout_centroid_l_success_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_d_success_accuracy",
            report.metrics.heldout_centroid_d_success_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_a_success_accuracy",
            report.metrics.heldout_centroid_a_success_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_vlda_success_accuracy",
            report.metrics.heldout_centroid_vlda_success_accuracy,
        ),
    ] {
        if let Some(value) = value {
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: name.to_string(),
                value,
                metadata: offline_vlda_heldout_split_metric_metadata(
                    report,
                    "train_split_nearest_centroid",
                    Some("train_standardized_euclidean"),
                    "accuracy",
                ),
            })?;
            idx += 1;
        }
    }
    for (name, value) in [
        (
            "offline_vlda.baseline.heldout_centroid_v_success_balanced_accuracy",
            report.metrics.heldout_centroid_v_success_balanced_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_l_success_balanced_accuracy",
            report.metrics.heldout_centroid_l_success_balanced_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_d_success_balanced_accuracy",
            report.metrics.heldout_centroid_d_success_balanced_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_a_success_balanced_accuracy",
            report.metrics.heldout_centroid_a_success_balanced_accuracy,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_vlda_success_balanced_accuracy",
            report
                .metrics
                .heldout_centroid_vlda_success_balanced_accuracy,
        ),
    ] {
        if let Some(value) = value {
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: name.to_string(),
                value,
                metadata: offline_vlda_heldout_split_metric_metadata(
                    report,
                    "train_split_nearest_centroid",
                    Some("train_standardized_euclidean"),
                    "balanced_accuracy",
                ),
            })?;
            idx += 1;
        }
    }
    for (name, value) in [
        (
            "offline_vlda.baseline.heldout_centroid_v_success_auroc",
            report.metrics.heldout_centroid_v_success_auroc,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_l_success_auroc",
            report.metrics.heldout_centroid_l_success_auroc,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_d_success_auroc",
            report.metrics.heldout_centroid_d_success_auroc,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_a_success_auroc",
            report.metrics.heldout_centroid_a_success_auroc,
        ),
        (
            "offline_vlda.baseline.heldout_centroid_vlda_success_auroc",
            report.metrics.heldout_centroid_vlda_success_auroc,
        ),
    ] {
        if let Some(value) = value {
            let mut metadata = offline_vlda_heldout_split_metric_metadata(
                report,
                "train_split_nearest_centroid",
                Some("train_standardized_euclidean"),
                "auroc",
            );
            metadata.insert(
                "score".to_string(),
                OFFLINE_CENTROID_SUCCESS_SCORE.to_string(),
            );
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: name.to_string(),
                value,
                metadata,
            })?;
            idx += 1;
        }
    }
    // SAFE-class internal-feature failure detector (logistic regression on pooled
    // train-standardized VLDA features). One event per metric that was produced.
    for (name, value, metric) in [
        (
            "offline_vlda.baseline.heldout_logreg_vlda_success_accuracy",
            report.metrics.heldout_logreg_vlda_success_accuracy,
            "accuracy",
        ),
        (
            "offline_vlda.baseline.heldout_logreg_vlda_success_balanced_accuracy",
            report.metrics.heldout_logreg_vlda_success_balanced_accuracy,
            "balanced_accuracy",
        ),
        (
            "offline_vlda.baseline.heldout_logreg_vlda_success_auroc",
            report.metrics.heldout_logreg_vlda_success_auroc,
            "auroc",
        ),
    ] {
        if let Some(value) = value {
            let mut metadata = offline_vlda_heldout_split_metric_metadata(
                report,
                "train_split_logreg",
                Some("train_standardized_l2_logistic"),
                metric,
            );
            metadata.insert("score".to_string(), "decision_function_logit".to_string());
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: name.to_string(),
                value,
                metadata,
            })?;
            idx += 1;
        }
    }
    write_heldout_failure_diagnostic_metric_events(writer, report, timestamp_base_ns, &mut idx)?;
    write_heldout_prediction_metric_events(writer, report, timestamp_base_ns, &mut idx)?;
    write_heldout_class_coverage_metric_events(writer, report, timestamp_base_ns, &mut idx)?;
    write_heldout_episode_disjoint_metric_events(writer, report, timestamp_base_ns, &mut idx)?;
    for (label, count) in &report.label_counts {
        writer.append(&RunLogEvent::EvaluationMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + idx,
            name: format!("offline_vlda.labels.{label}.count"),
            value: *count as f64,
            metadata: [("category".to_string(), "label".to_string())]
                .into_iter()
                .collect(),
        })?;
        idx += 1;
    }
    Ok(idx)
}

fn write_heldout_class_coverage_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    timestamp_base_ns: u64,
    idx: &mut u64,
) -> Result<()> {
    let Some(coverage) = &report.heldout_class_coverage else {
        return Ok(());
    };
    for (suffix, value) in [
        ("train_success_count", coverage.train_successes as f64),
        ("train_failure_count", coverage.train_failures as f64),
        ("heldout_success_count", coverage.heldout_successes as f64),
        ("heldout_failure_count", coverage.heldout_failures as f64),
        ("pass", if coverage.status == "pass" { 1.0 } else { 0.0 }),
    ] {
        writer.append(&RunLogEvent::EvaluationMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + *idx,
            name: format!("offline_vlda.heldout_split.class_coverage_{suffix}"),
            value,
            metadata: offline_vlda_heldout_class_coverage_metric_metadata(report, suffix),
        })?;
        *idx += 1;
    }
    Ok(())
}

fn write_heldout_episode_disjoint_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    timestamp_base_ns: u64,
    idx: &mut u64,
) -> Result<()> {
    let Some(disjoint) = &report.heldout_episode_disjoint else {
        return Ok(());
    };
    for (suffix, value) in [
        ("train_episode_count", disjoint.train_episodes as f64),
        ("heldout_episode_count", disjoint.heldout_episodes as f64),
        ("shared_episode_count", disjoint.shared_episodes as f64),
        (
            "missing_episode_sample_count",
            disjoint.missing_episode_samples as f64,
        ),
        ("pass", if disjoint.status == "pass" { 1.0 } else { 0.0 }),
    ] {
        writer.append(&RunLogEvent::EvaluationMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + *idx,
            name: format!("offline_vlda.heldout_split.episode_disjoint_{suffix}"),
            value,
            metadata: offline_vlda_heldout_episode_disjoint_metric_metadata(report, suffix),
        })?;
        *idx += 1;
    }
    Ok(())
}

fn write_heldout_prediction_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    timestamp_base_ns: u64,
    idx: &mut u64,
) -> Result<()> {
    for record in &report.heldout_predictions {
        writer.append(&RunLogEvent::EvaluationMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + *idx,
            name: "offline_vlda.heldout_prediction.correct".to_string(),
            value: if record.correct { 1.0 } else { 0.0 },
            metadata: offline_vlda_heldout_prediction_metric_metadata(report, record, "correct"),
        })?;
        *idx += 1;
        if let Some(value) = record.score {
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + *idx,
                name: "offline_vlda.heldout_prediction.score".to_string(),
                value,
                metadata: offline_vlda_heldout_prediction_metric_metadata(report, record, "score"),
            })?;
            *idx += 1;
        }
        if let Some(value) = record.squared_distance {
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + *idx,
                name: "offline_vlda.heldout_prediction.squared_distance".to_string(),
                value,
                metadata: offline_vlda_heldout_prediction_metric_metadata(
                    report,
                    record,
                    "squared_distance",
                ),
            })?;
            *idx += 1;
        }
    }
    Ok(())
}

fn offline_vlda_pid_metric_metadata(
    report: &OfflineVldaReport,
    name: &str,
    train_pid: Option<&OfflineVldaTrainSplitPidReport>,
) -> BTreeMap<String, String> {
    let mut metadata = offline_vlda_pid_scope_metadata(report, train_pid);
    let metric = name
        .strip_prefix("offline_vlda.pid.train_split.")
        .or_else(|| name.strip_prefix("offline_vlda.pid."))
        .unwrap_or(name);
    match metric {
        "mi_v_action" => {
            metadata.insert("source".to_string(), "V".to_string());
            metadata.insert("target".to_string(), "A".to_string());
        }
        "mi_l_action" => {
            metadata.insert("source".to_string(), "L".to_string());
            metadata.insert("target".to_string(), "A".to_string());
        }
        "mi_d_action" => {
            metadata.insert("source".to_string(), "D".to_string());
            metadata.insert("target".to_string(), "A".to_string());
        }
        "mi_vl_action"
        | "co_information_v_l_action"
        | "redundancy_v_l_action"
        | "unique_v_action"
        | "unique_l_action"
        | "synergy_v_l_action" => {
            metadata.insert("pid_pair".to_string(), "VL".to_string());
            metadata.insert("source_1".to_string(), "V".to_string());
            metadata.insert("source_2".to_string(), "L".to_string());
            metadata.insert("target".to_string(), "A".to_string());
        }
        _ => {}
    }
    metadata
}

fn offline_vlda_pid_pair_metric_metadata(
    report: &OfflineVldaReport,
    pair: &str,
    metrics: &OfflineVldaPidPairMetrics,
    train_pid: Option<&OfflineVldaTrainSplitPidReport>,
) -> BTreeMap<String, String> {
    let mut metadata = offline_vlda_pid_scope_metadata(report, train_pid);
    metadata.insert("pid_pair".to_string(), pair.to_string());
    metadata.insert("source_1".to_string(), metrics.source_1.clone());
    metadata.insert("source_2".to_string(), metrics.source_2.clone());
    metadata.insert("target".to_string(), metrics.target.clone());
    metadata
}

fn offline_vlda_pid_scope_metadata(
    report: &OfflineVldaReport,
    train_pid: Option<&OfflineVldaTrainSplitPidReport>,
) -> BTreeMap<String, String> {
    let mut metadata = [
        ("category".to_string(), "pid".to_string()),
        (
            "preprocessing".to_string(),
            "per_variable_standardized".to_string(),
        ),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    if let Some(train_pid) = train_pid {
        metadata.insert(
            "sample_scope".to_string(),
            "metadata_split_train".to_string(),
        );
        metadata.insert("split".to_string(), train_pid.split.clone());
        metadata.insert(
            "split_key".to_string(),
            train_pid.split_metadata_key.clone(),
        );
        metadata.insert("samples".to_string(), train_pid.samples.to_string());
        metadata.insert("train_samples".to_string(), train_pid.samples.to_string());
        metadata.insert(
            "heldout_samples_excluded".to_string(),
            train_pid.heldout_samples_excluded.to_string(),
        );
        metadata.insert(
            "preprocessing_fit_scope".to_string(),
            "metadata_split_train".to_string(),
        );
        metadata.insert("status".to_string(), train_pid.status.clone());
    } else {
        metadata.insert("sample_scope".to_string(), "all_samples".to_string());
        metadata.insert("samples".to_string(), report.dims.samples.to_string());
        metadata.insert(
            "preprocessing_fit_scope".to_string(),
            "all_samples".to_string(),
        );
        if let Some(split) = &report.heldout_split {
            metadata.insert("split_key".to_string(), split.metadata_key.clone());
            metadata.insert("train_samples".to_string(), split.train_samples.to_string());
            metadata.insert(
                "heldout_samples_included".to_string(),
                split.heldout_samples.to_string(),
            );
        }
    }
    metadata
}

fn write_heldout_failure_diagnostic_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    timestamp_base_ns: u64,
    idx: &mut u64,
) -> Result<()> {
    for diagnostic in &report.heldout_failure_diagnostics {
        let Some(prefix) = heldout_failure_metric_prefix(diagnostic) else {
            continue;
        };
        for (suffix, metric, value) in [
            (
                "true_positive_count",
                "failure_true_positive_count",
                diagnostic.failure_true_positives,
            ),
            (
                "false_positive_count",
                "failure_false_positive_count",
                diagnostic.failure_false_positives,
            ),
            (
                "true_negative_count",
                "failure_true_negative_count",
                diagnostic.failure_true_negatives,
            ),
            (
                "false_negative_count",
                "failure_false_negative_count",
                diagnostic.failure_false_negatives,
            ),
        ] {
            writer.append(&RunLogEvent::EvaluationMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + *idx,
                name: format!("{prefix}_{suffix}"),
                value: value as f64,
                metadata: offline_vlda_heldout_failure_metric_metadata(report, diagnostic, metric),
            })?;
            *idx += 1;
        }
        for (suffix, metric, value) in [
            (
                "precision",
                "failure_precision",
                diagnostic.failure_precision,
            ),
            ("recall", "failure_recall", diagnostic.failure_recall),
            (
                "specificity",
                "failure_specificity",
                diagnostic.failure_specificity,
            ),
            ("f1", "failure_f1", diagnostic.failure_f1),
        ] {
            if let Some(value) = value {
                writer.append(&RunLogEvent::EvaluationMetric {
                    step: report.dims.samples as u64,
                    timestamp_ns: timestamp_base_ns + *idx,
                    name: format!("{prefix}_{suffix}"),
                    value,
                    metadata: offline_vlda_heldout_failure_metric_metadata(
                        report, diagnostic, metric,
                    ),
                })?;
                *idx += 1;
            }
        }
    }
    Ok(())
}

fn heldout_failure_metric_prefix(
    diagnostic: &OfflineVldaHeldoutFailureDiagnostics,
) -> Option<String> {
    match diagnostic.classifier.as_str() {
        "train_split_majority" => {
            Some("offline_vlda.baseline.heldout_majority_failure".to_string())
        }
        "train_split_1nn" => diagnostic.variable.as_ref().map(|variable| {
            format!(
                "offline_vlda.baseline.heldout_nn_{}_failure",
                variable.to_ascii_lowercase()
            )
        }),
        "train_split_nearest_centroid" => diagnostic.variable.as_ref().map(|variable| {
            format!(
                "offline_vlda.baseline.heldout_centroid_{}_failure",
                variable.to_ascii_lowercase()
            )
        }),
        _ => None,
    }
}

fn offline_vlda_heldout_failure_metric_metadata(
    report: &OfflineVldaReport,
    diagnostic: &OfflineVldaHeldoutFailureDiagnostics,
    metric: &str,
) -> BTreeMap<String, String> {
    let distance = match diagnostic.classifier.as_str() {
        "train_split_1nn" => Some("raw_euclidean"),
        "train_split_nearest_centroid" => Some("train_standardized_euclidean"),
        _ => None,
    };
    let mut metadata = offline_vlda_heldout_split_metric_metadata(
        report,
        &diagnostic.classifier,
        distance,
        metric,
    );
    metadata.insert("target_class".to_string(), "failure".to_string());
    metadata.insert("positive_label".to_string(), "success_false".to_string());
    metadata.insert(
        "heldout_samples".to_string(),
        diagnostic.samples.to_string(),
    );
    metadata.insert(
        "true_failures".to_string(),
        diagnostic.true_failures.to_string(),
    );
    metadata.insert(
        "true_successes".to_string(),
        diagnostic.true_successes.to_string(),
    );
    if let Some(variable) = &diagnostic.variable {
        metadata.insert("variable".to_string(), variable.clone());
    }
    metadata
}

fn offline_vlda_heldout_split_metric_metadata(
    report: &OfflineVldaReport,
    classifier: &str,
    distance: Option<&str>,
    metric: &str,
) -> BTreeMap<String, String> {
    let mut metadata = [
        ("category".to_string(), "baseline".to_string()),
        ("classifier".to_string(), classifier.to_string()),
        ("metric".to_string(), metric.to_string()),
        ("split".to_string(), "metadata_split_heldout".to_string()),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    if let Some(distance) = distance {
        metadata.insert("distance".to_string(), distance.to_string());
    }
    if let Some(split) = &report.heldout_split {
        metadata.insert("split_key".to_string(), split.metadata_key.clone());
        metadata.insert("train_samples".to_string(), split.train_samples.to_string());
        metadata.insert(
            "heldout_samples".to_string(),
            split.heldout_samples.to_string(),
        );
        metadata.insert("train_values".to_string(), split.train_values.join(","));
        metadata.insert("heldout_values".to_string(), split.heldout_values.join(","));
    }
    metadata
}

fn offline_vlda_heldout_class_coverage_metric_metadata(
    report: &OfflineVldaReport,
    metric: &str,
) -> BTreeMap<String, String> {
    let mut metadata = [
        ("category".to_string(), "heldout_split_quality".to_string()),
        ("metric".to_string(), metric.to_string()),
        ("split".to_string(), "metadata_split_heldout".to_string()),
        (
            "class_label".to_string(),
            "offline_vlda.success".to_string(),
        ),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    if let Some(split) = &report.heldout_split {
        metadata.insert("split_key".to_string(), split.metadata_key.clone());
        metadata.insert("train_samples".to_string(), split.train_samples.to_string());
        metadata.insert(
            "heldout_samples".to_string(),
            split.heldout_samples.to_string(),
        );
    }
    if let Some(coverage) = &report.heldout_class_coverage {
        metadata.insert("status".to_string(), coverage.status.clone());
        metadata.insert("warnings".to_string(), coverage.warnings.len().to_string());
    }
    metadata
}

fn offline_vlda_heldout_episode_disjoint_metric_metadata(
    report: &OfflineVldaReport,
    metric: &str,
) -> BTreeMap<String, String> {
    let mut metadata = [
        ("category".to_string(), "heldout_split_quality".to_string()),
        ("metric".to_string(), metric.to_string()),
        ("split".to_string(), "metadata_split_heldout".to_string()),
        ("group_key".to_string(), "episode_id".to_string()),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    if let Some(split) = &report.heldout_split {
        metadata.insert("split_key".to_string(), split.metadata_key.clone());
        metadata.insert("train_samples".to_string(), split.train_samples.to_string());
        metadata.insert(
            "heldout_samples".to_string(),
            split.heldout_samples.to_string(),
        );
    }
    if let Some(disjoint) = &report.heldout_episode_disjoint {
        metadata.insert("status".to_string(), disjoint.status.clone());
        metadata.insert("warnings".to_string(), disjoint.warnings.len().to_string());
        metadata.insert(
            "shared_episodes".to_string(),
            disjoint.shared_episodes.to_string(),
        );
    }
    metadata
}

fn offline_vlda_heldout_prediction_metric_metadata(
    report: &OfflineVldaReport,
    record: &OfflineVldaHeldoutPredictionRecord,
    metric: &str,
) -> BTreeMap<String, String> {
    let mut metadata = [
        ("category".to_string(), "heldout_prediction".to_string()),
        ("metric".to_string(), metric.to_string()),
        ("split".to_string(), "metadata_split_heldout".to_string()),
        ("sample_id".to_string(), record.sample_id.clone()),
        ("split_value".to_string(), record.split_value.clone()),
        ("classifier".to_string(), record.classifier.clone()),
        ("true_success".to_string(), record.true_success.to_string()),
        (
            "predicted_success".to_string(),
            record.predicted_success.to_string(),
        ),
        ("correct".to_string(), record.correct.to_string()),
        (
            "target_label".to_string(),
            "offline_vlda.success".to_string(),
        ),
    ]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    if let Some(variable) = &record.variable {
        metadata.insert("variable".to_string(), variable.clone());
    }
    if let Some(episode_id) = &record.episode_id {
        metadata.insert("episode_id".to_string(), episode_id.clone());
    }
    if let Some(score_name) = &record.score_name {
        metadata.insert("score_name".to_string(), score_name.clone());
    }
    if let Some(nearest_train_sample_id) = &record.nearest_train_sample_id {
        metadata.insert(
            "nearest_train_sample_id".to_string(),
            nearest_train_sample_id.clone(),
        );
    }
    if let Some(split) = &report.heldout_split {
        metadata.insert("split_key".to_string(), split.metadata_key.clone());
        metadata.insert("train_samples".to_string(), split.train_samples.to_string());
        metadata.insert(
            "heldout_samples".to_string(),
            split.heldout_samples.to_string(),
        );
    }
    metadata
}

fn write_train_split_pid_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    timestamp_base_ns: u64,
    idx: &mut u64,
) -> Result<()> {
    let Some(train_pid) = &report.train_split_pid else {
        return Ok(());
    };
    let Some(metrics) = &train_pid.metrics else {
        return Ok(());
    };
    for (name, value) in [
        (
            "offline_vlda.pid.train_split.mi_v_action",
            metrics.mi_v_action,
        ),
        (
            "offline_vlda.pid.train_split.mi_l_action",
            metrics.mi_l_action,
        ),
        (
            "offline_vlda.pid.train_split.mi_d_action",
            metrics.mi_d_action,
        ),
        (
            "offline_vlda.pid.train_split.mi_vl_action",
            metrics.mi_vl_action,
        ),
        (
            "offline_vlda.pid.train_split.co_information_v_l_action",
            metrics.co_information_v_l_action,
        ),
        (
            "offline_vlda.pid.train_split.redundancy_v_l_action",
            metrics.redundancy_v_l_action,
        ),
        (
            "offline_vlda.pid.train_split.unique_v_action",
            metrics.unique_v_action,
        ),
        (
            "offline_vlda.pid.train_split.unique_l_action",
            metrics.unique_l_action,
        ),
        (
            "offline_vlda.pid.train_split.synergy_v_l_action",
            metrics.synergy_v_l_action,
        ),
    ] {
        writer.append(&RunLogEvent::PidMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + *idx,
            name: name.to_string(),
            value,
            metadata: offline_vlda_pid_metric_metadata(report, name, Some(train_pid)),
        })?;
        *idx += 1;
    }
    for pair in ["VD", "LD"] {
        if let Some(pair_metrics) = metrics.pid_pairs.get(pair) {
            write_pid_pair_metric_events(
                writer,
                report,
                pair,
                pair_metrics,
                OfflineVldaPidMetricEventScope {
                    prefix: "offline_vlda.pid.train_split",
                    train_pid: Some(train_pid),
                },
                timestamp_base_ns,
                idx,
            )?;
        }
    }
    Ok(())
}

fn write_pid_pair_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    pair: &str,
    metrics: &OfflineVldaPidPairMetrics,
    scope: OfflineVldaPidMetricEventScope<'_>,
    timestamp_base_ns: u64,
    idx: &mut u64,
) -> Result<()> {
    let source_1 = metrics.source_1.to_ascii_lowercase();
    let source_2 = metrics.source_2.to_ascii_lowercase();
    let pair_name = format!("{source_1}{source_2}");
    for (name, value) in [
        (
            format!("{}.mi_{pair_name}_action", scope.prefix),
            metrics.mi_joint_action,
        ),
        (
            format!(
                "{}.co_information_{source_1}_{source_2}_action",
                scope.prefix
            ),
            metrics.co_information,
        ),
        (
            format!("{}.redundancy_{source_1}_{source_2}_action", scope.prefix),
            metrics.redundancy,
        ),
        (
            format!("{}.unique_{source_1}_given_{source_2}_action", scope.prefix),
            metrics.unique_source_1,
        ),
        (
            format!("{}.unique_{source_2}_given_{source_1}_action", scope.prefix),
            metrics.unique_source_2,
        ),
        (
            format!("{}.synergy_{source_1}_{source_2}_action", scope.prefix),
            metrics.synergy,
        ),
    ] {
        writer.append(&RunLogEvent::PidMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + *idx,
            name,
            value,
            metadata: offline_vlda_pid_pair_metric_metadata(report, pair, metrics, scope.train_pid),
        })?;
        *idx += 1;
    }
    Ok(())
}

fn write_geometry_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    timestamp_base_ns: u64,
    idx: &mut u64,
) -> Result<()> {
    for (variable, geometry) in &report.geometry.variables {
        for (suffix, value) in [
            ("intrinsic_dimension", geometry.intrinsic_dimension),
            ("pairwise_cv", geometry.pairwise_cv),
            ("nn_over_pairwise_mean", geometry.nn_over_pairwise_mean),
            ("gromov_delta_rel", geometry.gromov_delta_rel),
        ] {
            if let Some(value) = value {
                writer.append(&RunLogEvent::GeometryMetric {
                    step: report.dims.samples as u64,
                    timestamp_ns: timestamp_base_ns + *idx,
                    name: format!("offline_vlda.geometry.{variable}.{suffix}"),
                    value,
                    metadata: [
                        ("category".to_string(), "geometry".to_string()),
                        ("variable".to_string(), variable.clone()),
                        ("space".to_string(), report.geometry.space.clone()),
                        ("metric".to_string(), report.geometry.metric.clone()),
                    ]
                    .into_iter()
                    .collect(),
                })?;
                *idx += 1;
            }
        }
    }
    writer.append(&RunLogEvent::GeometryMetric {
        step: report.dims.samples as u64,
        timestamp_ns: timestamp_base_ns + *idx,
        name: "offline_vlda.geometry.gate_pass".to_string(),
        value: if report.geometry.gates.status == "pass" {
            1.0
        } else {
            0.0
        },
        metadata: [
            ("category".to_string(), "geometry_gate".to_string()),
            ("space".to_string(), report.geometry.space.clone()),
            ("metric".to_string(), report.geometry.metric.clone()),
            (
                "warnings".to_string(),
                report.geometry.gates.warnings.len().to_string(),
            ),
        ]
        .into_iter()
        .collect(),
    })?;
    *idx += 1;
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pid_runlog::{read_events_from_path, summarize_events, validate_events};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_dataset() -> OfflineVldaDataset {
        let samples = (0..16)
            .map(|idx| {
                let x = idx as f64 / 15.0;
                let y = if idx % 2 == 0 { 1.0 } else { -1.0 };
                let action = 0.7 * x + 0.3 * y;
                OfflineVldaSample {
                    sample_id: format!("sample-{idx:03}"),
                    episode_id: Some(format!("episode-{:03}", idx / 2)),
                    v: vec![x, x * x],
                    l: vec![y],
                    d: vec![action - x],
                    a: vec![action],
                    labels: [("success".to_string(), json!(idx % 5 != 0))]
                        .into_iter()
                        .collect(),
                    metadata: [(
                        "split".to_string(),
                        if idx < 12 { "train" } else { "test" }.to_string(),
                    )]
                    .into_iter()
                    .collect(),
                }
            })
            .collect();
        OfflineVldaDataset {
            run_id: Some("offline-fixture-run".to_string()),
            source: Some("unit_test".to_string()),
            model: Some("fixture_policy".to_string()),
            task: Some("fixture_task".to_string()),
            samples,
        }
    }

    fn assert_metric_close(actual: Option<f64>, expected: f64) {
        let actual = actual.unwrap();
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
    }

    fn failure_diagnostic<'a>(
        report: &'a OfflineVldaReport,
        classifier: &str,
        variable: Option<&str>,
    ) -> &'a OfflineVldaHeldoutFailureDiagnostics {
        report
            .heldout_failure_diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.classifier == classifier && diagnostic.variable.as_deref() == variable
            })
            .unwrap()
    }

    fn preprocessing_variable(
        input_dim: usize,
        zero_variance_dims: usize,
    ) -> OfflineVldaPreprocessingVariable {
        OfflineVldaPreprocessingVariable {
            input_dim,
            output_dim: input_dim,
            zero_variance_dims,
            mean_sha256: String::new(),
            inv_std_sha256: String::new(),
        }
    }

    #[test]
    fn axis_provenance_flags_fabricated_and_misaligned_axes() {
        // Build samples carrying the provenance markers ncp-observer stamps.
        let sample = |l_src: &str, d_src: &str| OfflineVldaSample {
            sample_id: "s".into(),
            episode_id: None,
            v: vec![0.0],
            l: vec![0.0],
            d: vec![0.0],
            a: vec![0.0],
            labels: BTreeMap::new(),
            metadata: BTreeMap::from([
                ("l_source".to_string(), l_src.to_string()),
                ("d_source".to_string(), d_src.to_string()),
            ]),
        };
        // Two clean, one fabricated-L, one recency-misaligned-D.
        let samples = vec![
            sample("channel", "seq"),
            sample("channel", "seq"),
            sample("absent_zeroed", "seq"),
            sample("channel", "recency_fallback"),
        ];
        let prov = axis_provenance(&samples);
        let l = prov.iter().find(|p| p.axis == "L").expect("L provenance");
        assert_eq!(l.status, "degraded");
        assert_eq!(l.degraded_samples, 1);
        assert_eq!(l.total_samples, 4);
        assert_eq!(l.sources["channel"], 3);
        assert_eq!(l.sources["absent_zeroed"], 1);
        assert!(l.note.as_ref().unwrap().contains("NOT trustworthy"));
        let d = prov.iter().find(|p| p.axis == "D").expect("D provenance");
        assert_eq!(d.status, "degraded");
        assert_eq!(d.degraded_samples, 1);

        // No markers -> no provenance rows (a pure synthetic dataset).
        let clean = vec![OfflineVldaSample {
            sample_id: "s".into(),
            episode_id: None,
            v: vec![0.0],
            l: vec![0.0],
            d: vec![0.0],
            a: vec![0.0],
            labels: BTreeMap::new(),
            metadata: BTreeMap::new(),
        }];
        assert!(axis_provenance(&clean).is_empty());

        // All-clean markers -> status ok, no note.
        let ok = vec![sample("channel", "seq")];
        let p = axis_provenance(&ok);
        assert!(p.iter().all(|x| x.status == "ok" && x.note.is_none()));
    }

    #[test]
    fn axis_provenance_recognizes_safe_adapter_markers() {
        // The safe_adapter stamps `{v,l,d,a}_provenance`; `text_hash_proxy` is a
        // degraded (hash-surrogate) L, `token_slice:*` / `action_vector` are honest.
        let safe = |l_prov: &str| OfflineVldaSample {
            sample_id: "s".into(),
            episode_id: None,
            v: vec![0.0],
            l: vec![0.0],
            d: vec![0.0],
            a: vec![0.0],
            labels: BTreeMap::new(),
            metadata: BTreeMap::from([
                ("v_provenance".to_string(), "token_slice:vision".to_string()),
                ("l_provenance".to_string(), l_prov.to_string()),
                ("d_provenance".to_string(), "hidden_state_pool".to_string()),
                ("a_provenance".to_string(), "action_vector".to_string()),
            ]),
        };
        // Honest language -> all axes ok.
        let prov = axis_provenance(&[safe("token_slice:language")]);
        assert!(prov.iter().any(|p| p.axis == "L" && p.status == "ok"));
        assert!(prov.iter().any(|p| p.axis == "V" && p.status == "ok"));
        // Hash-proxy language -> L flagged degraded; V/D/A still ok.
        let prov = axis_provenance(&[safe("text_hash_proxy"), safe("token_slice:language")]);
        let l = prov
            .iter()
            .find(|p| p.axis == "L" && p.marker == "l_provenance")
            .unwrap();
        assert_eq!(l.status, "degraded");
        assert_eq!(l.degraded_samples, 1);
        assert!(prov.iter().find(|p| p.axis == "V").unwrap().status == "ok");
    }

    #[test]
    fn axis_provenance_gate_fails_on_degraded_and_on_absent_markers() {
        let prov = |axis: &str, status: &str, degraded: usize| OfflineVldaAxisProvenance {
            marker: format!("{}_provenance", axis.to_lowercase()),
            axis: axis.to_string(),
            sources: BTreeMap::new(),
            degraded_samples: degraded,
            total_samples: degraded.max(1),
            status: status.to_string(),
            note: None,
        };
        // All-honest markers present -> the gate passes (no failures).
        let honest = vec![prov("V", "ok", 0), prov("L", "ok", 0)];
        assert!(offline_vlda_axis_provenance_failure_messages(&honest).is_empty());
        // A degraded axis -> one failure naming the axis + the degraded-sample count.
        let degraded = vec![prov("V", "ok", 0), prov("L", "degraded", 3)];
        let msgs = offline_vlda_axis_provenance_failure_messages(&degraded);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("axis L") && msgs[0].contains('3'));
        // No markers at all -> positive-attestation failure (cannot pass vacuously).
        let absent = offline_vlda_axis_provenance_failure_messages(&[]);
        assert_eq!(absent.len(), 1);
        assert!(absent[0].contains("positive attestation"));
    }

    #[test]
    fn geometry_gates_flag_all_constant_variable_as_degenerate() {
        // An all-constant L (every dim zero-variance, e.g. a fabricated all-zero language
        // channel — NCP_DEV_PROMPT Gap 2) must be flagged: zero variance ⇒ zero mutual
        // information by construction, so any PID atom involving it is invalid.
        let mut variables = BTreeMap::new();
        let mut preprocessing = BTreeMap::new();
        preprocessing.insert("V".to_string(), preprocessing_variable(4, 0));
        preprocessing.insert("L".to_string(), preprocessing_variable(8, 8));
        let gates = compute_geometry_gates(
            &variables,
            &OfflineVldaPreprocessingReport {
                strategy: "per_variable_standardized".to_string(),
                variables: preprocessing.clone(),
            },
        );
        assert_eq!(gates.status, "warn");
        let degenerate: Vec<_> = gates
            .warnings
            .iter()
            .filter(|w| w.contains("all-constant"))
            .collect();
        assert_eq!(
            degenerate.len(),
            1,
            "exactly L should be flagged: {:?}",
            gates.warnings
        );
        assert!(degenerate[0].contains("geometry L is all-constant"));

        // A non-degenerate set (no fully zero-variance variable, no geometry variables)
        // produces no degenerate-axis warning.
        variables.clear();
        let mut healthy = BTreeMap::new();
        healthy.insert("V".to_string(), preprocessing_variable(4, 1));
        healthy.insert("L".to_string(), preprocessing_variable(8, 0));
        let gates = compute_geometry_gates(
            &variables,
            &OfflineVldaPreprocessingReport {
                strategy: "per_variable_standardized".to_string(),
                variables: healthy,
            },
        );
        assert!(
            gates.warnings.iter().all(|w| !w.contains("all-constant")),
            "no variable should be flagged degenerate: {:?}",
            gates.warnings
        );
    }

    #[test]
    fn discrete_mode_emits_imin_pairs_with_saturation_diagnostics() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Discrete,
            discrete_bins: 6,
            pls_components: 2,
        };
        let report = run_offline_vlda_harness_with_options(dataset, None, None, &options).unwrap();
        assert_eq!(report.metrics.pid_pairs.len(), 3);
        for (pair_name, pair) in &report.metrics.pid_pairs {
            let saturation = pair
                .discrete_saturation
                .as_ref()
                .unwrap_or_else(|| panic!("{pair_name} missing saturation diagnostics"));
            assert!(saturation.unique_fraction_joint > 0.0);
            // I_min identities: Red <= min marginal MI, so uniques are non-negative;
            // atoms computed exactly on one empirical joint are non-negative.
            let eps = 1e-9;
            assert!(pair.redundancy <= pair.mi_source_1_action.min(pair.mi_source_2_action) + eps);
            assert!(pair.unique_source_1 >= -eps, "{pair_name} Unq1 negative");
            assert!(pair.unique_source_2 >= -eps, "{pair_name} Unq2 negative");
            assert!(pair.synergy >= -eps, "{pair_name} Syn negative");
        }
    }

    #[test]
    fn discrete_pls_mode_projects_then_quantizes() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::DiscretePls,
            discrete_bins: 6,
            pls_components: 1,
        };
        let report = run_offline_vlda_harness_with_options(dataset, None, None, &options).unwrap();
        assert_eq!(report.metrics.pid_pairs.len(), 3);
        let vl = &report.metrics.pid_pairs["VL"];
        assert!(vl.discrete_saturation.is_some());
        assert!(vl.mi_source_1_action.is_finite());
        // Train-split screen must also run under the PLS-projected discrete path.
        let train_pid = report.train_split_pid.as_ref().unwrap();
        assert_eq!(train_pid.status, "available");
        assert_eq!(train_pid.metrics.as_ref().unwrap().pid_pairs.len(), 3);
    }

    #[test]
    fn continuous_mode_has_no_saturation_diagnostics() {
        let dataset = fixture_dataset();
        let report = run_offline_vlda_harness(dataset, None, None).unwrap();
        for pair in report.metrics.pid_pairs.values() {
            assert!(pair.discrete_saturation.is_none());
        }
    }

    #[test]
    fn offline_vlda_harness_validates_and_summarizes() {
        let dataset = fixture_dataset();
        let report = run_offline_vlda_harness(
            dataset.clone(),
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        assert_eq!(report.dims.samples, 16);
        assert_eq!(report.dims.v, 2);
        assert_eq!(report.metrics.success_rate, Some(0.75));
        assert_eq!(report.metrics.loo_nn_v_success_accuracy, Some(0.5625));
        assert_eq!(report.metrics.loo_nn_l_success_accuracy, Some(0.4375));
        assert_eq!(report.metrics.loo_nn_vlda_success_accuracy, Some(0.5625));
        assert_eq!(
            report.metrics.episode_loo_majority_success_accuracy,
            Some(0.75)
        );
        assert_eq!(
            report.metrics.episode_loo_nn_v_success_accuracy,
            Some(0.625)
        );
        assert_eq!(
            report.metrics.episode_loo_nn_l_success_accuracy,
            Some(0.4375)
        );
        assert_eq!(
            report.metrics.episode_loo_nn_vlda_success_accuracy,
            Some(0.5625)
        );
        let split = report.heldout_split.as_ref().unwrap();
        assert_eq!(split.train_samples, 12);
        assert_eq!(split.heldout_samples, 4);
        assert_eq!(
            split.train_sample_ids.first().map(String::as_str),
            Some("sample-000")
        );
        assert_eq!(
            split.heldout_sample_ids.first().map(String::as_str),
            Some("sample-012")
        );
        let coverage = report.heldout_class_coverage.as_ref().unwrap();
        assert_eq!(coverage.status, "pass");
        assert_eq!(coverage.train_successes, 9);
        assert_eq!(coverage.train_failures, 3);
        assert_eq!(coverage.heldout_successes, 3);
        assert_eq!(coverage.heldout_failures, 1);
        assert!(coverage.warnings.is_empty());
        let episode_disjoint = report.heldout_episode_disjoint.as_ref().unwrap();
        assert_eq!(episode_disjoint.status, "pass");
        assert_eq!(episode_disjoint.train_episodes, 6);
        assert_eq!(episode_disjoint.heldout_episodes, 2);
        assert_eq!(episode_disjoint.shared_episodes, 0);
        assert_eq!(episode_disjoint.missing_episode_samples, 0);
        assert!(episode_disjoint.shared_episode_ids.is_empty());
        assert!(episode_disjoint.warnings.is_empty());
        assert_eq!(report.metrics.heldout_majority_success_accuracy, Some(0.75));
        assert_eq!(
            report.metrics.heldout_majority_success_balanced_accuracy,
            Some(0.5)
        );
        assert_eq!(report.metrics.heldout_nn_v_success_accuracy, Some(0.75));
        assert_eq!(report.metrics.heldout_nn_l_success_accuracy, Some(0.25));
        assert_eq!(report.metrics.heldout_nn_d_success_accuracy, Some(0.25));
        assert_eq!(report.metrics.heldout_nn_a_success_accuracy, Some(0.0));
        assert_eq!(report.metrics.heldout_nn_vlda_success_accuracy, Some(0.25));
        assert_eq!(
            report.metrics.heldout_nn_v_success_balanced_accuracy,
            Some(0.5)
        );
        assert_metric_close(
            report.metrics.heldout_nn_l_success_balanced_accuracy,
            1.0 / 6.0,
        );
        assert_metric_close(
            report.metrics.heldout_nn_d_success_balanced_accuracy,
            1.0 / 6.0,
        );
        assert_eq!(
            report.metrics.heldout_nn_a_success_balanced_accuracy,
            Some(0.0)
        );
        assert_metric_close(
            report.metrics.heldout_nn_vlda_success_balanced_accuracy,
            1.0 / 6.0,
        );
        assert_eq!(
            report.metrics.heldout_centroid_v_success_accuracy,
            Some(0.75)
        );
        assert_eq!(
            report.metrics.heldout_centroid_l_success_accuracy,
            Some(0.25)
        );
        assert_eq!(
            report.metrics.heldout_centroid_d_success_accuracy,
            Some(0.25)
        );
        assert_eq!(
            report.metrics.heldout_centroid_a_success_accuracy,
            Some(0.25)
        );
        assert_eq!(
            report.metrics.heldout_centroid_vlda_success_accuracy,
            Some(0.25)
        );
        assert_eq!(
            report.metrics.heldout_centroid_v_success_balanced_accuracy,
            Some(0.5)
        );
        assert_metric_close(
            report.metrics.heldout_centroid_l_success_balanced_accuracy,
            1.0 / 6.0,
        );
        assert_metric_close(
            report.metrics.heldout_centroid_d_success_balanced_accuracy,
            1.0 / 6.0,
        );
        assert_eq!(
            report.metrics.heldout_centroid_a_success_balanced_accuracy,
            Some(0.5)
        );
        assert_metric_close(
            report
                .metrics
                .heldout_centroid_vlda_success_balanced_accuracy,
            1.0 / 6.0,
        );
        assert_eq!(report.metrics.heldout_centroid_v_success_auroc, Some(0.0));
        assert_metric_close(report.metrics.heldout_centroid_l_success_auroc, 1.0 / 6.0);
        assert_eq!(report.metrics.heldout_centroid_d_success_auroc, Some(0.0));
        assert_metric_close(report.metrics.heldout_centroid_a_success_auroc, 1.0 / 3.0);
        assert_eq!(
            report.metrics.heldout_centroid_vlda_success_auroc,
            Some(0.0)
        );
        // SAFE-class logistic-regression failure detector is produced (leakage-safe:
        // fit on train, scored on held-out) with metrics in valid ranges.
        let lr_acc = report
            .metrics
            .heldout_logreg_vlda_success_accuracy
            .expect("logreg accuracy emitted");
        assert!((0.0..=1.0).contains(&lr_acc));
        let lr_bacc = report
            .metrics
            .heldout_logreg_vlda_success_balanced_accuracy
            .expect("logreg balanced accuracy emitted");
        assert!((0.0..=1.0).contains(&lr_bacc));
        let lr_auroc = report
            .metrics
            .heldout_logreg_vlda_success_auroc
            .expect("logreg auroc emitted");
        assert!((0.0..=1.0).contains(&lr_auroc));
        assert_eq!(report.heldout_predictions.len(), 44);
        let centroid_prediction = report
            .heldout_predictions
            .iter()
            .find(|record| {
                record.sample_id == "sample-012"
                    && record.classifier == "train_split_nearest_centroid"
                    && record.variable.as_deref() == Some("VLDA")
            })
            .unwrap();
        assert_eq!(
            centroid_prediction.score_name.as_deref(),
            Some(OFFLINE_CENTROID_SUCCESS_SCORE)
        );
        assert!(centroid_prediction.score.is_some());
        assert_eq!(
            centroid_prediction.correct,
            centroid_prediction.predicted_success == centroid_prediction.true_success
        );
        let nn_prediction = report
            .heldout_predictions
            .iter()
            .find(|record| {
                record.sample_id == "sample-012"
                    && record.classifier == "train_split_1nn"
                    && record.variable.as_deref() == Some("VLDA")
            })
            .unwrap();
        assert!(nn_prediction.nearest_train_sample_id.is_some());
        assert!(nn_prediction.squared_distance.is_some());
        assert_eq!(report.heldout_failure_diagnostics.len(), 11);
        let majority_failure = failure_diagnostic(&report, "train_split_majority", None);
        assert_eq!(majority_failure.samples, 4);
        assert_eq!(majority_failure.true_failures, 1);
        assert_eq!(majority_failure.true_successes, 3);
        assert_eq!(majority_failure.predicted_failures, 0);
        assert_eq!(majority_failure.failure_true_positives, 0);
        assert_eq!(majority_failure.failure_false_positives, 0);
        assert_eq!(majority_failure.failure_true_negatives, 3);
        assert_eq!(majority_failure.failure_false_negatives, 1);
        assert_eq!(majority_failure.failure_precision, None);
        assert_eq!(majority_failure.failure_recall, Some(0.0));
        assert_eq!(majority_failure.failure_specificity, Some(1.0));
        assert_eq!(majority_failure.failure_f1, Some(0.0));
        let nn_vlda_failure = failure_diagnostic(&report, "train_split_1nn", Some("VLDA"));
        assert_eq!(nn_vlda_failure.samples, 4);
        assert_eq!(nn_vlda_failure.true_failures, 1);
        let train_pid = report.train_split_pid.as_ref().unwrap();
        assert_eq!(train_pid.status, "available");
        assert_eq!(train_pid.split_metadata_key, "split");
        assert_eq!(train_pid.split, "metadata_split_train");
        assert_eq!(train_pid.samples, 12);
        assert_eq!(train_pid.heldout_samples_excluded, 4);
        assert_eq!(
            train_pid.train_sample_ids.first().map(String::as_str),
            Some("sample-000")
        );
        assert_eq!(
            train_pid.preprocessing.as_ref().unwrap().variables["V"].input_dim,
            2
        );
        assert_eq!(train_pid.metrics.as_ref().unwrap().pid_pairs.len(), 3);
        assert_eq!(offline_vlda_train_split_pid_status(&report), "available");
        assert_eq!(report.metrics.pid_pairs.len(), 3);
        assert_eq!(report.metrics.pid_pairs["VD"].source_2, "D");
        assert_eq!(report.label_counts["success"], 16);
        assert_eq!(report.preprocessing.strategy, "per_variable_standardized");
        assert_eq!(report.preprocessing.variables["V"].input_dim, 2);
        assert_eq!(report.preprocessing.variables["V"].zero_variance_dims, 0);
        assert_eq!(report.geometry.metric, "chebyshev");
        assert_eq!(report.geometry.variables["V"].dims, vec![16, 2]);
        assert!(report.geometry.variables["V"].pairwise_cv.is_some());
        assert!(report.geometry.variables["L"]
            .intrinsic_dimension_error
            .is_some());
        assert_eq!(report.geometry.gates.status, "warn");
        assert!(!report.geometry.gates.warnings.is_empty());

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let summary_path = dir.join(format!("pid-offline-vlda-{stamp}.summary.json"));
        let runlog_path = dir.join(format!("pid-offline-vlda-{stamp}.jsonl"));
        write_offline_vlda_summary(&summary_path, &report).unwrap();
        write_offline_vlda_runlog(&runlog_path, Some(&summary_path), None, &dataset, &report)
            .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events);
        assert!(validation.is_valid(), "{:?}", validation.issues);
        let has_vd_synergy = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::PidMetric { name, metadata, .. }
                    if name == "offline_vlda.pid.synergy_v_d_action"
                        && metadata.get("pid_pair").map(String::as_str) == Some("VD")
            )
        });
        assert!(has_vd_synergy);
        let has_all_sample_pid_scope = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::PidMetric { name, metadata, .. }
                    if name == "offline_vlda.pid.synergy_v_d_action"
                        && metadata.get("pid_pair").map(String::as_str) == Some("VD")
                        && metadata.get("sample_scope").map(String::as_str) == Some("all_samples")
                        && metadata.get("heldout_samples_included").map(String::as_str)
                            == Some("4")
                        && metadata.get("preprocessing_fit_scope").map(String::as_str)
                            == Some("all_samples")
            )
        });
        assert!(has_all_sample_pid_scope);
        let has_train_split_pid_scope = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::PidMetric { name, metadata, .. }
                    if name == "offline_vlda.pid.train_split.synergy_v_d_action"
                        && metadata.get("pid_pair").map(String::as_str) == Some("VD")
                        && metadata.get("sample_scope").map(String::as_str)
                            == Some("metadata_split_train")
                        && metadata.get("train_samples").map(String::as_str) == Some("12")
                        && metadata.get("heldout_samples_excluded").map(String::as_str)
                            == Some("4")
                        && metadata.get("preprocessing_fit_scope").map(String::as_str)
                            == Some("metadata_split_train")
            )
        });
        assert!(has_train_split_pid_scope);
        let has_heldout_baseline = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric { name, metadata, .. }
                    if name == "offline_vlda.baseline.heldout_nn_vlda_success_accuracy"
                        && metadata.get("split").map(String::as_str)
                            == Some("metadata_split_heldout")
                        && metadata.get("train_samples").map(String::as_str) == Some("12")
                        && metadata.get("heldout_samples").map(String::as_str) == Some("4")
            )
        });
        assert!(has_heldout_baseline);
        let has_centroid_baseline = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric {
                    name,
                    metadata,
                    ..
                }
                    if name == "offline_vlda.baseline.heldout_centroid_vlda_success_accuracy"
                        && metadata.get("classifier").map(String::as_str)
                            == Some("train_split_nearest_centroid")
                        && metadata.get("distance").map(String::as_str)
                            == Some("train_standardized_euclidean")
                        && metadata.get("split").map(String::as_str)
                            == Some("metadata_split_heldout")
            )
        });
        assert!(has_centroid_baseline);
        let has_balanced_baseline = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric {
                    name,
                    metadata,
                    ..
                }
                    if name
                        == "offline_vlda.baseline.heldout_centroid_vlda_success_balanced_accuracy"
                        && metadata.get("metric").map(String::as_str)
                            == Some("balanced_accuracy")
                        && metadata.get("classifier").map(String::as_str)
                            == Some("train_split_nearest_centroid")
            )
        });
        assert!(has_balanced_baseline);
        let has_auroc_baseline = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric { name, metadata, .. }
                    if name == "offline_vlda.baseline.heldout_centroid_vlda_success_auroc"
                        && metadata.get("metric").map(String::as_str) == Some("auroc")
                        && metadata.get("score").map(String::as_str)
                            == Some(
                                "distance_to_failure_centroid_minus_distance_to_success_centroid"
                            )
            )
        });
        assert!(has_auroc_baseline);
        let has_failure_recall = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric {
                    name,
                    metadata,
                    value,
                    ..
                } if name == "offline_vlda.baseline.heldout_majority_failure_recall"
                    && *value == 0.0
                    && metadata.get("metric").map(String::as_str) == Some("failure_recall")
                    && metadata.get("target_class").map(String::as_str) == Some("failure")
                    && metadata.get("positive_label").map(String::as_str) == Some("success_false")
            )
        });
        assert!(has_failure_recall);
        let has_failure_confusion_count = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric {
                    name,
                    metadata,
                    value,
                    ..
                } if name == "offline_vlda.baseline.heldout_nn_vlda_failure_false_negative_count"
                    && *value >= 0.0
                    && metadata.get("variable").map(String::as_str) == Some("VLDA")
                    && metadata.get("metric").map(String::as_str)
                        == Some("failure_false_negative_count")
            )
        });
        assert!(has_failure_confusion_count);
        let heldout_prediction_correct_events = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    pid_runlog::RunLogEvent::EvaluationMetric { name, .. }
                        if name == "offline_vlda.heldout_prediction.correct"
                )
            })
            .count();
        assert_eq!(heldout_prediction_correct_events, 44);
        let heldout_prediction_score_events = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    pid_runlog::RunLogEvent::EvaluationMetric { name, .. }
                        if name == "offline_vlda.heldout_prediction.score"
                )
            })
            .count();
        assert_eq!(heldout_prediction_score_events, 20);
        let heldout_prediction_distance_events = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    pid_runlog::RunLogEvent::EvaluationMetric { name, .. }
                        if name == "offline_vlda.heldout_prediction.squared_distance"
                )
            })
            .count();
        assert_eq!(heldout_prediction_distance_events, 20);
        let has_prediction_record_event = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric { name, metadata, .. }
                    if name == "offline_vlda.heldout_prediction.correct"
                        && metadata.get("category").map(String::as_str)
                            == Some("heldout_prediction")
                        && metadata.get("sample_id").map(String::as_str) == Some("sample-012")
                        && metadata.get("classifier").map(String::as_str)
                            == Some("train_split_1nn")
                        && metadata.get("variable").map(String::as_str) == Some("VLDA")
                        && metadata.get("nearest_train_sample_id").is_some()
                        && metadata.get("true_success").map(String::as_str).is_some()
                        && metadata.get("predicted_success").map(String::as_str).is_some()
            )
        });
        assert!(has_prediction_record_event);
        let has_centroid_score_event = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric { name, metadata, .. }
                    if name == "offline_vlda.heldout_prediction.score"
                        && metadata.get("classifier").map(String::as_str)
                            == Some("train_split_nearest_centroid")
                        && metadata.get("variable").map(String::as_str) == Some("VLDA")
                        && metadata.get("score_name").map(String::as_str)
                            == Some(OFFLINE_CENTROID_SUCCESS_SCORE)
            )
        });
        assert!(has_centroid_score_event);
        let has_class_coverage_pass = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric { name, metadata, value, .. }
                    if name == "offline_vlda.heldout_split.class_coverage_pass"
                        && *value == 1.0
                        && metadata.get("category").map(String::as_str)
                            == Some("heldout_split_quality")
                        && metadata.get("status").map(String::as_str) == Some("pass")
                        && metadata.get("warnings").map(String::as_str) == Some("0")
            )
        });
        assert!(has_class_coverage_pass);
        let has_episode_disjoint_pass = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::EvaluationMetric { name, metadata, value, .. }
                    if name == "offline_vlda.heldout_split.episode_disjoint_pass"
                        && *value == 1.0
                        && metadata.get("category").map(String::as_str)
                            == Some("heldout_split_quality")
                        && metadata.get("group_key").map(String::as_str) == Some("episode_id")
                        && metadata.get("status").map(String::as_str) == Some("pass")
                        && metadata.get("shared_episodes").map(String::as_str) == Some("0")
            )
        });
        assert!(has_episode_disjoint_pass);
        let contract_uri = events
            .iter()
            .find_map(|event| {
                if let pid_runlog::RunLogEvent::EmbeddingContract { variables, .. } = event {
                    variables
                        .first()
                        .and_then(|variable| variable.artifact_uri.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(contract_uri, "memory://fixture.json");
        let summary = summarize_events(&events).unwrap();
        assert_eq!(summary.embedding_contracts, 1);
        assert_eq!(summary.embeddings, 4);
        assert_eq!(summary.labels, 16);
        assert_eq!(summary.pid_metrics, 42);
        assert!(summary.geometry_metrics >= 21);
        assert_eq!(summary.evaluation_metrics, 142);
        assert_eq!(summary.pid_metric_events, 42);
        assert!(summary.geometry_metric_events >= 21);
        assert_eq!(summary.evaluation_metric_events, 223);

        let _ = std::fs::remove_file(summary_path);
        let _ = std::fs::remove_file(runlog_path);
    }

    #[test]
    fn offline_vlda_runlog_timestamps_stay_monotonic_at_capture_scale() {
        // A real VLA capture emits ~21 metric events per labeled held-out
        // sample; once the total passes 10,000 the old fixed ArtifactLogged/
        // ErrorLogged/RunEnded offsets were overtaken and the run log failed
        // its own advertised `pid-runlog-replay --validate` step. Inflate the
        // held-out prediction records past that threshold and require the
        // emitted log to stay valid.
        let dataset = fixture_dataset();
        let mut report = run_offline_vlda_harness(dataset.clone(), None, None).unwrap();
        assert!(
            !report.heldout_predictions.is_empty(),
            "fixture must produce held-out prediction records"
        );
        let originals = report.heldout_predictions.clone();
        while report.heldout_predictions.len() < 12_000 {
            report.heldout_predictions.extend(originals.iter().cloned());
        }

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let runlog_path =
            std::env::temp_dir().join(format!("pid-offline-vlda-monotonic-scale-{stamp}.jsonl"));
        write_offline_vlda_runlog(&runlog_path, None, None, &dataset, &report).unwrap();
        let validation = pid_runlog::validate_events_from_path(&runlog_path).unwrap();
        assert_eq!(
            validation.errors,
            0,
            "capture-scale run log must validate: {:?}",
            validation
                .issues
                .iter()
                .filter(|issue| issue.severity == pid_runlog::ValidationSeverity::Error)
                .take(3)
                .collect::<Vec<_>>()
        );
        let _ = std::fs::remove_file(runlog_path);
    }

    #[test]
    fn offline_vlda_strict_heldout_class_coverage_marks_run_failed() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            if sample.metadata.get("split").map(String::as_str) == Some("test") {
                sample.labels.insert("success".to_string(), json!(true));
            }
        }
        let report = run_offline_vlda_harness(
            dataset.clone(),
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        assert_eq!(offline_vlda_heldout_class_coverage_status(&report), "warn");

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let runlog_path = dir.join(format!(
            "pid-offline-vlda-strict-heldout-class-coverage-{stamp}.jsonl"
        ));
        write_offline_vlda_runlog_with_options(
            &runlog_path,
            None,
            None,
            &dataset,
            &report,
            OfflineVldaRunlogOptions {
                require_geometry_pass: false,
                require_success_labels: false,
                require_heldout_split: false,
                require_heldout_class_coverage: true,
                require_heldout_episode_disjoint: false,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events);
        assert!(validation.is_valid(), "{:?}", validation.issues);
        let has_coverage_error = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::ErrorLogged { message, recoverable, .. }
                    if !recoverable && message.contains("held-out class coverage warn")
            )
        });
        assert!(has_coverage_error);
        let summary = summarize_events(&events).unwrap();
        assert_eq!(summary.status, Some(RunStatus::Failed));
        assert_eq!(summary.errors, 1);

        let _ = std::fs::remove_file(runlog_path);
    }

    #[test]
    fn offline_vlda_strict_heldout_episode_disjoint_marks_run_failed() {
        let mut dataset = fixture_dataset();
        dataset.samples[12].episode_id = dataset.samples[0].episode_id.clone();
        let report = run_offline_vlda_harness(
            dataset.clone(),
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        let disjoint = report.heldout_episode_disjoint.as_ref().unwrap();
        assert_eq!(disjoint.status, "warn");
        assert_eq!(disjoint.shared_episodes, 1);
        assert_eq!(disjoint.shared_episode_ids, vec!["episode-000".to_string()]);
        assert_eq!(
            offline_vlda_heldout_episode_disjoint_status(&report),
            "warn"
        );

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let runlog_path = dir.join(format!(
            "pid-offline-vlda-strict-heldout-episode-disjoint-{stamp}.jsonl"
        ));
        write_offline_vlda_runlog_with_options(
            &runlog_path,
            None,
            None,
            &dataset,
            &report,
            OfflineVldaRunlogOptions {
                require_geometry_pass: false,
                require_success_labels: false,
                require_heldout_split: false,
                require_heldout_class_coverage: false,
                require_heldout_episode_disjoint: true,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events);
        assert!(validation.is_valid(), "{:?}", validation.issues);
        let has_disjoint_error = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::ErrorLogged { message, recoverable, .. }
                    if !recoverable && message.contains("held-out episode disjointness warn")
            )
        });
        assert!(has_disjoint_error);
        let summary = summarize_events(&events).unwrap();
        assert_eq!(summary.status, Some(RunStatus::Failed));
        assert_eq!(summary.errors, 1);

        let _ = std::fs::remove_file(runlog_path);
    }

    #[test]
    fn offline_vlda_checked_fixture_runs() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/offline_vlda_fixture.json");
        let dataset = read_offline_vlda_dataset(&path).unwrap();
        let input_sha256 = pid_runlog::sha256_file(&path).unwrap();
        let report = run_offline_vlda_harness(
            dataset,
            Some(path.display().to_string()),
            Some(input_sha256),
        )
        .unwrap();
        assert_eq!(report.run_id, "offline-vlda-fixture-run");
        assert_eq!(report.dims.samples, 16);
        assert_eq!(report.label_counts["success"], 16);
        assert_eq!(report.metrics.success_rate, Some(0.75));
        assert_eq!(report.metrics.loo_nn_d_success_accuracy, Some(0.5625));
        assert_eq!(report.metrics.loo_nn_a_success_accuracy, Some(0.4375));
        assert_eq!(
            report.metrics.episode_loo_nn_v_success_accuracy,
            Some(0.625)
        );
        assert_eq!(report.metrics.heldout_majority_success_accuracy, Some(0.75));
        assert_eq!(
            report.metrics.heldout_majority_success_balanced_accuracy,
            Some(0.5)
        );
        assert_eq!(report.metrics.heldout_nn_vlda_success_accuracy, Some(0.25));
        assert_eq!(
            report.metrics.heldout_centroid_vlda_success_accuracy,
            Some(0.25)
        );
        assert_eq!(
            report.metrics.heldout_centroid_vlda_success_auroc,
            Some(0.0)
        );
        assert_eq!(report.heldout_split.as_ref().unwrap().train_samples, 12);
        assert_eq!(
            report
                .heldout_episode_disjoint
                .as_ref()
                .unwrap()
                .shared_episodes,
            0
        );
        assert_eq!(report.heldout_failure_diagnostics.len(), 11);
        assert_eq!(report.train_split_pid.as_ref().unwrap().status, "available");
        assert!(report.metrics.pid_pairs.contains_key("LD"));
        assert_eq!(report.geometry.variables.len(), 6);
        assert_eq!(report.geometry.gates.status, "warn");
    }

    #[test]
    fn offline_vlda_train_split_pid_excludes_heldout_samples() {
        let dataset = fixture_dataset();
        let base_report = run_offline_vlda_harness(
            dataset,
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        let mut changed_heldout = fixture_dataset();
        for (idx, sample) in changed_heldout.samples.iter_mut().enumerate() {
            if sample.metadata.get("split").map(String::as_str) == Some("test") {
                let offset = 100.0 + idx as f64;
                for value in &mut sample.v {
                    *value += offset;
                }
                for value in &mut sample.l {
                    *value -= offset * 0.5;
                }
                for value in &mut sample.d {
                    *value += offset * 0.25;
                }
                for value in &mut sample.a {
                    *value -= offset * 0.75;
                }
            }
        }
        let changed_report = run_offline_vlda_harness(
            changed_heldout,
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        let base_train_pid = base_report.train_split_pid.as_ref().unwrap();
        let changed_train_pid = changed_report.train_split_pid.as_ref().unwrap();
        assert_eq!(base_train_pid.status, "available");
        assert_eq!(changed_train_pid.status, "available");
        assert_eq!(
            base_train_pid.preprocessing,
            changed_train_pid.preprocessing
        );
        assert_eq!(base_train_pid.metrics, changed_train_pid.metrics);
        assert_ne!(
            base_report.preprocessing, changed_report.preprocessing,
            "full-sample preprocessing should still reflect held-out mutations"
        );
        assert_ne!(
            base_report.metrics.pid_pairs, changed_report.metrics.pid_pairs,
            "legacy all-sample PID screens should remain explicitly scoped because they include held-out samples"
        );
    }

    #[test]
    fn offline_vlda_strict_geometry_gate_marks_run_failed() {
        let dataset = fixture_dataset();
        let report = run_offline_vlda_harness(
            dataset.clone(),
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        assert_eq!(report.geometry.gates.status, "warn");

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let runlog_path = dir.join(format!("pid-offline-vlda-strict-{stamp}.jsonl"));
        write_offline_vlda_runlog_with_options(
            &runlog_path,
            None,
            None,
            &dataset,
            &report,
            OfflineVldaRunlogOptions {
                require_geometry_pass: true,
                require_success_labels: false,
                require_heldout_split: false,
                require_heldout_class_coverage: false,
                require_heldout_episode_disjoint: false,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events);
        assert!(validation.is_valid(), "{:?}", validation.issues);
        let summary = summarize_events(&events).unwrap();
        assert_eq!(summary.status, Some(RunStatus::Failed));
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.geometry_metrics, 21);
        assert_eq!(summary.geometry_metric_events, 21);

        let _ = std::fs::remove_file(runlog_path);
    }

    #[test]
    fn offline_vlda_centroid_baseline_requires_both_train_classes() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            if sample.metadata.get("split").map(String::as_str) == Some("train") {
                sample.labels.insert("success".to_string(), json!(true));
            }
        }
        let report = run_offline_vlda_harness(
            dataset,
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        assert!(report.heldout_split.is_some());
        assert!(report.metrics.heldout_majority_success_accuracy.is_some());
        assert_eq!(report.metrics.heldout_centroid_v_success_accuracy, None);
        assert_eq!(
            report.metrics.heldout_centroid_v_success_balanced_accuracy,
            None
        );
        assert_eq!(report.metrics.heldout_centroid_v_success_auroc, None);
        assert_eq!(report.metrics.heldout_centroid_vlda_success_accuracy, None);
        let coverage = report.heldout_class_coverage.as_ref().unwrap();
        assert_eq!(coverage.status, "warn");
        assert_eq!(coverage.train_successes, 12);
        assert_eq!(coverage.train_failures, 0);
        assert_eq!(coverage.warnings.len(), 1);
        assert_eq!(report.heldout_predictions.len(), 24);
        assert_eq!(report.heldout_failure_diagnostics.len(), 6);
        assert!(!report
            .heldout_predictions
            .iter()
            .any(|record| record.classifier == "train_split_nearest_centroid"));
    }

    #[test]
    fn offline_vlda_heldout_balanced_accuracy_requires_both_heldout_classes() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            if sample.metadata.get("split").map(String::as_str) == Some("test") {
                sample.labels.insert("success".to_string(), json!(true));
            }
        }
        let report = run_offline_vlda_harness(
            dataset,
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        assert!(report.metrics.heldout_majority_success_accuracy.is_some());
        assert_eq!(
            report.metrics.heldout_majority_success_balanced_accuracy,
            None
        );
        assert!(report.metrics.heldout_nn_v_success_accuracy.is_some());
        assert_eq!(report.metrics.heldout_nn_v_success_balanced_accuracy, None);
        assert!(report.metrics.heldout_centroid_v_success_accuracy.is_some());
        assert_eq!(
            report.metrics.heldout_centroid_v_success_balanced_accuracy,
            None
        );
        assert_eq!(report.metrics.heldout_centroid_v_success_auroc, None);
        let coverage = report.heldout_class_coverage.as_ref().unwrap();
        assert_eq!(coverage.status, "warn");
        assert_eq!(coverage.heldout_successes, 4);
        assert_eq!(coverage.heldout_failures, 0);
        assert_eq!(coverage.warnings.len(), 1);
        assert_eq!(report.heldout_predictions.len(), 44);
        assert_eq!(report.heldout_failure_diagnostics.len(), 11);
        let majority_failure = failure_diagnostic(&report, "train_split_majority", None);
        assert_eq!(majority_failure.true_failures, 0);
        assert_eq!(majority_failure.failure_recall, None);
    }

    #[test]
    fn offline_vlda_strict_success_labels_marks_run_failed() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            sample.labels.clear();
        }
        let report = run_offline_vlda_harness(
            dataset.clone(),
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        assert_eq!(report.metrics.success_rate, None);
        assert!(report.heldout_predictions.is_empty());
        assert!(report.heldout_failure_diagnostics.is_empty());

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let runlog_path = dir.join(format!("pid-offline-vlda-strict-labels-{stamp}.jsonl"));
        write_offline_vlda_runlog_with_options(
            &runlog_path,
            None,
            None,
            &dataset,
            &report,
            OfflineVldaRunlogOptions {
                require_geometry_pass: false,
                require_success_labels: true,
                require_heldout_split: false,
                require_heldout_class_coverage: false,
                require_heldout_episode_disjoint: false,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events);
        assert!(validation.is_valid(), "{:?}", validation.issues);
        let has_label_error = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::ErrorLogged { message, recoverable, .. }
                    if !recoverable && message.contains("success labels unavailable")
            )
        });
        assert!(has_label_error);
        let summary = summarize_events(&events).unwrap();
        assert_eq!(summary.status, Some(RunStatus::Failed));
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.evaluation_metrics, 5);
        assert_eq!(summary.evaluation_metric_events, 5);

        let _ = std::fs::remove_file(runlog_path);
    }

    #[test]
    fn offline_vlda_strict_heldout_split_marks_run_failed() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            sample.metadata.remove("split");
        }
        let report = run_offline_vlda_harness(
            dataset.clone(),
            Some("memory://fixture.json".to_string()),
            Some("abc".to_string()),
        )
        .unwrap();
        assert_eq!(report.heldout_split, None);
        assert_eq!(report.heldout_episode_disjoint, None);
        assert_eq!(report.train_split_pid, None);
        assert_eq!(offline_vlda_train_split_pid_status(&report), "missing");
        assert_eq!(report.metrics.heldout_majority_success_accuracy, None);
        assert!(report.heldout_predictions.is_empty());
        assert!(report.heldout_failure_diagnostics.is_empty());

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir();
        let runlog_path = dir.join(format!("pid-offline-vlda-strict-heldout-{stamp}.jsonl"));
        write_offline_vlda_runlog_with_options(
            &runlog_path,
            None,
            None,
            &dataset,
            &report,
            OfflineVldaRunlogOptions {
                require_geometry_pass: false,
                require_success_labels: false,
                require_heldout_split: true,
                require_heldout_class_coverage: false,
                require_heldout_episode_disjoint: false,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events);
        assert!(validation.is_valid(), "{:?}", validation.issues);
        let has_split_error = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::ErrorLogged { message, recoverable, .. }
                    if !recoverable && message.contains("held-out split unavailable")
            )
        });
        assert!(has_split_error);
        let summary = summarize_events(&events).unwrap();
        assert_eq!(summary.status, Some(RunStatus::Failed));
        assert_eq!(summary.errors, 1);
        assert_eq!(summary.evaluation_metrics, 14);
        assert_eq!(summary.evaluation_metric_events, 14);

        let _ = std::fs::remove_file(runlog_path);
    }

    #[test]
    fn offline_vlda_harness_rejects_bad_shapes() {
        let mut dataset = fixture_dataset();
        dataset.samples[1].v.pop();
        let err = run_offline_vlda_harness(dataset, None, None).unwrap_err();
        assert!(format!("{err:#}").contains("consistent dimensions"));
    }

    #[test]
    fn offline_vlda_harness_rejects_duplicate_sample_ids() {
        let mut dataset = fixture_dataset();
        dataset.samples[1].sample_id = dataset.samples[0].sample_id.clone();
        let err = run_offline_vlda_harness(dataset, None, None).unwrap_err();
        assert!(format!("{err:#}").contains("unique"));
    }

    #[test]
    fn pid_uncertainty_continuous_emits_cis_and_perm_pvalues() {
        let dataset = fixture_dataset();
        let cfg = OfflineVldaUncertaintyConfig {
            n_boot: 24,
            n_perm: 40,
            block_size: 1,
            alpha: 0.05,
            seed: 7,
        };
        let u = compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &cfg).unwrap();
        assert_eq!(u.mode, "continuous");
        assert_eq!(u.pairs.len(), 3);
        assert!(u.subsample_len >= 1);
        // Deterministic given the same config.
        let u2 = compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &cfg).unwrap();
        assert_eq!(u, u2);
        let vl = u.pairs.iter().find(|p| p.pair == "VL").unwrap();
        // Bootstrap CIs present and well-ordered (n_boot > 0).
        let red = vl.redundancy.as_ref().unwrap();
        assert!(red.ci_low <= red.ci_high);
        assert!(red.n_valid > 0 && red.n_valid <= cfg.n_boot);
        assert!(vl.synergy.is_some() && vl.unique_s1.is_some() && vl.unique_s2.is_some());
        // Permutation p-values present and valid (n_perm > 0).
        let p1 = vl.unique_s1_perm_p.unwrap();
        let p2 = vl.unique_s2_perm_p.unwrap();
        assert!((0.0..=1.0).contains(&p1) && (0.0..=1.0).contains(&p2));
        assert!(vl.perm_n_valid_s1 > 0 && vl.perm_n_valid_s2 > 0);
    }

    #[test]
    fn pid_uncertainty_skips_non_continuous_measures() {
        let dataset = fixture_dataset();
        let cfg = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            n_perm: 0,
            ..Default::default()
        };
        let u = compute_offline_pid_uncertainty(&dataset, PidMode::Discrete, &cfg).unwrap();
        assert!(u.mode.starts_with("skipped"), "mode={}", u.mode);
        assert!(u.pairs.is_empty());
    }

    #[test]
    fn pid_uncertainty_bootstrap_only_omits_perm_pvalues() {
        let dataset = fixture_dataset();
        let cfg = OfflineVldaUncertaintyConfig {
            n_boot: 16,
            n_perm: 0,
            ..Default::default()
        };
        let u = compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &cfg).unwrap();
        let vl = &u.pairs[0];
        assert!(vl.redundancy.is_some());
        assert!(vl.unique_s1_perm_p.is_none() && vl.unique_s2_perm_p.is_none());
        assert!(!OfflineVldaUncertaintyConfig::default().enabled());
    }
}
