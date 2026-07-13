use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let mut bytes = Vec::new();
    if stdout
        .by_ref()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > 1024 * 1024
    {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }
    if !child.wait().ok()?.success() {
        return None;
    }
    String::from_utf8(bytes)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_worktree_clean(repo: &Path) -> bool {
    let mut child = match Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=normal"])
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let Some(mut stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    };
    let mut first = [0_u8; 1];
    match stdout.read(&mut first) {
        Ok(0) => child.wait().is_ok_and(|status| status.success()),
        Ok(_) | Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            false
        }
    }
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
