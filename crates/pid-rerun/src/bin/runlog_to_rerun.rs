use anyhow::{bail, ensure, Context, Result};
use pid_rerun::{init_recording, prepare_new_recording_path, save_recording, RunLogRerunLogger};
use pid_runlog::{
    HashIdentity, HashRevision, RunLogEvent, RunLogHashIdentities, RunLogLimits, RunManifest,
};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

#[derive(Debug, PartialEq, Eq)]
struct ConverterOptions {
    input: PathBuf,
    save_path: Option<String>,
    serve: bool,
    allow_invalid: bool,
    load_attribution_artifacts: bool,
}

#[derive(Debug)]
struct PreparedRunLog {
    events: Vec<RunLogEvent>,
    manifest: RunManifest,
}

#[derive(Serialize)]
struct SnapshotArtifactManifestWire {
    name: String,
    kind: String,
    uri: String,
    sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_hash: Option<HashIdentity>,
}

#[derive(Serialize)]
struct SnapshotManifestWire {
    sidecar_schema_version: u32,
    schema_version: u32,
    run_id: Option<String>,
    config_hash: Option<String>,
    run_log_uri: String,
    run_log_sha256: Option<String>,
    run_log_hash: Option<HashIdentity>,
    trace_hash: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    trace_hash_v2: String,
    logical_trace_hash: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    logical_trace_hash_v3: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hash_identities: Option<RunLogHashIdentities>,
    event_count: usize,
    validation_errors: usize,
    validation_warnings: usize,
    artifacts: Vec<SnapshotArtifactManifestWire>,
}

fn read_bounded_snapshot(path: &Path, limits: RunLogLimits) -> Result<Vec<u8>> {
    let path_metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect run log {}", path.display()))?;
    ensure!(
        !path_metadata.file_type().is_symlink() && path_metadata.is_file(),
        "run-log input must be a non-symlink regular file: {}",
        path.display()
    );

    let mut options = OpenOptions::new();
    options.read(true);
    // A path replacement between the metadata check and open must not turn a
    // converter invocation into a blocking FIFO read or a symlink traversal.
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let file = options
        .open(path)
        .with_context(|| format!("failed to open run log {}", path.display()))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("failed to stat open run log {}", path.display()))?;
    ensure!(
        metadata.is_file(),
        "opened run-log input is not a regular file: {}",
        path.display()
    );
    ensure!(
        metadata.len() <= limits.max_file_bytes,
        "run-log file bytes exceed resource limit: requested {}, limit {}",
        metadata.len(),
        limits.max_file_bytes
    );

    let initial_capacity = usize::try_from(metadata.len())
        .context("run-log file length is not representable in memory")?;
    let mut snapshot = Vec::new();
    snapshot
        .try_reserve_exact(initial_capacity)
        .context("failed to reserve bounded run-log snapshot")?;
    file.take(limits.max_file_bytes.saturating_add(1))
        .read_to_end(&mut snapshot)
        .with_context(|| format!("failed to read run log {}", path.display()))?;
    let snapshot_len = u64::try_from(snapshot.len())
        .context("run-log snapshot length is not representable as u64")?;
    ensure!(
        snapshot_len <= limits.max_file_bytes,
        "run-log file bytes exceed resource limit: requested {snapshot_len}, limit {}",
        limits.max_file_bytes
    );
    Ok(snapshot)
}

fn manifest_for_snapshot(
    path: &Path,
    snapshot: &[u8],
    events: &[RunLogEvent],
    limits: RunLogLimits,
) -> Result<RunManifest> {
    let snapshot_len = u64::try_from(snapshot.len())
        .context("run-log snapshot length is not representable as u64")?;
    ensure!(
        snapshot_len <= limits.max_file_bytes,
        "run-log snapshot exceeds the configured file-byte limit"
    );
    let run_log_uri = path
        .to_str()
        .context("run-log manifest paths must be valid UTF-8")?
        .to_owned();
    ensure!(
        run_log_uri.len() <= limits.max_string_bytes,
        "run-log manifest URI exceeds the configured string-byte limit"
    );

    let summary = pid_runlog::summarize_events(events)?;
    let replay = pid_runlog::replay_events_with_limits(events, limits)?;
    let mut artifacts = Vec::new();
    artifacts
        .try_reserve_exact(replay.artifacts.len())
        .context("failed to reserve snapshot manifest artifacts")?;
    for artifact in replay.artifacts {
        let content_hash = artifact
            .sha256
            .as_deref()
            .map(|digest| HashIdentity::sha256(HashRevision::FileBytesV1, digest))
            .transpose()?;
        artifacts.push(SnapshotArtifactManifestWire {
            name: artifact.name,
            kind: artifact.kind,
            uri: artifact.uri,
            sha256: artifact.sha256,
            content_hash,
        });
    }

    let snapshot_sha256 = pid_runlog::sha256_hex(snapshot);
    // `RunManifest` is intentionally non-exhaustive and its path constructors reopen the file.
    // Materialize its public wire schema so both event parsing and file identity come from these
    // exact bytes; the parity regression below detects upstream manifest-schema drift.
    let wire_manifest = SnapshotManifestWire {
        sidecar_schema_version: pid_runlog::RUN_LOG_SIDECAR_SCHEMA_VERSION,
        schema_version: replay
            .schema_version
            .unwrap_or(pid_runlog::RUN_LOG_SCHEMA_VERSION),
        run_id: summary.run_id,
        config_hash: summary.config_hash,
        run_log_uri,
        run_log_sha256: Some(snapshot_sha256.clone()),
        run_log_hash: Some(HashIdentity::sha256(
            HashRevision::FileBytesV1,
            snapshot_sha256,
        )?),
        trace_hash: summary.trace_hash,
        trace_hash_v2: summary.trace_hash_v2,
        logical_trace_hash: summary.logical_trace_hash,
        logical_trace_hash_v3: summary.logical_trace_hash_v3,
        hash_identities: summary.hash_identities,
        event_count: summary.event_count,
        validation_errors: summary.validation_errors,
        validation_warnings: summary.validation_warnings,
        artifacts,
    };
    serde_json::from_value(
        serde_json::to_value(wire_manifest).context("failed to encode snapshot manifest")?,
    )
    .context("failed to construct snapshot-bound run-log manifest")
}

fn prepare_snapshot(
    path: &Path,
    snapshot: &[u8],
    allow_invalid: bool,
    limits: RunLogLimits,
) -> Result<PreparedRunLog> {
    let events = pid_runlog::read_events_with_limits(Cursor::new(snapshot), limits)?;
    let validation = pid_runlog::validate_events_with_limits(&events, limits)?;
    if !validation.is_valid() && !allow_invalid {
        bail!(
            "run log failed validation ({} error(s)); pass --allow-invalid to visualize anyway",
            validation.errors
        );
    }
    let manifest = manifest_for_snapshot(path, snapshot, &events, limits)?;
    Ok(PreparedRunLog { events, manifest })
}

fn prepare_run_log(path: &Path, allow_invalid: bool) -> Result<PreparedRunLog> {
    let limits = RunLogLimits::default();
    let snapshot = read_bounded_snapshot(path, limits)?;
    prepare_snapshot(path, &snapshot, allow_invalid, limits)
}

fn parse_options(args: &[String]) -> Result<ConverterOptions> {
    let Some(input) = args
        .first()
        .filter(|value| !value.is_empty() && !value.starts_with('-'))
    else {
        bail!(
            "usage: runlog-to-rerun <run-log.jsonl> [--save out.rrd] [--serve] \
             [--allow-invalid] [--load-attribution-artifacts]"
        );
    };
    let mut save_path: Option<String> = None;
    let mut serve = false;
    let mut allow_invalid = false;
    let mut load_attribution_artifacts = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--save" => {
                if save_path.is_some() {
                    bail!("--save may be specified only once");
                }
                let Some(path) = args
                    .get(i + 1)
                    .filter(|value| !value.is_empty() && !value.starts_with('-'))
                else {
                    bail!("--save requires a path");
                };
                save_path = Some(path.clone());
                i += 2;
            }
            "--serve" => {
                if serve {
                    bail!("--serve may be specified only once");
                }
                serve = true;
                i += 1;
            }
            "--allow-invalid" => {
                if allow_invalid {
                    bail!("--allow-invalid may be specified only once");
                }
                allow_invalid = true;
                i += 1;
            }
            "--load-attribution-artifacts" => {
                if load_attribution_artifacts {
                    bail!("--load-attribution-artifacts may be specified only once");
                }
                load_attribution_artifacts = true;
                i += 1;
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    if save_path.is_some() && serve {
        bail!("--save and --serve are mutually exclusive");
    }
    Ok(ConverterOptions {
        input: PathBuf::from(input),
        save_path,
        serve,
        allow_invalid,
        load_attribution_artifacts,
    })
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let ConverterOptions {
        input,
        save_path,
        serve,
        allow_invalid,
        load_attribution_artifacts,
    } = parse_options(&args)?;

    // Resolve and reject unsafe destinations before reading or converting the
    // source. `save_recording` repeats this check and performs the race-safe
    // no-clobber installation after explicit encoder finalization.
    let save_destination = save_path
        .as_deref()
        .map(prepare_new_recording_path)
        .transpose()?;

    let PreparedRunLog { events, manifest } = prepare_run_log(&input, allow_invalid)?;
    let rec = init_recording("prisoma_runlog", serve)?;
    let mut logger = RunLogRerunLogger::new(&rec);
    if load_attribution_artifacts {
        let parent = input
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        logger = logger.with_external_artifact_loading(parent)?;
    }
    let summary = logger.log_events_with_manifest(&events, Some(&manifest))?;
    if save_path.is_none() && !serve {
        println!("note: neither --save nor --serve given; recording will be discarded (dry run)");
    }
    println!(
        "converted events={} run_id={} trace_hash_v2={} trace_hash_revision=replay_trace_v2 validation_errors={} validation_warnings={}",
        summary.event_count,
        summary.run_id.as_deref().unwrap_or("<unknown>"),
        summary.trace_hash_v2,
        summary.validation_errors,
        summary.validation_warnings
    );
    if let Some(path) = save_destination {
        save_recording(&rec, &path)?;
        println!("saved {}", path.display());
    } else {
        println!("logged {} events", events.len());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        parse_options, prepare_snapshot, read_bounded_snapshot, ConverterOptions, PreparedRunLog,
    };
    use anyhow::{bail, ensure, Context, Result};
    use pid_runlog::{RunLogEvent, RunLogLimits, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::fs::{self, OpenOptions};
    use std::io::{ErrorKind, Write};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TempRunLog {
        path: PathBuf,
    }

    impl TempRunLog {
        fn create(bytes: &[u8]) -> Result<Self> {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            for _ in 0..1024 {
                let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "pid-rerun-snapshot-{}-{id}.jsonl",
                    std::process::id()
                ));
                match OpenOptions::new().write(true).create_new(true).open(&path) {
                    Ok(mut file) => {
                        file.write_all(bytes)?;
                        file.flush()?;
                        return Ok(Self { path });
                    }
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(error) => {
                        return Err(error).with_context(|| {
                            format!("failed to create temporary run log {}", path.display())
                        });
                    }
                }
            }
            bail!("failed to allocate a unique temporary run-log path")
        }
    }

    impl Drop for TempRunLog {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    fn run_log_bytes(run_id: &str) -> Result<Vec<u8>> {
        let config = json!({"dt": 0.1});
        let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
        let events = [
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: run_id.to_owned(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            },
            RunLogEvent::ArtifactLogged {
                timestamp_ns: 1,
                name: "snapshot_fixture".to_owned(),
                kind: "fixture".to_owned(),
                uri: "artifact.bin".to_owned(),
                sha256: Some(pid_runlog::sha256_hex(b"artifact")),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::RunEnded {
                run_id: run_id.to_owned(),
                timestamp_ns: 2,
                status: RunStatus::Succeeded,
                message: None,
            },
        ];
        let mut writer = RunLogWriter::new(Vec::new());
        for event in &events {
            writer.append(event)?;
        }
        Ok(writer.into_inner())
    }

    fn incomplete_run_log_bytes(run_id: &str) -> Result<Vec<u8>> {
        let config = json!({"dt": 0.1});
        let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
        let events = [
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: run_id.to_owned(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: BTreeMap::new(),
            },
            RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            },
        ];
        let mut writer = RunLogWriter::new(Vec::new());
        for event in &events {
            writer.append(event)?;
        }
        Ok(writer.into_inner())
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn converter_options_are_explicit() {
        assert_eq!(
            parse_options(&args(&["input.jsonl"])).unwrap(),
            ConverterOptions {
                input: PathBuf::from("input.jsonl"),
                save_path: None,
                serve: false,
                allow_invalid: false,
                load_attribution_artifacts: false,
            }
        );
        assert_eq!(
            parse_options(&args(&[
                "input.jsonl",
                "--save",
                "output.rrd",
                "--allow-invalid",
                "--load-attribution-artifacts",
            ]))
            .unwrap(),
            ConverterOptions {
                input: PathBuf::from("input.jsonl"),
                save_path: Some("output.rrd".to_owned()),
                serve: false,
                allow_invalid: true,
                load_attribution_artifacts: true,
            }
        );
    }

    #[test]
    fn converter_rejects_missing_or_ambiguous_arguments() {
        assert!(parse_options(&[]).is_err());
        assert!(parse_options(&args(&["--serve"])).is_err());
        assert!(parse_options(&args(&["input.jsonl", "--save"])).is_err());
        assert!(parse_options(&args(&["input.jsonl", "--save", "--serve"])).is_err());
        assert!(parse_options(&args(&[
            "input.jsonl",
            "--save",
            "a.rrd",
            "--save",
            "b.rrd",
        ]))
        .is_err());
        assert!(parse_options(&args(&["input.jsonl", "--serve", "--serve"])).is_err());
        assert!(
            parse_options(&args(&["input.jsonl", "--save", "output.rrd", "--serve",])).is_err()
        );
        assert!(parse_options(&args(&[
            "input.jsonl",
            "--allow-invalid",
            "--allow-invalid",
        ]))
        .is_err());
        assert!(parse_options(&args(&[
            "input.jsonl",
            "--load-attribution-artifacts",
            "--load-attribution-artifacts",
        ]))
        .is_err());
        assert!(parse_options(&args(&["input.jsonl", "--unknown"])).is_err());
    }

    #[test]
    fn snapshot_manifest_matches_canonical_path_manifest_for_stable_input() -> Result<()> {
        let bytes = run_log_bytes("snapshot-stable")?;
        let input = TempRunLog::create(&bytes)?;
        let limits = RunLogLimits::default();
        let snapshot = read_bounded_snapshot(&input.path, limits)?;

        let PreparedRunLog { manifest, .. } =
            prepare_snapshot(&input.path, &snapshot, false, limits)?;
        let canonical = pid_runlog::manifest_for_path(&input.path)?;

        assert_eq!(manifest, canonical);
        Ok(())
    }

    #[test]
    fn path_mutation_after_read_does_not_detach_events_or_manifest_from_snapshot() -> Result<()> {
        let first = run_log_bytes("snapshot-a")?;
        let second = run_log_bytes("snapshot-b")?;
        assert_eq!(first.len(), second.len());
        let input = TempRunLog::create(&first)?;
        let limits = RunLogLimits::default();
        let snapshot = read_bounded_snapshot(&input.path, limits)?;
        fs::write(&input.path, &second)?;

        let prepared = prepare_snapshot(&input.path, &snapshot, false, limits)?;
        let current = pid_runlog::manifest_for_path(&input.path)?;
        let parsed_run_id = prepared.events.iter().find_map(|event| match event {
            RunLogEvent::RunStarted { run_id, .. } => Some(run_id.as_str()),
            _ => None,
        });
        let observed = (
            parsed_run_id,
            prepared.manifest.run_id.as_deref(),
            prepared.manifest.run_log_sha256.as_deref(),
            prepared
                .manifest
                .run_log_hash
                .as_ref()
                .map(|identity| identity.digest.as_str()),
            current.run_id.as_deref(),
        );
        let snapshot_sha256 = pid_runlog::sha256_hex(&first);

        assert_eq!(
            observed,
            (
                Some("snapshot-a"),
                Some("snapshot-a"),
                Some(snapshot_sha256.as_str()),
                Some(snapshot_sha256.as_str()),
                Some("snapshot-b"),
            )
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_manifest_rejects_non_utf8_input_identity() -> Result<()> {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let bytes = run_log_bytes("snapshot-non-utf8")?;
        let path = PathBuf::from(OsString::from_vec(
            b"/tmp/prisoma-runlog-\xff.jsonl".to_vec(),
        ));
        let error = prepare_snapshot(&path, &bytes, false, RunLogLimits::default())
            .expect_err("lossy run-log identities must fail closed");

        assert!(format!("{error:#}").contains("valid UTF-8"));
        Ok(())
    }

    #[test]
    fn invalid_snapshot_fails_during_preparation() -> Result<()> {
        let snapshot = incomplete_run_log_bytes("snapshot-invalid")?;

        let error = prepare_snapshot(
            Path::new("unopened-invalid-runlog.jsonl"),
            &snapshot,
            false,
            RunLogLimits::default(),
        )
        .unwrap_err();

        assert!(format!("{error:#}").contains("run log failed validation"));
        Ok(())
    }

    #[test]
    fn allow_invalid_preserves_manifest_validation_errors() -> Result<()> {
        let snapshot = incomplete_run_log_bytes("snapshot-allowed")?;

        let prepared = prepare_snapshot(
            Path::new("unopened-allowed-runlog.jsonl"),
            &snapshot,
            true,
            RunLogLimits::default(),
        )?;

        assert!(prepared.manifest.validation_errors > 0);
        Ok(())
    }

    #[test]
    fn bounded_snapshot_rejects_oversize_input() -> Result<()> {
        let bytes = run_log_bytes("snapshot-oversize")?;
        let input = TempRunLog::create(&bytes)?;
        let limit = u64::try_from(bytes.len() - 1)?;

        let error = read_bounded_snapshot(
            &input.path,
            RunLogLimits::default().with_max_file_bytes(limit),
        )
        .unwrap_err();

        assert!(format!("{error:#}").contains("exceed resource limit"));
        Ok(())
    }

    #[test]
    fn bounded_snapshot_rejects_nonregular_and_symlink_inputs() -> Result<()> {
        let dir = tempfile::tempdir()?;
        assert!(read_bounded_snapshot(dir.path(), RunLogLimits::default()).is_err());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let target = dir.path().join("target.jsonl");
            fs::write(&target, run_log_bytes("snapshot-symlink")?)?;
            let alias = dir.path().join("alias.jsonl");
            symlink(&target, &alias)?;
            assert!(read_bounded_snapshot(&alias, RunLogLimits::default()).is_err());
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn bounded_snapshot_rejects_fifo_without_blocking() -> Result<()> {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};

        let dir = tempfile::tempdir()?;
        let fifo = dir.path().join("input.jsonl");
        let timeout = Duration::from_secs(2);
        let started = Instant::now();
        let mut child = Command::new("mkfifo")
            .arg(&fifo)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to spawn mkfifo fixture command")?;
        let status = loop {
            if let Some(status) = child
                .try_wait()
                .context("failed while waiting for mkfifo fixture command")?
            {
                break status;
            }
            if started.elapsed() >= timeout {
                child
                    .kill()
                    .context("failed to terminate timed-out mkfifo fixture command")?;
                let _status = child
                    .wait()
                    .context("failed to reap timed-out mkfifo fixture command")?;
                bail!("mkfifo fixture command exceeded its {timeout:?} deadline");
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        ensure!(status.success(), "mkfifo fixture command failed: {status}");
        assert!(read_bounded_snapshot(&fifo, RunLogLimits::default()).is_err());
        Ok(())
    }
}
