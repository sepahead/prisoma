use anyhow::{bail, Context, Result};
use base64::Engine;
use pid_runlog::{Actor, ActorType, RunLogEvent, RunLogWriter, RunStatus, RUN_LOG_SCHEMA_VERSION};
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;

const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

fn main() -> Result<()> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let program = args
        .first()
        .and_then(|value| value.to_str())
        .unwrap_or("pid-sim-bridge-ws");
    let mut safe_mode = false;
    let mut bind_addr = "127.0.0.1:38473".to_string();
    let mut path_arg = None;
    let mut idx = 1;
    while idx < args.len() {
        match args[idx].to_str() {
            Some("--safe-mode") => {
                safe_mode = true;
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
                eprintln!("usage: {program} [--safe-mode] [--bind ADDR] <run-log.jsonl>");
                return Ok(());
            }
            Some(_) if path_arg.is_none() => {
                path_arg = args.get(idx).cloned();
                idx += 1;
            }
            _ => bail!("usage: {program} [--safe-mode] [--bind ADDR] <run-log.jsonl>"),
        }
    }
    let Some(path_arg) = path_arg else {
        bail!("usage: {program} [--safe-mode] [--bind ADDR] <run-log.jsonl>");
    };
    let bind_addr: SocketAddr = bind_addr
        .parse()
        .with_context(|| format!("invalid bind address {bind_addr}"))?;

    let path = PathBuf::from(path_arg);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut writer = RunLogWriter::create(&path)?;
    let config = pid_sim::deterministic_sim_config(
        "pid-sim-bridge-ws",
        Some("websocket_jsonrpc"),
        None,
        None,
        Some(safe_mode),
    );
    let config_hash = pid_runlog::canonical_json_hash(&config)?;
    let mut metadata = BTreeMap::new();
    metadata.insert("source".to_string(), "pid-sim-bridge-ws".to_string());
    metadata.insert("safe_mode".to_string(), safe_mode.to_string());
    metadata.insert("bind_addr".to_string(), bind_addr.to_string());
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
    let mut session = pid_sim::SimBridgeSession::with_safe_mode(writer, sim, safe_mode);
    let actor = Actor {
        actor_type: ActorType::Script,
        actor_id: "pid-sim-bridge-ws".to_string(),
        session_id: Some("bridge-ws".to_string()),
    };

    let listener =
        TcpListener::bind(bind_addr).with_context(|| format!("failed to bind {bind_addr}"))?;
    let local_addr = listener
        .local_addr()
        .context("failed to read local address")?;
    eprintln!("listening {local_addr}");
    let (mut stream, peer_addr) = listener.accept().context("failed to accept TCP client")?;
    eprintln!("accepted {peer_addr}");
    perform_websocket_handshake(&mut stream)?;
    let handled = dispatch_websocket_messages(&mut stream, &mut session, actor)?;

    session.record_event(&RunLogEvent::RunEnded {
        run_id: "bridge-ws-run".to_string(),
        timestamp_ns: session.timestamp_ns(),
        status: RunStatus::Succeeded,
        message: Some(format!("processed {handled} request(s) from {peer_addr}")),
    })?;
    session.flush()?;
    eprintln!("wrote {}", path.display());
    Ok(())
}

fn perform_websocket_handshake(stream: &mut TcpStream) -> Result<()> {
    let request = read_http_upgrade_request(stream)?;
    let key = request
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("sec-websocket-key") {
                Some(value.trim())
            } else {
                None
            }
        })
        .context("missing Sec-WebSocket-Key header")?;
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

fn read_http_upgrade_request(stream: &mut TcpStream) -> Result<String> {
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
    while let Some(frame) = read_websocket_frame(stream)? {
        match frame {
            WebSocketFrame::Text(text) => {
                for line in text.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    handled += 1;
                    let response =
                        pid_sim::dispatch_rpc_text_request(line, handled, session, actor.clone());
                    let response = serde_json::to_string(&response)
                        .context("failed to encode JSON-RPC response")?;
                    write_websocket_frame(stream, 0x1, response.as_bytes())?;
                }
            }
            WebSocketFrame::Ping(payload) => write_websocket_frame(stream, 0xA, &payload)?,
            WebSocketFrame::Pong => {}
            WebSocketFrame::Close => {
                write_websocket_frame(stream, 0x8, &[])?;
                break;
            }
        }
    }
    Ok(handled)
}

enum WebSocketFrame {
    Text(String),
    Ping(Vec<u8>),
    Pong,
    Close,
}

fn read_websocket_frame(stream: &mut TcpStream) -> Result<Option<WebSocketFrame>> {
    let mut header = [0u8; 2];
    match stream.read_exact(&mut header) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(err).context("failed to read WebSocket frame header"),
    }
    let fin = header[0] & 0x80 != 0;
    let opcode = header[0] & 0x0f;
    let masked = header[1] & 0x80 != 0;
    let mut payload_len = (header[1] & 0x7f) as u64;
    if !fin {
        bail!("fragmented WebSocket messages are not supported");
    }
    if payload_len == 126 {
        let mut extended = [0u8; 2];
        stream
            .read_exact(&mut extended)
            .context("failed to read WebSocket extended length")?;
        payload_len = u16::from_be_bytes(extended) as u64;
    } else if payload_len == 127 {
        let mut extended = [0u8; 8];
        stream
            .read_exact(&mut extended)
            .context("failed to read WebSocket extended length")?;
        payload_len = u64::from_be_bytes(extended);
    }
    if payload_len > 1_048_576 {
        bail!("WebSocket frame exceeds 1 MiB limit");
    }
    let mut mask = [0u8; 4];
    if masked {
        stream
            .read_exact(&mut mask)
            .context("failed to read WebSocket mask")?;
    } else if matches!(opcode, 0x1 | 0x2 | 0x8 | 0x9 | 0xA) {
        bail!("client WebSocket frames must be masked");
    }
    let mut payload = vec![0u8; payload_len as usize];
    stream
        .read_exact(&mut payload)
        .context("failed to read WebSocket payload")?;
    if masked {
        for (idx, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[idx % 4];
        }
    }
    match opcode {
        0x1 => Ok(Some(WebSocketFrame::Text(
            String::from_utf8(payload).context("WebSocket text frame is not UTF-8")?,
        ))),
        0x8 => Ok(Some(WebSocketFrame::Close)),
        0x9 => Ok(Some(WebSocketFrame::Ping(payload))),
        0xA => Ok(Some(WebSocketFrame::Pong)),
        other => bail!("unsupported WebSocket opcode {other}"),
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

    #[test]
    fn websocket_accept_key_matches_rfc_example() {
        let mut hasher = Sha1::new();
        hasher.update(b"dGhlIHNhbXBsZSBub25jZQ==");
        hasher.update(WEBSOCKET_GUID.as_bytes());
        let accept = base64::engine::general_purpose::STANDARD.encode(hasher.finalize());
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
