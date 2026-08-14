use crate::file_snapshot::{
    parse_strict_json, read_bounded_regular_file, validate_strict_json_lines,
};
use anyhow::{anyhow, bail, ensure, Context, Result};
use pid_core::diagnostics::{
    distance_concentration_stats, intrinsic_dimension_levina_bickel,
    sampled_four_point_delta_summary, DistanceConcentrationConfig, HyperbolicityConfig,
    IntrinsicDimConfig,
};
use pid_core::experimental::continuous::raw_scalars::ksg_mi;
use pid_core::experimental::continuous::{
    pid2_isx, pid2_isx_estimate, pid2_resource_estimate, IsxConfig, Pid2Config, Pid2Result,
};
use pid_core::experimental::pipelines::{
    bootstrap_rows_stats, permutation_rows_pvalue_with, pls_cv_select_components_with_budget,
    BlockLengthSelection, BootstrapConfig, LogisticRegression, LogisticRegressionConfig,
    PlsCvCandidateStatus, PlsProjector, ResamplingValidityDeclaration, RowResampleScheme,
    StatisticCallbackDeclaration,
};
use pid_core::stable::continuous::{KsgConfig, NegativeHandling};
use pid_core::stable::preprocessing::{ConstantColumnPolicy, Standardizer};
#[cfg(test)]
use pid_core::stable::quantized::fitted_quantized_sxpid2_resource_estimate;
use pid_core::stable::quantized::{
    fitted_quantized_sxpid2_with_budget, EqualWidthQuantizer, OutOfRangePolicy, QuantizedData,
    QuantizerConfig,
};
use pid_core::{
    MatOwned, MatRef, Metric, PidError, ResourceBudget, DEFAULT_MAX_BYTES,
    DEFAULT_MAX_PAIRWISE_DISTANCES,
};
// Re-exported so the harness CLI (and downstream callers) can pick the permutation
// null without importing pid-core directly.
pub use pid_core::experimental::pipelines::PermutationScheme;
use pid_runlog::{
    EmbeddingVariableContract, RunLogEvent, RunLogLimits, RunLogWriter, RunStatus,
    RUN_LOG_SCHEMA_VERSION,
};
use serde::de::{DeserializeOwned, MapAccess, Visitor};
use serde::ser::{Error as _, SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;

const OFFLINE_NCP_PUBLICATION_RECEIPT_MAX_BYTES: usize = 64 * 1024;
const OFFLINE_NCP_RUNLOG_MAX_BYTES: usize = 64 * 1024 * 1024;
const OFFLINE_RESOURCE_LIMITS_MAX_BYTES: usize = 64 * 1_024;
const OFFLINE_SUMMARY_MAX_BYTES: u64 = 64 * 1_024 * 1_024;
const OFFLINE_UNCERTAINTY_MAX_BYTES: u64 = 8 * 1_024 * 1_024;
const OFFLINE_VLDA_REPORT_SCHEMA: &str = "prisoma.offline_vlda.report/5";
const OFFLINE_UNCERTAINTY_SCHEMA_VERSION: u32 = 3;

const OFFLINE_DEFAULT_MAX_INPUT_BYTES: u64 = 64 * 1_024 * 1_024;
const OFFLINE_DEFAULT_MAX_SAMPLES: usize = 1_024;
const OFFLINE_DEFAULT_MAX_AXIS_SCALARS: usize = 1_024 * 1_024;
const OFFLINE_DEFAULT_MAX_METADATA_ENTRIES: usize = 64 * 1_024;
const OFFLINE_DEFAULT_MAX_METADATA_JSON_NODES: usize = 64 * 1_024;
const OFFLINE_DEFAULT_MAX_METADATA_UTF8_BYTES: usize = 8 * 1_024 * 1_024;
const OFFLINE_DEFAULT_MAX_METADATA_DEPTH: usize = 64;
const OFFLINE_DEFAULT_MAX_PAIRWISE_DISTANCE_EVALUATIONS: u64 = 50_000_000;
const OFFLINE_DEFAULT_MAX_DISTANCE_COORDINATE_EVALUATIONS: u64 = 100_000_000;
const OFFLINE_DEFAULT_MAX_DENSE_SOLVER_OPERATIONS: u64 = 100_000_000;
const OFFLINE_DEFAULT_MAX_CATEGORICAL_PID_OPERATIONS: u64 = 500_000_000;

// These constants mirror the pinned pid-core 0.9 review contract. Unit tests compare the local
// dimension-only projections with pid-core's public estimates. A future submodule update must
// therefore review this aggregate harness model instead of silently inheriting new work.
const OFFLINE_PLS_MAX_ITERATIONS: u128 = 200;
const OFFLINE_PLS_MAX_SOLVER_COMPONENTS: u128 = 512;
const OFFLINE_LOGREG_MAX_ITERATIONS: u128 = 100;
const OFFLINE_LOGREG_MAX_SOLVER_COLUMNS: u128 = 1_024;

const OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION_WARNING: f64 = 20.0;
const OFFLINE_GEOMETRY_MIN_PAIRWISE_CV_WARNING: f64 = 0.1;
const OFFLINE_GEOMETRY_INTRINSIC_K: usize = 10;
const OFFLINE_GEOMETRY_HYPERBOLICITY_SAMPLES: usize = 500;
const OFFLINE_HELDOUT_SPLIT_METADATA_KEY: &str = "split";
const OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_KEY: &str = "split_scientific_eligibility";
const OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_BLOCKED: &str = "blocked_unfrozen_or_unreviewed";
const OFFLINE_CENTROID_SUCCESS_SCORE: &str =
    "distance_to_failure_centroid_minus_distance_to_success_centroid";
const OFFLINE_GEOMETRY_VARIABLES: u128 = 6;
const OFFLINE_BASELINE_FEATURE_VIEWS: u128 = 5;
// These counts match pid-core's ResourceEstimate::pairwise_distances contract. One KSG call
// accounts for its estimator pass plus the source, target, and joint support-shell preflights.
// PID2 contains two such KSG calls, one joint x-blocks pass, and one ISX pass.
const OFFLINE_KSG_PAIRWISE_PASSES: u128 = 4;
const OFFLINE_PID2_PAIRWISE_PASSES: u128 = 10;
const OFFLINE_CONTINUOUS_PID_PAIRWISE_PASSES: u128 =
    3 * OFFLINE_KSG_PAIRWISE_PASSES + 3 * OFFLINE_PID2_PAIRWISE_PASSES;
/// Unique-joint-bin fraction above which categorical empirical-PMF estimates are treated as
/// too sparse for application interpretation (grandplan §7.6). High occupancy relative to the
/// sample count creates severe plug-in bias and unstable atom allocation. It does not imply that
/// every MI term is near `ln(n)`.
const OFFLINE_CATEGORICAL_SATURATION_UNIQUE_FRACTION_MAX: f64 = 0.8;
const SCIENTIFIC_REASON_CATEGORICAL_SATURATION: &str = "categorical_saturation";
const SCIENTIFIC_REASON_SUPERVISED_SAME_ROW: &str = "supervised_same_row_preprocessing";
const SCIENTIFIC_REASON_SUPERVISED_SAME_ROW_AND_SATURATION: &str =
    "supervised_same_row_preprocessing_and_categorical_saturation";

struct BoundedJsonBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

/// Stream one canonical dataset representation into SHA-256 without materializing duplicate
/// JSON buffers or a second `serde_json::Value` tree.
///
/// The explicit destructuring below is intentional. Adding a dataset or sample field becomes a
/// compile error until its place in the canonical representation receives review. Object keys
/// use the same recursive lexicographic order as `pid_runlog::canonical_json_hash_v2`.
struct CanonicalOfflineVldaDataset<'a>(&'a OfflineVldaDataset);

struct CanonicalOfflineVldaSamples<'a>(&'a [OfflineVldaSample]);

struct CanonicalOfflineVldaSample<'a>(&'a OfflineVldaSample);

struct CanonicalOfflineVldaLabels<'a>(&'a BTreeMap<String, Value>);

struct CanonicalJsonValue<'a>(&'a Value);

impl Serialize for CanonicalOfflineVldaDataset<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let OfflineVldaDataset {
            run_id,
            source,
            model,
            task,
            support,
            continuous_tuple_support,
            capture_integrity,
            publication_receipt,
            publication_receipt_verified_content_sha256: _,
            samples,
        } = self.0;
        let field_count = 6
            + usize::from(!continuous_tuple_support.is_empty())
            + usize::from(capture_integrity.is_some())
            + usize::from(publication_receipt.is_some());
        let mut map = serializer.serialize_map(Some(field_count))?;
        if let Some(value) = capture_integrity {
            map.serialize_entry("capture_integrity", value)?;
        }
        if !continuous_tuple_support.is_empty() {
            map.serialize_entry("continuous_tuple_support", continuous_tuple_support)?;
        }
        map.serialize_entry("model", model)?;
        if let Some(value) = publication_receipt {
            map.serialize_entry("publication_receipt", value)?;
        }
        map.serialize_entry("run_id", run_id)?;
        map.serialize_entry("samples", &CanonicalOfflineVldaSamples(samples))?;
        map.serialize_entry("source", source)?;
        map.serialize_entry("support", support)?;
        map.serialize_entry("task", task)?;
        map.end()
    }
}

impl Serialize for CanonicalOfflineVldaSamples<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for sample in self.0 {
            sequence.serialize_element(&CanonicalOfflineVldaSample(sample))?;
        }
        sequence.end()
    }
}

impl Serialize for CanonicalOfflineVldaSample<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let OfflineVldaSample {
            sample_id,
            episode_id,
            v,
            l,
            d,
            a,
            labels,
            metadata,
        } = self.0;
        for value in v.iter().chain(l).chain(d).chain(a) {
            if !value.is_finite() {
                return Err(S::Error::custom(
                    "offline VLDA dataset contains a non-finite axis value",
                ));
            }
        }
        let mut map = serializer.serialize_map(Some(8))?;
        map.serialize_entry("a", a)?;
        map.serialize_entry("d", d)?;
        map.serialize_entry("episode_id", episode_id)?;
        map.serialize_entry("l", l)?;
        map.serialize_entry("labels", &CanonicalOfflineVldaLabels(labels))?;
        map.serialize_entry("metadata", metadata)?;
        map.serialize_entry("sample_id", sample_id)?;
        map.serialize_entry("v", v)?;
        map.end()
    }
}

impl Serialize for CanonicalOfflineVldaLabels<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in self.0 {
            map.serialize_entry(key, &CanonicalJsonValue(value))?;
        }
        map.end()
    }
}

impl Serialize for CanonicalJsonValue<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::Number(value) => value.serialize(serializer),
            Value::String(value) => serializer.serialize_str(value),
            Value::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(&CanonicalJsonValue(value))?;
                }
                sequence.end()
            }
            Value::Object(values) => {
                let mut entries = Vec::new();
                entries
                    .try_reserve_exact(values.len())
                    .map_err(S::Error::custom)?;
                entries.extend(values.iter());
                entries.sort_unstable_by_key(|(key, _)| *key);
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, &CanonicalJsonValue(value))?;
                }
                map.end()
            }
        }
    }
}

#[derive(Default)]
struct Sha256Writer(Sha256);

impl Write for Sha256Writer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn offline_vlda_dataset_content_sha256(dataset: &OfflineVldaDataset) -> Result<String> {
    let mut writer = Sha256Writer::default();
    serde_json::to_writer(&mut writer, &CanonicalOfflineVldaDataset(dataset))
        .context("failed to stream the canonical offline VLDA dataset")?;
    Ok(crate::lowercase_hex(writer.0.finalize()))
}

fn offline_vlda_sample_content_sha256(sample: &OfflineVldaSample) -> Result<String> {
    let mut writer = Sha256Writer::default();
    serde_json::to_writer(&mut writer, &CanonicalOfflineVldaSample(sample))
        .context("failed to stream the canonical offline VLDA sample")?;
    Ok(crate::lowercase_hex(writer.0.finalize()))
}

fn offline_vlda_report_analysis_seal(report: &OfflineVldaReport) -> Result<[u8; 32]> {
    let mut writer = Sha256Writer::default();
    writer
        .write_all(b"prisoma.offline_vlda.analysis_seal/v1\0")
        .context("failed to initialize the offline VLDA analysis seal")?;
    serde_json::to_writer(&mut writer, report)
        .context("failed to stream the offline VLDA report analysis seal")?;
    Ok(writer.0.finalize().into())
}

fn validate_offline_vlda_report_analysis_seal(report: &OfflineVldaReport) -> Result<()> {
    let recorded = report.analysis_seal.0.as_ref().ok_or_else(|| {
        anyhow!(
            "offline VLDA report lacks its in-process analysis seal; deserialize reports only as read-only evidence and rerun the analysis before publication"
        )
    })?;
    let reconstructed = offline_vlda_report_analysis_seal(report)?;
    ensure!(
        recorded == &reconstructed,
        "offline VLDA report changed after analysis and cannot be published"
    );
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct OfflineVldaAnalysisSeal(Option<[u8; 32]>);

// The seal is publication authority, not serialized report meaning. Keep derived report equality
// aligned with the public JSON evidence while publication checks the token explicitly.
impl PartialEq for OfflineVldaAnalysisSeal {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Write for BoundedJsonBuffer {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        let next_len = self
            .bytes
            .len()
            .checked_add(input.len())
            .ok_or_else(|| std::io::Error::other("JSON output length overflow"))?;
        if next_len > self.limit {
            return Err(std::io::Error::other(
                "JSON output exceeds the configured byte limit",
            ));
        }
        self.bytes
            .try_reserve(input.len())
            .map_err(|_| std::io::Error::other("JSON output allocation failed"))?;
        self.bytes.extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn serialize_pretty_json_bounded<T: Serialize>(value: &T, limit: u64) -> Result<Vec<u8>> {
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let mut output = BoundedJsonBuffer {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer_pretty(&mut output, value)
        .context("failed to serialize bounded pretty JSON")?;
    Ok(output.bytes)
}

fn deserialize_unique_string_map<'de, D, V>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct UniqueStringMapVisitor<V>(PhantomData<V>);

    impl<'de, V> Visitor<'de> for UniqueStringMapVisitor<V>
    where
        V: Deserialize<'de>,
    {
        type Value = BTreeMap<String, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a JSON object with unique string keys")
        }

        fn visit_map<A>(self, mut access: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<String, V>()? {
                if values.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate JSON object key {key:?}"
                    )));
                }
            }
            Ok(values)
        }
    }

    deserializer.deserialize_map(UniqueStringMapVisitor(PhantomData))
}

/// Admission limits for one offline `(V,L,D,A)` dataset and analysis.
///
/// The defaults admit routine committed fixtures while bounding decoded input size and the
/// always-on geometry/baseline work. The high-dimensional stress fixture uses an explicit
/// override. Use the additive `*_with_limits` entry points for other reviewed workloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaResourceLimits {
    /// Maximum bytes in the exact input JSON snapshot.
    pub max_input_bytes: u64,
    /// Maximum number of samples.
    pub max_samples: usize,
    /// Maximum total count of decoded `f64` values across all V/L/D/A vectors.
    pub max_total_axis_scalars: usize,
    /// Maximum total count of decoded map entries, including nested label-object entries.
    pub max_total_metadata_entries: usize,
    /// Maximum total count of nested JSON value nodes under sample labels.
    pub max_total_metadata_json_nodes: usize,
    /// Maximum UTF-8 bytes across identifiers, metadata, label keys/strings, and root strings.
    pub max_total_metadata_utf8_bytes: usize,
    /// Maximum nesting depth of a JSON value under a sample label.
    pub max_metadata_json_depth: usize,
    /// Maximum projected pairwise-distance work. Single-analysis public entry points apply this
    /// ceiling independently. The combined invocation entry point and CLI apply it to the checked
    /// sum of main and optional uncertainty work.
    pub max_pairwise_distance_evaluations: u64,
    /// Maximum projected coordinate contributions across all distance evaluations. This second
    /// ceiling prevents a high-dimensional input from hiding excessive work behind an acceptable
    /// pairwise-distance count.
    pub max_distance_coordinate_evaluations: u64,
    /// Maximum aggregate arithmetic projection for dense PLS and logistic-regression solvers.
    /// This is a complete main-analysis run cap, not a fresh allowance for each solver call.
    pub max_dense_solver_operations: u64,
    /// Maximum aggregate operation projection for fitted quantization and categorical
    /// shared-exclusions PID across the full-data, train-split, and shuffled-control screens.
    pub max_categorical_pid_operations: u64,
}

impl Default for OfflineVldaResourceLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: OFFLINE_DEFAULT_MAX_INPUT_BYTES,
            max_samples: OFFLINE_DEFAULT_MAX_SAMPLES,
            max_total_axis_scalars: OFFLINE_DEFAULT_MAX_AXIS_SCALARS,
            max_total_metadata_entries: OFFLINE_DEFAULT_MAX_METADATA_ENTRIES,
            max_total_metadata_json_nodes: OFFLINE_DEFAULT_MAX_METADATA_JSON_NODES,
            max_total_metadata_utf8_bytes: OFFLINE_DEFAULT_MAX_METADATA_UTF8_BYTES,
            max_metadata_json_depth: OFFLINE_DEFAULT_MAX_METADATA_DEPTH,
            max_pairwise_distance_evaluations: OFFLINE_DEFAULT_MAX_PAIRWISE_DISTANCE_EVALUATIONS,
            max_distance_coordinate_evaluations:
                OFFLINE_DEFAULT_MAX_DISTANCE_COORDINATE_EVALUATIONS,
            max_dense_solver_operations: OFFLINE_DEFAULT_MAX_DENSE_SOLVER_OPERATIONS,
            max_categorical_pid_operations: OFFLINE_DEFAULT_MAX_CATEGORICAL_PID_OPERATIONS,
        }
    }
}

/// Read one strict, bounded resource-limit override from a regular file snapshot.
///
/// Every field is required and must be positive. Unknown fields, symlinks, unstable files,
/// malformed JSON, and files larger than 64 KiB are rejected.
pub fn read_offline_vlda_resource_limits(
    path: impl AsRef<Path>,
) -> Result<OfflineVldaResourceLimits> {
    let path = path.as_ref();
    let snapshot = read_bounded_regular_file(
        path,
        OFFLINE_RESOURCE_LIMITS_MAX_BYTES as u64,
        "offline VLDA resource-limits file",
    )?;
    let limits: OfflineVldaResourceLimits = parse_strict_json(
        snapshot.exact_bytes(OFFLINE_RESOURCE_LIMITS_MAX_BYTES as u64)?,
        &format!("resource-limits file {}", path.display()),
    )?;
    validate_resource_limits(&limits)?;
    snapshot.verify_path()?;
    Ok(limits)
}

fn validate_resource_limits(limits: &OfflineVldaResourceLimits) -> Result<()> {
    for (name, value) in [
        ("max_input_bytes", u128::from(limits.max_input_bytes)),
        ("max_samples", limits.max_samples as u128),
        (
            "max_total_axis_scalars",
            limits.max_total_axis_scalars as u128,
        ),
        (
            "max_total_metadata_entries",
            limits.max_total_metadata_entries as u128,
        ),
        (
            "max_total_metadata_json_nodes",
            limits.max_total_metadata_json_nodes as u128,
        ),
        (
            "max_total_metadata_utf8_bytes",
            limits.max_total_metadata_utf8_bytes as u128,
        ),
        (
            "max_metadata_json_depth",
            limits.max_metadata_json_depth as u128,
        ),
        (
            "max_pairwise_distance_evaluations",
            u128::from(limits.max_pairwise_distance_evaluations),
        ),
        (
            "max_distance_coordinate_evaluations",
            u128::from(limits.max_distance_coordinate_evaluations),
        ),
        (
            "max_dense_solver_operations",
            u128::from(limits.max_dense_solver_operations),
        ),
        (
            "max_categorical_pid_operations",
            u128::from(limits.max_categorical_pid_operations),
        ),
    ] {
        if value == 0 {
            bail!("resource-limits field {name} must be greater than zero; observed 0");
        }
    }
    Ok(())
}

/// Observed decoded size and projected distance work admitted for one harness run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaResourceUsage {
    /// Observed sample count.
    pub samples: usize,
    /// Observed total V/L/D/A scalar count.
    pub total_axis_scalars: usize,
    /// Observed map-entry count, including nested label-object entries.
    pub total_metadata_entries: usize,
    /// Observed nested label JSON-node count.
    pub total_metadata_json_nodes: usize,
    /// Observed UTF-8 byte count for bounded decoded strings.
    pub total_metadata_utf8_bytes: usize,
    /// Deepest observed nested label JSON value.
    pub metadata_json_depth: usize,
    /// Conservative distance-work projection for the main harness analysis.
    pub projected_main_pairwise_distance_evaluations: u64,
    /// Conservative distance-work projection for optional uncertainty analysis.
    pub projected_uncertainty_pairwise_distance_evaluations: u64,
    /// Checked sum of the main and optional uncertainty projections.
    pub projected_total_pairwise_distance_evaluations: u64,
    /// Conservative main-analysis pairwise projection multiplied by the widest possible
    /// V/L/D/A distance vector in this dataset.
    pub projected_main_distance_coordinate_evaluations: u64,
    /// Conservative optional-uncertainty projection at the same maximum vector width.
    pub projected_uncertainty_distance_coordinate_evaluations: u64,
    /// Checked sum of the main and optional uncertainty coordinate projections.
    pub projected_total_distance_coordinate_evaluations: u64,
    /// Conservative aggregate arithmetic projection for every PLS fit, PLS transform, PLS
    /// component-selection fold, and applicable held-out logistic-regression fit in the main run.
    pub projected_dense_solver_operations: u64,
    /// Conservative aggregate fitted-quantization and categorical shared-exclusions work.
    pub projected_categorical_pid_operations: u64,
}

/// PID estimator mode: disabled, continuous shared exclusions, fitted categorical shared
/// exclusions, or PLS followed by fitted categorical shared exclusions.
///
/// Measure identity (grandplan §7.6): the continuous and categorical modes are distinct
/// shared-exclusions constructions. The categorical modes define fitted quantized variables and
/// use the averaged two-source Makkeh-Gutknecht-Wibral functional. The continuous mode uses the
/// Ehrlich construction and its kNN estimator. Never pool results across scientific objects,
/// preprocessing regimes, or estimators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PidMode {
    /// Do not request MI or PID estimates. Geometry and every static factual-outcome
    /// label/prediction baseline still run. The analysis feature still links shared
    /// `pid-core` geometry and logistic code.
    #[default]
    Disabled,
    /// Continuous PID using KSG kNN mutual information and shared-exclusions redundancy.
    Continuous,
    /// Averaged two-source MGW categorical shared-exclusions PID after fitted equal-width
    /// quantization of each axis.
    CategoricalSx,
    /// PLS supervised projection toward `A` followed by fitted categorical MGW PID. This is a
    /// same-row selection-inflation diagnostic, not an inferential escape hatch. Projection is
    /// fitted on the samples given to each screen. The train-split screen therefore fits and
    /// evaluates on train samples only; it does not score held-out categorical rows.
    CategoricalSxPls,
}

/// Options for the offline VLDA harness.
#[derive(Debug, Clone)]
pub struct OfflineVldaHarnessOptions {
    /// Exact PID estimator mode. Generic `discrete` is intentionally not an identity.
    pub pid_mode: PidMode,
    /// Number of fitted bins in a categorical shared-exclusions mode.
    pub categorical_bins: usize,
    /// PLS component selection when `pid_mode == CategoricalSxPls`.
    pub pls: PlsComponentSelection,
}

/// How the number of PLS latent components is chosen in `categorical-sx-pls` mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlsComponentSelection {
    /// A fixed count (the historical `--pls-components N`; default 2).
    Fixed(usize),
    /// Per-source leave-one-out CV Q² selection over `1..=max_components`
    /// (`--pls-components cv[:MAX]`) — the preregistered grandplan §6.2
    /// step 5(d) method, via `pid_core::pls_cv_select_components`. The chosen
    /// counts and their Q² are recorded in the screen's `pls_selection`.
    CvQ2 {
        /// Upper bound on the candidate component counts.
        max_components: usize,
    },
}

/// Per-source PLS component-selection provenance for a `categorical-sx-pls` screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPlsSelection {
    /// `"fixed"` or `"cv_q2"`.
    pub method: String,
    pub components_v: usize,
    pub components_l: usize,
    pub components_d: usize,
    /// LOO-CV Q² at the chosen count (CV mode only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q2_v: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q2_l: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub q2_d: Option<f64>,
}

impl Default for OfflineVldaHarnessOptions {
    fn default() -> Self {
        Self {
            pid_mode: PidMode::Disabled,
            categorical_bins: 10,
            pls: PlsComponentSelection::Fixed(2),
        }
    }
}

fn validate_harness_options(options: &OfflineVldaHarnessOptions) -> Result<()> {
    ensure!(
        options.categorical_bins >= 2,
        "offline VLDA categorical bin count must be at least 2"
    );
    match options.pls {
        PlsComponentSelection::Fixed(components) => {
            ensure!(
                components > 0,
                "offline VLDA fixed PLS component count must be positive"
            );
        }
        PlsComponentSelection::CvQ2 { max_components } => {
            ensure!(
                max_components > 0,
                "offline VLDA PLS CV maximum component count must be positive"
            );
        }
    }
    Ok(())
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_optional_input_binding(
    input_uri: Option<&str>,
    input_sha256: Option<&str>,
) -> Result<()> {
    ensure!(
        input_uri.is_some() == input_sha256.is_some(),
        "offline VLDA input URI and exact-byte SHA-256 must be supplied together"
    );
    if let Some(uri) = input_uri {
        ensure!(
            !uri.trim().is_empty(),
            "offline VLDA input URI must not be empty"
        );
    }
    if let Some(sha256) = input_sha256 {
        ensure!(
            is_lowercase_sha256(sha256),
            "offline VLDA input SHA-256 must be 64 lowercase hexadecimal characters"
        );
    }
    Ok(())
}

/// Declared population support for one `(V,L,D,A)` axis.
///
/// Support is **declared by the capture adapter, never inferred from observed values**
/// (`grandplan.md` §7.14). Exact ties or low observed cardinality can reject a *sample* for a
/// continuous estimator; they never prove the population law is discrete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineVldaDeclaredSupport {
    /// Absolutely continuous, full-dimensional, regular — the only law the continuous
    /// shared-exclusions / KSG estimators accept.
    ContinuousRegularFullDimensional,
    /// Categorical / discrete-valued by construction (e.g. a binary instruction indicator).
    Categorical,
    /// Declared, but neither purely continuous nor purely categorical.
    Mixed,
}

/// Caller-declared joint-law contract for one complete continuous estimator tuple.
///
/// Per-axis continuity does not imply joint absolute continuity or finite mutual information.
/// The caller must therefore declare this stronger contract for each requested MI or PID tuple.
/// The declaration is never inferred from a finite sample and does not clear scientific gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineVldaContinuousTupleSupport {
    /// Every marginal and joint law used by the named estimator call is regular,
    /// full-dimensional, absolutely continuous, and has finite required information quantities.
    RegularFullDimensionalFiniteInformation,
    /// At least one required law contains atomic and continuous components.
    KnownAtomicOrMixed,
    /// At least one required law is quantized.
    KnownQuantized,
    /// At least one required joint law is singular, stratified, fractal, or lower-dimensional.
    KnownSingularOrLowerDimensional,
}

impl OfflineVldaContinuousTupleSupport {
    fn is_regular(self) -> bool {
        matches!(self, Self::RegularFullDimensionalFiniteInformation)
    }
}

const CONTINUOUS_TUPLE_V_A: &str = "v_a";
const CONTINUOUS_TUPLE_L_A: &str = "l_a";
const CONTINUOUS_TUPLE_D_A: &str = "d_a";
const CONTINUOUS_TUPLE_V_L_A: &str = "v_l_a";
const CONTINUOUS_TUPLE_V_D_A: &str = "v_d_a";
const CONTINUOUS_TUPLE_L_D_A: &str = "l_d_a";
const CONTINUOUS_TUPLE_KEYS: [&str; 6] = [
    CONTINUOUS_TUPLE_V_A,
    CONTINUOUS_TUPLE_L_A,
    CONTINUOUS_TUPLE_D_A,
    CONTINUOUS_TUPLE_V_L_A,
    CONTINUOUS_TUPLE_V_D_A,
    CONTINUOUS_TUPLE_L_D_A,
];

fn validate_continuous_support_contract_consistency(dataset: &OfflineVldaDataset) -> Result<()> {
    let tuple_axes: [(&str, &[&str]); 6] = [
        (CONTINUOUS_TUPLE_V_A, &["v", "a"]),
        (CONTINUOUS_TUPLE_L_A, &["l", "a"]),
        (CONTINUOUS_TUPLE_D_A, &["d", "a"]),
        (CONTINUOUS_TUPLE_V_L_A, &["v", "l", "a"]),
        (CONTINUOUS_TUPLE_V_D_A, &["v", "d", "a"]),
        (CONTINUOUS_TUPLE_L_D_A, &["l", "d", "a"]),
    ];
    for (tuple, axes) in tuple_axes {
        if dataset
            .continuous_tuple_support
            .get(tuple)
            .copied()
            .is_some_and(OfflineVldaContinuousTupleSupport::is_regular)
        {
            for axis in axes {
                if dataset
                    .support
                    .get(*axis)
                    .is_some_and(|support| !support.is_continuous())
                {
                    bail!(
                        "offline VLDA continuous tuple {tuple:?} declares a regular full-dimensional law, but axis {axis:?} has an explicitly incompatible support declaration"
                    );
                }
            }
        }
    }

    for (joint_tuple, implied_marginals) in [
        (
            CONTINUOUS_TUPLE_V_L_A,
            [CONTINUOUS_TUPLE_V_A, CONTINUOUS_TUPLE_L_A],
        ),
        (
            CONTINUOUS_TUPLE_V_D_A,
            [CONTINUOUS_TUPLE_V_A, CONTINUOUS_TUPLE_D_A],
        ),
        (
            CONTINUOUS_TUPLE_L_D_A,
            [CONTINUOUS_TUPLE_L_A, CONTINUOUS_TUPLE_D_A],
        ),
    ] {
        if dataset
            .continuous_tuple_support
            .get(joint_tuple)
            .copied()
            .is_some_and(OfflineVldaContinuousTupleSupport::is_regular)
        {
            for marginal in implied_marginals {
                if dataset
                    .continuous_tuple_support
                    .get(marginal)
                    .is_some_and(|support| !support.is_regular())
                {
                    bail!(
                        "offline VLDA continuous tuple {joint_tuple:?} declares every required marginal regular, but the explicit {marginal:?} declaration is incompatible"
                    );
                }
            }
        }
    }
    Ok(())
}

impl OfflineVldaDeclaredSupport {
    fn is_continuous(self) -> bool {
        matches!(self, Self::ContinuousRegularFullDimensional)
    }
}

/// Why a requested estimate was not produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineVldaAbstainReason {
    /// An axis in the tuple is declared categorical/mixed: the continuous shared-exclusions
    /// estimand is not defined for it.
    DeclaredSupportIncompatibleContinuous,
    /// No population support was declared for an axis in the tuple. Fail closed.
    SupportContractUnspecified,
    /// Axis declarations exist, but the complete estimator tuple lacks its stronger joint-law and
    /// finite-information declaration.
    TupleSupportContractUnspecified,
    /// The complete estimator tuple is declared atomic, mixed, quantized, singular, or
    /// lower-dimensional relative to the required continuous reference law.
    DeclaredTupleSupportIncompatibleContinuous,
    /// The observed sample carries exact ties, incompatible with the estimator's ideal i.i.d.,
    /// unrounded continuous-sample conditions. Rejects the *sample*, not the population law.
    ObservedSampleIncompatibleExactTies,
    /// The estimator rejected the k-th-neighbour shell as ambiguous.
    AmbiguousNeighborShell,
    /// Continuous shared exclusions requires equal ambient source dimensions in the current
    /// pid-core review contract. This is an estimator-applicability limit — the small-ball gauge
    /// is only defined for equal ambient dimensions — not a statement about the population law.
    EstimatorRequiresEqualSourceDimensions,
    /// Every requested resampling/permutation statistic for an otherwise preflight-compatible
    /// pair was unavailable. No inferential value is published.
    UncertaintyStatisticsUnavailable,
}

impl OfflineVldaAbstainReason {
    /// Stable reason code. These strings are a data contract — do not rename.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeclaredSupportIncompatibleContinuous => {
                "declared_support_incompatible_continuous"
            }
            Self::SupportContractUnspecified => "support_contract_unspecified",
            Self::TupleSupportContractUnspecified => "tuple_support_contract_unspecified",
            Self::DeclaredTupleSupportIncompatibleContinuous => {
                "declared_tuple_support_incompatible_continuous"
            }
            Self::ObservedSampleIncompatibleExactTies => "observed_sample_incompatible_exact_ties",
            Self::AmbiguousNeighborShell => "ambiguous_neighbor_shell",
            Self::EstimatorRequiresEqualSourceDimensions => {
                "estimator_requires_equal_source_dimensions"
            }
            Self::UncertaintyStatisticsUnavailable => "uncertainty_statistics_unavailable",
        }
    }
}

/// Typed outcome of one requested estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineVldaEstimateStatus {
    /// The caller explicitly disabled this estimator family; no estimate was requested.
    NotRequested,
    /// The implementation produced a diagnostic value. This is a computation status, not a
    /// scientific eligibility verdict; consult `scientific_gates` before interpretation.
    #[serde(alias = "eligible")]
    Produced,
    /// The implementation produced a diagnostic value with a declared numerical or design
    /// warning.
    #[serde(alias = "eligible_with_warning")]
    ProducedWithWarning,
    Abstained,
}

/// Verdict for one of the four independent scientific gates in `grandplan.md` §7.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineVldaScientificGateVerdict {
    /// The gate passed against a versioned, machine-readable support envelope.
    Passed,
    /// The computation relies on a caller declaration that this sample cannot prove.
    Conditional,
    /// This harness did not run the evidence required to decide the gate.
    NotEvaluated,
    /// The gate is known not to pass for interpretation in the current application regime.
    Blocked,
    /// No estimate was requested, so the gate does not apply.
    NotApplicable,
}

/// Population/measure/estimator/application verdicts are separate from computation status.
/// Current offline screens are diagnostics: no committed application-support envelope validates
/// the intended dependent/high-dimensional VLA regime, so `interpretation_allowed` is false even
/// when a numerical value was produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineVldaScientificGates {
    pub population: OfflineVldaScientificGateVerdict,
    pub measure: OfflineVldaScientificGateVerdict,
    pub estimator: OfflineVldaScientificGateVerdict,
    pub application: OfflineVldaScientificGateVerdict,
    pub interpretation_allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_envelope_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
}

fn legacy_scientific_gates() -> OfflineVldaScientificGates {
    OfflineVldaScientificGates {
        population: OfflineVldaScientificGateVerdict::NotEvaluated,
        measure: OfflineVldaScientificGateVerdict::NotEvaluated,
        estimator: OfflineVldaScientificGateVerdict::NotEvaluated,
        application: OfflineVldaScientificGateVerdict::Blocked,
        interpretation_allowed: false,
        support_envelope_version: None,
        reason_code: Some("legacy_artifact_scientific_gates_unrecorded".to_string()),
    }
}

fn legacy_information_units() -> String {
    "nats".to_string()
}

/// Observed-sample evidence for one axis. Evidence only — never a population-support finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineVldaAxisDiagnostics {
    pub axis: String,
    pub rows: usize,
    pub unique_rows: usize,
    pub max_row_multiplicity: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_support: Option<OfflineVldaDeclaredSupport>,
}

/// Eligibility/abstention denominators over every *requested* estimate (`grandplan.md` §7.14:
/// "Report the denominator … Predictive performance among the small easiest subset is not
/// deployment performance.").
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineVldaEstimateDenominators {
    pub requested: usize,
    /// Requests whose caller-declared support is compatible with the selected estimator.
    /// This is not the four-gate scientific eligibility denominator.
    #[serde(default, alias = "support_eligible")]
    pub declared_support_compatible: usize,
    pub preflight_passed: usize,
    pub estimated: usize,
    pub warned: usize,
    pub abstained: usize,
    pub abstained_by_reason: BTreeMap<String, usize>,
}

impl OfflineVldaEstimateDenominators {
    fn record(&mut self, outcome: &OfflineVldaOutcome) {
        match outcome.status {
            OfflineVldaEstimateStatus::NotRequested => {}
            OfflineVldaEstimateStatus::Produced
            | OfflineVldaEstimateStatus::ProducedWithWarning => {
                self.requested += 1;
                if !outcome.axis_diagnostics.is_empty()
                    && outcome
                        .axis_diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.declared_support.is_some())
                {
                    self.declared_support_compatible += 1;
                }
                self.preflight_passed += 1;
                self.estimated += 1;
                if outcome.status == OfflineVldaEstimateStatus::ProducedWithWarning {
                    self.warned += 1;
                }
            }
            OfflineVldaEstimateStatus::Abstained => {
                self.requested += 1;
                // A tuple rejected only by finite-sample preflight was still compatible with the
                // caller-declared population support.
                if let Some(reason) = outcome.reason_code {
                    if matches!(
                        reason,
                        OfflineVldaAbstainReason::ObservedSampleIncompatibleExactTies
                            | OfflineVldaAbstainReason::AmbiguousNeighborShell
                            | OfflineVldaAbstainReason::EstimatorRequiresEqualSourceDimensions
                            | OfflineVldaAbstainReason::UncertaintyStatisticsUnavailable
                    ) {
                        self.declared_support_compatible += 1;
                    }
                    *self
                        .abstained_by_reason
                        .entry(reason.as_str().to_string())
                        .or_insert(0) += 1;
                }
                self.abstained += 1;
            }
        }
    }
}

/// Status/provenance of one requested estimate, shared by scalar-MI and PID-pair outcomes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaOutcome {
    pub status: OfflineVldaEstimateStatus,
    /// The requested functional and derived-variable domain. The estimator has its own field.
    /// Examples include `shannon_mutual_information_on_continuous_tuple` and the full-team MGW
    /// categorical shared-exclusions identity.
    pub measure: String,
    /// Exact estimator revision the value would have come from.
    pub estimator_revision: String,
    /// Unit of every produced numeric information quantity for this outcome.
    /// The current continuous and categorical routes both use natural logarithms.
    #[serde(default = "legacy_information_units")]
    pub information_units: String,
    pub axes: Vec<String>,
    /// Strong joint-law declaration for the exact continuous tuple. Categorical and disabled
    /// outcomes omit it. A missing declaration forces a continuous abstention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_continuous_tuple_support: Option<OfflineVldaContinuousTupleSupport>,
    #[serde(default = "legacy_scientific_gates")]
    pub scientific_gates: OfflineVldaScientificGates,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<OfflineVldaAbstainReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_detail: Option<String>,
    pub axis_diagnostics: Vec<OfflineVldaAxisDiagnostics>,
}

impl OfflineVldaOutcome {
    pub fn abstained(&self) -> bool {
        self.status == OfflineVldaEstimateStatus::Abstained
    }

    fn produced(&self) -> bool {
        matches!(
            self.status,
            OfflineVldaEstimateStatus::Produced | OfflineVldaEstimateStatus::ProducedWithWarning
        )
    }
}

/// A requested scalar mutual-information estimate. `value` is present **only** when produced —
/// there is no numeric placeholder for an abstention.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaMiEstimate {
    #[serde(flatten)]
    pub outcome: OfflineVldaOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaDataset {
    pub run_id: Option<String>,
    pub source: Option<String>,
    pub model: Option<String>,
    pub task: Option<String>,
    /// Declared population support per axis (`"v"`, `"l"`, `"d"`, `"a"`). An axis with no
    /// declaration fails closed as `support_contract_unspecified`.
    #[serde(default, deserialize_with = "deserialize_unique_string_map")]
    pub support: BTreeMap<String, OfflineVldaDeclaredSupport>,
    /// Strong declarations for complete continuous estimator tuples. Canonical keys are
    /// `v_a`, `l_a`, `d_a`, `v_l_a`, `v_d_a`, and `l_d_a`.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "deserialize_unique_string_map"
    )]
    pub continuous_tuple_support: BTreeMap<String, OfflineVldaContinuousTupleSupport>,
    /// Optional producer-side integrity grade. NCP artifacts require a committed,
    /// hash-verified publication receipt and a complete/complete-with-warning grade.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_integrity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication_receipt: Option<String>,
    /// Private authority token created only after the reader verifies the committed NCP
    /// publication chain. The digest binds that authority to the exact canonical in-memory
    /// content, so cloning and then mutating a verified dataset invalidates publication use.
    #[serde(skip)]
    publication_receipt_verified_content_sha256: Option<String>,
    pub samples: Vec<OfflineVldaSample>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OfflineNcpPublicationReceipt {
    schema_version: u32,
    committed: bool,
    dataset_uri: String,
    dataset_sha256: String,
    runlog_uri: String,
    runlog_sha256: String,
    capture_integrity: String,
}

const LEGACY_NCP_TAG: &str = "v0.8.0";
const LEGACY_NCP_REVISION: &str = "2f5bd586d4bb20c90362bb6f5698b7f64057ba4e";
const LEGACY_NCP_WIRE: &str = "0.8";
const LEGACY_NCP_COMPACT_HASH: &str = "d1b50a2d8a265276";

fn has_frozen_legacy_ncp_config(events: &[RunLogEvent]) -> bool {
    let mut configs = events.iter().filter_map(|event| match event {
        RunLogEvent::ConfigLogged { config, .. } => Some(config),
        _ => None,
    });
    let Some(config) = configs.next() else {
        return false;
    };
    if configs.next().is_some() {
        return false;
    }
    config.get("component").and_then(Value::as_str) == Some("ncp-observer")
        && config.pointer("/ncp/tag").and_then(Value::as_str) == Some(LEGACY_NCP_TAG)
        && config.pointer("/ncp/revision").and_then(Value::as_str) == Some(LEGACY_NCP_REVISION)
        && config.pointer("/ncp/wire").and_then(Value::as_str) == Some(LEGACY_NCP_WIRE)
        && config.pointer("/ncp/contract_hash").and_then(Value::as_str)
            == Some(LEGACY_NCP_COMPACT_HASH)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaSample {
    pub sample_id: String,
    pub episode_id: Option<String>,
    pub v: Vec<f64>,
    pub l: Vec<f64>,
    pub d: Vec<f64>,
    pub a: Vec<f64>,
    #[serde(default, deserialize_with = "deserialize_unique_string_map")]
    pub labels: BTreeMap<String, Value>,
    #[serde(default, deserialize_with = "deserialize_unique_string_map")]
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
    /// Requested marginal-MI estimates. A value is present only when produced; an abstained
    /// estimate carries a stable reason code and no numeric placeholder.
    pub mi_v_action: OfflineVldaMiEstimate,
    pub mi_l_action: OfflineVldaMiEstimate,
    pub mi_d_action: OfflineVldaMiEstimate,
    /// `(V,L)→A` aggregates, mirrored from the `VL` pair. Absent when that pair abstained.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mi_vl_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub co_information_v_l_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redundancy_v_l_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_v_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_l_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synergy_v_l_action: Option<f64>,
    /// Eligibility/abstention denominators over every requested estimate (grandplan §7.14).
    #[serde(default)]
    pub estimate_denominators: OfflineVldaEstimateDenominators,
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
    /// the strong learned baseline a diagnostic must beat (grandplan §6.5
    /// baseline hierarchy; §3.8 PID kill rules).
    pub heldout_logreg_vlda_success_accuracy: Option<f64>,
    pub heldout_logreg_vlda_success_balanced_accuracy: Option<f64>,
    pub heldout_logreg_vlda_success_auroc: Option<f64>,
    pub pid_pairs: BTreeMap<String, OfflineVldaPidPairMetrics>,
    /// Exact fitted-variable receipts for V, L, D, and A in a categorical Sx mode.
    #[serde(default)]
    pub categorical_quantization: BTreeMap<String, OfflineVldaQuantizationReceipt>,
    /// `categorical-sx-pls` only. See `OfflineVldaPidScreenMetrics::pls_selection`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pls_selection: Option<OfflineVldaPlsSelection>,
    /// `categorical-sx-pls` only. This is one fixed-seed shuffled-target negative-control draw,
    /// not a null distribution, p-value, bound, or floor. See
    /// `OfflineVldaPidScreenMetrics::pls_shuffled_target_control`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pls_shuffled_target_control: Option<Box<OfflineVldaPidScreenMetrics>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pls_control_seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPidScreenMetrics {
    /// Requested marginal-MI estimates. A value is present only when produced; an abstained
    /// estimate carries a stable reason code and no numeric placeholder.
    pub mi_v_action: OfflineVldaMiEstimate,
    pub mi_l_action: OfflineVldaMiEstimate,
    pub mi_d_action: OfflineVldaMiEstimate,
    /// `(V,L)→A` aggregates, mirrored from the `VL` pair. Absent when that pair abstained.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mi_vl_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub co_information_v_l_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redundancy_v_l_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_v_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_l_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synergy_v_l_action: Option<f64>,
    /// Eligibility/abstention denominators over every requested estimate (grandplan §7.14).
    #[serde(default)]
    pub estimate_denominators: OfflineVldaEstimateDenominators,
    pub pid_pairs: BTreeMap<String, OfflineVldaPidPairMetrics>,
    /// Exact fitted-variable receipts for V, L, D, and A in a categorical Sx mode.
    #[serde(default)]
    pub categorical_quantization: BTreeMap<String, OfflineVldaQuantizationReceipt>,
    /// `categorical-sx-pls` only: how many components each source's projector used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pls_selection: Option<OfflineVldaPlsSelection>,
    /// `categorical-sx-pls` only: one **fixed-seed shuffled-target negative-control draw**. The
    /// identical pipeline runs PLS plus categorical SxPID against a seeded row
    /// shuffle of the target `A` (grandplan §6.2 leakage-safe fitted preprocessing). It perturbs
    /// the observed X↔A pairing, but one draw can retain fixed points or chance alignment. Its
    /// residual atoms combine supervised-selection effects, finite-sample dependence, fitted
    /// quantization, and estimator bias. It is not a null distribution, p-value, lower bound, or
    /// value to subtract. Treat both screens as descriptive. Treat in-sample
    /// `categorical-sx-pls` output as screening-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pls_shuffled_target_control: Option<Box<OfflineVldaPidScreenMetrics>>,
    /// Seed of the control's target shuffle (recorded for reproducibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pls_control_seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPidPairMetrics {
    pub source_1: String,
    pub source_2: String,
    pub target: String,
    /// Status, requested measure, estimator revision, reason code, and observed axis evidence.
    #[serde(flatten)]
    pub outcome: OfflineVldaOutcome,
    /// Atoms and MI terms exist **only** when the estimate was produced. An abstained pair carries
    /// no numeric placeholder — not zero, not NaN.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mi_source_1_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mi_source_2_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mi_joint_action: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub co_information: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redundancy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_source_1: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_source_2: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synergy: Option<f64>,
    /// Informative and misinformative components of categorical shared-exclusions atoms.
    /// Continuous estimators expose only net atoms and therefore leave this field absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categorical_sx_components: Option<OfflineVldaCategoricalSxComponents>,
    /// Fitted-categorical saturation diagnostics (grandplan §7.6). Absent otherwise.
    #[serde(default)]
    pub categorical_saturation: Option<OfflineVldaCategoricalSaturation>,
}

/// The defining signed decomposition of one categorical shared-exclusions atom.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaCategoricalSxAtom {
    pub informative: f64,
    pub misinformative: f64,
    pub net: f64,
}

/// Informative, misinformative, and net atoms for one categorical PID2 result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaCategoricalSxComponents {
    pub redundancy: OfflineVldaCategoricalSxAtom,
    pub unique_source_1: OfflineVldaCategoricalSxAtom,
    pub unique_source_2: OfflineVldaCategoricalSxAtom,
    pub synergy: OfflineVldaCategoricalSxAtom,
}

/// Content-bound identity of one fitted categorical variable.
///
/// The codebook is part of the estimand. This receipt binds its exact edges, inputs, outputs,
/// occupancy, and declared transform without copying a potentially large edge table into every
/// screen. Reconstructing the edge digest also requires the exact bound input bytes and reviewed
/// quantizer implementation. The report alone does not contain the fitted edge table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineVldaQuantizationReceipt {
    pub axis: String,
    pub functional: String,
    pub quantizer: String,
    pub estimator_revision: String,
    /// Unit emitted by the bound categorical information functional.
    pub information_units: String,
    pub fitted_edges_sha256: String,
    pub fitted_edge_count: usize,
    pub training_input_sha256: String,
    pub transform_input_sha256: String,
    pub categorical_output_sha256: String,
    pub out_of_range_policy: String,
    pub scaling_description: String,
    pub samples: usize,
    pub dimensions: usize,
    pub bins_per_dimension: usize,
    /// Decimal text avoids a JSON implementation limit for large `u128` cardinalities.
    pub nominal_joint_cardinality: Option<String>,
    pub observed_joint_cardinality: usize,
    /// Decimal text avoids a JSON implementation limit for large `u128` cardinalities.
    pub empty_joint_cells: Option<String>,
    pub low_count_joint_cells: usize,
    pub minimum_observed_cell_count: usize,
    pub maximum_observed_cell_count: usize,
    pub estimand_statement: String,
}

/// Saturation diagnostics for fitted-categorical PID screens.
///
/// When almost every sample occupies its own joint bin, the empirical law is too sparse for
/// stable application interpretation. Individual MI terms can be zero or large, so the warning
/// does not assert a value near `ln(n)`. A warned pair is not application-interpretable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaCategoricalSaturation {
    pub unique_fraction_source_1: f64,
    pub unique_fraction_source_2: f64,
    pub unique_fraction_target: f64,
    pub unique_fraction_joint: f64,
    /// Exact empirical-PMF occupancy diagnostics returned by the pinned MGW estimator.
    pub empirical_sample_count: usize,
    pub observed_joint_states: usize,
    pub singleton_joint_states: usize,
    pub low_count_joint_states: usize,
    pub minimum_observed_count: usize,
    pub maximum_observed_count: usize,
    pub observed_coverage_indicator: f64,
    pub population_caveat: String,
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

/// Per-axis temporal-dependence diagnostic. Per-step rows are not independent when episodes
/// autocorrelate, while the point estimators do not model that dependence. This report provides a
/// descriptive within-unit-step-run Pearson lag-1 screen only. It is not an effective sample
/// size, a denominator correction, or a valid block-length selector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaTemporalReport {
    /// One entry per axis (V/L/D/A).
    pub variables: BTreeMap<String, OfflineVldaTemporalVariable>,
    /// Number of episode-topology segments before any sequence-index gap splits.
    pub segments: usize,
    /// Number of adjacent within-segment pairs that the episode topology could supply before the
    /// order-receipt check.
    pub potential_lag_pairs: usize,
    /// Number of adjacent within-segment pairs whose canonical `sequence_index` advances by one.
    pub lag_pairs: usize,
    /// Unit-step pairs in runs with at least three pairs. Only these pairs can enter the centered
    /// residual products. A two-pair Pearson correlation is forced to positive or negative one,
    /// so the descriptive screen excludes it.
    pub correlation_lag_pairs: usize,
    /// Adjacent within-segment pairs excluded because canonical `sequence_index` advances by more
    /// than one.
    pub sequence_index_gap_pairs: usize,
    /// `"within_episode"` when every row has an episode id,
    /// `"unidentified_without_episode_ids"` when no row has one, or
    /// `"known_episode_segments_only_mixed_ids"` when missing ids prevent a complete series.
    pub scope: String,
    /// Stable machine-readable warning against inferential reuse.
    pub interpretation: String,
    /// Evidence used to admit the row sequence. A same-episode label is not an order receipt.
    pub ordering_basis: String,
}

/// One axis's temporal diagnostic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaTemporalVariable {
    /// Mean Pearson lag-1 correlation across defined columns. Each contiguous unit-step run's left
    /// and right lag vectors are centered separately before their residual products are pooled.
    /// `None` means that no run has at least three adjacent unit-step pairs, or every column has
    /// zero residual variance on at least one side of the eligible lagged pairs.
    pub lag1_autocorr: Option<f64>,
    /// Number of columns in this axis.
    pub dimensions_total: usize,
    /// Number of columns whose two centered lag-pair vectors both had nonzero variance. The
    /// reported lag-1 mean uses this denominator, not [`Self::dimensions_total`].
    pub dimensions_with_defined_lag1: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaGeometryReport {
    pub space: String,
    pub metric: String,
    pub intrinsic_k: usize,
    pub hyperbolicity_samples: usize,
    /// Descriptive risk flags. These diagnostics never establish or block estimator validity.
    pub diagnostics: OfflineVldaGeometryDiagnostics,
    pub variables: BTreeMap<String, OfflineVldaGeometryVariable>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaGeometryDiagnostics {
    pub status: String,
    pub max_intrinsic_dimension_warning: f64,
    pub min_pairwise_cv_warning: f64,
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
    pub temporal: OfflineVldaTemporalReport,
    pub train_split_pid: Option<OfflineVldaTrainSplitPidReport>,
    pub heldout_split: Option<OfflineVldaHeldoutSplitReport>,
    pub heldout_class_coverage: Option<OfflineVldaHeldoutClassCoverageReport>,
    pub heldout_episode_disjoint: Option<OfflineVldaHeldoutEpisodeDisjointReport>,
    pub heldout_predictions: Vec<OfflineVldaHeldoutPredictionRecord>,
    pub heldout_failure_diagnostics: Vec<OfflineVldaHeldoutFailureDiagnostics>,
    /// Per-axis provenance honesty. This aggregates the provenance markers that each
    /// capture adapter stamps on every sample: `l_source`/`d_source` for an
    /// `ncp-observer` capture, or `{v,l,d,a}_provenance` for a `safe_adapter` capture.
    /// Missing, unrecognized, fabricated, misaligned, and proxy values are degraded.
    /// The list is empty when no sample carries a recognized provenance convention.
    pub axis_provenance: Vec<OfflineVldaAxisProvenance>,
    pub metrics: OfflineVldaMetrics,
    /// Process-local mutation seal over every serialized report field. The field is never
    /// serialized. A deserialized report is read-only evidence, not fresh publication authority.
    #[serde(skip)]
    analysis_seal: OfflineVldaAnalysisSeal,
}

/// Provenance summary for one `(V,L,D,A)` axis, aggregated across all dataset
/// samples. `status` is `"degraded"` when a required marker is missing, unrecognized,
/// or known to describe a proxy, fabricated, absent, or misaligned axis. PID atoms
/// involving that axis are not trustworthy for the affected samples.
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
    pub require_success_labels: bool,
    pub require_heldout_split: bool,
    pub require_heldout_class_coverage: bool,
    pub require_heldout_episode_disjoint: bool,
    /// Fail the run unless an active capture convention gives complete, recognized
    /// provenance for every sample. `ncp-observer` requires `l_source` and `d_source`.
    /// `safe_adapter` requires all four `{v,l,d,a}_provenance` markers. This gate is a
    /// positive mechanical attestation, not semantic validation or authentication.
    /// See [`offline_vlda_axis_provenance_failure_messages`].
    pub require_axis_provenance_honest: bool,
}

/// Optional files and computed sidecar bound into one offline run-log publication.
#[derive(Debug, Clone, Copy, Default)]
pub struct OfflineVldaRunlogArtifacts<'a> {
    pub summary_path: Option<&'a Path>,
    pub input_path: Option<&'a Path>,
    pub uncertainty_path: Option<&'a Path>,
    pub uncertainty: Option<&'a OfflineVldaPidUncertainty>,
}

fn offline_vlda_has_ncp_markers(dataset: &OfflineVldaDataset) -> bool {
    dataset.source.as_deref() == Some("ncp")
        || dataset.capture_integrity.is_some()
        || dataset.publication_receipt.is_some()
        || dataset.samples.iter().any(|sample| {
            sample.metadata.get("source").map(String::as_str) == Some("ncp")
                || sample.metadata.contains_key("l_source")
                || sample.metadata.contains_key("d_source")
        })
}

#[derive(Debug, Default)]
struct OfflineVldaDecodedUsage {
    total_axis_scalars: usize,
    total_metadata_entries: usize,
    total_metadata_json_nodes: usize,
    total_metadata_utf8_bytes: usize,
    metadata_json_depth: usize,
}

fn add_bounded_usize(
    observed: &mut usize,
    additional: usize,
    limit: usize,
    resource: &str,
) -> Result<()> {
    let updated = observed.checked_add(additional).ok_or_else(|| {
        anyhow::anyhow!(
            "offline VLDA resource accounting overflow for {resource}: observed {observed}, additional {additional}, limit {limit}"
        )
    })?;
    if updated > limit {
        bail!(
            "offline VLDA resource limit exceeded for {resource}: observed {updated}, limit {limit}"
        );
    }
    *observed = updated;
    Ok(())
}

fn account_metadata_utf8(
    value: &str,
    usage: &mut OfflineVldaDecodedUsage,
    limits: &OfflineVldaResourceLimits,
) -> Result<()> {
    add_bounded_usize(
        &mut usage.total_metadata_utf8_bytes,
        value.len(),
        limits.max_total_metadata_utf8_bytes,
        "metadata UTF-8 bytes",
    )
}

fn enqueue_metadata_json_node<'a>(
    stack: &mut Vec<(&'a Value, usize)>,
    value: &'a Value,
    depth: usize,
    usage: &mut OfflineVldaDecodedUsage,
    limits: &OfflineVldaResourceLimits,
) -> Result<()> {
    if depth > limits.max_metadata_json_depth {
        bail!(
            "offline VLDA resource limit exceeded for metadata JSON depth: observed {depth}, limit {}",
            limits.max_metadata_json_depth
        );
    }
    add_bounded_usize(
        &mut usage.total_metadata_json_nodes,
        1,
        limits.max_total_metadata_json_nodes,
        "metadata JSON nodes",
    )?;
    usage.metadata_json_depth = usage.metadata_json_depth.max(depth);
    stack.try_reserve(1).map_err(|_| {
        anyhow::anyhow!("offline VLDA metadata traversal scratch allocation failed")
    })?;
    stack.push((value, depth));
    Ok(())
}

fn account_metadata_json_value<'a>(
    value: &'a Value,
    stack: &mut Vec<(&'a Value, usize)>,
    usage: &mut OfflineVldaDecodedUsage,
    limits: &OfflineVldaResourceLimits,
) -> Result<()> {
    debug_assert!(stack.is_empty());
    enqueue_metadata_json_node(stack, value, 1, usage, limits)?;
    while let Some((current, depth)) = stack.pop() {
        match current {
            Value::Array(values) => {
                let child_depth = depth.checked_add(1).ok_or_else(|| {
                    anyhow::anyhow!(
                        "offline VLDA resource accounting overflow for metadata JSON depth: observed {depth}, limit {}",
                        limits.max_metadata_json_depth
                    )
                })?;
                for child in values {
                    enqueue_metadata_json_node(stack, child, child_depth, usage, limits)?;
                }
            }
            Value::Object(values) => {
                let child_depth = depth.checked_add(1).ok_or_else(|| {
                    anyhow::anyhow!(
                        "offline VLDA resource accounting overflow for metadata JSON depth: observed {depth}, limit {}",
                        limits.max_metadata_json_depth
                    )
                })?;
                for (key, child) in values {
                    add_bounded_usize(
                        &mut usage.total_metadata_entries,
                        1,
                        limits.max_total_metadata_entries,
                        "metadata entries",
                    )?;
                    account_metadata_utf8(key, usage, limits)?;
                    enqueue_metadata_json_node(stack, child, child_depth, usage, limits)?;
                }
            }
            Value::String(value) => account_metadata_utf8(value, usage, limits)?,
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
    Ok(())
}

fn account_decoded_resources(
    dataset: &OfflineVldaDataset,
    limits: &OfflineVldaResourceLimits,
) -> Result<OfflineVldaDecodedUsage> {
    let samples = dataset.samples.len();
    if samples > limits.max_samples {
        bail!(
            "offline VLDA resource limit exceeded for samples: observed {samples}, limit {}",
            limits.max_samples
        );
    }

    let mut usage = OfflineVldaDecodedUsage::default();
    let mut metadata_stack = Vec::new();
    for value in [
        dataset.run_id.as_deref(),
        dataset.source.as_deref(),
        dataset.model.as_deref(),
        dataset.task.as_deref(),
        dataset.capture_integrity.as_deref(),
        dataset.publication_receipt.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        account_metadata_utf8(value, &mut usage, limits)?;
    }
    for key in dataset.support.keys() {
        if !matches!(key.as_str(), "v" | "l" | "d" | "a") {
            bail!(
                "offline VLDA support declares unknown axis {key:?}; expected only v, l, d, or a"
            );
        }
        add_bounded_usize(
            &mut usage.total_metadata_entries,
            1,
            limits.max_total_metadata_entries,
            "metadata entries",
        )?;
        account_metadata_utf8(key, &mut usage, limits)?;
    }
    for key in dataset.continuous_tuple_support.keys() {
        if !CONTINUOUS_TUPLE_KEYS.contains(&key.as_str()) {
            bail!(
                "offline VLDA continuous_tuple_support declares unknown tuple {key:?}; expected only v_a, l_a, d_a, v_l_a, v_d_a, or l_d_a"
            );
        }
        add_bounded_usize(
            &mut usage.total_metadata_entries,
            1,
            limits.max_total_metadata_entries,
            "metadata entries",
        )?;
        account_metadata_utf8(key, &mut usage, limits)?;
    }

    for sample in &dataset.samples {
        for axis_len in [
            sample.v.len(),
            sample.l.len(),
            sample.d.len(),
            sample.a.len(),
        ] {
            add_bounded_usize(
                &mut usage.total_axis_scalars,
                axis_len,
                limits.max_total_axis_scalars,
                "axis scalars",
            )?;
        }
        account_metadata_utf8(&sample.sample_id, &mut usage, limits)?;
        if let Some(episode_id) = &sample.episode_id {
            account_metadata_utf8(episode_id, &mut usage, limits)?;
        }
        for (key, value) in &sample.labels {
            add_bounded_usize(
                &mut usage.total_metadata_entries,
                1,
                limits.max_total_metadata_entries,
                "metadata entries",
            )?;
            account_metadata_utf8(key, &mut usage, limits)?;
            account_metadata_json_value(value, &mut metadata_stack, &mut usage, limits)?;
        }
        for (key, value) in &sample.metadata {
            add_bounded_usize(
                &mut usage.total_metadata_entries,
                1,
                limits.max_total_metadata_entries,
                "metadata entries",
            )?;
            account_metadata_utf8(key, &mut usage, limits)?;
            account_metadata_utf8(value, &mut usage, limits)?;
        }
    }
    Ok(usage)
}

fn checked_work_add(left: u128, right: u128, context: &str) -> Result<u128> {
    left.checked_add(right).ok_or_else(|| {
        anyhow::anyhow!("offline VLDA resource projection overflow in {context}: {left} + {right}")
    })
}

fn checked_work_mul(left: u128, right: u128, context: &str) -> Result<u128> {
    left.checked_mul(right).ok_or_else(|| {
        anyhow::anyhow!("offline VLDA resource projection overflow in {context}: {left} * {right}")
    })
}

fn ordered_pair_count(samples: u128, context: &str) -> Result<u128> {
    checked_work_mul(samples, samples.saturating_sub(1), context)
}

fn unordered_pair_count(samples: u128, context: &str) -> Result<u128> {
    Ok(ordered_pair_count(samples, context)? / 2)
}

fn projected_pls_fit_operations(
    samples: u128,
    source_dim: u128,
    target_dim: u128,
    components: u128,
) -> Result<u128> {
    let doubled_dimensions = checked_work_mul(
        2,
        checked_work_add(source_dim, target_dim, "PLS fit dimensions")?,
        "PLS fit doubled dimensions",
    )?;
    let per_iteration = checked_work_mul(samples, doubled_dimensions, "PLS fit iteration")?;
    let iteration_work = checked_work_mul(
        checked_work_mul(
            components,
            OFFLINE_PLS_MAX_ITERATIONS,
            "PLS fit component iterations",
        )?,
        per_iteration,
        "PLS fit iteration work",
    )?;
    let deflation_work = checked_work_mul(
        components,
        checked_work_mul(
            samples,
            checked_work_add(source_dim, target_dim, "PLS deflation dimensions")?,
            "PLS deflation rows",
        )?,
        "PLS deflation work",
    )?;
    checked_work_add(iteration_work, deflation_work, "PLS fit total")
}

fn projected_pls_transform_operations(
    samples: u128,
    source_dim: u128,
    components: u128,
) -> Result<u128> {
    let component_square = checked_work_mul(components, components, "PLS transform k squared")?;
    let rotation = checked_work_add(
        checked_work_mul(component_square, components, "PLS transform QR")?,
        checked_work_mul(source_dim, component_square, "PLS transform rotation")?,
        "PLS transform rotation total",
    )?;
    let retained_output = checked_work_mul(samples, components, "PLS transform output")?;
    let affine_scratch = source_dim
        .checked_add(1)
        .context("offline VLDA resource projection overflow in PLS affine scratch")?;
    let evaluation = checked_work_mul(
        retained_output,
        source_dim,
        "PLS transform affine evaluation",
    )?;
    [rotation, retained_output, affine_scratch, evaluation]
        .into_iter()
        .try_fold(0_u128, |sum, value| {
            checked_work_add(sum, value, "PLS transform total")
        })
}

fn projected_pls_cv_operations(
    samples: u128,
    source_dim: u128,
    target_dim: u128,
    components: u128,
) -> Result<u128> {
    let train_samples = samples
        .checked_sub(1)
        .context("offline VLDA PLS CV requires at least two samples")?;
    let component_sum = checked_work_mul(
        components,
        components
            .checked_add(1)
            .context("offline VLDA resource projection overflow in PLS CV components")?,
        "PLS CV component sum",
    )? / 2;
    let per_component_iteration = checked_work_mul(
        train_samples,
        checked_work_mul(
            2,
            checked_work_add(source_dim, target_dim, "PLS CV dimensions")?,
            "PLS CV doubled dimensions",
        )?,
        "PLS CV per-component iteration",
    )?;
    let fit_work = checked_work_mul(
        checked_work_mul(samples, component_sum, "PLS CV folds and candidates")?,
        checked_work_mul(
            OFFLINE_PLS_MAX_ITERATIONS,
            per_component_iteration,
            "PLS CV fit iterations",
        )?,
        "PLS CV fit work",
    )?;
    let prediction_work = checked_work_mul(
        checked_work_mul(samples, component_sum, "PLS CV prediction folds")?,
        checked_work_mul(source_dim, target_dim.max(1), "PLS CV prediction width")?,
        "PLS CV prediction work",
    )?;
    checked_work_add(fit_work, prediction_work, "PLS CV total")
}

fn projected_pls_axis_operations(
    samples: u128,
    source_dim: u128,
    target_dim: u128,
    selection: PlsComponentSelection,
) -> Result<u128> {
    let components = projected_pls_output_dimension(samples, source_dim, selection)?;
    let selection_work = match selection {
        PlsComponentSelection::Fixed(_) => 0,
        PlsComponentSelection::CvQ2 { .. } => {
            projected_pls_cv_operations(samples, source_dim, target_dim, components)?
        }
    };
    let fit = projected_pls_fit_operations(samples, source_dim, target_dim, components)?;
    let transform = projected_pls_transform_operations(samples, source_dim, components)?;
    checked_work_add(
        selection_work,
        checked_work_add(fit, transform, "PLS fit and transform")?,
        "PLS axis total",
    )
}

fn projected_pls_screen_operations(
    samples: u128,
    dims: &OfflineVldaDims,
    selection: PlsComponentSelection,
) -> Result<u128> {
    let target_dim = dims.a as u128;
    let one_target = [dims.v, dims.l, dims.d]
        .into_iter()
        .try_fold(0_u128, |sum, source_dim| {
            checked_work_add(
                sum,
                projected_pls_axis_operations(samples, source_dim as u128, target_dim, selection)?,
                "PLS screen axes",
            )
        })?;
    // The shuffled-target negative control repeats the complete fitted pipeline.
    checked_work_mul(2, one_target, "PLS screen and shuffled control")
}

fn projected_logistic_operations(train_samples: u128, feature_dim: u128) -> Result<u128> {
    let columns = feature_dim
        .checked_add(1)
        .context("offline VLDA resource projection overflow in logistic intercept")?;
    ensure!(
        columns <= OFFLINE_LOGREG_MAX_SOLVER_COLUMNS,
        "offline VLDA held-out logistic baseline requires {columns} dense-solver columns, above pid-core's {}-column limit",
        OFFLINE_LOGREG_MAX_SOLVER_COLUMNS
    );
    let square = checked_work_mul(columns, columns, "logistic columns squared")?;
    let per_iteration = [
        checked_work_mul(train_samples, square, "logistic row-Hessian work")?,
        checked_work_mul(columns, square, "logistic factorization work")?,
        checked_work_mul(
            10,
            checked_work_mul(train_samples, columns, "logistic dense rows")?,
            "logistic vector work",
        )?,
    ]
    .into_iter()
    .try_fold(0_u128, |sum, value| {
        checked_work_add(sum, value, "logistic iteration total")
    })?;
    checked_work_mul(
        OFFLINE_LOGREG_MAX_ITERATIONS,
        per_iteration,
        "logistic fit total",
    )
}

fn projected_dense_solver_operations(
    dataset: &OfflineVldaDataset,
    options: &OfflineVldaHarnessOptions,
) -> Result<u128> {
    let dims = validate_dataset(dataset)?;
    let mut projected = 0u128;
    if options.pid_mode == PidMode::CategoricalSxPls {
        projected =
            projected_pls_screen_operations(dataset.samples.len() as u128, &dims, options.pls)?;
    }

    if let Some(split) = heldout_split_plan(&dataset.samples) {
        if options.pid_mode == PidMode::CategoricalSxPls {
            projected = checked_work_add(
                projected,
                projected_pls_screen_operations(
                    split.report.train_samples as u128,
                    &dims,
                    options.pls,
                )?,
                "full and train-split PLS screens",
            )?;
        }
        if let Some(labels) = success_labels(&dataset.samples) {
            let mut has_success = false;
            let mut has_failure = false;
            for (label, role) in labels.iter().zip(&split.roles) {
                if *role == OfflineVldaSplitRole::Train {
                    has_success |= *label;
                    has_failure |= !*label;
                }
            }
            if has_success && has_failure {
                let feature_dim = [dims.v, dims.l, dims.d, dims.a]
                    .into_iter()
                    .try_fold(0_u128, |sum, dim| {
                        checked_work_add(sum, dim as u128, "logistic feature width")
                    })?;
                projected = checked_work_add(
                    projected,
                    projected_logistic_operations(split.report.train_samples as u128, feature_dim)?,
                    "PLS and logistic dense-solver work",
                )?;
            }
        }
    }
    Ok(projected)
}

fn enforce_dense_solver_limit(projected: u128, limits: &OfflineVldaResourceLimits) -> Result<u64> {
    if projected > u128::from(limits.max_dense_solver_operations) {
        bail!(
            "offline VLDA resource limit exceeded for aggregate projected dense-solver operations: observed {projected}, limit {}",
            limits.max_dense_solver_operations
        );
    }
    u64::try_from(projected).map_err(|_| {
        anyhow::anyhow!("offline VLDA projected dense-solver operations {projected} do not fit u64")
    })
}

fn dense_solver_budget(limits: &OfflineVldaResourceLimits) -> Result<ResourceBudget> {
    ResourceBudget::new(
        DEFAULT_MAX_BYTES,
        DEFAULT_MAX_PAIRWISE_DISTANCES,
        u128::from(limits.max_dense_solver_operations),
        1,
    )
    .context("failed to construct the offline VLDA dense-solver budget")
}

fn categorical_pid_budget(limits: &OfflineVldaResourceLimits) -> Result<ResourceBudget> {
    ResourceBudget::new(
        DEFAULT_MAX_BYTES,
        DEFAULT_MAX_PAIRWISE_DISTANCES,
        u128::from(limits.max_categorical_pid_operations),
        1,
    )
    .context("failed to construct the offline VLDA categorical-PID budget")
}

fn ceil_log2_u128(value: u128) -> u128 {
    if value <= 1 {
        1
    } else {
        u128::BITS as u128 - (value - 1).leading_zeros() as u128
    }
}

/// Mirrors the pinned pid-core 0.9 two-source averaged SxPID operation estimate.
fn projected_categorical_sxpid2_operations(
    samples: u128,
    source_1_dim: u128,
    source_2_dim: u128,
    target_dim: u128,
) -> Result<u128> {
    let coordinates = checked_work_add(
        checked_work_add(source_1_dim, source_2_dim, "categorical source coordinates")?,
        target_dim,
        "categorical target coordinates",
    )?;
    let event_scans = checked_work_mul(
        checked_work_mul(samples, samples, "categorical SxPID event pairs")?,
        32,
        "two-source SxPID event scans",
    )?;
    let mobius_work = checked_work_mul(samples, 16, "two-source SxPID Mobius inversion")?;
    let histogram_work = checked_work_mul(
        checked_work_mul(3, samples, "two-source SxPID subset histograms")?,
        checked_work_mul(
            ceil_log2_u128(samples),
            coordinates.max(1),
            "categorical histogram comparison width",
        )?,
        "categorical histogram work",
    )?;
    checked_work_add(
        checked_work_add(event_scans, mobius_work, "categorical SxPID core")?,
        histogram_work,
        "categorical SxPID total",
    )
}

fn projected_quantizer_axis_operations(samples: u128, dimension: u128, bins: u128) -> Result<u128> {
    let edges = checked_work_mul(
        dimension,
        bins.checked_add(1)
            .context("quantizer bin-edge count overflow")?,
        "quantizer fitted edges",
    )?;
    let fit = checked_work_add(
        checked_work_mul(
            checked_work_mul(samples, dimension, "quantizer input coordinates")?,
            2,
            "quantizer fit scans",
        )?,
        edges,
        "quantizer fit work",
    )?;
    let transform = checked_work_add(
        checked_work_mul(
            checked_work_mul(samples, dimension, "quantizer transform coordinates")?,
            checked_work_add(
                3,
                checked_work_add(
                    ceil_log2_u128(samples),
                    ceil_log2_u128(bins),
                    "quantizer search logs",
                )?,
                "quantizer per-coordinate work",
            )?,
            "quantizer transform work",
        )?,
        edges,
        "quantizer transform and report",
    )?;
    checked_work_add(fit, transform, "quantizer axis total")
}

fn projected_categorical_pid_screen_operations(
    samples: u128,
    dims: [u128; 4],
    bins: u128,
) -> Result<u128> {
    let quantization = dims.into_iter().try_fold(0u128, |sum, dimension| {
        checked_work_add(
            sum,
            projected_quantizer_axis_operations(samples, dimension, bins)?,
            "categorical screen quantizers",
        )
    })?;
    let [v, l, d, a] = dims;
    let pairs = [(v, l), (v, d), (l, d)]
        .into_iter()
        .try_fold(0u128, |sum, (first, second)| {
            checked_work_add(
                sum,
                projected_categorical_sxpid2_operations(samples, first, second, a)?,
                "categorical screen PID pairs",
            )
        })?;
    // Project-owned work after pid-core returns: collapse row tuples into category IDs, build
    // paired/triple occupancy sets, serialize and hash every fitted-edge receipt, and encode four
    // fixed-size digest sets. This bound is intentionally conservative because hash-table probes
    // are data dependent. It keeps those steps inside the same aggregate admission decision.
    let coordinate_ids = checked_work_mul(
        samples,
        dims.into_iter().try_fold(0u128, |sum, dimension| {
            checked_work_add(sum, dimension, "categorical category-id coordinates")
        })?,
        "categorical category-id work",
    )?;
    let occupancy = checked_work_mul(samples, 32, "categorical occupancy bookkeeping")?;
    let edge_receipts = checked_work_mul(
        dims.into_iter().try_fold(0u128, |sum, dimension| {
            checked_work_add(
                sum,
                checked_work_mul(
                    dimension,
                    bins.checked_add(1)
                        .context("categorical receipt bin-edge overflow")?,
                    "categorical receipt edges",
                )?,
                "categorical receipt edge sum",
            )
        })?,
        32,
        "categorical edge serialization and hashing",
    )?;
    let receipt_digests = 4u128 * 4 * 64;
    let bookkeeping = checked_work_add(
        checked_work_add(coordinate_ids, occupancy, "categorical row bookkeeping")?,
        checked_work_add(
            edge_receipts,
            receipt_digests,
            "categorical receipt bookkeeping",
        )?,
        "categorical project-owned bookkeeping",
    )?;
    checked_work_add(
        checked_work_add(quantization, pairs, "categorical PID estimator screen")?,
        bookkeeping,
        "categorical PID screen",
    )
}

fn projected_pls_output_dimension(
    samples: u128,
    source_dim: u128,
    selection: PlsComponentSelection,
) -> Result<u128> {
    let components = match selection {
        PlsComponentSelection::Fixed(components) => components as u128,
        PlsComponentSelection::CvQ2 { max_components } => (max_components as u128)
            .min(source_dim)
            .min(samples.saturating_sub(1)),
    };
    ensure!(
        components > 0 && components <= source_dim.min(samples.saturating_sub(1)),
        "offline VLDA PLS component count {components} is invalid for {samples} samples and source width {source_dim}"
    );
    ensure!(
        components <= OFFLINE_PLS_MAX_SOLVER_COMPONENTS,
        "offline VLDA PLS component count {components} exceeds pid-core's {}-component dense-solver limit",
        OFFLINE_PLS_MAX_SOLVER_COMPONENTS
    );
    Ok(components)
}

fn projected_categorical_dimensions(
    samples: u128,
    dims: &OfflineVldaDims,
    options: &OfflineVldaHarnessOptions,
) -> Result<[u128; 4]> {
    if options.pid_mode == PidMode::CategoricalSxPls {
        return Ok([
            projected_pls_output_dimension(samples, dims.v as u128, options.pls)?,
            projected_pls_output_dimension(samples, dims.l as u128, options.pls)?,
            projected_pls_output_dimension(samples, dims.d as u128, options.pls)?,
            dims.a as u128,
        ]);
    }
    Ok([dims.v, dims.l, dims.d, dims.a].map(|dimension| dimension as u128))
}

fn projected_categorical_pid_operations(
    dataset: &OfflineVldaDataset,
    options: &OfflineVldaHarnessOptions,
) -> Result<u128> {
    if !matches!(
        options.pid_mode,
        PidMode::CategoricalSx | PidMode::CategoricalSxPls
    ) {
        return Ok(0);
    }
    let dims = validate_dataset(dataset)?;
    let screens_per_scope = if options.pid_mode == PidMode::CategoricalSxPls {
        2
    } else {
        1
    };
    let full_samples = dataset.samples.len() as u128;
    let full_dimensions = projected_categorical_dimensions(full_samples, &dims, options)?;
    let full = checked_work_mul(
        screens_per_scope,
        projected_categorical_pid_screen_operations(
            full_samples,
            full_dimensions,
            options.categorical_bins as u128,
        )?,
        "full-data categorical PID screens",
    )?;
    let Some(split) = heldout_split_plan(&dataset.samples) else {
        return Ok(full);
    };
    let train_samples = split.report.train_samples as u128;
    let train_dimensions = projected_categorical_dimensions(train_samples, &dims, options)?;
    let train = checked_work_mul(
        screens_per_scope,
        projected_categorical_pid_screen_operations(
            train_samples,
            train_dimensions,
            options.categorical_bins as u128,
        )?,
        "train-split categorical PID screens",
    )?;
    checked_work_add(full, train, "aggregate categorical PID screens")
}

fn enforce_categorical_pid_limit(
    projected: u128,
    limits: &OfflineVldaResourceLimits,
) -> Result<u64> {
    if projected > u128::from(limits.max_categorical_pid_operations) {
        bail!(
            "offline VLDA resource limit exceeded for aggregate categorical-PID operations: observed {projected}, limit {}",
            limits.max_categorical_pid_operations
        );
    }
    u64::try_from(projected).map_err(|_| {
        anyhow::anyhow!(
            "offline VLDA projected categorical-PID operations {projected} do not fit u64"
        )
    })
}

fn projected_geometry_distance_evaluations(samples: u128) -> Result<u128> {
    let ordered = ordered_pair_count(samples, "geometry ordered pairs")?;
    let pairwise_passes = checked_work_mul(ordered, 2, "geometry pairwise passes")?;
    let sampled = checked_work_mul(
        OFFLINE_GEOMETRY_HYPERBOLICITY_SAMPLES as u128,
        6,
        "geometry sampled four-point distances",
    )?;
    let per_variable = checked_work_add(
        checked_work_add(pairwise_passes, samples, "geometry row validation")?,
        sampled,
        "geometry sampled work",
    )?;
    checked_work_mul(
        per_variable,
        OFFLINE_GEOMETRY_VARIABLES,
        "geometry variables",
    )
}

fn projected_analysis_distance_evaluations(
    dataset: &OfflineVldaDataset,
    pid_mode: PidMode,
) -> Result<u128> {
    let samples = dataset.samples.len() as u128;
    let unordered = unordered_pair_count(samples, "all-sample unordered pairs")?;
    let mut projected = projected_geometry_distance_evaluations(samples)?;
    let split = heldout_split_diagnostics(dataset);
    let valid_split = split.missing_samples == 0
        && split.unrecognized_samples == 0
        && split.train_samples > 0
        && split.heldout_samples > 0;

    let complete_success_labels = dataset.samples.iter().all(|sample| {
        sample
            .labels
            .get("success")
            .and_then(Value::as_bool)
            .is_some()
    });
    if complete_success_labels {
        projected = checked_work_add(
            projected,
            checked_work_mul(
                OFFLINE_BASELINE_FEATURE_VIEWS,
                unordered,
                "leave-one-out nearest-neighbor baselines",
            )?,
            "leave-one-out nearest-neighbor baselines",
        )?;

        if valid_split {
            let heldout = split.heldout_samples as u128;
            let heldout_centroid = checked_work_mul(
                checked_work_mul(
                    OFFLINE_BASELINE_FEATURE_VIEWS,
                    2,
                    "held-out centroid classes",
                )?,
                heldout,
                "held-out centroid distances",
            )?;
            projected =
                checked_work_add(projected, heldout_centroid, "held-out centroid baselines")?;
        }
    }

    if pid_mode == PidMode::Continuous {
        let all_sample_pid = checked_work_mul(
            OFFLINE_CONTINUOUS_PID_PAIRWISE_PASSES,
            unordered_pair_count(samples, "all-sample continuous PID pairs")?,
            "all-sample continuous PID",
        )?;
        projected = checked_work_add(projected, all_sample_pid, "all-sample continuous PID")?;
        if valid_split {
            let train_pid = checked_work_mul(
                OFFLINE_CONTINUOUS_PID_PAIRWISE_PASSES,
                unordered_pair_count(
                    split.train_samples as u128,
                    "train-split continuous PID pairs",
                )?,
                "train-split continuous PID",
            )?;
            projected = checked_work_add(projected, train_pid, "train-split continuous PID")?;
        }
    }
    Ok(projected)
}

fn projected_uncertainty_distance_evaluations(
    samples: usize,
    config: &OfflineVldaUncertaintyConfig,
) -> Result<u128> {
    let samples = samples as u128;
    let full_pid2 = checked_work_mul(
        OFFLINE_PID2_PAIRWISE_PASSES,
        unordered_pair_count(samples, "uncertainty full-sample PID2 pairs")?,
        "uncertainty full-sample PID2",
    )?;
    let mut per_pair = 0u128;
    if config.n_boot > 0 {
        let subsample_len = (((samples / 2) / config.block_size as u128).max(1))
            .checked_mul(config.block_size as u128)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "offline VLDA pairwise-distance projection overflow in uncertainty subsample length"
                )
            })?;
        let subsample_pid2 = checked_work_mul(
            OFFLINE_PID2_PAIRWISE_PASSES,
            unordered_pair_count(subsample_len, "uncertainty subsample PID2 pairs")?,
            "uncertainty subsample PID2",
        )?;
        let bootstrap_replicates = checked_work_mul(
            config.n_boot as u128,
            subsample_pid2,
            "uncertainty bootstrap replicates",
        )?;
        per_pair = checked_work_add(
            per_pair,
            checked_work_add(
                full_pid2,
                bootstrap_replicates,
                "uncertainty bootstrap point and replicates",
            )?,
            "uncertainty bootstrap",
        )?;
    }
    if config.n_perm > 0 {
        let evaluations_per_permutation_test = checked_work_mul(
            (config.n_perm as u128).checked_add(1).ok_or_else(|| {
                anyhow::anyhow!(
                    "offline VLDA pairwise-distance projection overflow in uncertainty permutation count"
                )
            })?,
            full_pid2,
            "uncertainty permutation evaluations",
        )?;
        per_pair = checked_work_add(
            per_pair,
            checked_work_mul(
                2,
                evaluations_per_permutation_test,
                "uncertainty two-source permutation tests",
            )?,
            "uncertainty permutation tests",
        )?;
    }
    checked_work_mul(3, per_pair, "uncertainty PID pairs")
}

fn enforce_pairwise_distance_limit(
    projected: u128,
    limits: &OfflineVldaResourceLimits,
    scope: &str,
) -> Result<u64> {
    if projected > u128::from(limits.max_pairwise_distance_evaluations) {
        bail!(
            "offline VLDA resource limit exceeded for {scope} projected pairwise distance evaluations: observed {projected}, limit {}",
            limits.max_pairwise_distance_evaluations
        );
    }
    u64::try_from(projected).map_err(|_| {
        anyhow::anyhow!(
            "offline VLDA projected pairwise distance evaluations {projected} do not fit u64"
        )
    })
}

fn maximum_distance_vector_width(dataset: &OfflineVldaDataset) -> Result<u128> {
    let mut maximum = 0u128;
    for sample in &dataset.samples {
        let vl = checked_work_add(
            sample.v.len() as u128,
            sample.l.len() as u128,
            "V/L distance-vector width",
        )?;
        let da = checked_work_add(
            sample.d.len() as u128,
            sample.a.len() as u128,
            "D/A distance-vector width",
        )?;
        maximum = maximum.max(checked_work_add(vl, da, "V/L/D/A distance-vector width")?);
    }
    Ok(maximum.max(1))
}

fn projected_distance_coordinate_evaluations(
    pairwise_distance_evaluations: u128,
    maximum_vector_width: u128,
    context: &str,
) -> Result<u128> {
    checked_work_mul(pairwise_distance_evaluations, maximum_vector_width, context)
}

fn enforce_distance_coordinate_limit(
    projected: u128,
    limits: &OfflineVldaResourceLimits,
    scope: &str,
) -> Result<u64> {
    if projected > u128::from(limits.max_distance_coordinate_evaluations) {
        bail!(
            "offline VLDA resource limit exceeded for {scope} projected distance coordinate evaluations: observed {projected}, limit {}",
            limits.max_distance_coordinate_evaluations
        );
    }
    u64::try_from(projected).map_err(|_| {
        anyhow::anyhow!(
            "offline VLDA projected distance coordinate evaluations {projected} do not fit u64"
        )
    })
}

fn admit_dataset_resources(
    dataset: &OfflineVldaDataset,
    options: Option<&OfflineVldaHarnessOptions>,
    uncertainty_config: Option<&OfflineVldaUncertaintyConfig>,
    limits: &OfflineVldaResourceLimits,
) -> Result<OfflineVldaResourceUsage> {
    validate_resource_limits(limits)?;
    let decoded = account_decoded_resources(dataset, limits)?;
    validate_continuous_support_contract_consistency(dataset)?;
    let projected_main = match options {
        Some(options) => projected_analysis_distance_evaluations(dataset, options.pid_mode)?,
        None => 0,
    };
    let projected_uncertainty = match (options.map(|options| options.pid_mode), uncertainty_config)
    {
        (Some(PidMode::Continuous), Some(config)) if config.enabled() => {
            validate_uncertainty_config_for_samples(config, dataset.samples.len())?;
            if uncertainty_row_topology(&dataset.samples).supports(config) {
                projected_uncertainty_distance_evaluations(dataset.samples.len(), config)?
            } else {
                0
            }
        }
        (_, Some(config)) => {
            validate_uncertainty_config(config)?;
            0
        }
        (_, None) => 0,
    };
    let projected_total = checked_work_add(
        projected_main,
        projected_uncertainty,
        "aggregate main and uncertainty analyses",
    )?;
    let projected_total_pairwise_distance_evaluations = enforce_pairwise_distance_limit(
        projected_total,
        limits,
        if uncertainty_config.is_some() {
            "aggregate invocation"
        } else {
            "main analysis"
        },
    )?;
    let maximum_vector_width = maximum_distance_vector_width(dataset)?;
    let projected_main_coordinates = projected_distance_coordinate_evaluations(
        projected_main,
        maximum_vector_width,
        "main distance coordinate evaluations",
    )?;
    let projected_uncertainty_coordinates = projected_distance_coordinate_evaluations(
        projected_uncertainty,
        maximum_vector_width,
        "uncertainty distance coordinate evaluations",
    )?;
    let projected_total_coordinates = checked_work_add(
        projected_main_coordinates,
        projected_uncertainty_coordinates,
        "aggregate distance coordinate evaluations",
    )?;
    let projected_total_distance_coordinate_evaluations = enforce_distance_coordinate_limit(
        projected_total_coordinates,
        limits,
        if uncertainty_config.is_some() {
            "aggregate invocation"
        } else {
            "main analysis"
        },
    )?;
    let projected_dense_solver_operations = enforce_dense_solver_limit(
        match options {
            Some(options) => projected_dense_solver_operations(dataset, options)?,
            None => 0,
        },
        limits,
    )?;
    let projected_categorical_pid_operations = enforce_categorical_pid_limit(
        match options {
            Some(options) => projected_categorical_pid_operations(dataset, options)?,
            None => 0,
        },
        limits,
    )?;
    let projected_main_pairwise_distance_evaluations =
        u64::try_from(projected_main).map_err(|_| {
            anyhow::anyhow!(
                "offline VLDA main projected pairwise distance evaluations {projected_main} do not fit u64"
            )
        })?;
    let projected_uncertainty_pairwise_distance_evaluations =
        u64::try_from(projected_uncertainty).map_err(|_| {
            anyhow::anyhow!(
                "offline VLDA uncertainty projected pairwise distance evaluations {projected_uncertainty} do not fit u64"
            )
        })?;
    let projected_main_distance_coordinate_evaluations =
        u64::try_from(projected_main_coordinates).map_err(|_| {
            anyhow::anyhow!(
                "offline VLDA main projected distance coordinate evaluations {projected_main_coordinates} do not fit u64"
            )
        })?;
    let projected_uncertainty_distance_coordinate_evaluations =
        u64::try_from(projected_uncertainty_coordinates).map_err(|_| {
            anyhow::anyhow!(
                "offline VLDA uncertainty projected distance coordinate evaluations {projected_uncertainty_coordinates} do not fit u64"
            )
        })?;
    Ok(OfflineVldaResourceUsage {
        samples: dataset.samples.len(),
        total_axis_scalars: decoded.total_axis_scalars,
        total_metadata_entries: decoded.total_metadata_entries,
        total_metadata_json_nodes: decoded.total_metadata_json_nodes,
        total_metadata_utf8_bytes: decoded.total_metadata_utf8_bytes,
        metadata_json_depth: decoded.metadata_json_depth,
        projected_main_pairwise_distance_evaluations,
        projected_uncertainty_pairwise_distance_evaluations,
        projected_total_pairwise_distance_evaluations,
        projected_main_distance_coordinate_evaluations,
        projected_uncertainty_distance_coordinate_evaluations,
        projected_total_distance_coordinate_evaluations,
        projected_dense_solver_operations,
        projected_categorical_pid_operations,
    })
}

pub fn read_offline_vlda_dataset(path: impl AsRef<Path>) -> Result<OfflineVldaDataset> {
    read_offline_vlda_dataset_with_limits(path, &OfflineVldaResourceLimits::default())
}

/// Read and validate one dataset with explicit decoded-resource limits.
pub fn read_offline_vlda_dataset_with_limits(
    path: impl AsRef<Path>,
    limits: &OfflineVldaResourceLimits,
) -> Result<OfflineVldaDataset> {
    Ok(read_offline_vlda_dataset_with_hash_and_limits(path, limits)?.0)
}

/// Read, verify, and parse one immutable input snapshot, returning the SHA-256
/// of the exact bytes used for analysis. Callers must carry this digest into
/// reports/run logs rather than reopening a mutable path for provenance.
pub fn read_offline_vlda_dataset_with_hash(
    path: impl AsRef<Path>,
) -> Result<(OfflineVldaDataset, String)> {
    read_offline_vlda_dataset_with_hash_and_limits(path, &OfflineVldaResourceLimits::default())
}

/// Read one immutable input snapshot under explicit decoded-resource limits.
///
/// The raw file-byte ceiling is enforced before JSON deserialization. Decoded sample, scalar,
/// metadata-node, metadata-depth, and UTF-8 ceilings are enforced before NCP verification or any
/// analysis. The returned digest always identifies the exact parsed bytes. Unix requests
/// `O_NOFOLLOW` and `O_NONBLOCK`. Non-Unix calls fail closed until an equivalent descriptor-bound
/// snapshot exists.
pub fn read_offline_vlda_dataset_with_hash_and_limits(
    path: impl AsRef<Path>,
    limits: &OfflineVldaResourceLimits,
) -> Result<(OfflineVldaDataset, String)> {
    let path = path.as_ref();
    validate_resource_limits(limits)?;
    let mut dataset_snapshot =
        read_bounded_regular_file(path, limits.max_input_bytes, "offline VLDA input")?;
    let dataset_bytes = dataset_snapshot.exact_bytes(limits.max_input_bytes)?;
    let input_sha256 = dataset_snapshot
        .sha256
        .clone()
        .context("exact offline VLDA snapshot must carry a digest")?;
    pid_bridge::validate_strict_json_bytes(dataset_bytes)
        .with_context(|| format!("{} is not strict JSON", path.display()))?;
    let mut dataset: OfflineVldaDataset = serde_json::from_slice(dataset_bytes)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    // The decoded dataset owns all deserialized values. Retain the snapshot
    // identity and digest, but release the duplicate raw JSON before resource
    // admission and any NCP run-log verification.
    drop(dataset_snapshot.bytes.take());
    let _ = admit_dataset_resources(&dataset, None, None, limits)?;
    // Reject malformed sample contracts before opening any receipt or run-log path. NCP
    // verification can read another 64 MiB, so structural validation must remain the cheaper,
    // earlier gate.
    let _ = validate_dataset_structure(&dataset).with_context(|| {
        format!(
            "offline VLDA dataset {} failed structural validation",
            path.display()
        )
    })?;
    if offline_vlda_has_ncp_markers(&dataset) {
        if dataset.source.as_deref() != Some("ncp") {
            bail!("NCP-marked dataset must declare source=\"ncp\"");
        }
        let dataset_run_id = dataset
            .run_id
            .as_deref()
            .filter(|run_id| !run_id.is_empty())
            .ok_or_else(|| anyhow::anyhow!("NCP dataset is missing a nonempty run_id"))?;
        let integrity = dataset
            .capture_integrity
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("NCP dataset is missing its capture_integrity grade"))?;
        if !matches!(integrity, "complete" | "complete_with_warning") {
            bail!("NCP dataset capture_integrity={integrity}; failed captures are diagnostic-only");
        }
        let receipt_uri = dataset
            .publication_receipt
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("NCP dataset is missing its publication receipt URI"))?;
        let receipt_path = Path::new(receipt_uri);
        let mut expected_receipt = path.as_os_str().to_os_string();
        expected_receipt.push(".publication.json");
        let expected_receipt = std::path::PathBuf::from(expected_receipt);
        // Read only the adjacent canonical target. The dataset-supplied URI is
        // checked as a name for that snapshot below, but never selects an input.
        let receipt_snapshot = read_bounded_regular_file(
            &expected_receipt,
            OFFLINE_NCP_PUBLICATION_RECEIPT_MAX_BYTES as u64,
            "NCP publication receipt",
        )?;
        if !receipt_snapshot.same_file_as(receipt_path, "declared NCP publication receipt")?
            || std::fs::canonicalize(receipt_path).ok()
                != std::fs::canonicalize(&expected_receipt).ok()
        {
            bail!("NCP publication receipt must be the adjacent regular .publication.json file");
        }
        let receipt_bytes =
            receipt_snapshot.exact_bytes(OFFLINE_NCP_PUBLICATION_RECEIPT_MAX_BYTES as u64)?;
        pid_bridge::validate_strict_json_bytes(receipt_bytes).with_context(|| {
            format!(
                "NCP publication receipt {} is not strict JSON",
                receipt_path.display()
            )
        })?;
        let receipt: OfflineNcpPublicationReceipt = serde_json::from_slice(receipt_bytes)
            .with_context(|| {
                format!(
                    "failed to parse NCP publication receipt {}",
                    receipt_path.display()
                )
            })?;
        if receipt.schema_version != 1 || !receipt.committed {
            bail!("NCP publication receipt is not a committed schema-1 receipt");
        }
        if receipt.capture_integrity != integrity {
            bail!("NCP publication receipt capture grade does not match the dataset");
        }
        if input_sha256 != receipt.dataset_sha256 {
            bail!("NCP publication receipt dataset SHA-256 mismatch");
        }
        dataset_snapshot.verify_path()?;
        let canonical_dataset = std::fs::canonicalize(path)
            .with_context(|| format!("failed to canonicalize {}", path.display()))?;
        let receipt_dataset = std::fs::canonicalize(&receipt.dataset_uri).with_context(|| {
            format!(
                "failed to canonicalize receipt dataset {}",
                receipt.dataset_uri
            )
        })?;
        if canonical_dataset != receipt_dataset {
            bail!("NCP publication receipt names a different dataset path");
        }
        let runlog_path = Path::new(&receipt.runlog_uri);
        let mut runlog_snapshot = read_bounded_regular_file(
            runlog_path,
            OFFLINE_NCP_RUNLOG_MAX_BYTES as u64,
            "NCP canonical run log",
        )?;
        let runlog_bytes = runlog_snapshot.exact_bytes(OFFLINE_NCP_RUNLOG_MAX_BYTES as u64)?;
        if runlog_snapshot.sha256.as_deref() != Some(receipt.runlog_sha256.as_str()) {
            bail!("NCP publication receipt run-log SHA-256 mismatch");
        }
        validate_strict_json_lines(runlog_bytes, "NCP canonical run log")?;
        let events = pid_runlog::read_events(std::io::BufReader::new(runlog_bytes))
            .context("failed to parse the NCP canonical run log")?;
        // Parsed events own the values needed below. Release the duplicate raw
        // run-log bytes before validation and artifact binding.
        drop(runlog_snapshot.bytes.take());
        let validation = pid_runlog::validate_events(&events)
            .context("failed to validate the NCP canonical run log")?;
        if validation.errors > 0 {
            bail!("NCP publication receipt points to an invalid canonical run log");
        }
        if !has_frozen_legacy_ncp_config(&events) {
            bail!(
                "NCP schema-1 publication receipt does not bind the frozen {LEGACY_NCP_TAG} wire {LEGACY_NCP_WIRE} contract identity"
            );
        }
        if !events.iter().any(|event| {
            matches!(event, RunLogEvent::RunStarted { run_id, .. } if run_id == dataset_run_id)
        }) {
            bail!("NCP dataset run_id is not the canonical run-log run_id");
        }
        let artifact_bound = events.iter().any(|event| match event {
            RunLogEvent::ArtifactLogged {
                name,
                kind,
                uri,
                sha256: Some(hash),
                metadata,
                ..
            } => {
                name == "ncp_vlda_dataset"
                    && kind == "dataset_json"
                    && hash == &receipt.dataset_sha256
                    && std::fs::canonicalize(uri).ok().as_ref() == Some(&canonical_dataset)
                    && metadata.get("capture_integrity").map(String::as_str) == Some(integrity)
            }
            _ => false,
        });
        if !artifact_bound {
            bail!("NCP canonical run log does not bind the committed dataset artifact");
        }
        let summary = pid_runlog::summarize_events(&events)
            .context("failed to summarize the NCP canonical run log")?;
        if summary.status != Some(RunStatus::Succeeded) {
            bail!("NCP canonical run log did not end successfully");
        }
        receipt_snapshot.verify_path()?;
        runlog_snapshot.verify_path()?;
    }
    dataset_snapshot.verify_path()?;
    if offline_vlda_has_ncp_markers(&dataset) {
        dataset.publication_receipt_verified_content_sha256 =
            Some(offline_vlda_dataset_content_sha256(&dataset)?);
    }
    validate_dataset_publication_eligibility(&dataset).with_context(|| {
        format!(
            "offline VLDA dataset {} failed publication validation",
            path.display()
        )
    })?;
    Ok((dataset, input_sha256))
}

pub fn run_offline_vlda_harness(
    dataset: OfflineVldaDataset,
    input_uri: Option<String>,
    input_sha256: Option<String>,
) -> Result<OfflineVldaReport> {
    run_offline_vlda_harness_with_options_and_limits(
        dataset,
        input_uri,
        input_sha256,
        &OfflineVldaHarnessOptions::default(),
        &OfflineVldaResourceLimits::default(),
    )
}

/// Run the offline VLDA harness with explicit resource limits and default analysis options.
pub fn run_offline_vlda_harness_with_limits(
    dataset: OfflineVldaDataset,
    input_uri: Option<String>,
    input_sha256: Option<String>,
    limits: &OfflineVldaResourceLimits,
) -> Result<OfflineVldaReport> {
    run_offline_vlda_harness_with_options_and_limits(
        dataset,
        input_uri,
        input_sha256,
        &OfflineVldaHarnessOptions::default(),
        limits,
    )
}

/// Run the offline VLDA harness with explicit options (PID mode, bin count, etc.).
pub fn run_offline_vlda_harness_with_options(
    dataset: OfflineVldaDataset,
    input_uri: Option<String>,
    input_sha256: Option<String>,
    options: &OfflineVldaHarnessOptions,
) -> Result<OfflineVldaReport> {
    run_offline_vlda_harness_with_options_and_limits(
        dataset,
        input_uri,
        input_sha256,
        options,
        &OfflineVldaResourceLimits::default(),
    )
}

/// Run the offline VLDA harness with explicit analysis options and resource limits.
///
/// All decoded-size, pairwise-distance, and coordinate-work checks complete before matrix
/// preparation, geometry, nearest-neighbor baselines, or PID estimation begins.
pub fn run_offline_vlda_harness_with_options_and_limits(
    dataset: OfflineVldaDataset,
    input_uri: Option<String>,
    input_sha256: Option<String>,
    options: &OfflineVldaHarnessOptions,
    limits: &OfflineVldaResourceLimits,
) -> Result<OfflineVldaReport> {
    validate_harness_options(options)?;
    validate_optional_input_binding(input_uri.as_deref(), input_sha256.as_deref())?;
    let resource_usage = admit_dataset_resources(&dataset, Some(options), None, limits)?;
    let dataset_content_sha256 = offline_vlda_dataset_content_sha256(&dataset)
        .context("failed to hash the admitted offline VLDA dataset")?;
    run_offline_vlda_harness_after_resource_preflight(
        &dataset,
        PreflightedOfflineVldaRun {
            dataset_content_sha256,
            input_uri,
            input_sha256,
            options,
            limits,
            resource_usage,
            uncertainty_config: None,
        },
    )
}

/// Run the main harness after preflighting one aggregate main-plus-uncertainty invocation.
///
/// This additive entry point is the CLI contract. It checked-adds both projections against one
/// ceiling before matrix preparation, geometry, baselines, PID, or uncertainty work begins.
pub fn run_offline_vlda_harness_with_options_and_invocation_limits(
    dataset: OfflineVldaDataset,
    input_uri: Option<String>,
    input_sha256: Option<String>,
    options: &OfflineVldaHarnessOptions,
    uncertainty_config: &OfflineVldaUncertaintyConfig,
    limits: &OfflineVldaResourceLimits,
) -> Result<OfflineVldaReport> {
    run_offline_vlda_harness_borrowed_with_options_and_invocation_limits(
        &dataset,
        input_uri,
        input_sha256,
        options,
        uncertainty_config,
        limits,
    )
}

/// Run one aggregate main-plus-uncertainty invocation without taking ownership of the dataset.
///
/// This entry point avoids a full dataset clone when a caller must retain the admitted samples
/// for uncertainty computation or run-log publication. It applies the same preflight and report
/// contract as [`run_offline_vlda_harness_with_options_and_invocation_limits`].
pub fn run_offline_vlda_harness_borrowed_with_options_and_invocation_limits(
    dataset: &OfflineVldaDataset,
    input_uri: Option<String>,
    input_sha256: Option<String>,
    options: &OfflineVldaHarnessOptions,
    uncertainty_config: &OfflineVldaUncertaintyConfig,
    limits: &OfflineVldaResourceLimits,
) -> Result<OfflineVldaReport> {
    validate_harness_options(options)?;
    validate_optional_input_binding(input_uri.as_deref(), input_sha256.as_deref())?;
    let resource_usage =
        admit_dataset_resources(dataset, Some(options), Some(uncertainty_config), limits)?;
    let dataset_content_sha256 = offline_vlda_dataset_content_sha256(dataset)
        .context("failed to hash the admitted offline VLDA dataset")?;
    run_offline_vlda_harness_after_resource_preflight(
        dataset,
        PreflightedOfflineVldaRun {
            dataset_content_sha256,
            input_uri,
            input_sha256,
            options,
            limits,
            resource_usage,
            uncertainty_config: Some(uncertainty_config),
        },
    )
}

/// Main report and optional uncertainty sidecar from one admitted CLI-style invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct OfflineVldaInvocationResult {
    pub report: OfflineVldaReport,
    pub uncertainty: Option<OfflineVldaPidUncertainty>,
}

/// Run one aggregate main-plus-uncertainty invocation with one admission pass and one initial
/// dataset content hash.
///
/// Publication still recomputes the content hash as an independent trust boundary. This function
/// removes duplicate preflight and canonicalization work between the main and uncertainty phases.
pub fn run_offline_vlda_invocation_borrowed_with_options_and_limits(
    dataset: &OfflineVldaDataset,
    input_uri: Option<String>,
    input_sha256: Option<String>,
    options: &OfflineVldaHarnessOptions,
    uncertainty_config: &OfflineVldaUncertaintyConfig,
    limits: &OfflineVldaResourceLimits,
) -> Result<OfflineVldaInvocationResult> {
    validate_harness_options(options)?;
    validate_optional_input_binding(input_uri.as_deref(), input_sha256.as_deref())?;
    let resource_usage =
        admit_dataset_resources(dataset, Some(options), Some(uncertainty_config), limits)?;
    let dataset_content_sha256 = offline_vlda_dataset_content_sha256(dataset)
        .context("failed to hash the admitted offline VLDA dataset")?;
    let report = run_offline_vlda_harness_after_resource_preflight(
        dataset,
        PreflightedOfflineVldaRun {
            dataset_content_sha256: dataset_content_sha256.clone(),
            input_uri,
            input_sha256,
            options,
            limits,
            resource_usage,
            uncertainty_config: Some(uncertainty_config),
        },
    )?;
    let uncertainty = if uncertainty_config.enabled() {
        Some(compute_offline_pid_uncertainty_after_resource_preflight(
            dataset,
            options.pid_mode,
            uncertainty_config,
            limits,
            dataset_content_sha256,
        )?)
    } else {
        None
    };
    Ok(OfflineVldaInvocationResult {
        report,
        uncertainty,
    })
}

struct OfflineVldaMetricPipelineInputs<'a> {
    preprocessing: &'a OfflineVldaPreprocessingReport,
    geometry: &'a OfflineVldaGeometryReport,
    temporal: &'a OfflineVldaTemporalReport,
    train_split_pid: Option<&'a OfflineVldaTrainSplitPidReport>,
    heldout_split: Option<&'a OfflineVldaHeldoutSplitReport>,
    heldout_class_coverage: Option<&'a OfflineVldaHeldoutClassCoverageReport>,
    heldout_episode_disjoint: Option<&'a OfflineVldaHeldoutEpisodeDisjointReport>,
}

fn offline_vlda_metric_pipeline_config(
    options: &OfflineVldaHarnessOptions,
    inputs: OfflineVldaMetricPipelineInputs<'_>,
) -> Value {
    json!({
        "mi_functional": match options.pid_mode {
            PidMode::Disabled => "not_requested",
            PidMode::Continuous => MEASURE_CONTINUOUS_MI,
            PidMode::CategoricalSx | PidMode::CategoricalSxPls => MEASURE_CATEGORICAL_MI,
        },
        "mi_estimator": match options.pid_mode {
            PidMode::Disabled => "not_applicable",
            PidMode::Continuous => ESTIMATOR_CONTINUOUS_MI,
            PidMode::CategoricalSx | PidMode::CategoricalSxPls => ESTIMATOR_CATEGORICAL_MI,
        },
        "pid_functional": match options.pid_mode {
            PidMode::Disabled => "not_requested",
            PidMode::Continuous => MEASURE_CONTINUOUS_PID2,
            PidMode::CategoricalSx | PidMode::CategoricalSxPls => MEASURE_CATEGORICAL_PID2,
        },
        "pid_estimator": match options.pid_mode {
            PidMode::Disabled => "not_applicable",
            PidMode::Continuous => ESTIMATOR_CONTINUOUS_PID2,
            PidMode::CategoricalSx | PidMode::CategoricalSxPls => ESTIMATOR_CATEGORICAL_PID2,
        },
        "pid_mode": options.pid_mode,
        "pid_evaluation_relation": match options.pid_mode {
            PidMode::Disabled => "not_applicable",
            PidMode::Continuous => "same_rows_descriptive_point_estimation",
            PidMode::CategoricalSx => "same_rows_fitted_quantization_descriptive",
            PidMode::CategoricalSxPls => {
                "same_rows_target_supervised_projection_and_fitted_quantization_warning"
            }
        },
        "categorical_bins": options.categorical_bins,
        "pls_components": match options.pls {
            PlsComponentSelection::Fixed(k) => json!(k),
            PlsComponentSelection::CvQ2 { max_components } => {
                json!({"cv_max": max_components})
            }
        },
        "pid_pairs": if options.pid_mode == PidMode::Disabled {
            json!([])
        } else {
            json!([["V", "L"], ["V", "D"], ["L", "D"]])
        },
        "pid_sample_scopes": if options.pid_mode == PidMode::Disabled {
            Vec::<&str>::new()
        } else if inputs
            .train_split_pid
            .and_then(|report| report.metrics.as_ref())
            .is_some()
        {
            vec!["all_samples", "metadata_split_train"]
        } else {
            vec!["all_samples"]
        },
        "target": "A",
        "shared_source_metrics": if options.pid_mode == PidMode::Disabled {
            Vec::<&str>::new()
        } else {
            vec!["mi_v_action", "mi_l_action", "mi_d_action"]
        },
        "preprocessing": {
            "pid_geometry_space": inputs.preprocessing.strategy,
            "standardizer": "per_variable_center_scale_population_std",
            "full_sample_pid_fit_scope": "all_samples",
            "train_split_pid_fit_scope": inputs.train_split_pid
                .and_then(|report| report.metrics.as_ref())
                .map(|_| "metadata_split_train")
        },
        "continuous_support_contract": if options.pid_mode == PidMode::Continuous {
            "each_complete_mi_or_pid_tuple_requires_a_caller_declared_regular_full_dimensional_finite_information_joint_law"
        } else {
            "not_applicable"
        },
        "geometry": {
            "role": "descriptive_diagnostics_not_validity_gate",
            "metric": inputs.geometry.metric,
            "intrinsic_k": inputs.geometry.intrinsic_k,
            "hyperbolicity_samples": inputs.geometry.hyperbolicity_samples,
            "max_intrinsic_dimension_warning": OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION_WARNING,
            "min_pairwise_cv_warning": OFFLINE_GEOMETRY_MIN_PAIRWISE_CV_WARNING
        },
        "temporal": {
            "role": "descriptive_not_estimator_effective_sample_size_or_block_selector",
            "statistic": "within_unit_step_run_pearson_lag1",
            "minimum_lag_pairs_per_run": 3,
            "episode_identity_required": true,
            "strict_canonical_sequence_index_required": true,
            "gap_policy": "split_run_and_count_excluded_pair",
            "axis_aggregation": "mean_over_defined_columns_only",
            "scope": inputs.temporal.scope,
            "ordering_basis": inputs.temporal.ordering_basis
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
        "heldout_split": inputs.heldout_split,
        "train_split_pid": inputs.train_split_pid.map(|report| json!({
            "status": report.status,
            "split_metadata_key": report.split_metadata_key,
            "split": report.split,
            "samples": report.samples,
            "heldout_samples_excluded": report.heldout_samples_excluded,
            "preprocessing_available": report.preprocessing.is_some(),
            "metrics_available": report.metrics.is_some()
        })),
        "heldout_class_coverage": inputs.heldout_class_coverage,
        "heldout_episode_disjoint": inputs.heldout_episode_disjoint,
        "prediction_records": [
            "heldout_train_split_majority",
            "heldout_train_split_1nn",
            "heldout_train_split_nearest_centroid",
            "heldout_train_split_logreg"
        ],
        "negative_handling": "allow"
    })
}

struct PreflightedOfflineVldaRun<'a> {
    dataset_content_sha256: String,
    input_uri: Option<String>,
    input_sha256: Option<String>,
    options: &'a OfflineVldaHarnessOptions,
    limits: &'a OfflineVldaResourceLimits,
    resource_usage: OfflineVldaResourceUsage,
    uncertainty_config: Option<&'a OfflineVldaUncertaintyConfig>,
}

fn run_offline_vlda_harness_after_resource_preflight(
    dataset: &OfflineVldaDataset,
    preflight: PreflightedOfflineVldaRun<'_>,
) -> Result<OfflineVldaReport> {
    let PreflightedOfflineVldaRun {
        dataset_content_sha256,
        input_uri,
        input_sha256,
        options,
        limits,
        resource_usage,
        uncertainty_config,
    } = preflight;
    validate_optional_input_binding(input_uri.as_deref(), input_sha256.as_deref())?;
    let aggregate_invocation = uncertainty_config.is_some();
    let row_topology = uncertainty_row_topology(&dataset.samples);
    let uncertainty_request = match uncertainty_config {
        Some(config) => json!({
            "enabled": config.enabled(),
            "preprocessing_resampling": OFFLINE_UNCERTAINTY_PREPROCESSING_RESAMPLING,
            "n_boot": config.n_boot,
            "n_perm": config.n_perm,
            "block_size": config.block_size,
            "alpha": config.alpha,
            "seed": config.seed,
            "permutation_scheme": if config.n_perm > 0 {
                permutation_scheme_label(config.permutation_scheme)?
            } else {
                "not_requested".to_string()
            },
            "permutation_calibration": permutation_calibration_label(
                config.permutation_scheme,
                config.n_perm,
            )?,
            "row_topology": row_topology.label(),
            "execution": uncertainty_execution_label(options.pid_mode, config, row_topology),
        }),
        None => json!({
            "enabled": false,
            "scope": "not_requested_by_this_api",
        }),
    };
    let dims = validate_dataset(dataset)?;
    let label_counts = label_counts(&dataset.samples);
    let analysis = compute_analysis(
        &dataset.samples,
        &dataset.support,
        &dataset.continuous_tuple_support,
        &dims,
        options,
        dense_solver_budget(limits)?,
        categorical_pid_budget(limits)?,
    )?;
    let run_id = dataset
        .run_id
        .clone()
        .unwrap_or_else(|| "offline-vlda-run".to_string());
    let config = json!({
        "harness": "offline_vlda",
        "report_schema": OFFLINE_VLDA_REPORT_SCHEMA,
        "source": &dataset.source,
        "model": &dataset.model,
        "task": &dataset.task,
        "continuous_tuple_support": &dataset.continuous_tuple_support,
        "input_uri": input_uri,
        "input_sha256": input_sha256,
        "dataset_content_sha256": dataset_content_sha256,
        "dims": dims,
        "samples": dataset.samples.len(),
        "resource_limits": limits,
        "resource_usage": resource_usage,
        "uncertainty_request": uncertainty_request,
        "resource_accounting": {
            "pairwise_limit_scope": if aggregate_invocation {
                "aggregate_main_and_optional_uncertainty"
            } else {
                "single_main_analysis_call"
            },
            "resource_usage_scope": if aggregate_invocation {
                "complete_cli_invocation_projection"
            } else {
                "main_harness_analysis"
            },
            "distance_projection_model": "pairwise_units_times_max_combined_axis_width_v2",
            "dense_solver_projection_model": "pid_core_0_9_logreg_pls_worst_case_v1",
            "categorical_pid_projection_model": "pid_core_0_9_fitted_quantization_and_two_source_averaged_sxpid_v1",
            "optional_uncertainty": if aggregate_invocation {
                "included_or_typed_skip_in_aggregate_preflight"
            } else {
                "not_included_by_single_analysis_api"
            }
        },
        "metric_pipeline": offline_vlda_metric_pipeline_config(
            options,
            OfflineVldaMetricPipelineInputs {
                preprocessing: &analysis.preprocessing,
                geometry: &analysis.geometry,
                temporal: &analysis.temporal,
                train_split_pid: analysis.train_split_pid.as_ref(),
                heldout_split: analysis.heldout_split.as_ref(),
                heldout_class_coverage: analysis.heldout_class_coverage.as_ref(),
                heldout_episode_disjoint: analysis.heldout_episode_disjoint.as_ref(),
            },
        )
    });
    let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
    let mut report = OfflineVldaReport {
        run_id,
        config_hash,
        config,
        dims,
        label_counts,
        preprocessing: analysis.preprocessing,
        geometry: analysis.geometry,
        temporal: analysis.temporal,
        train_split_pid: analysis.train_split_pid,
        heldout_split: analysis.heldout_split,
        heldout_class_coverage: analysis.heldout_class_coverage,
        heldout_episode_disjoint: analysis.heldout_episode_disjoint,
        heldout_predictions: analysis.heldout_predictions,
        heldout_failure_diagnostics: analysis.heldout_failure_diagnostics,
        axis_provenance: axis_provenance(&dataset.samples),
        metrics: analysis.metrics,
        analysis_seal: OfflineVldaAnalysisSeal::default(),
    };
    report.analysis_seal =
        OfflineVldaAnalysisSeal(Some(offline_vlda_report_analysis_seal(&report)?));
    Ok(report)
}

fn train_split_pid_report(
    samples: &[OfflineVldaSample],
    dims: &OfflineVldaDims,
    split: &OfflineVldaHeldoutSplitPlan,
    contract: OfflineVldaPidScreenContract<'_>,
) -> OfflineVldaTrainSplitPidReport {
    let train_samples = split.report.train_samples;
    let train_dims = OfflineVldaDims {
        samples: train_samples,
        v: dims.v,
        l: dims.l,
        d: dims.d,
        a: dims.a,
    };
    if contract.options.pid_mode == PidMode::Disabled {
        return OfflineVldaTrainSplitPidReport {
            split_metadata_key: split.report.metadata_key.clone(),
            split: "metadata_split_train".to_string(),
            train_values: split.report.train_values.clone(),
            heldout_values: split.report.heldout_values.clone(),
            status: "disabled".to_string(),
            samples: train_samples,
            heldout_samples_excluded: split.report.heldout_samples,
            train_sample_ids: split.report.train_sample_ids.clone(),
            preprocessing: None,
            metrics: None,
            error: None,
        };
    }
    let result = (|| -> Result<(OfflineVldaPreprocessingReport, OfflineVldaPidScreenMetrics)> {
        let prepared =
            prepare_standardized_embeddings_for_train(samples, &split.roles, &train_dims)?;
        let metrics = compute_pid_screen_metrics_with_control(&prepared, contract)?;
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
        samples: train_samples,
        heldout_samples_excluded: split.report.heldout_samples,
        train_sample_ids: split.report.train_sample_ids.clone(),
        preprocessing,
        metrics,
        error,
    }
}

#[derive(Clone, Copy)]
struct OfflineVldaPidResourceBudgets {
    dense_solver: ResourceBudget,
    categorical_pid: ResourceBudget,
}

#[derive(Clone, Copy)]
struct OfflineVldaPidScreenContract<'a> {
    support: &'a BTreeMap<String, OfflineVldaDeclaredSupport>,
    continuous_tuple_support: &'a BTreeMap<String, OfflineVldaContinuousTupleSupport>,
    options: &'a OfflineVldaHarnessOptions,
    budgets: OfflineVldaPidResourceBudgets,
}

#[derive(Clone, Copy)]
struct OfflineVldaPidMatrices<'a> {
    v: MatRef<'a>,
    l: MatRef<'a>,
    d: MatRef<'a>,
    a: MatRef<'a>,
}

fn compute_pid_screen_metrics(
    matrices: OfflineVldaPidMatrices<'_>,
    contract: OfflineVldaPidScreenContract<'_>,
) -> Result<OfflineVldaPidScreenMetrics> {
    let OfflineVldaPidMatrices { v, l, d, a } = matrices;
    let OfflineVldaPidScreenContract {
        support,
        continuous_tuple_support,
        options,
        budgets,
    } = contract;
    let pid_mode = options.pid_mode;
    let categorical_bins = options.categorical_bins;
    let pls = options.pls;
    if pid_mode == PidMode::Disabled {
        return Ok(disabled_pid_screen_metrics());
    }

    // CategoricalSxPls: project each source toward A with PLS fitted on the samples
    // given to this screen (train-only in the train-split path; in-sample for the
    // all-samples screen, which the metric_pipeline provenance records). The
    // target A stays unprojected.
    let mut pls_selection = None;
    let pls_projected = match pid_mode {
        PidMode::CategoricalSxPls => {
            // Per-source component choice: fixed, or LOO-CV Q² selection
            // (grandplan §6.2 leakage-safe fitted preprocessing).
            let choose = |x: MatRef<'_>| -> Result<(usize, Option<f64>)> {
                match pls {
                    PlsComponentSelection::Fixed(k) => Ok((k, None)),
                    PlsComponentSelection::CvQ2 { max_components } => {
                        let cv = pls_cv_select_components_with_budget(
                            x,
                            a,
                            max_components,
                            budgets.dense_solver,
                        )?;
                        // In the current pid-core review contract, `best_components` is `None` when
                        // no candidate completed every predeclared fold, and Q² lives on the
                        // candidate outcome.
                        let best = cv.best_components.context(
                            "PLS CV selected no component count (no candidate completed every fold)",
                        )?;
                        let q2 = cv
                            .candidates
                            .iter()
                            .find(|candidate| candidate.components == best)
                            .and_then(|candidate| match candidate.status {
                                PlsCvCandidateStatus::Complete { q2 } => Some(q2),
                                _ => None,
                            });
                        Ok((best, q2))
                    }
                }
            };
            let (kv, q2v) = choose(v)?;
            let (kl, q2l) = choose(l)?;
            let (kd, q2d) = choose(d)?;
            let v_projector = PlsProjector::fit_with_budget(v, a, kv, budgets.dense_solver)?;
            let l_projector = PlsProjector::fit_with_budget(l, a, kl, budgets.dense_solver)?;
            let d_projector = PlsProjector::fit_with_budget(d, a, kd, budgets.dense_solver)?;
            let v_proj = v_projector.transform_with_budget(v, budgets.dense_solver)?;
            let l_proj = l_projector.transform_with_budget(l, budgets.dense_solver)?;
            let d_proj = d_projector.transform_with_budget(d, budgets.dense_solver)?;
            pls_selection = Some(OfflineVldaPlsSelection {
                method: match pls {
                    PlsComponentSelection::Fixed(_) => "fixed".to_string(),
                    PlsComponentSelection::CvQ2 { .. } => "cv_q2".to_string(),
                },
                components_v: kv,
                components_l: kl,
                components_d: kd,
                q2_v: q2v,
                q2_l: q2l,
                q2_d: q2d,
            });
            Some((v_proj, l_proj, d_proj))
        }
        PidMode::Disabled => unreachable!("disabled mode returns before PID preprocessing"),
        PidMode::Continuous | PidMode::CategoricalSx => None,
    };
    let (v_eff, l_eff, d_eff) = match &pls_projected {
        Some((v_proj, l_proj, d_proj)) => (v_proj.as_ref(), l_proj.as_ref(), d_proj.as_ref()),
        None => (v, l, d),
    };
    // Each screen carries the same per-axis diagnostics into several marginal
    // and pair outcomes. Compute each axis once and clone only the small typed
    // summary instead of rebuilding row-count maps for every estimate.
    let v_diagnostics = axis_diagnostics("V", v_eff, support);
    let l_diagnostics = axis_diagnostics("L", l_eff, support);
    let d_diagnostics = axis_diagnostics("D", d_eff, support);
    let a_diagnostics = axis_diagnostics("A", a, support);
    let quantized = if matches!(pid_mode, PidMode::CategoricalSx | PidMode::CategoricalSxPls) {
        Some((
            prepare_quantized_axis(v_eff, categorical_bins, budgets.categorical_pid)?,
            prepare_quantized_axis(l_eff, categorical_bins, budgets.categorical_pid)?,
            prepare_quantized_axis(d_eff, categorical_bins, budgets.categorical_pid)?,
            prepare_quantized_axis(a, categorical_bins, budgets.categorical_pid)?,
        ))
    } else {
        None
    };
    let categorical_quantization = match &quantized {
        Some((v_axis, l_axis, d_axis, a_axis)) => {
            [("V", v_axis), ("L", l_axis), ("D", d_axis), ("A", a_axis)]
                .into_iter()
                .map(|(axis, prepared)| {
                    Ok((axis.to_string(), quantization_receipt(axis, prepared)?))
                })
                .collect::<Result<BTreeMap<_, _>>>()?
        }
        None => BTreeMap::new(),
    };

    let v_source = OfflineVldaSourceMatrix {
        name: "V",
        matrix: v_eff,
    };
    let l_source = OfflineVldaSourceMatrix {
        name: "L",
        matrix: l_eff,
    };
    let d_source = OfflineVldaSourceMatrix {
        name: "D",
        matrix: d_eff,
    };
    let action_target = OfflineVldaTargetMatrix {
        name: "A",
        matrix: a,
    };

    let (mut vl_pair, mut vd_pair, mut ld_pair) = match pid_mode {
        PidMode::Continuous => {
            let ksg = ksg_config();
            let pid_cfg = pid2_config(&ksg);
            (
                compute_pid_pair_metrics(
                    v_source,
                    l_source,
                    action_target,
                    vec![
                        v_diagnostics.clone(),
                        l_diagnostics.clone(),
                        a_diagnostics.clone(),
                    ],
                    continuous_tuple_support
                        .get(CONTINUOUS_TUPLE_V_L_A)
                        .copied(),
                    &pid_cfg,
                )?,
                compute_pid_pair_metrics(
                    v_source,
                    d_source,
                    action_target,
                    vec![
                        v_diagnostics.clone(),
                        d_diagnostics.clone(),
                        a_diagnostics.clone(),
                    ],
                    continuous_tuple_support
                        .get(CONTINUOUS_TUPLE_V_D_A)
                        .copied(),
                    &pid_cfg,
                )?,
                compute_pid_pair_metrics(
                    l_source,
                    d_source,
                    action_target,
                    vec![
                        l_diagnostics.clone(),
                        d_diagnostics.clone(),
                        a_diagnostics.clone(),
                    ],
                    continuous_tuple_support
                        .get(CONTINUOUS_TUPLE_L_D_A)
                        .copied(),
                    &pid_cfg,
                )?,
            )
        }
        PidMode::CategoricalSx | PidMode::CategoricalSxPls => {
            let (v_quantized, l_quantized, d_quantized, a_quantized) = quantized
                .as_ref()
                .context("categorical Sx mode lacks prepared fitted-categorical axes")?;
            (
                compute_pid_pair_metrics_categorical_sx(
                    v_source,
                    l_source,
                    action_target,
                    PreparedCategoricalSxPair {
                        source_1: v_quantized,
                        source_2: l_quantized,
                        target: a_quantized,
                    },
                    vec![
                        v_diagnostics.clone(),
                        l_diagnostics.clone(),
                        a_diagnostics.clone(),
                    ],
                    budgets.categorical_pid,
                )?,
                compute_pid_pair_metrics_categorical_sx(
                    v_source,
                    d_source,
                    action_target,
                    PreparedCategoricalSxPair {
                        source_1: v_quantized,
                        source_2: d_quantized,
                        target: a_quantized,
                    },
                    vec![
                        v_diagnostics.clone(),
                        d_diagnostics.clone(),
                        a_diagnostics.clone(),
                    ],
                    budgets.categorical_pid,
                )?,
                compute_pid_pair_metrics_categorical_sx(
                    l_source,
                    d_source,
                    action_target,
                    PreparedCategoricalSxPair {
                        source_1: l_quantized,
                        source_2: d_quantized,
                        target: a_quantized,
                    },
                    vec![
                        l_diagnostics.clone(),
                        d_diagnostics.clone(),
                        a_diagnostics.clone(),
                    ],
                    budgets.categorical_pid,
                )?,
            )
        }
        PidMode::Disabled => unreachable!("disabled mode returns before PID estimation"),
    };

    // Every produced PID2 estimate already computes both source-target marginals.
    // Reuse those exact values instead of running three redundant estimators. A
    // continuous marginal runs separately only when every pair that contains
    // its source abstained before yielding that value.
    let v_pair_value = vl_pair.mi_source_1_action.or(vd_pair.mi_source_1_action);
    let l_pair_value = vl_pair.mi_source_2_action.or(ld_pair.mi_source_1_action);
    let d_pair_value = vd_pair.mi_source_2_action.or(ld_pair.mi_source_2_action);
    let (mut mi_v_action, mut mi_l_action, mut mi_d_action) = match pid_mode {
        PidMode::Continuous => {
            let ksg = ksg_config();
            let marginal = |source_name: &'static str,
                            source: MatRef<'_>,
                            source_diagnostics: &OfflineVldaAxisDiagnostics,
                            tuple_key: &'static str,
                            pair_value: Option<f64>|
             -> Result<OfflineVldaMiEstimate> {
                continuous_mi_estimate(
                    OfflineVldaSourceMatrix {
                        name: source_name,
                        matrix: source,
                    },
                    OfflineVldaTargetMatrix {
                        name: "A",
                        matrix: a,
                    },
                    vec![source_diagnostics.clone(), a_diagnostics.clone()],
                    continuous_tuple_support.get(tuple_key).copied(),
                    pair_value,
                    &ksg,
                )
            };
            (
                marginal(
                    "V",
                    v_eff,
                    &v_diagnostics,
                    CONTINUOUS_TUPLE_V_A,
                    v_pair_value,
                )?,
                marginal(
                    "L",
                    l_eff,
                    &l_diagnostics,
                    CONTINUOUS_TUPLE_L_A,
                    l_pair_value,
                )?,
                marginal(
                    "D",
                    d_eff,
                    &d_diagnostics,
                    CONTINUOUS_TUPLE_D_A,
                    d_pair_value,
                )?,
            )
        }
        PidMode::CategoricalSx | PidMode::CategoricalSxPls => {
            let (v_quantized, l_quantized, d_quantized, a_quantized) = quantized
                .as_ref()
                .context("categorical Sx mode lacks prepared fitted-categorical axes")?;
            let marginal = |source_name: &'static str,
                            source_diagnostics: &OfflineVldaAxisDiagnostics,
                            source_quantized: &PreparedQuantizedAxis,
                            pair_value: Option<f64>|
             -> Result<OfflineVldaMiEstimate> {
                let value = pair_value.with_context(|| {
                    format!(
                        "categorical SxPID pairs produced no marginal MI for {source_name} -> A"
                    )
                })?;
                Ok(OfflineVldaMiEstimate {
                    outcome: categorical_mi_outcome(
                        source_name,
                        vec![source_diagnostics.clone(), a_diagnostics.clone()],
                        source_quantized,
                        a_quantized,
                    ),
                    value: Some(value),
                })
            };
            (
                marginal("V", &v_diagnostics, v_quantized, v_pair_value)?,
                marginal("L", &l_diagnostics, l_quantized, l_pair_value)?,
                marginal("D", &d_diagnostics, d_quantized, d_pair_value)?,
            )
        }
        PidMode::Disabled => unreachable!("disabled mode returns before MI estimation"),
    };

    if pid_mode == PidMode::CategoricalSxPls {
        for outcome in [
            &mut mi_v_action.outcome,
            &mut mi_l_action.outcome,
            &mut mi_d_action.outcome,
            &mut vl_pair.outcome,
            &mut vd_pair.outcome,
            &mut ld_pair.outcome,
        ] {
            mark_supervised_same_row_warning(outcome);
        }
    }

    // Denominators over every requested estimate: three marginal MIs plus three pairs.
    let mut estimate_denominators = OfflineVldaEstimateDenominators::default();
    for outcome in [
        &mi_v_action.outcome,
        &mi_l_action.outcome,
        &mi_d_action.outcome,
    ] {
        estimate_denominators.record(outcome);
    }
    for pair in [&vl_pair, &vd_pair, &ld_pair] {
        estimate_denominators.record(&pair.outcome);
    }

    let mi_vl_action = vl_pair.mi_joint_action;
    let co_information_v_l_action = vl_pair.co_information;
    let redundancy_v_l_action = vl_pair.redundancy;
    let unique_v_action = vl_pair.unique_source_1;
    let unique_l_action = vl_pair.unique_source_2;
    let synergy_v_l_action = vl_pair.synergy;
    let pid_pairs = [
        ("VL".to_string(), vl_pair),
        ("VD".to_string(), vd_pair),
        ("LD".to_string(), ld_pair),
    ]
    .into_iter()
    .collect();
    Ok(OfflineVldaPidScreenMetrics {
        mi_v_action,
        mi_l_action,
        mi_d_action,
        // The `(V,L)→A` aggregates mirror the VL pair, so a VL abstention propagates: a partial
        // summary must never imply that all three pairs were estimated.
        mi_vl_action,
        co_information_v_l_action,
        redundancy_v_l_action,
        unique_v_action,
        unique_l_action,
        synergy_v_l_action,
        estimate_denominators,
        pid_pairs,
        categorical_quantization,
        pls_selection,
        pls_shuffled_target_control: None,
        pls_control_seed: None,
    })
}

/// Fixed seed of the `categorical-sx-pls` shuffled-target control.
const PLS_CONTROL_SEED: u64 = 0x51AF_F1ED;

/// [`compute_pid_screen_metrics`], plus, in `categorical-sx-pls` mode, the
/// fixed-seed **shuffled-target negative-control draw**: the identical pipeline re-run with
/// the target `A`'s rows shuffled by a seeded permutation, attached to the
/// returned metrics. One draw is not a null distribution, p-value, bound, or floor. See
/// `OfflineVldaPidScreenMetrics::pls_shuffled_target_control`.
fn compute_pid_screen_metrics_with_control(
    prepared: &PreparedVldaMatrices,
    contract: OfflineVldaPidScreenContract<'_>,
) -> Result<OfflineVldaPidScreenMetrics> {
    let mut metrics = compute_pid_screen_metrics(
        OfflineVldaPidMatrices {
            v: prepared.v.as_ref(),
            l: prepared.l.as_ref(),
            d: prepared.d.as_ref(),
            a: prepared.a.as_ref(),
        },
        contract,
    )?;
    if contract.options.pid_mode == PidMode::CategoricalSxPls {
        let shuffled_target = shuffled_target(prepared.a.as_ref(), PLS_CONTROL_SEED)?;
        let control = compute_pid_screen_metrics(
            OfflineVldaPidMatrices {
                v: prepared.v.as_ref(),
                l: prepared.l.as_ref(),
                d: prepared.d.as_ref(),
                a: shuffled_target.as_ref(),
            },
            contract,
        )?;
        metrics.pls_shuffled_target_control = Some(Box::new(control));
        metrics.pls_control_seed = Some(PLS_CONTROL_SEED);
    }
    Ok(metrics)
}

/// Return target `A` with rows permuted by a seeded Fisher-Yates shuffle
/// (SplitMix64 stream). The control borrows V/L/D and allocates no copies of
/// those unchanged matrices.
fn shuffled_target(a: MatRef<'_>, seed: u64) -> Result<MatOwned> {
    let n = a.nrows();
    let dim = a.ncols();
    let mut perm: Vec<usize> = (0..n).collect();
    let mut state = seed;
    let mut next_u64 = move || -> u64 {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    for i in (1..n).rev() {
        let upper = u64::try_from(i + 1).context("shuffle width exceeds u64")?;
        // Rejection sampling avoids the modulo bias of mapping all 2^64
        // generator outputs directly into a non-power-of-two interval.
        let threshold = upper.wrapping_neg() % upper;
        let draw = loop {
            let draw = next_u64();
            if draw >= threshold {
                break draw;
            }
        };
        let j = usize::try_from(draw % upper).context("shuffle index exceeds usize")?;
        perm.swap(i, j);
    }
    let mut data = Vec::with_capacity(n * dim);
    for &i in &perm {
        data.extend_from_slice(a.row(i));
    }
    MatOwned::new(data, n, dim).map_err(|e| anyhow::anyhow!("shuffled target: {e}"))
}

// ── Opt-in PID-screen stability summaries + permutation nulls ──

/// Machine-readable interpretation of the raw m-sample resampling percentiles.
///
/// This exact value is serialized into every uncertainty sidecar so consumers cannot
/// silently reinterpret the stability envelope as a calibrated confidence interval for the
/// full-n estimator or population quantity.
pub const RAW_M_SAMPLE_STABILITY_INTERPRETATION: &str =
    "raw_m_sample_percentiles_not_n_sample_confidence_intervals";

/// Current offline uncertainty holds the all-sample standardization fixed while rows are
/// resampled. This is a conditional stability diagnostic, not nested preprocessing uncertainty.
pub const OFFLINE_UNCERTAINTY_PREPROCESSING_RESAMPLING: &str =
    "fixed_full_sample_standardization_not_nested";

fn raw_m_sample_stability_interpretation() -> String {
    RAW_M_SAMPLE_STABILITY_INTERPRETATION.to_string()
}

/// Configuration for [`compute_offline_pid_uncertainty`].
#[derive(Debug, Clone, PartialEq)]
pub struct OfflineVldaUncertaintyConfig {
    /// Number of subsample resamples (0 disables raw m-sample stability percentiles).
    pub n_boot: usize,
    /// Number of permutations for single-source unique-atom nulls (0 disables them).
    pub n_perm: usize,
    /// Predeclared row dependence scale for the subsample resampler. A value of one is an
    /// exchangeability assertion only when rows are independent sampling units.
    pub block_size: usize,
    /// Two-sided tail mass used to select the raw m-sample percentiles.
    pub alpha: f64,
    /// Base seed for the resamplers.
    pub seed: u64,
    /// How the permutation null rearranges the shuffled source's rows.
    /// `FullShuffle` simulates **exchangeable (i.i.d.) rows** — on per-step
    /// captures with within-episode autocorrelation it is anti-conservative.
    /// `CircularShift { min_shift }` preserves one source's serial order up to a wrap seam and
    /// produces an approximate stationary-surrogate score, not a p-value. It is supported only
    /// for one ordered series. Current resamplers fail closed on multiple non-singleton episodes.
    pub permutation_scheme: PermutationScheme,
}

impl Default for OfflineVldaUncertaintyConfig {
    fn default() -> Self {
        Self {
            n_boot: 0,
            n_perm: 0,
            block_size: 1,
            alpha: 0.05,
            seed: 0xC0FFEE,
            permutation_scheme: PermutationScheme::FullShuffle,
        }
    }
}

/// Stable string label for the permutation scheme, recorded in the uncertainty
/// artifact so a standalone JSON consumer can tell which null produced the
/// tail fractions.
fn permutation_scheme_label(scheme: PermutationScheme) -> Result<String> {
    Ok(match scheme {
        PermutationScheme::FullShuffle => "full_shuffle".to_string(),
        PermutationScheme::CircularShift { min_shift } => {
            format!("circular_shift(min_shift={min_shift})")
        }
        PermutationScheme::BlockShuffle { block_size } => {
            format!("block_shuffle(block_size={block_size})")
        }
        // `PermutationScheme` is `#[non_exhaustive]`: new upstream nulls require an explicit,
        // reviewed label before they may appear in a publication artifact.
        other => bail!("unsupported permutation scheme for uncertainty provenance: {other:?}"),
    })
}

fn permutation_calibration_label(scheme: PermutationScheme, n_perm: usize) -> Result<&'static str> {
    if n_perm == 0 {
        return Ok("not_requested");
    }
    match scheme {
        PermutationScheme::FullShuffle => {
            Ok("monte_carlo_p_value_under_declared_row_exchangeability")
        }
        PermutationScheme::BlockShuffle { .. } => {
            Ok("monte_carlo_p_value_under_declared_whole_block_exchangeability")
        }
        PermutationScheme::CircularShift { .. } => {
            Ok("approximate_stationary_surrogate_score_not_p_value")
        }
        other => bail!("unsupported permutation scheme for uncertainty calibration: {other:?}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OfflineVldaUncertaintyRowTopology {
    /// No row has an episode id. This does not establish one continuous ordered series.
    RowOrderWithoutEpisodeIds,
    /// Every row belongs to one episode and carries a strict canonical sequence index.
    SingleOrderedEpisode,
    /// Every row belongs to one episode, but row order has no strict sequence-index receipt.
    SingleEpisodeWithoutVerifiedOrder,
    /// Every row has a distinct episode id and is therefore one complete sampling unit.
    SingletonEpisodes,
    /// More than one episode exists and at least one contains multiple ordered rows.
    MultipleEpisodesWithRepeatedRows,
    /// Some rows have episode ids and others do not.
    MixedEpisodeIdPresence,
}

impl OfflineVldaUncertaintyRowTopology {
    const fn label(self) -> &'static str {
        match self {
            Self::RowOrderWithoutEpisodeIds => "row_order_without_episode_ids",
            Self::SingleOrderedEpisode => "single_ordered_episode",
            Self::SingleEpisodeWithoutVerifiedOrder => "single_episode_without_verified_order",
            Self::SingletonEpisodes => "singleton_episodes",
            Self::MultipleEpisodesWithRepeatedRows => "multiple_episodes_with_repeated_rows",
            Self::MixedEpisodeIdPresence => "mixed_episode_id_presence",
        }
    }

    fn from_label(label: &str) -> Result<Self> {
        match label {
            "row_order_without_episode_ids" => Ok(Self::RowOrderWithoutEpisodeIds),
            "single_ordered_episode" => Ok(Self::SingleOrderedEpisode),
            "single_episode_without_verified_order" => Ok(Self::SingleEpisodeWithoutVerifiedOrder),
            "singleton_episodes" => Ok(Self::SingletonEpisodes),
            "multiple_episodes_with_repeated_rows" => Ok(Self::MultipleEpisodesWithRepeatedRows),
            "mixed_episode_id_presence" => Ok(Self::MixedEpisodeIdPresence),
            _ => bail!("unknown offline VLDA uncertainty row topology: {label}"),
        }
    }

    fn supports(self, config: &OfflineVldaUncertaintyConfig) -> bool {
        let row_exchangeable_only = || {
            (config.n_boot == 0 || config.block_size == 1)
                && (config.n_perm == 0
                    || matches!(config.permutation_scheme, PermutationScheme::FullShuffle))
        };
        match self {
            // Missing episode identities do not authorize a synthetic time series. The caller
            // may still make the explicit row-exchangeability assertion encoded by unit blocks
            // and a full shuffle.
            Self::RowOrderWithoutEpisodeIds | Self::SingletonEpisodes => row_exchangeable_only(),
            Self::SingleOrderedEpisode => true,
            Self::SingleEpisodeWithoutVerifiedOrder
            | Self::MultipleEpisodesWithRepeatedRows
            | Self::MixedEpisodeIdPresence => false,
        }
    }
}

fn has_strict_sequence_index(samples: &[OfflineVldaSample]) -> bool {
    let mut previous = None::<u64>;
    for sample in samples {
        let Some(raw) = sample.metadata.get("sequence_index") else {
            return false;
        };
        let Ok(current) = raw.parse::<u64>() else {
            return false;
        };
        if current.to_string() != *raw || previous.is_some_and(|value| current <= value) {
            return false;
        }
        previous = Some(current);
    }
    previous.is_some()
}

fn segments_have_strict_sequence_index(
    samples: &[OfflineVldaSample],
    segments: &[std::ops::Range<usize>],
) -> bool {
    segments.iter().all(|segment| {
        if segment.len() < 2 {
            return true;
        }
        let mut previous = None::<u64>;
        samples[segment.clone()].iter().all(|sample| {
            let Some(raw) = sample.metadata.get("sequence_index") else {
                return false;
            };
            let Ok(current) = raw.parse::<u64>() else {
                return false;
            };
            let valid = current.to_string() == *raw && previous.is_none_or(|value| current > value);
            if valid {
                previous = Some(current);
            }
            valid
        })
    })
}

fn split_segments_at_sequence_index_gaps(
    samples: &[OfflineVldaSample],
    segments: &[std::ops::Range<usize>],
) -> (Vec<std::ops::Range<usize>>, usize) {
    let mut unit_step_segments = Vec::with_capacity(segments.len());
    let mut gap_pairs = 0usize;
    for segment in segments {
        let mut start = segment.start;
        for idx in segment.start.saturating_add(1)..segment.end {
            let previous = samples[idx - 1].metadata["sequence_index"]
                .parse::<u64>()
                .expect("strict sequence receipt was verified");
            let current = samples[idx].metadata["sequence_index"]
                .parse::<u64>()
                .expect("strict sequence receipt was verified");
            if current - previous > 1 {
                unit_step_segments.push(start..idx);
                start = idx;
                gap_pairs += 1;
            }
        }
        unit_step_segments.push(start..segment.end);
    }
    (unit_step_segments, gap_pairs)
}

fn uncertainty_execution_label(
    pid_mode: PidMode,
    config: &OfflineVldaUncertaintyConfig,
    topology: OfflineVldaUncertaintyRowTopology,
) -> &'static str {
    if !config.enabled() {
        "not_requested"
    } else if pid_mode != PidMode::Continuous {
        "typed_skip_non_continuous_measure"
    } else if !topology.supports(config) {
        "typed_skip_episode_aware_resampling_required"
    } else {
        "eligible_for_execution"
    }
}

fn uncertainty_row_topology(samples: &[OfflineVldaSample]) -> OfflineVldaUncertaintyRowTopology {
    let present = samples
        .iter()
        .filter(|sample| sample.episode_id.is_some())
        .count();
    if present == 0 {
        return OfflineVldaUncertaintyRowTopology::RowOrderWithoutEpisodeIds;
    }
    if present != samples.len() {
        return OfflineVldaUncertaintyRowTopology::MixedEpisodeIdPresence;
    }
    let mut counts = BTreeMap::<&str, usize>::new();
    for sample in samples {
        let episode_id = sample
            .episode_id
            .as_deref()
            .expect("all episode ids were checked as present");
        *counts.entry(episode_id).or_default() += 1;
    }
    if counts.len() == 1 {
        if has_strict_sequence_index(samples) {
            OfflineVldaUncertaintyRowTopology::SingleOrderedEpisode
        } else {
            OfflineVldaUncertaintyRowTopology::SingleEpisodeWithoutVerifiedOrder
        }
    } else if counts.values().all(|count| *count == 1) {
        OfflineVldaUncertaintyRowTopology::SingletonEpisodes
    } else {
        OfflineVldaUncertaintyRowTopology::MultipleEpisodesWithRepeatedRows
    }
}

const UNSUPPORTED_UNCERTAINTY_TOPOLOGY_MODE: &str =
    "skipped:episode_aware_resampling_required_for_row_topology";

impl OfflineVldaUncertaintyConfig {
    pub fn enabled(&self) -> bool {
        self.n_boot > 0 || self.n_perm > 0
    }
}

fn validate_uncertainty_config(config: &OfflineVldaUncertaintyConfig) -> Result<()> {
    ensure!(config.block_size > 0, "uncertainty block_size must be >= 1");
    ensure!(
        config.n_boot == 0 || config.n_boot >= 2,
        "continuous uncertainty requires at least two bootstrap resamples"
    );
    ensure!(
        config.alpha.is_finite() && config.alpha > 0.0 && config.alpha < 1.0,
        "uncertainty raw-percentile tail mass must lie strictly inside (0, 1)"
    );
    if config.n_perm > 0 {
        let _ = permutation_scheme_label(config.permutation_scheme)?;
    }
    if config.n_boot > 0 && config.n_perm > 0 {
        let bootstrap_declares_row_exchangeability = config.block_size == 1;
        let permutation_declares_row_exchangeability =
            matches!(config.permutation_scheme, PermutationScheme::FullShuffle);
        ensure!(
            bootstrap_declares_row_exchangeability == permutation_declares_row_exchangeability,
            "one uncertainty request cannot combine incompatible row assumptions: unit-block bootstrap must pair with full shuffle, while multi-row block bootstrap must pair with a dependence-preserving permutation or surrogate"
        );
        if let PermutationScheme::BlockShuffle { block_size } = config.permutation_scheme {
            ensure!(
                block_size == config.block_size,
                "one uncertainty request cannot combine different bootstrap and block-shuffle dependence scales: bootstrap block_size={}, permutation block_size={block_size}",
                config.block_size
            );
        }
    }
    Ok(())
}

fn validate_uncertainty_config_for_samples(
    config: &OfflineVldaUncertaintyConfig,
    samples: usize,
) -> Result<()> {
    validate_uncertainty_config(config)?;
    if config.n_boot > 0 {
        ensure!(
            samples >= 2 && config.block_size <= samples / 2,
            "uncertainty block_size must fit inside the declared half-sample stability envelope: samples={samples}, block_size={}",
            config.block_size
        );
    }
    if config.n_perm > 0 {
        match config.permutation_scheme {
            PermutationScheme::FullShuffle => {}
            PermutationScheme::BlockShuffle { block_size } => {
                ensure!(
                    block_size > 0
                        && samples.is_multiple_of(block_size)
                        && samples / block_size >= 2,
                    "uncertainty block-shuffle requires a positive block_size that divides {samples} samples into at least two blocks"
                );
            }
            PermutationScheme::CircularShift { min_shift } => {
                let minimum_samples = min_shift
                    .checked_mul(2)
                    .and_then(|value| value.checked_add(1))
                    .context("uncertainty circular-shift min_shift is too large")?;
                ensure!(
                    min_shift > 0 && samples >= minimum_samples,
                    "uncertainty circular-shift requires samples >= 2*min_shift+1: samples={samples}, min_shift={min_shift}"
                );
            }
            other => bail!("unsupported permutation scheme for uncertainty provenance: {other:?}"),
        }
    }
    Ok(())
}

/// Raw m-sample percentile stability envelope for one PID atom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaAtomStabilityEnvelope {
    pub point: f64,
    /// Lower raw percentile of the m-sample resampling distribution.
    ///
    /// `ci_low` is accepted only to read pre-correction sidecars. New serialization always emits
    /// `m_sample_percentile_lower`.
    #[serde(alias = "ci_low")]
    pub m_sample_percentile_lower: f64,
    /// Upper raw percentile of the m-sample resampling distribution.
    ///
    /// `ci_high` is accepted only to read pre-correction sidecars. New serialization always emits
    /// `m_sample_percentile_upper`.
    #[serde(alias = "ci_high")]
    pub m_sample_percentile_upper: f64,
    pub n_valid: usize,
    /// Mean of the m-out-of-n subsample distribution. KSG/`I^sx` bias is
    /// sample-size dependent (it grows as samples shrink), so this estimates
    /// `E[θ̂_m]` at `m = subsample_len`, **not** `E[θ̂_n]` — the subsample
    /// distribution is *mis-centered* relative to the full-n point estimate,
    /// not merely wider. Read the raw percentiles only as an m-sample stability
    /// envelope, never as calibrated coverage for the full-n estimator or population atom.
    /// `None` on artifacts written before this field existed.
    #[serde(default)]
    pub boot_mean: Option<f64>,
    /// `boot_mean − point` — the m-dependent-bias diagnostic, precomputed for
    /// artifact consumers: a gap large relative to
    /// `m_sample_percentile_upper − m_sample_percentile_lower` flags that the
    /// m-sample distribution is dominated by small-sample estimator bias.
    /// `None` on old artifacts.
    #[serde(default)]
    pub bias_vs_point: Option<f64>,
}

/// Raw m-sample stability envelopes and null tail fractions for one two-source pair → A.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPairUncertainty {
    pub pair: String,
    /// `produced`, `produced_with_warning`, or `abstained` for a requested continuous pair.
    /// Status is derived from actual requested-component presence, not application eligibility.
    #[serde(default = "produced_status")]
    pub status: OfflineVldaEstimateStatus,
    /// The same four scientific verdicts carried by point-estimate outcomes. Old sidecars did not
    /// record these gates and deserialize conservatively as not evaluated/application blocked.
    #[serde(default = "legacy_scientific_gates")]
    pub scientific_gates: OfflineVldaScientificGates,
    /// Strong joint-law declaration for this exact continuous PID tuple.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_continuous_tuple_support: Option<OfflineVldaContinuousTupleSupport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<OfflineVldaAbstainReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_detail: Option<String>,
    /// Stable warnings for requested inferential components that could not be produced while at
    /// least one other requested component was produced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warning_codes: Vec<OfflineVldaUncertaintyWarning>,
    pub redundancy: Option<OfflineVldaAtomStabilityEnvelope>,
    pub unique_s1: Option<OfflineVldaAtomStabilityEnvelope>,
    pub unique_s2: Option<OfflineVldaAtomStabilityEnvelope>,
    pub synergy: Option<OfflineVldaAtomStabilityEnvelope>,
    /// One-sided null tail fraction for `unique_s1` after transforming source 1. This is a
    /// Monte Carlo p-value only under a supported exchangeability scheme and its exact declared
    /// null. A circular-shift value is an approximate surrogate score.
    #[serde(alias = "unique_s1_perm_p")]
    pub unique_s1_tail_fraction: Option<f64>,
    /// One-sided null tail fraction for `unique_s2` after transforming source 2.
    #[serde(alias = "unique_s2_perm_p")]
    pub unique_s2_tail_fraction: Option<f64>,
    pub perm_n_valid_s1: usize,
    pub perm_n_valid_s2: usize,
}

/// Why a pair-level uncertainty record is only partially produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineVldaUncertaintyWarning {
    BootstrapStatisticsUnavailable,
    UniqueSource1PermutationUnavailable,
    UniqueSource2PermutationUnavailable,
}

impl OfflineVldaUncertaintyWarning {
    fn as_str(self) -> &'static str {
        match self {
            Self::BootstrapStatisticsUnavailable => "bootstrap_statistics_unavailable",
            Self::UniqueSource1PermutationUnavailable => "unique_source_1_permutation_unavailable",
            Self::UniqueSource2PermutationUnavailable => "unique_source_2_permutation_unavailable",
        }
    }
}

fn produced_status() -> OfflineVldaEstimateStatus {
    OfflineVldaEstimateStatus::Produced
}

/// Result of [`compute_offline_pid_uncertainty`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineVldaPidUncertainty {
    /// Schema for the self-contained uncertainty companion.
    #[serde(default)]
    pub schema_version: u32,
    /// Canonical semantic hash of the exact decoded dataset used for this computation.
    #[serde(default)]
    pub dataset_content_sha256: String,
    /// Exact estimator revision used by every pair-level callback.
    #[serde(default)]
    pub estimator_revision: String,
    /// PID mode requested by the parent harness invocation.
    pub pid_mode: PidMode,
    /// `"continuous"` when requested stability/permutation summaries were computed, or
    /// `"skipped:<reason>"`.
    pub mode: String,
    /// Explicit scope of the raw subsample percentiles. Defaults to the same conservative
    /// interpretation when reading sidecars written before this field existed.
    #[serde(default = "raw_m_sample_stability_interpretation")]
    pub stability_interpretation: String,
    /// Whether fitted preprocessing is rerun inside each resample.
    #[serde(default = "offline_uncertainty_preprocessing_resampling")]
    pub preprocessing_resampling: String,
    pub n_boot: usize,
    pub n_perm: usize,
    pub block_size: usize,
    pub subsample_len: usize,
    /// Two-sided tail mass selecting the raw m-sample percentile endpoints. This is not a
    /// confidence-interval significance level.
    pub alpha: f64,
    /// Base seed used to derive every bootstrap and permutation stream.
    #[serde(default)]
    pub seed: u64,
    pub resample_scheme: String,
    /// Dataset row topology used to admit or reject the current resamplers.
    pub row_topology: String,
    /// Which permutation or surrogate transform produced the tail fractions.
    #[serde(default)]
    pub permutation_scheme: String,
    /// `"monte_carlo_p_value_under_declared_row_exchangeability"`,
    /// `"monte_carlo_p_value_under_declared_whole_block_exchangeability"`,
    /// `"approximate_stationary_surrogate_score_not_p_value"`, or `"not_requested"`.
    pub permutation_calibration: String,
    pub pairs: Vec<OfflineVldaPairUncertainty>,
}

fn offline_uncertainty_preprocessing_resampling() -> String {
    OFFLINE_UNCERTAINTY_PREPROCESSING_RESAMPLING.to_string()
}

/// Compute raw m-sample stability percentiles and single-source null tail fractions for the
/// three two-source `(V,L)→A` / `(V,D)→A` / `(L,D)→A` PID screens.
///
/// This is the analysis-side complement to the Exp0 uncertainty gate: it quantifies
/// uncertainty on the **continuous `I^sx_∩`** atoms. The categorical modes estimate a distinct
/// MGW shared-exclusions functional on explicitly quantized variables. They share a scientific
/// lineage, not one cross-domain measure. This continuous resampler does not cover the categorical
/// estimator and reports a typed skip. Resampling is Politis–Romano
/// subsampling without replacement. It avoids the
/// duplicate indices created by an ordinary with-replacement bootstrap, but does not establish
/// inferential validity for KSG or `I^sx_∩`; ties already present in the data can still reject a
/// resample. Its raw percentiles describe estimator stability at `m = subsample_len`; they do not
/// have calibrated n-sample confidence-interval coverage.
/// The all-sample standardization stays fixed inside every resample. The result is conditional
/// stability under that fitted transform, not nested preprocessing uncertainty.
/// Full shuffles require exchangeable rows. Circular shifts produce approximate stationary-series
/// surrogate scores, not p-values. The current implementation does not cross or pool dependent
/// episode boundaries: it returns a typed skip when multiple episodes contain repeated rows.
///
/// It is intentionally self-contained and written to a dedicated file by the
/// binary, so it never perturbs the canonical run-log / summary metric counts.
pub fn compute_offline_pid_uncertainty(
    dataset: &OfflineVldaDataset,
    pid_mode: PidMode,
    config: &OfflineVldaUncertaintyConfig,
) -> Result<OfflineVldaPidUncertainty> {
    compute_offline_pid_uncertainty_with_limits(
        dataset,
        pid_mode,
        config,
        &OfflineVldaResourceLimits::default(),
    )
}

/// Compute offline PID uncertainty under explicit decoded-size and distance-work limits.
pub fn compute_offline_pid_uncertainty_with_limits(
    dataset: &OfflineVldaDataset,
    pid_mode: PidMode,
    config: &OfflineVldaUncertaintyConfig,
    limits: &OfflineVldaResourceLimits,
) -> Result<OfflineVldaPidUncertainty> {
    validate_uncertainty_config(config)?;
    // Apply the decoded-size contract before hashing even when this call returns a typed skip.
    // Otherwise a disabled or non-continuous request could make the public `*_with_limits` API
    // serialize an over-limit in-memory dataset before enforcing its advertised boundary.
    let _ = admit_dataset_resources(dataset, None, None, limits)?;
    let dataset_content_sha256 = offline_vlda_dataset_content_sha256(dataset)
        .context("failed to hash the offline VLDA dataset for uncertainty provenance")?;
    compute_offline_pid_uncertainty_after_resource_preflight(
        dataset,
        pid_mode,
        config,
        limits,
        dataset_content_sha256,
    )
}

fn compute_offline_pid_uncertainty_after_resource_preflight(
    dataset: &OfflineVldaDataset,
    pid_mode: PidMode,
    config: &OfflineVldaUncertaintyConfig,
    limits: &OfflineVldaResourceLimits,
    dataset_content_sha256: String,
) -> Result<OfflineVldaPidUncertainty> {
    validate_uncertainty_config(config)?;
    let row_topology = uncertainty_row_topology(&dataset.samples);
    if !config.enabled() {
        return Ok(OfflineVldaPidUncertainty {
            schema_version: OFFLINE_UNCERTAINTY_SCHEMA_VERSION,
            dataset_content_sha256,
            estimator_revision: ESTIMATOR_CONTINUOUS_PID2.to_string(),
            pid_mode,
            mode: "skipped:no_uncertainty_requested".to_string(),
            stability_interpretation: raw_m_sample_stability_interpretation(),
            preprocessing_resampling: offline_uncertainty_preprocessing_resampling(),
            n_boot: 0,
            n_perm: 0,
            block_size: config.block_size,
            subsample_len: 0,
            alpha: config.alpha,
            seed: config.seed,
            resample_scheme: "not_requested".to_string(),
            row_topology: row_topology.label().to_string(),
            permutation_scheme: "not_requested".to_string(),
            permutation_calibration: "not_requested".to_string(),
            pairs: Vec::new(),
        });
    }
    if pid_mode != PidMode::Continuous {
        return Ok(OfflineVldaPidUncertainty {
            schema_version: OFFLINE_UNCERTAINTY_SCHEMA_VERSION,
            dataset_content_sha256,
            estimator_revision: ESTIMATOR_CONTINUOUS_PID2.to_string(),
            pid_mode,
            mode: format!("skipped:non_continuous_mode_is_a_different_measure ({pid_mode:?})"),
            stability_interpretation: raw_m_sample_stability_interpretation(),
            preprocessing_resampling: offline_uncertainty_preprocessing_resampling(),
            n_boot: config.n_boot,
            n_perm: config.n_perm,
            block_size: config.block_size,
            subsample_len: 0,
            alpha: config.alpha,
            seed: config.seed,
            resample_scheme: if config.n_boot > 0 {
                "politis_romano_subsample".to_string()
            } else {
                "not_requested".to_string()
            },
            row_topology: row_topology.label().to_string(),
            permutation_scheme: if config.n_perm > 0 {
                permutation_scheme_label(config.permutation_scheme)?
            } else {
                "not_requested".to_string()
            },
            permutation_calibration: permutation_calibration_label(
                config.permutation_scheme,
                config.n_perm,
            )?
            .to_string(),
            pairs: Vec::new(),
        });
    }
    validate_uncertainty_config_for_samples(config, dataset.samples.len())?;
    if !row_topology.supports(config) {
        return Ok(OfflineVldaPidUncertainty {
            schema_version: OFFLINE_UNCERTAINTY_SCHEMA_VERSION,
            dataset_content_sha256,
            estimator_revision: ESTIMATOR_CONTINUOUS_PID2.to_string(),
            pid_mode,
            mode: UNSUPPORTED_UNCERTAINTY_TOPOLOGY_MODE.to_string(),
            stability_interpretation: raw_m_sample_stability_interpretation(),
            preprocessing_resampling: offline_uncertainty_preprocessing_resampling(),
            n_boot: config.n_boot,
            n_perm: config.n_perm,
            block_size: config.block_size,
            subsample_len: 0,
            alpha: config.alpha,
            seed: config.seed,
            resample_scheme: if config.n_boot > 0 {
                "politis_romano_subsample".to_string()
            } else {
                "not_requested".to_string()
            },
            row_topology: row_topology.label().to_string(),
            permutation_scheme: if config.n_perm > 0 {
                permutation_scheme_label(config.permutation_scheme)?
            } else {
                "not_requested".to_string()
            },
            permutation_calibration: permutation_calibration_label(
                config.permutation_scheme,
                config.n_perm,
            )?
            .to_string(),
            pairs: Vec::new(),
        });
    }
    let projected = projected_uncertainty_distance_evaluations(dataset.samples.len(), config)?;
    let _ = enforce_pairwise_distance_limit(projected, limits, "uncertainty analysis")?;
    let projected_coordinates = projected_distance_coordinate_evaluations(
        projected,
        maximum_distance_vector_width(dataset)?,
        "uncertainty distance coordinate evaluations",
    )?;
    let _ =
        enforce_distance_coordinate_limit(projected_coordinates, limits, "uncertainty analysis")?;
    let dims = validate_dataset(dataset)?;
    let prepared = prepare_standardized_embeddings(&dataset.samples, &dims)?;
    let v = prepared.v.as_ref();
    let l = prepared.l.as_ref();
    let d = prepared.d.as_ref();
    let a = prepared.a.as_ref();
    let n = v.nrows();
    let v_diagnostics = axis_diagnostics("V", v, &dataset.support);
    let l_diagnostics = axis_diagnostics("L", l, &dataset.support);
    let d_diagnostics = axis_diagnostics("D", d, &dataset.support);
    let a_diagnostics = axis_diagnostics("A", a, &dataset.support);
    let diagnostic_for = |axis: &str| match axis {
        "V" => &v_diagnostics,
        "L" => &l_diagnostics,
        "D" => &d_diagnostics,
        "A" => &a_diagnostics,
        _ => unreachable!("uncertainty pair specification uses only V, L, D, and A"),
    };

    // Subsample length: half the rows in whole blocks (the conservative
    // Politis–Romano regime); clamp so there is at least one block.
    let subsample_len = if config.n_boot > 0 {
        (((n / 2) / config.block_size).max(1)) * config.block_size
    } else {
        0
    };

    let ksg = ksg_config();
    let pid_cfg = pid2_config(&ksg);

    let pairs_spec: [(&str, &'static str, &'static str, MatRef<'_>, MatRef<'_>); 3] = [
        ("VL", "V", "L", v, l),
        ("VD", "V", "D", v, d),
        ("LD", "L", "D", l, d),
    ];
    let mut pairs = Vec::with_capacity(3);
    for (name, axis_1, axis_2, s1, s2) in pairs_spec {
        let mats = [s1, s2, a];

        // Uncertainty is only meaningful for a pair the continuous estimator will actually run.
        // Preflight exactly as the screens do, and abstain rather than crash.
        let tuple_key = match name {
            "VL" => CONTINUOUS_TUPLE_V_L_A,
            "VD" => CONTINUOUS_TUPLE_V_D_A,
            "LD" => CONTINUOUS_TUPLE_L_D_A,
            _ => unreachable!("uncertainty pair specification uses only VL, VD, and LD"),
        };
        let tuple_support = dataset.continuous_tuple_support.get(tuple_key).copied();
        let (diagnostics, rejection) = continuous_preflight_from_diagnostics(
            vec![
                diagnostic_for(axis_1).clone(),
                diagnostic_for(axis_2).clone(),
                diagnostic_for("A").clone(),
            ],
            tuple_support,
        );
        // `pid2_resource_estimate` also rejects structurally-inapplicable pairs (e.g. unequal
        // ambient source dimensions), so consult it before doing any resampling work.
        let (rejection, pair_resource_estimate) = if rejection.is_some() {
            (rejection, None)
        } else {
            match pid2_resource_estimate(s1, s2, a, &pid_cfg) {
                Ok(estimate) => (None, Some(estimate)),
                Err(err) => {
                    let message = err.to_string();
                    match abstain_reason_for_error(&err) {
                        Some(reason) => (Some((reason, message)), None),
                        None => {
                            return Err(anyhow::anyhow!(
                                "pid2 uncertainty resource preflight ({axis_1}, {axis_2} -> A) failed: {message}"
                            ));
                        }
                    }
                }
            }
        };
        if let Some((reason, detail)) = rejection {
            pairs.push(OfflineVldaPairUncertainty {
                pair: name.to_string(),
                status: OfflineVldaEstimateStatus::Abstained,
                scientific_gates: abstained_scientific_gates(reason),
                declared_continuous_tuple_support: tuple_support,
                reason_code: Some(reason),
                reason_detail: Some(detail),
                warning_codes: Vec::new(),
                redundancy: None,
                unique_s1: None,
                unique_s2: None,
                synergy: None,
                unique_s1_tail_fraction: None,
                unique_s2_tail_fraction: None,
                perm_n_valid_s1: 0,
                perm_n_valid_s2: 0,
            });
            continue;
        }
        let pair_resource_estimate = pair_resource_estimate
            .context("accepted uncertainty pair lacks its resource estimate")?;

        let (redundancy, unique_s1, unique_s2, synergy) = if config.n_boot > 0 {
            // The caller fixes this assumption before the outcomes are visible. Unit blocks
            // assert independent, exchangeable rows. Longer blocks assert a weakly dependent
            // stationary series at the declared scale.
            let validity = if config.block_size == 1 {
                ResamplingValidityDeclaration::independent_rows(BlockLengthSelection::FixedAPriori)
            } else {
                ResamplingValidityDeclaration::weakly_dependent_stationary(
                    config.block_size,
                    BlockLengthSelection::FixedAPriori,
                )?
            };
            let boot_cfg = BootstrapConfig::new(
                config.n_boot,
                config.block_size,
                config.seed,
                config.alpha,
                validity,
            )?;
            let scheme = RowResampleScheme::Subsample { subsample_len };
            // The current pid-core review contract preflights the callback's cost, so its output
            // width and per-call resources must be declared up front. Four atoms per invocation.
            let callback = StatisticCallbackDeclaration::vector(4, pair_resource_estimate)?;
            let res = bootstrap_rows_stats(&mats, &boot_cfg, scheme, callback, |m| {
                let r = pid2_isx(m[0], m[1], m[2], &pid_cfg)?;
                Ok(vec![r.redundancy, r.unique_s1, r.unique_s2, r.synergy])
            })
            .map_err(|e| anyhow::anyhow!("pid2 bootstrap failed for {name}: {e}"))?;
            // `stats` is `None` when any replicate failed: the current pid-core review contract
            // refuses to summarize the successful subset selectively. Abstain from the stability
            // envelope rather than report a selectively summarized one.
            let to_stability_envelope = |idx: usize| {
                let s = res.stats.as_ref()?.get(idx)?;
                Some(OfflineVldaAtomStabilityEnvelope {
                    point: s.point_estimate,
                    m_sample_percentile_lower: s.percentile_lower,
                    m_sample_percentile_upper: s.percentile_upper,
                    n_valid: s.n_valid,
                    boot_mean: Some(s.resample_mean),
                    bias_vs_point: Some(s.resample_mean - s.point_estimate),
                })
            };
            (
                to_stability_envelope(0),
                to_stability_envelope(1),
                to_stability_envelope(2),
                to_stability_envelope(3),
            )
        } else {
            (None, None, None, None)
        };

        let (unique_s1_tail_fraction, perm_n_valid_s1, unique_s2_tail_fraction, perm_n_valid_s2) =
            if config.n_perm > 0 {
                // Transform source 1 for its unique-atom null; likewise source 2. FullShuffle
                // asserts row exchangeability. CircularShift preserves one series up to a wrap
                // seam and returns an approximate surrogate tail fraction, not a p-value.
                let callback = StatisticCallbackDeclaration::scalar(pair_resource_estimate);
                let p1 = permutation_rows_pvalue_with(
                    &mats,
                    0,
                    config.n_perm,
                    config.seed,
                    config.permutation_scheme,
                    callback,
                    |m| Ok(pid2_isx(m[0], m[1], m[2], &pid_cfg)?.unique_s1),
                )
                .map_err(|e| anyhow::anyhow!("pid2 permutation (s1) failed for {name}: {e}"))?;
                let p2 = permutation_rows_pvalue_with(
                    &mats,
                    1,
                    config.n_perm,
                    config.seed.wrapping_add(1),
                    config.permutation_scheme,
                    callback,
                    |m| Ok(pid2_isx(m[0], m[1], m[2], &pid_cfg)?.unique_s2),
                )
                .map_err(|e| anyhow::anyhow!("pid2 permutation (s2) failed for {name}: {e}"))?;
                // `p_value` became `tail_fraction: Option<f64>` — `None` when a transform failed.
                (
                    p1.tail_fraction.filter(|value| value.is_finite()),
                    p1.n_valid,
                    p2.tail_fraction.filter(|value| value.is_finite()),
                    p2.n_valid,
                )
            } else {
                (None, 0, None, 0)
            };

        let mut warning_codes = Vec::new();
        if config.n_boot > 0 && redundancy.is_none() {
            warning_codes.push(OfflineVldaUncertaintyWarning::BootstrapStatisticsUnavailable);
        }
        if config.n_perm > 0 && unique_s1_tail_fraction.is_none() {
            warning_codes.push(OfflineVldaUncertaintyWarning::UniqueSource1PermutationUnavailable);
        }
        if config.n_perm > 0 && unique_s2_tail_fraction.is_none() {
            warning_codes.push(OfflineVldaUncertaintyWarning::UniqueSource2PermutationUnavailable);
        }
        let requested_components =
            usize::from(config.n_boot > 0) + 2 * usize::from(config.n_perm > 0);
        let produced_components = usize::from(redundancy.is_some())
            + usize::from(unique_s1_tail_fraction.is_some())
            + usize::from(unique_s2_tail_fraction.is_some());
        let (status, scientific_gates, reason_code, reason_detail) = if produced_components
            == requested_components
        {
            (
                OfflineVldaEstimateStatus::Produced,
                produced_scientific_gates(&diagnostics),
                None,
                None,
            )
        } else if produced_components == 0 {
            let reason = OfflineVldaAbstainReason::UncertaintyStatisticsUnavailable;
            (
                OfflineVldaEstimateStatus::Abstained,
                abstained_scientific_gates(reason),
                Some(reason),
                Some(format!(
                    "none of the requested uncertainty components were available: {}",
                    warning_codes
                        .iter()
                        .map(|code| code.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )),
            )
        } else {
            let mut gates = produced_scientific_gates(&diagnostics);
            gates.estimator = OfflineVldaScientificGateVerdict::Blocked;
            gates.reason_code = Some("uncertainty_statistics_partially_unavailable".to_string());
            (
                OfflineVldaEstimateStatus::ProducedWithWarning,
                gates,
                None,
                Some(format!(
                    "some requested uncertainty components were unavailable: {}",
                    warning_codes
                        .iter()
                        .map(|code| code.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )),
            )
        };
        if status == OfflineVldaEstimateStatus::Abstained {
            warning_codes.clear();
        }

        pairs.push(OfflineVldaPairUncertainty {
            pair: name.to_string(),
            status,
            scientific_gates,
            declared_continuous_tuple_support: tuple_support,
            reason_code,
            reason_detail,
            warning_codes,
            redundancy,
            unique_s1,
            unique_s2,
            synergy,
            unique_s1_tail_fraction,
            unique_s2_tail_fraction,
            perm_n_valid_s1,
            perm_n_valid_s2,
        });
    }

    Ok(OfflineVldaPidUncertainty {
        schema_version: OFFLINE_UNCERTAINTY_SCHEMA_VERSION,
        dataset_content_sha256,
        estimator_revision: ESTIMATOR_CONTINUOUS_PID2.to_string(),
        pid_mode,
        mode: "continuous".to_string(),
        stability_interpretation: raw_m_sample_stability_interpretation(),
        preprocessing_resampling: offline_uncertainty_preprocessing_resampling(),
        n_boot: config.n_boot,
        n_perm: config.n_perm,
        block_size: config.block_size,
        subsample_len,
        alpha: config.alpha,
        seed: config.seed,
        resample_scheme: if config.n_boot > 0 {
            "politis_romano_subsample".to_string()
        } else {
            "not_requested".to_string()
        },
        row_topology: row_topology.label().to_string(),
        permutation_scheme: if config.n_perm > 0 {
            permutation_scheme_label(config.permutation_scheme)?
        } else {
            "not_requested".to_string()
        },
        permutation_calibration: permutation_calibration_label(
            config.permutation_scheme,
            config.n_perm,
        )?
        .to_string(),
        pairs,
    })
}

fn validate_outcome_contract(
    outcome: &OfflineVldaOutcome,
    has_numeric_value: bool,
    context: &str,
) -> Result<()> {
    ensure!(
        !outcome.measure.trim().is_empty() && outcome.measure == outcome.measure.trim(),
        "{context}: measure is empty or has surrounding whitespace"
    );
    ensure!(
        !outcome.estimator_revision.trim().is_empty()
            && outcome.estimator_revision == outcome.estimator_revision.trim(),
        "{context}: estimator revision is empty or has surrounding whitespace"
    );
    let expected_units = if outcome.status == OfflineVldaEstimateStatus::NotRequested {
        "not_applicable"
    } else {
        "nats"
    };
    ensure!(
        outcome.information_units == expected_units,
        "{context}: information units must be {expected_units}"
    );
    let is_continuous = matches!(
        outcome.measure.as_str(),
        MEASURE_CONTINUOUS_MI | MEASURE_CONTINUOUS_PID2
    );
    if !is_continuous {
        ensure!(
            outcome.declared_continuous_tuple_support.is_none(),
            "{context}: non-continuous outcome carries a continuous tuple-support contract"
        );
    } else if outcome.produced() {
        ensure!(
            outcome
                .declared_continuous_tuple_support
                .is_some_and(OfflineVldaContinuousTupleSupport::is_regular),
            "{context}: produced continuous outcome lacks its regular joint-law and finite-information tuple contract"
        );
    }
    ensure!(!outcome.axes.is_empty(), "{context}: axes are empty");
    ensure!(
        outcome
            .axes
            .iter()
            .all(|axis| !axis.trim().is_empty() && axis == axis.trim()),
        "{context}: an axis name is empty or has surrounding whitespace"
    );
    let unique_axes: BTreeSet<&str> = outcome.axes.iter().map(String::as_str).collect();
    ensure!(
        unique_axes.len() == outcome.axes.len(),
        "{context}: axis names are not unique"
    );
    ensure!(
        outcome.produced() == has_numeric_value,
        "{context}: computation status {:?} is inconsistent with numeric-value presence={has_numeric_value}",
        outcome.status
    );
    match outcome.status {
        OfflineVldaEstimateStatus::NotRequested => {
            ensure!(
                outcome.reason_code.is_none(),
                "{context}: not-requested outcome carries an abstention reason"
            );
            ensure!(
                outcome
                    .reason_detail
                    .as_ref()
                    .is_some_and(|detail| !detail.trim().is_empty()),
                "{context}: not-requested outcome is missing a reason detail"
            );
            ensure!(
                [
                    outcome.scientific_gates.population,
                    outcome.scientific_gates.measure,
                    outcome.scientific_gates.estimator,
                    outcome.scientific_gates.application,
                ]
                .into_iter()
                .all(|gate| gate == OfflineVldaScientificGateVerdict::NotApplicable),
                "{context}: not-requested outcome has an applicable scientific gate"
            );
            ensure!(
                !outcome.scientific_gates.interpretation_allowed,
                "{context}: not-requested outcome permits interpretation"
            );
        }
        OfflineVldaEstimateStatus::Produced => {
            ensure!(
                outcome.reason_code.is_none() && outcome.reason_detail.is_none(),
                "{context}: clean produced outcome carries an abstention/warning reason"
            );
        }
        OfflineVldaEstimateStatus::ProducedWithWarning => {
            ensure!(
                outcome.reason_code.is_none(),
                "{context}: produced-with-warning outcome carries an abstention reason"
            );
            ensure!(
                outcome
                    .reason_detail
                    .as_ref()
                    .is_some_and(|detail| !detail.trim().is_empty()),
                "{context}: produced-with-warning outcome is missing warning detail"
            );
            ensure!(
                !outcome.scientific_gates.interpretation_allowed,
                "{context}: produced-with-warning outcome permits interpretation"
            );
        }
        OfflineVldaEstimateStatus::Abstained => {
            let reason = outcome.reason_code.context(format!(
                "{context}: abstention is missing its stable reason code"
            ))?;
            ensure!(
                outcome
                    .reason_detail
                    .as_ref()
                    .is_some_and(|detail| !detail.trim().is_empty()),
                "{context}: abstention is missing reason detail"
            );
            ensure!(
                outcome.scientific_gates.reason_code.as_deref() == Some(reason.as_str()),
                "{context}: abstention and scientific-gate reason codes disagree"
            );
            ensure!(
                !outcome.scientific_gates.interpretation_allowed,
                "{context}: an abstained estimate cannot permit interpretation"
            );
            if reason == OfflineVldaAbstainReason::TupleSupportContractUnspecified {
                ensure!(
                    outcome.declared_continuous_tuple_support.is_none(),
                    "{context}: missing-tuple abstention carries a tuple declaration"
                );
            }
            if reason == OfflineVldaAbstainReason::DeclaredTupleSupportIncompatibleContinuous {
                ensure!(
                    outcome
                        .declared_continuous_tuple_support
                        .is_some_and(|support| !support.is_regular()),
                    "{context}: incompatible-tuple abstention lacks its non-regular tuple declaration"
                );
            }
        }
    }
    ensure!(
        outcome
            .scientific_gates
            .reason_code
            .as_ref()
            .is_none_or(|code| !code.trim().is_empty() && code == code.trim()),
        "{context}: scientific-gate reason code is empty or has surrounding whitespace"
    );
    ensure!(
        outcome
            .scientific_gates
            .support_envelope_version
            .as_ref()
            .is_none_or(|version| !version.trim().is_empty() && version == version.trim()),
        "{context}: support-envelope version is empty or has surrounding whitespace"
    );
    if outcome.scientific_gates.interpretation_allowed {
        ensure!(
            [
                outcome.scientific_gates.population,
                outcome.scientific_gates.measure,
                outcome.scientific_gates.estimator,
                outcome.scientific_gates.application,
            ]
            .into_iter()
            .all(|gate| gate == OfflineVldaScientificGateVerdict::Passed),
            "{context}: interpretation is allowed although at least one scientific gate did not pass"
        );
        ensure!(
            outcome.scientific_gates.support_envelope_version.is_some(),
            "{context}: interpretation is allowed without a support-envelope version"
        );
    }
    Ok(())
}

fn validate_mi_estimate(estimate: &OfflineVldaMiEstimate, context: &str) -> Result<()> {
    if let Some(value) = estimate.value {
        ensure!(value.is_finite(), "{context}: numeric value is non-finite");
    }
    validate_outcome_contract(&estimate.outcome, estimate.value.is_some(), context)
}

fn validate_pid_pair(pair_name: &str, pair: &OfflineVldaPidPairMetrics) -> Result<()> {
    let context = format!("PID pair {pair_name}");
    ensure!(
        !pair.source_1.is_empty() && !pair.source_2.is_empty() && !pair.target.is_empty(),
        "{context}: source/target identity is empty"
    );
    ensure!(
        pair.source_1 != pair.source_2,
        "{context}: source identities must be distinct"
    );
    let values = [
        pair.mi_source_1_action,
        pair.mi_source_2_action,
        pair.mi_joint_action,
        pair.co_information,
        pair.redundancy,
        pair.unique_source_1,
        pair.unique_source_2,
        pair.synergy,
    ];
    ensure!(
        values.iter().flatten().all(|value| value.is_finite()),
        "{context}: a numeric value is non-finite"
    );
    let present = values.iter().filter(|value| value.is_some()).count();
    ensure!(
        present == 0 || present == values.len(),
        "{context}: PID atom/MI vector is only partially present"
    );
    validate_outcome_contract(&pair.outcome, present == values.len(), &context)?;
    if let (
        Some(mi_source_1),
        Some(mi_source_2),
        Some(mi_joint),
        Some(co_information),
        Some(redundancy),
        Some(unique_source_1),
        Some(unique_source_2),
        Some(synergy),
    ) = (
        pair.mi_source_1_action,
        pair.mi_source_2_action,
        pair.mi_joint_action,
        pair.co_information,
        pair.redundancy,
        pair.unique_source_1,
        pair.unique_source_2,
        pair.synergy,
    ) {
        for (identity, actual, expected) in [
            (
                "unique_source_1 = MI(source_1;target) - redundancy",
                unique_source_1,
                mi_source_1 - redundancy,
            ),
            (
                "unique_source_2 = MI(source_2;target) - redundancy",
                unique_source_2,
                mi_source_2 - redundancy,
            ),
            (
                "synergy = MI(joint;target) - MI(source_1;target) - MI(source_2;target) + redundancy",
                synergy,
                mi_joint - mi_source_1 - mi_source_2 + redundancy,
            ),
            (
                "co_information = MI(source_1;target) + MI(source_2;target) - MI(joint;target)",
                co_information,
                mi_source_1 + mi_source_2 - mi_joint,
            ),
            (
                "redundancy + unique_source_1 + unique_source_2 + synergy = MI(joint;target)",
                redundancy + unique_source_1 + unique_source_2 + synergy,
                mi_joint,
            ),
        ] {
            ensure!(
                approximately_equal_f64(actual, expected),
                "{context}: PID identity failed: {identity}; actual={actual}, expected={expected}"
            );
        }
    }
    match (&pair.categorical_sx_components, pair.outcome.measure.as_str()) {
        (Some(components), MEASURE_CATEGORICAL_PID2) => {
            let component_atoms = [
                ("redundancy", components.redundancy, pair.redundancy),
                (
                    "unique_source_1",
                    components.unique_source_1,
                    pair.unique_source_1,
                ),
                (
                    "unique_source_2",
                    components.unique_source_2,
                    pair.unique_source_2,
                ),
                ("synergy", components.synergy, pair.synergy),
            ];
            for (name, atom, reported_net) in component_atoms {
                ensure!(
                    [atom.informative, atom.misinformative, atom.net]
                        .into_iter()
                        .all(f64::is_finite),
                    "{context}: categorical shared-exclusions {name} component is non-finite"
                );
                let tolerance = 256.0 * f64::EPSILON;
                ensure!(
                    atom.informative >= -tolerance && atom.misinformative >= -tolerance,
                    "{context}: categorical shared-exclusions {name} has a negative informative or misinformative component"
                );
                ensure!(
                    approximately_equal_f64(atom.net, atom.informative - atom.misinformative),
                    "{context}: categorical shared-exclusions {name} does not satisfy net = informative - misinformative"
                );
                ensure!(
                    reported_net.is_some_and(|value| value.to_bits() == atom.net.to_bits()),
                    "{context}: categorical shared-exclusions {name} net does not match its report atom"
                );
            }
        }
        (None, MEASURE_CATEGORICAL_PID2) => bail!(
            "{context}: categorical shared-exclusions result lacks informative/misinformative components"
        ),
        (Some(_), _) => bail!(
            "{context}: a non-categorical estimate carries categorical shared-exclusions components"
        ),
        (None, _) => {}
    }
    if let Some(saturation) = &pair.categorical_saturation {
        let fractions = [
            saturation.unique_fraction_source_1,
            saturation.unique_fraction_source_2,
            saturation.unique_fraction_target,
            saturation.unique_fraction_joint,
        ];
        ensure!(
            fractions
                .into_iter()
                .all(|fraction| fraction.is_finite() && (0.0..=1.0).contains(&fraction)),
            "{context}: categorical saturation fraction is outside [0, 1]"
        );
        ensure!(
            saturation.empirical_sample_count > 0
                && saturation.observed_joint_states > 0
                && saturation.observed_joint_states <= saturation.empirical_sample_count
                && saturation.singleton_joint_states <= saturation.observed_joint_states
                && saturation.low_count_joint_states <= saturation.observed_joint_states
                && saturation.minimum_observed_count > 0
                && saturation.maximum_observed_count >= saturation.minimum_observed_count
                && saturation.maximum_observed_count <= saturation.empirical_sample_count,
            "{context}: categorical empirical-PMF occupancy counts are inconsistent"
        );
        ensure!(
            saturation.observed_coverage_indicator.is_finite()
                && (0.0..=1.0).contains(&saturation.observed_coverage_indicator),
            "{context}: categorical empirical-PMF coverage indicator is outside [0, 1]"
        );
        ensure!(
            approximately_equal_f64(
                saturation.unique_fraction_joint,
                saturation.observed_joint_states as f64 / saturation.empirical_sample_count as f64,
            ),
            "{context}: categorical joint-state fraction contradicts the empirical-PMF occupancy"
        );
        ensure!(
            saturation.population_caveat
                == "direct evaluation on the empirical categorical PMF; unseen population states have empirical probability zero and plug-in bias remains",
            "{context}: categorical empirical-PMF population caveat is missing or changed"
        );
        ensure!(
            pair.outcome.produced(),
            "{context}: an unproduced estimate carries saturation diagnostics"
        );
        let expected_warning = fractions
            .into_iter()
            .any(|fraction| fraction > OFFLINE_CATEGORICAL_SATURATION_UNIQUE_FRACTION_MAX);
        ensure!(
            saturation.saturation_warning == expected_warning,
            "{context}: categorical saturation warning contradicts the recorded fractions"
        );
        ensure!(
            if expected_warning {
                pair.outcome.status == OfflineVldaEstimateStatus::ProducedWithWarning
                    && pair.outcome.scientific_gates.estimator
                        == OfflineVldaScientificGateVerdict::Blocked
                    && matches!(
                        pair.outcome.scientific_gates.reason_code.as_deref(),
                        Some(SCIENTIFIC_REASON_CATEGORICAL_SATURATION)
                            | Some(SCIENTIFIC_REASON_SUPERVISED_SAME_ROW_AND_SATURATION)
                    )
            } else {
                pair.outcome.status == OfflineVldaEstimateStatus::Produced
                    || (pair.outcome.status == OfflineVldaEstimateStatus::ProducedWithWarning
                        && pair.outcome.scientific_gates.estimator
                            == OfflineVldaScientificGateVerdict::Blocked
                        && pair.outcome.scientific_gates.reason_code.as_deref()
                            == Some(SCIENTIFIC_REASON_SUPERVISED_SAME_ROW))
            },
            "{context}: categorical saturation state contradicts the computation outcome"
        );
    }
    Ok(())
}

fn approximately_equal_f64(left: f64, right: f64) -> bool {
    if left.to_bits() == right.to_bits() {
        return true;
    }
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 256.0 * f64::EPSILON * scale
}

fn same_optional_f64(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        (None, None) => true,
        _ => false,
    }
}

fn validate_pid_screen_contract(
    context: &str,
    mi_estimates: [&OfflineVldaMiEstimate; 3],
    mirrored_vl: [(&str, Option<f64>); 6],
    pid_pairs: &BTreeMap<String, OfflineVldaPidPairMetrics>,
    denominators: &OfflineVldaEstimateDenominators,
) -> Result<()> {
    for (axis, estimate) in ["V", "L", "D"].into_iter().zip(mi_estimates) {
        validate_mi_estimate(estimate, &format!("{context} MI({axis};A)"))?;
    }
    for (pair_name, pair) in pid_pairs {
        validate_pid_pair(pair_name, pair)?;
    }

    for (axis, estimate, sources) in [
        ("V", mi_estimates[0], [("VL", true), ("VD", true)]),
        ("L", mi_estimates[1], [("VL", false), ("LD", true)]),
        ("D", mi_estimates[2], [("VD", false), ("LD", false)]),
    ] {
        let pair_values = sources.map(|(pair_name, first_source)| {
            pid_pairs.get(pair_name).and_then(|pair| {
                if first_source {
                    pair.mi_source_1_action
                } else {
                    pair.mi_source_2_action
                }
            })
        });
        if let (Some(left), Some(right)) = (pair_values[0], pair_values[1]) {
            ensure!(
                approximately_equal_f64(left, right),
                "{context}: repeated MI({axis};A) terms disagree across PID pairs: {left} versus {right}"
            );
        }
        if let (Some(marginal), Some(pair_value)) =
            (estimate.value, pair_values.into_iter().flatten().next())
        {
            ensure!(
                approximately_equal_f64(marginal, pair_value),
                "{context}: MI({axis};A) does not match its PID-pair marginal: {marginal} versus {pair_value}"
            );
        }
    }

    let expected_vl = pid_pairs.get("VL").map(|pair| {
        [
            pair.mi_joint_action,
            pair.co_information,
            pair.redundancy,
            pair.unique_source_1,
            pair.unique_source_2,
            pair.synergy,
        ]
    });
    for (index, (name, actual)) in mirrored_vl.into_iter().enumerate() {
        let expected = expected_vl.and_then(|values| values[index]);
        ensure!(
            same_optional_f64(actual, expected),
            "{context}: mirrored VL field {name} does not exactly match the VL pair"
        );
    }

    let mut expected_denominators = OfflineVldaEstimateDenominators::default();
    for estimate in mi_estimates {
        expected_denominators.record(&estimate.outcome);
    }
    for pair in pid_pairs.values() {
        expected_denominators.record(&pair.outcome);
    }
    ensure!(
        denominators == &expected_denominators,
        "{context}: estimate denominators do not reconstruct from typed outcomes"
    );
    Ok(())
}

fn validate_pid_screen_metrics(metrics: &OfflineVldaPidScreenMetrics, context: &str) -> Result<()> {
    validate_pid_screen_contract(
        context,
        [
            &metrics.mi_v_action,
            &metrics.mi_l_action,
            &metrics.mi_d_action,
        ],
        [
            ("mi_vl_action", metrics.mi_vl_action),
            (
                "co_information_v_l_action",
                metrics.co_information_v_l_action,
            ),
            ("redundancy_v_l_action", metrics.redundancy_v_l_action),
            ("unique_v_action", metrics.unique_v_action),
            ("unique_l_action", metrics.unique_l_action),
            ("synergy_v_l_action", metrics.synergy_v_l_action),
        ],
        &metrics.pid_pairs,
        &metrics.estimate_denominators,
    )?;
    if let Some(control) = &metrics.pls_shuffled_target_control {
        ensure!(
            control.pls_shuffled_target_control.is_none(),
            "{context}: shuffled-target control nests another control"
        );
        validate_pid_screen_contract(
            &format!("{context} shuffled-target control"),
            [
                &control.mi_v_action,
                &control.mi_l_action,
                &control.mi_d_action,
            ],
            [
                ("mi_vl_action", control.mi_vl_action),
                (
                    "co_information_v_l_action",
                    control.co_information_v_l_action,
                ),
                ("redundancy_v_l_action", control.redundancy_v_l_action),
                ("unique_v_action", control.unique_v_action),
                ("unique_l_action", control.unique_l_action),
                ("synergy_v_l_action", control.synergy_v_l_action),
            ],
            &control.pid_pairs,
            &control.estimate_denominators,
        )?;
    }
    Ok(())
}

fn validate_pls_selection_contract(
    selection: &OfflineVldaPlsSelection,
    configured: PlsComponentSelection,
    context: &str,
) -> Result<()> {
    ensure!(
        selection.components_v > 0 && selection.components_l > 0 && selection.components_d > 0,
        "{context}: PLS selection contains a zero component count"
    );
    match configured {
        PlsComponentSelection::Fixed(components) => ensure!(
            selection.method == "fixed"
                && selection.components_v == components
                && selection.components_l == components
                && selection.components_d == components
                && selection.q2_v.is_none()
                && selection.q2_l.is_none()
                && selection.q2_d.is_none(),
            "{context}: fixed PLS selection contradicts the recorded configuration"
        ),
        PlsComponentSelection::CvQ2 { max_components } => {
            ensure!(
                selection.method == "cv_q2"
                    && selection.components_v <= max_components
                    && selection.components_l <= max_components
                    && selection.components_d <= max_components,
                "{context}: CV PLS selection contradicts the recorded maximum"
            );
            ensure!(
                [selection.q2_v, selection.q2_l, selection.q2_d]
                    .into_iter()
                    .all(|q2| q2.is_some_and(f64::is_finite)),
                "{context}: CV PLS selection lacks a finite selected Q-squared value"
            );
        }
    }
    Ok(())
}

fn validate_categorical_warning_contract(
    outcome: &OfflineVldaOutcome,
    pid_mode: PidMode,
    saturation_warning: Option<bool>,
    context: &str,
) -> Result<()> {
    let expected_reason = match (pid_mode, saturation_warning.unwrap_or(false)) {
        (PidMode::CategoricalSxPls, true) => {
            Some(SCIENTIFIC_REASON_SUPERVISED_SAME_ROW_AND_SATURATION)
        }
        (PidMode::CategoricalSxPls, false) => Some(SCIENTIFIC_REASON_SUPERVISED_SAME_ROW),
        (PidMode::CategoricalSx, true) => Some(SCIENTIFIC_REASON_CATEGORICAL_SATURATION),
        (PidMode::CategoricalSx, false) => None,
        _ => return Ok(()),
    };
    match expected_reason {
        Some(reason) => ensure!(
            outcome.status == OfflineVldaEstimateStatus::ProducedWithWarning
                && outcome.scientific_gates.estimator == OfflineVldaScientificGateVerdict::Blocked
                && outcome.scientific_gates.application
                    == OfflineVldaScientificGateVerdict::Blocked
                && !outcome.scientific_gates.interpretation_allowed
                && outcome.scientific_gates.reason_code.as_deref() == Some(reason)
                && outcome
                    .reason_detail
                    .as_ref()
                    .is_some_and(|detail| !detail.trim().is_empty()),
            "{context}: categorical warning does not match the fitted-preprocessing contract"
        ),
        None => ensure!(
            outcome.status == OfflineVldaEstimateStatus::Produced,
            "{context}: warning is not justified by saturation or supervised same-row preprocessing"
        ),
    }
    Ok(())
}

struct OfflineVldaPidScreenValidation<'a> {
    mi_estimates: [&'a OfflineVldaMiEstimate; 3],
    pid_pairs: &'a BTreeMap<String, OfflineVldaPidPairMetrics>,
    quantization: &'a BTreeMap<String, OfflineVldaQuantizationReceipt>,
    pls_selection: Option<&'a OfflineVldaPlsSelection>,
    pls_control: Option<&'a OfflineVldaPidScreenMetrics>,
    pls_control_seed: Option<u64>,
    expects_pls_control: bool,
    context: &'a str,
}

fn validate_pid_mode_screen_contract(
    screen: OfflineVldaPidScreenValidation<'_>,
    options: &OfflineVldaHarnessOptions,
) -> Result<()> {
    let OfflineVldaPidScreenValidation {
        mi_estimates,
        pid_pairs,
        quantization,
        pls_selection,
        pls_control,
        pls_control_seed,
        expects_pls_control,
        context,
    } = screen;
    if options.pid_mode == PidMode::Disabled {
        for (axis, estimate) in ["V", "L", "D"].into_iter().zip(mi_estimates) {
            let expected = OfflineVldaMiEstimate {
                outcome: not_requested_outcome(&[axis, "A"]),
                value: None,
            };
            ensure!(
                estimate == &expected,
                "{context}: disabled PID mode carries a requested MI({axis};A) outcome"
            );
        }
        ensure!(
            pid_pairs.is_empty()
                && quantization.is_empty()
                && pls_selection.is_none()
                && pls_control.is_none()
                && pls_control_seed.is_none(),
            "{context}: disabled PID mode carries PID, quantization, or PLS results"
        );
        return Ok(());
    }

    let (mi_measure, pid_measure) = match options.pid_mode {
        PidMode::Continuous => (MEASURE_CONTINUOUS_MI, MEASURE_CONTINUOUS_PID2),
        PidMode::CategoricalSx | PidMode::CategoricalSxPls => {
            (MEASURE_CATEGORICAL_MI, MEASURE_CATEGORICAL_PID2)
        }
        PidMode::Disabled => unreachable!("disabled mode returned above"),
    };
    if matches!(
        options.pid_mode,
        PidMode::CategoricalSx | PidMode::CategoricalSxPls
    ) {
        ensure!(
            quantization.len() == 4,
            "{context}: categorical Sx screen must bind exactly V, L, D, and A quantizers"
        );
        for axis in ["V", "L", "D", "A"] {
            let receipt = quantization
                .get(axis)
                .with_context(|| format!("{context}: missing {axis} quantizer receipt"))?;
            let nominal_joint_cardinality = receipt
                .nominal_joint_cardinality
                .as_deref()
                .map(str::parse::<u128>)
                .transpose()
                .with_context(|| {
                    format!("{context}: {axis} nominal joint cardinality is not a u128")
                })?;
            let empty_joint_cells = receipt
                .empty_joint_cells
                .as_deref()
                .map(str::parse::<u128>)
                .transpose()
                .with_context(|| {
                    format!("{context}: {axis} empty joint-cell count is not a u128")
                })?;
            ensure!(
                receipt.axis == axis
                    && receipt.functional
                        == "Makkeh-Gutknecht-Wibral averaged two-source categorical shared exclusions"
                    && receipt.quantizer
                        == "pid_core::stable::quantized::EqualWidthQuantizer"
                    && receipt.estimator_revision == ESTIMATOR_CATEGORICAL_PID2
                    && receipt.information_units == "nats"
                    && receipt.bins_per_dimension == options.categorical_bins
                    && receipt.samples > 0
                    && receipt.dimensions > 0
                    && receipt.fitted_edge_count
                        == receipt
                            .dimensions
                            .checked_mul(
                                receipt
                                    .bins_per_dimension
                                    .checked_add(1)
                                    .context("quantizer receipt bin-edge count overflow")?,
                            )
                            .context("quantizer receipt edge-count overflow")?
                    && receipt.out_of_range_policy == "error"
                    && receipt.scaling_description
                        == "per-variable standardization followed by fitted equal-width bins"
                    && receipt.estimand_statement
                        == "PID of the declared fitted equal-width quantized variables; not continuous PID"
                    && receipt.observed_joint_cardinality > 0
                    && receipt.observed_joint_cardinality <= receipt.samples
                    && receipt.low_count_joint_cells <= receipt.observed_joint_cardinality
                    && receipt.minimum_observed_cell_count > 0
                    && receipt.maximum_observed_cell_count
                        >= receipt.minimum_observed_cell_count
                    && receipt.maximum_observed_cell_count <= receipt.samples,
                "{context}: {axis} quantizer receipt contradicts the categorical Sx contract"
            );
            match (nominal_joint_cardinality, empty_joint_cells) {
                (Some(nominal), Some(empty)) => ensure!(
                    nominal >= receipt.observed_joint_cardinality as u128
                        && empty == nominal - receipt.observed_joint_cardinality as u128,
                    "{context}: {axis} quantizer receipt has inconsistent nominal and empty cardinalities"
                ),
                (None, None) => {}
                _ => bail!(
                    "{context}: {axis} quantizer receipt must record nominal and empty cardinalities together"
                ),
            }
            for digest in [
                &receipt.fitted_edges_sha256,
                &receipt.training_input_sha256,
                &receipt.transform_input_sha256,
                &receipt.categorical_output_sha256,
            ] {
                ensure!(
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
                    "{context}: {axis} quantizer receipt has an invalid SHA-256"
                );
            }
        }
    } else {
        ensure!(
            quantization.is_empty(),
            "{context}: non-categorical PID mode carries quantizer receipts"
        );
    }
    for (axis, estimate) in ["V", "L", "D"].into_iter().zip(mi_estimates) {
        ensure!(
            estimate.outcome.status != OfflineVldaEstimateStatus::NotRequested
                && estimate.outcome.measure == mi_measure
                && estimate.outcome.estimator_revision
                    == estimator_revision_for_measure(mi_measure)
                && estimate.outcome.axes == [axis.to_string(), "A".to_string()],
            "{context}: MI({axis};A) identity contradicts the recorded PID mode"
        );
        if matches!(
            options.pid_mode,
            PidMode::CategoricalSx | PidMode::CategoricalSxPls
        ) {
            ensure!(
                estimate.outcome.produced(),
                "{context}: fitted-categorical MI({axis};A) was not produced"
            );
            let saturation_warning = matches!(
                estimate.outcome.scientific_gates.reason_code.as_deref(),
                Some(SCIENTIFIC_REASON_CATEGORICAL_SATURATION)
                    | Some(SCIENTIFIC_REASON_SUPERVISED_SAME_ROW_AND_SATURATION)
            );
            validate_categorical_warning_contract(
                &estimate.outcome,
                options.pid_mode,
                Some(saturation_warning),
                &format!("{context}: fitted-categorical MI({axis};A)"),
            )?;
        }
    }

    let expected_pairs = [("VL", "V", "L"), ("VD", "V", "D"), ("LD", "L", "D")];
    ensure!(
        pid_pairs.len() == expected_pairs.len(),
        "{context}: requested PID mode must contain exactly VL, VD, and LD"
    );
    for (pair_name, source_1, source_2) in expected_pairs {
        let pair = pid_pairs
            .get(pair_name)
            .with_context(|| format!("{context}: missing PID pair {pair_name}"))?;
        ensure!(
            pair.source_1 == source_1
                && pair.source_2 == source_2
                && pair.target == "A"
                && pair.outcome.status != OfflineVldaEstimateStatus::NotRequested
                && pair.outcome.measure == pid_measure
                && pair.outcome.estimator_revision == estimator_revision_for_measure(pid_measure)
                && pair.outcome.axes
                    == [source_1.to_string(), source_2.to_string(), "A".to_string(),],
            "{context}: PID pair {pair_name} identity contradicts the recorded PID mode"
        );
        match options.pid_mode {
            PidMode::Continuous => ensure!(
                pair.categorical_saturation.is_none(),
                "{context}: continuous PID pair {pair_name} carries categorical saturation data"
            ),
            PidMode::CategoricalSx | PidMode::CategoricalSxPls => ensure!(
                pair.categorical_saturation.is_some() == pair.outcome.produced(),
                "{context}: categorical PID pair {pair_name} saturation data contradicts its outcome"
            ),
            PidMode::Disabled => unreachable!("disabled mode returned above"),
        }
        if matches!(
            options.pid_mode,
            PidMode::CategoricalSx | PidMode::CategoricalSxPls
        ) {
            validate_categorical_warning_contract(
                &pair.outcome,
                options.pid_mode,
                pair.categorical_saturation
                    .as_ref()
                    .map(|saturation| saturation.saturation_warning),
                &format!("{context}: categorical PID pair {pair_name}"),
            )?;
        }
    }

    if options.pid_mode == PidMode::CategoricalSxPls {
        let selection = pls_selection.with_context(|| {
            format!("{context}: categorical-sx-pls mode lacks PLS selection data")
        })?;
        validate_pls_selection_contract(selection, options.pls, context)?;
        if expects_pls_control {
            ensure!(
                pls_control_seed == Some(PLS_CONTROL_SEED),
                "{context}: categorical-sx-pls control seed is absent or incorrect"
            );
            let control = pls_control.with_context(|| {
                format!("{context}: categorical-sx-pls mode lacks its shuffled-target control")
            })?;
            validate_pid_mode_screen_contract(
                OfflineVldaPidScreenValidation {
                    mi_estimates: [
                        &control.mi_v_action,
                        &control.mi_l_action,
                        &control.mi_d_action,
                    ],
                    pid_pairs: &control.pid_pairs,
                    quantization: &control.categorical_quantization,
                    pls_selection: control.pls_selection.as_ref(),
                    pls_control: control.pls_shuffled_target_control.as_deref(),
                    pls_control_seed: control.pls_control_seed,
                    expects_pls_control: false,
                    context: &format!("{context} shuffled-target control"),
                },
                options,
            )?;
        } else {
            ensure!(
                pls_control.is_none() && pls_control_seed.is_none(),
                "{context}: shuffled-target control nests another control"
            );
        }
    } else {
        ensure!(
            pls_selection.is_none() && pls_control.is_none() && pls_control_seed.is_none(),
            "{context}: non-PLS PID mode carries PLS selection or control data"
        );
    }
    Ok(())
}

fn validate_report_pid_mode_contract(
    report: &OfflineVldaReport,
    options: &OfflineVldaHarnessOptions,
) -> Result<()> {
    validate_pid_mode_screen_contract(
        OfflineVldaPidScreenValidation {
            mi_estimates: [
                &report.metrics.mi_v_action,
                &report.metrics.mi_l_action,
                &report.metrics.mi_d_action,
            ],
            pid_pairs: &report.metrics.pid_pairs,
            quantization: &report.metrics.categorical_quantization,
            pls_selection: report.metrics.pls_selection.as_ref(),
            pls_control: report.metrics.pls_shuffled_target_control.as_deref(),
            pls_control_seed: report.metrics.pls_control_seed,
            expects_pls_control: true,
            context: "full-data PID screen",
        },
        options,
    )?;
    if let Some(train) = &report.train_split_pid {
        match (&train.metrics, options.pid_mode) {
            (Some(_), PidMode::Disabled) => {
                bail!("train-split PID screen carries metrics while PID is disabled")
            }
            (Some(metrics), _) => {
                ensure!(
                    train.status == "available"
                        && train.preprocessing.is_some()
                        && train.error.is_none(),
                    "available train-split PID screen has inconsistent status fields"
                );
                validate_pid_mode_screen_contract(
                    OfflineVldaPidScreenValidation {
                        mi_estimates: [
                            &metrics.mi_v_action,
                            &metrics.mi_l_action,
                            &metrics.mi_d_action,
                        ],
                        pid_pairs: &metrics.pid_pairs,
                        quantization: &metrics.categorical_quantization,
                        pls_selection: metrics.pls_selection.as_ref(),
                        pls_control: metrics.pls_shuffled_target_control.as_deref(),
                        pls_control_seed: metrics.pls_control_seed,
                        expects_pls_control: true,
                        context: "train-split PID screen",
                    },
                    options,
                )?;
            }
            (None, PidMode::Disabled) => ensure!(
                train.status == "disabled"
                    && train.preprocessing.is_none()
                    && train.error.is_none(),
                "train-split PID screen does not record a consistent disabled status"
            ),
            (None, _) => ensure!(
                train.status == "error"
                    && train.preprocessing.is_none()
                    && train.error.as_ref().is_some_and(|value| !value.is_empty()),
                "unavailable train-split PID screen lacks a typed error status"
            ),
        }
    }
    Ok(())
}

fn validate_report_continuous_tuple_bindings(
    report: &OfflineVldaReport,
    options: &OfflineVldaHarnessOptions,
) -> Result<()> {
    let declared: BTreeMap<String, OfflineVldaContinuousTupleSupport> = serde_json::from_value(
        report
            .config
            .get("continuous_tuple_support")
            .context("offline VLDA report lacks continuous tuple-support declarations")?
            .clone(),
    )
    .context("offline VLDA report has malformed continuous tuple-support declarations")?;
    ensure!(
        declared
            .keys()
            .all(|key| CONTINUOUS_TUPLE_KEYS.contains(&key.as_str())),
        "offline VLDA report has an unknown continuous tuple-support key"
    );
    let expected_contract = if options.pid_mode == PidMode::Continuous {
        "each_complete_mi_or_pid_tuple_requires_a_caller_declared_regular_full_dimensional_finite_information_joint_law"
    } else {
        "not_applicable"
    };
    ensure!(
        report.config["metric_pipeline"]["continuous_support_contract"] == expected_contract,
        "offline VLDA report has an invalid continuous-support contract description"
    );
    if options.pid_mode != PidMode::Continuous {
        return Ok(());
    }
    let validate_outcomes =
        |outcomes: [(&str, &OfflineVldaOutcome); 6], context: &str| -> Result<()> {
            for (key, outcome) in outcomes {
                ensure!(
                    outcome.declared_continuous_tuple_support == declared.get(key).copied(),
                    "{context}: {key} outcome contradicts the report tuple-support declaration"
                );
            }
            Ok(())
        };
    validate_outcomes(
        [
            (CONTINUOUS_TUPLE_V_A, &report.metrics.mi_v_action.outcome),
            (CONTINUOUS_TUPLE_L_A, &report.metrics.mi_l_action.outcome),
            (CONTINUOUS_TUPLE_D_A, &report.metrics.mi_d_action.outcome),
            (
                CONTINUOUS_TUPLE_V_L_A,
                &report.metrics.pid_pairs["VL"].outcome,
            ),
            (
                CONTINUOUS_TUPLE_V_D_A,
                &report.metrics.pid_pairs["VD"].outcome,
            ),
            (
                CONTINUOUS_TUPLE_L_D_A,
                &report.metrics.pid_pairs["LD"].outcome,
            ),
        ],
        "full-data PID screen",
    )?;
    if let Some(metrics) = report
        .train_split_pid
        .as_ref()
        .and_then(|train| train.metrics.as_ref())
    {
        validate_outcomes(
            [
                (CONTINUOUS_TUPLE_V_A, &metrics.mi_v_action.outcome),
                (CONTINUOUS_TUPLE_L_A, &metrics.mi_l_action.outcome),
                (CONTINUOUS_TUPLE_D_A, &metrics.mi_d_action.outcome),
                (CONTINUOUS_TUPLE_V_L_A, &metrics.pid_pairs["VL"].outcome),
                (CONTINUOUS_TUPLE_V_D_A, &metrics.pid_pairs["VD"].outcome),
                (CONTINUOUS_TUPLE_L_D_A, &metrics.pid_pairs["LD"].outcome),
            ],
            "train-split PID screen",
        )?;
    }
    Ok(())
}

#[derive(Debug)]
struct ReportResourceBinding {
    limits: OfflineVldaResourceLimits,
    usage: OfflineVldaResourceUsage,
    options: OfflineVldaHarnessOptions,
    uncertainty_config: Option<OfflineVldaUncertaintyConfig>,
}

fn deserialize_config_value<T: DeserializeOwned>(
    value: &Value,
    pointer: &str,
    description: &str,
) -> Result<T> {
    let field = value
        .pointer(pointer)
        .with_context(|| format!("offline VLDA report configuration is missing {description}"))?;
    serde_json::from_value(field.clone())
        .with_context(|| format!("offline VLDA report configuration has invalid {description}"))
}

fn parse_recorded_permutation_scheme(label: &str, n_perm: usize) -> Result<PermutationScheme> {
    if n_perm == 0 {
        ensure!(
            label == "not_requested",
            "offline VLDA report records a permutation scheme without permutations"
        );
        return Ok(PermutationScheme::FullShuffle);
    }
    if label == "full_shuffle" {
        return Ok(PermutationScheme::FullShuffle);
    }
    if let Some(raw) = label
        .strip_prefix("circular_shift(min_shift=")
        .and_then(|value| value.strip_suffix(')'))
    {
        let min_shift = raw
            .parse::<usize>()
            .context("offline VLDA report has an invalid circular-shift minimum")?;
        return Ok(PermutationScheme::CircularShift { min_shift });
    }
    if let Some(raw) = label
        .strip_prefix("block_shuffle(block_size=")
        .and_then(|value| value.strip_suffix(')'))
    {
        let block_size = raw
            .parse::<usize>()
            .context("offline VLDA report has an invalid block-shuffle size")?;
        return Ok(PermutationScheme::BlockShuffle { block_size });
    }
    bail!("offline VLDA report has an unsupported permutation scheme: {label}")
}

fn parse_recorded_pls_selection(value: &Value) -> Result<PlsComponentSelection> {
    if let Some(components) = value.as_u64() {
        return Ok(PlsComponentSelection::Fixed(
            usize::try_from(components)
                .context("offline VLDA report fixed PLS component count does not fit usize")?,
        ));
    }
    let fields = value.as_object().context(
        "offline VLDA report PLS component selection is neither an integer nor an object",
    )?;
    ensure!(
        fields.len() == 1 && fields.contains_key("cv_max"),
        "offline VLDA report PLS CV selection must contain only cv_max"
    );
    let max_components = fields["cv_max"]
        .as_u64()
        .context("offline VLDA report PLS CV maximum is not an unsigned integer")?;
    Ok(PlsComponentSelection::CvQ2 {
        max_components: usize::try_from(max_components)
            .context("offline VLDA report PLS CV maximum does not fit usize")?,
    })
}

fn report_resource_binding(report: &OfflineVldaReport) -> Result<ReportResourceBinding> {
    let limits: OfflineVldaResourceLimits =
        deserialize_config_value(&report.config, "/resource_limits", "resource limits")?;
    validate_resource_limits(&limits)?;
    let usage: OfflineVldaResourceUsage =
        deserialize_config_value(&report.config, "/resource_usage", "resource usage")?;
    let pid_mode: PidMode =
        deserialize_config_value(&report.config, "/metric_pipeline/pid_mode", "PID mode")?;
    let categorical_bins: usize = deserialize_config_value(
        &report.config,
        "/metric_pipeline/categorical_bins",
        "categorical Sx bin count",
    )?;
    let pls_value = report
        .config
        .pointer("/metric_pipeline/pls_components")
        .context("offline VLDA report configuration is missing its PLS component selection")?;
    let options = OfflineVldaHarnessOptions {
        pid_mode,
        categorical_bins,
        pls: parse_recorded_pls_selection(pls_value)?,
    };
    validate_harness_options(&options)?;

    ensure!(
        usage.samples == report.dims.samples,
        "offline VLDA report resource sample count contradicts its dimensions"
    );
    let width = [report.dims.v, report.dims.l, report.dims.d, report.dims.a]
        .into_iter()
        .try_fold(0_u128, |sum, dimension| {
            checked_work_add(sum, dimension as u128, "recorded total axis width")
        })?;
    let expected_axis_scalars = checked_work_mul(
        report.dims.samples as u128,
        width,
        "recorded total axis scalars",
    )?;
    ensure!(
        usage.total_axis_scalars as u128 == expected_axis_scalars,
        "offline VLDA report axis-scalar usage contradicts its dimensions"
    );
    for (resource, observed, limit) in [
        (
            "axis scalars",
            usage.total_axis_scalars,
            limits.max_total_axis_scalars,
        ),
        (
            "metadata entries",
            usage.total_metadata_entries,
            limits.max_total_metadata_entries,
        ),
        (
            "metadata JSON nodes",
            usage.total_metadata_json_nodes,
            limits.max_total_metadata_json_nodes,
        ),
        (
            "metadata UTF-8 bytes",
            usage.total_metadata_utf8_bytes,
            limits.max_total_metadata_utf8_bytes,
        ),
        (
            "metadata JSON depth",
            usage.metadata_json_depth,
            limits.max_metadata_json_depth,
        ),
    ] {
        ensure!(
            observed <= limit,
            "offline VLDA report resource usage exceeds its {resource} limit"
        );
    }
    ensure!(
        usage.samples <= limits.max_samples,
        "offline VLDA report resource usage exceeds its sample limit"
    );
    let projected_total = usage
        .projected_main_pairwise_distance_evaluations
        .checked_add(usage.projected_uncertainty_pairwise_distance_evaluations)
        .context("offline VLDA report projected pairwise total overflowed u64")?;
    ensure!(
        projected_total == usage.projected_total_pairwise_distance_evaluations,
        "offline VLDA report projected pairwise total does not equal main plus uncertainty"
    );
    ensure!(
        projected_total <= limits.max_pairwise_distance_evaluations,
        "offline VLDA report projected pairwise usage exceeds its limit"
    );
    let vl_width = checked_work_add(
        report.dims.v as u128,
        report.dims.l as u128,
        "recorded V/L distance-vector width",
    )?;
    let da_width = checked_work_add(
        report.dims.d as u128,
        report.dims.a as u128,
        "recorded D/A distance-vector width",
    )?;
    let maximum_vector_width =
        checked_work_add(vl_width, da_width, "recorded V/L/D/A distance-vector width")?.max(1);
    let expected_main_coordinates = projected_distance_coordinate_evaluations(
        u128::from(usage.projected_main_pairwise_distance_evaluations),
        maximum_vector_width,
        "recorded main distance coordinate evaluations",
    )?;
    let expected_uncertainty_coordinates = projected_distance_coordinate_evaluations(
        u128::from(usage.projected_uncertainty_pairwise_distance_evaluations),
        maximum_vector_width,
        "recorded uncertainty distance coordinate evaluations",
    )?;
    ensure!(
        expected_main_coordinates
            == u128::from(usage.projected_main_distance_coordinate_evaluations),
        "offline VLDA report main distance coordinate projection contradicts its dimensions"
    );
    ensure!(
        expected_uncertainty_coordinates
            == u128::from(usage.projected_uncertainty_distance_coordinate_evaluations),
        "offline VLDA report uncertainty distance coordinate projection contradicts its dimensions"
    );
    let projected_total_coordinates = usage
        .projected_main_distance_coordinate_evaluations
        .checked_add(usage.projected_uncertainty_distance_coordinate_evaluations)
        .context("offline VLDA report projected distance coordinate total overflowed u64")?;
    ensure!(
        projected_total_coordinates == usage.projected_total_distance_coordinate_evaluations,
        "offline VLDA report projected distance coordinate total does not equal main plus uncertainty"
    );
    ensure!(
        projected_total_coordinates <= limits.max_distance_coordinate_evaluations,
        "offline VLDA report projected distance coordinate usage exceeds its limit"
    );
    ensure!(
        usage.projected_dense_solver_operations <= limits.max_dense_solver_operations,
        "offline VLDA report projected dense-solver usage exceeds its limit"
    );
    ensure!(
        usage.projected_categorical_pid_operations <= limits.max_categorical_pid_operations,
        "offline VLDA report projected categorical-PID usage exceeds its limit"
    );

    let pairwise_scope = report
        .config
        .pointer("/resource_accounting/pairwise_limit_scope")
        .and_then(Value::as_str)
        .context("offline VLDA report configuration is missing its pairwise-limit scope")?;
    let uncertainty_config = match pairwise_scope {
        "single_main_analysis_call" => {
            ensure!(
                report.config["resource_accounting"]
                    == json!({
                        "pairwise_limit_scope": "single_main_analysis_call",
                        "resource_usage_scope": "main_harness_analysis",
                        "distance_projection_model": "pairwise_units_times_max_combined_axis_width_v2",
                        "dense_solver_projection_model": "pid_core_0_9_logreg_pls_worst_case_v1",
                        "categorical_pid_projection_model": "pid_core_0_9_fitted_quantization_and_two_source_averaged_sxpid_v1",
                        "optional_uncertainty": "not_included_by_single_analysis_api",
                    }),
                "offline VLDA report has an invalid single-analysis resource-accounting contract"
            );
            ensure!(
                report.config["uncertainty_request"]
                    == json!({
                        "enabled": false,
                        "scope": "not_requested_by_this_api",
                    }),
                "offline VLDA single-analysis report carries an invalid uncertainty request"
            );
            ensure!(
                usage.projected_uncertainty_pairwise_distance_evaluations == 0,
                "offline VLDA single-analysis report carries uncertainty work"
            );
            None
        }
        "aggregate_main_and_optional_uncertainty" => {
            ensure!(
                report.config["resource_accounting"]
                    == json!({
                        "pairwise_limit_scope": "aggregate_main_and_optional_uncertainty",
                        "resource_usage_scope": "complete_cli_invocation_projection",
                        "distance_projection_model": "pairwise_units_times_max_combined_axis_width_v2",
                        "dense_solver_projection_model": "pid_core_0_9_logreg_pls_worst_case_v1",
                        "categorical_pid_projection_model": "pid_core_0_9_fitted_quantization_and_two_source_averaged_sxpid_v1",
                        "optional_uncertainty": "included_or_typed_skip_in_aggregate_preflight",
                    }),
                "offline VLDA report has an invalid aggregate resource-accounting contract"
            );
            let request = report
                .config
                .get("uncertainty_request")
                .context("offline VLDA aggregate report is missing its uncertainty request")?;
            let n_boot: usize = deserialize_config_value(request, "/n_boot", "bootstrap count")?;
            let n_perm: usize = deserialize_config_value(request, "/n_perm", "permutation count")?;
            let block_size: usize =
                deserialize_config_value(request, "/block_size", "uncertainty block size")?;
            let alpha: f64 = deserialize_config_value(request, "/alpha", "uncertainty tail mass")?;
            let seed: u64 = deserialize_config_value(request, "/seed", "uncertainty seed")?;
            let scheme_label = request
                .get("permutation_scheme")
                .and_then(Value::as_str)
                .context("offline VLDA aggregate report is missing its permutation scheme")?;
            let permutation_scheme = parse_recorded_permutation_scheme(scheme_label, n_perm)?;
            let topology_label = request
                .get("row_topology")
                .and_then(Value::as_str)
                .context("offline VLDA aggregate report is missing its row topology")?;
            let row_topology = OfflineVldaUncertaintyRowTopology::from_label(topology_label)?;
            let config = OfflineVldaUncertaintyConfig {
                n_boot,
                n_perm,
                block_size,
                alpha,
                seed,
                permutation_scheme,
            };
            validate_uncertainty_config(&config)?;
            let execution = uncertainty_execution_label(options.pid_mode, &config, row_topology);
            ensure!(
                request
                    == &json!({
                        "enabled": config.enabled(),
                        "preprocessing_resampling": OFFLINE_UNCERTAINTY_PREPROCESSING_RESAMPLING,
                        "n_boot": config.n_boot,
                        "n_perm": config.n_perm,
                        "block_size": config.block_size,
                        "alpha": config.alpha,
                        "seed": config.seed,
                        "permutation_scheme": if config.n_perm > 0 {
                            permutation_scheme_label(config.permutation_scheme)?
                        } else {
                            "not_requested".to_string()
                        },
                        "permutation_calibration": permutation_calibration_label(
                            config.permutation_scheme,
                            config.n_perm,
                        )?,
                        "row_topology": row_topology.label(),
                        "execution": execution,
                    }),
                "offline VLDA aggregate report carries an invalid uncertainty request"
            );
            ensure!(
                execution == "eligible_for_execution"
                    || usage.projected_uncertainty_pairwise_distance_evaluations == 0,
                "offline VLDA typed uncertainty skip carries projected computation work"
            );
            Some(config)
        }
        other => bail!("offline VLDA report has an unknown pairwise-limit scope: {other}"),
    };

    Ok(ReportResourceBinding {
        limits,
        usage,
        options,
        uncertainty_config,
    })
}

fn validate_offline_vlda_report(report: &OfflineVldaReport) -> Result<()> {
    ensure!(
        !report.run_id.is_empty(),
        "offline VLDA report run_id must not be empty"
    );
    ensure!(
        report.dims.samples >= 8,
        "offline VLDA report must describe at least 8 samples"
    );
    ensure!(
        report.dims.v > 0 && report.dims.l > 0 && report.dims.d > 0 && report.dims.a > 0,
        "offline VLDA report dimensions must be nonzero"
    );
    for (label, count) in &report.label_counts {
        ensure!(
            !label.is_empty() && *count > 0 && *count <= report.dims.samples,
            "offline VLDA report carries an invalid label count"
        );
    }
    let expected_config_hash = pid_runlog::canonical_json_hash_v2(&report.config)
        .context("failed to hash the offline VLDA report configuration")?;
    ensure!(
        report.config_hash == expected_config_hash,
        "offline VLDA report config_hash does not match its configuration"
    );
    ensure!(
        report.config.get("harness").and_then(Value::as_str) == Some("offline_vlda"),
        "offline VLDA report configuration has the wrong harness identity"
    );
    ensure!(
        report.config.get("report_schema").and_then(Value::as_str)
            == Some(OFFLINE_VLDA_REPORT_SCHEMA),
        "offline VLDA report configuration has an unsupported report schema"
    );
    let recorded_input_uri = match report.config.get("input_uri") {
        Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.as_str()),
        _ => bail!("offline VLDA report input_uri must be a string or null"),
    };
    let recorded_input_sha256 = match report.config.get("input_sha256") {
        Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.as_str()),
        _ => bail!("offline VLDA report input_sha256 must be a string or null"),
    };
    validate_optional_input_binding(recorded_input_uri, recorded_input_sha256)?;
    ensure!(
        report.config.get("samples").and_then(Value::as_u64)
            == u64::try_from(report.dims.samples).ok(),
        "offline VLDA report sample count contradicts its configuration"
    );
    let expected_dims = serde_json::to_value(&report.dims)
        .context("failed to encode the offline VLDA report dimensions")?;
    ensure!(
        report.config.get("dims") == Some(&expected_dims),
        "offline VLDA report dimensions contradict its configuration"
    );
    validate_temporal_report(&report.temporal, &report.dims)?;
    let resource_binding = report_resource_binding(report)?;
    let expected_metric_pipeline = offline_vlda_metric_pipeline_config(
        &resource_binding.options,
        OfflineVldaMetricPipelineInputs {
            preprocessing: &report.preprocessing,
            geometry: &report.geometry,
            temporal: &report.temporal,
            train_split_pid: report.train_split_pid.as_ref(),
            heldout_split: report.heldout_split.as_ref(),
            heldout_class_coverage: report.heldout_class_coverage.as_ref(),
            heldout_episode_disjoint: report.heldout_episode_disjoint.as_ref(),
        },
    );
    ensure!(
        report.config.get("metric_pipeline") == Some(&expected_metric_pipeline),
        "offline VLDA report metric-pipeline configuration does not reconstruct from the report"
    );
    validate_report_pid_mode_contract(report, &resource_binding.options)?;
    validate_report_continuous_tuple_bindings(report, &resource_binding.options)?;
    let metrics = &report.metrics;
    validate_pid_screen_contract(
        "full-data PID screen",
        [
            &metrics.mi_v_action,
            &metrics.mi_l_action,
            &metrics.mi_d_action,
        ],
        [
            ("mi_vl_action", metrics.mi_vl_action),
            (
                "co_information_v_l_action",
                metrics.co_information_v_l_action,
            ),
            ("redundancy_v_l_action", metrics.redundancy_v_l_action),
            ("unique_v_action", metrics.unique_v_action),
            ("unique_l_action", metrics.unique_l_action),
            ("synergy_v_l_action", metrics.synergy_v_l_action),
        ],
        &metrics.pid_pairs,
        &metrics.estimate_denominators,
    )?;
    if let Some(control) = &metrics.pls_shuffled_target_control {
        validate_pid_screen_metrics(control, "full-data shuffled-target control")?;
    }
    if let Some(train) = &report.train_split_pid {
        if let Some(metrics) = &train.metrics {
            validate_pid_screen_metrics(metrics, "train-split PID screen")?;
        }
    }
    validate_heldout_prediction_contract(report)?;
    Ok(())
}

fn validate_temporal_report(
    temporal: &OfflineVldaTemporalReport,
    dims: &OfflineVldaDims,
) -> Result<()> {
    ensure!(
        temporal.interpretation
            == "descriptive_within_unit_step_run_pearson_lag1_not_estimator_effective_sample_size_or_block_selector",
        "offline VLDA temporal report carries an unknown interpretation"
    );
    ensure!(
        matches!(
            temporal.scope.as_str(),
            "within_episode"
                | "unidentified_without_episode_ids"
                | "known_episode_segments_only_mixed_ids"
        ),
        "offline VLDA temporal report carries an unknown scope"
    );
    ensure!(
        matches!(
            temporal.ordering_basis.as_str(),
            "strict_canonical_metadata_sequence_index_unit_steps_within_segments"
                | "missing_or_nonmonotone_metadata_sequence_index_no_lag_pairs"
                | "episode_identity_absent_no_lag_pairs"
                | "no_within_segment_pair"
        ),
        "offline VLDA temporal report carries an unknown ordering basis"
    );
    ensure!(
        temporal.segments <= dims.samples,
        "offline VLDA temporal report carries an invalid segment count"
    );
    ensure!(
        temporal.potential_lag_pairs
            == if temporal.scope == "unidentified_without_episode_ids" {
                0
            } else {
                dims.samples - temporal.segments
            },
        "offline VLDA temporal potential lag-pair count contradicts its segment count"
    );
    ensure!(
        temporal.scope != "within_episode" || temporal.segments > 0,
        "offline VLDA within-episode scope must contain at least one segment"
    );
    ensure!(
        temporal.scope != "unidentified_without_episode_ids" || temporal.segments == 0,
        "offline VLDA unidentified temporal scope must contain no segment"
    );
    ensure!(
        temporal.scope != "known_episode_segments_only_mixed_ids" || temporal.segments >= 2,
        "offline VLDA mixed-id temporal scope must contain at least two segments"
    );
    let expected_lag_pairs = match temporal.ordering_basis.as_str() {
        "strict_canonical_metadata_sequence_index_unit_steps_within_segments" => {
            ensure!(
                temporal.potential_lag_pairs > 0,
                "offline VLDA strict temporal order receipt has no potential lag pair"
            );
            temporal
                .potential_lag_pairs
                .checked_sub(temporal.sequence_index_gap_pairs)
                .ok_or_else(|| anyhow!("offline VLDA temporal sequence-gap count is invalid"))?
        }
        "missing_or_nonmonotone_metadata_sequence_index_no_lag_pairs" => {
            ensure!(
                temporal.potential_lag_pairs > 0,
                "offline VLDA missing temporal order receipt has no potential lag pair"
            );
            0
        }
        "episode_identity_absent_no_lag_pairs" => {
            ensure!(
                temporal.scope == "unidentified_without_episode_ids"
                    && temporal.potential_lag_pairs == 0,
                "offline VLDA absent-episode order basis contradicts its topology"
            );
            0
        }
        "no_within_segment_pair" => {
            ensure!(
                temporal.scope != "unidentified_without_episode_ids"
                    && temporal.potential_lag_pairs == 0,
                "offline VLDA zero-pair order basis contradicts its topology"
            );
            0
        }
        _ => unreachable!("ordering basis allowlist was checked above"),
    };
    ensure!(
        temporal.lag_pairs == expected_lag_pairs,
        "offline VLDA temporal admitted lag-pair count contradicts its order receipt"
    );
    ensure!(
        temporal.sequence_index_gap_pairs <= temporal.potential_lag_pairs,
        "offline VLDA temporal sequence-gap count exceeds its potential lag-pair count"
    );
    ensure!(
        temporal.correlation_lag_pairs <= temporal.lag_pairs,
        "offline VLDA temporal correlation-pair count exceeds its admitted lag-pair count"
    );
    ensure!(
        temporal.correlation_lag_pairs == 0 || temporal.correlation_lag_pairs >= 3,
        "offline VLDA temporal correlation-pair count cannot come from the minimum three-pair runs"
    );
    ensure!(
        temporal.ordering_basis
            == "strict_canonical_metadata_sequence_index_unit_steps_within_segments"
            || temporal.sequence_index_gap_pairs == 0,
        "offline VLDA temporal sequence gaps require a strict order receipt"
    );
    let expected_dimensions =
        BTreeMap::from([("V", dims.v), ("L", dims.l), ("D", dims.d), ("A", dims.a)]);
    ensure!(
        temporal.variables.len() == expected_dimensions.len()
            && temporal
                .variables
                .keys()
                .map(String::as_str)
                .eq(expected_dimensions.keys().copied()),
        "offline VLDA temporal report must contain exactly V, L, D, and A"
    );
    for (axis, expected_dimension) in expected_dimensions {
        let variable = &temporal.variables[axis];
        ensure!(
            variable.dimensions_total == expected_dimension
                && variable.dimensions_with_defined_lag1 <= variable.dimensions_total,
            "offline VLDA temporal {axis} dimension coverage contradicts the report dimensions"
        );
        ensure!(
            temporal.correlation_lag_pairs > 0 || variable.dimensions_with_defined_lag1 == 0,
            "offline VLDA temporal {axis} cannot define lag-1 columns without centered correlation pairs"
        );
        ensure!(
            variable.lag1_autocorr.is_some()
                == (temporal.correlation_lag_pairs > 0
                    && variable.dimensions_with_defined_lag1 > 0),
            "offline VLDA temporal {axis} lag-1 presence contradicts its defined-dimension coverage"
        );
        if let Some(lag1) = variable.lag1_autocorr {
            ensure!(
                lag1.is_finite() && (-1.0..=1.0).contains(&lag1),
                "offline VLDA temporal {axis} lag-1 value is invalid"
            );
        }
    }
    Ok(())
}

fn validate_atom_stability_envelope(
    atom: &OfflineVldaAtomStabilityEnvelope,
    n_boot: usize,
    context: &str,
) -> Result<()> {
    ensure!(
        [
            atom.point,
            atom.m_sample_percentile_lower,
            atom.m_sample_percentile_upper,
        ]
        .into_iter()
        .all(f64::is_finite),
        "{context}: point or raw m-sample percentile endpoint is non-finite"
    );
    ensure!(
        atom.m_sample_percentile_lower <= atom.m_sample_percentile_upper,
        "{context}: raw m-sample percentile endpoints are reversed"
    );
    ensure!(
        atom.n_valid == n_boot,
        "{context}: stability envelope exists without every requested resample: valid={}, requested={n_boot}",
        atom.n_valid
    );
    let boot_mean = atom
        .boot_mean
        .context(format!("{context}: m-sample resample mean is absent"))?;
    let bias_vs_point = atom
        .bias_vs_point
        .context(format!("{context}: m-sample bias diagnostic is absent"))?;
    ensure!(
        boot_mean.is_finite() && bias_vs_point.is_finite(),
        "{context}: m-sample resample mean or bias diagnostic is non-finite"
    );
    ensure!(
        bias_vs_point.to_bits() == (boot_mean - atom.point).to_bits(),
        "{context}: m-sample bias diagnostic does not equal boot_mean - point"
    );
    Ok(())
}

fn validate_uncertainty_points_match_report(
    uncertainty: &OfflineVldaPidUncertainty,
    report: &OfflineVldaReport,
) -> Result<()> {
    for pair in &uncertainty.pairs {
        let report_pair = report.metrics.pid_pairs.get(&pair.pair).with_context(|| {
            format!(
                "offline VLDA PID uncertainty pair {} is absent from the main report",
                pair.pair
            )
        })?;
        for (atom_name, envelope, report_point) in [
            (
                "redundancy",
                pair.redundancy.as_ref(),
                report_pair.redundancy,
            ),
            (
                "unique_s1",
                pair.unique_s1.as_ref(),
                report_pair.unique_source_1,
            ),
            (
                "unique_s2",
                pair.unique_s2.as_ref(),
                report_pair.unique_source_2,
            ),
            ("synergy", pair.synergy.as_ref(), report_pair.synergy),
        ] {
            let Some(envelope) = envelope else {
                continue;
            };
            let report_point = report_point.with_context(|| {
                format!(
                    "offline VLDA PID uncertainty {} {atom_name} has a point value while the main report abstained",
                    pair.pair
                )
            })?;
            ensure!(
                envelope.point.to_bits() == report_point.to_bits(),
                "offline VLDA PID uncertainty {} {atom_name} point does not match the main report",
                pair.pair
            );
        }
    }
    Ok(())
}

fn validate_offline_pid_uncertainty(uncertainty: &OfflineVldaPidUncertainty) -> Result<()> {
    ensure!(
        uncertainty.schema_version == OFFLINE_UNCERTAINTY_SCHEMA_VERSION,
        "PID uncertainty schema must be {OFFLINE_UNCERTAINTY_SCHEMA_VERSION}"
    );
    ensure!(
        uncertainty.dataset_content_sha256.len() == 64
            && uncertainty
                .dataset_content_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "PID uncertainty dataset content SHA-256 must be 64 lowercase hexadecimal characters"
    );
    ensure!(
        uncertainty.estimator_revision == ESTIMATOR_CONTINUOUS_PID2,
        "PID uncertainty estimator revision does not match the pinned review surface"
    );
    ensure!(
        uncertainty.stability_interpretation == RAW_M_SAMPLE_STABILITY_INTERPRETATION,
        "PID uncertainty stability interpretation must be {RAW_M_SAMPLE_STABILITY_INTERPRETATION}"
    );
    ensure!(
        uncertainty.preprocessing_resampling == OFFLINE_UNCERTAINTY_PREPROCESSING_RESAMPLING,
        "PID uncertainty preprocessing-resampling contract must be {OFFLINE_UNCERTAINTY_PREPROCESSING_RESAMPLING}"
    );
    ensure!(
        uncertainty.alpha.is_finite() && uncertainty.alpha > 0.0 && uncertainty.alpha < 1.0,
        "PID uncertainty raw-percentile tail mass must lie strictly inside (0, 1)"
    );
    ensure!(
        uncertainty.block_size > 0,
        "PID uncertainty block size must be positive"
    );
    let row_topology = OfflineVldaUncertaintyRowTopology::from_label(&uncertainty.row_topology)?;
    let permutation_scheme =
        parse_recorded_permutation_scheme(&uncertainty.permutation_scheme, uncertainty.n_perm)?;
    ensure!(
        uncertainty.permutation_calibration
            == permutation_calibration_label(permutation_scheme, uncertainty.n_perm)?,
        "PID uncertainty permutation calibration contradicts its scheme"
    );
    let config = OfflineVldaUncertaintyConfig {
        n_boot: uncertainty.n_boot,
        n_perm: uncertainty.n_perm,
        block_size: uncertainty.block_size,
        alpha: uncertainty.alpha,
        seed: uncertainty.seed,
        permutation_scheme,
    };
    validate_uncertainty_config(&config)?;
    if uncertainty.mode.starts_with("skipped:") {
        ensure!(
            uncertainty.pairs.is_empty(),
            "skipped PID uncertainty artifact carries pair results"
        );
        ensure!(
            uncertainty.subsample_len == 0,
            "skipped PID uncertainty artifact carries a subsample length"
        );
        if uncertainty.mode == "skipped:no_uncertainty_requested" {
            ensure!(
                uncertainty.n_boot == 0
                    && uncertainty.n_perm == 0
                    && uncertainty.resample_scheme == "not_requested"
                    && uncertainty.permutation_scheme == "not_requested"
                    && uncertainty.permutation_calibration == "not_requested",
                "no-request PID uncertainty skip carries a requested component or scheme"
            );
        } else if uncertainty.mode == UNSUPPORTED_UNCERTAINTY_TOPOLOGY_MODE {
            ensure!(
                uncertainty.pid_mode == PidMode::Continuous
                    && config.enabled()
                    && !row_topology.supports(&config),
                "PID uncertainty topology skip is inconsistent with its row topology or request"
            );
        } else {
            ensure!(
                uncertainty.pid_mode != PidMode::Continuous
                    && uncertainty.mode
                        == format!(
                            "skipped:non_continuous_mode_is_a_different_measure ({:?})",
                            uncertainty.pid_mode
                        ),
                "PID uncertainty artifact carries an unknown skip reason"
            );
            ensure!(
                uncertainty.n_boot > 0 || uncertainty.n_perm > 0,
                "non-continuous PID uncertainty skip records no requested component"
            );
        }
        ensure!(
            uncertainty.resample_scheme
                == if uncertainty.n_boot > 0 {
                    "politis_romano_subsample"
                } else {
                    "not_requested"
                },
            "PID uncertainty skip carries the wrong resampling scheme"
        );
        ensure!(
            uncertainty.permutation_scheme
                == if uncertainty.n_perm > 0 {
                    permutation_scheme_label(permutation_scheme)?
                } else {
                    "not_requested".to_string()
                },
            "PID uncertainty skip carries the wrong permutation scheme"
        );
        return Ok(());
    }
    ensure!(
        uncertainty.pid_mode == PidMode::Continuous && uncertainty.mode == "continuous",
        "PID uncertainty mode is neither continuous nor an explicit skip"
    );
    ensure!(
        uncertainty.n_boot > 0 || uncertainty.n_perm > 0,
        "continuous PID uncertainty artifact requested no inferential component"
    );
    ensure!(
        row_topology.supports(&config),
        "continuous PID uncertainty artifact uses a resampler that crosses unsupported episode boundaries"
    );
    if uncertainty.n_boot > 0 {
        ensure!(
            uncertainty.subsample_len > 0
                && uncertainty
                    .subsample_len
                    .is_multiple_of(uncertainty.block_size),
            "PID uncertainty subsample length is not a positive whole-block count"
        );
        ensure!(
            uncertainty.resample_scheme == "politis_romano_subsample",
            "PID uncertainty stability-resampling scheme is mislabeled"
        );
    } else {
        ensure!(
            uncertainty.subsample_len == 0 && uncertainty.resample_scheme == "not_requested",
            "unrequested PID stability resampling carries a scheme or subsample length"
        );
    }
    if uncertainty.n_perm > 0 {
        ensure!(
            !uncertainty.permutation_scheme.is_empty()
                && uncertainty.permutation_scheme != "not_requested"
                && !uncertainty.permutation_scheme.starts_with("unknown("),
            "PID uncertainty permutation scheme is missing or unknown"
        );
    } else {
        ensure!(
            uncertainty.permutation_scheme == "not_requested",
            "unrequested PID permutation carries a scheme"
        );
    }
    let expected_pairs = BTreeSet::from(["VL", "VD", "LD"]);
    let actual_pairs: BTreeSet<&str> = uncertainty
        .pairs
        .iter()
        .map(|pair| pair.pair.as_str())
        .collect();
    ensure!(
        uncertainty.pairs.len() == expected_pairs.len() && actual_pairs == expected_pairs,
        "continuous PID uncertainty artifact must contain exactly VL, VD, and LD once"
    );
    for pair in &uncertainty.pairs {
        let atoms = [
            ("redundancy", pair.redundancy.as_ref()),
            ("unique_s1", pair.unique_s1.as_ref()),
            ("unique_s2", pair.unique_s2.as_ref()),
            ("synergy", pair.synergy.as_ref()),
        ];
        let has_all_atoms = atoms.iter().all(|(_, atom)| atom.is_some());
        let has_any_atom = atoms.iter().any(|(_, atom)| atom.is_some());
        ensure!(
            has_all_atoms == has_any_atom,
            "PID uncertainty pair {} has a partial atom vector",
            pair.pair
        );
        ensure!(
            uncertainty.n_boot > 0 || !has_any_atom,
            "PID uncertainty pair {} carries an unrequested stability envelope",
            pair.pair
        );
        ensure!(
            uncertainty.n_perm > 0
                || (pair.unique_s1_tail_fraction.is_none()
                    && pair.unique_s2_tail_fraction.is_none()
                    && pair.perm_n_valid_s1 == 0
                    && pair.perm_n_valid_s2 == 0),
            "PID uncertainty pair {} carries unrequested permutation results",
            pair.pair
        );
        let has_any_numeric = has_any_atom
            || pair.unique_s1_tail_fraction.is_some()
            || pair.unique_s2_tail_fraction.is_some();
        let requested_complete = (uncertainty.n_boot == 0 || has_all_atoms)
            && (uncertainty.n_perm == 0
                || (pair.unique_s1_tail_fraction.is_some()
                    && pair.unique_s2_tail_fraction.is_some()));
        let synthetic_outcome = OfflineVldaOutcome {
            status: pair.status,
            measure: MEASURE_CONTINUOUS_PID2.to_string(),
            estimator_revision: ESTIMATOR_CONTINUOUS_PID2.to_string(),
            information_units: "nats".to_string(),
            axes: vec![
                "source_1".to_string(),
                "source_2".to_string(),
                "A".to_string(),
            ],
            scientific_gates: pair.scientific_gates.clone(),
            declared_continuous_tuple_support: pair.declared_continuous_tuple_support,
            reason_code: pair.reason_code,
            reason_detail: pair.reason_detail.clone(),
            axis_diagnostics: Vec::new(),
        };
        validate_outcome_contract(
            &synthetic_outcome,
            has_any_numeric,
            &format!("PID uncertainty pair {}", pair.pair),
        )?;
        match pair.status {
            OfflineVldaEstimateStatus::NotRequested => {
                bail!(
                    "PID uncertainty pair {} uses not_requested inside a requested artifact",
                    pair.pair
                );
            }
            OfflineVldaEstimateStatus::Produced => {
                ensure!(
                    requested_complete && pair.warning_codes.is_empty(),
                    "PID uncertainty pair {} is produced without every requested component",
                    pair.pair
                );
            }
            OfflineVldaEstimateStatus::ProducedWithWarning => {
                ensure!(
                    has_any_numeric && !requested_complete,
                    "PID uncertainty pair {} warning status is inconsistent with component presence",
                    pair.pair
                );
                let mut expected_warnings = Vec::new();
                if uncertainty.n_boot > 0 && !has_all_atoms {
                    expected_warnings
                        .push(OfflineVldaUncertaintyWarning::BootstrapStatisticsUnavailable);
                }
                if uncertainty.n_perm > 0 && pair.unique_s1_tail_fraction.is_none() {
                    expected_warnings
                        .push(OfflineVldaUncertaintyWarning::UniqueSource1PermutationUnavailable);
                }
                if uncertainty.n_perm > 0 && pair.unique_s2_tail_fraction.is_none() {
                    expected_warnings
                        .push(OfflineVldaUncertaintyWarning::UniqueSource2PermutationUnavailable);
                }
                ensure!(
                    pair.warning_codes == expected_warnings,
                    "PID uncertainty pair {} warning codes do not match missing components",
                    pair.pair
                );
            }
            OfflineVldaEstimateStatus::Abstained => {
                ensure!(
                    !has_any_numeric && pair.warning_codes.is_empty(),
                    "PID uncertainty pair {} abstention carries numeric values or warnings",
                    pair.pair
                );
            }
        }
        for (name, atom) in atoms {
            if let Some(atom) = atom {
                validate_atom_stability_envelope(
                    atom,
                    uncertainty.n_boot,
                    &format!("PID uncertainty {} {name}", pair.pair),
                )?;
            }
        }
        for (name, value, n_valid) in [
            (
                "unique_s1_tail_fraction",
                pair.unique_s1_tail_fraction,
                pair.perm_n_valid_s1,
            ),
            (
                "unique_s2_tail_fraction",
                pair.unique_s2_tail_fraction,
                pair.perm_n_valid_s2,
            ),
        ] {
            if uncertainty.n_perm == 0 {
                ensure!(
                    value.is_none() && n_valid == 0,
                    "PID uncertainty pair {} {name} carries an unrequested permutation result",
                    pair.pair
                );
                continue;
            }
            if let Some(value) = value {
                ensure!(
                    value.is_finite() && (0.0..=1.0).contains(&value),
                    "PID uncertainty pair {} {name} is outside [0, 1]",
                    pair.pair
                );
                ensure!(
                    n_valid == uncertainty.n_perm,
                    "PID uncertainty pair {} {name} exists without every requested permutation",
                    pair.pair
                );
            } else {
                ensure!(
                    n_valid < uncertainty.n_perm,
                    "PID uncertainty pair {} {name} is absent despite every requested permutation being valid",
                    pair.pair
                );
            }
        }
    }
    Ok(())
}

/// Write a [`OfflineVldaPidUncertainty`] to a JSON file.
pub fn write_offline_pid_uncertainty(
    path: impl AsRef<Path>,
    uncertainty: &OfflineVldaPidUncertainty,
) -> Result<()> {
    validate_offline_pid_uncertainty(uncertainty)?;
    ensure_parent(path.as_ref())?;
    pid_runlog::write_json_file_with_limits(
        path,
        uncertainty,
        RunLogLimits::default().with_max_file_bytes(OFFLINE_UNCERTAINTY_MAX_BYTES),
    )
}

pub fn write_offline_vlda_summary(
    path: impl AsRef<Path>,
    report: &OfflineVldaReport,
) -> Result<()> {
    validate_offline_vlda_report(report)?;
    validate_offline_vlda_report_analysis_seal(report)?;
    ensure_parent(path.as_ref())?;
    pid_runlog::write_json_file_with_limits(
        path,
        report,
        RunLogLimits::default().with_max_file_bytes(OFFLINE_SUMMARY_MAX_BYTES),
    )
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
    write_offline_vlda_runlog_with_options_and_uncertainty(
        path,
        OfflineVldaRunlogArtifacts {
            summary_path,
            input_path,
            ..OfflineVldaRunlogArtifacts::default()
        },
        dataset,
        report,
        options,
    )
}

/// Write the canonical run log and bind an optional PID-uncertainty companion.
///
/// A report produced by the aggregate invocation API with enabled uncertainty requires the
/// companion path and the caller-supplied validated result. The file must equal that result and
/// match the request recorded in the report. The CLI supplies the result it computed. The run log
/// records stable file digests and rechecks each named path before its terminal event.
pub fn write_offline_vlda_runlog_with_options_and_uncertainty(
    path: impl AsRef<Path>,
    artifacts: OfflineVldaRunlogArtifacts<'_>,
    dataset: &OfflineVldaDataset,
    report: &OfflineVldaReport,
    options: OfflineVldaRunlogOptions,
) -> Result<()> {
    let OfflineVldaRunlogArtifacts {
        summary_path,
        input_path,
        uncertainty_path,
        uncertainty,
    } = artifacts;
    // `OfflineVldaReport` is a public serde type. Recheck its structural invariants, process-local
    // analysis seal, and dataset binding before publication. The seal detects changes to fields
    // that structural reconstruction cannot derive from the dataset without rerunning analysis.
    validate_offline_vlda_report(report)?;
    validate_offline_vlda_report_analysis_seal(report)?;
    let expected_dims = validate_dataset(dataset)?;
    ensure!(
        report.dims == expected_dims,
        "offline VLDA report dimensions do not match the publication dataset"
    );
    ensure!(
        report.label_counts == label_counts(&dataset.samples),
        "offline VLDA report label counts do not match the publication dataset"
    );
    ensure!(
        report.axis_provenance == axis_provenance(&dataset.samples),
        "offline VLDA report axis provenance does not reconstruct from the publication dataset"
    );
    let resource_binding = report_resource_binding(report)?;
    let reconstructed_usage = admit_dataset_resources(
        dataset,
        Some(&resource_binding.options),
        resource_binding.uncertainty_config.as_ref(),
        &resource_binding.limits,
    )?;
    ensure!(
        reconstructed_usage == resource_binding.usage,
        "offline VLDA report resource usage does not reconstruct from the publication dataset"
    );
    let expected_run_id = dataset.run_id.as_deref().unwrap_or("offline-vlda-run");
    ensure!(
        report.run_id == expected_run_id,
        "offline VLDA report run_id does not match the publication dataset"
    );
    let dataset_content_sha256 = offline_vlda_dataset_content_sha256(dataset)
        .context("failed to hash the offline VLDA publication dataset")?;
    ensure!(
        report
            .config
            .get("dataset_content_sha256")
            .and_then(Value::as_str)
            == Some(dataset_content_sha256.as_str()),
        "offline VLDA report does not bind the publication dataset"
    );
    if let Some(config) = resource_binding.uncertainty_config.as_ref() {
        let request = report
            .config
            .get("uncertainty_request")
            .context("offline VLDA aggregate report is missing its uncertainty request")?;
        let expected_row_topology = uncertainty_row_topology(&dataset.samples);
        ensure!(
            request.get("row_topology").and_then(Value::as_str)
                == Some(expected_row_topology.label())
                && request.get("execution").and_then(Value::as_str)
                    == Some(uncertainty_execution_label(
                        resource_binding.options.pid_mode,
                        config,
                        expected_row_topology,
                    )),
            "offline VLDA uncertainty request does not match the publication dataset row topology"
        );
    }
    let summary_uri = summary_path
        .map(|path| exact_artifact_uri(path, "offline VLDA summary path"))
        .transpose()?;
    let uncertainty_uri = uncertainty_path
        .map(|path| exact_artifact_uri(path, "offline VLDA uncertainty path"))
        .transpose()?;
    let summary_snapshot = summary_path
        .map(|path| {
            read_bounded_regular_file(path, OFFLINE_SUMMARY_MAX_BYTES, "offline VLDA summary")
        })
        .transpose()?;
    if let Some(snapshot) = &summary_snapshot {
        let recorded = snapshot.exact_bytes(OFFLINE_SUMMARY_MAX_BYTES)?;
        let expected = serialize_pretty_json_bounded(report, OFFLINE_SUMMARY_MAX_BYTES)?;
        ensure!(
            recorded == expected,
            "offline VLDA summary selected for run-log publication is not the exact JSON serialization of the report"
        );
    }
    let uncertainty_enabled = report
        .config
        .pointer("/uncertainty_request/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    ensure!(
        uncertainty_enabled == uncertainty_path.is_some()
            && uncertainty_enabled == uncertainty.is_some(),
        if uncertainty_enabled {
            "offline VLDA report requests uncertainty, but its value or artifact was not supplied"
        } else {
            "offline VLDA uncertainty value or artifact was supplied for a report that did not request it"
        }
    );
    let uncertainty_snapshot = uncertainty_path
        .map(|path| {
            read_bounded_regular_file(
                path,
                OFFLINE_UNCERTAINTY_MAX_BYTES,
                "offline VLDA PID uncertainty artifact",
            )
        })
        .transpose()?;
    if let (Some(snapshot), Some(uncertainty)) = (&uncertainty_snapshot, uncertainty) {
        let recorded_bytes = snapshot.exact_bytes(OFFLINE_UNCERTAINTY_MAX_BYTES)?;
        let recorded: OfflineVldaPidUncertainty = serde_json::from_slice(recorded_bytes).context(
            "failed to decode the PID uncertainty artifact selected for run-log publication",
        )?;
        validate_offline_pid_uncertainty(uncertainty)?;
        validate_offline_pid_uncertainty(&recorded)?;
        let supplied_bytes =
            serialize_pretty_json_bounded(uncertainty, OFFLINE_UNCERTAINTY_MAX_BYTES)?;
        ensure!(
            recorded_bytes == supplied_bytes,
            "offline VLDA PID uncertainty artifact is not the exact JSON serialization of the supplied result"
        );
        let request = report
            .config
            .get("uncertainty_request")
            .context("offline VLDA report is missing its uncertainty request")?;
        let expected_subsample_len = if recorded.mode == "continuous" && recorded.n_boot > 0 {
            (((dataset.samples.len() / 2) / recorded.block_size).max(1)) * recorded.block_size
        } else {
            0
        };
        let expected_row_topology = uncertainty_row_topology(&dataset.samples);
        ensure!(
            recorded.dataset_content_sha256 == dataset_content_sha256
                && recorded.pid_mode == resource_binding.options.pid_mode
                && recorded.subsample_len == expected_subsample_len
                && recorded.row_topology == expected_row_topology.label()
                && recorded.permutation_calibration
                    == permutation_calibration_label(
                        parse_recorded_permutation_scheme(
                            &recorded.permutation_scheme,
                            recorded.n_perm,
                        )?,
                        recorded.n_perm,
                    )?
                && report
                    .config
                    .get("dataset_content_sha256")
                    .and_then(Value::as_str)
                    == Some(dataset_content_sha256.as_str())
                && request
                    .get("preprocessing_resampling")
                    .and_then(Value::as_str)
                    == Some(recorded.preprocessing_resampling.as_str())
                && request.get("n_boot").and_then(Value::as_u64)
                    == u64::try_from(recorded.n_boot).ok()
                && request.get("n_perm").and_then(Value::as_u64)
                    == u64::try_from(recorded.n_perm).ok()
                && request.get("block_size").and_then(Value::as_u64)
                    == u64::try_from(recorded.block_size).ok()
                && request.get("alpha").and_then(Value::as_f64) == Some(recorded.alpha)
                && request.get("seed").and_then(Value::as_u64) == Some(recorded.seed)
                && request.get("permutation_scheme").and_then(Value::as_str)
                    == Some(recorded.permutation_scheme.as_str())
                && request
                    .get("permutation_calibration")
                    .and_then(Value::as_str)
                    == Some(recorded.permutation_calibration.as_str())
                && request.get("row_topology").and_then(Value::as_str)
                    == Some(recorded.row_topology.as_str())
                && request.get("execution").and_then(Value::as_str)
                    == Some(uncertainty_execution_label(
                        recorded.pid_mode,
                        &OfflineVldaUncertaintyConfig {
                            n_boot: recorded.n_boot,
                            n_perm: recorded.n_perm,
                            block_size: recorded.block_size,
                            alpha: recorded.alpha,
                            seed: recorded.seed,
                            permutation_scheme: parse_recorded_permutation_scheme(
                                &recorded.permutation_scheme,
                                recorded.n_perm,
                            )?,
                        },
                        expected_row_topology,
                    )),
            "offline VLDA PID uncertainty artifact does not match the report request"
        );
        validate_uncertainty_points_match_report(&recorded, report)?;
    }
    let summary_sha256 = summary_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.sha256.clone());
    let uncertainty_sha256 = uncertainty_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.sha256.clone());
    let configured_input_uri = report
        .config
        .get("input_uri")
        .and_then(Value::as_str)
        .map(str::to_string);
    let configured_input_sha256 = report
        .config
        .get("input_sha256")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut input_snapshot = None;
    let input_uri = match input_path {
        Some(path) => {
            let path_uri = exact_artifact_uri(path, "offline VLDA input path")?;
            if configured_input_uri.as_deref() != Some(path_uri.as_str()) {
                bail!("offline VLDA run log input path does not match the analyzed snapshot URI");
            }
            if configured_input_sha256.is_none() {
                bail!("offline VLDA run log is missing the analyzed input snapshot SHA-256");
            }
            let mut snapshot = read_bounded_regular_file(
                path,
                resource_binding.limits.max_input_bytes,
                "offline VLDA publication input",
            )?;
            ensure!(
                snapshot.sha256.as_deref() == configured_input_sha256.as_deref(),
                "offline VLDA publication input no longer matches the analyzed snapshot"
            );
            let recorded_bytes = snapshot.exact_bytes(resource_binding.limits.max_input_bytes)?;
            pid_bridge::validate_strict_json_bytes(recorded_bytes)
                .context("offline VLDA publication input is not strict JSON")?;
            let recorded_dataset: OfflineVldaDataset = serde_json::from_slice(recorded_bytes)
                .context("failed to decode the offline VLDA publication input")?;
            let _ =
                admit_dataset_resources(&recorded_dataset, None, None, &resource_binding.limits)?;
            let recorded_dataset_sha256 = offline_vlda_dataset_content_sha256(&recorded_dataset)
                .context("failed to hash the offline VLDA publication input dataset")?;
            ensure!(
                recorded_dataset_sha256 == dataset_content_sha256,
                "offline VLDA publication input does not encode the publication dataset"
            );
            // Publication needs only the identity and digest after this comparison.
            // Release the duplicate byte buffer before emitting the run log.
            snapshot.bytes.take();
            input_snapshot = Some(snapshot);
            Some(path_uri)
        }
        None => configured_input_uri,
    };
    // Never reopen a mutable input path here. This is the digest of the exact
    // byte buffer parsed for the report, supplied by the snapshot reader.
    let input_sha256 = configured_input_sha256;
    ensure_parent(path.as_ref())?;
    let mut writer = RunLogWriter::create(path.as_ref())?;
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: report.run_id.clone(),
        timestamp_ns: 0,
        config_hash: report.config_hash.clone(),
        metadata: [
            ("source".to_string(), "pid-offline-harness".to_string()),
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
                "geometry_diagnostic_status".to_string(),
                report.geometry.diagnostics.status.clone(),
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
            observation_hash: Some(offline_vlda_sample_content_sha256(sample)?),
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
    // and count scales with the dataset (roughly two dozen events per labeled
    // held-out sample). Everything appended after them must continue from the returned
    // count — a fixed offset would be overtaken on realistic capture sizes and
    // the log would fail pid-runlog's nondecreasing-timestamp validation.
    let metric_events = write_metric_events(&mut writer, report, metric_timestamp_base)?;
    let mut next_timestamp_ns = metric_timestamp_base + metric_events;
    if input_path.is_some() {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns: next_timestamp_ns,
            name: "offline_vlda_input_json".to_string(),
            kind: "dataset_json".to_string(),
            uri: input_uri
                .clone()
                .context("offline VLDA publication input lacks its exact artifact URI")?,
            sha256: input_sha256,
            metadata: BTreeMap::new(),
        })?;
        next_timestamp_ns += 1;
    }
    if let Some(summary_uri) = summary_uri {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns: next_timestamp_ns,
            name: "offline_vlda_summary_json".to_string(),
            kind: "summary_json".to_string(),
            uri: summary_uri,
            sha256: summary_sha256,
            metadata: BTreeMap::new(),
        })?;
        next_timestamp_ns += 1;
    }
    if let Some(uncertainty_uri) = uncertainty_uri {
        writer.append(&RunLogEvent::ArtifactLogged {
            timestamp_ns: next_timestamp_ns,
            name: "offline_vlda_pid_uncertainty_json".to_string(),
            kind: "pid_uncertainty_json".to_string(),
            uri: uncertainty_uri,
            sha256: uncertainty_sha256,
            metadata: BTreeMap::from([
                (
                    "stability_interpretation".to_string(),
                    RAW_M_SAMPLE_STABILITY_INTERPRETATION.to_string(),
                ),
                (
                    "preprocessing_resampling".to_string(),
                    OFFLINE_UNCERTAINTY_PREPROCESSING_RESAMPLING.to_string(),
                ),
            ]),
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
    if let Some(snapshot) = &summary_snapshot {
        snapshot.verify_path()?;
    }
    if let Some(snapshot) = &uncertainty_snapshot {
        snapshot.verify_path()?;
    }
    if let Some(snapshot) = &input_snapshot {
        snapshot.verify_path()?;
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
        Some(train_pid) if train_pid.status == "disabled" => "disabled",
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

/// Return a failure when an upstream adapter explicitly marks the split as
/// scientifically ineligible. A wholly absent marker remains supported for generic
/// VLDA datasets; once present, the marker must be known, homogeneous, and stamped
/// on every sample. An explicit blocked verdict cannot be overridden by structural
/// train/test metadata or a downstream strict flag.
pub fn offline_vlda_split_scientific_eligibility_failure_message(
    dataset: &OfflineVldaDataset,
) -> Option<String> {
    let mut ready_samples = 0;
    let mut blocked_samples = 0;
    let mut invalid_samples = 0;
    for sample in &dataset.samples {
        match sample
            .metadata
            .get(OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_KEY)
            .map(String::as_str)
        {
            Some("structural_split_ready") => ready_samples += 1,
            Some(OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_BLOCKED) => blocked_samples += 1,
            Some(_) => invalid_samples += 1,
            None => {}
        }
    }
    if invalid_samples > 0 {
        return Some(format!(
            "offline VLDA split scientific eligibility invalid: {invalid_samples}/{} sample(s) carry an unknown status",
            dataset.samples.len()
        ));
    }
    if ready_samples > 0 && blocked_samples > 0 {
        return Some(format!(
            "offline VLDA split scientific eligibility inconsistent: ready={ready_samples} blocked={blocked_samples}"
        ));
    }
    let marked_samples = ready_samples + blocked_samples;
    if marked_samples > 0 && marked_samples < dataset.samples.len() {
        return Some(format!(
            "offline VLDA split scientific eligibility incomplete: marked={marked_samples} total={}",
            dataset.samples.len()
        ));
    }
    (blocked_samples > 0).then(|| {
        format!(
            "offline VLDA split scientific eligibility blocked: {blocked_samples}/{} sample(s) were marked unfrozen or contamination-unreviewed",
            dataset.samples.len()
        )
    })
}

/// Gate messages for `--require-axis-provenance-honest`. Returns a failure for every
/// V/L/D/A axis whose provenance is `degraded`. This includes missing, unrecognized,
/// fabricated, recency-misaligned, and proxy values. It also returns one failure when
/// no recognized provenance convention is present. The gate therefore cannot pass
/// vacuously or from sparse marker coverage.
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
    if options.require_heldout_split
        || options.require_heldout_class_coverage
        || options.require_heldout_episode_disjoint
    {
        if let Some(message) = offline_vlda_split_scientific_eligibility_failure_message(dataset) {
            failures.push(message);
        }
    }
    if options.require_axis_provenance_honest {
        failures.extend(offline_vlda_axis_provenance_failure_messages(
            &report.axis_provenance,
        ));
    }
    failures
}

fn validate_dataset_publication_eligibility(dataset: &OfflineVldaDataset) -> Result<()> {
    if let Some(verified_sha256) = &dataset.publication_receipt_verified_content_sha256 {
        let current_sha256 = offline_vlda_dataset_content_sha256(dataset)
            .context("failed to bind the verified NCP publication receipt to dataset content")?;
        if &current_sha256 != verified_sha256 {
            bail!(
                "dataset content changed after NCP publication receipt verification; reread and reverify the committed artifact"
            );
        }
    }
    if offline_vlda_has_ncp_markers(dataset) {
        if dataset.source.as_deref() != Some("ncp") {
            bail!("NCP-marked dataset must declare source=\"ncp\"");
        }
        if dataset
            .publication_receipt_verified_content_sha256
            .is_none()
        {
            bail!("NCP dataset lacks a verified committed publication receipt");
        }
        if !matches!(
            dataset.capture_integrity.as_deref(),
            Some("complete" | "complete_with_warning")
        ) {
            bail!("NCP dataset capture integrity is not analysis-eligible");
        }
    }
    Ok(())
}

fn validate_dataset_structure(dataset: &OfflineVldaDataset) -> Result<OfflineVldaDims> {
    for (name, value) in [
        ("run_id", dataset.run_id.as_deref()),
        ("source", dataset.source.as_deref()),
        ("model", dataset.model.as_deref()),
        ("task", dataset.task.as_deref()),
        ("capture_integrity", dataset.capture_integrity.as_deref()),
        (
            "publication_receipt",
            dataset.publication_receipt.as_deref(),
        ),
    ] {
        if value == Some("") {
            bail!("{name} must not be empty when present");
        }
    }
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
        if sample.episode_id.as_deref() == Some("") {
            bail!("episode_id must not be empty when present");
        }
        if !sample_ids.insert(sample.sample_id.as_str()) {
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

fn validate_dataset(dataset: &OfflineVldaDataset) -> Result<OfflineVldaDims> {
    validate_dataset_publication_eligibility(dataset)?;
    validate_dataset_structure(dataset)
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

/// Aggregate per-sample axis-provenance markers into one summary per marker.
///
/// A capture convention becomes active when any sample carries one of its markers.
/// Every marker in that convention must then be present with an accepted value on
/// every sample. This prevents a sparse or invented declaration from satisfying the
/// positive-attestation gate.
fn axis_provenance(samples: &[OfflineVldaSample]) -> Vec<OfflineVldaAxisProvenance> {
    #[derive(Clone, Copy)]
    struct MarkerSpec {
        marker: &'static str,
        axis: &'static str,
        accepted_exact: &'static [&'static str],
        accepts_token_slice: bool,
        known_degraded: &'static [&'static str],
    }

    const KNOWN_DEGRADED: &[&str] = &[
        "text_hash_proxy",
        "absent_zeroed",
        "recency_fallback",
        "zeroed",
        "absent",
    ];
    const NCP_MARKERS: &[MarkerSpec] = &[
        MarkerSpec {
            marker: "l_source",
            axis: "L",
            accepted_exact: &["channel"],
            accepts_token_slice: false,
            known_degraded: &["absent_zeroed"],
        },
        MarkerSpec {
            marker: "d_source",
            axis: "D",
            accepted_exact: &["source"],
            accepts_token_slice: false,
            known_degraded: &["recency_fallback", "absent"],
        },
    ];
    const SAFE_MARKERS: &[MarkerSpec] = &[
        MarkerSpec {
            marker: "v_provenance",
            axis: "V",
            accepted_exact: &["explicit_features", "hidden_state_pool"],
            accepts_token_slice: true,
            known_degraded: KNOWN_DEGRADED,
        },
        MarkerSpec {
            marker: "l_provenance",
            axis: "L",
            accepted_exact: &["explicit_features", "hidden_state_pool"],
            accepts_token_slice: true,
            known_degraded: KNOWN_DEGRADED,
        },
        MarkerSpec {
            marker: "d_provenance",
            axis: "D",
            accepted_exact: &["hidden_state_pool"],
            accepts_token_slice: true,
            known_degraded: KNOWN_DEGRADED,
        },
        MarkerSpec {
            marker: "a_provenance",
            axis: "A",
            accepted_exact: &["action_vector"],
            accepts_token_slice: false,
            known_degraded: KNOWN_DEGRADED,
        },
    ];

    fn accepted(spec: MarkerSpec, value: &str) -> bool {
        if spec.accepted_exact.contains(&value) {
            return true;
        }
        if !spec.accepts_token_slice {
            return false;
        }
        value.strip_prefix("token_slice:").is_some_and(|group| {
            !group.is_empty() && group.len() <= 128 && !group.chars().any(char::is_control)
        })
    }

    let convention_is_active = |markers: &[MarkerSpec]| {
        samples.iter().any(|sample| {
            markers
                .iter()
                .any(|spec| sample.metadata.contains_key(spec.marker))
        })
    };
    let mut active_specs = Vec::new();
    if convention_is_active(NCP_MARKERS) {
        active_specs.extend_from_slice(NCP_MARKERS);
    }
    if convention_is_active(SAFE_MARKERS) {
        active_specs.extend_from_slice(SAFE_MARKERS);
    }

    let mut out = Vec::new();
    for spec in active_specs {
        let mut sources: BTreeMap<String, usize> = BTreeMap::new();
        let mut present_samples = 0usize;
        let mut known_degraded_samples = 0usize;
        let mut unrecognized_samples = 0usize;
        for sample in samples {
            if let Some(value) = sample.metadata.get(spec.marker) {
                *sources.entry(value.clone()).or_insert(0) += 1;
                present_samples += 1;
                if spec.known_degraded.contains(&value.as_str()) {
                    known_degraded_samples += 1;
                } else if !accepted(spec, value) {
                    unrecognized_samples += 1;
                }
            }
        }
        let missing_samples = samples.len().saturating_sub(present_samples);
        let degraded_samples = known_degraded_samples + unrecognized_samples + missing_samples;
        let total_samples = samples.len();
        let (status, note) = if degraded_samples > 0 {
            (
                "degraded".to_string(),
                Some(format!(
                    "{degraded_samples}/{total_samples} samples lack accepted {axis} provenance \
                     for {} (known degraded: {known_degraded_samples}; unrecognized: \
                     {unrecognized_samples}; missing: {missing_samples}); PID atoms involving \
                     {axis} are NOT trustworthy for those samples",
                    spec.marker,
                    axis = spec.axis,
                )),
            )
        } else {
            ("ok".to_string(), None)
        };
        out.push(OfflineVldaAxisProvenance {
            marker: spec.marker.to_string(),
            axis: spec.axis.to_string(),
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
    temporal: OfflineVldaTemporalReport,
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
    preprocessing: OfflineVldaPreprocessingReport,
}

fn compute_analysis(
    samples: &[OfflineVldaSample],
    support: &BTreeMap<String, OfflineVldaDeclaredSupport>,
    continuous_tuple_support: &BTreeMap<String, OfflineVldaContinuousTupleSupport>,
    dims: &OfflineVldaDims,
    options: &OfflineVldaHarnessOptions,
    dense_solver_budget: ResourceBudget,
    categorical_pid_budget: ResourceBudget,
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
    let pid_contract = OfflineVldaPidScreenContract {
        support,
        continuous_tuple_support,
        options,
        budgets: OfflineVldaPidResourceBudgets {
            dense_solver: dense_solver_budget,
            categorical_pid: categorical_pid_budget,
        },
    };
    let (metrics, heldout_predictions) = compute_metrics(
        samples,
        &prepared,
        heldout_split.as_ref(),
        success_labels.as_deref(),
        pid_contract,
    )?;
    let heldout_failure_diagnostics = heldout_failure_diagnostics(&heldout_predictions);
    let geometry = compute_geometry_report(&prepared)?;
    let temporal = compute_temporal_report(samples, &prepared);
    let PreparedVldaMatrices {
        v,
        l,
        d,
        a,
        preprocessing,
    } = prepared;
    drop((v, l, d, a));
    // A train-only screen must fit its own preprocessing. Release the larger
    // all-sample matrices before allocating that independent analysis.
    let train_split_pid = heldout_split
        .as_ref()
        .map(|split| train_split_pid_report(samples, dims, split, pid_contract));
    Ok(OfflineVldaAnalysis {
        metrics,
        preprocessing,
        geometry,
        temporal,
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
    prepare_standardized_embeddings_selected(samples, None, dims)
}

fn prepare_standardized_embeddings_for_train(
    samples: &[OfflineVldaSample],
    roles: &[OfflineVldaSplitRole],
    dims: &OfflineVldaDims,
) -> Result<PreparedVldaMatrices> {
    prepare_standardized_embeddings_selected(samples, Some(roles), dims)
}

fn prepare_standardized_embeddings_selected(
    samples: &[OfflineVldaSample],
    train_roles: Option<&[OfflineVldaSplitRole]>,
    dims: &OfflineVldaDims,
) -> Result<PreparedVldaMatrices> {
    let n = dims.samples;
    let mut variables = BTreeMap::new();
    // Flatten and standardize one axis at a time. This keeps only one raw
    // matrix alive while the four retained standardized matrices accumulate.
    let v = standardize_embedding(
        "V",
        flatten_selected(samples, train_roles, n, dims.v, |sample| &sample.v)?,
        n,
        dims.v,
        &mut variables,
    )?;
    let l = standardize_embedding(
        "L",
        flatten_selected(samples, train_roles, n, dims.l, |sample| &sample.l)?,
        n,
        dims.l,
        &mut variables,
    )?;
    let d = standardize_embedding(
        "D",
        flatten_selected(samples, train_roles, n, dims.d, |sample| &sample.d)?,
        n,
        dims.d,
        &mut variables,
    )?;
    let a = standardize_embedding(
        "A",
        flatten_selected(samples, train_roles, n, dims.a, |sample| &sample.a)?,
        n,
        dims.a,
        &mut variables,
    )?;
    Ok(PreparedVldaMatrices {
        v,
        l,
        d,
        a,
        preprocessing: OfflineVldaPreprocessingReport {
            strategy: "per_variable_standardized".to_string(),
            variables,
        },
    })
}

fn concatenate_rows(matrices: &[MatRef<'_>]) -> Result<MatOwned> {
    let first = matrices
        .first()
        .context("offline VLDA concatenation requires at least one matrix")?;
    let rows = first.nrows();
    ensure!(
        matrices.iter().all(|matrix| matrix.nrows() == rows),
        "offline VLDA concatenation requires equal row counts"
    );
    let columns = matrices.iter().try_fold(0usize, |total, matrix| {
        total
            .checked_add(matrix.ncols())
            .context("offline VLDA concatenated column count overflowed usize")
    })?;
    let scalars = rows
        .checked_mul(columns)
        .context("offline VLDA concatenated scalar count overflowed usize")?;
    let mut data = Vec::new();
    data.try_reserve_exact(scalars)
        .context("failed to reserve offline VLDA concatenated matrix")?;
    for row in 0..rows {
        for matrix in matrices {
            data.extend_from_slice(matrix.row(row));
        }
    }
    MatOwned::new(data, rows, columns)
        .map_err(|error| anyhow::anyhow!("offline VLDA matrix concatenation failed: {error}"))
}

fn standardize_embedding(
    name: &str,
    data: Vec<f64>,
    n: usize,
    dim: usize,
    variables: &mut BTreeMap<String, OfflineVldaPreprocessingVariable>,
) -> Result<MatOwned> {
    let raw = MatRef::new(&data, n, dim)?;
    // `LeaveCentered` is documented upstream as the pre-1.0 behavior: a constant column stays in
    // the output, mean-centered but unscaled. Any other policy would change the standardization
    // provenance hashed below.
    let (standardized, standardizer) =
        Standardizer::fit_transform(raw, ConstantColumnPolicy::LeaveCentered)?;
    variables.insert(
        name.to_string(),
        OfflineVldaPreprocessingVariable {
            input_dim: dim,
            output_dim: dim,
            zero_variance_dims: zero_variance_dims(&data, n, dim),
            mean_sha256: pid_runlog::canonical_json_hash_v2(&standardizer.mean().to_vec())?,
            inv_std_sha256: pid_runlog::canonical_json_hash_v2(&standardizer.inv_std()?)?,
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
    success_labels: Option<&[bool]>,
    contract: OfflineVldaPidScreenContract<'_>,
) -> Result<(OfflineVldaMetrics, Vec<OfflineVldaHeldoutPredictionRecord>)> {
    let pid_screen = compute_pid_screen_metrics_with_control(prepared, contract)?;
    let (success_rate, majority_success_accuracy) = success_metrics(success_labels);
    let episode_ids = episode_ids(samples);
    let episode_loo_majority_success_accuracy = success_labels
        .zip(episode_ids.as_deref())
        .map(|(labels, episode_ids)| episode_loo_majority_success_accuracy(labels, episode_ids));
    let roles = heldout_split.map(|split| split.roles.as_slice());
    let mut heldout_predictions = Vec::new();
    if let (Some(labels), Some(roles)) = (success_labels, roles) {
        append_heldout_majority_prediction_records(
            &mut heldout_predictions,
            samples,
            labels,
            roles,
        );
    }
    let mut nn_v = success_labels
        .map(|labels| {
            compute_nn_baselines(
                samples,
                labels,
                episode_ids.as_deref(),
                roles,
                "V",
                |left, right| squared_euclidean(&left.v, &right.v),
            )
        })
        .transpose()?;
    let mut nn_l = success_labels
        .map(|labels| {
            compute_nn_baselines(
                samples,
                labels,
                episode_ids.as_deref(),
                roles,
                "L",
                |left, right| squared_euclidean(&left.l, &right.l),
            )
        })
        .transpose()?;
    let mut nn_d = success_labels
        .map(|labels| {
            compute_nn_baselines(
                samples,
                labels,
                episode_ids.as_deref(),
                roles,
                "D",
                |left, right| squared_euclidean(&left.d, &right.d),
            )
        })
        .transpose()?;
    let mut nn_a = success_labels
        .map(|labels| {
            compute_nn_baselines(
                samples,
                labels,
                episode_ids.as_deref(),
                roles,
                "A",
                |left, right| squared_euclidean(&left.a, &right.a),
            )
        })
        .transpose()?;
    let mut nn_vlda = success_labels
        .map(|labels| {
            compute_nn_baselines(
                samples,
                labels,
                episode_ids.as_deref(),
                roles,
                "VLDA",
                squared_euclidean_vlda,
            )
        })
        .transpose()?;
    for baseline in [&mut nn_v, &mut nn_l, &mut nn_d, &mut nn_a, &mut nn_vlda]
        .into_iter()
        .flatten()
    {
        heldout_predictions.append(&mut baseline.heldout_predictions);
    }
    let vlda_centroid_model = match (success_labels, roles) {
        (Some(labels), Some(roles)) => append_all_heldout_centroid_prediction_records(
            &mut heldout_predictions,
            samples,
            labels,
            roles,
        )?,
        _ => None,
    };
    let loo_nn_v_success_accuracy = nn_v.as_ref().map(|baseline| baseline.loo_accuracy);
    let loo_nn_l_success_accuracy = nn_l.as_ref().map(|baseline| baseline.loo_accuracy);
    let loo_nn_d_success_accuracy = nn_d.as_ref().map(|baseline| baseline.loo_accuracy);
    let loo_nn_a_success_accuracy = nn_a.as_ref().map(|baseline| baseline.loo_accuracy);
    let loo_nn_vlda_success_accuracy = nn_vlda.as_ref().map(|baseline| baseline.loo_accuracy);
    let episode_loo_nn_v_success_accuracy =
        nn_v.as_ref().and_then(|baseline| baseline.episode_accuracy);
    let episode_loo_nn_l_success_accuracy =
        nn_l.as_ref().and_then(|baseline| baseline.episode_accuracy);
    let episode_loo_nn_d_success_accuracy =
        nn_d.as_ref().and_then(|baseline| baseline.episode_accuracy);
    let episode_loo_nn_a_success_accuracy =
        nn_a.as_ref().and_then(|baseline| baseline.episode_accuracy);
    let episode_loo_nn_vlda_success_accuracy = nn_vlda
        .as_ref()
        .and_then(|baseline| baseline.episode_accuracy);
    let heldout_majority_success_metrics =
        heldout_metrics_from_records(&heldout_predictions, "train_split_majority", None);
    let heldout_majority_success_accuracy =
        heldout_majority_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_majority_success_balanced_accuracy =
        heldout_majority_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_nn_v_success_metrics =
        heldout_metrics_from_records(&heldout_predictions, "train_split_1nn", Some("V"));
    let heldout_nn_l_success_metrics =
        heldout_metrics_from_records(&heldout_predictions, "train_split_1nn", Some("L"));
    let heldout_nn_d_success_metrics =
        heldout_metrics_from_records(&heldout_predictions, "train_split_1nn", Some("D"));
    let heldout_nn_a_success_metrics =
        heldout_metrics_from_records(&heldout_predictions, "train_split_1nn", Some("A"));
    let heldout_nn_vlda_success_metrics =
        heldout_metrics_from_records(&heldout_predictions, "train_split_1nn", Some("VLDA"));
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
    let heldout_centroid_v_success_metrics = heldout_metrics_from_records(
        &heldout_predictions,
        "train_split_nearest_centroid",
        Some("V"),
    );
    let heldout_centroid_l_success_metrics = heldout_metrics_from_records(
        &heldout_predictions,
        "train_split_nearest_centroid",
        Some("L"),
    );
    let heldout_centroid_d_success_metrics = heldout_metrics_from_records(
        &heldout_predictions,
        "train_split_nearest_centroid",
        Some("D"),
    );
    let heldout_centroid_a_success_metrics = heldout_metrics_from_records(
        &heldout_predictions,
        "train_split_nearest_centroid",
        Some("A"),
    );
    let heldout_centroid_vlda_success_metrics = heldout_metrics_from_records(
        &heldout_predictions,
        "train_split_nearest_centroid",
        Some("VLDA"),
    );
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
    if let (Some(labels), Some(roles), Some(model)) =
        (success_labels, roles, vlda_centroid_model.as_ref())
    {
        append_heldout_logreg_prediction_records(
            &mut heldout_predictions,
            samples,
            labels,
            roles,
            model,
            contract.budgets.dense_solver,
        )?;
    }
    let heldout_logreg_vlda_success_metrics =
        heldout_metrics_from_records(&heldout_predictions, "train_split_logreg", Some("VLDA"));
    let heldout_logreg_vlda_success_accuracy =
        heldout_logreg_vlda_success_metrics.map(|metrics| metrics.accuracy);
    let heldout_logreg_vlda_success_balanced_accuracy =
        heldout_logreg_vlda_success_metrics.and_then(|metrics| metrics.balanced_accuracy);
    let heldout_logreg_vlda_success_auroc =
        heldout_logreg_vlda_success_metrics.and_then(|metrics| metrics.auroc);
    let OfflineVldaPidScreenMetrics {
        mi_v_action,
        mi_l_action,
        mi_d_action,
        mi_vl_action,
        co_information_v_l_action,
        redundancy_v_l_action,
        unique_v_action,
        unique_l_action,
        synergy_v_l_action,
        estimate_denominators,
        pid_pairs,
        categorical_quantization,
        pls_selection,
        pls_shuffled_target_control,
        pls_control_seed,
    } = pid_screen;
    let metrics = OfflineVldaMetrics {
        mi_v_action,
        mi_l_action,
        mi_d_action,
        mi_vl_action,
        co_information_v_l_action,
        redundancy_v_l_action,
        unique_v_action,
        unique_l_action,
        synergy_v_l_action,
        estimate_denominators,
        categorical_quantization,
        pls_selection,
        pls_shuffled_target_control,
        pls_control_seed,
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
        pid_pairs,
    };
    Ok((metrics, heldout_predictions))
}

#[derive(Debug, Clone, Copy)]
struct OfflineVldaSourceMatrix<'a> {
    name: &'static str,
    matrix: MatRef<'a>,
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
    diagnostics: Vec<OfflineVldaAxisDiagnostics>,
    tuple_support: Option<OfflineVldaContinuousTupleSupport>,
    pid_cfg: &Pid2Config,
) -> Result<OfflineVldaPidPairMetrics> {
    let axes = [source_1.name, source_2.name, target.name];
    let empty = |outcome: OfflineVldaOutcome| OfflineVldaPidPairMetrics {
        source_1: source_1.name.to_string(),
        source_2: source_2.name.to_string(),
        target: target.name.to_string(),
        outcome,
        mi_source_1_action: None,
        mi_source_2_action: None,
        mi_joint_action: None,
        co_information: None,
        redundancy: None,
        unique_source_1: None,
        unique_source_2: None,
        synergy: None,
        categorical_sx_components: None,
        categorical_saturation: None,
    };

    // The estimate is requested only when the COMPLETE source-target tuple is support-compatible
    // and its observed sample survives preflight.
    let (diagnostics, rejection) =
        continuous_preflight_from_diagnostics(diagnostics, tuple_support);
    if let Some((reason, detail)) = rejection {
        return Ok(empty(abstained_outcome(
            MEASURE_CONTINUOUS_PID2,
            &axes,
            diagnostics,
            reason,
            detail,
            tuple_support,
        )));
    }

    // One estimator pass. `pid2_isx_estimate` already computes the two marginal
    // MIs, the joint MI, and the I^sx redundancy; the atoms, the joint, and the
    // co-information are algebraic in those terms, so recomputing them with
    // standalone `ksg_mi_concat_xy` / `co_information_pairwise` calls (as this
    // fn previously did) was ~2× redundant O(n²) kNN work per pair for
    // bit-identical results (same estimator code paths, `Allow` forced).
    let est = match pid2_isx_estimate(source_1.matrix, source_2.matrix, target.matrix, pid_cfg) {
        Ok(est) => est,
        Err(err) => {
            let message = err.to_string();
            return match abstain_reason_for_error(&err) {
                Some(reason) => Ok(empty(abstained_outcome(
                    MEASURE_CONTINUOUS_PID2,
                    &axes,
                    diagnostics,
                    reason,
                    message,
                    tuple_support,
                ))),
                None => Err(anyhow::anyhow!(
                    "pid2_isx({}, {} -> {}) failed: {message}",
                    source_1.name,
                    source_2.name,
                    target.name
                )),
            };
        }
    };
    let pid = Pid2Result::from_estimate(est)?;
    Ok(OfflineVldaPidPairMetrics {
        source_1: source_1.name.to_string(),
        source_2: source_2.name.to_string(),
        target: target.name.to_string(),
        outcome: produced_outcome(MEASURE_CONTINUOUS_PID2, &axes, diagnostics, tuple_support),
        mi_source_1_action: Some(est.mi_s1_t),
        mi_source_2_action: Some(est.mi_s2_t),
        mi_joint_action: Some(est.mi_s1s2_t),
        co_information: Some(est.mi_s1_t + est.mi_s2_t - est.mi_s1s2_t),
        redundancy: Some(pid.redundancy),
        unique_source_1: Some(pid.unique_s1),
        unique_source_2: Some(pid.unique_s2),
        synergy: Some(pid.synergy),
        categorical_sx_components: None,
        categorical_saturation: None,
    })
}

const MEASURE_CONTINUOUS_MI: &str = "shannon_mutual_information_on_continuous_tuple";
const MEASURE_CONTINUOUS_PID2: &str =
    "ehrlich_schick_poland_makkeh_lanfermann_wollstadt_wibral_2024_continuous_shared_exclusions_pid2";
const MEASURE_CATEGORICAL_MI: &str =
    "shannon_mutual_information_on_fitted_equal_width_categorical_tuple";
const MEASURE_CATEGORICAL_PID2: &str =
    "makkeh_gutknecht_wibral_2021_averaged_categorical_shared_exclusions_pid2_on_fitted_equal_width_categories";

// Scientific-object firewall. The pin alone is not an estimator identity. Each value names the
// specific reviewed implementation route. Update these strings only with a reviewed pin or route.
const ESTIMATOR_CONTINUOUS_MI: &str =
    "pid-rs@796c11e/pid-core-0.9.0::experimental::continuous::raw_scalars::ksg_mi";
const ESTIMATOR_CONTINUOUS_PID2: &str =
    "pid-rs@796c11e/pid-core-0.9.0::experimental::continuous::pid2_isx_estimate/ehrlich_ksg";
const ESTIMATOR_CATEGORICAL_MI: &str =
    "pid-rs@796c11e/pid-core-0.9.0::stable::quantized::fitted_quantized_sxpid2_with_budget/marginal_mi";
const ESTIMATOR_CATEGORICAL_PID2: &str =
    "pid-rs@796c11e/pid-core-0.9.0::stable::quantized::fitted_quantized_sxpid2_with_budget/mgw_averaged_pid2";

fn estimator_revision_for_measure(measure: &str) -> &'static str {
    match measure {
        MEASURE_CONTINUOUS_MI => ESTIMATOR_CONTINUOUS_MI,
        MEASURE_CONTINUOUS_PID2 => ESTIMATOR_CONTINUOUS_PID2,
        MEASURE_CATEGORICAL_MI => ESTIMATOR_CATEGORICAL_MI,
        MEASURE_CATEGORICAL_PID2 => ESTIMATOR_CATEGORICAL_PID2,
        _ => "unreviewed_measure_has_no_estimator_identity",
    }
}

#[derive(Clone, Copy, Debug)]
struct RowBits<'a>(&'a [f64]);

fn canonical_row_bits(value: f64) -> u64 {
    if value == 0.0 {
        0
    } else {
        value.to_bits()
    }
}

impl PartialEq for RowBits<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .zip(other.0)
                .all(|(left, right)| canonical_row_bits(*left) == canonical_row_bits(*right))
    }
}

impl Eq for RowBits<'_> {}

impl Hash for RowBits<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for value in self.0 {
            canonical_row_bits(*value).hash(state);
        }
    }
}

/// Observed-sample evidence for one axis.
///
/// Evidence, not a population-support finding: exact ties reject the sample for a continuous
/// estimator but do not establish that the population law is discrete.
fn axis_diagnostics(
    axis: &str,
    matrix: MatRef<'_>,
    support: &BTreeMap<String, OfflineVldaDeclaredSupport>,
) -> OfflineVldaAxisDiagnostics {
    // The matrix outlives this map. Borrow each row instead of copying every
    // high-dimensional row into a second allocation solely to count ties.
    let mut counts: HashMap<RowBits<'_>, usize> = HashMap::new();
    for i in 0..matrix.nrows() {
        *counts.entry(RowBits(matrix.row(i))).or_insert(0) += 1;
    }
    OfflineVldaAxisDiagnostics {
        axis: axis.to_string(),
        rows: matrix.nrows(),
        unique_rows: counts.len(),
        max_row_multiplicity: counts.values().copied().max().unwrap_or(0),
        declared_support: support.get(&axis.to_ascii_lowercase()).copied(),
    }
}

/// Evaluate declared-support and observed-sample diagnostics for one continuous estimate.
///
/// A continuous estimate is requested only when **every** axis of the complete source–target tuple
/// declares an absolutely-continuous population law *and* the observed sample survives the
/// exact-tie check. The declared checks run first: they are statements about the estimand and hold
/// regardless of what this particular sample looks like.
fn continuous_preflight_from_diagnostics(
    diagnostics: Vec<OfflineVldaAxisDiagnostics>,
    tuple_support: Option<OfflineVldaContinuousTupleSupport>,
) -> (
    Vec<OfflineVldaAxisDiagnostics>,
    Option<(OfflineVldaAbstainReason, String)>,
) {
    let undeclared: Vec<&str> = diagnostics
        .iter()
        .filter(|d| d.declared_support.is_none())
        .map(|d| d.axis.as_str())
        .collect();
    if !undeclared.is_empty() {
        let detail = format!(
            "no declared population support for axis/axes: {}",
            undeclared.join(", ")
        );
        return (
            diagnostics,
            Some((OfflineVldaAbstainReason::SupportContractUnspecified, detail)),
        );
    }

    let incompatible: Vec<String> = diagnostics
        .iter()
        .filter(|d| d.declared_support.is_some_and(|s| !s.is_continuous()))
        .map(|d| format!("{} declared {:?}", d.axis, d.declared_support.unwrap()))
        .collect();
    if !incompatible.is_empty() {
        let detail = format!(
            "continuous shared-exclusions estimand is undefined for: {}",
            incompatible.join(", ")
        );
        return (
            diagnostics,
            Some((
                OfflineVldaAbstainReason::DeclaredSupportIncompatibleContinuous,
                detail,
            )),
        );
    }

    let Some(tuple_support) = tuple_support else {
        return (
            diagnostics,
            Some((
                OfflineVldaAbstainReason::TupleSupportContractUnspecified,
                "no complete-tuple joint-law and finite-information support contract was declared"
                    .to_string(),
            )),
        );
    };
    if !tuple_support.is_regular() {
        return (
            diagnostics,
            Some((
                OfflineVldaAbstainReason::DeclaredTupleSupportIncompatibleContinuous,
                format!(
                    "complete continuous estimator tuple declared {tuple_support:?}; the required joint law is not regular full-dimensional with finite information"
                ),
            )),
        );
    }

    let tied: Vec<String> = diagnostics
        .iter()
        .filter(|d| d.max_row_multiplicity > 1)
        .map(|d| {
            format!(
                "{} ({} unique rows of {}, max multiplicity {})",
                d.axis, d.unique_rows, d.rows, d.max_row_multiplicity
            )
        })
        .collect();
    if !tied.is_empty() {
        let detail = format!(
            "observed exact ties reject this sample for the continuous estimator; they do not \
             identify the population law: {}",
            tied.join("; ")
        );
        return (
            diagnostics,
            Some((
                OfflineVldaAbstainReason::ObservedSampleIncompatibleExactTies,
                detail,
            )),
        );
    }

    (diagnostics, None)
}

/// Classify a pid-core estimator failure as an abstention reason.
///
/// `None` means the error is not a known support / finite-sample rejection and must propagate — a
/// genuine bug is never silently converted into an abstention.
fn abstain_reason_for_error(error: &PidError) -> Option<OfflineVldaAbstainReason> {
    match error {
        PidError::SourceDimensionMismatch { .. } => {
            Some(OfflineVldaAbstainReason::EstimatorRequiresEqualSourceDimensions)
        }
        PidError::AmbiguousKthNeighborShell { .. } => {
            Some(OfflineVldaAbstainReason::AmbiguousNeighborShell)
        }
        PidError::ObservedContinuousSampleIncompatibility { .. } => {
            Some(OfflineVldaAbstainReason::ObservedSampleIncompatibleExactTies)
        }
        // `PidError` is non-exhaustive by design. New upstream errors remain hard failures until
        // this adapter makes an explicit, reviewed decision that they are support abstentions.
        _ => None,
    }
}

fn abstained_outcome(
    measure: &str,
    axes: &[&str],
    diagnostics: Vec<OfflineVldaAxisDiagnostics>,
    reason: OfflineVldaAbstainReason,
    detail: String,
    tuple_support: Option<OfflineVldaContinuousTupleSupport>,
) -> OfflineVldaOutcome {
    OfflineVldaOutcome {
        status: OfflineVldaEstimateStatus::Abstained,
        measure: measure.to_string(),
        estimator_revision: estimator_revision_for_measure(measure).to_string(),
        information_units: "nats".to_string(),
        axes: axes.iter().map(|a| (*a).to_string()).collect(),
        declared_continuous_tuple_support: tuple_support,
        scientific_gates: abstained_scientific_gates(reason),
        reason_code: Some(reason),
        reason_detail: Some(detail),
        axis_diagnostics: diagnostics,
    }
}

fn abstained_scientific_gates(reason: OfflineVldaAbstainReason) -> OfflineVldaScientificGates {
    let (population, measure_gate, estimator) = match reason {
        OfflineVldaAbstainReason::DeclaredSupportIncompatibleContinuous => (
            OfflineVldaScientificGateVerdict::Conditional,
            OfflineVldaScientificGateVerdict::Blocked,
            OfflineVldaScientificGateVerdict::NotEvaluated,
        ),
        OfflineVldaAbstainReason::SupportContractUnspecified => (
            OfflineVldaScientificGateVerdict::NotEvaluated,
            OfflineVldaScientificGateVerdict::NotEvaluated,
            OfflineVldaScientificGateVerdict::NotEvaluated,
        ),
        OfflineVldaAbstainReason::TupleSupportContractUnspecified => (
            OfflineVldaScientificGateVerdict::NotEvaluated,
            OfflineVldaScientificGateVerdict::NotEvaluated,
            OfflineVldaScientificGateVerdict::NotEvaluated,
        ),
        OfflineVldaAbstainReason::DeclaredTupleSupportIncompatibleContinuous => (
            OfflineVldaScientificGateVerdict::Conditional,
            OfflineVldaScientificGateVerdict::Blocked,
            OfflineVldaScientificGateVerdict::NotEvaluated,
        ),
        OfflineVldaAbstainReason::ObservedSampleIncompatibleExactTies
        | OfflineVldaAbstainReason::AmbiguousNeighborShell
        | OfflineVldaAbstainReason::EstimatorRequiresEqualSourceDimensions
        | OfflineVldaAbstainReason::UncertaintyStatisticsUnavailable => (
            OfflineVldaScientificGateVerdict::Conditional,
            OfflineVldaScientificGateVerdict::NotEvaluated,
            OfflineVldaScientificGateVerdict::Blocked,
        ),
    };
    OfflineVldaScientificGates {
        population,
        measure: measure_gate,
        estimator,
        application: OfflineVldaScientificGateVerdict::Blocked,
        interpretation_allowed: false,
        support_envelope_version: None,
        reason_code: Some(reason.as_str().to_string()),
    }
}

fn produced_scientific_gates(
    diagnostics: &[OfflineVldaAxisDiagnostics],
) -> OfflineVldaScientificGates {
    let population = if !diagnostics.is_empty()
        && diagnostics
            .iter()
            .all(|diagnostic| diagnostic.declared_support.is_some())
    {
        OfflineVldaScientificGateVerdict::Conditional
    } else {
        OfflineVldaScientificGateVerdict::NotEvaluated
    };
    OfflineVldaScientificGates {
        population,
        measure: OfflineVldaScientificGateVerdict::NotEvaluated,
        estimator: OfflineVldaScientificGateVerdict::NotEvaluated,
        application: OfflineVldaScientificGateVerdict::Blocked,
        interpretation_allowed: false,
        support_envelope_version: None,
        reason_code: Some("application_support_envelope_not_validated".to_string()),
    }
}

fn produced_outcome(
    measure: &str,
    axes: &[&str],
    diagnostics: Vec<OfflineVldaAxisDiagnostics>,
    tuple_support: Option<OfflineVldaContinuousTupleSupport>,
) -> OfflineVldaOutcome {
    let scientific_gates = produced_scientific_gates(&diagnostics);
    OfflineVldaOutcome {
        status: OfflineVldaEstimateStatus::Produced,
        measure: measure.to_string(),
        estimator_revision: estimator_revision_for_measure(measure).to_string(),
        information_units: "nats".to_string(),
        axes: axes.iter().map(|a| (*a).to_string()).collect(),
        declared_continuous_tuple_support: tuple_support,
        scientific_gates,
        reason_code: None,
        reason_detail: None,
        axis_diagnostics: diagnostics,
    }
}

/// Mark a value whose sources were projected toward the same target rows later analyzed.
///
/// The computation remains a valid evaluation of its fitted empirical categorical law. It is not
/// an unbiased held-out estimate of a pre-existing transform or a valid rescue for high-dimensional
/// continuous PID. The warning survives alongside the separate empirical-PMF saturation warning.
fn mark_supervised_same_row_warning(outcome: &mut OfflineVldaOutcome) {
    debug_assert!(outcome.produced());
    let saturated = outcome.scientific_gates.reason_code.as_deref()
        == Some(SCIENTIFIC_REASON_CATEGORICAL_SATURATION);
    outcome.status = OfflineVldaEstimateStatus::ProducedWithWarning;
    outcome.scientific_gates.estimator = OfflineVldaScientificGateVerdict::Blocked;
    outcome.scientific_gates.application = OfflineVldaScientificGateVerdict::Blocked;
    outcome.scientific_gates.interpretation_allowed = false;
    outcome.scientific_gates.reason_code = Some(
        if saturated {
            SCIENTIFIC_REASON_SUPERVISED_SAME_ROW_AND_SATURATION
        } else {
            SCIENTIFIC_REASON_SUPERVISED_SAME_ROW
        }
        .to_string(),
    );
    let design_warning = "PLS selected a target-supervised transform from the same rows used by \
                          this fitted-categorical screen; the value is a descriptive \
                          selection-inflation diagnostic, not held-out or application-valid";
    outcome.reason_detail = Some(match outcome.reason_detail.take() {
        Some(existing) => format!("{existing}; {design_warning}"),
        None => design_warning.to_string(),
    });
}

fn not_requested_outcome(axes: &[&str]) -> OfflineVldaOutcome {
    OfflineVldaOutcome {
        status: OfflineVldaEstimateStatus::NotRequested,
        measure: "not_requested_pid_disabled".to_string(),
        estimator_revision: "not_applicable_pid_disabled".to_string(),
        information_units: "not_applicable".to_string(),
        axes: axes.iter().map(|axis| (*axis).to_string()).collect(),
        declared_continuous_tuple_support: None,
        scientific_gates: OfflineVldaScientificGates {
            population: OfflineVldaScientificGateVerdict::NotApplicable,
            measure: OfflineVldaScientificGateVerdict::NotApplicable,
            estimator: OfflineVldaScientificGateVerdict::NotApplicable,
            application: OfflineVldaScientificGateVerdict::NotApplicable,
            interpretation_allowed: false,
            support_envelope_version: None,
            reason_code: Some("pid_disabled".to_string()),
        },
        reason_code: None,
        reason_detail: Some("PID/MI estimation disabled by configuration".to_string()),
        axis_diagnostics: Vec::new(),
    }
}

fn disabled_pid_screen_metrics() -> OfflineVldaPidScreenMetrics {
    let not_requested_mi = |source| OfflineVldaMiEstimate {
        outcome: not_requested_outcome(&[source, "A"]),
        value: None,
    };
    OfflineVldaPidScreenMetrics {
        mi_v_action: not_requested_mi("V"),
        mi_l_action: not_requested_mi("L"),
        mi_d_action: not_requested_mi("D"),
        mi_vl_action: None,
        co_information_v_l_action: None,
        redundancy_v_l_action: None,
        unique_v_action: None,
        unique_l_action: None,
        synergy_v_l_action: None,
        estimate_denominators: OfflineVldaEstimateDenominators::default(),
        pid_pairs: BTreeMap::new(),
        categorical_quantization: BTreeMap::new(),
        pls_selection: None,
        pls_shuffled_target_control: None,
        pls_control_seed: None,
    }
}

/// One requested continuous marginal MI, `I(source; target)`.
fn continuous_mi_estimate(
    source: OfflineVldaSourceMatrix<'_>,
    target: OfflineVldaTargetMatrix<'_>,
    diagnostics: Vec<OfflineVldaAxisDiagnostics>,
    tuple_support: Option<OfflineVldaContinuousTupleSupport>,
    precomputed_value: Option<f64>,
    ksg: &KsgConfig,
) -> Result<OfflineVldaMiEstimate> {
    let source_name = source.name;
    let target_name = target.name;
    let axes = [source_name, target_name];
    let (diagnostics, rejection) =
        continuous_preflight_from_diagnostics(diagnostics, tuple_support);
    if let Some((reason, detail)) = rejection {
        return Ok(OfflineVldaMiEstimate {
            outcome: abstained_outcome(
                MEASURE_CONTINUOUS_MI,
                &axes,
                diagnostics,
                reason,
                detail,
                tuple_support,
            ),
            value: None,
        });
    }
    if let Some(value) = precomputed_value {
        return Ok(OfflineVldaMiEstimate {
            outcome: produced_outcome(MEASURE_CONTINUOUS_MI, &axes, diagnostics, tuple_support),
            value: Some(value),
        });
    }
    match ksg_mi(source.matrix, target.matrix, ksg) {
        Ok(value) => Ok(OfflineVldaMiEstimate {
            outcome: produced_outcome(MEASURE_CONTINUOUS_MI, &axes, diagnostics, tuple_support),
            value: Some(value),
        }),
        Err(err) => {
            let message = err.to_string();
            match abstain_reason_for_error(&err) {
                Some(reason) => Ok(OfflineVldaMiEstimate {
                    outcome: abstained_outcome(
                        MEASURE_CONTINUOUS_MI,
                        &axes,
                        diagnostics,
                        reason,
                        message,
                        tuple_support,
                    ),
                    value: None,
                }),
                None => Err(anyhow::anyhow!(
                    "ksg_mi({source_name}, {target_name}) failed: {message}"
                )),
            }
        }
    }
}

/// The KSG configuration used by every continuous screen in this harness.
///
/// The current pid-core review contract fails closed on `SupportContract::Unspecified`: the caller
/// must state the population-law assumption. `assume_regular_full_dimensional` asserts that every
/// marginal and joint law in the call is full-dimensional and absolutely continuous. It is an
/// *assertion*, not a proof; eligibility for a given tuple is decided by `continuous_preflight`.
///
/// `NegativeHandling::Allow` is mandatory, not a preference: clamping an MI term before the
/// subtraction breaks the PID identity `Red + Unq1 + Unq2 + Syn = I(S1,S2;T)`.
fn ksg_config() -> KsgConfig {
    KsgConfig::assume_regular_full_dimensional().with_negative_handling(NegativeHandling::Allow)
}

/// The continuous 2-source PID configuration, carrying the same support assertion as [`ksg_config`].
fn pid2_config(ksg: &KsgConfig) -> Pid2Config {
    Pid2Config {
        ksg: ksg.clone(),
        isx: IsxConfig {
            k: ksg.k,
            metric: ksg.metric,
            tie_epsilon: ksg.tie_epsilon,
            ..IsxConfig::assume_regular_full_dimensional()
        },
    }
}

/// Fit an equal-width codebook on `x` and quantize `x` with it.
///
/// The current pid-core review surface omits the free `quantize_equal_width`; binning goes through
/// a fitted `EqualWidthQuantizer` whose edges are part of the estimand. Fitting on `x` itself
/// reproduces the legacy in-sample binning exactly. `grandplan.md` §7.6 requires the codebook to be
/// fit on training rows only in an inferential workflow; these screens are descriptive, and the
/// caller passes the train split where one exists.
fn quantize(x: MatRef<'_>, bins: usize, budget: ResourceBudget) -> Result<QuantizedData> {
    let config = QuantizerConfig::new(
        pid_core::stable::quantized::OutOfRangePolicy::Error,
        true,
        5,
        "per-variable standardization followed by fitted equal-width bins",
        budget,
    )?;
    let quantizer = EqualWidthQuantizer::fit(x, bins, config)
        .map_err(|e| anyhow::anyhow!("quantizer fit: {e}"))?;
    quantizer
        .transform_with_report(x)
        .map_err(|e| anyhow::anyhow!("quantizer transform: {e}"))
}

struct PreparedQuantizedAxis {
    data: QuantizedData,
    category_ids: Vec<u32>,
    unique_fraction: f64,
}

fn quantization_receipt(
    axis: &str,
    prepared: &PreparedQuantizedAxis,
) -> Result<OfflineVldaQuantizationReceipt> {
    let report = &prepared.data.report;
    let training_input_hash = report
        .training_input_hash
        .context("categorical Sx quantization omitted its required fitted-training input hash")?;
    let fitted_edge_count = report.bin_edges.iter().try_fold(0usize, |total, edges| {
        total
            .checked_add(edges.len())
            .context("categorical quantizer edge count overflow")
    })?;
    let out_of_range_policy = match report.out_of_range_policy {
        OutOfRangePolicy::Error => "error",
        OutOfRangePolicy::ClampToBoundary => "clamp_to_boundary",
        _ => "unknown_non_exhaustive",
    };
    ensure!(
        out_of_range_policy != "unknown_non_exhaustive",
        "unreviewed categorical quantizer out-of-range policy"
    );
    Ok(OfflineVldaQuantizationReceipt {
        axis: axis.to_string(),
        functional: "Makkeh-Gutknecht-Wibral averaged two-source categorical shared exclusions"
            .to_string(),
        quantizer: "pid_core::stable::quantized::EqualWidthQuantizer".to_string(),
        estimator_revision: ESTIMATOR_CATEGORICAL_PID2.to_string(),
        information_units: "nats".to_string(),
        fitted_edges_sha256: pid_runlog::canonical_json_hash_v2(&report.bin_edges)?,
        fitted_edge_count,
        training_input_sha256: crate::lowercase_hex(training_input_hash),
        transform_input_sha256: crate::lowercase_hex(report.transform_input_hash),
        categorical_output_sha256: crate::lowercase_hex(report.categorical_output_hash),
        out_of_range_policy: out_of_range_policy.to_string(),
        scaling_description: report.scaling_description.clone(),
        samples: report.n_samples,
        dimensions: report.dimensions,
        bins_per_dimension: report.bins_per_dimension,
        nominal_joint_cardinality: report
            .nominal_joint_cardinality
            .map(|value| value.to_string()),
        observed_joint_cardinality: report.observed_joint_cardinality,
        empty_joint_cells: report.empty_joint_cells.map(|value| value.to_string()),
        low_count_joint_cells: report.low_count_joint_cells,
        minimum_observed_cell_count: report.minimum_observed_cell_count,
        maximum_observed_cell_count: report.maximum_observed_cell_count,
        estimand_statement: report.estimand_statement.to_string(),
    })
}

fn prepare_quantized_axis(
    matrix: MatRef<'_>,
    bins: usize,
    budget: ResourceBudget,
) -> Result<PreparedQuantizedAxis> {
    let data = quantize(matrix, bins, budget)?;
    let category_ids = category_ids(&data)?;
    let unique_fraction = if category_ids.is_empty() {
        0.0
    } else {
        category_ids
            .iter()
            .copied()
            .max()
            .map_or(0, |maximum| maximum as usize + 1) as f64
            / category_ids.len() as f64
    };
    Ok(PreparedQuantizedAxis {
        data,
        category_ids,
        unique_fraction,
    })
}

/// Collapse each row's bin tuple into one category id, preserving row-tuple equality.
fn category_ids(quantized: &QuantizedData) -> Result<Vec<u32>> {
    let matrix = quantized.matrix.as_ref();
    // IDs follow first appearance in row order. Map iteration order is never observed, so a
    // borrowed-row hash lookup stays deterministic and avoids copying each high-dimensional row.
    let mut ids: HashMap<&[usize], u32> = HashMap::new();
    ids.try_reserve(matrix.nrows())
        .context("failed to reserve quantized category lookup")?;
    let mut out = Vec::with_capacity(matrix.nrows());
    for i in 0..matrix.nrows() {
        let next = u32::try_from(ids.len()).context("too many distinct bin tuples for u32")?;
        let id = *ids.entry(matrix.row(i)).or_insert(next);
        out.push(id);
    }
    Ok(out)
}

fn triple_unique_fraction(first: &[u32], second: &[u32], third: &[u32]) -> f64 {
    debug_assert_eq!(first.len(), second.len());
    debug_assert_eq!(first.len(), third.len());
    if first.is_empty() {
        return 0.0;
    }
    first
        .iter()
        .copied()
        .zip(second.iter().copied())
        .zip(third.iter().copied())
        .map(|((first, second), third)| (first, second, third))
        .collect::<std::collections::HashSet<_>>()
        .len() as f64
        / first.len() as f64
}

fn paired_unique_fraction(left: &[u32], right: &[u32]) -> f64 {
    debug_assert_eq!(left.len(), right.len());
    if left.is_empty() {
        return 0.0;
    }
    left.iter()
        .copied()
        .zip(right.iter().copied())
        .collect::<std::collections::HashSet<_>>()
        .len() as f64
        / left.len() as f64
}

fn categorical_mi_outcome(
    source_name: &'static str,
    diagnostics: Vec<OfflineVldaAxisDiagnostics>,
    source: &PreparedQuantizedAxis,
    target: &PreparedQuantizedAxis,
) -> OfflineVldaOutcome {
    let source_target_unique_fraction =
        paired_unique_fraction(&source.category_ids, &target.category_ids);
    let saturation_warning = [
        source.unique_fraction,
        target.unique_fraction,
        source_target_unique_fraction,
    ]
    .into_iter()
    .any(|fraction| fraction > OFFLINE_CATEGORICAL_SATURATION_UNIQUE_FRACTION_MAX);
    let mut outcome = produced_outcome(
        MEASURE_CATEGORICAL_MI,
        &[source_name, "A"],
        diagnostics,
        None,
    );
    if saturation_warning {
        outcome.status = OfflineVldaEstimateStatus::ProducedWithWarning;
        outcome.scientific_gates.estimator = OfflineVldaScientificGateVerdict::Blocked;
        outcome.scientific_gates.reason_code =
            Some(SCIENTIFIC_REASON_CATEGORICAL_SATURATION.to_string());
        outcome.reason_detail = Some(format!(
            "fitted-categorical empirical-PMF MI is support-sparse: source_unique_fraction={:.6}, \
             target_unique_fraction={:.6}, source_target_unique_fraction={:.6}; occupancy is too \
             high relative to sample count for stable application interpretation (grandplan section 7.6)",
            source.unique_fraction, target.unique_fraction, source_target_unique_fraction
        ));
    }
    outcome
}

/// Quantized categorical shared-exclusions PID pair metrics.
///
/// The fitted equal-width transforms define the variables. The categorical estimator returns the
/// informative, misinformative, and net parts of every atom. Saturation diagnostics flag regimes
/// where an empirical PMF is too sparse for application interpretation.
struct PreparedCategoricalSxPair<'a> {
    source_1: &'a PreparedQuantizedAxis,
    source_2: &'a PreparedQuantizedAxis,
    target: &'a PreparedQuantizedAxis,
}

fn compute_pid_pair_metrics_categorical_sx(
    source_1: OfflineVldaSourceMatrix<'_>,
    source_2: OfflineVldaSourceMatrix<'_>,
    target: OfflineVldaTargetMatrix<'_>,
    prepared: PreparedCategoricalSxPair<'_>,
    pair_diagnostics: Vec<OfflineVldaAxisDiagnostics>,
    estimator_budget: ResourceBudget,
) -> Result<OfflineVldaPidPairMetrics> {
    let axes = [source_1.name, source_2.name, target.name];
    // This is a separately requested quantized estimand. It is never an automatic fallback for a
    // failed continuous estimate and is never pooled with another preprocessing regime.
    let fitted = fitted_quantized_sxpid2_with_budget(
        &prepared.source_1.data,
        &prepared.source_2.data,
        &prepared.target.data,
        estimator_budget,
    )?;
    let pid = fitted.pid;
    let empirical_pmf = &pid.empirical_pmf;
    let mi_s1s2_t = pid.mi_s1s2_t;
    // Co-information: MI(S1;T) + MI(S2;T) - MI(S1,S2;T)
    let co_information = pid.mi_s1_t + pid.mi_s2_t - mi_s1s2_t;
    // Saturation diagnostics (grandplan §7.6).
    let unique_fraction_source_1 = prepared.source_1.unique_fraction;
    let unique_fraction_source_2 = prepared.source_2.unique_fraction;
    let unique_fraction_target = prepared.target.unique_fraction;
    let unique_fraction_joint = triple_unique_fraction(
        &prepared.source_1.category_ids,
        &prepared.source_2.category_ids,
        &prepared.target.category_ids,
    );
    let saturation_warning = [
        unique_fraction_source_1,
        unique_fraction_source_2,
        unique_fraction_target,
        unique_fraction_joint,
    ]
    .iter()
    .any(|&fraction| fraction > OFFLINE_CATEGORICAL_SATURATION_UNIQUE_FRACTION_MAX);
    let mut outcome = produced_outcome(MEASURE_CATEGORICAL_PID2, &axes, pair_diagnostics, None);
    if saturation_warning {
        outcome.status = OfflineVldaEstimateStatus::ProducedWithWarning;
        outcome.scientific_gates.estimator = OfflineVldaScientificGateVerdict::Blocked;
        outcome.scientific_gates.reason_code =
            Some(SCIENTIFIC_REASON_CATEGORICAL_SATURATION.to_string());
        outcome.reason_detail = Some(
            "fitted-categorical empirical-PMF terms are support-sparse: nearly every sample occupies \
             its own joint bin, so plug-in bias and atom allocation are not application-valid \
             (grandplan §7.6)"
                .to_string(),
        );
    }
    Ok(OfflineVldaPidPairMetrics {
        source_1: source_1.name.to_string(),
        source_2: source_2.name.to_string(),
        target: target.name.to_string(),
        outcome,
        mi_source_1_action: Some(pid.mi_s1_t),
        mi_source_2_action: Some(pid.mi_s2_t),
        mi_joint_action: Some(mi_s1s2_t),
        co_information: Some(co_information),
        redundancy: Some(pid.red.net),
        unique_source_1: Some(pid.unq1.net),
        unique_source_2: Some(pid.unq2.net),
        synergy: Some(pid.syn.net),
        categorical_sx_components: Some(OfflineVldaCategoricalSxComponents {
            redundancy: OfflineVldaCategoricalSxAtom {
                informative: pid.red.informative,
                misinformative: pid.red.misinformative,
                net: pid.red.net,
            },
            unique_source_1: OfflineVldaCategoricalSxAtom {
                informative: pid.unq1.informative,
                misinformative: pid.unq1.misinformative,
                net: pid.unq1.net,
            },
            unique_source_2: OfflineVldaCategoricalSxAtom {
                informative: pid.unq2.informative,
                misinformative: pid.unq2.misinformative,
                net: pid.unq2.net,
            },
            synergy: OfflineVldaCategoricalSxAtom {
                informative: pid.syn.informative,
                misinformative: pid.syn.misinformative,
                net: pid.syn.net,
            },
        }),
        categorical_saturation: Some(OfflineVldaCategoricalSaturation {
            unique_fraction_source_1,
            unique_fraction_source_2,
            unique_fraction_target,
            unique_fraction_joint,
            empirical_sample_count: empirical_pmf.sample_count,
            observed_joint_states: empirical_pmf.observed_joint_states,
            singleton_joint_states: empirical_pmf.singleton_joint_states,
            low_count_joint_states: empirical_pmf.low_count_joint_states,
            minimum_observed_count: empirical_pmf.minimum_observed_count,
            maximum_observed_count: empirical_pmf.maximum_observed_count,
            observed_coverage_indicator: empirical_pmf.observed_coverage_indicator,
            population_caveat: empirical_pmf.population_caveat.to_string(),
            saturation_warning,
        }),
    })
}

/// Mean Pearson lag-1 correlation across the defined columns of one standardized axis matrix.
/// Lag pairs never cross episode boundaries. Each unit-step run's left and right lag vectors are
/// centered separately before their residual products are pooled. This prevents between-run level
/// differences from appearing as temporal dependence.
fn axis_lag1_autocorr(
    matrix: &MatOwned,
    segments: &[std::ops::Range<usize>],
) -> (Option<f64>, usize) {
    let m = matrix.as_ref();
    let d = m.ncols();
    let correlation_segments = segments
        .iter()
        .filter(|segment| segment.len() >= 4)
        .cloned()
        .collect::<Vec<_>>();
    if d == 0 || correlation_segments.is_empty() {
        return (None, 0);
    }
    let mut correlation_sum = 0.0;
    let mut defined_dimensions = 0usize;
    for column in 0..d {
        // Scale first so finite, extreme inputs cannot overflow the sums of squares.
        let scale = correlation_segments
            .iter()
            .flat_map(|segment| segment.start..segment.end.saturating_sub(1))
            .fold(0.0_f64, |maximum, row| {
                maximum
                    .max(m.row(row)[column].abs())
                    .max(m.row(row + 1)[column].abs())
            });
        if scale == 0.0 {
            continue;
        }
        let mut centered_cross = 0.0;
        let mut centered_left_square = 0.0;
        let mut centered_right_square = 0.0;
        for segment in &correlation_segments {
            let segment_pairs = segment.len().saturating_sub(1);
            let (left_sum, right_sum) =
                (segment.start..segment.end - 1).fold((0.0, 0.0), |(left_sum, right_sum), row| {
                    (
                        left_sum + m.row(row)[column] / scale,
                        right_sum + m.row(row + 1)[column] / scale,
                    )
                });
            let left_mean = left_sum / segment_pairs as f64;
            let right_mean = right_sum / segment_pairs as f64;
            for row in segment.start..segment.end.saturating_sub(1) {
                let left = m.row(row)[column] / scale - left_mean;
                let right = m.row(row + 1)[column] / scale - right_mean;
                centered_cross += left * right;
                centered_left_square += left * left;
                centered_right_square += right * right;
            }
        }
        let denominator = centered_left_square.sqrt() * centered_right_square.sqrt();
        if denominator > 0.0 {
            correlation_sum += (centered_cross / denominator).clamp(-1.0, 1.0);
            defined_dimensions += 1;
        }
    }
    (
        (defined_dimensions > 0)
            .then(|| (correlation_sum / defined_dimensions as f64).clamp(-1.0, 1.0)),
        defined_dimensions,
    )
}

/// See [`OfflineVldaTemporalReport`]. Segments are maximal runs of consecutive rows sharing an
/// `episode_id`. Without ids, the report has no within-series segment and emits no lag statistic.
fn compute_temporal_report(
    samples: &[OfflineVldaSample],
    prepared: &PreparedVldaMatrices,
) -> OfflineVldaTemporalReport {
    let n = samples.len();
    let have_ids = samples.iter().all(|sample| sample.episode_id.is_some());
    let have_no_ids = samples.iter().all(|sample| sample.episode_id.is_none());
    let mut segments: Vec<std::ops::Range<usize>> = Vec::new();
    if have_ids {
        let mut start = 0usize;
        for idx in 1..=n {
            let boundary = idx == n || samples[idx].episode_id != samples[idx - 1].episode_id;
            if boundary {
                segments.push(start..idx);
                start = idx;
            }
        }
    } else if !have_no_ids {
        // Missing ids do not authorize a synthetic bridge between two known episode segments.
        // Retain only maximal runs with the same known id. Treat each missing-id row as a
        // singleton, which contributes no lag pair.
        let mut idx = 0usize;
        while idx < n {
            let start = idx;
            let Some(episode_id) = samples[idx].episode_id.as_deref() else {
                segments.push(idx..idx + 1);
                idx += 1;
                continue;
            };
            idx += 1;
            while idx < n && samples[idx].episode_id.as_deref() == Some(episode_id) {
                idx += 1;
            }
            segments.push(start..idx);
        }
    }

    let mut variables = BTreeMap::new();
    let potential_lag_pairs = segments
        .iter()
        .map(|segment| segment.len().saturating_sub(1))
        .sum::<usize>();
    let order_verified =
        potential_lag_pairs > 0 && segments_have_strict_sequence_index(samples, &segments);
    let (unit_step_segments, sequence_index_gap_pairs) = if order_verified {
        split_segments_at_sequence_index_gaps(samples, &segments)
    } else {
        (Vec::new(), 0)
    };
    let lag_pairs = if order_verified {
        potential_lag_pairs - sequence_index_gap_pairs
    } else {
        0
    };
    let correlation_lag_pairs = unit_step_segments
        .iter()
        .filter(|segment| segment.len() >= 4)
        .map(|segment| segment.len() - 1)
        .sum();
    for (name, matrix) in [
        ("V", &prepared.v),
        ("L", &prepared.l),
        ("D", &prepared.d),
        ("A", &prepared.a),
    ] {
        let (r1, dimensions_with_defined_lag1) = axis_lag1_autocorr(matrix, &unit_step_segments);
        variables.insert(
            name.to_string(),
            OfflineVldaTemporalVariable {
                lag1_autocorr: r1,
                dimensions_total: matrix.as_ref().ncols(),
                dimensions_with_defined_lag1,
            },
        );
    }
    OfflineVldaTemporalReport {
        variables,
        segments: segments.len(),
        potential_lag_pairs,
        lag_pairs,
        correlation_lag_pairs,
        sequence_index_gap_pairs,
        scope: if have_ids {
            "within_episode".to_string()
        } else if have_no_ids {
            "unidentified_without_episode_ids".to_string()
        } else {
            "known_episode_segments_only_mixed_ids".to_string()
        },
        interpretation: "descriptive_within_unit_step_run_pearson_lag1_not_estimator_effective_sample_size_or_block_selector".to_string(),
        ordering_basis: if order_verified {
            "strict_canonical_metadata_sequence_index_unit_steps_within_segments".to_string()
        } else if have_no_ids {
            "episode_identity_absent_no_lag_pairs".to_string()
        } else if potential_lag_pairs == 0 {
            "no_within_segment_pair".to_string()
        } else {
            "missing_or_nonmonotone_metadata_sequence_index_no_lag_pairs".to_string()
        },
    }
}

fn compute_geometry_report(prepared: &PreparedVldaMatrices) -> Result<OfflineVldaGeometryReport> {
    let metric = Metric::Chebyshev;
    let intrinsic_cfg = IntrinsicDimConfig::default()
        .with_k(OFFLINE_GEOMETRY_INTRINSIC_K)
        .with_metric(metric);
    let distance_cfg = DistanceConcentrationConfig::default().with_metric(metric);
    let hyperbolicity_cfg = HyperbolicityConfig::default()
        .with_n_samples(OFFLINE_GEOMETRY_HYPERBOLICITY_SAMPLES)
        .with_metric(metric)
        .with_seed(0x2026_0509);
    let mut variables = BTreeMap::new();
    for (name, matrix) in [
        ("V", prepared.v.as_ref()),
        ("L", prepared.l.as_ref()),
        ("D", prepared.d.as_ref()),
        ("A", prepared.a.as_ref()),
    ] {
        variables.insert(
            name.to_string(),
            compute_geometry_variable(matrix, &intrinsic_cfg, &distance_cfg, &hyperbolicity_cfg),
        );
    }
    let vl = concatenate_rows(&[prepared.v.as_ref(), prepared.l.as_ref()])?;
    variables.insert(
        "VL".to_string(),
        compute_geometry_variable(
            vl.as_ref(),
            &intrinsic_cfg,
            &distance_cfg,
            &hyperbolicity_cfg,
        ),
    );
    drop(vl);
    let vlda = concatenate_rows(&[
        prepared.v.as_ref(),
        prepared.l.as_ref(),
        prepared.d.as_ref(),
        prepared.a.as_ref(),
    ])?;
    variables.insert(
        "VLDA".to_string(),
        compute_geometry_variable(
            vlda.as_ref(),
            &intrinsic_cfg,
            &distance_cfg,
            &hyperbolicity_cfg,
        ),
    );
    let diagnostics = compute_geometry_diagnostics(&variables, &prepared.preprocessing);
    Ok(OfflineVldaGeometryReport {
        space: "per_variable_standardized".to_string(),
        metric: "chebyshev".to_string(),
        intrinsic_k: OFFLINE_GEOMETRY_INTRINSIC_K,
        hyperbolicity_samples: OFFLINE_GEOMETRY_HYPERBOLICITY_SAMPLES,
        diagnostics,
        variables,
    })
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
    // The current pid-core review surface uses `sampled_four_point_delta_summary` and returns a
    // distribution rather than one number. `.mean` is the same sampled-mean delta this field
    // always held — descriptive only, never a validity gate (`grandplan.md` §7.9).
    let (gromov_delta, gromov_error) =
        match sampled_four_point_delta_summary(matrix, hyperbolicity_cfg) {
            Ok(summary) if summary.mean.is_finite() => (Some(summary.mean), None),
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

fn compute_geometry_diagnostics(
    variables: &BTreeMap<String, OfflineVldaGeometryVariable>,
    preprocessing: &OfflineVldaPreprocessingReport,
) -> OfflineVldaGeometryDiagnostics {
    let mut warnings = Vec::new();
    // Degenerate-sample guard. An axis that is constant in this observed sample carries no
    // sample variation, and the continuous estimator rejects its exact ties. This does not prove
    // that the population variable is constant, that a measure-relative atom is zero, or that a
    // discrete estimand is undefined. Reuse the preprocessing count so a fabricated all-zero
    // channel is never silently treated as eligible continuous evidence.
    for (name, variable) in &preprocessing.variables {
        if variable.input_dim > 0 && variable.zero_variance_dims == variable.input_dim {
            warnings.push(format!(
                "geometry {name} is all-constant (zero_variance_dims == input_dim == {}): \
                 this observed sample has no {name} variation and is ineligible for the current \
                 continuous estimator; do not infer a population law or zero PID atom",
                variable.input_dim
            ));
        }
    }
    for (name, variable) in variables {
        match variable.intrinsic_dimension {
            Some(value) if value > OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION_WARNING => warnings.push(
                format!(
                    "geometry {name} intrinsic_dimension {value:.4} exceeds the descriptive warning threshold {OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION_WARNING:.4}"
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
            Some(value) if value < OFFLINE_GEOMETRY_MIN_PAIRWISE_CV_WARNING => warnings.push(format!(
                "geometry {name} pairwise_cv {value:.4} is below the descriptive warning threshold {OFFLINE_GEOMETRY_MIN_PAIRWISE_CV_WARNING:.4}"
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
        if variable.gromov_delta_rel.is_none() {
            warnings.push(format!(
                "geometry {name} delta_rel unavailable: {}",
                variable
                    .gromov_error
                    .as_deref()
                    .unwrap_or("missing diameter")
            ));
        }
    }
    OfflineVldaGeometryDiagnostics {
        status: if warnings.is_empty() {
            "clear".to_string()
        } else {
            "warning".to_string()
        },
        max_intrinsic_dimension_warning: OFFLINE_GEOMETRY_MAX_INTRINSIC_DIMENSION_WARNING,
        min_pairwise_cv_warning: OFFLINE_GEOMETRY_MIN_PAIRWISE_CV_WARNING,
        warnings,
    }
}

fn flatten_selected<F>(
    samples: &[OfflineVldaSample],
    train_roles: Option<&[OfflineVldaSplitRole]>,
    expected_rows: usize,
    dim: usize,
    values: F,
) -> Result<Vec<f64>>
where
    F: Fn(&OfflineVldaSample) -> &[f64],
{
    ensure!(
        train_roles.is_none_or(|roles| roles.len() == samples.len()),
        "offline VLDA selected roles do not match the sample count"
    );
    let scalars = expected_rows
        .checked_mul(dim)
        .context("offline VLDA selected matrix size overflowed usize")?;
    let mut out = Vec::new();
    out.try_reserve_exact(scalars)
        .context("failed to reserve offline VLDA selected matrix")?;
    let mut rows = 0usize;
    for (index, sample) in samples.iter().enumerate() {
        if train_roles.is_some_and(|roles| roles[index] != OfflineVldaSplitRole::Train) {
            continue;
        }
        let row = values(sample);
        ensure!(
            row.len() == dim,
            "offline VLDA selected row has width {}, expected {dim}",
            row.len()
        );
        out.extend_from_slice(row);
        rows += 1;
    }
    ensure!(
        rows == expected_rows,
        "offline VLDA selection produced {rows} rows, expected {expected_rows}"
    );
    Ok(out)
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
        let Some(episode_id) = sample.episode_id.as_deref() else {
            missing_episode_samples += 1;
            continue;
        };
        match role {
            OfflineVldaSplitRole::Train => {
                train_episode_ids.insert(episode_id);
            }
            OfflineVldaSplitRole::Heldout => {
                heldout_episode_ids.insert(episode_id);
            }
        }
    }
    let shared_episode_ids = train_episode_ids
        .intersection(&heldout_episode_ids)
        .map(|episode_id| (*episode_id).to_string())
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

fn success_metrics(labels: Option<&[bool]>) -> (Option<f64>, Option<f64>) {
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

fn append_all_heldout_centroid_prediction_records(
    records: &mut Vec<OfflineVldaHeldoutPredictionRecord>,
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
) -> Result<Option<OfflineVldaCentroidModel>> {
    let Some(first) = samples.first() else {
        return Ok(None);
    };
    let v_end = first.v.len();
    let l_end = v_end
        .checked_add(first.l.len())
        .context("offline VLDA centroid feature dimension overflow")?;
    let d_end = l_end
        .checked_add(first.d.len())
        .context("offline VLDA centroid feature dimension overflow")?;
    let a_end = d_end
        .checked_add(first.a.len())
        .context("offline VLDA centroid feature dimension overflow")?;
    let Some(model) = train_standardized_centroid_model(samples, labels, roles)? else {
        return Ok(None);
    };
    for (variable, range) in [
        ("V", 0..v_end),
        ("L", v_end..l_end),
        ("D", l_end..d_end),
        ("A", d_end..a_end),
        ("VLDA", 0..a_end),
    ] {
        append_heldout_centroid_prediction_records_from_model(
            records, samples, labels, roles, variable, range, &model,
        )?;
    }
    Ok(Some(model))
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

struct OfflineVldaNnBaselines {
    loo_accuracy: f64,
    episode_accuracy: Option<f64>,
    heldout_predictions: Vec<OfflineVldaHeldoutPredictionRecord>,
}

fn update_nearest_candidate(
    best: &mut Option<(usize, f64)>,
    samples: &[OfflineVldaSample],
    candidate_idx: usize,
    distance: f64,
) {
    let replace = match *best {
        None => true,
        Some((current_idx, current_distance)) => match distance.total_cmp(&current_distance) {
            Ordering::Less => true,
            Ordering::Equal => {
                samples[candidate_idx].sample_id.as_str() < samples[current_idx].sample_id.as_str()
            }
            Ordering::Greater => false,
        },
    };
    if replace {
        *best = Some((candidate_idx, distance));
    }
}

fn compute_nn_baselines<F>(
    samples: &[OfflineVldaSample],
    labels: &[bool],
    episode_ids: Option<&[&str]>,
    roles: Option<&[OfflineVldaSplitRole]>,
    variable: &str,
    squared_distance: F,
) -> Result<OfflineVldaNnBaselines>
where
    F: Fn(&OfflineVldaSample, &OfflineVldaSample) -> f64,
{
    ensure!(
        samples.len() == labels.len()
            && episode_ids.is_none_or(|ids| ids.len() == samples.len())
            && roles.is_none_or(|roles| roles.len() == samples.len()),
        "offline VLDA nearest-neighbor inputs have inconsistent lengths"
    );
    let mut nearest = vec![None; samples.len()];
    let mut nearest_other_episode = vec![None; samples.len()];
    let mut nearest_train = vec![None; samples.len()];
    // Distance is symmetric. Visit each unordered pair once, then update both
    // query rows. This halves the dominant baseline work without changing the
    // sample-id tie rule or any emitted prediction.
    for left_idx in 0..samples.len() {
        for right_idx in (left_idx + 1)..samples.len() {
            let distance = squared_distance(&samples[left_idx], &samples[right_idx]);
            ensure!(
                distance.is_finite() && distance >= 0.0,
                "offline VLDA {variable} squared distance is not finite for samples {} and {}",
                samples[left_idx].sample_id,
                samples[right_idx].sample_id
            );
            update_nearest_candidate(&mut nearest[left_idx], samples, right_idx, distance);
            update_nearest_candidate(&mut nearest[right_idx], samples, left_idx, distance);
            if episode_ids.is_some_and(|ids| ids[left_idx] != ids[right_idx]) {
                update_nearest_candidate(
                    &mut nearest_other_episode[left_idx],
                    samples,
                    right_idx,
                    distance,
                );
                update_nearest_candidate(
                    &mut nearest_other_episode[right_idx],
                    samples,
                    left_idx,
                    distance,
                );
            }
            if let Some(roles) = roles {
                match (roles[left_idx], roles[right_idx]) {
                    (OfflineVldaSplitRole::Heldout, OfflineVldaSplitRole::Train) => {
                        update_nearest_candidate(
                            &mut nearest_train[left_idx],
                            samples,
                            right_idx,
                            distance,
                        );
                    }
                    (OfflineVldaSplitRole::Train, OfflineVldaSplitRole::Heldout) => {
                        update_nearest_candidate(
                            &mut nearest_train[right_idx],
                            samples,
                            left_idx,
                            distance,
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    let mut loo_correct = 0usize;
    let mut episode_correct = 0usize;
    let mut heldout_predictions = Vec::new();
    for idx in 0..samples.len() {
        let (nearest_idx, _) = nearest[idx].context(
            "offline VLDA nearest-neighbor baseline requires at least two validated samples",
        )?;
        loo_correct += usize::from(labels[nearest_idx] == labels[idx]);
        if episode_ids.is_some() {
            let (nearest_idx, _) = nearest_other_episode[idx].context(
                "offline VLDA episode baseline requires at least two validated episodes",
            )?;
            episode_correct += usize::from(labels[nearest_idx] == labels[idx]);
        }
        if roles.is_some_and(|roles| roles[idx] == OfflineVldaSplitRole::Heldout) {
            let (nearest_idx, squared_distance) = nearest_train[idx].context(
                "offline VLDA held-out baseline requires at least one validated train sample",
            )?;
            heldout_predictions.push(heldout_prediction_record(
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
    Ok(OfflineVldaNnBaselines {
        loo_accuracy: loo_correct as f64 / labels.len() as f64,
        episode_accuracy: episode_ids.map(|_| episode_correct as f64 / labels.len() as f64),
        heldout_predictions,
    })
}

fn append_heldout_centroid_prediction_records_from_model(
    records: &mut Vec<OfflineVldaHeldoutPredictionRecord>,
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    variable: &str,
    feature_range: std::ops::Range<usize>,
    model: &OfflineVldaCentroidModel,
) -> Result<()> {
    for idx in heldout_indices(roles) {
        let false_distance = squared_euclidean(
            &model.row(idx)[feature_range.clone()],
            &model.centroids[0][feature_range.clone()],
        );
        let true_distance = squared_euclidean(
            &model.row(idx)[feature_range.clone()],
            &model.centroids[1][feature_range.clone()],
        );
        let score = false_distance - true_distance;
        ensure!(
            false_distance.is_finite() && true_distance.is_finite() && score.is_finite(),
            "offline VLDA {variable} centroid score is not finite for sample {}",
            samples[idx].sample_id
        );
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
    Ok(())
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

fn episode_ids(samples: &[OfflineVldaSample]) -> Option<Vec<&str>> {
    let episode_ids = samples
        .iter()
        .map(|sample| sample.episode_id.as_deref())
        .collect::<Option<Vec<_>>>()?;
    (episode_ids.iter().copied().collect::<BTreeSet<_>>().len() >= 2).then_some(episode_ids)
}

fn episode_loo_majority_success_accuracy(labels: &[bool], episode_ids: &[&str]) -> f64 {
    let total_successes = labels.iter().filter(|label| **label).count();
    let mut episode_counts = BTreeMap::<&str, (usize, usize)>::new();
    for (label, episode_id) in labels.iter().zip(episode_ids) {
        let (successes, total) = episode_counts.entry(episode_id).or_default();
        *total += 1;
        *successes += usize::from(*label);
    }
    let correct = labels
        .iter()
        .enumerate()
        .filter(|(idx, label)| {
            let (episode_successes, episode_total) = episode_counts[episode_ids[*idx]];
            let outside_successes = total_successes - episode_successes;
            let outside_total = labels.len() - episode_total;
            let majority = outside_successes * 2 >= outside_total;
            majority == **label
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
    features: Vec<f64>,
    feature_dim: usize,
    centroids: [Vec<f64>; 2],
}

impl OfflineVldaCentroidModel {
    fn row(&self, index: usize) -> &[f64] {
        let start = index * self.feature_dim;
        &self.features[start..start + self.feature_dim]
    }
}

fn heldout_metrics_from_records(
    records: &[OfflineVldaHeldoutPredictionRecord],
    classifier: &str,
    variable: Option<&str>,
) -> Option<OfflineVldaHeldoutClassifierMetrics> {
    let mut correct = 0usize;
    let mut total = 0usize;
    let mut class_correct = [0usize; 2];
    let mut class_total = [0usize; 2];
    let mut scores = Vec::new();
    for record in records
        .iter()
        .filter(|record| record.classifier == classifier && record.variable.as_deref() == variable)
    {
        let class = usize::from(record.true_success);
        total += 1;
        class_total[class] += 1;
        if record.correct {
            correct += 1;
            class_correct[class] += 1;
        }
        if let Some(score) = record.score {
            scores.push((score, record.true_success));
        }
    }
    if total == 0 {
        return None;
    }
    let balanced_accuracy = (class_total[0] > 0 && class_total[1] > 0).then_some(
        (class_correct[0] as f64 / class_total[0] as f64
            + class_correct[1] as f64 / class_total[1] as f64)
            / 2.0,
    );
    Some(OfflineVldaHeldoutClassifierMetrics {
        accuracy: correct as f64 / total as f64,
        balanced_accuracy,
        auroc: heldout_auroc(&scores),
    })
}

fn validate_heldout_prediction_contract(report: &OfflineVldaReport) -> Result<()> {
    let records = &report.heldout_predictions;
    if records.is_empty() {
        ensure!(
            report.heldout_failure_diagnostics.is_empty(),
            "offline VLDA report has held-out failure diagnostics without prediction records"
        );
    } else {
        let split = report
            .heldout_split
            .as_ref()
            .context("offline VLDA report has held-out predictions without a split")?;
        let heldout_ids = split
            .heldout_sample_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut identities = BTreeSet::new();
        for record in records {
            ensure!(
                heldout_ids.contains(record.sample_id.as_str()),
                "offline VLDA held-out prediction names a sample outside the held-out split: {}",
                record.sample_id
            );
            ensure!(
                record.correct == (record.predicted_success == record.true_success),
                "offline VLDA held-out prediction has an inconsistent correctness flag for {}",
                record.sample_id
            );
            ensure!(
                record.score.is_some() == record.score_name.is_some(),
                "offline VLDA held-out prediction score and score name disagree for {}",
                record.sample_id
            );
            ensure!(
                record.score.is_none_or(f64::is_finite)
                    && record
                        .squared_distance
                        .is_none_or(|distance| distance.is_finite() && distance >= 0.0),
                "offline VLDA held-out prediction has a non-finite or negative diagnostic for {}",
                record.sample_id
            );
            let identity = (
                record.classifier.as_str(),
                record.variable.as_deref(),
                record.sample_id.as_str(),
            );
            ensure!(
                identities.insert(identity),
                "offline VLDA held-out prediction identity is duplicated: classifier={}, variable={:?}, sample_id={}",
                record.classifier,
                record.variable,
                record.sample_id
            );
            match (record.classifier.as_str(), record.variable.as_deref()) {
                ("train_split_majority", None) => ensure!(
                    record.score.is_none()
                        && record.nearest_train_sample_id.is_none()
                        && record.squared_distance.is_none(),
                    "offline VLDA majority prediction carries an inapplicable diagnostic"
                ),
                ("train_split_1nn", Some("V" | "L" | "D" | "A" | "VLDA")) => {
                    ensure!(
                        record.score.is_none()
                            && record.nearest_train_sample_id.is_some()
                            && record.squared_distance.is_some(),
                        "offline VLDA 1-NN prediction has an invalid diagnostic shape"
                    );
                }
                (
                    "train_split_nearest_centroid",
                    Some("V" | "L" | "D" | "A" | "VLDA"),
                ) => {
                    ensure!(
                        record.score_name.as_deref() == Some(OFFLINE_CENTROID_SUCCESS_SCORE)
                            && record
                                .score
                                .is_some_and(|score| record.predicted_success == (score > 0.0))
                            && record.nearest_train_sample_id.is_none()
                            && record.squared_distance.is_none(),
                        "offline VLDA centroid prediction has an invalid score contract"
                    );
                }
                ("train_split_logreg", Some("VLDA")) => ensure!(
                    record.score_name.as_deref() == Some("decision_function_logit")
                        && record
                            .score
                            .is_some_and(|score| record.predicted_success == (score >= 0.0))
                        && record.nearest_train_sample_id.is_none()
                        && record.squared_distance.is_none(),
                    "offline VLDA logistic prediction has an invalid score contract"
                ),
                _ => bail!(
                    "offline VLDA held-out prediction has an unknown classifier/variable contract: {}/{}",
                    record.classifier,
                    record.variable.as_deref().unwrap_or("none")
                ),
            }
        }
        let mut group_counts = BTreeMap::<(&str, Option<&str>), usize>::new();
        for record in records {
            *group_counts
                .entry((record.classifier.as_str(), record.variable.as_deref()))
                .or_default() += 1;
        }
        ensure!(
            group_counts
                .values()
                .all(|count| *count == split.heldout_samples),
            "offline VLDA held-out prediction group omits one or more held-out samples"
        );
    }

    let metrics = &report.metrics;
    for (classifier, variable, actual_accuracy, actual_balanced, actual_auroc) in [
        (
            "train_split_majority",
            None,
            metrics.heldout_majority_success_accuracy,
            metrics.heldout_majority_success_balanced_accuracy,
            None,
        ),
        (
            "train_split_1nn",
            Some("V"),
            metrics.heldout_nn_v_success_accuracy,
            metrics.heldout_nn_v_success_balanced_accuracy,
            None,
        ),
        (
            "train_split_1nn",
            Some("L"),
            metrics.heldout_nn_l_success_accuracy,
            metrics.heldout_nn_l_success_balanced_accuracy,
            None,
        ),
        (
            "train_split_1nn",
            Some("D"),
            metrics.heldout_nn_d_success_accuracy,
            metrics.heldout_nn_d_success_balanced_accuracy,
            None,
        ),
        (
            "train_split_1nn",
            Some("A"),
            metrics.heldout_nn_a_success_accuracy,
            metrics.heldout_nn_a_success_balanced_accuracy,
            None,
        ),
        (
            "train_split_1nn",
            Some("VLDA"),
            metrics.heldout_nn_vlda_success_accuracy,
            metrics.heldout_nn_vlda_success_balanced_accuracy,
            None,
        ),
        (
            "train_split_nearest_centroid",
            Some("V"),
            metrics.heldout_centroid_v_success_accuracy,
            metrics.heldout_centroid_v_success_balanced_accuracy,
            metrics.heldout_centroid_v_success_auroc,
        ),
        (
            "train_split_nearest_centroid",
            Some("L"),
            metrics.heldout_centroid_l_success_accuracy,
            metrics.heldout_centroid_l_success_balanced_accuracy,
            metrics.heldout_centroid_l_success_auroc,
        ),
        (
            "train_split_nearest_centroid",
            Some("D"),
            metrics.heldout_centroid_d_success_accuracy,
            metrics.heldout_centroid_d_success_balanced_accuracy,
            metrics.heldout_centroid_d_success_auroc,
        ),
        (
            "train_split_nearest_centroid",
            Some("A"),
            metrics.heldout_centroid_a_success_accuracy,
            metrics.heldout_centroid_a_success_balanced_accuracy,
            metrics.heldout_centroid_a_success_auroc,
        ),
        (
            "train_split_nearest_centroid",
            Some("VLDA"),
            metrics.heldout_centroid_vlda_success_accuracy,
            metrics.heldout_centroid_vlda_success_balanced_accuracy,
            metrics.heldout_centroid_vlda_success_auroc,
        ),
        (
            "train_split_logreg",
            Some("VLDA"),
            metrics.heldout_logreg_vlda_success_accuracy,
            metrics.heldout_logreg_vlda_success_balanced_accuracy,
            metrics.heldout_logreg_vlda_success_auroc,
        ),
    ] {
        let expected = heldout_metrics_from_records(records, classifier, variable);
        ensure!(
            same_optional_f64(actual_accuracy, expected.map(|value| value.accuracy))
                && same_optional_f64(
                    actual_balanced,
                    expected.and_then(|value| value.balanced_accuracy)
                )
                && same_optional_f64(actual_auroc, expected.and_then(|value| value.auroc)),
            "offline VLDA held-out aggregate does not reconstruct from predictions: classifier={classifier}, variable={variable:?}"
        );
    }

    ensure!(
        report.heldout_failure_diagnostics == heldout_failure_diagnostics(records),
        "offline VLDA held-out failure diagnostics do not reconstruct from predictions"
    );
    Ok(())
}

fn train_standardized_centroid_model(
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
) -> Result<Option<OfflineVldaCentroidModel>> {
    let Some(first) = samples.first() else {
        return Ok(None);
    };
    let dim = first
        .v
        .len()
        .checked_add(first.l.len())
        .and_then(|value| value.checked_add(first.d.len()))
        .and_then(|value| value.checked_add(first.a.len()))
        .context("offline VLDA centroid feature dimension overflow")?;
    if dim == 0 {
        return Ok(None);
    }
    ensure!(
        labels.len() == samples.len() && roles.len() == samples.len(),
        "offline VLDA centroid inputs have inconsistent dimensions"
    );
    let feature_count = samples
        .len()
        .checked_mul(dim)
        .context("offline VLDA centroid feature count overflow")?;
    let mut features = Vec::with_capacity(feature_count);
    for sample in samples {
        let sample_dim = sample
            .v
            .len()
            .checked_add(sample.l.len())
            .and_then(|value| value.checked_add(sample.d.len()))
            .and_then(|value| value.checked_add(sample.a.len()))
            .context("offline VLDA centroid feature dimension overflow")?;
        ensure!(
            sample_dim == dim,
            "offline VLDA centroid inputs have inconsistent dimensions"
        );
        features.extend_from_slice(&sample.v);
        features.extend_from_slice(&sample.l);
        features.extend_from_slice(&sample.d);
        features.extend_from_slice(&sample.a);
    }
    let train_total = roles
        .iter()
        .filter(|role| **role == OfflineVldaSplitRole::Train)
        .count();
    if train_total == 0 {
        return Ok(None);
    }

    // Scale each column before summation. This gives the same train-only
    // z-score semantics without overflowing merely because finite inputs have
    // a large common magnitude.
    let mut scale = vec![0.0_f64; dim];
    for (feature, role) in features.chunks_exact(dim).zip(roles) {
        if *role == OfflineVldaSplitRole::Train {
            for (scale, value) in scale.iter_mut().zip(feature) {
                *scale = scale.max(value.abs());
            }
        }
    }
    let mut scaled_mean = vec![0.0; dim];
    for (feature, role) in features.chunks_exact(dim).zip(roles) {
        if *role == OfflineVldaSplitRole::Train {
            for ((sum, value), scale) in scaled_mean.iter_mut().zip(feature).zip(&scale) {
                if *scale != 0.0 {
                    *sum += *value / *scale;
                }
            }
        }
    }
    for value in &mut scaled_mean {
        *value /= train_total as f64;
    }
    let mut variance = vec![0.0; dim];
    for (feature, role) in features.chunks_exact(dim).zip(roles) {
        if *role == OfflineVldaSplitRole::Train {
            for (((sum, value), scale), mean) in variance
                .iter_mut()
                .zip(feature)
                .zip(&scale)
                .zip(&scaled_mean)
            {
                let scaled = if *scale == 0.0 { 0.0 } else { *value / *scale };
                let delta = scaled - mean;
                *sum += delta * delta;
            }
        }
    }
    let inv_std = variance
        .into_iter()
        .map(|sum| {
            if sum == 0.0 {
                // A train-constant feature has no fitted information. Mapping
                // it to zero for every row also prevents an arbitrary held-out
                // deviation from adding a common, possibly overflowing term to
                // both centroid distances.
                0.0
            } else {
                (train_total as f64 / sum).sqrt()
            }
        })
        .collect::<Vec<_>>();
    for feature in features.chunks_exact_mut(dim) {
        for (((value, scale), mean), inv_std) in feature
            .iter_mut()
            .zip(&scale)
            .zip(&scaled_mean)
            .zip(&inv_std)
        {
            let scaled = if *scale == 0.0 { 0.0 } else { *value / *scale };
            *value = (scaled - mean) * inv_std;
            ensure!(
                value.is_finite(),
                "offline VLDA train-standardized feature is not finite"
            );
        }
    }
    let mut centroids = [vec![0.0; dim], vec![0.0; dim]];
    let mut counts = [0usize, 0usize];
    for (idx, feature) in features.chunks_exact(dim).enumerate() {
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
        return Ok(None);
    }
    for (centroid, count) in centroids.iter_mut().zip(counts) {
        for value in centroid {
            *value /= count as f64;
        }
    }
    Ok(Some(OfflineVldaCentroidModel {
        features,
        feature_dim: dim,
        centroids,
    }))
}

/// SAFE-class internal-feature failure-detector baseline: fit an L2-regularized
/// logistic regression on the train split (features standardized with train-only
/// statistics) and score the held-out split. The caller shares the exact VLDA
/// standardization used by the centroid baseline. A requested fit failure is an
/// analysis error.
fn append_heldout_logreg_prediction_records(
    records: &mut Vec<OfflineVldaHeldoutPredictionRecord>,
    samples: &[OfflineVldaSample],
    labels: &[bool],
    roles: &[OfflineVldaSplitRole],
    model: &OfflineVldaCentroidModel,
    dense_solver_budget: ResourceBudget,
) -> Result<()> {
    let dim = model.feature_dim;
    if dim == 0 {
        return Ok(());
    }

    // Assemble the train design matrix + labels (standardized features, train rows).
    let mut train_rows = Vec::new();
    let mut train_labels = Vec::new();
    for (idx, role) in roles.iter().enumerate() {
        if *role == OfflineVldaSplitRole::Train {
            train_rows.extend_from_slice(model.row(idx));
            train_labels.push(labels[idx]);
        }
    }
    let n_train = train_labels.len();
    if n_train == 0 {
        return Ok(());
    }
    let x_train = MatOwned::new(train_rows, n_train, dim)
        .context("failed to construct held-out logistic-regression training matrix")?;
    // This SAFE-class static factual-outcome baseline is dependency groundwork, not an H1
    // response or prospective H2 endpoint. Once applicable and admitted, it must not disappear.
    let logreg = LogisticRegression::fit_with_budget(
        x_train.as_ref(),
        &train_labels,
        &LogisticRegressionConfig::default(),
        dense_solver_budget,
    )
    .context("held-out VLDA logistic-regression baseline failed")?;

    for idx in heldout_indices(roles) {
        // Decision-function logit on the train-standardized held-out features.
        let logit = logreg.intercept()
            + model
                .row(idx)
                .iter()
                .zip(logreg.weights())
                .map(|(a, b)| a * b)
                .sum::<f64>();
        ensure!(
            logit.is_finite(),
            "offline VLDA held-out logistic-regression score is not finite for sample {}",
            samples[idx].sample_id
        );
        records.push(heldout_prediction_record(
            samples,
            labels,
            idx,
            OfflineVldaHeldoutPredictionInput {
                classifier: "train_split_logreg",
                variable: Some("VLDA"),
                predicted_success: logit >= 0.0,
                score: Some(logit),
                score_name: Some("decision_function_logit".to_string()),
                nearest_train_sample_id: None,
                squared_distance: None,
            },
        ));
    }
    Ok(())
}

fn heldout_auroc(scores: &[(f64, bool)]) -> Option<f64> {
    let positives = scores.iter().filter(|(_, label)| *label).count();
    let negatives = scores.len().saturating_sub(positives);
    if positives == 0 || negatives == 0 {
        return None;
    }

    // Count Mann-Whitney wins by equal-score groups. This preserves the exact
    // half-credit tie rule while reducing the former O(P*N) comparison loop to
    // one O(n log n) sort and one linear scan.
    let mut ranked = scores.to_vec();
    ranked.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut negatives_before = 0u128;
    let mut doubled_wins = 0u128;
    let mut start = 0usize;
    while start < ranked.len() {
        let mut end = start + 1;
        while end < ranked.len() && ranked[end].0.total_cmp(&ranked[start].0) == Ordering::Equal {
            end += 1;
        }
        let group_positives = ranked[start..end]
            .iter()
            .filter(|(_, label)| *label)
            .count() as u128;
        let group_negatives = (end - start) as u128 - group_positives;
        doubled_wins += 2 * group_positives * negatives_before + group_positives * group_negatives;
        negatives_before += group_negatives;
        start = end;
    }
    Some(doubled_wins as f64 / (2 * positives as u128 * negatives as u128) as f64)
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

fn squared_euclidean_vlda(left: &OfflineVldaSample, right: &OfflineVldaSample) -> f64 {
    left.v
        .iter()
        .chain(&left.l)
        .chain(&left.d)
        .chain(&left.a)
        .zip(
            right
                .v
                .iter()
                .chain(&right.l)
                .chain(&right.d)
                .chain(&right.a),
        )
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
    // An abstained estimate emits NO `PidMetric`: there is no numeric placeholder for a value that
    // was never produced. Its status/reason is emitted as a structured `LabelObserved` instead, so
    // run-log replay reconstructs the abstention rather than silently seeing a missing metric.
    let vl_outcome = report.metrics.pid_pairs.get("VL").map(|pair| &pair.outcome);
    let metrics: [(&str, Option<f64>, Option<&OfflineVldaOutcome>); 9] = [
        (
            "offline_vlda.pid.mi_v_action",
            report.metrics.mi_v_action.value,
            Some(&report.metrics.mi_v_action.outcome),
        ),
        (
            "offline_vlda.pid.mi_l_action",
            report.metrics.mi_l_action.value,
            Some(&report.metrics.mi_l_action.outcome),
        ),
        (
            "offline_vlda.pid.mi_d_action",
            report.metrics.mi_d_action.value,
            Some(&report.metrics.mi_d_action.outcome),
        ),
        (
            "offline_vlda.pid.mi_vl_action",
            report.metrics.mi_vl_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.co_information_v_l_action",
            report.metrics.co_information_v_l_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.redundancy_v_l_action",
            report.metrics.redundancy_v_l_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.unique_v_action",
            report.metrics.unique_v_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.unique_l_action",
            report.metrics.unique_l_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.synergy_v_l_action",
            report.metrics.synergy_v_l_action,
            vl_outcome,
        ),
    ];
    let mut idx = 0u64;
    for (name, value, outcome) in metrics {
        let Some(value) = value else { continue };
        let outcome =
            outcome.with_context(|| format!("{name} has a value but no typed outcome"))?;
        writer.append(&RunLogEvent::PidMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + idx,
            name: name.to_string(),
            value,
            metadata: offline_vlda_pid_metric_metadata(report, name, None, outcome),
        })?;
        idx += 1;
    }
    if let Some(pair) = report.metrics.pid_pairs.get("VL") {
        write_categorical_sx_component_metric_events(
            writer,
            report,
            pair,
            OfflineVldaPidMetricEventScope {
                prefix: "offline_vlda.pid",
                train_pid: None,
            },
            timestamp_base_ns,
            &mut idx,
        )?;
    }

    // Structured abstention records + the eligibility denominators.
    for (axis, estimate) in [
        ("V", &report.metrics.mi_v_action),
        ("L", &report.metrics.mi_l_action),
        ("D", &report.metrics.mi_d_action),
    ] {
        if estimate.outcome.abstained() {
            writer.append(&RunLogEvent::LabelObserved {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: format!("offline_vlda.pid.abstained.{axis}"),
                value: serde_json::to_value(&estimate.outcome)?,
                metadata: BTreeMap::new(),
            })?;
            idx += 1;
        }
    }
    for (pair_name, pair) in &report.metrics.pid_pairs {
        if pair.outcome.abstained() {
            writer.append(&RunLogEvent::LabelObserved {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + idx,
                name: format!("offline_vlda.pid.abstained.{pair_name}"),
                value: serde_json::to_value(&pair.outcome)?,
                metadata: BTreeMap::new(),
            })?;
            idx += 1;
        }
    }
    writer.append(&RunLogEvent::LabelObserved {
        step: report.dims.samples as u64,
        timestamp_ns: timestamp_base_ns + idx,
        name: "offline_vlda.pid.estimate_denominators".to_string(),
        value: serde_json::to_value(&report.metrics.estimate_denominators)?,
        metadata: BTreeMap::new(),
    })?;
    idx += 1;
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
                None,
                metric,
            );
            metadata.insert(
                "feature_space".to_string(),
                "train_standardized_vlda".to_string(),
            );
            metadata.insert("model".to_string(), "l2_logistic_regression".to_string());
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
    outcome: &OfflineVldaOutcome,
) -> BTreeMap<String, String> {
    let mut metadata = offline_vlda_pid_scope_metadata(report, train_pid);
    // Every information quantity in this crate is in nats (pid-core convention,
    // both KSG/I^sx continuous and plug-in discrete paths). Stamp it so a
    // standalone JSONL consumer never has to guess nats vs bits.
    metadata.insert("units".to_string(), outcome.information_units.clone());
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
    insert_offline_vlda_outcome_metadata(&mut metadata, outcome);
    metadata
}

fn insert_offline_vlda_outcome_metadata(
    metadata: &mut BTreeMap<String, String>,
    outcome: &OfflineVldaOutcome,
) {
    let status = match outcome.status {
        OfflineVldaEstimateStatus::NotRequested => "not_requested",
        OfflineVldaEstimateStatus::Produced => "produced",
        OfflineVldaEstimateStatus::ProducedWithWarning => "produced_with_warning",
        OfflineVldaEstimateStatus::Abstained => "abstained",
    };
    let gate = |verdict| match verdict {
        OfflineVldaScientificGateVerdict::Passed => "passed",
        OfflineVldaScientificGateVerdict::Conditional => "conditional",
        OfflineVldaScientificGateVerdict::NotEvaluated => "not_evaluated",
        OfflineVldaScientificGateVerdict::Blocked => "blocked",
        OfflineVldaScientificGateVerdict::NotApplicable => "not_applicable",
    };
    metadata.insert("computation_status".to_string(), status.to_string());
    metadata.insert("measure".to_string(), outcome.measure.clone());
    metadata.insert(
        "estimator_revision".to_string(),
        outcome.estimator_revision.clone(),
    );
    metadata.insert("axes".to_string(), outcome.axes.join(","));
    metadata.insert(
        "scientific_gate_population".to_string(),
        gate(outcome.scientific_gates.population).to_string(),
    );
    metadata.insert(
        "scientific_gate_measure".to_string(),
        gate(outcome.scientific_gates.measure).to_string(),
    );
    metadata.insert(
        "scientific_gate_estimator".to_string(),
        gate(outcome.scientific_gates.estimator).to_string(),
    );
    metadata.insert(
        "scientific_gate_application".to_string(),
        gate(outcome.scientific_gates.application).to_string(),
    );
    metadata.insert(
        "interpretation_allowed".to_string(),
        outcome.scientific_gates.interpretation_allowed.to_string(),
    );
    if let Some(version) = &outcome.scientific_gates.support_envelope_version {
        metadata.insert("support_envelope_version".to_string(), version.clone());
    }
    if let Some(reason) = &outcome.scientific_gates.reason_code {
        metadata.insert("scientific_reason_code".to_string(), reason.clone());
        if outcome.status == OfflineVldaEstimateStatus::ProducedWithWarning {
            metadata.insert("warning_code".to_string(), reason.clone());
        }
    }
    if let Some(reason) = outcome.reason_code {
        metadata.insert(
            "computation_reason_code".to_string(),
            reason.as_str().to_string(),
        );
    }
}

fn offline_vlda_pid_pair_metric_metadata(
    report: &OfflineVldaReport,
    pair: &str,
    metrics: &OfflineVldaPidPairMetrics,
    train_pid: Option<&OfflineVldaTrainSplitPidReport>,
) -> BTreeMap<String, String> {
    let mut metadata = offline_vlda_pid_scope_metadata(report, train_pid);
    metadata.insert(
        "units".to_string(),
        metrics.outcome.information_units.clone(),
    );
    metadata.insert("pid_pair".to_string(), pair.to_string());
    metadata.insert("source_1".to_string(), metrics.source_1.clone());
    metadata.insert("source_2".to_string(), metrics.source_2.clone());
    metadata.insert("target".to_string(), metrics.target.clone());
    insert_offline_vlda_outcome_metadata(&mut metadata, &metrics.outcome);
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
        "train_split_logreg" if diagnostic.variable.as_deref() == Some("VLDA") => {
            Some("offline_vlda.baseline.heldout_logreg_vlda_failure".to_string())
        }
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
    if diagnostic.classifier == "train_split_logreg" {
        metadata.insert(
            "feature_space".to_string(),
            "train_standardized_vlda".to_string(),
        );
        metadata.insert("model".to_string(), "l2_logistic_regression".to_string());
        metadata.insert("score".to_string(), "decision_function_logit".to_string());
    }
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
        writer.append(&RunLogEvent::LabelObserved {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + *idx,
            name: "offline_vlda.pid.train_split.status".to_string(),
            value: json!({
                "status": train_pid.status.as_str(),
                "error": train_pid.error.as_deref(),
                "samples": train_pid.samples,
                "heldout_samples_excluded": train_pid.heldout_samples_excluded,
            }),
            metadata: BTreeMap::new(),
        })?;
        *idx += 1;
        return Ok(());
    };
    let vl_outcome = metrics.pid_pairs.get("VL").map(|pair| &pair.outcome);
    for (name, value, outcome) in [
        (
            "offline_vlda.pid.train_split.mi_v_action",
            metrics.mi_v_action.value,
            Some(&metrics.mi_v_action.outcome),
        ),
        (
            "offline_vlda.pid.train_split.mi_l_action",
            metrics.mi_l_action.value,
            Some(&metrics.mi_l_action.outcome),
        ),
        (
            "offline_vlda.pid.train_split.mi_d_action",
            metrics.mi_d_action.value,
            Some(&metrics.mi_d_action.outcome),
        ),
        (
            "offline_vlda.pid.train_split.mi_vl_action",
            metrics.mi_vl_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.train_split.co_information_v_l_action",
            metrics.co_information_v_l_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.train_split.redundancy_v_l_action",
            metrics.redundancy_v_l_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.train_split.unique_v_action",
            metrics.unique_v_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.train_split.unique_l_action",
            metrics.unique_l_action,
            vl_outcome,
        ),
        (
            "offline_vlda.pid.train_split.synergy_v_l_action",
            metrics.synergy_v_l_action,
            vl_outcome,
        ),
    ] {
        // Abstained train-split estimates emit no metric event and no placeholder.
        let Some(value) = value else { continue };
        let outcome =
            outcome.with_context(|| format!("{name} has a value but no typed outcome"))?;
        writer.append(&RunLogEvent::PidMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + *idx,
            name: name.to_string(),
            value,
            metadata: offline_vlda_pid_metric_metadata(report, name, Some(train_pid), outcome),
        })?;
        *idx += 1;
    }
    if let Some(pair) = metrics.pid_pairs.get("VL") {
        write_categorical_sx_component_metric_events(
            writer,
            report,
            pair,
            OfflineVldaPidMetricEventScope {
                prefix: "offline_vlda.pid.train_split",
                train_pid: Some(train_pid),
            },
            timestamp_base_ns,
            idx,
        )?;
    }
    for (axis, estimate) in [
        ("V", &metrics.mi_v_action),
        ("L", &metrics.mi_l_action),
        ("D", &metrics.mi_d_action),
    ] {
        if estimate.outcome.abstained() {
            writer.append(&RunLogEvent::LabelObserved {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + *idx,
                name: format!("offline_vlda.pid.train_split.abstained.{axis}"),
                value: serde_json::to_value(&estimate.outcome)?,
                metadata: BTreeMap::new(),
            })?;
            *idx += 1;
        }
    }
    for (pair_name, pair) in &metrics.pid_pairs {
        if pair.outcome.abstained() {
            writer.append(&RunLogEvent::LabelObserved {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + *idx,
                name: format!("offline_vlda.pid.train_split.abstained.{pair_name}"),
                value: serde_json::to_value(&pair.outcome)?,
                metadata: BTreeMap::new(),
            })?;
            *idx += 1;
        }
    }
    writer.append(&RunLogEvent::LabelObserved {
        step: report.dims.samples as u64,
        timestamp_ns: timestamp_base_ns + *idx,
        name: "offline_vlda.pid.train_split.estimate_denominators".to_string(),
        value: serde_json::to_value(&metrics.estimate_denominators)?,
        metadata: BTreeMap::new(),
    })?;
    *idx += 1;
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
        // An abstained pair emits no metric events at all — no zero, no NaN.
        let Some(value) = value else { continue };
        writer.append(&RunLogEvent::PidMetric {
            step: report.dims.samples as u64,
            timestamp_ns: timestamp_base_ns + *idx,
            name,
            value,
            metadata: offline_vlda_pid_pair_metric_metadata(report, pair, metrics, scope.train_pid),
        })?;
        *idx += 1;
    }
    write_categorical_sx_component_metric_events(
        writer,
        report,
        metrics,
        scope,
        timestamp_base_ns,
        idx,
    )?;
    Ok(())
}

fn write_categorical_sx_component_metric_events<W: Write>(
    writer: &mut RunLogWriter<W>,
    report: &OfflineVldaReport,
    metrics: &OfflineVldaPidPairMetrics,
    scope: OfflineVldaPidMetricEventScope<'_>,
    timestamp_base_ns: u64,
    idx: &mut u64,
) -> Result<()> {
    let Some(components) = &metrics.categorical_sx_components else {
        return Ok(());
    };
    let source_1 = metrics.source_1.to_ascii_lowercase();
    let source_2 = metrics.source_2.to_ascii_lowercase();
    for (atom_name, atom) in [
        (
            format!("redundancy_{source_1}_{source_2}_action"),
            components.redundancy,
        ),
        (
            format!("unique_{source_1}_given_{source_2}_action"),
            components.unique_source_1,
        ),
        (
            format!("unique_{source_2}_given_{source_1}_action"),
            components.unique_source_2,
        ),
        (
            format!("synergy_{source_1}_{source_2}_action"),
            components.synergy,
        ),
    ] {
        for (component_name, value) in [
            ("informative", atom.informative),
            ("misinformative", atom.misinformative),
        ] {
            let mut metadata = offline_vlda_pid_pair_metric_metadata(
                report,
                &format!(
                    "{}{}",
                    metrics.source_1.to_ascii_uppercase(),
                    metrics.source_2.to_ascii_uppercase()
                ),
                metrics,
                scope.train_pid,
            );
            metadata.insert("sx_atom".to_string(), atom_name.clone());
            metadata.insert("sx_component".to_string(), component_name.to_string());
            writer.append(&RunLogEvent::PidMetric {
                step: report.dims.samples as u64,
                timestamp_ns: timestamp_base_ns + *idx,
                name: format!(
                    "{}.categorical_sx.{atom_name}.{component_name}",
                    scope.prefix
                ),
                value,
                metadata,
            })?;
            *idx += 1;
        }
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
        name: "offline_vlda.geometry.diagnostics_clear".to_string(),
        value: if report.geometry.diagnostics.status == "clear" {
            1.0
        } else {
            0.0
        },
        metadata: [
            ("category".to_string(), "geometry_diagnostics".to_string()),
            ("space".to_string(), report.geometry.space.clone()),
            ("metric".to_string(), report.geometry.metric.clone()),
            (
                "warnings".to_string(),
                report.geometry.diagnostics.warnings.len().to_string(),
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

fn exact_artifact_uri(path: &Path, description: &str) -> Result<String> {
    path.to_str()
        .map(str::to_string)
        .with_context(|| format!("{description} must be valid UTF-8 for run-log provenance"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pid_runlog::{read_events_from_path, summarize_events, validate_events};
    use std::collections::HashSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn continuous_options() -> OfflineVldaHarnessOptions {
        OfflineVldaHarnessOptions {
            pid_mode: PidMode::Continuous,
            ..OfflineVldaHarnessOptions::default()
        }
    }

    #[test]
    fn row_identity_matches_floating_point_zero_semantics() {
        let positive = [0.0, 1.0];
        let negative = [-0.0, 1.0];
        assert_eq!(RowBits(&positive), RowBits(&negative));

        let mut rows = HashSet::new();
        rows.insert(RowBits(&positive));
        rows.insert(RowBits(&negative));
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn estimator_abstentions_are_classified_by_typed_error_variant() {
        assert_eq!(
            abstain_reason_for_error(&PidError::SourceDimensionMismatch {
                context: "test",
                left_cols: 1,
                right_cols: 2,
            }),
            Some(OfflineVldaAbstainReason::EstimatorRequiresEqualSourceDimensions)
        );
        assert_eq!(
            abstain_reason_for_error(&PidError::ObservedContinuousSampleIncompatibility {
                context: "test",
                input_index: 0,
                coordinate: Some(0),
                unique_values: 1,
                n_samples: 2,
                max_multiplicity: 2,
            }),
            Some(OfflineVldaAbstainReason::ObservedSampleIncompatibleExactTies)
        );
        assert_eq!(
            abstain_reason_for_error(&PidError::AmbiguousKthNeighborShell {
                context: "test",
                query_index: 0,
                k: 1,
                radius: 1.0,
                interior_count: 0,
                boundary_count: 2,
            }),
            Some(OfflineVldaAbstainReason::AmbiguousNeighborShell)
        );
        assert_eq!(
            abstain_reason_for_error(&PidError::NumericalInstability {
                context: "shell ties"
            }),
            None,
            "display wording must never turn an unreviewed error variant into abstention"
        );
    }

    #[test]
    fn publication_rejects_status_value_contradiction_before_creating_output() {
        let mut report = run_offline_vlda_harness_with_options(
            fixture_dataset(),
            None,
            None,
            &continuous_options(),
        )
        .unwrap();
        assert!(report.metrics.mi_l_action.outcome.abstained());
        assert!(report.metrics.mi_l_action.value.is_none());
        report.metrics.mi_l_action.value = Some(0.0);

        let dir = tempfile::tempdir().unwrap();
        let summary_path = dir.path().join("contradictory-summary.json");
        let error = write_offline_vlda_summary(&summary_path, &report).unwrap_err();
        assert!(error
            .to_string()
            .contains("inconsistent with numeric-value"));
        assert!(!summary_path.exists());
    }

    #[test]
    fn analysis_rejects_incomplete_or_malformed_input_provenance() {
        let error = run_offline_vlda_harness(
            fixture_dataset(),
            Some("memory://fixture.json".to_string()),
            None,
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("input URI and exact-byte SHA-256 must be supplied together"));

        let error = run_offline_vlda_harness(
            fixture_dataset(),
            Some("memory://fixture.json".to_string()),
            Some("not-a-sha256".to_string()),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("input SHA-256 must be 64 lowercase hexadecimal characters"));
    }

    #[test]
    fn publication_rejects_structurally_valid_metric_mutation_after_analysis() {
        let mut report = run_offline_vlda_harness(fixture_dataset(), None, None).unwrap();
        let original = report
            .metrics
            .success_rate
            .expect("fixture carries complete success labels");
        report.metrics.success_rate = Some(if original < 0.75 { 0.75 } else { 0.25 });

        let directory = tempfile::tempdir().unwrap();
        let summary_path = directory.path().join("mutated-analysis-summary.json");
        let error = write_offline_vlda_summary(&summary_path, &report).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("changed after analysis and cannot be published"),
            "unexpected error: {error:#}"
        );
        assert!(!summary_path.exists());
    }

    #[test]
    fn deserialized_report_is_read_only_evidence_not_publication_authority() {
        let report = run_offline_vlda_harness(fixture_dataset(), None, None).unwrap();
        let serialized = serde_json::to_vec(&report).unwrap();
        let decoded: OfflineVldaReport = serde_json::from_slice(&serialized).unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), serialized);
        assert_eq!(decoded, report);

        let directory = tempfile::tempdir().unwrap();
        let summary_path = directory.path().join("republished-summary.json");
        let error = write_offline_vlda_summary(&summary_path, &decoded).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("lacks its in-process analysis seal"),
            "unexpected error: {error:#}"
        );
        assert!(!summary_path.exists());
    }

    #[test]
    fn publication_rejects_pid_mode_that_contradicts_the_metric_family() {
        let mut report = run_offline_vlda_harness_with_options(
            fixture_dataset(),
            None,
            None,
            &continuous_options(),
        )
        .unwrap();
        report.config["metric_pipeline"]["pid_mode"] = json!(PidMode::Disabled);
        report.config_hash = pid_runlog::canonical_json_hash_v2(&report.config).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let summary_path = dir.path().join("forged-pid-mode-summary.json");
        let error = write_offline_vlda_summary(&summary_path, &report).unwrap_err();

        assert!(
            format!("{error:#}").contains("metric-pipeline configuration does not reconstruct")
                || format!("{error:#}").contains("disabled PID mode carries"),
            "unexpected error: {error:#}"
        );
        assert!(!summary_path.exists());
    }

    #[test]
    fn runlog_publication_reconstructs_recorded_resource_usage() {
        let dataset = fixture_dataset();
        let mut report = run_offline_vlda_harness(dataset.clone(), None, None).unwrap();
        let recorded = report.config["resource_usage"]["total_axis_scalars"]
            .as_u64()
            .unwrap();
        report.config["resource_usage"]["total_axis_scalars"] = json!(recorded - 1);
        report.config_hash = pid_runlog::canonical_json_hash_v2(&report.config).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let runlog_path = directory.path().join("forged-resource-usage.jsonl");

        let error =
            write_offline_vlda_runlog(&runlog_path, None, None, &dataset, &report).unwrap_err();

        assert!(error
            .to_string()
            .contains("axis-scalar usage contradicts its dimensions"));
        assert!(!runlog_path.exists());
    }

    #[test]
    fn runlog_publication_requires_the_exact_summary_bytes() {
        let dataset = fixture_dataset();
        let report = run_offline_vlda_harness(dataset.clone(), None, None).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let summary_path = directory.path().join("summary-with-trailing-space.json");
        let runlog_path = directory.path().join("summary-with-trailing-space.jsonl");
        write_offline_vlda_summary(&summary_path, &report).unwrap();
        let mut bytes = std::fs::read(&summary_path).unwrap();
        bytes.push(b' ');
        std::fs::write(&summary_path, bytes).unwrap();

        let error =
            write_offline_vlda_runlog(&runlog_path, Some(&summary_path), None, &dataset, &report)
                .unwrap_err();

        assert!(error
            .to_string()
            .contains("not the exact JSON serialization"));
        assert!(!runlog_path.exists());
    }

    #[test]
    fn runlog_publication_reconstructs_axis_provenance() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            sample
                .metadata
                .insert("v_provenance".to_string(), "token_slice:vision".to_string());
            sample.metadata.insert(
                "l_provenance".to_string(),
                "token_slice:language".to_string(),
            );
            sample
                .metadata
                .insert("d_provenance".to_string(), "token_slice:state".to_string());
            sample
                .metadata
                .insert("a_provenance".to_string(), "action_vector".to_string());
        }
        let mut report = run_offline_vlda_harness(dataset.clone(), None, None).unwrap();
        assert_eq!(report.axis_provenance.len(), 4);
        report.axis_provenance.clear();
        report.analysis_seal =
            OfflineVldaAnalysisSeal(Some(offline_vlda_report_analysis_seal(&report).unwrap()));
        let directory = tempfile::tempdir().unwrap();
        let runlog_path = directory.path().join("forged-axis-provenance.jsonl");

        let error =
            write_offline_vlda_runlog(&runlog_path, None, None, &dataset, &report).unwrap_err();

        assert!(error
            .to_string()
            .contains("axis provenance does not reconstruct"));
        assert!(!runlog_path.exists());
    }

    #[test]
    fn in_memory_resource_limits_require_every_positive_ceiling() {
        let limits = OfflineVldaResourceLimits {
            max_metadata_json_depth: 0,
            ..OfflineVldaResourceLimits::default()
        };

        let error = admit_dataset_resources(&fixture_dataset(), None, None, &limits).unwrap_err();

        assert!(error.to_string().contains("max_metadata_json_depth"));
        assert!(error.to_string().contains("greater than zero"));
    }

    #[test]
    fn legacy_outcome_statuses_deserialize_as_computation_statuses_with_blocked_gates() {
        for (legacy_status, expected) in [
            ("eligible", OfflineVldaEstimateStatus::Produced),
            (
                "eligible_with_warning",
                OfflineVldaEstimateStatus::ProducedWithWarning,
            ),
        ] {
            let outcome: OfflineVldaOutcome = serde_json::from_value(json!({
                "status": legacy_status,
                "measure": "legacy_measure",
                "estimator_revision": "legacy_revision",
                "axes": ["V", "A"],
                "axis_diagnostics": []
            }))
            .unwrap();

            assert_eq!(outcome.status, expected);
            assert_eq!(
                outcome.scientific_gates.population,
                OfflineVldaScientificGateVerdict::NotEvaluated
            );
            assert_eq!(
                outcome.scientific_gates.application,
                OfflineVldaScientificGateVerdict::Blocked
            );
            assert!(!outcome.scientific_gates.interpretation_allowed);
            assert_eq!(
                outcome.scientific_gates.reason_code.as_deref(),
                Some("legacy_artifact_scientific_gates_unrecorded")
            );
        }
    }

    #[test]
    fn legacy_denominators_deserialize_support_eligible_alias() {
        let denominators: OfflineVldaEstimateDenominators = serde_json::from_value(json!({
            "requested": 6,
            "support_eligible": 4,
            "preflight_passed": 3,
            "estimated": 3,
            "warned": 1,
            "abstained": 3,
            "abstained_by_reason": {"ambiguous_neighbor_shell": 3}
        }))
        .unwrap();

        assert_eq!(denominators.declared_support_compatible, 4);
    }

    #[test]
    fn legacy_uncertainty_pair_deserializes_status_and_conservative_gates() {
        let pair: OfflineVldaPairUncertainty = serde_json::from_value(json!({
            "pair": "VL",
            "status": "eligible",
            "redundancy": null,
            "unique_s1": null,
            "unique_s2": null,
            "synergy": null,
            "unique_s1_perm_p": null,
            "unique_s2_perm_p": null,
            "perm_n_valid_s1": 0,
            "perm_n_valid_s2": 0
        }))
        .unwrap();

        assert_eq!(pair.status, OfflineVldaEstimateStatus::Produced);
        assert_eq!(
            pair.scientific_gates.application,
            OfflineVldaScientificGateVerdict::Blocked
        );
        assert!(!pair.scientific_gates.interpretation_allowed);
    }

    #[test]
    fn not_requested_outcome_is_excluded_from_estimate_denominators() {
        let mut denominators = OfflineVldaEstimateDenominators::default();
        denominators.record(&not_requested_outcome(&["V", "A"]));

        assert_eq!(denominators, OfflineVldaEstimateDenominators::default());
    }

    #[test]
    fn resource_limits_accept_every_observed_value_at_the_exact_limit() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Disabled,
            ..OfflineVldaHarnessOptions::default()
        };
        let observed = admit_dataset_resources(
            &dataset,
            Some(&options),
            None,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap();
        let limits = OfflineVldaResourceLimits {
            max_input_bytes: OFFLINE_DEFAULT_MAX_INPUT_BYTES,
            max_samples: observed.samples,
            max_total_axis_scalars: observed.total_axis_scalars,
            max_total_metadata_entries: observed.total_metadata_entries,
            max_total_metadata_json_nodes: observed.total_metadata_json_nodes,
            max_total_metadata_utf8_bytes: observed.total_metadata_utf8_bytes,
            max_metadata_json_depth: observed.metadata_json_depth,
            max_pairwise_distance_evaluations: observed
                .projected_total_pairwise_distance_evaluations,
            max_distance_coordinate_evaluations: observed
                .projected_total_distance_coordinate_evaluations,
            max_dense_solver_operations: observed.projected_dense_solver_operations.max(1),
            max_categorical_pid_operations: observed.projected_categorical_pid_operations.max(1),
        };

        let report = run_offline_vlda_harness_with_options_and_limits(
            dataset, None, None, &options, &limits,
        )
        .unwrap();

        assert_eq!(
            report.config["resource_limits"],
            serde_json::to_value(&limits).unwrap()
        );
        assert_eq!(
            report.config["resource_usage"],
            serde_json::to_value(&observed).unwrap()
        );
        assert_eq!(
            report.config["resource_accounting"]["pairwise_limit_scope"],
            "single_main_analysis_call"
        );
        assert_eq!(
            observed.projected_main_pairwise_distance_evaluations,
            21_616
        );
        assert_eq!(
            observed.projected_uncertainty_pairwise_distance_evaluations,
            0
        );
        assert_eq!(
            observed.projected_total_pairwise_distance_evaluations,
            21_616
        );
        assert_eq!(
            observed.projected_main_distance_coordinate_evaluations,
            108_080
        );
        assert_eq!(
            observed.projected_uncertainty_distance_coordinate_evaluations,
            0
        );
        assert_eq!(
            observed.projected_total_distance_coordinate_evaluations,
            108_080
        );
        assert_eq!(observed.projected_dense_solver_operations, 136_800);
    }

    #[test]
    fn dense_solver_projection_matches_pinned_pid_core_resource_contract() {
        let dataset = fixture_dataset();
        let dims = validate_dataset(&dataset).unwrap();
        let prepared = prepare_standardized_embeddings(&dataset.samples, &dims).unwrap();
        let x = prepared.v.as_ref();
        let y = prepared.a.as_ref();
        let components = 1usize;

        assert_eq!(
            projected_pls_fit_operations(
                x.nrows() as u128,
                x.ncols() as u128,
                y.ncols() as u128,
                components as u128,
            )
            .unwrap(),
            PlsProjector::fit_resource_estimate(x, y, components)
                .unwrap()
                .operations_hint
        );
        assert_eq!(
            projected_pls_cv_operations(
                x.nrows() as u128,
                x.ncols() as u128,
                y.ncols() as u128,
                components as u128,
            )
            .unwrap(),
            pid_core::experimental::pipelines::pls_cv_select_components_resource_estimate(
                x, y, components,
            )
            .unwrap()
            .operations_hint
        );

        let projector = PlsProjector::fit(x, y, components).unwrap();
        assert_eq!(
            projected_pls_transform_operations(
                x.nrows() as u128,
                x.ncols() as u128,
                components as u128,
            )
            .unwrap(),
            projector
                .transform_resource_estimate(x.nrows())
                .unwrap()
                .operations_hint
        );

        let logistic = LogisticRegressionConfig::default();
        assert_eq!(
            projected_logistic_operations(x.nrows() as u128, x.ncols() as u128).unwrap(),
            LogisticRegression::fit_resource_estimate(x, &logistic)
                .unwrap()
                .operations_hint
        );
    }

    #[test]
    fn categorical_pid_projection_matches_pinned_pid_core_resource_contract() {
        let dataset = fixture_dataset();
        let dims = validate_dataset(&dataset).unwrap();
        let prepared = prepare_standardized_embeddings(&dataset.samples, &dims).unwrap();
        let budget = ResourceBudget::default();
        let v = prepare_quantized_axis(prepared.v.as_ref(), 6, budget).unwrap();
        let l = prepare_quantized_axis(prepared.l.as_ref(), 6, budget).unwrap();
        let a = prepare_quantized_axis(prepared.a.as_ref(), 6, budget).unwrap();

        assert_eq!(
            projected_categorical_sxpid2_operations(
                dims.samples as u128,
                dims.v as u128,
                dims.l as u128,
                dims.a as u128,
            )
            .unwrap(),
            fitted_quantized_sxpid2_resource_estimate(&v.data, &l.data, &a.data)
                .unwrap()
                .operations_hint
        );
    }

    #[test]
    fn categorical_pid_projection_uses_pls_output_widths() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            sample.metadata.remove("split");
        }
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::CategoricalSxPls,
            categorical_bins: 6,
            pls: PlsComponentSelection::Fixed(1),
        };
        let dims = validate_dataset(&dataset).unwrap();
        let samples = dims.samples as u128;
        let projected = projected_categorical_pid_operations(&dataset, &options).unwrap();
        let one_screen = projected_categorical_pid_screen_operations(
            samples,
            [1, 1, 1, dims.a as u128],
            options.categorical_bins as u128,
        )
        .unwrap();

        assert_eq!(projected, 2 * one_screen);
        assert!(
            projected
                < 2 * projected_categorical_pid_screen_operations(
                    samples,
                    [dims.v, dims.l, dims.d, dims.a].map(|dimension| dimension as u128),
                    options.categorical_bins as u128,
                )
                .unwrap()
        );
    }

    #[test]
    fn categorical_pid_aggregate_limit_rejects_before_analysis() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::CategoricalSx,
            categorical_bins: 6,
            pls: PlsComponentSelection::Fixed(2),
        };
        let projected = projected_categorical_pid_operations(&dataset, &options).unwrap();
        assert!(projected > 1);
        let limits = OfflineVldaResourceLimits {
            max_categorical_pid_operations: u64::try_from(projected - 1).unwrap(),
            ..OfflineVldaResourceLimits::default()
        };

        let error = run_offline_vlda_harness_with_options_and_limits(
            dataset, None, None, &options, &limits,
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("aggregate categorical-PID operations"));
    }

    #[test]
    fn aggregate_dense_solver_limit_rejects_before_analysis() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Disabled,
            ..OfflineVldaHarnessOptions::default()
        };
        let projected = projected_dense_solver_operations(&dataset, &options).unwrap();
        assert!(
            projected > 0,
            "fixture must exercise held-out logistic work"
        );
        let limits = OfflineVldaResourceLimits {
            max_dense_solver_operations: u64::try_from(projected - 1).unwrap(),
            ..OfflineVldaResourceLimits::default()
        };

        let error = run_offline_vlda_harness_with_options_and_limits(
            dataset, None, None, &options, &limits,
        )
        .unwrap_err();

        assert!(error.to_string().contains("dense-solver operations"));
        assert!(error.to_string().contains(&format!(
            "observed {projected}, limit {}",
            limits.max_dense_solver_operations
        )));
    }

    #[test]
    fn default_resource_limits_admit_routine_fixtures_and_reject_stress_work() {
        for (name, encoded) in [
            (
                "offline_vlda_fixture.json",
                include_str!("../fixtures/offline_vlda_fixture.json"),
            ),
            (
                "offline_vlda_continuous_fixture.json",
                include_str!("../fixtures/offline_vlda_continuous_fixture.json"),
            ),
        ] {
            let dataset: OfflineVldaDataset = serde_json::from_str(encoded).unwrap();
            let options = OfflineVldaHarnessOptions {
                pid_mode: PidMode::Continuous,
                ..OfflineVldaHarnessOptions::default()
            };
            admit_dataset_resources(
                &dataset,
                Some(&options),
                None,
                &OfflineVldaResourceLimits::default(),
            )
            .unwrap_or_else(|error| panic!("default limits rejected {name}: {error:#}"));
        }

        let stress: OfflineVldaDataset = serde_json::from_str(include_str!(
            "../fixtures/offline_vlda_highdim_fixture.json"
        ))
        .unwrap();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Continuous,
            ..OfflineVldaHarnessOptions::default()
        };
        let error = admit_dataset_resources(
            &stress,
            Some(&options),
            None,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("dense-solver operations"));

        let stress_limits: OfflineVldaResourceLimits =
            serde_json::from_str(include_str!("../fixtures/offline_vlda_highdim_limits.json"))
                .unwrap();
        admit_dataset_resources(&stress, Some(&options), None, &stress_limits).unwrap();
    }

    #[test]
    fn in_memory_entry_point_rejects_one_sample_over_before_shape_validation() {
        let template = fixture_dataset().samples[0].clone();
        let dataset = OfflineVldaDataset {
            samples: vec![template; OFFLINE_DEFAULT_MAX_SAMPLES + 1],
            ..fixture_dataset()
        };
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Disabled,
            ..OfflineVldaHarnessOptions::default()
        };

        let error =
            run_offline_vlda_harness_with_options(dataset, None, None, &options).unwrap_err();

        assert!(error.to_string().contains("samples"));
        assert!(error.to_string().contains("observed 1025, limit 1024"));
        assert!(!error
            .to_string()
            .contains("sample_id values must be unique"));
    }

    #[test]
    fn file_entry_point_rejects_one_sample_over_custom_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dataset.json");
        std::fs::write(&path, serde_json::to_vec(&fixture_dataset()).unwrap()).unwrap();
        let limits = OfflineVldaResourceLimits {
            max_samples: 15,
            ..OfflineVldaResourceLimits::default()
        };

        let error = read_offline_vlda_dataset_with_limits(&path, &limits).unwrap_err();

        assert!(error.to_string().contains("observed 16, limit 15"));
    }

    #[test]
    fn file_entry_point_applies_the_typed_raw_input_byte_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dataset.json");
        let encoded = serde_json::to_vec(&fixture_dataset()).unwrap();
        std::fs::write(&path, &encoded).unwrap();
        let limits = OfflineVldaResourceLimits {
            max_input_bytes: u64::try_from(encoded.len() - 1).unwrap(),
            ..OfflineVldaResourceLimits::default()
        };

        let error = read_offline_vlda_dataset_with_limits(&path, &limits).unwrap_err();

        assert!(error.to_string().contains("offline VLDA input"));
        assert!(error.to_string().contains("exceeds"));
        assert!(!error.to_string().contains("failed to parse"));
    }

    #[test]
    fn file_entry_point_rejects_unknown_dataset_and_sample_fields() {
        let dir = tempfile::tempdir().unwrap();
        for (name, pointer) in [
            ("unknown-dataset.json", ""),
            ("unknown-sample.json", "/samples/0"),
        ] {
            let mut value = serde_json::to_value(fixture_dataset()).unwrap();
            value
                .pointer_mut(pointer)
                .and_then(Value::as_object_mut)
                .unwrap()
                .insert("misspelled_contract_field".to_string(), json!(true));
            let path = dir.path().join(name);
            std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();

            let error = read_offline_vlda_dataset(&path).unwrap_err();

            assert!(
                format!("{error:#}").contains("unknown field `misspelled_contract_field`"),
                "unexpected error for {name}: {error:#}"
            );
        }
    }

    #[test]
    fn file_entry_point_rejects_duplicate_contract_map_keys() {
        let dir = tempfile::tempdir().unwrap();
        let encoded = serde_json::to_string(&fixture_dataset()).unwrap();
        let cases = [
            (
                "duplicate-support.json",
                encoded.replacen("\"support\":{", "\"support\":{\"v\":\"categorical\",", 1),
            ),
            (
                "duplicate-label.json",
                encoded.replacen(
                    "\"labels\":{\"success\":false}",
                    "\"labels\":{\"success\":true,\"success\":false}",
                    1,
                ),
            ),
            (
                "duplicate-metadata.json",
                encoded.replacen(
                    "\"metadata\":{\"split\":\"train\"}",
                    "\"metadata\":{\"split\":\"heldout\",\"split\":\"train\"}",
                    1,
                ),
            ),
            (
                "duplicate-nested-label-object.json",
                encoded.replacen(
                    "\"labels\":{\"success\":false}",
                    "\"labels\":{\"success\":false,\"nested\":{\"key\":1,\"key\":2}}",
                    1,
                ),
            ),
        ];
        for (name, duplicate) in cases {
            assert_ne!(
                duplicate, encoded,
                "test fixture replacement failed for {name}"
            );
            let path = dir.path().join(name);
            std::fs::write(&path, duplicate).unwrap();

            let error = read_offline_vlda_dataset(&path).unwrap_err();

            assert!(
                format!("{error:#}").contains("duplicate JSON object key"),
                "unexpected error for {name}: {error:#}"
            );
        }
    }

    #[test]
    fn resource_preflight_rejects_unknown_support_axes_and_empty_episode_ids() {
        let mut unknown_support = fixture_dataset();
        unknown_support.support.insert(
            "action".to_string(),
            OfflineVldaDeclaredSupport::Categorical,
        );
        let error = admit_dataset_resources(
            &unknown_support,
            None,
            None,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown axis \"action\""));

        let mut empty_episode = fixture_dataset();
        empty_episode.samples[0].episode_id = Some(String::new());
        let error = run_offline_vlda_harness_with_options(
            empty_episode,
            None,
            None,
            &OfflineVldaHarnessOptions {
                pid_mode: PidMode::Disabled,
                ..OfflineVldaHarnessOptions::default()
            },
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("episode_id must not be empty when present"));
    }

    #[test]
    fn decoded_axis_and_metadata_limits_reject_one_over() {
        let dataset = fixture_dataset();
        let observed =
            admit_dataset_resources(&dataset, None, None, &OfflineVldaResourceLimits::default())
                .unwrap();
        for (resource, limits) in [
            (
                "axis scalars",
                OfflineVldaResourceLimits {
                    max_total_axis_scalars: observed.total_axis_scalars - 1,
                    ..OfflineVldaResourceLimits::default()
                },
            ),
            (
                "metadata entries",
                OfflineVldaResourceLimits {
                    max_total_metadata_entries: observed.total_metadata_entries - 1,
                    ..OfflineVldaResourceLimits::default()
                },
            ),
            (
                "metadata JSON nodes",
                OfflineVldaResourceLimits {
                    max_total_metadata_json_nodes: observed.total_metadata_json_nodes - 1,
                    ..OfflineVldaResourceLimits::default()
                },
            ),
            (
                "metadata UTF-8 bytes",
                OfflineVldaResourceLimits {
                    max_total_metadata_utf8_bytes: observed.total_metadata_utf8_bytes - 1,
                    ..OfflineVldaResourceLimits::default()
                },
            ),
        ] {
            let error = admit_dataset_resources(&dataset, None, None, &limits).unwrap_err();
            assert!(
                error.to_string().contains(resource),
                "expected {resource} rejection, got {error:#}"
            );
        }
    }

    #[test]
    fn metadata_json_depth_limit_is_iterative_and_fail_closed() {
        let mut dataset = fixture_dataset();
        dataset.samples[0].labels.insert(
            "nested".to_string(),
            json!({"level_2": [{"level_4": "value"}]}),
        );
        let limits = OfflineVldaResourceLimits {
            max_metadata_json_depth: 3,
            ..OfflineVldaResourceLimits::default()
        };

        let error = admit_dataset_resources(&dataset, None, None, &limits).unwrap_err();

        assert!(error
            .to_string()
            .contains("metadata JSON depth: observed 4, limit 3"));
    }

    #[test]
    fn pid_none_still_rejects_projected_pairwise_work_one_over() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Disabled,
            ..OfflineVldaHarnessOptions::default()
        };
        let observed = projected_analysis_distance_evaluations(&dataset, options.pid_mode).unwrap();
        let limits = OfflineVldaResourceLimits {
            max_pairwise_distance_evaluations: u64::try_from(observed - 1).unwrap(),
            ..OfflineVldaResourceLimits::default()
        };

        let error = run_offline_vlda_harness_with_options_and_limits(
            dataset, None, None, &options, &limits,
        )
        .unwrap_err();

        assert!(error.to_string().contains(&format!(
            "observed {observed}, limit {}",
            limits.max_pairwise_distance_evaluations
        )));
    }

    #[test]
    fn distance_coordinate_cap_rejects_high_dimensional_work_one_over() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Disabled,
            ..OfflineVldaHarnessOptions::default()
        };
        let observed = admit_dataset_resources(
            &dataset,
            Some(&options),
            None,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap();
        let limit = observed
            .projected_total_distance_coordinate_evaluations
            .checked_sub(1)
            .unwrap();
        let limits = OfflineVldaResourceLimits {
            max_distance_coordinate_evaluations: limit,
            ..OfflineVldaResourceLimits::default()
        };

        let error = run_offline_vlda_harness_with_options_and_limits(
            dataset, None, None, &options, &limits,
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("projected distance coordinate evaluations"));
        assert!(error.to_string().contains(&format!(
            "observed {}, limit {limit}",
            observed.projected_total_distance_coordinate_evaluations
        )));
    }

    #[test]
    fn uncertainty_entry_point_applies_pairwise_limit_independently() {
        let dataset = as_single_ordered_episode(fixture_dataset());
        let config = OfflineVldaUncertaintyConfig {
            n_perm: 1,
            ..OfflineVldaUncertaintyConfig::default()
        };
        let projected =
            projected_uncertainty_distance_evaluations(dataset.samples.len(), &config).unwrap();
        let limits = OfflineVldaResourceLimits {
            max_pairwise_distance_evaluations: u64::try_from(projected - 1).unwrap(),
            ..OfflineVldaResourceLimits::default()
        };

        let error = compute_offline_pid_uncertainty_with_limits(
            &dataset,
            PidMode::Continuous,
            &config,
            &limits,
        )
        .unwrap_err();

        assert!(error.to_string().contains(&format!(
            "observed {projected}, limit {}",
            limits.max_pairwise_distance_evaluations
        )));
    }

    #[test]
    fn pid_projection_matches_pid_core_pairwise_budget_units() {
        let dataset = fixture_dataset();
        let dims = validate_dataset(&dataset).unwrap();
        let prepared = prepare_standardized_embeddings(&dataset.samples, &dims).unwrap();
        let pairs = unordered_pair_count(dataset.samples.len() as u128, "test pairs").unwrap();
        let ksg = ksg_config();
        let pid_cfg = pid2_config(&ksg);

        let ksg_estimate = pid_core::stable::continuous::ksg_resource_estimate(
            prepared.v.as_ref(),
            prepared.a.as_ref(),
        )
        .unwrap();
        let pid2_estimate = pid2_resource_estimate(
            prepared.v.as_ref(),
            prepared.v.as_ref(),
            prepared.a.as_ref(),
            &pid_cfg,
        )
        .unwrap();

        assert_eq!(
            ksg_estimate.pairwise_distances,
            OFFLINE_KSG_PAIRWISE_PASSES * pairs
        );
        assert_eq!(
            pid2_estimate.pairwise_distances,
            OFFLINE_PID2_PAIRWISE_PASSES * pairs
        );
    }

    #[test]
    fn aggregate_invocation_checked_adds_main_and_uncertainty_before_analysis() {
        let dataset = as_single_ordered_episode(fixture_dataset());
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Continuous,
            ..OfflineVldaHarnessOptions::default()
        };
        let uncertainty = OfflineVldaUncertaintyConfig {
            n_perm: 1,
            ..OfflineVldaUncertaintyConfig::default()
        };
        let main = projected_analysis_distance_evaluations(&dataset, options.pid_mode).unwrap();
        let optional =
            projected_uncertainty_distance_evaluations(dataset.samples.len(), &uncertainty)
                .unwrap();
        let individually_sufficient = main.max(optional);
        let aggregate = checked_work_add(main, optional, "test aggregate").unwrap();
        assert!(aggregate > individually_sufficient);
        let limits = OfflineVldaResourceLimits {
            max_pairwise_distance_evaluations: u64::try_from(individually_sufficient).unwrap(),
            ..OfflineVldaResourceLimits::default()
        };

        let error = run_offline_vlda_harness_with_options_and_invocation_limits(
            dataset,
            None,
            None,
            &options,
            &uncertainty,
            &limits,
        )
        .unwrap_err();

        assert!(error.to_string().contains("aggregate invocation"));
        assert!(error.to_string().contains(&format!(
            "observed {aggregate}, limit {individually_sufficient}"
        )));
    }

    #[test]
    fn aggregate_invocation_binds_exact_main_uncertainty_and_total_usage() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Continuous,
            ..OfflineVldaHarnessOptions::default()
        };
        let uncertainty = OfflineVldaUncertaintyConfig {
            n_perm: 1,
            ..OfflineVldaUncertaintyConfig::default()
        };
        let expected = admit_dataset_resources(
            &dataset,
            Some(&options),
            Some(&uncertainty),
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap();
        let limits = OfflineVldaResourceLimits {
            max_pairwise_distance_evaluations: expected
                .projected_total_pairwise_distance_evaluations,
            ..OfflineVldaResourceLimits::default()
        };

        let report = run_offline_vlda_harness_with_options_and_invocation_limits(
            dataset,
            None,
            None,
            &options,
            &uncertainty,
            &limits,
        )
        .unwrap();

        assert_eq!(
            report.config["resource_usage"],
            serde_json::to_value(&expected).unwrap()
        );
        assert_eq!(
            report.config["resource_accounting"]["pairwise_limit_scope"],
            "aggregate_main_and_optional_uncertainty"
        );
    }

    #[test]
    fn aggregate_invocation_rejects_a_block_larger_than_the_half_sample_envelope() {
        let dataset = fixture_dataset();
        let samples = dataset.samples.len();
        let uncertainty = OfflineVldaUncertaintyConfig {
            n_boot: 2,
            block_size: samples / 2 + 1,
            ..OfflineVldaUncertaintyConfig::default()
        };

        let error = run_offline_vlda_harness_with_options_and_invocation_limits(
            dataset,
            None,
            None,
            &continuous_options(),
            &uncertainty,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("half-sample stability envelope"));
    }

    #[test]
    fn continuous_uncertainty_rejects_one_bootstrap_before_main_analysis() {
        let mut dataset = fixture_dataset();
        dataset.samples[1].sample_id = dataset.samples[0].sample_id.clone();
        let uncertainty = OfflineVldaUncertaintyConfig {
            n_boot: 1,
            ..OfflineVldaUncertaintyConfig::default()
        };

        let error = run_offline_vlda_harness_with_options_and_invocation_limits(
            dataset,
            None,
            None,
            &continuous_options(),
            &uncertainty,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("at least two bootstrap resamples"));
        assert!(!error
            .to_string()
            .contains("sample_id values must be unique"));
    }

    #[test]
    fn aggregate_invocation_rejects_an_impossible_circular_shift() {
        let dataset = fixture_dataset();
        let min_shift = dataset.samples.len() / 2;
        let uncertainty = OfflineVldaUncertaintyConfig {
            n_perm: 1,
            permutation_scheme: PermutationScheme::CircularShift { min_shift },
            ..OfflineVldaUncertaintyConfig::default()
        };

        let error = run_offline_vlda_harness_with_options_and_invocation_limits(
            dataset,
            None,
            None,
            &continuous_options(),
            &uncertainty,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("requires samples >= 2*min_shift+1"));
    }

    #[test]
    fn huge_uncertainty_count_rejects_before_main_dataset_validation() {
        let mut dataset = as_single_ordered_episode(fixture_dataset());
        dataset.samples[1].sample_id = dataset.samples[0].sample_id.clone();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::Continuous,
            ..OfflineVldaHarnessOptions::default()
        };
        let uncertainty = OfflineVldaUncertaintyConfig {
            n_perm: usize::MAX,
            ..OfflineVldaUncertaintyConfig::default()
        };

        let error = run_offline_vlda_harness_with_options_and_invocation_limits(
            dataset,
            None,
            None,
            &options,
            &uncertainty,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("aggregate invocation"));
        assert!(error
            .to_string()
            .contains("projected pairwise distance evaluations"));
        assert!(!error
            .to_string()
            .contains("sample_id values must be unique"));
    }

    #[test]
    fn synthetic_pairwise_projection_overflow_returns_error() {
        let error = projected_geometry_distance_evaluations(u128::MAX).unwrap_err();

        assert!(error.to_string().contains("resource projection overflow"));
    }

    #[test]
    fn temporal_report_distinguishes_persistent_from_alternating_series() {
        // One long episode; V is a slow ramp (lag-1 near +1), L alternates
        // sign every step (lag-1 near -1). Both are descriptive correlations.
        let n = 32usize;
        let samples: Vec<OfflineVldaSample> = (0..n)
            .map(|idx| {
                let ramp = idx as f64;
                let alt = if idx % 2 == 0 { 1.0 } else { -1.0 };
                OfflineVldaSample {
                    sample_id: format!("s{idx:03}"),
                    episode_id: Some("ep-0".to_string()),
                    v: vec![ramp],
                    l: vec![alt],
                    d: vec![ramp * 0.5 + alt],
                    a: vec![ramp * 0.25 + if idx % 3 == 0 { 0.5 } else { -0.5 }],
                    labels: [("success".to_string(), json!(idx % 2 == 0))]
                        .into_iter()
                        .collect(),
                    metadata: [("sequence_index".to_string(), idx.to_string())]
                        .into_iter()
                        .collect(),
                }
            })
            .collect();
        let dataset = OfflineVldaDataset {
            samples,
            ..fixture_dataset()
        };
        let report = run_offline_vlda_harness(dataset, None, None).unwrap();
        let t = &report.temporal;
        assert_eq!(t.scope, "within_episode");
        let v = &t.variables["V"];
        let l = &t.variables["L"];
        let v_r1 = v.lag1_autocorr.expect("ramp has lag pairs");
        let l_r1 = l.lag1_autocorr.expect("alternating series has lag pairs");
        assert!(v_r1 > 0.8, "ramp lag1 = {v_r1}");
        assert!(l_r1 < -0.8, "alternating lag1 = {l_r1}");
        assert_eq!(t.segments, 1);
        assert_eq!(t.potential_lag_pairs, n - 1);
        assert_eq!(t.lag_pairs, n - 1);
        assert_eq!(t.correlation_lag_pairs, n - 1);
        assert_eq!(t.sequence_index_gap_pairs, 0);
        assert_eq!(
            t.interpretation,
            "descriptive_within_unit_step_run_pearson_lag1_not_estimator_effective_sample_size_or_block_selector"
        );
        assert_eq!(
            t.ordering_basis,
            "strict_canonical_metadata_sequence_index_unit_steps_within_segments"
        );
        // The fixture's own report carries the diagnostic too.
        let base = run_offline_vlda_harness(fixture_dataset(), None, None).unwrap();
        assert_eq!(base.temporal.variables.len(), 4);
        assert_eq!(
            base.config["report_schema"],
            json!(OFFLINE_VLDA_REPORT_SCHEMA)
        );
    }

    #[test]
    fn report_validation_rejects_an_unversioned_summary() {
        let mut report = run_offline_vlda_harness(fixture_dataset(), None, None).unwrap();
        report
            .config
            .as_object_mut()
            .expect("report configuration is an object")
            .remove("report_schema");
        report.config_hash = pid_runlog::canonical_json_hash_v2(&report.config).unwrap();

        let error = validate_offline_vlda_report(&report).unwrap_err();

        assert!(
            error.to_string().contains("unsupported report schema"),
            "{error:#}"
        );
    }

    #[test]
    fn report_validation_checks_temporal_coverage() {
        let n = 24usize;
        let samples = (0..n)
            .map(|idx| OfflineVldaSample {
                sample_id: format!("temporal-validation-{idx}"),
                episode_id: Some("episode".to_string()),
                v: vec![idx as f64, 0.0],
                l: vec![if idx % 2 == 0 { -1.0 } else { 1.0 }],
                d: vec![(idx % 3) as f64],
                a: vec![(idx % 5) as f64],
                labels: BTreeMap::new(),
                metadata: [("sequence_index".to_string(), idx.to_string())]
                    .into_iter()
                    .collect(),
            })
            .collect();
        let dataset = OfflineVldaDataset {
            samples,
            ..fixture_dataset()
        };
        let report = run_offline_vlda_harness(dataset, None, None).unwrap();
        assert_eq!(report.temporal.variables["V"].dimensions_total, 2);
        assert_eq!(
            report.temporal.variables["V"].dimensions_with_defined_lag1,
            1
        );
        validate_offline_vlda_report(&report).unwrap();

        let mut forged_coverage = report.clone();
        forged_coverage
            .temporal
            .variables
            .get_mut("V")
            .unwrap()
            .dimensions_with_defined_lag1 = 0;
        assert!(validate_offline_vlda_report(&forged_coverage)
            .unwrap_err()
            .to_string()
            .contains("lag-1 presence contradicts"));

        let mut forged_order = report;
        forged_order.temporal.ordering_basis =
            "missing_or_nonmonotone_metadata_sequence_index_no_lag_pairs".to_string();
        assert!(validate_offline_vlda_report(&forged_order)
            .unwrap_err()
            .to_string()
            .contains("admitted lag-pair count contradicts"));

        let mut forged_minimum = run_offline_vlda_harness(
            OfflineVldaDataset {
                samples: (0..24)
                    .map(|idx| OfflineVldaSample {
                        sample_id: format!("forged-minimum-{idx}"),
                        episode_id: Some("episode".to_string()),
                        v: vec![idx as f64],
                        l: vec![idx as f64],
                        d: vec![idx as f64],
                        a: vec![idx as f64],
                        labels: BTreeMap::new(),
                        metadata: [("sequence_index".to_string(), idx.to_string())]
                            .into_iter()
                            .collect(),
                    })
                    .collect(),
                ..fixture_dataset()
            },
            None,
            None,
        )
        .unwrap();
        forged_minimum.temporal.correlation_lag_pairs = 2;
        assert!(validate_offline_vlda_report(&forged_minimum)
            .unwrap_err()
            .to_string()
            .contains("minimum three-pair runs"));
    }

    #[test]
    fn lag1_does_not_treat_between_episode_offsets_as_temporal_dependence() {
        let matrix = MatOwned::new(vec![-1.0, -1.0, 1.0, 1.0], 4, 1).unwrap();

        assert_eq!(axis_lag1_autocorr(&matrix, &[0..2, 2..4]), (None, 0));
    }

    #[test]
    fn temporal_report_distinguishes_admitted_pairs_from_correlation_pairs() {
        let mut dataset = fixture_dataset();
        for (idx, sample) in dataset.samples.iter_mut().enumerate() {
            sample.episode_id = Some(format!("pair-{}", idx / 2));
            sample
                .metadata
                .insert("sequence_index".to_string(), (idx % 2).to_string());
        }

        let report = run_offline_vlda_harness(dataset, None, None).unwrap();

        assert_eq!(report.temporal.lag_pairs, report.dims.samples / 2);
        assert_eq!(report.temporal.correlation_lag_pairs, 0);
        assert!(report.temporal.variables.values().all(|variable| {
            variable.lag1_autocorr.is_none() && variable.dimensions_with_defined_lag1 == 0
        }));
        validate_offline_vlda_report(&report).unwrap();
    }

    #[test]
    fn lag1_pools_centered_residuals_without_crossing_segments() {
        let matrix = MatOwned::new(vec![1.0, 2.0, 3.0, 4.0, 10.0, 8.0, 6.0, 4.0], 8, 1).unwrap();

        let correlation = axis_lag1_autocorr(&matrix, &[0..4, 4..8]);

        assert!(correlation
            .0
            .is_some_and(|value| (value - 1.0).abs() < 1e-12));
        assert_eq!(correlation.1, 1);
    }

    #[test]
    fn lag1_scaling_ignores_runs_that_cannot_support_centering() {
        let matrix = MatOwned::new(vec![1.0, 2.0, 3.0, 4.0, 1.0e300, -1.0e300], 6, 1).unwrap();

        let (correlation, dimensions) = axis_lag1_autocorr(&matrix, &[0..4, 4..6]);
        assert!(correlation.is_some_and(|value| (value - 1.0).abs() < 1e-12));
        assert_eq!(dimensions, 1);
    }

    #[test]
    fn lag1_excludes_undefined_constant_dimensions_from_the_axis_mean() {
        let one_defined =
            MatOwned::new(vec![1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0, 0.0, 5.0, 0.0], 5, 2).unwrap();
        let all_undefined = MatOwned::new(vec![0.0; 10], 5, 2).unwrap();
        let segment = 0..5;
        let segments = std::slice::from_ref(&segment);

        let (defined_value, defined_count) = axis_lag1_autocorr(&one_defined, segments);
        assert!(defined_value.is_some_and(|value| (value - 1.0).abs() < 1e-12));
        assert_eq!(defined_count, 1);
        assert_eq!(axis_lag1_autocorr(&all_undefined, segments), (None, 0));
    }

    #[test]
    fn lag1_centers_each_side_within_one_unit_step_run() {
        // A ramp remains perfectly positive after centering each lagged side.
        let matrix = MatOwned::new(vec![1.0, 2.0, 3.0, 4.0], 4, 1).unwrap();
        let segment = 0..4;
        let segments = std::slice::from_ref(&segment);

        let (correlation, dimensions) = axis_lag1_autocorr(&matrix, segments);
        assert!(correlation.is_some_and(|value| (value - 1.0).abs() < 1e-12));
        assert_eq!(dimensions, 1);

        // This sequence has a positive uncentered cosine but a negative Pearson correlation.
        let reversed = MatOwned::new(vec![0.0, 2.0, 0.0, 3.0], 4, 1).unwrap();
        assert!(axis_lag1_autocorr(&reversed, segments)
            .0
            .is_some_and(|value| value < -0.8));
    }

    #[test]
    fn temporal_report_withholds_lag1_without_within_series_pairs() {
        let mut dataset = fixture_dataset();
        for (idx, sample) in dataset.samples.iter_mut().enumerate() {
            sample.episode_id = Some(format!("singleton-{idx}"));
        }

        let report = run_offline_vlda_harness(dataset, None, None).unwrap();

        assert_eq!(report.temporal.scope, "within_episode");
        assert_eq!(report.temporal.segments, report.dims.samples);
        assert_eq!(report.temporal.potential_lag_pairs, 0);
        assert_eq!(report.temporal.lag_pairs, 0);
        assert_eq!(report.temporal.correlation_lag_pairs, 0);
        assert_eq!(report.temporal.sequence_index_gap_pairs, 0);
        assert!(report.temporal.variables.values().all(|variable| {
            variable.lag1_autocorr.is_none() && variable.dimensions_with_defined_lag1 == 0
        }));
        validate_offline_vlda_report(&report).unwrap();

        let mut forged_coverage = report;
        forged_coverage
            .temporal
            .variables
            .get_mut("V")
            .unwrap()
            .dimensions_with_defined_lag1 = 1;
        assert!(validate_offline_vlda_report(&forged_coverage)
            .unwrap_err()
            .to_string()
            .contains("cannot define lag-1 columns without centered correlation pairs"));
    }

    #[test]
    fn temporal_report_does_not_infer_one_series_from_unlabeled_row_order() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            sample.episode_id = None;
        }

        let report = run_offline_vlda_harness(dataset, None, None).unwrap();

        assert_eq!(report.temporal.scope, "unidentified_without_episode_ids");
        assert_eq!(report.temporal.segments, 0);
        assert_eq!(report.temporal.potential_lag_pairs, 0);
        assert_eq!(report.temporal.lag_pairs, 0);
        assert_eq!(report.temporal.correlation_lag_pairs, 0);
        assert_eq!(report.temporal.sequence_index_gap_pairs, 0);
        assert!(report
            .temporal
            .variables
            .values()
            .all(|variable| variable.lag1_autocorr.is_none()));
        assert_eq!(
            report.temporal.ordering_basis,
            "episode_identity_absent_no_lag_pairs"
        );
    }

    #[test]
    fn temporal_report_does_not_infer_order_from_one_episode_id() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            sample.episode_id = Some("one-episode".to_string());
            sample.metadata.remove("sequence_index");
        }

        let report = run_offline_vlda_harness(dataset, None, None).unwrap();

        assert_eq!(report.temporal.potential_lag_pairs, report.dims.samples - 1);
        assert_eq!(report.temporal.lag_pairs, 0);
        assert_eq!(report.temporal.correlation_lag_pairs, 0);
        assert_eq!(report.temporal.sequence_index_gap_pairs, 0);
        assert!(report
            .temporal
            .variables
            .values()
            .all(|variable| variable.lag1_autocorr.is_none()));
        assert_eq!(
            report.temporal.ordering_basis,
            "missing_or_nonmonotone_metadata_sequence_index_no_lag_pairs"
        );
    }

    #[test]
    fn temporal_report_rejects_a_nonmonotone_sequence_receipt() {
        let mut dataset = fixture_dataset();
        for (idx, sample) in dataset.samples.iter_mut().enumerate() {
            sample.episode_id = Some("one-episode".to_string());
            let sequence = if idx == 8 { 7 } else { idx };
            sample
                .metadata
                .insert("sequence_index".to_string(), sequence.to_string());
        }

        let report = run_offline_vlda_harness(dataset, None, None).unwrap();

        assert_eq!(report.temporal.potential_lag_pairs, report.dims.samples - 1);
        assert_eq!(report.temporal.lag_pairs, 0);
        assert_eq!(report.temporal.correlation_lag_pairs, 0);
        assert_eq!(report.temporal.sequence_index_gap_pairs, 0);
        assert_eq!(
            report.temporal.ordering_basis,
            "missing_or_nonmonotone_metadata_sequence_index_no_lag_pairs"
        );
        assert!(report
            .temporal
            .variables
            .values()
            .all(|variable| variable.lag1_autocorr.is_none()));
    }

    #[test]
    fn temporal_report_never_bridges_missing_episode_ids() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples[2..] {
            sample.episode_id = None;
        }

        let report = run_offline_vlda_harness(dataset, None, None).unwrap();

        assert_eq!(
            report.temporal.scope,
            "known_episode_segments_only_mixed_ids"
        );
        assert_eq!(report.temporal.segments, report.dims.samples - 1);
        assert_eq!(report.temporal.potential_lag_pairs, 1);
        assert_eq!(report.temporal.lag_pairs, 0);
        assert_eq!(report.temporal.correlation_lag_pairs, 0);
        assert_eq!(report.temporal.sequence_index_gap_pairs, 0);
        assert!(report
            .temporal
            .variables
            .values()
            .all(|variable| variable.lag1_autocorr.is_none()));
    }

    #[test]
    fn temporal_report_accepts_per_episode_sequence_resets_and_records_gaps() {
        let mut dataset = fixture_dataset();
        for (idx, sample) in dataset.samples.iter_mut().enumerate() {
            let episode = idx / 8;
            let within_episode = idx % 8;
            sample.episode_id = Some(format!("episode-{episode}"));
            let sequence = if within_episode >= 4 {
                within_episode + 2
            } else {
                within_episode
            };
            sample
                .metadata
                .insert("sequence_index".to_string(), sequence.to_string());
        }

        let report = run_offline_vlda_harness(dataset, None, None).unwrap();

        assert_eq!(report.temporal.scope, "within_episode");
        assert_eq!(report.temporal.segments, report.dims.samples / 8);
        assert_eq!(
            report.temporal.lag_pairs,
            report.dims.samples - (2 * report.temporal.segments)
        );
        assert_eq!(
            report.temporal.sequence_index_gap_pairs,
            report.temporal.segments
        );
        assert_eq!(
            report.temporal.correlation_lag_pairs,
            report.temporal.lag_pairs
        );
        assert_eq!(
            report.temporal.ordering_basis,
            "strict_canonical_metadata_sequence_index_unit_steps_within_segments"
        );
        validate_offline_vlda_report(&report).unwrap();
    }

    #[test]
    fn centroid_standardization_handles_large_finite_common_scales() {
        let make_sample = |idx: usize, value: f64| OfflineVldaSample {
            sample_id: format!("large-{idx}"),
            episode_id: None,
            v: vec![value],
            l: vec![value],
            d: vec![value],
            a: vec![value],
            labels: BTreeMap::new(),
            metadata: BTreeMap::new(),
        };
        let samples = vec![
            make_sample(0, -f64::MAX),
            make_sample(1, f64::MAX),
            make_sample(2, -f64::MAX / 2.0),
            make_sample(3, f64::MAX / 2.0),
        ];
        let labels = [false, true, false, true];
        let roles = [
            OfflineVldaSplitRole::Train,
            OfflineVldaSplitRole::Train,
            OfflineVldaSplitRole::Heldout,
            OfflineVldaSplitRole::Heldout,
        ];

        let model = train_standardized_centroid_model(&samples, &labels, &roles)
            .unwrap()
            .unwrap();

        assert!(model.features.iter().all(|value| value.is_finite()));
        assert!(model
            .centroids
            .iter()
            .flatten()
            .all(|value| value.is_finite()));
    }

    #[test]
    fn centroid_standardization_deactivates_train_constant_columns() {
        let make_sample = |idx: usize, value: f64| OfflineVldaSample {
            sample_id: format!("constant-{idx}"),
            episode_id: None,
            v: vec![value],
            l: vec![value],
            d: vec![value],
            a: vec![value],
            labels: BTreeMap::new(),
            metadata: BTreeMap::new(),
        };
        let samples = vec![
            make_sample(0, 7.0),
            make_sample(1, 7.0),
            make_sample(2, f64::MAX),
        ];
        let labels = [false, true, true];
        let roles = [
            OfflineVldaSplitRole::Train,
            OfflineVldaSplitRole::Train,
            OfflineVldaSplitRole::Heldout,
        ];

        let model = train_standardized_centroid_model(&samples, &labels, &roles)
            .unwrap()
            .unwrap();

        assert!(model.features.iter().all(|value| *value == 0.0));
    }

    #[test]
    fn nearest_neighbor_rejects_unrepresentable_squared_distance() {
        let make_sample = |idx: usize, value: f64| OfflineVldaSample {
            sample_id: format!("extreme-{idx}"),
            episode_id: None,
            v: vec![value],
            l: vec![0.0],
            d: vec![0.0],
            a: vec![0.0],
            labels: BTreeMap::new(),
            metadata: BTreeMap::new(),
        };
        let samples = [make_sample(0, -f64::MAX), make_sample(1, f64::MAX)];

        let error =
            match compute_nn_baselines(&samples, &[false, true], None, None, "V", |left, right| {
                squared_euclidean(&left.v, &right.v)
            }) {
                Ok(_) => panic!("unrepresentable squared distance must reject"),
                Err(error) => error,
            };

        assert!(error.to_string().contains("squared distance is not finite"));
    }

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
            // Mixed-support regression fixture. `L` is a binary instruction/condition indicator by
            // construction — that is a property of this fixture's DGP, declared here, NOT inferred
            // from the observed cardinality. It exists to prove that unsupported inputs produce a
            // clean, auditable abstention.
            support: declared_support(&[
                (
                    "v",
                    OfflineVldaDeclaredSupport::ContinuousRegularFullDimensional,
                ),
                ("l", OfflineVldaDeclaredSupport::Categorical),
                (
                    "d",
                    OfflineVldaDeclaredSupport::ContinuousRegularFullDimensional,
                ),
                (
                    "a",
                    OfflineVldaDeclaredSupport::ContinuousRegularFullDimensional,
                ),
            ]),
            continuous_tuple_support: BTreeMap::new(),
            capture_integrity: None,
            publication_receipt: None,
            publication_receipt_verified_content_sha256: None,
            samples,
        }
    }

    #[test]
    fn streaming_dataset_hash_matches_canonical_json_v2() {
        let mut dataset = fixture_dataset();
        dataset.capture_integrity = Some("complete_with_warning".to_string());
        dataset.publication_receipt = Some("receipt.json".to_string());
        dataset.samples[0].labels.insert(
            "nested".to_string(),
            json!({
                "z": [null, true, {"b": 2, "a": 1}],
                "a": "value"
            }),
        );

        assert_eq!(
            offline_vlda_dataset_content_sha256(&dataset).unwrap(),
            pid_runlog::canonical_json_hash_v2(&dataset).unwrap()
        );
    }

    #[test]
    fn streaming_sample_hash_matches_canonical_json_v2() {
        let mut sample = fixture_dataset().samples.remove(0);
        sample.labels.insert(
            "nested".to_string(),
            json!({
                "z": [null, true, {"b": 2, "a": 1}],
                "a": "value"
            }),
        );

        assert_eq!(
            offline_vlda_sample_content_sha256(&sample).unwrap(),
            pid_runlog::canonical_json_hash_v2(&sample).unwrap()
        );
    }

    #[test]
    fn streaming_sample_hash_has_no_runlog_line_size_ceiling() {
        let mut sample = fixture_dataset().samples.remove(0);
        sample.labels.insert(
            "payload".to_string(),
            Value::String("x".repeat(4 * 1_024 * 1_024)),
        );

        assert!(pid_runlog::canonical_json_hash_v2(&sample).is_err());
        let expected = pid_runlog::canonical_json_hash_v2_with_limits(
            &sample,
            RunLogLimits::default()
                .with_max_line_bytes(8 * 1_024 * 1_024)
                .with_max_string_bytes(8 * 1_024 * 1_024),
        )
        .unwrap();
        assert_eq!(
            offline_vlda_sample_content_sha256(&sample).unwrap(),
            expected
        );
    }

    #[test]
    fn streaming_dataset_hash_has_no_runlog_line_size_ceiling() {
        let label = "x".repeat(4_096);
        let samples = (0..1_024)
            .map(|index| OfflineVldaSample {
                sample_id: format!("sample-{index:04}"),
                episode_id: None,
                v: vec![index as f64],
                l: vec![0.0],
                d: vec![0.0],
                a: vec![0.0],
                labels: [("payload".to_string(), Value::String(label.clone()))]
                    .into_iter()
                    .collect(),
                metadata: BTreeMap::new(),
            })
            .collect();
        let dataset = OfflineVldaDataset {
            run_id: None,
            source: None,
            model: None,
            task: None,
            support: BTreeMap::new(),
            continuous_tuple_support: BTreeMap::new(),
            capture_integrity: None,
            publication_receipt: None,
            publication_receipt_verified_content_sha256: None,
            samples,
        };

        assert!(pid_runlog::canonical_json_hash_v2(&dataset).is_err());
        let expected = pid_runlog::canonical_json_hash_v2_with_limits(
            &dataset,
            RunLogLimits::default().with_max_line_bytes(8 * 1_024 * 1_024),
        )
        .unwrap();
        assert_eq!(
            offline_vlda_dataset_content_sha256(&dataset).unwrap(),
            expected
        );
    }

    fn legacy_ncp_fixture_config() -> serde_json::Value {
        json!({
            "component": "ncp-observer",
            "ncp": {
                "tag": LEGACY_NCP_TAG,
                "revision": LEGACY_NCP_REVISION,
                "wire": LEGACY_NCP_WIRE,
                "contract_hash": LEGACY_NCP_COMPACT_HASH,
            },
            "fixture": "ncp-publication",
        })
    }

    fn write_ncp_publication_fixture_with_config(
        integrity: &str,
        config: serde_json::Value,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "pid-offline-ncp-publication-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let dataset_path = dir.join("dataset.json");
        let runlog_path = dir.join("runlog.jsonl");
        let receipt_path = dir.join("dataset.json.publication.json");

        let mut dataset = fixture_dataset();
        dataset.source = Some("ncp".to_string());
        dataset.run_id = Some("ncp-fixture".to_string());
        dataset.capture_integrity = Some(integrity.to_string());
        dataset.publication_receipt = Some(receipt_path.display().to_string());
        dataset.publication_receipt_verified_content_sha256 = None;
        let dataset_bytes = serde_json::to_vec_pretty(&dataset).unwrap();
        std::fs::write(&dataset_path, &dataset_bytes).unwrap();

        let config_hash = pid_runlog::canonical_json_hash_v2(&config).unwrap();
        let mut writer = RunLogWriter::new(Vec::new());
        writer
            .append(&RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "ncp-fixture".to_string(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: BTreeMap::new(),
            })
            .unwrap();
        writer
            .append(&RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            })
            .unwrap();
        writer
            .append(&RunLogEvent::ArtifactLogged {
                timestamp_ns: 0,
                name: "ncp_vlda_dataset".to_string(),
                kind: "dataset_json".to_string(),
                uri: std::fs::canonicalize(&dataset_path)
                    .unwrap()
                    .display()
                    .to_string(),
                sha256: Some(pid_runlog::sha256_hex(&dataset_bytes)),
                metadata: BTreeMap::from([(
                    "capture_integrity".to_string(),
                    integrity.to_string(),
                )]),
            })
            .unwrap();
        writer
            .append(&RunLogEvent::RunEnded {
                run_id: "ncp-fixture".to_string(),
                timestamp_ns: 1,
                status: RunStatus::Succeeded,
                message: None,
            })
            .unwrap();
        writer.flush().unwrap();
        let runlog_bytes = writer.into_inner();
        std::fs::write(&runlog_path, &runlog_bytes).unwrap();

        let receipt = json!({
            "schema_version": 1,
            "committed": true,
            "dataset_uri": std::fs::canonicalize(&dataset_path).unwrap().display().to_string(),
            "dataset_sha256": pid_runlog::sha256_hex(&dataset_bytes),
            "runlog_uri": std::fs::canonicalize(&runlog_path).unwrap().display().to_string(),
            "runlog_sha256": pid_runlog::sha256_hex(&runlog_bytes),
            "capture_integrity": integrity,
        });
        std::fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        (dataset_path, dir)
    }

    fn write_ncp_publication_fixture(integrity: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        write_ncp_publication_fixture_with_config(integrity, legacy_ncp_fixture_config())
    }

    #[test]
    fn ncp_input_requires_committed_hash_verified_complete_publication() {
        let mut unverified_ncp_convention = fixture_dataset();
        for sample in &mut unverified_ncp_convention.samples {
            sample
                .metadata
                .insert("l_source".to_string(), "channel".to_string());
            sample
                .metadata
                .insert("d_source".to_string(), "source".to_string());
        }
        let error = run_offline_vlda_harness(unverified_ncp_convention, None, None).unwrap_err();
        assert!(error
            .to_string()
            .contains("NCP-marked dataset must declare source=\"ncp\""));

        let (dataset_path, dir) = write_ncp_publication_fixture("complete");
        let dataset = read_offline_vlda_dataset(&dataset_path).unwrap();
        assert!(dataset
            .publication_receipt_verified_content_sha256
            .is_some());
        assert!(run_offline_vlda_harness(dataset.clone(), None, None).is_ok());

        let mut mutated = dataset;
        mutated.samples[0].a[0] += 1.0;
        let error = run_offline_vlda_harness(mutated, None, None).unwrap_err();
        assert!(error
            .to_string()
            .contains("changed after NCP publication receipt verification"));
        std::fs::remove_dir_all(dir).ok();

        let (dataset_path, dir) = write_ncp_publication_fixture("complete_with_warning");
        let dataset = read_offline_vlda_dataset(&dataset_path).unwrap();
        assert!(dataset
            .publication_receipt_verified_content_sha256
            .is_some());
        std::fs::remove_dir_all(dir).ok();

        let (dataset_path, dir) = write_ncp_publication_fixture("invalid");
        let error = read_offline_vlda_dataset(&dataset_path).unwrap_err();
        assert!(error.to_string().contains("diagnostic-only"));
        std::fs::remove_dir_all(dir).ok();

        let (dataset_path, dir) = write_ncp_publication_fixture("complete");
        std::fs::remove_file(dir.join("dataset.json.publication.json")).unwrap();
        let error = read_offline_vlda_dataset(&dataset_path).unwrap_err();
        assert!(error
            .to_string()
            .contains("failed to inspect NCP publication receipt"));
        std::fs::remove_dir_all(dir).ok();

        let (dataset_path, dir) = write_ncp_publication_fixture("complete");
        let mut dataset: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&dataset_path).unwrap()).unwrap();
        dataset["run_id"] = serde_json::Value::Null;
        std::fs::write(&dataset_path, serde_json::to_vec_pretty(&dataset).unwrap()).unwrap();
        let error = read_offline_vlda_dataset(&dataset_path).unwrap_err();
        assert!(error.to_string().contains("nonempty run_id"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn malformed_ncp_input_rejects_before_receipt_io() {
        let (dataset_path, dir) = write_ncp_publication_fixture("complete");
        let mut dataset: Value =
            serde_json::from_slice(&std::fs::read(&dataset_path).unwrap()).unwrap();
        dataset["samples"] = Value::Array(
            dataset["samples"]
                .as_array()
                .unwrap()
                .iter()
                .take(1)
                .cloned()
                .collect(),
        );
        std::fs::write(&dataset_path, serde_json::to_vec(&dataset).unwrap()).unwrap();
        std::fs::remove_file(dir.join("dataset.json.publication.json")).unwrap();

        let error = read_offline_vlda_dataset(&dataset_path).unwrap_err();

        assert!(format!("{error:#}").contains("must contain at least 8 samples"));
        assert!(!format!("{error:#}").contains("publication receipt"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ncp_input_requires_the_exact_dataset_artifact_event_identity() {
        let (dataset_path, dir) = write_ncp_publication_fixture("complete");
        let runlog_path = dir.join("runlog.jsonl");
        let mut events = read_events_from_path(&runlog_path).unwrap();
        for event in &mut events {
            if let RunLogEvent::ArtifactLogged { name, kind, .. } = event {
                *name = "unrelated_artifact".to_string();
                *kind = "model".to_string();
            }
        }
        let mut writer = RunLogWriter::new(Vec::new());
        for event in &events {
            writer.append(event).unwrap();
        }
        writer.flush().unwrap();
        let runlog_bytes = writer.into_inner();
        std::fs::write(&runlog_path, &runlog_bytes).unwrap();
        let receipt_path = dir.join("dataset.json.publication.json");
        let mut receipt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&receipt_path).unwrap()).unwrap();
        receipt["runlog_sha256"] = json!(pid_runlog::sha256_hex(&runlog_bytes));
        std::fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();

        let error = read_offline_vlda_dataset(&dataset_path).unwrap_err();

        assert!(error.to_string().contains("does not bind"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ncp_input_rejects_duplicate_runlog_members_even_when_receipt_hash_matches() {
        let (dataset_path, dir) = write_ncp_publication_fixture("complete");
        let runlog_path = dir.join("runlog.jsonl");
        let original = std::fs::read_to_string(&runlog_path).unwrap();
        let duplicate = original.replacen(
            "\"run_id\":\"ncp-fixture\"",
            "\"run_id\":\"ncp-fixture\",\"run_id\":\"ncp-fixture\"",
            1,
        );
        assert_ne!(duplicate, original);
        std::fs::write(&runlog_path, duplicate.as_bytes()).unwrap();

        let receipt_path = dir.join("dataset.json.publication.json");
        let mut receipt: Value =
            serde_json::from_slice(&std::fs::read(&receipt_path).unwrap()).unwrap();
        receipt["runlog_sha256"] = json!(pid_runlog::sha256_hex(duplicate.as_bytes()));
        std::fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();

        let error = read_offline_vlda_dataset(&dataset_path)
            .expect_err("content-bound duplicate keys must fail closed");

        assert!(format!("{error:#}").contains("duplicate JSON object key \"run_id\""));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ncp_schema_1_receipt_rejects_missing_or_drifted_legacy_identity() {
        let (dataset_path, dir) = write_ncp_publication_fixture_with_config(
            "complete",
            json!({"fixture": "ncp-publication"}),
        );
        let error = read_offline_vlda_dataset(&dataset_path).unwrap_err();
        assert!(error.to_string().contains("does not bind the frozen"));
        std::fs::remove_dir_all(dir).ok();

        for (pointer, drifted_value) in [
            ("/ncp/tag", json!("v1.0.0")),
            (
                "/ncp/revision",
                json!("1ffd3bf9a6c52d0279eb31a56e0664e4eec24d68"),
            ),
            ("/ncp/wire", json!("1.0")),
            ("/ncp/contract_hash", json!("163acc57d8a62b66")),
        ] {
            let mut config = legacy_ncp_fixture_config();
            *config.pointer_mut(pointer).unwrap() = drifted_value;
            let (dataset_path, dir) = write_ncp_publication_fixture_with_config("complete", config);
            let error = read_offline_vlda_dataset(&dataset_path).unwrap_err();
            assert!(
                error.to_string().contains("does not bind the frozen"),
                "schema-1 receipt accepted drift at {pointer}: {error}"
            );
            std::fs::remove_dir_all(dir).ok();
        }
    }

    #[test]
    fn frozen_legacy_identity_requires_exactly_one_config_event() {
        let exact = RunLogEvent::ConfigLogged {
            timestamp_ns: 0,
            config_hash: "exact".to_string(),
            config: legacy_ncp_fixture_config(),
        };
        assert!(has_frozen_legacy_ncp_config(std::slice::from_ref(&exact)));

        let confounding = RunLogEvent::ConfigLogged {
            timestamp_ns: 1,
            config_hash: "confounding".to_string(),
            config: json!({
                "component": "ncp-observer10",
                "ncp": {
                    "tag": "1.0.0-rc.1",
                    "revision": "1ffd3bf9a6c52d0279eb31a56e0664e4eec24d68",
                    "wire": "1.0",
                    "contract_hash": "163acc57d8a62b66",
                },
            }),
        };
        assert!(!has_frozen_legacy_ncp_config(&[exact, confounding]));
        assert!(!has_frozen_legacy_ncp_config(&[]));
    }

    #[test]
    fn offline_input_must_be_a_regular_file() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pid-offline-nonregular-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();

        let error = read_offline_vlda_dataset(&dir).unwrap_err();

        assert!(error.to_string().contains("regular file"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn runlog_rejects_an_input_path_replaced_after_analysis() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pid-offline-snapshot-{stamp}"));
        std::fs::create_dir_all(&dir).unwrap();
        let input_path = dir.join("dataset.json");
        let runlog_path = dir.join("runlog.jsonl");
        std::fs::write(
            &input_path,
            serde_json::to_vec_pretty(&fixture_dataset()).unwrap(),
        )
        .unwrap();
        let (dataset, snapshot_sha256) = read_offline_vlda_dataset_with_hash(&input_path).unwrap();
        let report = run_offline_vlda_harness(
            dataset.clone(),
            Some(input_path.to_str().unwrap().to_string()),
            Some(snapshot_sha256.clone()),
        )
        .unwrap();
        std::fs::write(&input_path, b"replacement after parse").unwrap();

        let error =
            write_offline_vlda_runlog(&runlog_path, None, Some(&input_path), &dataset, &report)
                .unwrap_err();

        assert!(error
            .to_string()
            .contains("no longer matches the analyzed snapshot"));
        assert!(!runlog_path.exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn runlog_rejects_input_bytes_that_do_not_encode_the_publication_dataset() {
        let directory = tempfile::tempdir().unwrap();
        let input_path = directory.path().join("dataset.json");
        let runlog_path = directory.path().join("runlog.jsonl");
        let recorded_dataset = fixture_dataset();
        std::fs::write(
            &input_path,
            serde_json::to_vec_pretty(&recorded_dataset).unwrap(),
        )
        .unwrap();
        let input_sha256 = pid_runlog::sha256_file(&input_path).unwrap();

        let mut publication_dataset = recorded_dataset;
        publication_dataset.task = Some("different-publication-dataset".to_string());
        let report = run_offline_vlda_harness(
            publication_dataset.clone(),
            Some(input_path.to_str().unwrap().to_string()),
            Some(input_sha256),
        )
        .unwrap();

        let error = write_offline_vlda_runlog(
            &runlog_path,
            None,
            Some(&input_path),
            &publication_dataset,
            &report,
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("does not encode the publication dataset"));
        assert!(!runlog_path.exists());
    }

    #[test]
    fn runlog_rejects_ambiguous_publication_input_json() {
        let directory = tempfile::tempdir().unwrap();
        let input_path = directory.path().join("dataset.json");
        let runlog_path = directory.path().join("runlog.jsonl");
        let dataset = fixture_dataset();
        let encoded = serde_json::to_string_pretty(&dataset).unwrap();
        let ambiguous = encoded.replacen(
            "  \"task\": \"fixture_task\",",
            "  \"task\": \"fixture_task\",\n  \"task\": \"fixture_task\",",
            1,
        );
        assert_ne!(ambiguous, encoded);
        std::fs::write(&input_path, ambiguous).unwrap();
        let input_sha256 = pid_runlog::sha256_file(&input_path).unwrap();
        let report = run_offline_vlda_harness(
            dataset.clone(),
            Some(input_path.to_str().unwrap().to_string()),
            Some(input_sha256),
        )
        .unwrap();

        let error =
            write_offline_vlda_runlog(&runlog_path, None, Some(&input_path), &dataset, &report)
                .unwrap_err();

        assert!(format!("{error:#}").contains("duplicate JSON object key"));
        assert!(!runlog_path.exists());
    }

    /// pid-runlog schema 2 requires a real 64-character hex SHA-256 digest; a stub like "abc" is
    /// now a validation ERROR rather than a legacy warning.
    const TEST_INPUT_SHA256: &str =
        "834c3f1794205b56bc0446f7524d4625fe90809341db76e5acdfa1d581c019f6";

    fn declared_support(
        entries: &[(&str, OfflineVldaDeclaredSupport)],
    ) -> BTreeMap<String, OfflineVldaDeclaredSupport> {
        entries
            .iter()
            .map(|(axis, support)| ((*axis).to_string(), *support))
            .collect()
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

    /// Positive-path fixture: the committed all-continuous dataset.
    ///
    /// Declared continuous on every axis, **equal ambient source dimensions** (continuous shared
    /// exclusions requires them), and tie-free — so the continuous KSG / `I^sx` path stays covered
    /// even though the mixed-support fixture abstains from it. Loaded from the real committed
    /// fixture so the tests exercise exactly what ships.
    pub(super) fn continuous_fixture_dataset() -> OfflineVldaDataset {
        serde_json::from_str(include_str!(
            "../fixtures/offline_vlda_continuous_fixture.json"
        ))
        .expect("continuous fixture parses")
    }

    fn as_single_ordered_episode(mut dataset: OfflineVldaDataset) -> OfflineVldaDataset {
        for (index, sample) in dataset.samples.iter_mut().enumerate() {
            sample.episode_id = Some("single-ordered-episode".to_string());
            sample
                .metadata
                .insert("sequence_index".to_string(), index.to_string());
        }
        dataset
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
            sample("channel", "source"),
            sample("channel", "source"),
            sample("absent_zeroed", "source"),
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
        let ok = vec![sample("channel", "source")];
        let p = axis_provenance(&ok);
        assert!(p.iter().all(|x| x.status == "ok" && x.note.is_none()));

        // Invented values do not become honest merely because they avoid a denylist.
        let unknown = axis_provenance(&[sample("channel", "invented")]);
        let d = unknown.iter().find(|p| p.axis == "D").unwrap();
        assert_eq!(d.status, "degraded");
        assert_eq!(d.degraded_samples, 1);
        assert!(d.note.as_ref().unwrap().contains("unrecognized: 1"));
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

        // Once the SAFE convention is active, every sample must carry all four
        // markers. Sparse positive attestation fails closed.
        let mut sparse = safe("token_slice:language");
        sparse.metadata.remove("d_provenance");
        let prov = axis_provenance(&[safe("token_slice:language"), sparse]);
        let d = prov
            .iter()
            .find(|p| p.axis == "D" && p.marker == "d_provenance")
            .unwrap();
        assert_eq!(d.status, "degraded");
        assert_eq!(d.total_samples, 2);
        assert_eq!(d.degraded_samples, 1);
        assert!(d.note.as_ref().unwrap().contains("missing: 1"));

        // Empty token-slice declarations are not accepted provenance values.
        let malformed = axis_provenance(&[safe("token_slice:")]);
        let l = malformed
            .iter()
            .find(|p| p.axis == "L" && p.marker == "l_provenance")
            .unwrap();
        assert_eq!(l.status, "degraded");
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
    fn geometry_diagnostics_flag_all_constant_variable_as_degenerate() {
        // An all-constant L (every dim zero-variance, e.g. a fabricated all-zero language
        // channel — NCP_DEV_PROMPT Gap 2) must be flagged. The observed sample contains no
        // variation on that axis, and the continuous estimator rejects the exact ties. This
        // diagnostic does not infer the population law or assign zero to a PID atom.
        let mut variables = BTreeMap::new();
        let mut preprocessing = BTreeMap::new();
        preprocessing.insert("V".to_string(), preprocessing_variable(4, 0));
        preprocessing.insert("L".to_string(), preprocessing_variable(8, 8));
        let diagnostics = compute_geometry_diagnostics(
            &variables,
            &OfflineVldaPreprocessingReport {
                strategy: "per_variable_standardized".to_string(),
                variables: preprocessing.clone(),
            },
        );
        assert_eq!(diagnostics.status, "warning");
        let degenerate: Vec<_> = diagnostics
            .warnings
            .iter()
            .filter(|w| w.contains("all-constant"))
            .collect();
        assert_eq!(
            degenerate.len(),
            1,
            "exactly L should be flagged: {:?}",
            diagnostics.warnings
        );
        assert!(degenerate[0].contains("geometry L is all-constant"));

        // A non-degenerate set (no fully zero-variance variable, no geometry variables)
        // produces no degenerate-axis warning.
        variables.clear();
        let mut healthy = BTreeMap::new();
        healthy.insert("V".to_string(), preprocessing_variable(4, 1));
        healthy.insert("L".to_string(), preprocessing_variable(8, 0));
        let diagnostics = compute_geometry_diagnostics(
            &variables,
            &OfflineVldaPreprocessingReport {
                strategy: "per_variable_standardized".to_string(),
                variables: healthy,
            },
        );
        assert!(
            diagnostics
                .warnings
                .iter()
                .all(|w| !w.contains("all-constant")),
            "no variable should be flagged degenerate: {:?}",
            diagnostics.warnings
        );
    }

    #[test]
    fn categorical_sx_mode_emits_components_and_saturation_diagnostics() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::CategoricalSx,
            categorical_bins: 6,
            pls: PlsComponentSelection::Fixed(2),
        };
        let report =
            run_offline_vlda_harness_with_options(dataset.clone(), None, None, &options).unwrap();
        assert_eq!(report.metrics.pid_pairs.len(), 3);
        assert_eq!(
            report
                .metrics
                .categorical_quantization
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["A", "D", "L", "V"]
        );
        for (axis, receipt) in &report.metrics.categorical_quantization {
            assert_eq!(&receipt.axis, axis);
            assert_eq!(receipt.bins_per_dimension, 6);
            assert_eq!(receipt.samples, report.dims.samples);
            assert_eq!(receipt.training_input_sha256.len(), 64);
            // The same matrix is bound in two deliberately domain-separated hashes.
            assert_ne!(
                receipt.training_input_sha256,
                receipt.transform_input_sha256
            );
            assert_eq!(receipt.out_of_range_policy, "error");
            assert!(receipt.functional.contains("Makkeh-Gutknecht-Wibral"));
            assert_eq!(receipt.information_units, "nats");
        }
        for (pair_name, pair) in &report.metrics.pid_pairs {
            assert_eq!(pair.outcome.information_units, "nats");
            let saturation = pair
                .categorical_saturation
                .as_ref()
                .unwrap_or_else(|| panic!("{pair_name} missing saturation diagnostics"));
            assert!(saturation.unique_fraction_joint > 0.0);
            assert_eq!(saturation.empirical_sample_count, report.dims.samples);
            assert_eq!(
                saturation.observed_joint_states as f64 / saturation.empirical_sample_count as f64,
                saturation.unique_fraction_joint
            );
            assert!(saturation
                .population_caveat
                .contains("plug-in bias remains"));
            let components = pair
                .categorical_sx_components
                .as_ref()
                .unwrap_or_else(|| panic!("{pair_name} missing categorical Sx components"));
            for (atom, net) in [
                (components.redundancy, pair.redundancy.unwrap()),
                (components.unique_source_1, pair.unique_source_1.unwrap()),
                (components.unique_source_2, pair.unique_source_2.unwrap()),
                (components.synergy, pair.synergy.unwrap()),
            ] {
                assert!(atom.informative >= -1e-12);
                assert!(atom.misinformative >= -1e-12);
                assert!((atom.net - (atom.informative - atom.misinformative)).abs() < 1e-10);
                assert_eq!(atom.net.to_bits(), net.to_bits());
            }
            let reconstructed = pair.redundancy.unwrap()
                + pair.unique_source_1.unwrap()
                + pair.unique_source_2.unwrap()
                + pair.synergy.unwrap();
            assert!((reconstructed - pair.mi_joint_action.unwrap()).abs() < 1e-10);
        }

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let runlog_path =
            std::env::temp_dir().join(format!("pid-offline-vlda-categorical-sx-{stamp}.jsonl"));
        write_offline_vlda_runlog(&runlog_path, None, None, &dataset, &report).unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let warned_pair_metadata = events.iter().find_map(|event| match event {
            RunLogEvent::PidMetric { metadata, .. }
                if metadata.get("pid_pair").map(String::as_str) == Some("VL") =>
            {
                Some(metadata)
            }
            _ => None,
        });
        let warned_pair_metadata = warned_pair_metadata.expect("VL metric event");
        assert_eq!(
            warned_pair_metadata
                .get("computation_status")
                .map(String::as_str),
            Some("produced_with_warning")
        );
        assert_eq!(
            warned_pair_metadata
                .get("scientific_gate_application")
                .map(String::as_str),
            Some("blocked")
        );
        assert_eq!(
            warned_pair_metadata
                .get("interpretation_allowed")
                .map(String::as_str),
            Some("false")
        );
        assert_eq!(
            warned_pair_metadata.get("warning_code").map(String::as_str),
            Some("categorical_saturation")
        );
        assert_eq!(
            warned_pair_metadata.get("units").map(String::as_str),
            Some("nats")
        );
        std::fs::remove_file(runlog_path).unwrap();

        let mut forged_units = report.clone();
        forged_units
            .metrics
            .categorical_quantization
            .get_mut("V")
            .unwrap()
            .information_units = "bits".to_string();
        assert!(validate_offline_vlda_report(&forged_units)
            .unwrap_err()
            .to_string()
            .contains("quantizer receipt contradicts"));

        let mut forged_estimand = report.clone();
        forged_estimand
            .metrics
            .categorical_quantization
            .get_mut("V")
            .unwrap()
            .estimand_statement = "continuous PID after quantization".to_string();
        assert!(validate_offline_vlda_report(&forged_estimand)
            .unwrap_err()
            .to_string()
            .contains("quantizer receipt contradicts"));

        let mut forged_occupancy = report;
        let receipt = forged_occupancy
            .metrics
            .categorical_quantization
            .get_mut("V")
            .unwrap();
        let nominal = receipt
            .nominal_joint_cardinality
            .as_ref()
            .expect("fixture cardinality fits u128")
            .parse::<u128>()
            .unwrap();
        receipt.empty_joint_cells = Some(nominal.to_string());
        assert!(validate_offline_vlda_report(&forged_occupancy)
            .unwrap_err()
            .to_string()
            .contains("inconsistent nominal and empty cardinalities"));
    }

    #[test]
    fn categorical_sx_pls_mode_projects_then_quantizes() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::CategoricalSxPls,
            categorical_bins: 6,
            pls: PlsComponentSelection::Fixed(1),
        };
        let report = run_offline_vlda_harness_with_options(dataset, None, None, &options).unwrap();
        assert_eq!(report.metrics.pid_pairs.len(), 3);
        assert_eq!(
            report.config["metric_pipeline"]["pid_evaluation_relation"],
            "same_rows_target_supervised_projection_and_fitted_quantization_warning"
        );
        for outcome in [
            &report.metrics.mi_v_action.outcome,
            &report.metrics.mi_l_action.outcome,
            &report.metrics.mi_d_action.outcome,
        ] {
            assert_eq!(
                outcome.status,
                OfflineVldaEstimateStatus::ProducedWithWarning
            );
            assert_eq!(
                outcome.scientific_gates.estimator,
                OfflineVldaScientificGateVerdict::Blocked
            );
            assert!(matches!(
                outcome.scientific_gates.reason_code.as_deref(),
                Some(SCIENTIFIC_REASON_SUPERVISED_SAME_ROW)
                    | Some(SCIENTIFIC_REASON_SUPERVISED_SAME_ROW_AND_SATURATION)
            ));
        }
        let vl = &report.metrics.pid_pairs["VL"];
        assert!(vl.categorical_saturation.is_some());
        assert!(vl.mi_source_1_action.unwrap().is_finite());
        assert_eq!(
            vl.outcome.status,
            OfflineVldaEstimateStatus::ProducedWithWarning
        );
        assert_eq!(
            vl.outcome.scientific_gates.reason_code.as_deref(),
            Some(SCIENTIFIC_REASON_SUPERVISED_SAME_ROW_AND_SATURATION)
        );
        // Preregistered mitigations (grandplan §6.2 leakage-safe fitted preprocessing): selection
        // provenance and the fixed-seed shuffled-target negative-control draw ride along.
        let sel = report.metrics.pls_selection.as_ref().unwrap();
        assert_eq!(sel.method, "fixed");
        assert_eq!(
            (sel.components_v, sel.components_l, sel.components_d),
            (1, 1, 1)
        );
        let control = report
            .metrics
            .pls_shuffled_target_control
            .as_ref()
            .expect("categorical-sx-pls carries its selection-inflation control");
        assert!(report.metrics.pls_control_seed.is_some());
        assert_eq!(control.pid_pairs.len(), 3);
        // The control ran the identical pipeline against a shuffled target;
        // its values must be finite, and it must not recurse into its own
        // control. NOTE the fixture is small enough that the binned joint
        // table has all-singleton cells under BOTH pairings, so the discrete
        // MI here saturates to a pure function of the marginals and the
        // control EQUALS the real screen. This fixture therefore shows no separation from this one
        // negative-control draw. One draw does not estimate a null distribution or prove that all
        // signal is artifact. (The
        // per-pair `categorical_saturation` diagnostic flags the same regime.)
        assert!(control.mi_v_action.value.unwrap().is_finite());
        assert_eq!(
            control.mi_v_action.value, report.metrics.mi_v_action.value,
            "saturated fixture: the negative-control draw equals the observed screen"
        );
        assert!(control.pls_shuffled_target_control.is_none());
        // Train-split screen must also run under the PLS-projected discrete path.
        let train_pid = report.train_split_pid.as_ref().unwrap();
        assert_eq!(train_pid.status, "available");
        assert_eq!(train_pid.metrics.as_ref().unwrap().pid_pairs.len(), 3);
        assert!(train_pid
            .metrics
            .as_ref()
            .unwrap()
            .pid_pairs
            .values()
            .all(|pair| pair.outcome.status == OfflineVldaEstimateStatus::ProducedWithWarning));

        let mut forged_clean = report;
        forged_clean
            .metrics
            .pid_pairs
            .get_mut("VL")
            .unwrap()
            .outcome
            .status = OfflineVldaEstimateStatus::Produced;
        let error = validate_offline_vlda_report(&forged_clean)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("categorical warning does not match the fitted-preprocessing contract"),
            "unexpected validation error: {error}"
        );
    }

    #[test]
    fn categorical_sx_pls_cv_selection_reports_components_and_q2() {
        let dataset = fixture_dataset();
        let options = OfflineVldaHarnessOptions {
            pid_mode: PidMode::CategoricalSxPls,
            categorical_bins: 6,
            pls: PlsComponentSelection::CvQ2 { max_components: 3 },
        };
        let report =
            run_offline_vlda_harness_with_options(dataset.clone(), None, None, &options).unwrap();
        let sel = report.metrics.pls_selection.as_ref().unwrap();
        assert_eq!(sel.method, "cv_q2");
        for k in [sel.components_v, sel.components_l, sel.components_d] {
            assert!((1..=3).contains(&k), "selected components {k} out of range");
        }
        assert!(sel.q2_v.is_some() && sel.q2_l.is_some() && sel.q2_d.is_some());
        // Deterministic given the same inputs.
        let report2 = run_offline_vlda_harness_with_options(dataset, None, None, &options).unwrap();
        assert_eq!(report.metrics.pls_selection, report2.metrics.pls_selection);
        assert_eq!(
            report.metrics.pls_shuffled_target_control,
            report2.metrics.pls_shuffled_target_control
        );
    }

    #[test]
    fn non_pls_modes_carry_no_pls_provenance() {
        let dataset = fixture_dataset();
        let report = run_offline_vlda_harness(dataset, None, None).unwrap();
        assert!(report.metrics.pls_selection.is_none());
        assert!(report.metrics.pls_shuffled_target_control.is_none());
        assert!(report.metrics.pls_control_seed.is_none());
        assert!(report.metrics.categorical_quantization.is_empty());
    }

    #[test]
    fn continuous_mode_has_no_saturation_diagnostics() {
        let dataset = fixture_dataset();
        let report = run_offline_vlda_harness(dataset, None, None).unwrap();
        for pair in report.metrics.pid_pairs.values() {
            assert!(pair.categorical_saturation.is_none());
        }
    }

    #[test]
    fn continuous_mode_requires_each_complete_tuple_support_contract() {
        let mut missing = continuous_fixture_dataset();
        missing
            .continuous_tuple_support
            .remove(CONTINUOUS_TUPLE_V_A);
        let report =
            run_offline_vlda_harness_with_options(missing, None, None, &continuous_options())
                .unwrap();
        assert_eq!(
            report.metrics.mi_v_action.outcome.reason_code,
            Some(OfflineVldaAbstainReason::TupleSupportContractUnspecified)
        );
        assert!(report.metrics.mi_v_action.value.is_none());
        assert!(report
            .metrics
            .mi_l_action
            .outcome
            .declared_continuous_tuple_support
            .is_some_and(OfflineVldaContinuousTupleSupport::is_regular));

        let mut singular = continuous_fixture_dataset();
        singular.continuous_tuple_support.insert(
            CONTINUOUS_TUPLE_V_L_A.to_string(),
            OfflineVldaContinuousTupleSupport::KnownSingularOrLowerDimensional,
        );
        let report =
            run_offline_vlda_harness_with_options(singular, None, None, &continuous_options())
                .unwrap();
        let pair = &report.metrics.pid_pairs["VL"];
        assert_eq!(
            pair.outcome.reason_code,
            Some(OfflineVldaAbstainReason::DeclaredTupleSupportIncompatibleContinuous)
        );
        assert_eq!(
            pair.outcome.declared_continuous_tuple_support,
            Some(OfflineVldaContinuousTupleSupport::KnownSingularOrLowerDimensional)
        );
        assert!(pair.redundancy.is_none());
    }

    #[test]
    fn continuous_support_contract_rejects_internal_contradictions() {
        let mut axis_conflict = continuous_fixture_dataset();
        axis_conflict
            .support
            .insert("v".to_string(), OfflineVldaDeclaredSupport::Categorical);
        let error =
            run_offline_vlda_harness_with_options(axis_conflict, None, None, &continuous_options())
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("axis \"v\" has an explicitly incompatible support declaration"));

        let mut marginal_conflict = continuous_fixture_dataset();
        marginal_conflict.continuous_tuple_support.insert(
            CONTINUOUS_TUPLE_V_A.to_string(),
            OfflineVldaContinuousTupleSupport::KnownSingularOrLowerDimensional,
        );
        let error = run_offline_vlda_harness_with_options(
            marginal_conflict,
            None,
            None,
            &continuous_options(),
        )
        .unwrap_err();
        assert!(error.to_string().contains(
            "tuple \"v_l_a\" declares every required marginal regular, but the explicit \"v_a\" declaration is incompatible"
        ));

        let mut compatible = continuous_fixture_dataset();
        compatible.continuous_tuple_support.insert(
            CONTINUOUS_TUPLE_V_L_A.to_string(),
            OfflineVldaContinuousTupleSupport::KnownSingularOrLowerDimensional,
        );
        assert!(run_offline_vlda_harness_with_options(
            compatible,
            None,
            None,
            &continuous_options(),
        )
        .is_ok());
    }

    #[test]
    fn categorical_sx_mode_marks_missing_population_support_not_evaluated() {
        let mut dataset = fixture_dataset();
        dataset.support.remove("v");
        let report = run_offline_vlda_harness_with_options(
            dataset,
            None,
            None,
            &OfflineVldaHarnessOptions {
                pid_mode: PidMode::CategoricalSx,
                categorical_bins: 6,
                pls: PlsComponentSelection::Fixed(2),
            },
        )
        .unwrap();

        assert_eq!(
            report
                .metrics
                .mi_v_action
                .outcome
                .scientific_gates
                .population,
            OfflineVldaScientificGateVerdict::NotEvaluated
        );
        assert_eq!(
            report.metrics.pid_pairs["VL"]
                .outcome
                .scientific_gates
                .population,
            OfflineVldaScientificGateVerdict::NotEvaluated
        );
        assert_eq!(
            report
                .metrics
                .mi_l_action
                .outcome
                .scientific_gates
                .population,
            OfflineVldaScientificGateVerdict::Conditional
        );
        assert_eq!(
            report
                .metrics
                .estimate_denominators
                .declared_support_compatible,
            3
        );
    }

    #[test]
    fn pid_disabled_mode_preserves_baselines_and_emits_no_pid_metrics() {
        let dataset = fixture_dataset();
        let report = run_offline_vlda_harness_with_options(
            dataset.clone(),
            Some("memory://fixture.json".to_string()),
            Some(TEST_INPUT_SHA256.to_string()),
            &OfflineVldaHarnessOptions {
                pid_mode: PidMode::Disabled,
                ..OfflineVldaHarnessOptions::default()
            },
        )
        .unwrap();

        assert_eq!(
            report.config["metric_pipeline"]["pid_functional"],
            "not_requested"
        );
        assert_eq!(
            report.config["metric_pipeline"]["pid_estimator"],
            "not_applicable"
        );
        assert_eq!(report.metrics.estimate_denominators.requested, 0);
        assert!(report.metrics.pid_pairs.is_empty());
        assert_eq!(
            report.metrics.mi_v_action.outcome.status,
            OfflineVldaEstimateStatus::NotRequested
        );
        assert!(report.metrics.mi_v_action.value.is_none());
        assert!(report.metrics.majority_success_accuracy.is_some());
        assert!(report
            .metrics
            .heldout_logreg_vlda_success_accuracy
            .is_some());
        assert_eq!(offline_vlda_train_split_pid_status(&report), "disabled");

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let runlog_path =
            std::env::temp_dir().join(format!("pid-offline-vlda-disabled-{stamp}.jsonl"));
        write_offline_vlda_runlog(&runlog_path, None, None, &dataset, &report).unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        assert!(validate_events(&events).unwrap().is_valid());
        assert!(!events
            .iter()
            .any(|event| matches!(event, RunLogEvent::PidMetric { .. })));
        assert!(events.iter().any(|event| matches!(
            event,
            RunLogEvent::EvaluationMetric { name, .. }
                if name == "offline_vlda.baseline.heldout_logreg_vlda_success_accuracy"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            RunLogEvent::LabelObserved { name, value, .. }
                if name == "offline_vlda.pid.train_split.status"
                    && value.get("status").and_then(Value::as_str) == Some("disabled")
        )));
        std::fs::remove_file(runlog_path).unwrap();
    }

    #[test]
    fn offline_vlda_harness_validates_and_summarizes() {
        let dataset = fixture_dataset();
        let report = run_offline_vlda_harness_with_options(
            dataset.clone(),
            Some("memory://fixture.json".to_string()),
            Some(TEST_INPUT_SHA256.to_string()),
            &continuous_options(),
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
        assert_eq!(report.heldout_predictions.len(), 48);
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
        assert_eq!(report.heldout_failure_diagnostics.len(), 12);
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
        assert_eq!(report.geometry.diagnostics.status, "warning");
        assert!(!report.geometry.diagnostics.warnings.is_empty());

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
        let validation = validate_events(&events).unwrap();
        assert!(validation.is_valid(), "{:?}", validation.issues);
        // This is the MIXED-SUPPORT regression fixture: `L` is declared categorical, so every
        // L-involving continuous term abstains, and `V`(2-d) vs `D`(1-d) is structurally
        // inapplicable to continuous shared exclusions (equal ambient source dimensions required).
        // No pair is produced here, so NO pid metric event may be emitted for any of them — an
        // abstained estimate has no numeric placeholder, not zero and not NaN.
        let emitted_pid_pair_metric = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::PidMetric { metadata, .. }
                    if metadata.contains_key("pid_pair")
            )
        });
        assert!(
            !emitted_pid_pair_metric,
            "abstained pairs must emit no pid metric events"
        );
        let produced_mi_metadata = events.iter().find_map(|event| match event {
            RunLogEvent::PidMetric { metadata, .. }
                if !metadata.contains_key("pid_pair")
                    && metadata.get("measure").map(String::as_str)
                        == Some(MEASURE_CONTINUOUS_MI) =>
            {
                Some(metadata)
            }
            _ => None,
        });
        assert!(
            produced_mi_metadata.is_none(),
            "per-axis declarations must not silently promote to a joint-law contract"
        );
        // The abstention itself is preserved in the run log, with its stable reason code.
        for pair in ["VL", "VD", "LD"] {
            let has_abstention = events.iter().any(|event| {
                matches!(
                    event,
                    pid_runlog::RunLogEvent::LabelObserved { name, .. }
                        if name == &format!("offline_vlda.pid.abstained.{pair}")
                )
            });
            assert!(has_abstention, "{pair} abstention missing from the run log");
        }
        let has_denominators = events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::LabelObserved { name, .. }
                    if name == "offline_vlda.pid.estimate_denominators"
            )
        });
        assert!(has_denominators);
        for name in [
            "offline_vlda.pid.abstained.V",
            "offline_vlda.pid.abstained.L",
            "offline_vlda.pid.abstained.D",
            "offline_vlda.pid.train_split.abstained.V",
            "offline_vlda.pid.train_split.abstained.L",
            "offline_vlda.pid.train_split.abstained.D",
            "offline_vlda.pid.train_split.abstained.VL",
            "offline_vlda.pid.train_split.abstained.VD",
            "offline_vlda.pid.train_split.abstained.LD",
            "offline_vlda.pid.train_split.estimate_denominators",
        ] {
            assert!(
                events.iter().any(|event| matches!(
                    event,
                    RunLogEvent::LabelObserved { name: observed, .. }
                        if observed == name
                )),
                "{name} missing from the run log"
            );
        }
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
        assert_eq!(heldout_prediction_correct_events, 48);
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
        assert_eq!(heldout_prediction_score_events, 24);
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
        // 16 success labels, plus the structured abstention records and the estimate denominators
        // (both `LabelObserved`, so that replay reconstructs the abstentions).
        let success_labels = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    pid_runlog::RunLogEvent::LabelObserved { name, .. }
                        if name == "offline_vlda.success"
                )
            })
            .count();
        assert_eq!(success_labels, 16);
        assert!(summary.labels > success_labels);
        // Per-axis continuity is insufficient for a joint KSG/Ehrlich assertion. This fixture
        // intentionally omits complete-tuple contracts, so every continuous request abstains.
        assert_eq!(summary.pid_metrics, 0);
        // `L` is binary: duplicate rows give a zero nearest-neighbor distance, so the current
        // pid-core review contract fails its geometry diagnostics closed (degenerate data /
        // ambiguous shell) and records the reason instead of emitting a number. 21 -> 19.
        assert!(summary.geometry_metrics >= 19);
        assert_eq!(summary.evaluation_metrics, 149);
        assert_eq!(summary.pid_metric_events, 0);
        assert!(summary.geometry_metric_events >= 19);
        assert_eq!(summary.evaluation_metric_events, 238);

        let _ = std::fs::remove_file(summary_path);
        let _ = std::fs::remove_file(runlog_path);
    }

    #[test]
    fn offline_vlda_runlog_timestamps_stay_monotonic_at_capture_scale() {
        // A real VLA capture emits roughly two dozen metric events per labeled held-out
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

        let metric_timestamp_base = 10_000_u64;
        let mut writer = RunLogWriter::new(Vec::new());
        let metric_events =
            write_metric_events(&mut writer, &report, metric_timestamp_base).unwrap();
        assert!(metric_events > 10_000);
        writer
            .append(&RunLogEvent::RunEnded {
                run_id: report.run_id.clone(),
                timestamp_ns: metric_timestamp_base + metric_events,
                status: RunStatus::Succeeded,
                message: None,
            })
            .unwrap();
        writer.flush().unwrap();
        let timestamps = writer
            .into_inner()
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| {
                serde_json::from_slice::<Value>(line).unwrap()["timestamp_ns"]
                    .as_u64()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(timestamps.windows(2).all(|pair| pair[0] <= pair[1]));
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
            Some(TEST_INPUT_SHA256.to_string()),
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
                require_success_labels: false,
                require_heldout_split: false,
                require_heldout_class_coverage: true,
                require_heldout_episode_disjoint: false,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events).unwrap();
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
            Some(TEST_INPUT_SHA256.to_string()),
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
                require_success_labels: false,
                require_heldout_split: false,
                require_heldout_class_coverage: false,
                require_heldout_episode_disjoint: true,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events).unwrap();
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
    fn offline_vlda_strict_split_rejects_explicit_scientific_block() {
        let mut dataset = fixture_dataset();
        for sample in &mut dataset.samples {
            sample.metadata.insert(
                OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_KEY.to_string(),
                OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_BLOCKED.to_string(),
            );
        }
        let report = run_offline_vlda_harness(
            dataset.clone(),
            Some("memory://fixture.json".to_string()),
            Some(TEST_INPUT_SHA256.to_string()),
        )
        .unwrap();
        assert!(report.metrics.heldout_majority_success_accuracy.is_some());

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let runlog_path = std::env::temp_dir().join(format!(
            "pid-offline-vlda-strict-scientific-split-{stamp}.jsonl"
        ));
        write_offline_vlda_runlog_with_options(
            &runlog_path,
            None,
            None,
            &dataset,
            &report,
            OfflineVldaRunlogOptions {
                require_success_labels: false,
                require_heldout_split: true,
                require_heldout_class_coverage: false,
                require_heldout_episode_disjoint: false,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();

        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events).unwrap();
        assert!(validation.is_valid(), "{:?}", validation.issues);
        assert!(events.iter().any(|event| {
            matches!(
                event,
                pid_runlog::RunLogEvent::ErrorLogged { message, recoverable, .. }
                    if !recoverable && message.contains("split scientific eligibility blocked")
            )
        }));
        assert_eq!(
            summarize_events(&events).unwrap().status,
            Some(RunStatus::Failed)
        );

        let _ = std::fs::remove_file(runlog_path);
    }

    #[test]
    fn offline_vlda_strict_split_rejects_unknown_or_mixed_eligibility() {
        let mut unknown = fixture_dataset();
        unknown.samples[0].metadata.insert(
            OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_KEY.to_string(),
            "typo".to_string(),
        );
        assert!(
            offline_vlda_split_scientific_eligibility_failure_message(&unknown)
                .unwrap()
                .contains("eligibility invalid")
        );

        let mut mixed = fixture_dataset();
        mixed.samples[0].metadata.insert(
            OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_KEY.to_string(),
            "structural_split_ready".to_string(),
        );
        mixed.samples[1].metadata.insert(
            OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_KEY.to_string(),
            OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_BLOCKED.to_string(),
        );
        assert!(
            offline_vlda_split_scientific_eligibility_failure_message(&mixed)
                .unwrap()
                .contains("eligibility inconsistent")
        );

        let mut partial = fixture_dataset();
        partial.samples[0].metadata.insert(
            OFFLINE_SPLIT_SCIENTIFIC_ELIGIBILITY_KEY.to_string(),
            "structural_split_ready".to_string(),
        );
        assert!(
            offline_vlda_split_scientific_eligibility_failure_message(&partial)
                .unwrap()
                .contains("eligibility incomplete")
        );
    }

    #[test]
    fn offline_vlda_checked_fixture_runs() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/offline_vlda_fixture.json");
        let dataset = read_offline_vlda_dataset(&path).unwrap();
        let input_sha256 = pid_runlog::sha256_file(&path).unwrap();
        let report = run_offline_vlda_harness_with_options(
            dataset,
            Some(path.display().to_string()),
            Some(input_sha256),
            &continuous_options(),
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
        assert_eq!(report.heldout_failure_diagnostics.len(), 12);
        assert_eq!(report.train_split_pid.as_ref().unwrap().status, "available");
        assert!(report.metrics.pid_pairs.contains_key("LD"));
        assert_eq!(report.geometry.variables.len(), 6);
        assert_eq!(report.geometry.diagnostics.status, "warning");
    }

    #[test]
    fn offline_vlda_train_split_pid_excludes_heldout_samples() {
        let dataset = fixture_dataset();
        let base_report = run_offline_vlda_harness_with_options(
            dataset,
            Some("memory://fixture.json".to_string()),
            Some(TEST_INPUT_SHA256.to_string()),
            &continuous_options(),
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
        let changed_report = run_offline_vlda_harness_with_options(
            changed_heldout,
            Some("memory://fixture.json".to_string()),
            Some(TEST_INPUT_SHA256.to_string()),
            &continuous_options(),
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
            Some(TEST_INPUT_SHA256.to_string()),
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
            Some(TEST_INPUT_SHA256.to_string()),
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
        assert_eq!(report.heldout_predictions.len(), 48);
        assert_eq!(report.heldout_failure_diagnostics.len(), 12);
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
            Some(TEST_INPUT_SHA256.to_string()),
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
                require_success_labels: true,
                require_heldout_split: false,
                require_heldout_class_coverage: false,
                require_heldout_episode_disjoint: false,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events).unwrap();
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
            Some(TEST_INPUT_SHA256.to_string()),
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
                require_success_labels: false,
                require_heldout_split: true,
                require_heldout_class_coverage: false,
                require_heldout_episode_disjoint: false,
                require_axis_provenance_honest: false,
            },
        )
        .unwrap();
        let events = read_events_from_path(&runlog_path).unwrap();
        let validation = validate_events(&events).unwrap();
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
    fn uncertainty_tail_mass_rejects_zero_in_requests_and_artifacts() {
        let dataset = fixture_dataset();
        let invalid = OfflineVldaUncertaintyConfig {
            alpha: 0.0,
            ..OfflineVldaUncertaintyConfig::default()
        };
        let request_error =
            compute_offline_pid_uncertainty(&dataset, PidMode::Disabled, &invalid).unwrap_err();
        assert!(request_error.to_string().contains("strictly inside"));

        let mut artifact = compute_offline_pid_uncertainty(
            &dataset,
            PidMode::Disabled,
            &OfflineVldaUncertaintyConfig::default(),
        )
        .unwrap();
        artifact.alpha = 0.0;
        let artifact_error = validate_offline_pid_uncertainty(&artifact).unwrap_err();
        assert!(artifact_error.to_string().contains("strictly inside"));
    }

    #[test]
    fn uncertainty_rejects_incoherent_combined_row_assumptions() {
        for config in [
            OfflineVldaUncertaintyConfig {
                n_boot: 8,
                n_perm: 8,
                block_size: 2,
                permutation_scheme: PermutationScheme::FullShuffle,
                ..OfflineVldaUncertaintyConfig::default()
            },
            OfflineVldaUncertaintyConfig {
                n_boot: 8,
                n_perm: 8,
                block_size: 1,
                permutation_scheme: PermutationScheme::CircularShift { min_shift: 1 },
                ..OfflineVldaUncertaintyConfig::default()
            },
            OfflineVldaUncertaintyConfig {
                n_boot: 8,
                n_perm: 8,
                block_size: 2,
                permutation_scheme: PermutationScheme::BlockShuffle { block_size: 4 },
                ..OfflineVldaUncertaintyConfig::default()
            },
        ] {
            let error = validate_uncertainty_config(&config).unwrap_err();
            assert!(
                error.to_string().contains("incompatible row assumptions")
                    || error
                        .to_string()
                        .contains("different bootstrap and block-shuffle")
            );
        }

        for config in [
            OfflineVldaUncertaintyConfig {
                n_boot: 8,
                n_perm: 8,
                block_size: 1,
                permutation_scheme: PermutationScheme::FullShuffle,
                ..OfflineVldaUncertaintyConfig::default()
            },
            OfflineVldaUncertaintyConfig {
                n_boot: 8,
                n_perm: 8,
                block_size: 2,
                permutation_scheme: PermutationScheme::CircularShift { min_shift: 2 },
                ..OfflineVldaUncertaintyConfig::default()
            },
            OfflineVldaUncertaintyConfig {
                n_boot: 8,
                n_perm: 8,
                block_size: 2,
                permutation_scheme: PermutationScheme::BlockShuffle { block_size: 2 },
                ..OfflineVldaUncertaintyConfig::default()
            },
        ] {
            validate_uncertainty_config(&config).unwrap();
        }
    }

    #[test]
    fn permutation_calibration_names_the_exact_exchangeability_unit() {
        assert_eq!(
            permutation_calibration_label(PermutationScheme::FullShuffle, 1).unwrap(),
            "monte_carlo_p_value_under_declared_row_exchangeability"
        );
        assert_eq!(
            permutation_calibration_label(PermutationScheme::BlockShuffle { block_size: 2 }, 1,)
                .unwrap(),
            "monte_carlo_p_value_under_declared_whole_block_exchangeability"
        );
        assert_eq!(
            permutation_calibration_label(PermutationScheme::CircularShift { min_shift: 1 }, 1)
                .unwrap(),
            "approximate_stationary_surrogate_score_not_p_value"
        );
        assert_eq!(
            permutation_calibration_label(PermutationScheme::FullShuffle, 0).unwrap(),
            "not_requested"
        );
    }

    #[test]
    fn pid_uncertainty_continuous_emits_stability_envelopes_and_null_tail_fractions() {
        let dataset = as_single_ordered_episode(continuous_fixture_dataset());
        let cfg = OfflineVldaUncertaintyConfig {
            n_boot: 24,
            n_perm: 40,
            block_size: 1,
            alpha: 0.05,
            seed: 7,
            permutation_scheme: PermutationScheme::FullShuffle,
        };
        let u = compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &cfg).unwrap();
        assert_eq!(u.mode, "continuous");
        assert_eq!(
            u.stability_interpretation,
            RAW_M_SAMPLE_STABILITY_INTERPRETATION
        );
        assert_eq!(
            u.preprocessing_resampling,
            OFFLINE_UNCERTAINTY_PREPROCESSING_RESAMPLING
        );
        assert_eq!(u.pairs.len(), 3);
        assert!(u.subsample_len >= 1);
        assert_eq!(u.permutation_scheme, "full_shuffle");
        assert_eq!(
            u.permutation_calibration,
            "monte_carlo_p_value_under_declared_row_exchangeability"
        );
        assert_eq!(u.row_topology, "single_ordered_episode");
        // Deterministic given the same config.
        let u2 = compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &cfg).unwrap();
        assert_eq!(u, u2);
        let vl = u.pairs.iter().find(|p| p.pair == "VL").unwrap();
        // Raw m-sample stability percentiles are present and ordered (n_boot > 0).
        let red = vl.redundancy.as_ref().unwrap();
        assert!(red.m_sample_percentile_lower <= red.m_sample_percentile_upper);
        assert_eq!(red.n_valid, cfg.n_boot);
        // Subsample-bias diagnostic: the m-out-of-n center is exposed alongside
        // the point estimate, and the precomputed gap is exactly their difference.
        let boot_mean = red.boot_mean.expect("boot_mean present on new artifacts");
        let gap = red.bias_vs_point.expect("bias_vs_point present");
        assert!((gap - (boot_mean - red.point)).abs() < 1e-12);
        assert!(vl.synergy.is_some() && vl.unique_s1.is_some() && vl.unique_s2.is_some());
        // Null tail fractions are present and bounded (n_perm > 0).
        let p1 = vl.unique_s1_tail_fraction.unwrap();
        let p2 = vl.unique_s2_tail_fraction.unwrap();
        assert!((0.0..=1.0).contains(&p1) && (0.0..=1.0).contains(&p2));
        assert!(vl.perm_n_valid_s1 > 0 && vl.perm_n_valid_s2 > 0);

        let serialized = serde_json::to_value(&u).unwrap();
        assert_eq!(
            serialized["stability_interpretation"],
            RAW_M_SAMPLE_STABILITY_INTERPRETATION
        );
        let serialized_red = &serialized["pairs"][0]["redundancy"];
        assert!(serialized_red.get("m_sample_percentile_lower").is_some());
        assert!(serialized_red.get("m_sample_percentile_upper").is_some());
        assert!(serialized_red.get("ci_low").is_none());
        assert!(serialized_red.get("ci_high").is_none());
    }

    #[test]
    fn atom_stability_envelope_reads_legacy_ci_endpoint_names_but_never_reemits_them() {
        let legacy = serde_json::json!({
            "point": 0.75,
            "ci_low": 0.5,
            "ci_high": 1.0,
            "n_valid": 20,
            "boot_mean": 0.8,
            "bias_vs_point": 0.050000000000000044
        });
        let envelope: OfflineVldaAtomStabilityEnvelope = serde_json::from_value(legacy).unwrap();
        assert_eq!(envelope.m_sample_percentile_lower, 0.5);
        assert_eq!(envelope.m_sample_percentile_upper, 1.0);

        let serialized = serde_json::to_value(envelope).unwrap();
        assert_eq!(serialized["m_sample_percentile_lower"], 0.5);
        assert_eq!(serialized["m_sample_percentile_upper"], 1.0);
        assert!(serialized.get("ci_low").is_none());
        assert!(serialized.get("ci_high").is_none());
    }

    #[test]
    fn pid_uncertainty_skips_non_continuous_measures() {
        let dataset = fixture_dataset();
        let cfg = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            n_perm: 0,
            ..Default::default()
        };
        let u = compute_offline_pid_uncertainty(&dataset, PidMode::CategoricalSx, &cfg).unwrap();
        assert!(u.mode.starts_with("skipped"), "mode={}", u.mode);
        assert!(u.pairs.is_empty());
    }

    #[test]
    fn pid_uncertainty_fails_closed_on_multiple_dependent_episodes() {
        let dataset = continuous_fixture_dataset();
        let options = continuous_options();
        let config = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            n_perm: 8,
            ..Default::default()
        };

        let invocation = run_offline_vlda_invocation_borrowed_with_options_and_limits(
            &dataset,
            None,
            None,
            &options,
            &config,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap();
        let uncertainty = invocation.uncertainty.expect("typed skip is published");

        assert_eq!(uncertainty.mode, UNSUPPORTED_UNCERTAINTY_TOPOLOGY_MODE);
        assert_eq!(
            uncertainty.row_topology,
            "multiple_episodes_with_repeated_rows"
        );
        assert!(uncertainty.pairs.is_empty());
        assert_eq!(uncertainty.subsample_len, 0);
        assert_eq!(
            invocation.report.config["uncertainty_request"]["execution"],
            "typed_skip_episode_aware_resampling_required"
        );
        assert_eq!(
            invocation.report.config["resource_usage"]
                ["projected_uncertainty_pairwise_distance_evaluations"],
            0
        );
    }

    #[test]
    fn pid_uncertainty_requires_an_identified_series_for_circular_shift() {
        let mut dataset = continuous_fixture_dataset();
        for sample in &mut dataset.samples {
            sample.episode_id = None;
        }
        let config = OfflineVldaUncertaintyConfig {
            n_perm: 8,
            permutation_scheme: PermutationScheme::CircularShift { min_shift: 2 },
            ..Default::default()
        };

        let uncertainty =
            compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &config).unwrap();

        assert_eq!(uncertainty.mode, UNSUPPORTED_UNCERTAINTY_TOPOLOGY_MODE);
        assert_eq!(uncertainty.row_topology, "row_order_without_episode_ids");
        assert!(uncertainty.pairs.is_empty());
        assert_eq!(uncertainty.subsample_len, 0);
    }

    #[test]
    fn pid_uncertainty_requires_a_sequence_receipt_for_one_episode() {
        let mut dataset = continuous_fixture_dataset();
        for sample in &mut dataset.samples {
            sample.episode_id = Some("one-episode".to_string());
            sample.metadata.remove("sequence_index");
        }
        let config = OfflineVldaUncertaintyConfig {
            n_perm: 8,
            permutation_scheme: PermutationScheme::CircularShift { min_shift: 2 },
            ..Default::default()
        };

        let uncertainty =
            compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &config).unwrap();

        assert_eq!(uncertainty.mode, UNSUPPORTED_UNCERTAINTY_TOPOLOGY_MODE);
        assert_eq!(
            uncertainty.row_topology,
            "single_episode_without_verified_order"
        );
        assert!(uncertainty.pairs.is_empty());
    }

    #[test]
    fn pid_uncertainty_records_application_block_for_produced_pairs() {
        let config = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            ..Default::default()
        };
        let uncertainty = compute_offline_pid_uncertainty(
            &as_single_ordered_episode(continuous_fixture_dataset()),
            PidMode::Continuous,
            &config,
        )
        .unwrap();
        let pair = uncertainty
            .pairs
            .iter()
            .find(|pair| pair.status == OfflineVldaEstimateStatus::Produced)
            .expect("continuous fixture should produce at least one pair");

        assert_eq!(
            pair.scientific_gates.population,
            OfflineVldaScientificGateVerdict::Conditional
        );
        assert_eq!(
            pair.scientific_gates.application,
            OfflineVldaScientificGateVerdict::Blocked
        );
        assert!(!pair.scientific_gates.interpretation_allowed);
    }

    #[test]
    fn pid_uncertainty_records_measure_block_for_support_abstention() {
        let config = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            ..Default::default()
        };
        let uncertainty = compute_offline_pid_uncertainty(
            &as_single_ordered_episode(fixture_dataset()),
            PidMode::Continuous,
            &config,
        )
        .unwrap();
        let pair = uncertainty
            .pairs
            .iter()
            .find(|pair| pair.pair == "VL")
            .expect("mixed-support fixture carries the VL request");

        assert_eq!(pair.status, OfflineVldaEstimateStatus::Abstained);
        assert_eq!(
            pair.reason_code,
            Some(OfflineVldaAbstainReason::DeclaredSupportIncompatibleContinuous)
        );
        assert_eq!(
            pair.scientific_gates.measure,
            OfflineVldaScientificGateVerdict::Blocked
        );
        assert_eq!(
            pair.scientific_gates.application,
            OfflineVldaScientificGateVerdict::Blocked
        );
        assert!(!pair.scientific_gates.interpretation_allowed);
    }

    #[test]
    fn pid_uncertainty_bootstrap_only_omits_null_tail_fractions() {
        let dataset = as_single_ordered_episode(continuous_fixture_dataset());
        let cfg = OfflineVldaUncertaintyConfig {
            n_boot: 24,
            n_perm: 0,
            block_size: 1,
            alpha: 0.05,
            seed: 7,
            permutation_scheme: PermutationScheme::FullShuffle,
        };
        let u = compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &cfg).unwrap();
        validate_offline_pid_uncertainty(&u).unwrap();
        let vl = &u.pairs[0];
        assert_eq!(vl.status, OfflineVldaEstimateStatus::Produced);
        assert!(vl.redundancy.is_some());
        assert!(vl.unique_s1_tail_fraction.is_none() && vl.unique_s2_tail_fraction.is_none());
        assert!(!OfflineVldaUncertaintyConfig::default().enabled());
    }

    #[test]
    fn pid_uncertainty_disabled_is_typed_skip_without_pair_placeholders() {
        let uncertainty = compute_offline_pid_uncertainty(
            &continuous_fixture_dataset(),
            PidMode::Continuous,
            &OfflineVldaUncertaintyConfig::default(),
        )
        .unwrap();

        assert_eq!(uncertainty.mode, "skipped:no_uncertainty_requested");
        assert!(uncertainty.pairs.is_empty());
        assert_eq!(uncertainty.subsample_len, 0);
        assert_eq!(uncertainty.resample_scheme, "not_requested");
        assert_eq!(uncertainty.permutation_scheme, "not_requested");
    }

    #[test]
    fn pid_uncertainty_typed_skips_still_enforce_decoded_resource_limits() {
        let dataset = fixture_dataset();
        let limits = OfflineVldaResourceLimits {
            max_samples: dataset.samples.len() - 1,
            ..OfflineVldaResourceLimits::default()
        };

        for (mode, config) in [
            (PidMode::Continuous, OfflineVldaUncertaintyConfig::default()),
            (
                PidMode::CategoricalSx,
                OfflineVldaUncertaintyConfig {
                    n_perm: 1,
                    ..OfflineVldaUncertaintyConfig::default()
                },
            ),
        ] {
            let error =
                compute_offline_pid_uncertainty_with_limits(&dataset, mode, &config, &limits)
                    .unwrap_err();
            assert!(error
                .to_string()
                .contains("resource limit exceeded for samples"));
        }
    }

    #[test]
    fn uncertainty_publication_rejects_produced_status_without_requested_values() {
        let config = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            ..Default::default()
        };
        let mut uncertainty = compute_offline_pid_uncertainty(
            &as_single_ordered_episode(continuous_fixture_dataset()),
            PidMode::Continuous,
            &config,
        )
        .unwrap();
        let pair = uncertainty
            .pairs
            .iter_mut()
            .find(|pair| pair.status == OfflineVldaEstimateStatus::Produced)
            .unwrap();
        pair.redundancy = None;
        pair.unique_s1 = None;
        pair.unique_s2 = None;
        pair.synergy = None;

        let path = std::env::temp_dir().join(format!(
            "prisoma-invalid-uncertainty-{}-{}.json",
            std::process::id(),
            config.seed
        ));
        let _ = std::fs::remove_file(&path);
        let error = write_offline_pid_uncertainty(&path, &uncertainty).unwrap_err();
        assert!(
            format!("{error:#}").contains("inconsistent with numeric-value presence=false"),
            "{error:#}"
        );
        assert!(!path.exists());
    }

    #[test]
    fn uncertainty_publication_rejects_ambiguous_stability_scope() {
        let config = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            ..Default::default()
        };
        let mut uncertainty = compute_offline_pid_uncertainty(
            &as_single_ordered_episode(continuous_fixture_dataset()),
            PidMode::Continuous,
            &config,
        )
        .unwrap();
        uncertainty.stability_interpretation = "confidence_interval".to_string();

        let path = std::env::temp_dir().join(format!(
            "prisoma-ambiguous-stability-{}-{}.json",
            std::process::id(),
            config.seed
        ));
        let _ = std::fs::remove_file(&path);
        let error = write_offline_pid_uncertainty(&path, &uncertainty).unwrap_err();
        assert!(
            format!("{error:#}").contains(RAW_M_SAMPLE_STABILITY_INTERPRETATION),
            "{error:#}"
        );
        assert!(!path.exists());

        uncertainty.stability_interpretation = RAW_M_SAMPLE_STABILITY_INTERPRETATION.to_string();
        uncertainty.preprocessing_resampling = "nested_refit".to_string();
        let error = write_offline_pid_uncertainty(&path, &uncertainty).unwrap_err();
        assert!(
            format!("{error:#}").contains(OFFLINE_UNCERTAINTY_PREPROCESSING_RESAMPLING),
            "{error:#}"
        );
        assert!(!path.exists());
    }

    #[test]
    fn uncertainty_validation_binds_tail_presence_to_complete_permutation_counts() {
        let config = OfflineVldaUncertaintyConfig {
            n_perm: 8,
            ..Default::default()
        };
        let uncertainty = compute_offline_pid_uncertainty(
            &as_single_ordered_episode(continuous_fixture_dataset()),
            PidMode::Continuous,
            &config,
        )
        .unwrap();

        let mut partial_with_value = uncertainty.clone();
        let pair = partial_with_value
            .pairs
            .iter_mut()
            .find(|pair| pair.unique_s1_tail_fraction.is_some())
            .unwrap();
        pair.perm_n_valid_s1 -= 1;
        let error = validate_offline_pid_uncertainty(&partial_with_value).unwrap_err();
        assert!(error
            .to_string()
            .contains("exists without every requested permutation"));

        let mut complete_without_value = uncertainty;
        let pair = complete_without_value
            .pairs
            .iter_mut()
            .find(|pair| pair.unique_s1_tail_fraction.is_some())
            .unwrap();
        pair.unique_s1_tail_fraction = None;
        pair.status = OfflineVldaEstimateStatus::ProducedWithWarning;
        pair.warning_codes =
            vec![OfflineVldaUncertaintyWarning::UniqueSource1PermutationUnavailable];
        pair.scientific_gates.estimator = OfflineVldaScientificGateVerdict::Blocked;
        pair.scientific_gates.reason_code =
            Some("uncertainty_statistics_partially_unavailable".to_string());
        pair.reason_detail = Some(
            "some requested uncertainty components were unavailable: unique_source_1_permutation_unavailable"
                .to_string(),
        );
        let error = validate_offline_pid_uncertainty(&complete_without_value).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("absent despite every requested permutation being valid"),
            "{error:#}"
        );
    }

    #[test]
    fn uncertainty_validation_requires_every_requested_bootstrap_replicate() {
        let config = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            ..Default::default()
        };
        let mut uncertainty = compute_offline_pid_uncertainty(
            &as_single_ordered_episode(continuous_fixture_dataset()),
            PidMode::Continuous,
            &config,
        )
        .unwrap();
        let atom = uncertainty
            .pairs
            .iter_mut()
            .find_map(|pair| pair.redundancy.as_mut())
            .expect("continuous fixture should produce a stability envelope");
        atom.n_valid -= 1;

        let error = validate_offline_pid_uncertainty(&uncertainty).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("exists without every requested resample"),
            "{error:#}"
        );
    }

    #[test]
    fn runlog_publication_rejects_uncertainty_point_that_disagrees_with_report() {
        let dataset = as_single_ordered_episode(continuous_fixture_dataset());
        let options = continuous_options();
        let config = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            ..Default::default()
        };
        let report = run_offline_vlda_harness_borrowed_with_options_and_invocation_limits(
            &dataset,
            None,
            None,
            &options,
            &config,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap();
        let mut uncertainty =
            compute_offline_pid_uncertainty(&dataset, options.pid_mode, &config).unwrap();
        let atom = uncertainty
            .pairs
            .iter_mut()
            .find_map(|pair| pair.redundancy.as_mut())
            .expect("continuous fixture should produce a stability envelope");
        atom.point += 1.0;
        atom.bias_vs_point = Some(atom.boot_mean.unwrap() - atom.point);
        validate_offline_pid_uncertainty(&uncertainty).unwrap();

        let directory = tempfile::tempdir().unwrap();
        let uncertainty_path = directory.path().join("forged-point.json");
        let runlog_path = directory.path().join("forged-point.jsonl");
        write_offline_pid_uncertainty(&uncertainty_path, &uncertainty).unwrap();

        let error = write_offline_vlda_runlog_with_options_and_uncertainty(
            &runlog_path,
            OfflineVldaRunlogArtifacts {
                uncertainty_path: Some(&uncertainty_path),
                uncertainty: Some(&uncertainty),
                ..OfflineVldaRunlogArtifacts::default()
            },
            &dataset,
            &report,
            OfflineVldaRunlogOptions::default(),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("point does not match the main report"),
            "{error:#}"
        );
        assert!(!runlog_path.exists());
    }

    #[test]
    fn runlog_publication_binds_disabled_uncertainty_topology_to_dataset() {
        let dataset = continuous_fixture_dataset();
        let options = continuous_options();
        let config = OfflineVldaUncertaintyConfig::default();
        let mut report = run_offline_vlda_harness_borrowed_with_options_and_invocation_limits(
            &dataset,
            None,
            None,
            &options,
            &config,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap();
        let expected = uncertainty_row_topology(&dataset.samples);
        let forged = if expected == OfflineVldaUncertaintyRowTopology::RowOrderWithoutEpisodeIds {
            OfflineVldaUncertaintyRowTopology::SingleOrderedEpisode
        } else {
            OfflineVldaUncertaintyRowTopology::RowOrderWithoutEpisodeIds
        };
        report.config["uncertainty_request"]["row_topology"] = json!(forged.label());
        report.config_hash = pid_runlog::canonical_json_hash_v2(&report.config).unwrap();
        report.analysis_seal =
            OfflineVldaAnalysisSeal(Some(offline_vlda_report_analysis_seal(&report).unwrap()));

        let directory = tempfile::tempdir().unwrap();
        let runlog_path = directory.path().join("forged-topology.jsonl");
        let error = write_offline_vlda_runlog_with_options_and_uncertainty(
            &runlog_path,
            OfflineVldaRunlogArtifacts::default(),
            &dataset,
            &report,
            OfflineVldaRunlogOptions::default(),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("does not match the publication dataset row topology"),
            "{error:#}"
        );
        assert!(!runlog_path.exists());
    }

    #[test]
    fn uncertainty_skip_rejects_forged_mode_and_scheme_provenance() {
        let config = OfflineVldaUncertaintyConfig {
            n_boot: 2,
            ..Default::default()
        };
        let uncertainty = compute_offline_pid_uncertainty(
            &continuous_fixture_dataset(),
            PidMode::CategoricalSx,
            &config,
        )
        .unwrap();
        assert_eq!(uncertainty.pid_mode, PidMode::CategoricalSx);

        let mut forged_mode = uncertainty.clone();
        forged_mode.pid_mode = PidMode::Continuous;
        let error = validate_offline_pid_uncertainty(&forged_mode).unwrap_err();
        assert!(error.to_string().contains("unknown skip reason"));

        let mut forged_scheme = uncertainty;
        forged_scheme.resample_scheme = "bootstrap_with_replacement".to_string();
        let error = validate_offline_pid_uncertainty(&forged_scheme).unwrap_err();
        assert!(error.to_string().contains("wrong resampling scheme"));
    }

    #[test]
    fn uncertainty_sidecar_round_trips_exactly_and_rejects_forged_subsample_length() {
        let dataset = as_single_ordered_episode(continuous_fixture_dataset());
        let options = continuous_options();
        let config = OfflineVldaUncertaintyConfig {
            n_boot: 8,
            ..Default::default()
        };
        let report = run_offline_vlda_harness_borrowed_with_options_and_invocation_limits(
            &dataset,
            None,
            None,
            &options,
            &config,
            &OfflineVldaResourceLimits::default(),
        )
        .unwrap();
        let uncertainty =
            compute_offline_pid_uncertainty(&dataset, options.pid_mode, &config).unwrap();

        let directory = tempfile::tempdir().unwrap();
        let uncertainty_path = directory.path().join("uncertainty.json");
        let runlog_path = directory.path().join("uncertainty.jsonl");
        write_offline_pid_uncertainty(&uncertainty_path, &uncertainty).unwrap();
        let recorded_bytes = std::fs::read(&uncertainty_path).unwrap();
        assert_eq!(
            recorded_bytes,
            serde_json::to_vec_pretty(&uncertainty).unwrap()
        );
        let round_tripped: OfflineVldaPidUncertainty =
            serde_json::from_slice(&recorded_bytes).unwrap();
        assert_eq!(round_tripped, uncertainty);
        write_offline_vlda_runlog_with_options_and_uncertainty(
            &runlog_path,
            OfflineVldaRunlogArtifacts {
                uncertainty_path: Some(&uncertainty_path),
                uncertainty: Some(&uncertainty),
                ..OfflineVldaRunlogArtifacts::default()
            },
            &dataset,
            &report,
            OfflineVldaRunlogOptions::default(),
        )
        .unwrap();
        assert_eq!(
            pid_runlog::validate_events(&pid_runlog::read_events_from_path(&runlog_path).unwrap())
                .unwrap()
                .errors,
            0
        );

        let mut forged = uncertainty;
        forged.subsample_len -= forged.block_size;
        validate_offline_pid_uncertainty(&forged).unwrap();
        let forged_path = directory.path().join("forged-uncertainty.json");
        let forged_runlog_path = directory.path().join("forged-uncertainty.jsonl");
        write_offline_pid_uncertainty(&forged_path, &forged).unwrap();

        let error = write_offline_vlda_runlog_with_options_and_uncertainty(
            &forged_runlog_path,
            OfflineVldaRunlogArtifacts {
                uncertainty_path: Some(&forged_path),
                uncertainty: Some(&forged),
                ..OfflineVldaRunlogArtifacts::default()
            },
            &dataset,
            &report,
            OfflineVldaRunlogOptions::default(),
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("uncertainty artifact does not match the report request"),
            "unexpected error: {error:#}"
        );
        assert!(!forged_runlog_path.exists());
    }

    #[test]
    fn pid_uncertainty_circular_shift_surrogate_is_supported_and_recorded() {
        // A circular shift preserves one source series up to its wrap seam. The restricted shifts
        // form a surrogate distribution, not a randomization-test p-value. The fixture has n = 48
        // rows, so min_shift = 4 leaves 41 admissible offsets.
        let dataset = as_single_ordered_episode(continuous_fixture_dataset());
        let cfg = OfflineVldaUncertaintyConfig {
            n_perm: 40,
            permutation_scheme: PermutationScheme::CircularShift { min_shift: 4 },
            ..Default::default()
        };
        let u = compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &cfg).unwrap();
        assert_eq!(u.mode, "continuous");
        assert_eq!(u.permutation_scheme, "circular_shift(min_shift=4)");
        assert_eq!(
            u.permutation_calibration,
            "approximate_stationary_surrogate_score_not_p_value"
        );
        let vl = u.pairs.iter().find(|p| p.pair == "VL").unwrap();
        let p1 = vl.unique_s1_tail_fraction.unwrap();
        let p2 = vl.unique_s2_tail_fraction.unwrap();
        assert!((0.0..=1.0).contains(&p1) && (0.0..=1.0).contains(&p2));
        assert!(vl.perm_n_valid_s1 > 0 && vl.perm_n_valid_s2 > 0);
        // Deterministic given the same config.
        let u2 = compute_offline_pid_uncertainty(&dataset, PidMode::Continuous, &cfg).unwrap();
        assert_eq!(u, u2);
    }
}
