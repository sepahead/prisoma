use anyhow::{bail, Context, Result};
use pid_core::experimental::continuous::raw_scalars::{
    co_information_pairwise, ksg_mi, ksg_mi_concat_xy,
};
use pid_core::experimental::continuous::{pid2_isx, IsxConfig, Pid2Config};
use pid_core::stable::continuous::{KsgConfig, NegativeHandling};
use pid_core::MatRef;
use pid_runlog::{
    EmbeddingVariableContract, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

/// Finite ceiling for the deliberately small quadratic-time toy estimator run.
///
/// This is a software resource bound, not a recommended study sample size.
pub const MAX_TOY_EPISODES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToyHarnessConfig {
    pub episodes: usize,
    pub seed: u64,
    pub failure_period: usize,
    pub success_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToyEpisodeSample {
    pub episode_id: String,
    pub vision_scalar: f64,
    pub language_scalar: f64,
    pub target_action: f64,
    pub policy_action: f64,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToyPidMetrics {
    pub mi_vision_action: f64,
    pub mi_language_action: f64,
    pub mi_joint_action: f64,
    pub co_information: f64,
    pub redundancy: f64,
    pub unique_vision: f64,
    pub unique_language: f64,
    pub synergy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToyBaselineMetrics {
    pub success_rate: f64,
    pub majority_accuracy: f64,
    pub vision_only_accuracy: f64,
    pub language_only_accuracy: f64,
    pub action_error_accuracy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToyHarnessReport {
    pub run_id: String,
    pub config_hash: String,
    pub config: ToyHarnessConfig,
    pub samples: Vec<ToyEpisodeSample>,
    pub pid: ToyPidMetrics,
    pub baselines: ToyBaselineMetrics,
}

impl Default for ToyHarnessConfig {
    fn default() -> Self {
        Self {
            episodes: 64,
            seed: 0x2026_0509,
            failure_period: 5,
            success_threshold: 0.2,
        }
    }
}

impl ToyHarnessReport {
    pub fn failures(&self) -> usize {
        self.samples.iter().filter(|sample| !sample.success).count()
    }
}

pub fn run_toy_harness(config: ToyHarnessConfig) -> Result<ToyHarnessReport> {
    validate_config(&config)?;
    let config_json = serde_json::to_value(&config)?;
    let config_hash = pid_runlog::canonical_json_hash_v2(&config_json)?;
    let samples = generate_samples(&config);
    let pid = compute_pid_metrics(&samples)?;
    let baselines = compute_baselines(&samples, config.success_threshold);
    Ok(ToyHarnessReport {
        run_id: "toy-vla-baseline-run".to_string(),
        config_hash,
        config,
        samples,
        pid,
        baselines,
    })
}

pub fn write_toy_harness_summary(path: impl AsRef<Path>, report: &ToyHarnessReport) -> Result<()> {
    ensure_parent(path.as_ref())?;
    pid_runlog::write_json_file(path, report)
}

/// Reject output paths that already name, or may name, the same file.
pub fn ensure_distinct_toy_output_paths(runlog: &Path, summary: &Path) -> Result<()> {
    let aliases = runlog == summary
        || (runlog.exists()
            && summary.exists()
            && same_file::is_same_file(runlog, summary).with_context(|| {
                format!(
                    "failed to compare toy run log {} with summary {}",
                    runlog.display(),
                    summary.display()
                )
            })?);
    if aliases {
        bail!("toy run log and summary must be distinct files");
    }
    Ok(())
}

pub fn write_toy_harness_runlog(
    path: impl AsRef<Path>,
    summary_path: Option<&Path>,
    report: &ToyHarnessReport,
) -> Result<()> {
    let path = path.as_ref();
    ensure_parent(path)?;
    if let Some(summary_path) = summary_path {
        ensure_distinct_toy_output_paths(path, summary_path)?;
    }
    let (summary_uri, summary_sha256) = match summary_path {
        Some(summary_path) => {
            let metadata = std::fs::symlink_metadata(summary_path).with_context(|| {
                format!("failed to inspect toy summary {}", summary_path.display())
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                bail!(
                    "toy summary must be a non-symlink regular file: {}",
                    summary_path.display()
                );
            }
            (
                Some(summary_path.display().to_string()),
                Some(pid_runlog::sha256_file(summary_path).with_context(|| {
                    format!("failed to hash toy summary {}", summary_path.display())
                })?),
            )
        }
        None => (None, None),
    };
    let mut writer = RunLogWriter::create(path)?;
    let embedding_timestamp_base = report.samples.len() as u64 * 1_000_000 + 1_000_000;
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: report.run_id.clone(),
        timestamp_ns: 0,
        config_hash: report.config_hash.clone(),
        metadata: [
            ("source".to_string(), "pid-toy-harness".to_string()),
            ("task".to_string(), "toy_pick_place_label".to_string()),
            ("label".to_string(), "success".to_string()),
        ]
        .into_iter()
        .collect(),
    })?;
    writer.append(&RunLogEvent::ConfigLogged {
        timestamp_ns: 0,
        config_hash: report.config_hash.clone(),
        config: serde_json::to_value(&report.config)?,
    })?;
    for (idx, sample) in report.samples.iter().enumerate() {
        let step = idx as u64;
        let timestamp_ns = step * 1_000_000;
        writer.append(&RunLogEvent::FrameObserved {
            step,
            timestamp_ns,
            observation_hash: Some(pid_runlog::canonical_json_hash_v2(sample)?),
            metadata: [
                ("episode_id".to_string(), sample.episode_id.clone()),
                ("success".to_string(), sample.success.to_string()),
            ]
            .into_iter()
            .collect(),
        })?;
        writer.append(&RunLogEvent::LabelObserved {
            step,
            timestamp_ns,
            name: "toy_vla.success".to_string(),
            value: json!(sample.success),
            metadata: [
                ("episode_id".to_string(), sample.episode_id.clone()),
                ("label_kind".to_string(), "success".to_string()),
            ]
            .into_iter()
            .collect(),
        })?;
    }
    writer.append(&RunLogEvent::EmbeddingContract {
        timestamp_ns: embedding_timestamp_base,
        name: "toy_vla.vlda_contract".to_string(),
        variables: [
            ("V", "toy_vla.vision_scalar"),
            ("L", "toy_vla.language_scalar"),
            ("D", "toy_vla.target_action"),
            ("A", "toy_vla.policy_action"),
        ]
        .into_iter()
        .map(|(variable, source)| EmbeddingVariableContract {
            variable: variable.to_string(),
            source: source.to_string(),
            dims: vec![report.samples.len(), 1],
            artifact_uri: summary_uri.clone(),
            sha256: summary_sha256.clone(),
        })
        .collect(),
        metadata: [
            ("task".to_string(), "toy_pick_place_label".to_string()),
            ("model".to_string(), "deterministic_toy_policy".to_string()),
            ("preprocessing".to_string(), "scalar_identity".to_string()),
            ("decomposition".to_string(), "(V,L,D,A)".to_string()),
        ]
        .into_iter()
        .collect(),
    })?;
    for (idx, (name, dims)) in [
        ("toy_vla.vision_scalar", vec![report.samples.len(), 1]),
        ("toy_vla.language_scalar", vec![report.samples.len(), 1]),
        ("toy_vla.target_action", vec![report.samples.len(), 1]),
        ("toy_vla.policy_action", vec![report.samples.len(), 1]),
    ]
    .into_iter()
    .enumerate()
    {
        writer.append(&RunLogEvent::EmbeddingCaptured {
            step: report.samples.len() as u64,
            timestamp_ns: embedding_timestamp_base + idx as u64 + 1,
            name: name.to_string(),
            dims,
            artifact_uri: summary_uri.clone(),
            sha256: summary_sha256.clone(),
            metadata: [("source".to_string(), "toy_harness_summary".to_string())]
                .into_iter()
                .collect(),
        })?;
    }
    let metric_timestamp_base = embedding_timestamp_base + 10_000;
    write_metric_events(&mut writer, report, metric_timestamp_base)?;
    if let Some(summary_path) = summary_path {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns: metric_timestamp_base + 10_000,
            name: "toy_vla_summary_json".to_string(),
            kind: "summary_json".to_string(),
            uri: summary_path.display().to_string(),
            sha256: summary_sha256,
            metadata: [("task".to_string(), "toy_pick_place_label".to_string())]
                .into_iter()
                .collect(),
        })?;
    }
    writer.append(&RunLogEvent::RunEnded {
        run_id: report.run_id.clone(),
        timestamp_ns: metric_timestamp_base + 20_000,
        status: RunStatus::Succeeded,
        message: Some(format!(
            "toy harness complete: {} episodes, {} failures",
            report.samples.len(),
            report.failures()
        )),
    })?;
    writer.flush()?;
    Ok(())
}

fn validate_config(config: &ToyHarnessConfig) -> Result<()> {
    if config.episodes < 8 {
        bail!("episodes must be at least 8");
    }
    if config.episodes > MAX_TOY_EPISODES {
        bail!("episodes must not exceed the {MAX_TOY_EPISODES}-episode toy limit");
    }
    if config.failure_period == 0 {
        bail!("failure_period must be positive");
    }
    if !config.success_threshold.is_finite() || config.success_threshold <= 0.0 {
        bail!("success_threshold must be positive and finite");
    }
    Ok(())
}

fn generate_samples(config: &ToyHarnessConfig) -> Vec<ToyEpisodeSample> {
    let mut rng = ToyRng::new(config.seed);
    (0..config.episodes)
        .map(|idx| {
            let instruction = if idx % 2 == 0 { 1.0 } else { -1.0 };
            let scene_sign = if rng.next_unit() >= 0.5 { 1.0 } else { -1.0 };
            let object_offset = scene_sign * (0.4 + 0.6 * rng.next_unit());
            let vision_scalar = object_offset + 0.02 * rng.normal();
            let language_scalar = instruction + 0.02 * rng.normal();
            let target_action = instruction * object_offset;
            let faulted = idx % config.failure_period == 0;
            let policy_action = if faulted {
                object_offset + 0.03 * rng.normal()
            } else {
                target_action + 0.03 * rng.normal()
            };
            let success = (policy_action - target_action).abs() <= config.success_threshold;
            ToyEpisodeSample {
                episode_id: format!("toy-episode-{idx:03}"),
                vision_scalar,
                language_scalar,
                target_action,
                policy_action,
                success,
            }
        })
        .collect()
}

fn compute_pid_metrics(samples: &[ToyEpisodeSample]) -> Result<ToyPidMetrics> {
    let n = samples.len();
    let vision: Vec<f64> = samples.iter().map(|sample| sample.vision_scalar).collect();
    let language: Vec<f64> = samples
        .iter()
        .map(|sample| sample.language_scalar)
        .collect();
    let action: Vec<f64> = samples.iter().map(|sample| sample.policy_action).collect();
    let vision = MatRef::new(&vision, n, 1)?;
    let language = MatRef::new(&language, n, 1)?;
    let action = MatRef::new(&action, n, 1)?;
    // The current pid-core review contract fails closed unless the caller asserts the population
    // law. The toy harness's
    // (V,L,A) are synthetic continuous variables, so the full-dimensional absolutely-continuous
    // assertion holds by construction. `Allow` preserves the PID identity (never clamp before the
    // subtraction).
    let ksg = KsgConfig::assume_regular_full_dimensional()
        .with_negative_handling(NegativeHandling::Allow);
    let pid_cfg = Pid2Config {
        ksg: ksg.clone(),
        isx: IsxConfig {
            k: ksg.k,
            metric: ksg.metric,
            tie_epsilon: ksg.tie_epsilon,
            ..IsxConfig::assume_regular_full_dimensional()
        },
    };
    let pid = pid2_isx(vision, language, action, &pid_cfg)?;
    let mi_vision_action = ksg_mi(vision, action, &ksg)?;
    let mi_language_action = ksg_mi(language, action, &ksg)?;
    let mi_joint_action = ksg_mi_concat_xy(vision, language, action, &ksg)?;
    let co_information = co_information_pairwise(vision, language, action, &ksg)?;
    Ok(ToyPidMetrics {
        mi_vision_action,
        mi_language_action,
        mi_joint_action,
        co_information,
        redundancy: pid.redundancy,
        unique_vision: pid.unique_s1,
        unique_language: pid.unique_s2,
        synergy: pid.synergy,
    })
}

fn compute_baselines(samples: &[ToyEpisodeSample], threshold: f64) -> ToyBaselineMetrics {
    let success_rate =
        samples.iter().filter(|sample| sample.success).count() as f64 / samples.len() as f64;
    let majority_label = success_rate >= 0.5;
    let majority_accuracy = accuracy(samples, |_| majority_label);
    let vision_only_accuracy = accuracy(samples, |sample| sample.vision_scalar.abs() < 0.75);
    let language_only_accuracy = accuracy(samples, |sample| sample.language_scalar > 0.0);
    let action_error_accuracy = accuracy(samples, |sample| {
        (sample.policy_action - sample.target_action).abs() <= threshold
    });
    ToyBaselineMetrics {
        success_rate,
        majority_accuracy,
        vision_only_accuracy,
        language_only_accuracy,
        action_error_accuracy,
    }
}

fn accuracy<F>(samples: &[ToyEpisodeSample], predict: F) -> f64
where
    F: Fn(&ToyEpisodeSample) -> bool,
{
    let correct = samples
        .iter()
        .filter(|sample| predict(sample) == sample.success)
        .count();
    correct as f64 / samples.len() as f64
}

fn write_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &ToyHarnessReport,
    timestamp_base_ns: u64,
) -> Result<()> {
    let mut metrics = vec![
        (
            "toy_vla.pid.mi_vision_action",
            report.pid.mi_vision_action,
            "pid",
        ),
        (
            "toy_vla.pid.mi_language_action",
            report.pid.mi_language_action,
            "pid",
        ),
        (
            "toy_vla.pid.mi_joint_action",
            report.pid.mi_joint_action,
            "pid",
        ),
        (
            "toy_vla.pid.co_information",
            report.pid.co_information,
            "pid",
        ),
        ("toy_vla.pid.redundancy", report.pid.redundancy, "pid"),
        ("toy_vla.pid.unique_vision", report.pid.unique_vision, "pid"),
        (
            "toy_vla.pid.unique_language",
            report.pid.unique_language,
            "pid",
        ),
        ("toy_vla.pid.synergy", report.pid.synergy, "pid"),
        (
            "toy_vla.baseline.success_rate",
            report.baselines.success_rate,
            "baseline",
        ),
        (
            "toy_vla.baseline.majority_accuracy",
            report.baselines.majority_accuracy,
            "baseline",
        ),
        (
            "toy_vla.baseline.vision_only_accuracy",
            report.baselines.vision_only_accuracy,
            "baseline",
        ),
        (
            "toy_vla.baseline.language_only_accuracy",
            report.baselines.language_only_accuracy,
            "baseline",
        ),
        (
            "toy_vla.baseline.action_error_accuracy",
            report.baselines.action_error_accuracy,
            "baseline",
        ),
    ];
    metrics.push(("toy_vla.labels.failures", report.failures() as f64, "label"));
    for (idx, (name, value, category)) in metrics.into_iter().enumerate() {
        let mut metadata = BTreeMap::new();
        metadata.insert("category".to_string(), category.to_string());
        let event = if category == "pid" {
            // Information quantities are in nats (pid-core convention).
            metadata.insert("units".to_string(), "nats".to_string());
            RunLogEvent::PidMetric {
                step: report.samples.len() as u64,
                timestamp_ns: timestamp_base_ns + idx as u64,
                name: name.to_string(),
                value,
                metadata,
            }
        } else {
            RunLogEvent::EvaluationMetric {
                step: report.samples.len() as u64,
                timestamp_ns: timestamp_base_ns + idx as u64,
                name: name.to_string(),
                value,
                metadata,
            }
        };
        writer.append(&event)?;
    }
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

struct ToyRng {
    state: u64,
}

impl ToyRng {
    fn new(seed: u64) -> Self {
        // xorshift has a single absorbing zero state, and `seed ^ CONST` hits it
        // for the (user-settable) seed == 0x9E37_79B9_7F4A_7C15 — which would
        // degenerate the whole stream to a constant. Remap that one state to a
        // fixed odd constant so every seed yields a full-period stream.
        let state = seed ^ 0x9E37_79B9_7F4A_7C15;
        Self {
            state: if state == 0 {
                0x2545_F491_4F6C_DD1D
            } else {
                state
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_unit(&mut self) -> f64 {
        let u = self.next_u64() >> 11;
        (u as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    fn normal(&mut self) -> f64 {
        let u1 = self.next_unit().max(1e-12);
        let u2 = self.next_unit();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        r * theta.cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toy_rng_survives_the_zero_state_seed() {
        // seed == the xor constant used to hit the xorshift absorbing zero state.
        let mut rng = ToyRng::new(0x9E37_79B9_7F4A_7C15);
        let a = rng.next_u64();
        let b = rng.next_u64();
        assert!(a != 0 || b != 0, "stream is stuck at zero");
        assert_ne!(a, b, "stream is constant");
    }
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("prisoma-toy-harness-{name}-{stamp}"))
    }

    #[test]
    fn toy_harness_generates_labels_pid_and_baselines() {
        let report = run_toy_harness(ToyHarnessConfig {
            episodes: 48,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(report.samples.len(), 48);
        assert!(report.failures() > 0);
        assert!(report.baselines.success_rate > 0.0 && report.baselines.success_rate < 1.0);
        assert!(report.baselines.majority_accuracy < 1.0);
        assert_eq!(report.baselines.action_error_accuracy, 1.0);
        for value in [
            report.pid.mi_vision_action,
            report.pid.mi_language_action,
            report.pid.mi_joint_action,
            report.pid.co_information,
            report.pid.redundancy,
            report.pid.unique_vision,
            report.pid.unique_language,
            report.pid.synergy,
        ] {
            assert!(value.is_finite());
        }
    }

    #[test]
    fn toy_harness_rejects_unbounded_episode_counts() {
        let error = run_toy_harness(ToyHarnessConfig {
            episodes: MAX_TOY_EPISODES + 1,
            ..Default::default()
        })
        .unwrap_err();
        assert!(error.to_string().contains("toy limit"));
    }

    #[test]
    fn toy_runlog_rejects_missing_or_aliased_summary_evidence() {
        let report = run_toy_harness(ToyHarnessConfig {
            episodes: 32,
            ..Default::default()
        })
        .unwrap();
        let missing = temp_path("missing-summary.json");
        let runlog = temp_path("missing-summary-runlog.jsonl");
        assert!(write_toy_harness_runlog(&runlog, Some(&missing), &report).is_err());
        assert!(!runlog.exists());

        let same = temp_path("same-output.json");
        write_toy_harness_summary(&same, &report).unwrap();
        let before = std::fs::read(&same).unwrap();
        assert!(write_toy_harness_runlog(&same, Some(&same), &report)
            .unwrap_err()
            .to_string()
            .contains("distinct"));
        assert_eq!(std::fs::read(&same).unwrap(), before);
        let _ = std::fs::remove_file(same);
    }

    #[cfg(unix)]
    #[test]
    fn toy_runlog_rejects_symlink_summary_evidence() {
        use std::os::unix::fs::symlink;

        let report = run_toy_harness(ToyHarnessConfig {
            episodes: 32,
            ..Default::default()
        })
        .unwrap();
        let summary = temp_path("summary-target.json");
        let link = temp_path("summary-link.json");
        let runlog = temp_path("summary-link-runlog.jsonl");
        write_toy_harness_summary(&summary, &report).unwrap();
        symlink(&summary, &link).unwrap();

        assert!(write_toy_harness_runlog(&runlog, Some(&link), &report)
            .unwrap_err()
            .to_string()
            .contains("non-symlink"));
        assert!(!runlog.exists());
        let _ = std::fs::remove_file(link);
        let _ = std::fs::remove_file(summary);
    }

    #[test]
    fn toy_harness_runlog_validates() {
        let summary_path = temp_path("summary.json");
        let runlog_path = temp_path("runlog.jsonl");
        let report = run_toy_harness(ToyHarnessConfig {
            episodes: 32,
            ..Default::default()
        })
        .unwrap();
        write_toy_harness_summary(&summary_path, &report).unwrap();
        write_toy_harness_runlog(&runlog_path, Some(&summary_path), &report).unwrap();
        let events = pid_runlog::read_events_from_path(&runlog_path).unwrap();
        let validation = pid_runlog::validate_events(&events).unwrap();
        assert!(validation.is_valid(), "{:?}", validation.issues);
        let summary = pid_runlog::summarize_events(&events).unwrap();
        assert_eq!(summary.run_id.as_deref(), Some("toy-vla-baseline-run"));
        assert_eq!(summary.pid_metrics, 8);
        assert_eq!(summary.evaluation_metrics, 6);
        assert_eq!(summary.pid_metric_events, 8);
        assert_eq!(summary.evaluation_metric_events, 6);
        assert_eq!(summary.labels, 32);
        assert_eq!(summary.embeddings, 4);
        assert_eq!(summary.embedding_contracts, 1);
        assert_eq!(summary.artifacts, 1);
        assert_eq!(summary.errors, 0);
        let _ = std::fs::remove_file(summary_path);
        let _ = std::fs::remove_file(runlog_path);
    }
}
