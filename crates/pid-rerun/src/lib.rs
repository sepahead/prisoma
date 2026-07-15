//! `pid-rerun`: Rerun visualization adapters for prisoma diagnostics.
//!
//! This crate provides logging adapters to visualize:
//! - PID metrics (redundancy, synergy, unique information) over time
//! - VLA embeddings and their geometry
//! - 3D object flow trajectories
//! - Ghost splat overlays for predicted vs actual states
//!
//! # Architecture (Rerun-First, Phases 1-3)
//! Uses Rerun SDK as the primary visualization backend. Entities are logged to:
//! - `world/reality`: Captured scene data
//! - `world/ghost`: Predicted flow as secondary point cloud
//! - `pid/metrics`: PID atom time series
//! - `vla/embeddings`: Embedding geometry diagnostics

pub mod adapters;
pub mod data;
pub mod entities;
pub mod runlog;

mod bounded_process;

pub use adapters::{PidLogger, VlaLogger};
pub use data::{VlaEpisode, VlaFrame};
pub use entities::EntityPaths;
pub use runlog::RunLogRerunLogger;

use anyhow::{bail, ensure, Context, Result};
use rerun::{
    log::LogMsg,
    sink::{LogSink, SinkFlushError},
    RecordingStream, RecordingStreamBuilder,
};
use std::{
    io::Write,
    ops::Deref,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

const RECORDING_FINALIZE_TIMEOUT: Duration = Duration::from_secs(30);
const VIEWER_VERSION_TIMEOUT: Duration = Duration::from_secs(5);
const VIEWER_VERSION_OUTPUT_LIMIT: usize = 16 * 1024;

// Re-export rerun for convenience
pub use rerun;

#[derive(Default)]
struct CaptureState {
    messages: Vec<LogMsg>,
    sealed: bool,
}

#[derive(Default)]
struct CaptureStorage {
    state: Mutex<CaptureState>,
}

impl CaptureStorage {
    /// Linearization point for sink delivery. A message is either appended before sealing or
    /// rejected after it; it can never land in a replacement buffer after a successful snapshot.
    fn send(&self, message: LogMsg) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.sealed {
            return false;
        }
        state.messages.push(message);
        true
    }

    fn send_all(&self, mut messages: Vec<LogMsg>) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.sealed {
            return false;
        }
        state.messages.append(&mut messages);
        true
    }

    fn seal_and_take(&self) -> Result<Vec<LogMsg>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Rerun capture storage lock is poisoned"))?;
        ensure!(!state.sealed, "Rerun capture storage is already sealed");
        state.sealed = true;
        Ok(std::mem::take(&mut state.messages))
    }
}

struct CaptureSink {
    storage: Arc<CaptureStorage>,
}

impl LogSink for CaptureSink {
    fn send(&self, message: LogMsg) {
        // `LogSink::send` has no error return. The terminal wrapper therefore rejects deliveries
        // after the storage's linearization point; callers must still obey the documented rule
        // that logging stops before finalization.
        let _accepted = self.storage.send(message);
    }

    fn send_all(&self, messages: Vec<LogMsg>) {
        let _accepted = self.storage.send_all(messages);
    }

    fn flush_blocking(&self, _timeout: Duration) -> Result<(), SinkFlushError> {
        Ok(())
    }
}

/// A recording created by [`init_recording`].
///
/// Headless instances capture messages from their first byte without a late sink swap. Saving is
/// a terminal, one-shot operation; callers must stop logging before finalization begins.
pub struct PrisomaRecording {
    stream: RecordingStream,
    capture: Option<Arc<CaptureStorage>>,
    finalization_started: AtomicBool,
}

impl Deref for PrisomaRecording {
    type Target = RecordingStream;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

/// Initialize a Rerun recording stream for prisoma visualization.
pub fn init_recording(app_id: &str, spawn_viewer: bool) -> Result<PrisomaRecording> {
    let builder = RecordingStreamBuilder::new(app_id);

    let (stream, capture) = if spawn_viewer {
        require_matching_viewer_version()?;
        (builder.spawn()?, None)
    } else {
        let storage = Arc::new(CaptureStorage::default());
        let sink = CaptureSink {
            storage: Arc::clone(&storage),
        };
        let sinks: Vec<Box<dyn LogSink>> = vec![Box::new(sink)];
        (builder.set_sinks(sinks)?, Some(storage))
    };

    Ok(PrisomaRecording {
        stream,
        capture,
        finalization_started: AtomicBool::new(false),
    })
}

/// Require the `rerun` executable used by interactive paths to match the linked SDK.
pub fn require_matching_viewer_version() -> Result<()> {
    let expected = rerun::build_info().version.to_string();
    let output = bounded_process::run_bounded(
        Command::new("rerun").arg("--version"),
        VIEWER_VERSION_TIMEOUT,
        VIEWER_VERSION_OUTPUT_LIMIT,
    )
    .context("interactive Rerun mode requires a bounded `rerun --version` probe")?;
    if !output.status.success() {
        bail!("`rerun --version` exited with status {}", output.status);
    }
    let stdout =
        String::from_utf8(output.stdout).context("`rerun --version` returned non-UTF-8 output")?;
    let actual = parse_viewer_version(&stdout)
        .context("could not parse the version reported by `rerun --version`")?;
    if actual != expected {
        bail!(
            "Rerun viewer/SDK version mismatch: viewer={actual}, SDK={expected}; \
             install the matching Rerun {expected} viewer or use headless --save mode"
        );
    }
    Ok(())
}

fn parse_viewer_version(output: &str) -> Option<&str> {
    let mut fields = output.split_whitespace();
    match fields.next()? {
        "rerun" | "rerun-cli" => {}
        _ => return None,
    }
    let version = fields.next()?;
    version
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || ".-+".contains(character))
        .then_some(version)
}

/// Resolve a new `.rrd` destination through its existing parent directory.
///
/// The returned path is canonical except for its final, not-yet-created file
/// name. Existing destinations are rejected; the later no-clobber install is
/// the authoritative race-safe check.
pub fn prepare_new_recording_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    ensure!(
        path.extension().and_then(|extension| extension.to_str()) == Some("rrd"),
        "Rerun recording output must end in .rrd: {}",
        path.display()
    );
    let file_name = path
        .file_name()
        .filter(|name| !name.is_empty())
        .context("Rerun recording output must name a file")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let canonical_parent = parent.canonicalize().with_context(|| {
        format!(
            "Rerun recording output parent must already exist: {}",
            parent.display()
        )
    })?;
    ensure!(
        canonical_parent.is_dir(),
        "Rerun recording output parent is not a directory: {}",
        canonical_parent.display()
    );
    let destination = canonical_parent.join(file_name);
    match std::fs::symlink_metadata(&destination) {
        Ok(_) => bail!(
            "Rerun recording output already exists; refusing to overwrite {}",
            destination.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to inspect Rerun recording output {}",
                    destination.display()
                )
            });
        }
    }
    Ok(destination)
}

/// Finalize and install a new RRD recording without overwriting any path.
///
/// Rerun's direct file sink swaps sinks asynchronously and opens its target with truncation
/// semantics. This wrapper finalizes a headless capture that was installed when the recording was
/// created, syncs a staged file, and atomically installs it with no-clobber semantics. No fallible
/// operation follows the final install, so an error return never strands a successful destination.
pub fn save_recording(rec: &PrisomaRecording, path: impl AsRef<Path>) -> Result<()> {
    let destination = prepare_new_recording_path(path)?;
    let bytes = finalize_recording_bytes(rec)?;

    let parent = destination
        .parent()
        .context("resolved Rerun recording output has no parent")?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to stage Rerun recording in {}", parent.display()))?;
    staged
        .write_all(&bytes)
        .with_context(|| format!("failed to stage Rerun recording {}", destination.display()))?;
    staged
        .as_file()
        .sync_all()
        .with_context(|| format!("failed to sync Rerun recording {}", destination.display()))?;
    let _installed = staged.persist_noclobber(&destination).map_err(|error| {
        anyhow::Error::new(error.error).context(format!(
            "Rerun recording output already exists or cannot be installed: {}",
            destination.display()
        ))
    })?;
    Ok(())
}

/// Finalize a headless recording into one complete RRD byte stream.
///
/// This is terminal and one-shot. It never calls Rerun's unbounded sink-switch path and never
/// creates a `BinaryStreamStorage` whose destructor performs an unbounded flush.
pub fn finalize_recording_bytes(rec: &PrisomaRecording) -> Result<Vec<u8>> {
    let capture = rec
        .capture
        .as_ref()
        .context("interactive Rerun recordings cannot be finalized as headless RRD bytes")?;
    ensure!(
        rec.finalization_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok(),
        "Rerun recording finalization is terminal and may run only once"
    );

    let stream = rec.stream.clone();
    let (result_sender, result_receiver) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("prisoma-rerun-finalize".to_owned())
        .spawn(move || {
            let result = stream.flush_with_timeout(RECORDING_FINALIZE_TIMEOUT);
            let _ = result_sender.send(result);
        })
        .context("failed to start bounded Rerun finalization")?;
    let flush_result = result_receiver
        .recv_timeout(RECORDING_FINALIZE_TIMEOUT)
        .context("timed out while finalizing the Rerun recording")?;
    flush_result.context("failed while finalizing the Rerun recording")?;

    // This mutex-protected transition is the capture linearization point. Every sink delivery
    // ordered before it is included; every delivery ordered after it is rejected rather than
    // being appended to an unseen replacement vector.
    let messages = capture.seal_and_take()?;
    ensure!(
        !messages.is_empty(),
        "Rerun encoder received no recording messages"
    );
    let bytes = re_log_encoding::Encoder::encode(messages.into_iter().map(Ok))
        .context("Rerun encoder failed to finalize the recording")?;
    ensure!(
        bytes.starts_with(b"RRF2"),
        "Rerun encoder produced an unrecognized recording header"
    );
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::{
        finalize_recording_bytes, init_recording, parse_viewer_version, prepare_new_recording_path,
        save_recording, RECORDING_FINALIZE_TIMEOUT,
    };
    use anyhow::Result;
    use rerun::log::{Chunk, LogMsg};
    use rerun_types::archetypes::Scalars;
    use std::fs;
    use std::io::Cursor;

    #[test]
    fn parses_supported_viewer_version_forms() {
        assert_eq!(
            parse_viewer_version("rerun-cli 0.34.1 (base)\n"),
            Some("0.34.1")
        );
        assert_eq!(parse_viewer_version("rerun 0.34.1\n"), Some("0.34.1"));
    }

    #[test]
    fn rejects_unrecognized_or_malformed_viewer_versions() {
        assert_eq!(parse_viewer_version("other 0.34.1\n"), None);
        assert_eq!(parse_viewer_version("rerun 0.34.1;rm\n"), None);
        assert_eq!(parse_viewer_version("rerun\n"), None);
    }

    #[test]
    fn finalized_recording_is_complete_rrd_and_never_clobbers() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let destination = dir.path().join("recording.rrd");
        let rec = init_recording("safe_save_test", false)?;
        rec.log("metric", &Scalars::single(1.0))?;
        save_recording(&rec, &destination)?;
        let bytes = fs::read(&destination)?;
        assert!(bytes.starts_with(b"RRF2"));
        assert!(bytes.len() > 4);
        let decoded = re_log_encoding::DecoderApp::decode_eager(Cursor::new(bytes.as_slice()))?
            .collect::<Result<Vec<_>, _>>()?;
        let decoded_paths = decoded
            .iter()
            .filter_map(|message| match message {
                LogMsg::ArrowMsg(_, arrow) => {
                    Some(Chunk::from_arrow_msg(arrow).map(|chunk| chunk.entity_path().to_string()))
                }
                _ => None,
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert!(
            decoded_paths
                .iter()
                .any(|path| path.trim_start_matches('/') == "metric"),
            "decoded RRD omitted the logged metric chunk: {decoded_paths:?}"
        );

        let replacement = init_recording("safe_save_again", false)?;
        replacement.log("metric", &Scalars::single(2.0))?;
        assert!(save_recording(&replacement, &destination).is_err());
        assert_eq!(fs::read(&destination)?, bytes);
        Ok(())
    }

    #[test]
    fn headless_finalization_is_terminal_and_one_shot() -> Result<()> {
        let rec = init_recording("one_shot_save", false)?;
        rec.log("metric", &Scalars::single(1.0))?;
        assert!(finalize_recording_bytes(&rec)?.starts_with(b"RRF2"));
        assert!(finalize_recording_bytes(&rec).is_err());
        Ok(())
    }

    #[test]
    fn capture_storage_rejects_sink_delivery_after_finalization_cut() -> Result<()> {
        let rec = init_recording("sealed_capture", false)?;
        rec.log("before_cut", &Scalars::single(1.0))?;
        assert!(finalize_recording_bytes(&rec)?.starts_with(b"RRF2"));

        // The third-party `LogSink` trait cannot report a late-delivery error to `log`, but the
        // capture state must remain sealed: a late message may never enter an unseen replacement
        // buffer after successful finalization.
        rec.log("after_cut", &Scalars::single(2.0))?;
        rec.stream
            .flush_with_timeout(RECORDING_FINALIZE_TIMEOUT)
            .map_err(|error| anyhow::anyhow!("late-log flush failed: {error:?}"))?;
        let capture = rec.capture.as_ref().unwrap();
        let state = capture
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(state.sealed);
        assert!(state.messages.is_empty());
        Ok(())
    }

    #[test]
    fn recording_destination_requires_rrd_and_existing_parent() -> Result<()> {
        let dir = tempfile::tempdir()?;
        assert!(prepare_new_recording_path(dir.path().join("recording.jsonl")).is_err());
        assert!(prepare_new_recording_path(dir.path().join("missing/out.rrd")).is_err());
        Ok(())
    }

    #[test]
    fn recording_destination_refuses_existing_hardlink_alias() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let source = dir.path().join("source.jsonl");
        fs::write(&source, b"source bytes")?;
        let alias = dir.path().join("alias.rrd");
        fs::hard_link(&source, &alias)?;

        assert!(prepare_new_recording_path(&alias).is_err());
        assert_eq!(fs::read(&source)?, b"source bytes");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn recording_destination_refuses_existing_symlink_alias() -> Result<()> {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir()?;
        let source = dir.path().join("source.jsonl");
        fs::write(&source, b"source bytes")?;
        let alias = dir.path().join("alias.rrd");
        symlink(&source, &alias)?;

        assert!(prepare_new_recording_path(&alias).is_err());
        assert_eq!(fs::read(&source)?, b"source bytes");
        Ok(())
    }
}
