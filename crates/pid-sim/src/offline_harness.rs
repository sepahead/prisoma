use anyhow::{bail, Context, Result};
use pid_core::{
    co_information_pairwise, concat_horiz, distance_concentration_stats, gromov_hyperbolicity,
    intrinsic_dimension_levina_bickel, pid2_isx, DistanceConcentrationConfig, HyperbolicityConfig,
    IntrinsicDimConfig, IsxConfig, KsgConfig, MatOwned, MatRef, Metric, NegativeHandling,
    Pid2Config, Standardizer,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaReport {
    pub run_id: String,
    pub config_hash: String,
    pub config: Value,
    pub dims: OfflineVldaDims,
    pub label_counts: BTreeMap<String, usize>,
    pub preprocessing: OfflineVldaPreprocessingReport,
    pub geometry: OfflineVldaGeometryReport,
    pub metrics: OfflineVldaMetrics,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OfflineVldaRunlogOptions {
    pub require_geometry_pass: bool,
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
    let dims = validate_dataset(&dataset)?;
    let label_counts = label_counts(&dataset.samples);
    let analysis = compute_analysis(&dataset.samples, &dims)?;
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
            "mi": "ksg",
            "pid": "isx_ehrlich_ksg",
            "pid_sources": ["V", "L"],
            "target": "A",
            "additional_metrics": ["mi_d_action"],
            "preprocessing": {
                "pid_geometry_space": analysis.preprocessing.strategy.clone(),
                "standardizer": "per_variable_center_scale_population_std"
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
                "loo_nn_vlda_success_accuracy"
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
        metrics: analysis.metrics,
    })
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
                "geometry_gate_status".to_string(),
                report.geometry.gates.status.clone(),
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
    write_metric_events(&mut writer, report, metric_timestamp_base)?;
    if let Some(input_path) = input_path {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns: metric_timestamp_base + 10_000,
            name: "offline_vlda_input_json".to_string(),
            kind: "dataset_json".to_string(),
            uri: input_path.display().to_string(),
            sha256: input_sha256,
            metadata: BTreeMap::new(),
        })?;
    }
    if let Some(summary_path) = summary_path {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns: metric_timestamp_base + 10_001,
            name: "offline_vlda_summary_json".to_string(),
            kind: "summary_json".to_string(),
            uri: summary_path.display().to_string(),
            sha256: summary_sha256,
            metadata: BTreeMap::new(),
        })?;
    }
    let gate_failed = options.require_geometry_pass && report.geometry.gates.status != "pass";
    let run_message = if gate_failed {
        offline_vlda_geometry_gate_failure_message(report)
    } else {
        format!(
            "offline VLDA harness complete: {} samples",
            report.dims.samples
        )
    };
    if gate_failed {
        writer.append(&RunLogEvent::ErrorLogged {
            step: Some(report.dims.samples as u64),
            timestamp_ns: metric_timestamp_base + 19_999,
            message: run_message.clone(),
            recoverable: false,
        })?;
    }
    writer.append(&RunLogEvent::RunEnded {
        run_id: report.run_id.clone(),
        timestamp_ns: metric_timestamp_base + 20_000,
        status: if gate_failed {
            RunStatus::Failed
        } else {
            RunStatus::Succeeded
        },
        message: Some(run_message),
    })?;
    writer.flush()?;
    Ok(())
}

pub fn offline_vlda_geometry_gate_failure_message(report: &OfflineVldaReport) -> String {
    format!(
        "offline VLDA geometry gate {}: {} warning(s)",
        report.geometry.gates.status,
        report.geometry.gates.warnings.len()
    )
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

struct OfflineVldaAnalysis {
    metrics: OfflineVldaMetrics,
    preprocessing: OfflineVldaPreprocessingReport,
    geometry: OfflineVldaGeometryReport,
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
) -> Result<OfflineVldaAnalysis> {
    let prepared = prepare_standardized_embeddings(samples, dims)?;
    let metrics = compute_metrics(samples, &prepared)?;
    let geometry = compute_geometry_report(&prepared);
    Ok(OfflineVldaAnalysis {
        metrics,
        preprocessing: prepared.preprocessing,
        geometry,
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
) -> Result<OfflineVldaMetrics> {
    let v = prepared.v.as_ref();
    let l = prepared.l.as_ref();
    let d = prepared.d.as_ref();
    let a = prepared.a.as_ref();
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
    let pid = pid2_isx(v, l, a, &pid_cfg)?;
    let mi_v_action = pid_core::ksg_mi(v, a, &ksg)?;
    let mi_l_action = pid_core::ksg_mi(l, a, &ksg)?;
    let mi_d_action = pid_core::ksg_mi(d, a, &ksg)?;
    let mi_vl_action = pid_core::ksg_mi_concat_xy(v, l, a, &ksg)?;
    let co_information_v_l_action = co_information_pairwise(v, l, a, &ksg)?;
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
    Ok(OfflineVldaMetrics {
        mi_v_action,
        mi_l_action,
        mi_d_action,
        mi_vl_action,
        co_information_v_l_action,
        redundancy_v_l_action: pid.redundancy,
        unique_v_action: pid.unique_s1,
        unique_l_action: pid.unique_s2,
        synergy_v_l_action: pid.synergy,
        success_rate,
        majority_success_accuracy,
        loo_nn_v_success_accuracy,
        loo_nn_l_success_accuracy,
        loo_nn_d_success_accuracy,
        loo_nn_a_success_accuracy,
        loo_nn_vlda_success_accuracy,
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
    let gates = compute_geometry_gates(&variables);
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
) -> OfflineVldaGeometryGates {
    let mut warnings = Vec::new();
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

fn squared_euclidean(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            let delta = left - right;
            delta * delta
        })
        .sum()
}

fn write_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    timestamp_base_ns: u64,
) -> Result<()> {
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
            metadata: [("category".to_string(), "pid".to_string())]
                .into_iter()
                .collect(),
        })?;
    }
    let mut idx = metrics.len() as u64;
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
                    metadata: BTreeMap::new(),
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
        assert_eq!(summary.pid_metrics, 9);
        assert!(summary.geometry_metrics >= 21);
        assert!(summary.evaluation_metrics >= 8);

        let _ = std::fs::remove_file(summary_path);
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
        assert_eq!(report.geometry.variables.len(), 6);
        assert_eq!(report.geometry.gates.status, "warn");
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
}
