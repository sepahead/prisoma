//! Deterministic offline NCP wire-fault observatory.
//!
//! This binary opens no network or control path. It applies a frozen logical
//! schedule registry to a complete bounded baseline trace, replays every case
//! twice through the production callback/decoder seams, and publishes a
//! receipt-last report plus canonical run log.

use ncp_observer::observatory::{
    run_observatory, verify_observatory_publication, ConsumerProvenance,
};
use ncp_observer::read_bounded_regular_snapshot;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[path = "../../bounded_process.rs"]
mod bounded_process;

const GIT_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const GIT_OUTPUT_LIMIT: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Run {
        out_dir: PathBuf,
        trace: Option<PathBuf>,
    },
    Verify {
        root: PathBuf,
    },
}

#[derive(Debug, PartialEq, Eq)]
struct Args {
    mode: Mode,
}

fn usage() -> &'static str {
    "usage: ncp-fault-observatory (--out-dir DIR [--trace FROZEN_V1_BASELINE.json] | --verify DIR)"
}

fn parse_args() -> anyhow::Result<Option<Args>> {
    parse_args_from(std::env::args().collect())
}

fn parse_args_from(argv: Vec<String>) -> anyhow::Result<Option<Args>> {
    let mut out_dir = None;
    let mut trace = None;
    let mut verify = None;
    let mut index = 1;
    while index < argv.len() {
        let flag = &argv[index];
        if flag == "--help" || flag == "-h" {
            if argv.len() != 2 {
                anyhow::bail!("--help cannot be combined with other arguments");
            }
            return Ok(None);
        }
        if !matches!(flag.as_str(), "--out-dir" | "--trace" | "--verify") {
            anyhow::bail!("unknown argument {flag:?}; {}", usage());
        }
        let value = argv
            .get(index + 1)
            .ok_or_else(|| anyhow::anyhow!("flag {flag:?} expects a value"))?;
        if value.is_empty() {
            anyhow::bail!("flag {flag:?} expects a non-empty value");
        }
        match flag.as_str() {
            "--out-dir" => {
                if out_dir.replace(PathBuf::from(value)).is_some() {
                    anyhow::bail!("--out-dir may be supplied only once");
                }
            }
            "--trace" => {
                if trace.replace(PathBuf::from(value)).is_some() {
                    anyhow::bail!("--trace may be supplied only once");
                }
            }
            "--verify" => {
                if verify.replace(PathBuf::from(value)).is_some() {
                    anyhow::bail!("--verify may be supplied only once");
                }
            }
            _ => unreachable!("recognized flag handled above"),
        }
        index += 2;
    }
    let mode = match (out_dir, verify) {
        (Some(out_dir), None) => Mode::Run { out_dir, trace },
        (None, Some(root)) if trace.is_none() => Mode::Verify { root },
        (None, None) => anyhow::bail!("one of --out-dir or --verify is required; {}", usage()),
        (Some(_), Some(_)) => anyhow::bail!("--out-dir and --verify are mutually exclusive"),
        (None, Some(_)) => anyhow::bail!("--trace is valid only with --out-dir"),
    };
    Ok(Some(Args { mode }))
}

fn git_output(repo: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = bounded_process::run_bounded(
        Command::new("git").args(args).current_dir(repo),
        GIT_PROBE_TIMEOUT,
        GIT_OUTPUT_LIMIT,
    )
    .ok()?;
    output.status.success().then_some(output.stdout)
}

fn command_stdout_is_empty(command: &mut Command) -> Option<bool> {
    let output = bounded_process::run_bounded(command, GIT_PROBE_TIMEOUT, 0).ok()?;
    Some(output.status.success() && output.stdout.is_empty())
}

fn canonical_output_target(path: &Path) -> anyhow::Result<PathBuf> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("output directory has no final path component"))?;
    Ok(std::fs::canonicalize(parent)?.join(file_name))
}

fn worktree_clean_excluding(repo: &Path, out_dir: &Path) -> Option<bool> {
    let output = canonical_output_target(out_dir).ok()?;
    let relative = output.strip_prefix(repo).ok();
    let mut command = Command::new("git");
    command
        .args([
            "status",
            "--porcelain",
            "--untracked-files=normal",
            "--",
            ".",
        ])
        .current_dir(repo);
    if let Some(relative) = relative {
        let relative = relative.to_str()?;
        if !relative.is_empty() {
            command.arg(format!(":(exclude,literal){relative}"));
        }
    }
    command_stdout_is_empty(&mut command)
}

fn consumer_provenance(out_dir: &Path) -> anyhow::Result<ConsumerProvenance> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = std::fs::canonicalize(
        manifest_dir
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| anyhow::anyhow!("failed to locate repository root"))?,
    )?;
    let revision = git_output(&repo, &["rev-parse", "HEAD"])
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty() && text.len() <= 256)
        .unwrap_or_else(|| "not_recorded".to_string());
    let worktree_clean = worktree_clean_excluding(&repo, out_dir);
    let lockfile = manifest_dir.join("Cargo.lock");
    let lockfile_sha256 = if lockfile.exists() {
        Some(pid_runlog::sha256_hex(&read_bounded_regular_snapshot(
            &lockfile,
            16 * 1024 * 1024,
        )?))
    } else {
        None
    };
    let executable_sha256 = Some(pid_runlog::sha256_hex(&read_bounded_regular_snapshot(
        &std::env::current_exe()?,
        128 * 1024 * 1024,
    )?));
    let build_revision = option_env!("PRISOMA_BUILD_GIT_REVISION").map(str::to_string);
    let build_worktree_clean =
        option_env!("PRISOMA_BUILD_WORKTREE_CLEAN").and_then(|value| match value {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        });
    ConsumerProvenance::with_build_attestation(
        revision,
        worktree_clean,
        lockfile_sha256,
        executable_sha256,
        build_revision,
        build_worktree_clean,
    )
}

fn main() -> anyhow::Result<()> {
    let Some(args) = parse_args()? else {
        println!("{}", usage());
        return Ok(());
    };
    let outcome = match args.mode {
        Mode::Run { out_dir, trace } => {
            let consumer = consumer_provenance(&out_dir)?;
            run_observatory(&out_dir, trace.as_deref(), consumer)?
        }
        Mode::Verify { root } => verify_observatory_publication(&root)?,
    };
    println!(
        "[ncp-fault-observatory] report={} runlog={} receipt={} expectations_matched={}",
        outcome.report_path.display(),
        outcome.runlog_path.display(),
        outcome.receipt_path.display(),
        outcome.all_expectations_matched
    );
    if !outcome.all_expectations_matched {
        anyhow::bail!(
            "observatory completed and published, but one or more frozen expectations mismatched"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<String> {
        std::iter::once("ncp-fault-observatory")
            .chain(args.iter().copied())
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn strict_cli_requires_one_output_directory() {
        assert!(parse_args_from(argv(&[])).is_err());
        assert!(parse_args_from(argv(&["--out-dir"])).is_err());
        assert!(parse_args_from(argv(&["--wat", "x"])).is_err());
        assert!(parse_args_from(argv(&["--out-dir", "a", "--out-dir", "b"])).is_err());
        assert_eq!(
            parse_args_from(argv(&["--out-dir", "out", "--trace", "trace.json"])).unwrap(),
            Some(Args {
                mode: Mode::Run {
                    out_dir: PathBuf::from("out"),
                    trace: Some(PathBuf::from("trace.json")),
                },
            })
        );
        assert_eq!(
            parse_args_from(argv(&["--verify", "out"])).unwrap(),
            Some(Args {
                mode: Mode::Verify {
                    root: PathBuf::from("out"),
                },
            })
        );
        assert!(parse_args_from(argv(&["--verify", "out", "--trace", "trace.json"])).is_err());
        assert!(parse_args_from(argv(&["--verify", "a", "--out-dir", "b"])).is_err());
    }

    #[test]
    fn help_is_side_effect_free_and_exclusive() {
        assert_eq!(parse_args_from(argv(&["--help"])).unwrap(), None);
        assert!(parse_args_from(argv(&["--help", "--out-dir", "x"])).is_err());
    }
}
