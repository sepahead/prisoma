use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

mod bounded_process;

const GIT_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const GIT_OUTPUT_LIMIT: usize = 1024 * 1024;

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    let output = bounded_process::run_bounded(
        Command::new("git").args(args).current_dir(repo),
        GIT_PROBE_TIMEOUT,
        GIT_OUTPUT_LIMIT,
    )
    .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_worktree_clean(repo: &Path) -> bool {
    bounded_process::run_bounded(
        Command::new("git")
            .args(["status", "--porcelain", "--untracked-files=normal"])
            .current_dir(repo),
        GIT_PROBE_TIMEOUT,
        0,
    )
    .is_ok_and(|output| output.status.success() && output.stdout.is_empty())
}

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let repo = manifest
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."));
    let revision = git_output(repo, &["rev-parse", "HEAD"])
        .filter(|value| value.len() <= 256)
        .unwrap_or_else(|| "not_recorded".to_string());
    let clean = git_worktree_clean(repo);
    println!("cargo:rustc-env=PRISOMA_BUILD_GIT_REVISION={revision}");
    println!(
        "cargo:rustc-env=PRISOMA_BUILD_WORKTREE_CLEAN={}",
        if clean { "true" } else { "false" }
    );
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=bounded_process.rs");
    if let Some(head) = git_output(repo, &["rev-parse", "--git-path", "HEAD"]) {
        let head = PathBuf::from(head);
        let head = if head.is_absolute() {
            head
        } else {
            repo.join(head)
        };
        println!("cargo:rerun-if-changed={}", head.display());
    }
    if let Some(reference) = git_output(repo, &["symbolic-ref", "-q", "HEAD"])
        .and_then(|reference| git_output(repo, &["rev-parse", "--git-path", &reference]))
    {
        let reference = PathBuf::from(reference);
        let reference = if reference.is_absolute() {
            reference
        } else {
            repo.join(reference)
        };
        println!("cargo:rerun-if-changed={}", reference.display());
    }
}
