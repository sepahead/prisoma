use std::{
    fmt,
    io::{self, Read},
    process::{Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicU8, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(5);
const STATE_RUNNING: u8 = 0;
const STATE_STDOUT_OVERFLOW: u8 = 1;
const STATE_STDERR_OVERFLOW: u8 = 2;
const STATE_STDOUT_READ_FAILED: u8 = 3;
const STATE_STDERR_READ_FAILED: u8 = 4;

#[derive(Debug)]
pub(crate) struct BoundedOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputStream {
    Stdout,
    Stderr,
}

impl fmt::Display for OutputStream {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdout => formatter.write_str("stdout"),
            Self::Stderr => formatter.write_str("stderr"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum BoundedCommandError {
    Spawn(io::Error),
    ReaderSpawn {
        stream: OutputStream,
        source: io::Error,
    },
    Wait(io::Error),
    Timeout(Duration),
    OutputLimit {
        stream: OutputStream,
        limit: usize,
    },
    Read {
        stream: OutputStream,
        source: io::Error,
    },
    ReaderDisconnected(OutputStream),
}

impl fmt::Display for BoundedCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(error) => write!(formatter, "failed to spawn command: {error}"),
            Self::ReaderSpawn { stream, source } => {
                write!(formatter, "failed to spawn {stream} reader: {source}")
            }
            Self::Wait(error) => write!(formatter, "failed while waiting for command: {error}"),
            Self::Timeout(timeout) => {
                write!(formatter, "command exceeded its {:?} deadline", timeout)
            }
            Self::OutputLimit { stream, limit } => {
                write!(
                    formatter,
                    "command {stream} exceeded the {limit}-byte limit"
                )
            }
            Self::Read { stream, source } => {
                write!(formatter, "failed to read command {stream}: {source}")
            }
            Self::ReaderDisconnected(stream) => {
                write!(formatter, "command {stream} reader disconnected")
            }
        }
    }
}

impl std::error::Error for BoundedCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn(error)
            | Self::Wait(error)
            | Self::ReaderSpawn { source: error, .. }
            | Self::Read { source: error, .. } => Some(error),
            Self::Timeout(_) | Self::OutputLimit { .. } | Self::ReaderDisconnected(_) => None,
        }
    }
}

enum ReaderResult {
    Complete {
        stream: OutputStream,
        bytes: Vec<u8>,
    },
    Failed {
        stream: OutputStream,
        error: io::Error,
    },
}

fn signal_state(state: &AtomicU8, value: u8) {
    let _ = state.compare_exchange(STATE_RUNNING, value, Ordering::AcqRel, Ordering::Acquire);
}

fn read_stream<R: Read>(
    mut reader: R,
    stream: OutputStream,
    limit: usize,
    state: &AtomicU8,
) -> ReaderResult {
    let mut bytes = Vec::with_capacity(limit.min(8 * 1024));
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return ReaderResult::Complete { stream, bytes },
            Ok(count) => {
                let remaining = limit.saturating_sub(bytes.len());
                if count > remaining {
                    bytes.extend_from_slice(&chunk[..remaining]);
                    signal_state(
                        state,
                        match stream {
                            OutputStream::Stdout => STATE_STDOUT_OVERFLOW,
                            OutputStream::Stderr => STATE_STDERR_OVERFLOW,
                        },
                    );
                    return ReaderResult::Complete { stream, bytes };
                }
                bytes.extend_from_slice(&chunk[..count]);
            }
            Err(error) => {
                signal_state(
                    state,
                    match stream {
                        OutputStream::Stdout => STATE_STDOUT_READ_FAILED,
                        OutputStream::Stderr => STATE_STDERR_READ_FAILED,
                    },
                );
                return ReaderResult::Failed { stream, error };
            }
        }
    }
}

fn kill_and_reap(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn state_error(state: u8, limit: usize) -> Option<BoundedCommandError> {
    match state {
        STATE_RUNNING => None,
        STATE_STDOUT_OVERFLOW => Some(BoundedCommandError::OutputLimit {
            stream: OutputStream::Stdout,
            limit,
        }),
        STATE_STDERR_OVERFLOW => Some(BoundedCommandError::OutputLimit {
            stream: OutputStream::Stderr,
            limit,
        }),
        STATE_STDOUT_READ_FAILED => Some(BoundedCommandError::ReaderDisconnected(
            OutputStream::Stdout,
        )),
        STATE_STDERR_READ_FAILED => Some(BoundedCommandError::ReaderDisconnected(
            OutputStream::Stderr,
        )),
        _ => Some(BoundedCommandError::ReaderDisconnected(
            OutputStream::Stdout,
        )),
    }
}

pub(crate) fn run_bounded(
    command: &mut Command,
    timeout: Duration,
    output_limit: usize,
) -> Result<BoundedOutput, BoundedCommandError> {
    let started = Instant::now();
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(BoundedCommandError::Spawn)?;
    let stdout = child.stdout.take().ok_or_else(|| {
        kill_and_reap(&mut child);
        BoundedCommandError::ReaderDisconnected(OutputStream::Stdout)
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        kill_and_reap(&mut child);
        BoundedCommandError::ReaderDisconnected(OutputStream::Stderr)
    })?;
    let state = Arc::new(AtomicU8::new(STATE_RUNNING));
    let (sender, receiver) = mpsc::channel();

    let stdout_state = Arc::clone(&state);
    let stdout_sender = sender.clone();
    if let Err(error) = thread::Builder::new()
        .name("ncp-probe-stdout".to_owned())
        .spawn(move || {
            let result = read_stream(stdout, OutputStream::Stdout, output_limit, &stdout_state);
            let _ = stdout_sender.send(result);
        })
    {
        kill_and_reap(&mut child);
        return Err(BoundedCommandError::ReaderSpawn {
            stream: OutputStream::Stdout,
            source: error,
        });
    }

    let stderr_state = Arc::clone(&state);
    if let Err(error) = thread::Builder::new()
        .name("ncp-probe-stderr".to_owned())
        .spawn(move || {
            let result = read_stream(stderr, OutputStream::Stderr, output_limit, &stderr_state);
            let _ = sender.send(result);
        })
    {
        kill_and_reap(&mut child);
        return Err(BoundedCommandError::ReaderSpawn {
            stream: OutputStream::Stderr,
            source: error,
        });
    }

    let status = loop {
        if let Some(error) = state_error(state.load(Ordering::Acquire), output_limit) {
            kill_and_reap(&mut child);
            return Err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                kill_and_reap(&mut child);
                return Err(BoundedCommandError::Wait(error));
            }
        }
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            kill_and_reap(&mut child);
            return Err(BoundedCommandError::Timeout(timeout));
        }
        thread::sleep(POLL_INTERVAL.min(timeout - elapsed));
    };

    let mut stdout = None;
    let mut stderr_complete = false;
    while stdout.is_none() || !stderr_complete {
        if let Some(error) = state_error(state.load(Ordering::Acquire), output_limit) {
            return Err(error);
        }
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            return Err(BoundedCommandError::Timeout(timeout));
        }
        let remaining = timeout - elapsed;
        let result = receiver
            .recv_timeout(remaining)
            .map_err(|_| BoundedCommandError::Timeout(timeout))?;
        match result {
            ReaderResult::Complete {
                stream: OutputStream::Stdout,
                bytes,
            } => stdout = Some(bytes),
            ReaderResult::Complete {
                stream: OutputStream::Stderr,
                ..
            } => stderr_complete = true,
            ReaderResult::Failed { stream, error } => {
                return Err(BoundedCommandError::Read {
                    stream,
                    source: error,
                });
            }
        }
    }

    Ok(BoundedOutput {
        status,
        stdout: stdout.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    const FIXTURE_ENV: &str = "PRISOMA_NCP_BOUNDED_PROCESS_FIXTURE";

    fn fixture_command(mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "bounded_process::tests::fixture_process",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(FIXTURE_ENV, mode);
        command
    }

    #[test]
    fn fixture_process() {
        match std::env::var(FIXTURE_ENV).as_deref() {
            Ok("both") => {
                std::io::stdout().write_all(&vec![b'o'; 64 * 1024]).unwrap();
                std::io::stderr().write_all(&vec![b'e'; 64 * 1024]).unwrap();
            }
            Ok("overflow") => {
                std::io::stdout().write_all(&vec![b'x'; 64 * 1024]).unwrap();
            }
            Ok("stderr_overflow") => {
                std::io::stderr().write_all(&vec![b'x'; 64 * 1024]).unwrap();
            }
            Ok("timeout") => thread::sleep(Duration::from_secs(10)),
            Ok(other) => panic!("unknown fixture mode {other}"),
            Err(_) => {}
        }
    }

    #[test]
    fn run_bounded_drains_stdout_and_stderr_concurrently() {
        let output = run_bounded(
            &mut fixture_command("both"),
            Duration::from_secs(2),
            128 * 1024,
        )
        .unwrap();

        assert!(output.status.success());
    }

    #[test]
    fn run_bounded_rejects_output_over_limit() {
        let error = run_bounded(
            &mut fixture_command("overflow"),
            Duration::from_secs(2),
            1024,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            BoundedCommandError::OutputLimit {
                stream: OutputStream::Stdout,
                limit: 1024
            }
        ));
    }

    #[test]
    fn run_bounded_rejects_stderr_over_limit() {
        let error = run_bounded(
            &mut fixture_command("stderr_overflow"),
            Duration::from_secs(2),
            1024,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            BoundedCommandError::OutputLimit {
                stream: OutputStream::Stderr,
                limit: 1024
            }
        ));
    }

    #[test]
    fn run_bounded_kills_and_reaps_after_deadline() {
        let started = Instant::now();
        let error = run_bounded(
            &mut fixture_command("timeout"),
            Duration::from_millis(50),
            1024,
        )
        .unwrap_err();

        assert!(matches!(error, BoundedCommandError::Timeout(_)));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
