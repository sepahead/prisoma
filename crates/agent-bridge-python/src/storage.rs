//! Private local files. The caller supplies an existing trusted parent directory.

use anyhow::{ensure, Context, Result};
use rustix::fs::{open, Mode, OFlags};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

pub const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct Identity {
    device: u64,
    inode: u64,
    mode: u32,
    owner: u32,
    links: u64,
    bytes: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl From<&Metadata> for Identity {
    fn from(value: &Metadata) -> Self {
        Self {
            device: value.dev(),
            inode: value.ino(),
            mode: value.mode(),
            owner: value.uid(),
            links: value.nlink(),
            bytes: value.len(),
            modified: (value.mtime(), value.mtime_nsec()),
            changed: (value.ctime(), value.ctime_nsec()),
        }
    }
}

pub fn private_regular(value: &Metadata) -> Result<()> {
    ensure!(value.is_file(), "expected a regular file");
    ensure!(
        value.uid() == rustix::process::geteuid().as_raw(),
        "file owner mismatch"
    );
    ensure!(
        value.mode() & 0o077 == 0,
        "file permissions must be private"
    );
    ensure!(value.nlink() == 1, "file must have one link");
    Ok(())
}

pub fn new_path(path: &Path) -> Result<PathBuf> {
    let name = path.file_name().context("missing file name")?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = fs::canonicalize(parent).context("existing trusted parent required")?;
    ensure!(parent.is_dir(), "parent is not a directory");
    Ok(parent.join(name))
}

pub fn read_private(path: &Path, maximum: u64) -> Result<(File, Identity)> {
    let file = File::from(open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    let metadata = file.metadata()?;
    private_regular(&metadata)?;
    ensure!(metadata.len() <= maximum, "file exceeds byte limit");
    let initial = Identity::from(&metadata);
    rejoin(path, &file, &initial)?;
    Ok((file, initial))
}

pub fn rejoin(path: &Path, file: &File, initial: &Identity) -> Result<()> {
    ensure!(
        *initial == Identity::from(&file.metadata()?)
            && *initial == Identity::from(&fs::symlink_metadata(path)?),
        "file changed during observation"
    );
    Ok(())
}

pub fn artifact_path(log: &Path, name: &str) -> Result<PathBuf> {
    ensure!(
        !name.is_empty() && name.len() <= 128,
        "artifact name length"
    );
    let mut components = Path::new(name).components();
    ensure!(
        matches!(components.next(), Some(Component::Normal(_)))
            && components.next().is_none()
            && !name.contains(['/', '\\']),
        "artifact must be a single relative file name"
    );
    let path = log.parent().context("log parent")?.join(name);
    ensure!(path != log, "artifact cannot reference its own run log");
    Ok(path)
}

pub fn artifact_digest(path: &Path) -> Result<(u64, String)> {
    let (mut file, initial) = read_private(path, MAX_ARTIFACT_BYTES)?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .context("artifact byte overflow")?;
        ensure!(total <= MAX_ARTIFACT_BYTES, "artifact exceeds byte limit");
        hasher.update(&buffer[..count]);
    }
    ensure!(total == initial.bytes, "artifact size changed");
    rejoin(path, &file, &initial)?;
    let mut hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(hex, "{byte:02x}")?;
    }
    Ok((total, hex))
}

pub struct SynchronizedFile {
    file: File,
    path: PathBuf,
    written: u64,
}

impl SynchronizedFile {
    pub fn create(path: &Path) -> Result<Self> {
        let file = File::from(open(
            path,
            OFlags::WRONLY
                | OFlags::CREATE
                | OFlags::EXCL
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )?);
        private_regular(&file.metadata()?)?;
        file.sync_all()?;
        File::open(path.parent().context("log parent")?)?.sync_all()?;
        Ok(Self {
            file,
            path: path.to_owned(),
            written: 0,
        })
    }

    fn synchronize(&mut self) -> Result<()> {
        self.file.flush()?;
        self.file.sync_all()?;
        let metadata = self.file.metadata()?;
        private_regular(&metadata)?;
        ensure!(metadata.len() == self.written, "unexpected log size");
        rejoin(&self.path, &self.file, &Identity::from(&metadata))
    }
}

impl Write for SynchronizedFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = self.file.write(bytes)?;
        self.written = self
            .written
            .checked_add(count as u64)
            .ok_or_else(|| io::Error::other("log byte overflow"))?;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.synchronize().map_err(io::Error::other)
    }
}
