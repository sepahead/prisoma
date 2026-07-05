use anyhow::{bail, Result};
use pid_rerun::{init_recording, save_recording, RunLogRerunLogger};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        bail!(
            "usage: {} <run-log.jsonl> [--save out.rrd] [--serve] [--allow-invalid]\n\
             (with neither --save nor --serve, the run log is converted and validated but the \
             recording is discarded — a dry run)",
            args[0]
        );
    }

    let input = PathBuf::from(&args[1]);
    let mut save_path: Option<String> = None;
    let mut serve = false;
    let mut allow_invalid = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--save" => {
                let Some(path) = args.get(i + 1) else {
                    bail!("--save requires a path");
                };
                save_path = Some(path.clone());
                i += 2;
            }
            "--serve" => {
                serve = true;
                i += 1;
            }
            "--allow-invalid" => {
                allow_invalid = true;
                i += 1;
            }
            other => bail!("unknown argument: {other}"),
        }
    }

    let events = pid_runlog::read_events_from_path(&input)?;
    let validation = pid_runlog::validate_events(&events);
    if !validation.is_valid() && !allow_invalid {
        bail!(
            "run log failed validation ({} error(s)); pass --allow-invalid to visualize anyway",
            validation.errors
        );
    }
    let manifest = pid_runlog::manifest_for_path(&input)?;
    let rec = init_recording("prisoma_runlog", serve)?;
    // Relative attribution artifact_uris are written next to the run log, so
    // resolve them against its directory, not the converter's CWD.
    let mut logger = RunLogRerunLogger::new(&rec);
    if let Some(parent) = input.parent().filter(|p| !p.as_os_str().is_empty()) {
        logger = logger.with_artifact_base_dir(parent);
    }
    let summary = logger.log_events_with_manifest(&events, Some(&manifest))?;
    if save_path.is_none() && !serve {
        println!("note: neither --save nor --serve given; recording will be discarded (dry run)");
    }
    println!(
        "converted events={} run_id={} trace_hash={} validation_errors={} validation_warnings={}",
        summary.event_count,
        summary.run_id.as_deref().unwrap_or("<unknown>"),
        summary.trace_hash,
        summary.validation_errors,
        summary.validation_warnings
    );
    if let Some(path) = save_path {
        save_recording(&rec, &path)?;
        println!("saved {path}");
    } else {
        println!("logged {} events", events.len());
    }
    Ok(())
}
