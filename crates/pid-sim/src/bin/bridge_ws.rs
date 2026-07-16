use anyhow::{bail, Context, Result};
use base64::Engine;
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:38473";
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_FRAME_BYTES: u64 = 1024 * 1024;

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
        "pid-sim-bridge-ws",
        Some("websocket_jsonrpc"),
        None,
        None,
        Some(safe_mode),
    );
    let config_hash = pid_runlog::canonical_json_hash_v2(&config)?;
    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "pid-sim-bridge-ws".to_string());
    metadata.insert("safe_mode".to_string(), safe_mode.to_string());
    metadata.insert("bind_addr".to_string(), local_addr.to_string());
    metadata.insert("requested_bind_addr".to_string(), bind_addr.to_string());
    metadata.insert(
        "artifact_root".to_string(),
        artifact_root.display().to_string(),
    );
    writer.append(&RunLogEvent::RunStarted {
        schema_version: RUN_LOG_SCHEMA_VERSION,
        run_id: "bridge-ws-run".to_string(),
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
        "bridge-ws-run",
    );
    session.set_run_log_path(&path);
    session.set_artifact_root(&artifact_root)?;
    // Detect buffered provenance-storage failures before advertising the
    // listener or accepting a control client.
    session.flush()?;
    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: "pid-sim-bridge-ws".to_string(),
        session_id: Some("bridge-ws".to_string()),
    };

    eprintln!("listening {local_addr}");
    let handled = (|| -> Result<(usize, SocketAddr)> {
        let (mut stream, peer_addr) = listener.accept().context("failed to accept TCP client")?;
        eprintln!("accepted {peer_addr}");
        configure_client_stream(&stream)?;
        perform_websocket_handshake(&mut stream)?;
        let count = dispatch_websocket_messages(&mut stream, &mut session, actor)?;
        Ok((count, peer_addr))
    })();

    // When provenance storage remains writable, seal mid-session protocol and
    // transport errors as Failed. A provenance-storage error itself may leave
    // a partial/unreadable log and cannot be repaired by this transport.
    let (status, message) = match &handled {
        Ok((count, peer_addr)) => (
            RunStatus::Succeeded,
            format!("processed {count} request(s) from {peer_addr}"),
        ),
        Err(err) => (
            RunStatus::Failed,
            format!("WebSocket transport error: {err:#}"),
        ),
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
        .unwrap_or_else(|| "pid-sim-bridge-ws".to_string());
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

fn configure_client_stream(stream: &TcpStream) -> Result<()> {
    stream
        .set_read_timeout(Some(CLIENT_IO_TIMEOUT))
        .context("failed to set WebSocket client read timeout")?;
    stream
        .set_write_timeout(Some(CLIENT_IO_TIMEOUT))
        .context("failed to set WebSocket client write timeout")?;
    Ok(())
}

fn perform_websocket_handshake(stream: &mut TcpStream) -> Result<()> {
    let request = read_http_upgrade_request(stream)?;
    let key = validate_websocket_upgrade_request(&request)?;
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());
    let accept = base64::engine::general_purpose::STANDARD.encode(hasher.finalize());
    write!(
        stream,
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\
         \r\n"
    )
    .context("failed to write WebSocket handshake")?;
    stream
        .flush()
        .context("failed to flush WebSocket handshake")?;
    Ok(())
}

fn validate_websocket_upgrade_request(request: &str) -> Result<&str> {
    let request = request
        .strip_suffix("\r\n\r\n")
        .context("WebSocket upgrade request must end with CRLF CRLF")?;
    let mut lines = request.split("\r\n");
    let request_line = lines
        .next()
        .context("missing WebSocket HTTP request line")?;
    validate_request_line(request_line)?;

    let mut host = None;
    let mut upgrade = None;
    let mut connection = None;
    let mut version = None;
    let mut key = None;
    for line in lines {
        if line.is_empty() {
            bail!("WebSocket upgrade request contains an empty header line");
        }
        let (name, raw_value) = line
            .split_once(':')
            .context("malformed WebSocket HTTP header")?;
        if name.trim() != name || !is_http_token(name) {
            bail!("malformed WebSocket HTTP header name");
        }
        if raw_value
            .bytes()
            .any(|byte| (byte < b' ' && byte != b'\t') || byte == 0x7f)
        {
            bail!("malformed WebSocket HTTP header value");
        }
        let value = raw_value.trim_matches([' ', '\t']);
        if name.eq_ignore_ascii_case("origin") {
            bail!("Origin-bearing WebSocket requests are not accepted");
        } else if name.eq_ignore_ascii_case("host") {
            set_unique_header(&mut host, value, "Host")?;
        } else if name.eq_ignore_ascii_case("upgrade") {
            set_unique_header(&mut upgrade, value, "Upgrade")?;
        } else if name.eq_ignore_ascii_case("connection") {
            set_unique_header(&mut connection, value, "Connection")?;
        } else if name.eq_ignore_ascii_case("sec-websocket-version") {
            set_unique_header(&mut version, value, "Sec-WebSocket-Version")?;
        } else if name.eq_ignore_ascii_case("sec-websocket-key") {
            set_unique_header(&mut key, value, "Sec-WebSocket-Key")?;
        }
    }

    let host = host.context("missing Host header")?;
    if host.is_empty() {
        bail!("Host header must be nonempty");
    }
    let upgrade = upgrade.context("missing Upgrade header")?;
    if !upgrade.eq_ignore_ascii_case("websocket") {
        bail!("Upgrade header must be websocket");
    }
    let connection = connection.context("missing Connection header")?;
    validate_connection_header(connection)?;
    let version = version.context("missing Sec-WebSocket-Version header")?;
    if version != "13" {
        bail!("Sec-WebSocket-Version must be 13");
    }
    let key = key.context("missing Sec-WebSocket-Key header")?;
    let decoded_key = base64::engine::general_purpose::STANDARD
        .decode(key.as_bytes())
        .context("Sec-WebSocket-Key is not valid base64")?;
    if decoded_key.len() != 16 {
        bail!("Sec-WebSocket-Key must decode to exactly 16 bytes");
    }
    Ok(key)
}

fn validate_request_line(request_line: &str) -> Result<()> {
    let mut parts = request_line.split(' ');
    let method = parts.next().context("missing HTTP method")?;
    let target = parts.next().context("missing HTTP request target")?;
    let version = parts.next().context("missing HTTP version")?;
    if parts.next().is_some() || target.is_empty() {
        bail!("malformed WebSocket HTTP request line");
    }
    if method != "GET" || version != "HTTP/1.1" {
        bail!("WebSocket upgrade requires GET HTTP/1.1");
    }
    if target != "/bridge" {
        bail!("WebSocket request target must be /bridge");
    }
    Ok(())
}

fn is_http_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

fn set_unique_header<'a>(slot: &mut Option<&'a str>, value: &'a str, name: &str) -> Result<()> {
    if slot.replace(value).is_some() {
        bail!("{name} header must appear exactly once");
    }
    Ok(())
}

fn validate_connection_header(value: &str) -> Result<()> {
    let mut contains_upgrade = false;
    for token in value.split(',') {
        let token = token.trim_matches([' ', '\t']);
        if token.is_empty() {
            bail!("Connection header contains an empty token");
        }
        if !is_http_token(token) {
            bail!("Connection header contains a malformed token");
        }
        contains_upgrade |= token.eq_ignore_ascii_case("upgrade");
    }
    if !contains_upgrade {
        bail!("Connection header must contain upgrade");
    }
    Ok(())
}

fn read_http_upgrade_request<R: Read>(stream: &mut R) -> Result<String> {
    let mut bytes = Vec::new();
    let mut byte = [0u8; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() >= 16 * 1024 {
            bail!("WebSocket upgrade request is too large");
        }
        stream
            .read_exact(&mut byte)
            .context("failed to read WebSocket upgrade request")?;
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes).context("WebSocket upgrade request is not UTF-8")
}

fn dispatch_websocket_messages<W: Write>(
    stream: &mut TcpStream,
    session: &mut pid_sim::SimBridgeSession<W>,
    actor: Actor,
) -> Result<usize> {
    let mut handled = 0usize;
    loop {
        let frame = read_websocket_frame(stream)?
            .context("WebSocket connection closed without a close frame")?;
        match frame {
            WebSocketFrame::Text(text) => {
                handled += 1;
                let response =
                    dispatch_websocket_text_message(&text, handled, session, actor.clone())?;
                if let Some(response) = response {
                    let response = serde_json::to_string(&response)
                        .context("failed to encode JSON-RPC response")?;
                    write_websocket_frame(stream, 0x1, response.as_bytes())?;
                }
                if session.stop_requested() || session.run_ended() {
                    return Ok(handled);
                }
            }
            WebSocketFrame::Ping(payload) => write_websocket_frame(stream, 0xA, &payload)?,
            WebSocketFrame::Pong => {}
            WebSocketFrame::Close => {
                write_websocket_frame(stream, 0x8, &[])?;
                return Ok(handled);
            }
        }
    }
}

fn dispatch_websocket_text_message<W: Write>(
    text: &str,
    request_index: usize,
    session: &mut pid_sim::SimBridgeSession<W>,
    actor: Actor,
) -> Result<Option<pid_bridge::BridgeRpcResponse>> {
    // One WebSocket text message carries exactly one JSON-RPC Request object.
    // Trimming permits pretty-printed JSON; embedded newlines are not a second
    // multiplexing layer, and concatenated objects fail as one parse error.
    pid_sim::dispatch_rpc_text_request(text.trim(), request_index, session, actor)
}

#[derive(Debug)]
enum WebSocketFrame {
    Text(String),
    Ping(Vec<u8>),
    Pong,
    Close,
}

fn read_websocket_frame<R: Read>(stream: &mut R) -> Result<Option<WebSocketFrame>> {
    let mut header = [0u8; 2];
    loop {
        match stream.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(1) => break,
            Ok(_) => unreachable!("one-byte read buffer returned more than one byte"),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error).context("failed to read WebSocket frame header"),
        }
    }
    stream
        .read_exact(&mut header[1..])
        .context("truncated WebSocket frame header")?;
    let fin = header[0] & 0x80 != 0;
    let reserved = header[0] & 0x70;
    let opcode = header[0] & 0x0f;
    let masked = header[1] & 0x80 != 0;
    let length_code = header[1] & 0x7f;
    let mut payload_len = u64::from(length_code);
    if !fin {
        bail!("fragmented WebSocket messages are not supported");
    }
    if reserved != 0 {
        bail!("WebSocket reserved bits require an unsupported extension");
    }
    if !matches!(opcode, 0x1 | 0x8 | 0x9 | 0xA) {
        bail!("unsupported WebSocket opcode {opcode}");
    }
    if !masked {
        bail!("client WebSocket frames must be masked");
    }
    let control_frame = opcode & 0x08 != 0;
    if control_frame && length_code > 125 {
        bail!("WebSocket control frames must not use extended payload lengths");
    }
    if length_code == 126 {
        let mut extended = [0u8; 2];
        stream
            .read_exact(&mut extended)
            .context("failed to read WebSocket extended length")?;
        payload_len = u16::from_be_bytes(extended) as u64;
        if payload_len < 126 {
            bail!("WebSocket frame uses a non-minimal payload length encoding");
        }
    } else if length_code == 127 {
        let mut extended = [0u8; 8];
        stream
            .read_exact(&mut extended)
            .context("failed to read WebSocket extended length")?;
        if extended[0] & 0x80 != 0 {
            bail!("WebSocket 64-bit payload length must have its high bit clear");
        }
        payload_len = u64::from_be_bytes(extended);
        if payload_len <= u64::from(u16::MAX) {
            bail!("WebSocket frame uses a non-minimal payload length encoding");
        }
    }
    if payload_len > MAX_FRAME_BYTES {
        bail!("WebSocket frame exceeds 1 MiB limit");
    }
    let mut mask = [0u8; 4];
    stream
        .read_exact(&mut mask)
        .context("failed to read WebSocket mask")?;
    let mut payload = vec![0u8; payload_len as usize];
    stream
        .read_exact(&mut payload)
        .context("failed to read WebSocket payload")?;
    for (idx, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask[idx % 4];
    }
    match opcode {
        0x1 => Ok(Some(WebSocketFrame::Text(
            String::from_utf8(payload).context("WebSocket text frame is not UTF-8")?,
        ))),
        0x8 => {
            if payload.len() == 1 {
                bail!("WebSocket close payload must be empty or include a two-byte status code");
            }
            if payload.len() > 2 {
                std::str::from_utf8(&payload[2..])
                    .context("WebSocket close reason is not UTF-8")?;
            }
            if payload.len() >= 2 {
                let code = u16::from_be_bytes([payload[0], payload[1]]);
                if !matches!(code, 1000..=1003 | 1007..=1014 | 3000..=4999) {
                    bail!("WebSocket close status code {code} is not valid on the wire");
                }
            }
            Ok(Some(WebSocketFrame::Close))
        }
        0x9 => Ok(Some(WebSocketFrame::Ping(payload))),
        0xA => Ok(Some(WebSocketFrame::Pong)),
        _ => unreachable!("opcode was validated before reading the payload"),
    }
}

fn write_websocket_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<()> {
    let mut header = vec![0x80 | (opcode & 0x0f)];
    if payload.len() < 126 {
        header.push(payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        header.push(126);
        header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        header.push(127);
        header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    stream
        .write_all(&header)
        .context("failed to write WebSocket frame header")?;
    stream
        .write_all(payload)
        .context("failed to write WebSocket frame payload")?;
    stream.flush().context("failed to flush WebSocket frame")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_PATH: AtomicU64 = AtomicU64::new(0);

    const VALID_REQUEST: &str = "GET /bridge HTTP/1.1\r\n\
                                Host: localhost\r\n\
                                Upgrade: websocket\r\n\
                                Connection: keep-alive, Upgrade\r\n\
                                Sec-WebSocket-Version: 13\r\n\
                                Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                                \r\n";

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
            "prisoma-bridge-ws-{}-{}.jsonl",
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

    fn frame_error(bytes: &[u8]) -> String {
        read_websocket_frame(&mut Cursor::new(bytes))
            .expect_err("frame should fail closed")
            .to_string()
    }

    #[test]
    fn parse_args_defaults_to_safe_mode() {
        let args = parsed_run(&["bridge-ws", "runlog.jsonl"]);

        assert!(args.safe_mode);
    }

    #[test]
    fn parse_args_allows_explicit_mutation_opt_in() {
        let args = parsed_run(&["bridge-ws", "--allow-mutations", "runlog.jsonl"]);

        assert!(!args.safe_mode);
    }

    #[test]
    fn parse_args_retains_explicit_safe_mode_flag() {
        let args = parsed_run(&["bridge-ws", "--safe-mode", "runlog.jsonl"]);

        assert!(args.safe_mode);
    }

    #[test]
    fn validate_bind_addr_rejects_non_loopback_ipv4() {
        let addr = "0.0.0.0:38473".parse().expect("address should parse");

        assert!(validate_bind_addr(addr).is_err());
    }

    #[test]
    fn validate_bind_addr_rejects_non_loopback_ipv6() {
        let addr = "[::]:38473".parse().expect("address should parse");

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

    #[test]
    fn websocket_accept_key_matches_rfc_example() {
        let mut hasher = Sha1::new();
        hasher.update(b"dGhlIHNhbXBsZSBub25jZQ==");
        hasher.update(WEBSOCKET_GUID.as_bytes());
        let accept = base64::engine::general_purpose::STANDARD.encode(hasher.finalize());
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn validate_websocket_upgrade_request_accepts_complete_handshake() {
        let key = validate_websocket_upgrade_request(VALID_REQUEST)
            .expect("complete handshake should validate");

        assert_eq!(key, "dGhlIHNhbXBsZSBub25jZQ==");
    }

    #[test]
    fn websocket_text_message_accepts_one_pretty_printed_request() {
        let writer = RunLogWriter::new(Vec::new());
        let mut session = pid_sim::SimBridgeSession::new(writer, pid_sim::demo_sim());
        let request = r#"{
            "jsonrpc": "2.0",
            "id": "pretty",
            "method": "sim.status",
            "params": {}
        }"#;

        let response = dispatch_websocket_text_message(
            request,
            1,
            &mut session,
            Actor {
                actor_type: ActorType::Script,
                actor_id: "ws-pretty-test".to_string(),
                session_id: None,
            },
        )
        .unwrap()
        .unwrap();

        assert!(response.is_ok());
        assert_eq!(response.id, serde_json::json!("pretty"));
        let events = pid_runlog::read_events(Cursor::new(session.into_inner())).unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, RunLogEvent::BridgeRequest { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn websocket_text_message_does_not_multiplex_newline_delimited_requests() {
        let writer = RunLogWriter::new(Vec::new());
        let mut session = pid_sim::SimBridgeSession::new(writer, pid_sim::demo_sim());
        let requests = concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"sim.status","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"sim.status","params":{}}"#
        );

        let response = dispatch_websocket_text_message(
            requests,
            1,
            &mut session,
            Actor {
                actor_type: ActorType::Script,
                actor_id: "ws-single-request-test".to_string(),
                session_id: None,
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(response.error.as_ref().unwrap().code, -32700);
        let events = pid_runlog::read_events(Cursor::new(session.into_inner())).unwrap();
        assert!(!events
            .iter()
            .any(|event| matches!(event, RunLogEvent::BridgeRequest { .. })));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, RunLogEvent::ErrorLogged { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_origin_header() {
        let request = VALID_REQUEST.replace(
            "Host: localhost\r\n",
            "Host: localhost\r\nOrigin: https://example.test\r\n",
        );

        let error = validate_websocket_upgrade_request(&request)
            .expect_err("Origin-bearing handshake must fail");

        assert!(error.to_string().contains("Origin-bearing"));
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_non_get_method() {
        let request = VALID_REQUEST.replacen("GET", "POST", 1);

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_non_http_1_1() {
        let request = VALID_REQUEST.replacen("HTTP/1.1", "HTTP/1.0", 1);

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_other_request_target() {
        let request = VALID_REQUEST.replacen("/bridge", "/other", 1);

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_missing_host() {
        let request = VALID_REQUEST.replace("Host: localhost\r\n", "");

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_empty_host() {
        let request = VALID_REQUEST.replace("Host: localhost", "Host: \t");

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_duplicate_host() {
        let request = VALID_REQUEST.replace(
            "Host: localhost\r\n",
            "Host: localhost\r\nHost: localhost\r\n",
        );

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_wrong_upgrade() {
        let request = VALID_REQUEST.replace("Upgrade: websocket", "Upgrade: h2c");

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_connection_without_upgrade() {
        let request =
            VALID_REQUEST.replace("Connection: keep-alive, Upgrade", "Connection: keep-alive");

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_malformed_connection_token() {
        let request = VALID_REQUEST.replace(
            "Connection: keep-alive, Upgrade",
            "Connection: keep-alive, bad token, Upgrade",
        );

        let error = validate_websocket_upgrade_request(&request).unwrap_err();
        assert!(error.to_string().contains("malformed token"));
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_wrong_version() {
        let request =
            VALID_REQUEST.replace("Sec-WebSocket-Version: 13", "Sec-WebSocket-Version: 12");

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_duplicate_key() {
        let request = VALID_REQUEST.replace(
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n",
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n",
        );

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_missing_key() {
        let request = VALID_REQUEST.replace("Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n", "");

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_invalid_base64_key() {
        let request = VALID_REQUEST.replace("dGhlIHNhbXBsZSBub25jZQ==", "not-a-valid-base64-key");

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn validate_websocket_upgrade_request_rejects_non_16_byte_key() {
        let request = VALID_REQUEST.replace("dGhlIHNhbXBsZSBub25jZQ==", "c2hvcnQ=");

        assert!(validate_websocket_upgrade_request(&request).is_err());
    }

    #[test]
    fn read_websocket_frame_rejects_reserved_bits() {
        assert!(frame_error(&[0xC1, 0x80]).contains("reserved bits"));
    }

    #[test]
    fn read_websocket_frame_rejects_unmasked_client_frame() {
        assert!(frame_error(&[0x81, 0x00]).contains("must be masked"));
    }

    #[test]
    fn read_websocket_frame_rejects_unsupported_opcode() {
        assert!(frame_error(&[0x82, 0x80]).contains("unsupported WebSocket opcode"));
    }

    #[test]
    fn read_websocket_frame_rejects_extended_control_payload() {
        assert!(frame_error(&[0x89, 0xFE]).contains("control frames"));
    }

    #[test]
    fn read_websocket_frame_rejects_nonminimal_length() {
        let frame = [0x81, 0xFE, 0x00, 0x01];
        assert!(frame_error(&frame).contains("non-minimal"));
    }

    #[test]
    fn read_websocket_frame_rejects_high_bit_in_64_bit_length() {
        let frame = [0x81, 0xFF, 0x80, 0, 0, 0, 0, 0, 0, 0];
        assert!(frame_error(&frame).contains("high bit clear"));
    }

    #[test]
    fn read_websocket_frame_rejects_one_byte_close_payload() {
        let frame = [0x88, 0x81, 0, 0, 0, 0, 0];
        assert!(frame_error(&frame).contains("two-byte status code"));
    }

    #[test]
    fn read_websocket_frame_rejects_partial_header() {
        assert!(frame_error(&[0x81]).contains("truncated WebSocket frame header"));
    }

    #[test]
    fn read_websocket_frame_rejects_reserved_close_code() {
        let frame = [0x88, 0x82, 0, 0, 0, 0, 0x03, 0xED];
        assert!(frame_error(&frame).contains("1005 is not valid"));
    }

    #[test]
    fn read_http_upgrade_request_rejects_oversized_header() {
        let bytes = vec![b'a'; 16 * 1024];
        let error = read_http_upgrade_request(&mut Cursor::new(bytes)).unwrap_err();
        assert!(error.to_string().contains("too large"));
    }

    #[test]
    fn read_websocket_frame_rejects_oversized_payload_before_allocation() {
        let length = MAX_FRAME_BYTES + 1;
        let mut frame = vec![0x81, 0xFF];
        frame.extend_from_slice(&length.to_be_bytes());
        let error = frame_error(&frame);
        assert!(error.contains("exceeds 1 MiB"), "{error}");
    }
}
