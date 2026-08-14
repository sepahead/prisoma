use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use pid_runlog::{
    canonical_json_hash_v2, Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus,
    RUN_LOG_SCHEMA_VERSION,
};
use pid_sim::file_snapshot::sync_directory;
use pid_sim::world_model::{
    execute_published_decision, propose_reference_session_decision, record_forecast_commit,
    record_oracle_label, reference_config, reference_sim, verify_world_model_runlog,
    REFERENCE_RUN_ID, REFERENCE_SOURCE,
};
use pid_sim::{canonical_new_artifact_path, FsyncFileWriter, SimBridgeSession};
use serde_json::json;

const DEFAULT_RUN_LOG: &str = "outputs/world_model_reference.jsonl";
const DEFAULT_SUMMARY: &str = "outputs/world_model_reference.summary.json";

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run { run_log: PathBuf, summary: PathBuf },
    Verify { run_log: PathBuf },
}

fn main() -> Result<()> {
    match parse_args()? {
        Command::Run { run_log, summary } => run_reference(&run_log, &summary),
        Command::Verify { run_log } => {
            let report = verify_world_model_runlog(&run_log)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            report.require_valid()
        }
    }
}

fn parse_args() -> Result<Command> {
    parse_args_from(std::env::args_os().skip(1))
}

fn parse_args_from(args: impl IntoIterator<Item = OsString>) -> Result<Command> {
    let args = args.into_iter().collect::<Vec<_>>();
    if args.is_empty() {
        return Ok(Command::Run {
            run_log: PathBuf::from(DEFAULT_RUN_LOG),
            summary: PathBuf::from(DEFAULT_SUMMARY),
        });
    }
    let mut run_log = None;
    let mut summary = None;
    let mut verify = None;
    let mut run_log_supplied = false;
    let mut summary_supplied = false;
    let mut index = 0;
    while index < args.len() {
        let argument = args[index]
            .to_str()
            .context("world-model arguments must be valid UTF-8")?;
        match argument {
            "--run-log" => {
                run_log = Some(argument_path(&args, index, "--run-log")?);
                run_log_supplied = true;
                index += 2;
            }
            "--summary" => {
                summary = Some(argument_path(&args, index, "--summary")?);
                summary_supplied = true;
                index += 2;
            }
            "--verify" => {
                verify = Some(argument_path(&args, index, "--verify")?);
                index += 2;
            }
            "--help" | "-h" => {
                println!(
                    "usage:\n  pid-world-model-reference [--run-log PATH] [--summary PATH]\n  \
                     pid-world-model-reference --verify PATH\n\nOutputs are never overwritten."
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    if let Some(run_log) = verify {
        if run_log_supplied || summary_supplied {
            bail!("--verify cannot be combined with --run-log or --summary");
        }
        return Ok(Command::Verify { run_log });
    }
    Ok(Command::Run {
        run_log: run_log.unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_LOG)),
        summary: summary.unwrap_or_else(|| PathBuf::from(DEFAULT_SUMMARY)),
    })
}

fn argument_path(args: &[std::ffi::OsString], index: usize, name: &str) -> Result<PathBuf> {
    let path = args
        .get(index + 1)
        .map(PathBuf::from)
        .with_context(|| format!("{name} requires a path"))?;
    if path.as_os_str().is_empty() {
        bail!("{name} path must not be empty");
    }
    Ok(path)
}

fn run_reference(run_log: &Path, summary: &Path) -> Result<()> {
    prepare_default_parent(run_log)?;
    prepare_default_parent(summary)?;
    let (run_log, run_log_parent) = canonical_new_artifact_path(run_log)?;
    let (summary, summary_parent) = canonical_new_artifact_path(summary)?;
    if run_log == summary {
        bail!("run-log and summary paths must differ");
    }
    ensure_absent(&run_log, "world-model run log")?;
    ensure_absent(&summary, "world-model summary")?;

    // Finish deterministic configuration and model fitting before claiming an output path.
    let config = reference_config()?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&run_log)
        .with_context(|| format!("failed to create run log {}", run_log.display()))?;
    let config_value = json!({
        "source": REFERENCE_SOURCE,
        "world_model_decision": &config,
    });
    let config_hash = canonical_json_hash_v2(&config_value)?;
    let mut writer = RunLogWriter::new(FsyncFileWriter::new(file));
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: REFERENCE_RUN_ID.to_string(),
        timestamp_ns: 0,
        config_hash: config_hash.clone(),
        metadata: [("source".to_string(), REFERENCE_SOURCE.to_string())]
            .into_iter()
            .collect(),
    })?;
    writer.append(&RunLogEvent::ConfigLogged {
        timestamp_ns: 0,
        config_hash,
        config: config_value,
    })?;
    let sim = reference_sim(&config)?;
    writer.append(&sim.snapshot_event())?;
    writer.flush()?;

    let mut session = SimBridgeSession::with_run_id(writer, sim, REFERENCE_RUN_ID);
    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: REFERENCE_SOURCE.to_string(),
        session_id: Some(REFERENCE_RUN_ID.to_string()),
    };
    for decision_index in 0..config.decisions {
        let committed = propose_reference_session_decision(&config, &session, decision_index)?
            .commit_forecast()?;
        let published = record_forecast_commit(&mut session, committed)?;
        let executed = execute_published_decision(&mut session, published, &actor)?;
        let labeled = executed.label_oracle()?;
        record_oracle_label(&mut session, labeled)?;
    }
    session.finish_run(
        RunStatus::Succeeded,
        Some("world-model contract reference complete".to_string()),
    )?;
    drop(session);
    sync_directory(&run_log_parent, "world-model run-log parent")?;

    let report = verify_world_model_runlog(&run_log)?;
    report.require_valid()?;
    write_summary(&summary, &summary_parent, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    eprintln!("run_log={}", run_log.display());
    eprintln!("summary={}", summary.display());
    Ok(())
}

fn write_summary(
    path: &Path,
    parent: &Path,
    report: &pid_sim::world_model::WorldModelVerificationReport,
) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create summary {}", path.display()))?;
    let bytes = serde_json::to_vec_pretty(report)?;
    file.write_all(&bytes)
        .with_context(|| format!("failed to write summary {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("failed to finish summary {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync summary {}", path.display()))?;
    sync_directory(parent, "world-model summary parent")
}

fn prepare_default_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        if parent == Path::new("outputs") && !parent.exists() {
            std::fs::create_dir(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    Ok(())
}

fn ensure_absent(path: &Path, description: &str) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => bail!("{description} already exists: {}", path.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to inspect {description} {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_mode_rejects_run_flags() {
        let result = parse_args_from([
            OsString::from("--verify"),
            OsString::from("run.jsonl"),
            OsString::from("--summary"),
            OsString::from("summary.json"),
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn run_mode_parses_distinct_outputs() {
        let command = parse_args_from([
            OsString::from("--run-log"),
            OsString::from("run.jsonl"),
            OsString::from("--summary"),
            OsString::from("summary.json"),
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::Run {
                run_log: PathBuf::from("run.jsonl"),
                summary: PathBuf::from("summary.json"),
            }
        );
    }
}
