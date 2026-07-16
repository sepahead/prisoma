use anyhow::{bail, Context, Result};
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:38472";
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Eq, PartialEq)]
struct BridgeArgs {
    safe_mode: bool,
    bind_addr: SocketAddr,
    path: PathBuf,
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
        bind_addr,
        path,
    } = args;

    validate_bind_addr(bind_addr)?;
    let listener =
        TcpListener::bind(bind_addr).with_context(|| format!("failed to bind {bind_addr}"))?;
    let local_addr = listener
        .local_addr()
        .context("failed to read local address")?;
    let (path, artifact_root) = prepare_artifact_path(&path)?;
    let mut writer = create_run_log(&path)?;
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
    metadata.insert("bind_addr".to_string(), local_addr.to_string());
    metadata.insert("requested_bind_addr".to_string(), bind_addr.to_string());
    metadata.insert(
        "artifact_root".to_string(),
        artifact_root.display().to_string(),
    );
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: "bridge-tcp-run".to_string(),
        timestamp_ns: 0,
        config_hash: config_hash.clone(),
        metadata,
    })?;
    writer.append(&RunLogEvent::ConfigLogged {
        timestamp_ns: 0,
        config_hash,
        config,
    })?;

    let sim = pid_sim::demo_sim();
    writer.append(&sim.snapshot_event())?;
    let mut session = pid_sim::SimBridgeSession::with_safe_mode_and_run_id(
        writer,
        sim,
        safe_mode,
        "bridge-tcp-run",
    );
    session.set_run_log_path(&path);
    session.set_artifact_root(&artifact_root)?;
    // Detect buffered provenance-storage failures before advertising the
    // listener or accepting a control client.
    session.flush()?;
    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: "pid-sim-bridge-tcp".to_string(),
        session_id: Some("bridge-tcp".to_string()),
    };

    eprintln!("listening {local_addr}");
    let handled = (|| -> Result<(usize, SocketAddr)> {
        let (stream, peer_addr) = listener.accept().context("failed to accept TCP client")?;
        eprintln!("accepted {peer_addr}");
        configure_client_stream(&stream)?;
        let reader = BufReader::new(stream.try_clone().context("failed to clone TCP stream")?);
        let mut output = BufWriter::new(stream);
        let handled = pid_sim::dispatch_rpc_lines(reader, &mut output, &mut session, actor)?;
        output.flush().context("failed to flush TCP responses")?;
        Ok((handled, peer_addr))
    })();

    // When provenance storage remains writable, always seal accepted-client
    // transport errors as Failed. A provenance-storage error itself may leave
    // a partial/unreadable log and cannot be repaired by this transport.
    let (status, message) = match &handled {
        Ok((count, peer_addr)) => (
            RunStatus::Succeeded,
            format!("processed {count} request(s) from {peer_addr}"),
        ),
        Err(err) => (RunStatus::Failed, format!("TCP transport error: {err:#}")),
    };
    session.finish_run(status, Some(message))?;
    session.flush()?;
    eprintln!("wrote {}", path.display());
    handled.map(|_| ())
}

fn parse_args(args: &[OsString]) -> Result<ParsedCommand> {
    let program = args
        .first()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pid-sim-bridge-tcp".to_string());
    let mut policy = None;
    let mut bind_addr = DEFAULT_BIND_ADDR.to_string();
    let mut path_arg = None;
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
                if policy == Some(true) {
                    bail!(
                        "--allow-mutations conflicts with --safe-mode\n{}",
                        usage(&program)
                    );
                }
                policy = Some(false);
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
    let Some(path_arg) = path_arg else {
        bail!("{}", usage(&program));
    };
    let bind_addr: SocketAddr = bind_addr
        .parse()
        .with_context(|| format!("invalid bind address {bind_addr}"))?;
    Ok(ParsedCommand::Run(BridgeArgs {
        safe_mode: policy.unwrap_or(true),
        bind_addr,
        path: PathBuf::from(path_arg),
    }))
}

fn usage(program: &str) -> String {
    format!(
        "usage: {program} [--safe-mode | --allow-mutations] [--bind LOOPBACK_ADDR] <run-log.jsonl>\n\
         mutations are disabled by default; --allow-mutations explicitly enables them"
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

fn create_run_log(path: &Path) -> Result<RunLogWriter<pid_sim::FsyncFileWriter>> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create new run log {}", path.display()))?;
    Ok(RunLogWriter::new(pid_sim::FsyncFileWriter::new(file)))
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
    }

    #[test]
    fn parse_args_allows_explicit_mutation_opt_in() {
        let args = parsed_run(&["bridge-tcp", "--allow-mutations", "runlog.jsonl"]);

        assert!(!args.safe_mode);
    }

    #[test]
    fn parse_args_retains_explicit_safe_mode_flag() {
        let args = parsed_run(&["bridge-tcp", "--safe-mode", "runlog.jsonl"]);

        assert!(args.safe_mode);
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
}
