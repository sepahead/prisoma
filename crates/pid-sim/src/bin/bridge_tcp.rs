use anyhow::{bail, Context, Result};
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:38472";
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(30);
const UNIQUE_RUN_LOG_CREATE_ATTEMPTS: u16 = 128;
/// Window for answering connections queued while the bound connection ran. A
/// replay socket already in the listen backlog is accepted on the first poll.
const POST_PAIRING_DRAIN: Duration = Duration::from_millis(250);
const POST_PAIRING_POLL: Duration = Duration::from_millis(5);

#[derive(Debug, Eq, PartialEq)]
enum RunLogTarget {
    Exact(PathBuf),
    UniqueInDirectory(PathBuf),
}

#[derive(Debug, Eq, PartialEq)]
struct BridgeArgs {
    safe_mode: bool,
    engram_host: bool,
    bind_addr: SocketAddr,
    run_log_target: RunLogTarget,
}

enum ParsedCommand {
    Run(BridgeArgs),
    Help { program: String },
}

fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let args = match parse_args(&args)? {
        ParsedCommand::Run(args) => args,
        ParsedCommand::Help { program } => {
            eprintln!("{}", usage(&program));
            return Ok(());
        }
    };
    let BridgeArgs {
        safe_mode,
        engram_host,
        bind_addr,
        run_log_target,
    } = args;

    validate_bind_addr(bind_addr)?;
    // Generate the startup pairing secret before the listener exists. A CSPRNG
    // failure must never leave an unauthenticated loopback listener open.
    let pairing_secret = if engram_host {
        Some(
            pid_sim::PairingSecret::generate()
                .context("failed to generate the bridge pairing secret")?,
        )
    } else {
        None
    };
    let listener =
        TcpListener::bind(bind_addr).with_context(|| format!("failed to bind {bind_addr}"))?;
    let local_addr = listener
        .local_addr()
        .context("failed to read local address")?;
    let PreparedRunLog {
        path,
        artifact_root,
        run_id,
        writer,
    } = prepare_run_log(&run_log_target)?;
    let config = pid_sim::deterministic_sim_config(
        "pid-sim-bridge-tcp",
        Some("tcp_jsonl"),
        None,
        None,
        Some(safe_mode),
    );
    let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "pid-sim-bridge-tcp".to_string());
    metadata.insert("safe_mode".to_string(), safe_mode.to_string());
    metadata.insert(
        "active_profile".to_string(),
        if engram_host {
            pid_sim::ENGRAM_HOST_BRIDGE_PROFILE
        } else if safe_mode {
            pid_sim::SAFE_MODE_BRIDGE_PROFILE
        } else {
            pid_sim::STANDARD_BRIDGE_PROFILE
        }
        .to_string(),
    );
    metadata.insert("bind_addr".to_string(), local_addr.to_string());
    metadata.insert("requested_bind_addr".to_string(), bind_addr.to_string());
    metadata.insert(
        "artifact_root".to_string(),
        artifact_root.display().to_string(),
    );
    // The mechanism is public; the secret itself is never written to provenance.
    metadata.insert(
        "pairing_mechanism".to_string(),
        if engram_host {
            pid_sim::PAIRING_MECHANISM
        } else {
            "none"
        }
        .to_string(),
    );
    let run_started = RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: run_id.clone(),
        timestamp_ns: 0,
        config_hash: config_hash.clone(),
        metadata,
    };
    let config_logged = RunLogEvent::ConfigLogged {
        timestamp_ns: 0,
        config_hash,
        config,
    };

    let sim = pid_sim::demo_sim();
    let initial_snapshot = sim.snapshot_event();
    let mut session = if engram_host {
        pid_sim::SimBridgeSession::with_engram_host_profile_and_run_id(writer, sim, &run_id)
    } else {
        pid_sim::SimBridgeSession::with_safe_mode_and_run_id(writer, sim, safe_mode, &run_id)
    };
    let announcement_token = match pairing_secret {
        Some(secret) => {
            let token = secret.announcement_token();
            session.enable_engram_pairing(secret, pid_sim::MAX_PAIRING_ATTEMPTS);
            Some(token)
        }
        None => None,
    };
    append_session_prefix(
        &mut session,
        &[run_started, config_logged, initial_snapshot],
    )?;
    session.set_run_log_path(&path);
    session.set_artifact_root(&artifact_root)?;
    // Detect buffered provenance-storage failures before advertising the
    // listener or accepting a control client.
    session.flush()?;
    // The startup secret appears exactly once, on this stderr line, and nowhere
    // else: not in the run log, not in a response, not in any file.
    match &announcement_token {
        Some(token) => eprintln!("run_log={} pairing={token}", path.display()),
        None => eprintln!("run_log={}", path.display()),
    }
    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: "pid-sim-bridge-tcp".to_string(),
        session_id: Some("bridge-tcp".to_string()),
    };

    eprintln!("listening {local_addr}");
    let served = serve_connections(&listener, &mut session, &actor);

    // When provenance storage remains writable, always seal accepted-client
    // transport errors as Failed. A provenance-storage error itself may leave
    // a partial/unreadable log and cannot be repaired by this transport.
    let (status, message) = match &served {
        Ok(served) if served.pairing_required && !served.paired => (
            RunStatus::Failed,
            format!(
                "pairing rejected every one of {} accepted connection(s); the startup secret never bound",
                served.connections
            ),
        ),
        Ok(served) => (
            RunStatus::Succeeded,
            format!(
                "processed {} request(s) from {} connection(s)",
                served.handled, served.connections
            ),
        ),
        Err(err) => (RunStatus::Failed, format!("TCP transport error: {err:#}")),
    };
    session.finish_run(status, Some(message))?;
    session.flush()?;
    eprintln!("wrote {}", path.display());
    served.map(|_| ())
}

/// Outcome of one bridge run's accepted connections.
#[derive(Debug, Default, Eq, PartialEq)]
struct ServedConnections {
    handled: usize,
    connections: usize,
    pairing_required: bool,
    paired: bool,
}

fn serve_connections<W: Write>(
    listener: &TcpListener,
    session: &mut pid_sim::SimBridgeSession<W>,
    actor: &Actor,
) -> Result<ServedConnections> {
    if !session.pairing_required() {
        let (stream, peer_addr) = listener.accept().context("failed to accept TCP client")?;
        eprintln!("accepted {peer_addr}");
        let handled = serve_client(stream, session, actor, None, false)?;
        return Ok(ServedConnections {
            handled,
            connections: 1,
            pairing_required: false,
            paired: false,
        });
    }

    let mut served = ServedConnections {
        pairing_required: true,
        ..ServedConnections::default()
    };
    // Serve accepted connections until one of them proves possession of the
    // startup secret and finishes, or the finite attempt budget runs out.
    while !session.pairing_bound()
        && !session.pairing_latched()
        && !session.pairing_attempts_exhausted()
    {
        let (stream, peer_addr) = listener.accept().context("failed to accept TCP client")?;
        eprintln!("accepted {peer_addr}");
        served.connections += 1;
        session.begin_pairing_attempt()?;
        let mut pairing = pid_sim::ConnectionPairing::new();
        let result = serve_client(stream, session, actor, Some(&mut pairing), false);
        served.handled += finish_pairing_connection(result, &pairing, session)?;
        if pairing.accepted() {
            served.paired = true;
        }
    }
    if served.paired {
        served.handled +=
            drain_post_pairing_connections(listener, session, actor, &mut served.connections)?;
    }
    Ok(served)
}

/// Answer connections that were queued while the bound connection was served.
///
/// After binding, a captured-request replay on a new socket must observe the
/// pairing rejection rather than a silent close. A connection already waiting in
/// the listen backlog is accepted immediately; the short deadline only covers
/// accept scheduling, so the bridge still terminates without a client.
fn drain_post_pairing_connections<W: Write>(
    listener: &TcpListener,
    session: &mut pid_sim::SimBridgeSession<W>,
    actor: &Actor,
    connections: &mut usize,
) -> Result<usize> {
    listener
        .set_nonblocking(true)
        .context("failed to set the listener non-blocking")?;
    let deadline = Instant::now() + POST_PAIRING_DRAIN;
    let mut handled = 0usize;
    loop {
        if session.pairing_attempts_exhausted() || Instant::now() >= deadline {
            break;
        }
        match listener.accept() {
            Ok((stream, peer_addr)) => {
                stream
                    .set_nonblocking(false)
                    .context("failed to set the accepted stream blocking")?;
                eprintln!("accepted {peer_addr}");
                *connections += 1;
                session.begin_pairing_attempt()?;
                let mut pairing = pid_sim::ConnectionPairing::new();
                let result = serve_client(stream, session, actor, Some(&mut pairing), true);
                handled += finish_pairing_connection(result, &pairing, session)?;
                // One post-binding rejection is enough evidence; do not hold the
                // run open waiting for more.
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(POST_PAIRING_POLL);
            }
            Err(error) => {
                return Err(error).context("failed to accept a post-pairing TCP client");
            }
        }
    }
    listener
        .set_nonblocking(false)
        .context("failed to restore the blocking listener")?;
    Ok(handled)
}

/// Complete one accepted-connection pairing unit without hiding fatal state.
///
/// A connection-local failure before proof acceptance, including a read
/// timeout, consumes the unit and lets the finite pairing loop continue. An
/// error after proof acceptance or provenance poisoning remains fatal.
fn finish_pairing_connection<W: Write>(
    result: Result<usize>,
    pairing: &pid_sim::ConnectionPairing,
    session: &mut pid_sim::SimBridgeSession<W>,
) -> Result<usize> {
    match result {
        Ok(handled) => {
            if !pairing.accepted() {
                session.record_failed_pairing_attempt();
            }
            Ok(handled)
        }
        Err(error) if !pairing.accepted() && !session.poisoned() => {
            session.record_failed_pairing_attempt();
            eprintln!("pairing connection ended before proof acceptance: {error:#}");
            Ok(0)
        }
        Err(error) => Err(error),
    }
}

fn serve_client<W: Write>(
    stream: std::net::TcpStream,
    session: &mut pid_sim::SimBridgeSession<W>,
    actor: &Actor,
    pairing: Option<&mut pid_sim::ConnectionPairing>,
    bound_elsewhere: bool,
) -> Result<usize> {
    configure_client_stream(&stream)?;
    let reader = BufReader::new(stream.try_clone().context("failed to clone TCP stream")?);
    let mut output = BufWriter::new(stream);
    let handled = pid_sim::dispatch_rpc_lines_paired(
        reader,
        &mut output,
        session,
        actor.clone(),
        pairing,
        bound_elsewhere,
    )?;
    output.flush().context("failed to flush TCP responses")?;
    Ok(handled)
}

fn parse_args(args: &[OsString]) -> Result<ParsedCommand> {
    let program = args
        .first()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pid-sim-bridge-tcp".to_string());
    let mut policy = None;
    let mut engram_host = false;
    let mut bind_addr = DEFAULT_BIND_ADDR.to_string();
    let mut path_arg = None;
    let mut unique_run_log_dir = None;
    let mut idx = 1;
    while idx < args.len() {
        match args[idx].to_str() {
            Some("--safe-mode") => {
                if policy == Some(false) {
                    bail!(
                        "--safe-mode conflicts with --allow-mutations\n{}",
                        usage(&program)
                    );
                }
                policy = Some(true);
                idx += 1;
            }
            Some("--allow-mutations") => {
                if engram_host {
                    bail!(
                        "--allow-mutations conflicts with --engram-host\n{}",
                        usage(&program)
                    );
                }
                if policy == Some(true) {
                    bail!(
                        "--allow-mutations conflicts with --safe-mode\n{}",
                        usage(&program)
                    );
                }
                policy = Some(false);
                idx += 1;
            }
            Some("--engram-host") => {
                if policy == Some(false) {
                    bail!(
                        "--engram-host conflicts with --allow-mutations\n{}",
                        usage(&program)
                    );
                }
                engram_host = true;
                policy = Some(true);
                idx += 1;
            }
            Some("--bind") => {
                idx += 1;
                let Some(value) = args.get(idx).and_then(|value| value.to_str()) else {
                    bail!("--bind requires an address");
                };
                bind_addr = value.to_string();
                idx += 1;
            }
            Some("--unique-run-log-dir") => {
                if unique_run_log_dir.is_some() {
                    bail!(
                        "--unique-run-log-dir can be specified only once\n{}",
                        usage(&program)
                    );
                }
                idx += 1;
                let Some(value) = args.get(idx) else {
                    bail!("--unique-run-log-dir requires a directory");
                };
                unique_run_log_dir = Some(PathBuf::from(value));
                idx += 1;
            }
            Some("-h" | "--help") => {
                return Ok(ParsedCommand::Help { program });
            }
            Some(value) if !value.starts_with('-') && path_arg.is_none() => {
                path_arg = args.get(idx).cloned();
                idx += 1;
            }
            _ => bail!("{}", usage(&program)),
        }
    }
    if unique_run_log_dir.is_some() && !engram_host {
        bail!(
            "--unique-run-log-dir requires --engram-host\n{}",
            usage(&program)
        );
    }
    let run_log_target = match (path_arg, unique_run_log_dir) {
        (Some(path), None) => RunLogTarget::Exact(PathBuf::from(path)),
        (None, Some(directory)) => RunLogTarget::UniqueInDirectory(directory),
        (Some(_), Some(_)) => bail!(
            "choose either an exact run-log path or --unique-run-log-dir\n{}",
            usage(&program)
        ),
        (None, None) => bail!("{}", usage(&program)),
    };
    let bind_addr: SocketAddr = bind_addr
        .parse()
        .with_context(|| format!("invalid bind address {bind_addr}"))?;
    Ok(ParsedCommand::Run(BridgeArgs {
        safe_mode: policy.unwrap_or(true),
        engram_host,
        bind_addr,
        run_log_target,
    }))
}

fn usage(program: &str) -> String {
    format!(
        "usage: {program} [--safe-mode | --allow-mutations | --engram-host] [--bind LOOPBACK_ADDR] \
         (<run-log.jsonl> | --unique-run-log-dir DIR)\n\
         mutations are disabled by default; --engram-host selects the strict Engram read-only profile;\n\
         --unique-run-log-dir atomically creates a no-clobber engram-host-*.jsonl file for that profile"
    )
}

fn validate_bind_addr(bind_addr: SocketAddr) -> Result<()> {
    if !bind_addr.ip().is_loopback() {
        bail!("refusing non-loopback bind address {bind_addr}");
    }
    Ok(())
}

fn prepare_artifact_path(path: &Path) -> Result<(PathBuf, PathBuf)> {
    pid_sim::canonical_new_artifact_path(path)
}

struct PreparedRunLog {
    path: PathBuf,
    artifact_root: PathBuf,
    run_id: String,
    writer: RunLogWriter<pid_sim::FsyncFileWriter>,
}

fn prepare_run_log(target: &RunLogTarget) -> Result<PreparedRunLog> {
    match target {
        RunLogTarget::Exact(requested_path) => {
            let (path, artifact_root) = prepare_artifact_path(requested_path)?;
            let writer = create_run_log(&path)?;
            Ok(PreparedRunLog {
                path,
                artifact_root,
                run_id: "bridge-tcp-run".to_string(),
                writer,
            })
        }
        RunLogTarget::UniqueInDirectory(requested_directory) => {
            let token = unique_run_log_token()?;
            prepare_unique_run_log(requested_directory, &token)
        }
    }
}

fn unique_run_log_token() -> Result<String> {
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos();
    Ok(format!("{timestamp_ns:x}-{}", std::process::id()))
}

fn prepare_unique_run_log(requested_directory: &Path, token: &str) -> Result<PreparedRunLog> {
    if token.is_empty()
        || token.len() > 64
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        bail!("unique run-log token is invalid");
    }
    let placeholder = requested_directory.join("engram-host-placeholder.jsonl");
    let (_, artifact_root) = prepare_artifact_path(&placeholder)?;
    for attempt in 0..UNIQUE_RUN_LOG_CREATE_ATTEMPTS {
        let run_id = format!("engram-host-{token}-{attempt:03}");
        let path = artifact_root.join(format!("{run_id}.jsonl"));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                return Ok(PreparedRunLog {
                    path,
                    artifact_root,
                    run_id,
                    writer: run_log_writer(file),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to create unique run log {}", path.display())
                });
            }
        }
    }
    bail!(
        "failed to create a unique run log in {} after {} collision attempts",
        artifact_root.display(),
        UNIQUE_RUN_LOG_CREATE_ATTEMPTS
    )
}

fn create_run_log(path: &Path) -> Result<RunLogWriter<pid_sim::FsyncFileWriter>> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create new run log {}", path.display()))?;
    Ok(run_log_writer(file))
}

fn run_log_writer(file: File) -> RunLogWriter<pid_sim::FsyncFileWriter> {
    RunLogWriter::new(pid_sim::FsyncFileWriter::new(file))
}

fn append_session_prefix<W: Write>(
    session: &mut pid_sim::SimBridgeSession<W>,
    events: &[RunLogEvent],
) -> Result<()> {
    for event in events {
        session.record_event(event)?;
    }
    Ok(())
}

fn configure_client_stream(stream: &std::net::TcpStream) -> Result<()> {
    stream
        .set_read_timeout(Some(CLIENT_IO_TIMEOUT))
        .context("failed to set TCP client read timeout")?;
    stream
        .set_write_timeout(Some(CLIENT_IO_TIMEOUT))
        .context("failed to set TCP client write timeout")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pid_bridge::{BridgeMethod, BridgeRequest};
    use std::net::TcpStream;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_PATH: AtomicU64 = AtomicU64::new(0);

    fn test_args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn parsed_run(values: &[&str]) -> BridgeArgs {
        match parse_args(&test_args(values)).expect("arguments should parse") {
            ParsedCommand::Run(args) => args,
            ParsedCommand::Help { .. } => panic!("expected runnable arguments"),
        }
    }

    fn unique_test_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "prisoma-bridge-tcp-{}-{}.jsonl",
            std::process::id(),
            NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn connected_stream() -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
        let client = TcpStream::connect(
            listener
                .local_addr()
                .expect("test listener address should resolve"),
        )
        .expect("test client should connect");
        let (server, _) = listener.accept().expect("test listener should accept");
        drop(client);
        server
    }

    #[test]
    fn parse_args_defaults_to_safe_mode() {
        let args = parsed_run(&["bridge-tcp", "runlog.jsonl"]);

        assert!(args.safe_mode);
        assert!(!args.engram_host);
        assert_eq!(
            args.run_log_target,
            RunLogTarget::Exact(PathBuf::from("runlog.jsonl"))
        );
    }

    #[test]
    fn parse_args_allows_explicit_mutation_opt_in() {
        let args = parsed_run(&["bridge-tcp", "--allow-mutations", "runlog.jsonl"]);

        assert!(!args.safe_mode);
        assert!(!args.engram_host);
    }

    #[test]
    fn parse_args_retains_explicit_safe_mode_flag() {
        let args = parsed_run(&["bridge-tcp", "--safe-mode", "runlog.jsonl"]);

        assert!(args.safe_mode);
        assert!(!args.engram_host);
    }

    #[test]
    fn parse_args_engram_host_implies_safe_mode() {
        let args = parsed_run(&["bridge-tcp", "--engram-host", "runlog.jsonl"]);

        assert!(args.safe_mode);
        assert!(args.engram_host);
    }

    #[test]
    fn parse_args_engram_host_accepts_redundant_safe_mode() {
        let args = parsed_run(&["bridge-tcp", "--safe-mode", "--engram-host", "runlog.jsonl"]);

        assert!(args.safe_mode);
        assert!(args.engram_host);
    }

    #[test]
    fn parse_args_accepts_an_auto_unique_run_log_directory() {
        for values in [
            [
                "bridge-tcp",
                "--engram-host",
                "--unique-run-log-dir",
                "outputs",
            ],
            [
                "bridge-tcp",
                "--unique-run-log-dir",
                "outputs",
                "--engram-host",
            ],
        ] {
            let args = parsed_run(&values);
            assert!(args.safe_mode);
            assert!(args.engram_host);
            assert_eq!(
                args.run_log_target,
                RunLogTarget::UniqueInDirectory(PathBuf::from("outputs"))
            );
        }
    }

    #[test]
    fn parse_args_requires_exactly_one_run_log_target() {
        for values in [
            vec!["bridge-tcp"],
            vec!["bridge-tcp", "--unique-run-log-dir"],
            vec![
                "bridge-tcp",
                "--unique-run-log-dir",
                "outputs",
                "runlog.jsonl",
            ],
            vec![
                "bridge-tcp",
                "--unique-run-log-dir",
                "outputs",
                "--unique-run-log-dir",
                "other",
            ],
            vec!["bridge-tcp", "--unique-run-log-dir", "outputs"],
        ] {
            assert!(
                parse_args(&test_args(&values)).is_err(),
                "arguments must fail: {values:?}"
            );
        }
    }

    #[test]
    fn parse_args_rejects_mutation_opt_in_with_engram_host_in_any_order() {
        for values in [
            [
                "bridge-tcp",
                "--engram-host",
                "--allow-mutations",
                "runlog.jsonl",
            ],
            [
                "bridge-tcp",
                "--allow-mutations",
                "--engram-host",
                "runlog.jsonl",
            ],
        ] {
            let error = match parse_args(&test_args(&values)) {
                Err(error) => error,
                Ok(_) => panic!("Engram host profile must conflict with mutations"),
            };

            assert!(error.to_string().contains("conflicts"), "{error:#}");
        }
    }

    #[test]
    fn validate_bind_addr_rejects_non_loopback_ipv4() {
        let addr = "0.0.0.0:38472".parse().expect("address should parse");

        assert!(validate_bind_addr(addr).is_err());
    }

    #[test]
    fn validate_bind_addr_rejects_non_loopback_ipv6() {
        let addr = "[::]:38472".parse().expect("address should parse");

        assert!(validate_bind_addr(addr).is_err());
    }

    #[test]
    fn create_run_log_preserves_existing_target() {
        let path = unique_test_path();
        std::fs::write(&path, b"sentinel").expect("test target should be created");

        assert!(create_run_log(&path).is_err());
        let contents = std::fs::read(&path).expect("test target should remain readable");
        std::fs::remove_file(&path).expect("test target should be removable");
        assert_eq!(contents, b"sentinel");
    }

    #[test]
    fn unique_run_log_directory_creates_distinct_no_clobber_outputs() {
        let directory = tempfile::tempdir().expect("test output directory should exist");
        let token = "deterministic-test-token";
        let existing_path = directory
            .path()
            .join(format!("engram-host-{token}-000.jsonl"));
        std::fs::write(&existing_path, b"sentinel").expect("collision fixture should be created");

        let first = prepare_unique_run_log(directory.path(), token)
            .expect("first unique run log should be created");
        let first_path = first.path.clone();
        let first_run_id = first.run_id.clone();
        assert_eq!(
            first_path.parent(),
            Some(first.artifact_root.as_path()),
            "the generated file must remain in its canonical artifact root"
        );
        assert_eq!(
            first_path.file_stem().and_then(|value| value.to_str()),
            Some(first_run_id.as_str())
        );
        assert_eq!(first_run_id, format!("engram-host-{token}-001"));
        assert_eq!(
            first_path.extension().and_then(|value| value.to_str()),
            Some("jsonl")
        );
        drop(first.writer);

        let second = prepare_unique_run_log(directory.path(), token)
            .expect("second unique run log should be created");
        let second_path = second.path.clone();
        assert_eq!(second.run_id, format!("engram-host-{token}-002"));
        drop(second.writer);

        assert_ne!(second_path, first_path);
        assert!(first_path.is_file());
        assert!(second_path.is_file());
        assert_eq!(
            std::fs::read(existing_path).expect("collision fixture must remain readable"),
            b"sentinel"
        );
    }

    #[test]
    fn unique_run_log_directory_rejects_invalid_roots_and_tokens() {
        let directory = tempfile::tempdir().expect("test output directory should exist");
        let regular_file = directory.path().join("not-a-directory");
        std::fs::write(&regular_file, b"file").expect("regular-file fixture should be created");

        assert!(prepare_unique_run_log(&directory.path().join("missing"), "valid-token").is_err());
        assert!(prepare_unique_run_log(&regular_file, "valid-token").is_err());
        assert!(prepare_unique_run_log(directory.path(), "../escape").is_err());
        assert!(prepare_unique_run_log(directory.path(), "").is_err());
    }

    #[test]
    fn unique_run_log_directory_fails_after_bounded_collisions_without_clobbering() {
        let directory = tempfile::tempdir().expect("test output directory should exist");
        let token = "exhaustion-test-token";
        let mut collision_paths = Vec::new();
        for attempt in 0..UNIQUE_RUN_LOG_CREATE_ATTEMPTS {
            let path = directory
                .path()
                .join(format!("engram-host-{token}-{attempt:03}.jsonl"));
            std::fs::write(&path, b"sentinel").expect("collision fixture should be created");
            collision_paths.push(path);
        }

        let error = match prepare_unique_run_log(directory.path(), token) {
            Err(error) => error,
            Ok(_) => panic!("the bounded collision set must be exhausted"),
        };
        assert!(
            error.to_string().contains("128 collision attempts"),
            "{error:#}"
        );
        for path in collision_paths {
            assert_eq!(
                std::fs::read(path).expect("collision fixture must remain readable"),
                b"sentinel"
            );
        }
    }

    #[test]
    fn configure_client_stream_sets_bounded_io_timeouts() {
        let stream = connected_stream();

        configure_client_stream(&stream).expect("timeouts should be configurable");

        assert_eq!(
            (
                stream
                    .read_timeout()
                    .expect("read timeout should be readable"),
                stream
                    .write_timeout()
                    .expect("write timeout should be readable")
            ),
            (Some(CLIENT_IO_TIMEOUT), Some(CLIENT_IO_TIMEOUT))
        );
    }

    #[test]
    fn unpaired_transport_failure_consumes_the_attempt_without_poisoning_the_run() {
        let writer = RunLogWriter::new(Vec::new());
        let mut session = pid_sim::SimBridgeSession::with_engram_host_profile_and_run_id(
            writer,
            pid_sim::demo_sim(),
            "pairing-timeout",
        );
        session.enable_engram_pairing(pid_sim::PairingSecret::from_bytes([7; 32]), 1);
        session
            .begin_pairing_attempt()
            .expect("the first pairing unit should be available");
        let pairing = pid_sim::ConnectionPairing::new();

        let handled = finish_pairing_connection(
            Err(anyhow::anyhow!("simulated read timeout")),
            &pairing,
            &mut session,
        )
        .expect("an unpaired connection error should be recoverable");

        assert_eq!(
            (
                handled,
                session.pairing_attempts_used(),
                session.pairing_latched(),
                session.poisoned(),
            ),
            (0, 1, true, false)
        );
    }

    #[test]
    fn unpaired_failure_does_not_hide_a_poisoned_provenance_log() {
        struct FailingWriter;

        impl Write for FailingWriter {
            fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("simulated provenance failure"))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let writer = RunLogWriter::new(FailingWriter);
        let mut session = pid_sim::SimBridgeSession::with_engram_host_profile_and_run_id(
            writer,
            pid_sim::demo_sim(),
            "pairing-poison",
        );
        session.enable_engram_pairing(pid_sim::PairingSecret::from_bytes([9; 32]), 1);
        session
            .begin_pairing_attempt()
            .expect("the first pairing unit should be available");
        session
            .record_rejected_rpc("pairing-poison", "force the provenance failure")
            .expect_err("the failing writer must poison the session");
        let pairing = pid_sim::ConnectionPairing::new();

        let error = finish_pairing_connection(
            Err(anyhow::anyhow!("simulated client failure")),
            &pairing,
            &mut session,
        )
        .expect_err("a poisoned provenance log must remain fatal");

        assert_eq!(error.to_string(), "simulated client failure");
    }

    #[test]
    fn engram_profile_session_usage_includes_the_complete_tcp_prefix() {
        let config = pid_sim::deterministic_sim_config(
            "prefix-test",
            Some("tcp_jsonl"),
            None,
            None,
            Some(true),
        );
        let config_hash =
            pid_runlog::canonical_json_hash_v2(&config).expect("config hash should serialize");
        let sim = pid_sim::demo_sim();
        let prefix = [
            RunLogEvent::RunStarted {
                schema_version: RUN_LOG_SCHEMA_VERSION,
                run_id: "bridge-tcp-run".to_string(),
                timestamp_ns: 0,
                config_hash: config_hash.clone(),
                metadata: Default::default(),
            },
            RunLogEvent::ConfigLogged {
                timestamp_ns: 0,
                config_hash,
                config,
            },
            sim.snapshot_event(),
        ];
        let expected_prefix_bytes = prefix
            .iter()
            .map(|event| {
                u64::try_from(
                    serde_json::to_vec(event)
                        .expect("prefix event should serialize")
                        .len(),
                )
                .expect("prefix event length should fit u64")
                    + 1
            })
            .sum::<u64>();
        let writer = RunLogWriter::new(Vec::new());
        let mut session = pid_sim::SimBridgeSession::with_engram_host_profile_and_run_id(
            writer,
            sim,
            "bridge-tcp-run",
        );

        append_session_prefix(&mut session, &prefix).expect("prefix should fit hosted limits");
        assert_eq!(session.run_log_usage().bytes, expected_prefix_bytes);
        assert_eq!(session.run_log_usage().events, prefix.len());

        let request = BridgeRequest {
            request_id: "session-after-prefix".to_string(),
            step: Some(0),
            timestamp_ns: 0,
            actor: Actor {
                actor_type: ActorType::Script,
                actor_id: "prefix-test".to_string(),
                session_id: Some("prefix-test".to_string()),
            },
            method: BridgeMethod::BridgeSession,
            payload: serde_json::json!({}),
        };
        let request_bytes = u64::try_from(
            serde_json::to_vec(
                &request
                    .to_runlog_event()
                    .expect("request event should serialize"),
            )
            .expect("request event should serialize")
            .len(),
        )
        .expect("request event length should fit u64")
            + 1;

        let response = session
            .dispatch(&request)
            .expect("session request should fit");
        let result = response
            .result
            .expect("session response should contain a result");

        assert_eq!(
            result["resource_usage"]["session_run_log_bytes"],
            expected_prefix_bytes + request_bytes
        );
        assert_eq!(
            result["resource_usage"]["session_run_log_events"],
            prefix.len() + 1
        );
        assert_eq!(
            result["resource_usage"]["observed_at"],
            "before_bridge_session_response"
        );
    }
}
