//! Bounded, descriptor-bound snapshots for local file inputs.
//!
//! The reader returns either all bytes and their digest or a typed oversize result.
//! It never assigns a whole-file digest to a retained prefix.

use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::fs::{File, Metadata, OpenOptions};
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    links: u64,
    len: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
impl FileIdentity {
    fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            links: metadata.nlink(),
            len: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }
}

#[cfg(not(unix))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    len: u64,
    modified: Option<std::time::SystemTime>,
}

#[cfg(not(unix))]
impl FileIdentity {
    fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        }
    }
}

/// One stable local input observation.
#[derive(Debug)]
pub struct BoundedFileSnapshot {
    path: PathBuf,
    description: String,
    identity: FileIdentity,
    /// Exact bytes, or `None` when the stable file exceeds the caller's limit.
    pub bytes: Option<Vec<u8>>,
    /// SHA-256 of `bytes`, or `None` for an oversized file.
    pub sha256: Option<String>,
    /// Stable descriptor length. This is exact even for an oversized result.
    pub byte_len: u64,
}

impl BoundedFileSnapshot {
    /// Recheck that the lexical path still names the retained file state.
    pub fn verify_path(&self) -> Result<()> {
        let metadata = std::fs::symlink_metadata(&self.path).with_context(|| {
            format!(
                "failed to re-inspect {} {}",
                self.description,
                self.path.display()
            )
        })?;
        let identity = regular_identity(&metadata, &self.path, &self.description)?;
        if identity != self.identity {
            bail!(
                "{} {} changed after it was snapshotted",
                self.description,
                self.path.display()
            );
        }
        Ok(())
    }

    /// Borrow the exact bytes or return a resource-limit error.
    pub fn exact_bytes(&self, maximum: u64) -> Result<&[u8]> {
        if self.byte_len > maximum {
            bail!(
                "{} {} exceeds the {maximum}-byte limit: observed {} bytes",
                self.description,
                self.path.display(),
                self.byte_len
            );
        }
        self.bytes.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "{} {} has no retained exact bytes despite fitting the {maximum}-byte limit",
                self.description,
                self.path.display(),
            )
        })
    }

    /// Test whether another regular lexical path names the retained file state.
    pub fn same_file_as(&self, path: impl AsRef<Path>, description: &str) -> Result<bool> {
        let path = path.as_ref();
        let metadata = std::fs::symlink_metadata(path)
            .with_context(|| format!("failed to inspect {description} {}", path.display()))?;
        Ok(regular_identity(&metadata, path, description)? == self.identity)
    }
}

fn regular_identity(metadata: &Metadata, path: &Path, description: &str) -> Result<FileIdentity> {
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "{description} must be a non-symlink regular file: {}",
            path.display()
        );
    }
    Ok(FileIdentity::from_metadata(metadata))
}

fn directory_identity(metadata: &Metadata, path: &Path, description: &str) -> Result<FileIdentity> {
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!(
            "{description} must be a non-symlink directory: {}",
            path.display()
        );
    }
    Ok(FileIdentity::from_metadata(metadata))
}

fn open_snapshot(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    options.open(path)
}

#[cfg(unix)]
fn open_directory(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    options.open(path)
}

#[cfg(not(unix))]
fn open_directory(_path: &Path) -> std::io::Result<File> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "descriptor-bound directory sync requires Unix O_NOFOLLOW support",
    ))
}

#[cfg(unix)]
fn require_exact_snapshot_platform() -> Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn require_exact_snapshot_platform() -> Result<()> {
    bail!("descriptor-bound exact snapshots require Unix O_NOFOLLOW support")
}

/// Sync one stable, non-symlink directory through a descriptor bound to its path.
pub fn sync_directory(path: impl AsRef<Path>, description: &str) -> Result<()> {
    require_exact_snapshot_platform()?;
    let path = path.as_ref();
    let lexical_before = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect {description} {}", path.display()))?;
    let lexical_before_identity = directory_identity(&lexical_before, path, description)?;
    let directory = open_directory(path)
        .with_context(|| format!("failed to open {description} {}", path.display()))?;
    let opened_before = directory
        .metadata()
        .with_context(|| format!("failed to inspect opened {description} {}", path.display()))?;
    let opened_before_identity = directory_identity(&opened_before, path, description)?;
    if opened_before_identity != lexical_before_identity {
        bail!(
            "{description} {} changed between path inspection and descriptor open",
            path.display()
        );
    }
    directory
        .sync_all()
        .with_context(|| format!("failed to sync {description} {}", path.display()))?;
    let opened_after = directory.metadata().with_context(|| {
        format!(
            "failed to re-inspect opened {description} {}",
            path.display()
        )
    })?;
    let lexical_after = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to re-inspect {description} {}", path.display()))?;
    if directory_identity(&opened_after, path, description)? != opened_before_identity
        || directory_identity(&lexical_after, path, description)? != opened_before_identity
    {
        bail!(
            "{description} {} changed while it was synced",
            path.display()
        );
    }
    Ok(())
}

/// Read at most `maximum + 1` bytes from one stable, regular local file.
pub fn read_bounded_regular_file(
    path: impl AsRef<Path>,
    maximum: u64,
    description: impl Into<String>,
) -> Result<BoundedFileSnapshot> {
    require_exact_snapshot_platform()?;
    let path = path.as_ref();
    let description = description.into();
    let lexical_before = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect {description} {}", path.display()))?;
    let lexical_before_identity = regular_identity(&lexical_before, path, &description)?;

    let mut file = open_snapshot(path)
        .with_context(|| format!("failed to open {} {}", description, path.display()))?;
    let opened_before = file
        .metadata()
        .with_context(|| format!("failed to stat opened {} {}", description, path.display()))?;
    let opened_before_identity = regular_identity(&opened_before, path, &description)?;
    if opened_before_identity != lexical_before_identity {
        bail!(
            "{} {} changed between path inspection and descriptor open",
            description,
            path.display()
        );
    }

    // Bind even an already-oversized result to the opened descriptor. This
    // avoids reading or allocating for it while keeping `byte_len` exact.
    if opened_before.len() > maximum {
        let snapshot = BoundedFileSnapshot {
            path: path.to_path_buf(),
            description,
            identity: opened_before_identity,
            bytes: None,
            sha256: None,
            byte_len: opened_before.len(),
        };
        snapshot.verify_path()?;
        return Ok(snapshot);
    }

    let initial_capacity = usize::try_from(opened_before.len().min(maximum)).unwrap_or(0);
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(initial_capacity)
        .context("failed to reserve bounded input snapshot")?;
    (&mut file)
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .with_context(|| format!("failed to read {} {}", description, path.display()))?;

    let opened_after = file.metadata().with_context(|| {
        format!(
            "failed to re-inspect opened {} {}",
            description,
            path.display()
        )
    })?;
    let opened_after_identity = regular_identity(&opened_after, path, &description)?;
    if opened_after_identity != opened_before_identity {
        bail!(
            "{} {} changed while its descriptor was read",
            description,
            path.display()
        );
    }
    let byte_len = opened_after.len();
    let oversized = byte_len > maximum || u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum;
    if !oversized && u64::try_from(bytes.len()).ok() != Some(byte_len) {
        bail!(
            "{} {} snapshot length does not match its descriptor",
            description,
            path.display()
        );
    }
    let (bytes, sha256) = if oversized {
        (None, None)
    } else {
        let sha256 = crate::lowercase_hex(Sha256::digest(&bytes));
        (Some(bytes), Some(sha256))
    };
    let snapshot = BoundedFileSnapshot {
        path: path.to_path_buf(),
        description,
        identity: opened_after_identity,
        bytes,
        sha256,
        byte_len,
    };
    snapshot.verify_path()?;
    Ok(snapshot)
}

/// Hash one stable regular file without retaining its contents.
pub fn hash_bounded_regular_file(
    path: impl AsRef<Path>,
    maximum: u64,
    description: impl Into<String>,
) -> Result<(String, u64)> {
    require_exact_snapshot_platform()?;
    let path = path.as_ref();
    let description = description.into();
    let lexical_before = std::fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect {description} {}", path.display()))?;
    let lexical_before_identity = regular_identity(&lexical_before, path, &description)?;
    if lexical_before.len() > maximum {
        bail!(
            "{} {} exceeds the {maximum}-byte limit: observed {} bytes",
            description,
            path.display(),
            lexical_before.len()
        );
    }

    let mut file = open_snapshot(path)
        .with_context(|| format!("failed to open {} {}", description, path.display()))?;
    let opened_before = file
        .metadata()
        .with_context(|| format!("failed to stat opened {} {}", description, path.display()))?;
    let opened_before_identity = regular_identity(&opened_before, path, &description)?;
    if opened_before_identity != lexical_before_identity {
        bail!(
            "{} {} changed between path inspection and descriptor open",
            description,
            path.display()
        );
    }

    let mut hasher = Sha256::new();
    let mut byte_len = 0_u64;
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        let remaining = maximum.saturating_sub(byte_len).saturating_add(1);
        let count = (&mut file)
            .take(remaining.min(chunk.len() as u64))
            .read(&mut chunk)
            .with_context(|| format!("failed to read {} {}", description, path.display()))?;
        if count == 0 {
            break;
        }
        byte_len = byte_len
            .checked_add(count as u64)
            .context("bounded file byte count overflowed u64")?;
        if byte_len > maximum {
            bail!(
                "{} {} exceeds the {maximum}-byte limit: observed at least {byte_len} bytes",
                description,
                path.display()
            );
        }
        hasher.update(&chunk[..count]);
    }

    let opened_after = file.metadata().with_context(|| {
        format!(
            "failed to re-inspect opened {} {}",
            description,
            path.display()
        )
    })?;
    let opened_after_identity = regular_identity(&opened_after, path, &description)?;
    if opened_after_identity != opened_before_identity || byte_len != opened_after.len() {
        bail!(
            "{} {} changed while its descriptor was read",
            description,
            path.display()
        );
    }
    let snapshot = BoundedFileSnapshot {
        path: path.to_path_buf(),
        description,
        identity: opened_after_identity,
        bytes: None,
        sha256: None,
        byte_len,
    };
    snapshot.verify_path()?;
    Ok((crate::lowercase_hex(hasher.finalize()), byte_len))
}

/// Parse one complete JSON document without duplicate object members.
pub fn parse_strict_json<T: DeserializeOwned>(bytes: &[u8], description: &str) -> Result<T> {
    pid_bridge::validate_strict_json_bytes(bytes)
        .with_context(|| format!("{description} is not strict JSON"))?;
    serde_json::from_slice(bytes).with_context(|| format!("failed to parse {description}"))
}

/// Reject duplicate object members in every nonempty JSONL record.
///
/// The run-log parser applies its own line, event, and aggregate limits. Call this helper only on
/// an already bounded snapshot before that typed parse.
pub fn validate_strict_json_lines(bytes: &[u8], description: &str) -> Result<()> {
    for (line_index, raw_line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        let line = raw_line.strip_suffix(b"\r").unwrap_or(raw_line);
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        pid_bridge::validate_strict_json_bytes(line)
            .with_context(|| format!("{description} line {} is not strict JSON", line_index + 1))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_oversized_results_are_distinct() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("input.bin");
        std::fs::write(&path, b"abcd").unwrap();

        let exact = read_bounded_regular_file(&path, 4, "test input").unwrap();
        let expected_sha256 = pid_runlog::sha256_hex(b"abcd");
        assert_eq!(exact.bytes.as_deref(), Some(b"abcd".as_slice()));
        assert_eq!(exact.sha256.as_deref(), Some(expected_sha256.as_str()));
        let error = exact.exact_bytes(3).unwrap_err();
        assert!(error.to_string().contains("exceeds the 3-byte limit"));

        let oversized = read_bounded_regular_file(&path, 3, "test input").unwrap();
        assert!(oversized.bytes.is_none());
        assert!(oversized.sha256.is_none());
        assert_eq!(oversized.byte_len, 4);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_inputs_are_rejected() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target.bin");
        let link = directory.path().join("link.bin");
        std::fs::write(&target, b"data").unwrap();
        symlink(&target, &link).unwrap();

        let error = read_bounded_regular_file(&link, 4, "test input").unwrap_err();
        assert!(error.to_string().contains("non-symlink regular file"));
    }

    #[test]
    fn strict_json_rejects_duplicate_members_in_documents_and_jsonl() {
        let duplicate = br#"{"scope":"first","scope":"second"}"#;

        assert!(parse_strict_json::<serde_json::Value>(duplicate, "test document").is_err());
        assert!(validate_strict_json_lines(duplicate, "test JSONL").is_err());
        assert!(validate_strict_json_lines(b"\n{\"scope\":\"one\"}\r\n", "test JSONL").is_ok());
    }
}
