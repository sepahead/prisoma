//! `ncp-observe` — run the passive NCP → (V,L,D,A) observer.
//!
//! Subscribes read-only to a session's NCP data planes over Zenoh and, on
//! SIGINT/SIGTERM/SIGHUP, writes an `OfflineVldaDataset` artifact (run it
//! through `pid-offline-harness`) plus a provenance run log. It drives nothing —
//! the Agent Bridge stays the only control plane.
//!
//! ```bash
//! # ncp-observer is excluded from the default workspace; run it by manifest path:
//! cargo run --manifest-path crates/ncp-observer/Cargo.toml --bin ncp-observe -- \
//!     --secure --session uav3 --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
//! # then:
//! cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json ...
//! ```
//!
//! Zenoh connectivity is explicit: `--open` uses NCP's unauthenticated,
//! scouting-off client default; `--secure` calls `ZenohBus::open_secure` and
//! requires `NCP_ZENOH_CONFIG` to name a strict TLS-only client configuration.
//! The observer never treats a realm name as authentication.

use ncp_core::keys::Keys;
use ncp_core::{decode_validated, CommandFrame, ObservationFrame, SensorFrame, NCP_VERSION};
use ncp_observer::{Mapping, Observer};
use ncp_zenoh::ZenohBus;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Accept an observation frame off the plane under the NCP wire-0.8 contract.
///
/// The observer is a passive, read-only tap, so it must never panic and never
/// drive anything — a frame that fails the wire contract is DROPPED and COUNTED,
/// exactly like a serde decode failure. `decode_validated` enforces the right
/// `kind`, a compatible `ncp_version` (an absent/incompatible version is rejected,
/// never coerced — the wire-0.8 gate), the wire-0.8 stream identity (a canonical
/// `stream.epoch`, `stream.seq >= 1`, well-formed `session_id`, and a canonical
/// `session.generation`), and — when present — a valid `source`.
///
/// The plane form carries a `source` echoing the driving `SensorFrame.stream`
/// (the cross-plane join key); the pull/RPC-reply form is distinguished by
/// `source` ABSENCE (the wire-0.8 successor to the retired `seq == 0` sentinel).
///
/// Returns the accepted frame, or an `AcceptError` telling the caller which
/// counter to bump: `Invalid` (version-less / incompatible / wrong kind /
/// unparseable) vs `Unstamped` (a valid pull/RPC-form frame with no `source`).
/// Both are dropped; a plane observer never promotes source-less D by recency.
enum AcceptError {
    Invalid,
    Unstamped,
}

fn accept_observation(bytes: &[u8]) -> Result<ObservationFrame, AcceptError> {
    match decode_validated::<ObservationFrame>(bytes) {
        Ok(f) if f.source.is_some() => Ok(f),
        // Valid frame, wrong medium: no `source` is the pull/RPC-reply form and has
        // no exact driving tick, so it must not enter the observation-plane join.
        Ok(_) => Err(AcceptError::Unstamped),
        Err(_) => Err(AcceptError::Invalid),
    }
}

const HANDOFF_CAPACITY: usize = 1024;

enum Ingress {
    Sensor(Vec<u8>),
    Command(Vec<u8>),
    Observation(Vec<u8>),
}

#[derive(Debug, Default)]
struct CaptureCounters {
    sensor_decode_failures: u64,
    command_decode_failures: u64,
    observation_decode_failures: u64,
    observation_unstamped: u64,
}

struct CaptureResult {
    observer: Observer,
    counters: CaptureCounters,
    error: Option<anyhow::Error>,
}

async fn capture_worker(
    mut observer: Observer,
    mut ingress: mpsc::Receiver<Ingress>,
    expected_session: String,
) -> CaptureResult {
    let mut counters = CaptureCounters::default();
    let mut error = None;
    while let Some(frame) = ingress.recv().await {
        let result = match frame {
            Ingress::Sensor(bytes) => match decode_validated::<SensorFrame>(&bytes) {
                Ok(frame) => observer.on_sensor(&frame),
                Err(_) => {
                    counters.sensor_decode_failures =
                        counters.sensor_decode_failures.saturating_add(1);
                    continue;
                }
            },
            Ingress::Command(bytes) => match decode_validated::<CommandFrame>(&bytes) {
                Ok(frame) => observer.on_command(&frame),
                Err(_) => {
                    counters.command_decode_failures =
                        counters.command_decode_failures.saturating_add(1);
                    continue;
                }
            },
            Ingress::Observation(bytes) => match accept_observation(&bytes) {
                Ok(frame) if frame.session_id == expected_session => {
                    observer.on_observation(&frame)
                }
                Ok(_) => {
                    counters.observation_decode_failures =
                        counters.observation_decode_failures.saturating_add(1);
                    continue;
                }
                Err(AcceptError::Unstamped) => {
                    counters.observation_unstamped =
                        counters.observation_unstamped.saturating_add(1);
                    continue;
                }
                Err(AcceptError::Invalid) => {
                    counters.observation_decode_failures =
                        counters.observation_decode_failures.saturating_add(1);
                    continue;
                }
            },
        };
        if let Err(capture_error) = result {
            // Latch the first state-machine failure and stop consuming rather
            // than building evidence after a corrupted state transition.
            error = Some(capture_error);
            break;
        }
    }
    CaptureResult {
        observer,
        counters,
        error,
    }
}

fn enqueue(sender: &mpsc::Sender<Ingress>, frame: Ingress, handoff_drops: &AtomicU64) {
    if sender.try_send(frame).is_err() {
        let _ = handoff_drops.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
            Some(count.saturating_add(1))
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionMode {
    Open,
    Secure,
}

#[derive(Debug)]
struct Args {
    session: String,
    realm: String,
    out: String,
    runlog: Option<String>,
    model: String,
    task: String,
    language_channel: String,
    episode: Option<String>,
    mode: Option<ConnectionMode>,
}

fn parse_args() -> anyhow::Result<Args> {
    parse_args_from(std::env::args().collect())
}

fn parse_args_from(argv: Vec<String>) -> anyhow::Result<Args> {
    let mut a = Args {
        session: "default".into(),
        // This observer taps an Engram deployment, whose rendezvous realm is
        // "engram/ncp" — a DEPLOYMENT choice, named here explicitly rather than
        // inherited from ncp_core::DEFAULT_REALM (which is the neutral "ncp": NCP the
        // protocol names no consumer). Override with --realm for another deployment.
        realm: "engram/ncp".into(),
        out: "ncp_vlda.json".into(),
        runlog: None,
        model: "nest".into(),
        task: "ncp".into(),
        language_channel: "instruction".into(),
        episode: None,
        mode: None,
    };
    let mut i = 1;
    while i < argv.len() {
        let flag = argv[i].clone();
        let requested_mode = match flag.as_str() {
            "--open" => Some(ConnectionMode::Open),
            "--secure" => Some(ConnectionMode::Secure),
            _ => None,
        };
        if let Some(mode) = requested_mode {
            if a.mode.is_some_and(|existing| existing != mode) {
                anyhow::bail!("--open and --secure are mutually exclusive");
            }
            a.mode = Some(mode);
            i += 1;
            continue;
        }
        // Mode flags are valueless and mutually exclusive. Every other
        // recognized flag takes exactly one value argument; unknown args fail
        // closed so a misspelled security or output option is never ignored.
        let known = matches!(
            flag.as_str(),
            "--session"
                | "--realm"
                | "--out"
                | "--runlog"
                | "--model"
                | "--task"
                | "--language-channel"
                | "--episode"
        );
        if !known {
            anyhow::bail!("unknown argument {flag:?}");
        }
        let value = match argv.get(i + 1) {
            Some(v) => v.clone(),
            None => anyhow::bail!("flag {flag:?} expects a value"),
        };
        match flag.as_str() {
            "--session" => a.session = value,
            "--realm" => a.realm = value,
            "--out" => a.out = value,
            "--runlog" => a.runlog = Some(value),
            "--model" => a.model = value,
            "--task" => a.task = value,
            "--language-channel" => a.language_channel = value,
            "--episode" => a.episode = Some(value),
            _ => unreachable!("known flag handled above"),
        }
        i += 2;
    }
    if a.mode.is_none() {
        anyhow::bail!(
            "explicit connection mode required: pass --secure (recommended) or acknowledge unauthenticated transport with --open"
        );
    }
    if a.runlog.is_none() {
        anyhow::bail!("--runlog is required because the canonical run log is the source of truth");
    }
    Ok(a)
}

/// Wait for any terminating signal. SIGINT alone is not enough: SIGTERM is the
/// default for `docker stop` / systemd / plain `kill`, and losing an hours-long
/// capture because the supervisor sent the polite signal is not acceptable.
async fn shutdown_signal() -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate())?;
        let mut hup = signal(SignalKind::hangup())?;
        tokio::select! {
            r = tokio::signal::ctrl_c() => r?,
            _ = term.recv() => {}
            _ = hup.recv() => {}
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = parse_args()?;
    let mode = args
        .mode
        .ok_or_else(|| anyhow::anyhow!("connection mode was not validated"))?;
    let mapping = Mapping {
        language_channel: args.language_channel.clone(),
        success_channel: Some("success".into()),
        episode_id: args.episode.clone(),
    };
    let mut observer = Observer::new(
        format!("ncp-{}", args.session),
        args.model.clone(),
        args.task.clone(),
        mapping,
    )
    // Wire 0.8 carries `session_id` + `session.generation` on the data plane; the
    // observer validates every frame's payload identity against the captured
    // session and the first-seen live incarnation.
    .with_expected_session(args.session.clone());
    let runlog_path = args
        .runlog
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("run-log requirement was not validated"))?;
    observer = observer.with_runlog(runlog_path)?;
    let keys = Keys::try_new(args.realm.clone())
        .map_err(|error| anyhow::anyhow!("invalid NCP realm {:?}: {error}", args.realm))?;
    let bus = match mode {
        ConnectionMode::Open => ZenohBus::open_realm(keys).await?,
        ConnectionMode::Secure => ZenohBus::open_secure(keys).await?,
    };

    // Zenoh callbacks only enqueue owned bytes into this bounded handoff. One
    // Tokio worker owns Observer and performs decode/join/log bookkeeping in
    // arrival order, so callback threads never block on a mutex or do disk/JSON
    // work. Saturation drops are explicit per-plane counters.
    let (ingress_tx, ingress_rx) = mpsc::channel(HANDOFF_CAPACITY);
    let worker = tokio::spawn(capture_worker(observer, ingress_rx, args.session.clone()));
    let sensor_handoff_drops = Arc::new(AtomicU64::new(0));
    let command_handoff_drops = Arc::new(AtomicU64::new(0));
    let observation_handoff_drops = Arc::new(AtomicU64::new(0));

    let tx = ingress_tx.clone();
    let drops = sensor_handoff_drops.clone();
    bus.subscribe_sensors(&args.session, move |_k, bytes| {
        enqueue(&tx, Ingress::Sensor(bytes), &drops);
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe sensors: {e}"))?;

    let tx = ingress_tx.clone();
    let drops = command_handoff_drops.clone();
    bus.subscribe_commands(&args.session, move |_k, bytes| {
        enqueue(&tx, Ingress::Command(bytes), &drops);
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe commands: {e}"))?;

    let tx = ingress_tx.clone();
    let drops = observation_handoff_drops.clone();
    bus.subscribe_observations(&args.session, move |_k, bytes| {
        enqueue(&tx, Ingress::Observation(bytes), &drops);
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe observations: {e}"))?;

    let mode_label = match mode {
        ConnectionMode::Open => "open/unauthenticated",
        ConnectionMode::Secure => "secure/fail-closed client config",
    };
    println!(
        "[ncp-observe] tapping '{}/session/{}/{{sensor,command,observation}}' \
         (read-only; {mode_label}). Ctrl-C / SIGTERM to finalize → {}",
        args.realm, args.session, args.out
    );
    if mode == ConnectionMode::Open {
        eprintln!(
            "[ncp-observe] WARNING: open mode does not authenticate the router/peers; \
             use --secure with NCP_ZENOH_CONFIG pointing at the strict NCP client config"
        );
    }
    let shutdown_error = shutdown_signal().await.err();

    // Stop new callbacks, drop their sender clones, then let the sole worker
    // drain every frame already admitted before finalization.
    let close_error = bus.close().await.err();
    drop(bus);
    drop(ingress_tx);
    let CaptureResult {
        mut observer,
        counters,
        error,
    } = worker
        .await
        .map_err(|join_error| anyhow::anyhow!("capture worker failed: {join_error}"))?;
    if let Some(error) = error {
        return Err(anyhow::anyhow!("capture worker latched: {error:#}"));
    }
    let (sf, cf, of) = (
        counters.sensor_decode_failures,
        counters.command_decode_failures,
        counters.observation_decode_failures,
    );
    let unstamped = counters.observation_unstamped;
    let decode_dropped = sf.saturating_add(cf).saturating_add(of);
    let handoff = (
        sensor_handoff_drops.load(Ordering::Relaxed),
        command_handoff_drops.load(Ordering::Relaxed),
        observation_handoff_drops.load(Ordering::Relaxed),
    );
    let handoff_dropped = handoff
        .0
        .saturating_add(handoff.1)
        .saturating_add(handoff.2);
    observer.record_ingress_drops(decode_dropped, unstamped, handoff_dropped)?;
    let stats = observer.finalize(&args.out)?;
    println!(
        "[ncp-observe] wrote {} (V,L,D,A) samples → {}",
        stats.kept_samples, args.out
    );
    println!("[ncp-observe] capture quality: {stats:?}");
    if decode_dropped > 0 {
        eprintln!(
            "[ncp-observe] WARNING: dropped {} frame(s) that failed the wire-{NCP_VERSION} \
             contract (sensor={sf} command={cf} observation={of}) — version-less, \
             incompatible, wrong-kind, observation-session-mismatched, or invalid; check the publisher \
             and subscribed session against this NCP build",
            decode_dropped
        );
    }
    if unstamped > 0 {
        eprintln!(
            "[ncp-observe] WARNING: dropped {unstamped} observation-plane frame(s) with \
             seq=0. Pull/RPC-form observations have no exact driving tick and are never \
             promoted into D by recency."
        );
    }
    if handoff_dropped > 0 {
        eprintln!(
            "[ncp-observe] WARNING: bounded ingress handoff dropped {} frame(s) under load \
             (sensor={} command={} observation={}); capture is incomplete",
            handoff_dropped, handoff.0, handoff.1, handoff.2
        );
    }
    if let Some(error) = close_error {
        anyhow::bail!("capture finalized, but Zenoh close failed: {error}");
    }
    if let Some(error) = shutdown_error {
        anyhow::bail!("capture finalized, but shutdown signal handling failed: {error}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ncp_core::{SessionRef, StreamPosition, NCP_VERSION};

    // Canonical wire-0.8 UUIDv4s for a valid stream identity in constructed frames.
    const EPOCH: &str = "00000000-0000-4000-8000-0000000000a1";
    const GENERATION: &str = "00000000-0000-4000-8000-0000000000c3";

    fn argv(args: &[&str]) -> Vec<String> {
        std::iter::once("ncp-observe")
            .chain(args.iter().copied())
            .map(str::to_string)
            .collect()
    }

    /// A wire-0.8 observation with a valid own `stream`, `session_id`, and
    /// `session.generation`; `source_seq` present = plane form, `None` = pull/RPC.
    fn obs_json(ver: &str, source_seq: Option<i64>) -> Vec<u8> {
        let frame = ObservationFrame {
            ncp_version: ver.to_string(),
            session_id: "s".to_string(),
            stream: StreamPosition {
                epoch: EPOCH.to_string(),
                seq: 1,
            },
            source: source_seq.map(|seq| StreamPosition {
                epoch: EPOCH.to_string(),
                seq,
            }),
            session: SessionRef {
                generation: GENERATION.to_string(),
            },
            ..Default::default()
        };
        serde_json::to_vec(&frame).unwrap()
    }

    #[test]
    fn accept_observation_enforces_wire_0_8_plane_source() {
        // A stamped, current-wire plane frame (carrying a source) is accepted.
        assert!(accept_observation(&obs_json(NCP_VERSION, Some(1))).is_ok());
        // A valid pull/RPC-form frame (no source) is rejected on the plane.
        assert!(matches!(
            accept_observation(&obs_json(NCP_VERSION, None)),
            Err(AcceptError::Unstamped)
        ));
        // A previous-wire frame is incompatible and dropped, never coerced.
        assert!(matches!(
            accept_observation(&obs_json("0.6", Some(1))),
            Err(AcceptError::Invalid)
        ));
        // A version-less frame is dropped.
        assert!(matches!(
            accept_observation(br#"{"kind":"observation_frame","session_id":"s"}"#),
            Err(AcceptError::Invalid)
        ));
        // A wrong-kind or unparseable payload is dropped.
        assert!(matches!(
            accept_observation(
                format!(r#"{{"kind":"sensor_frame","ncp_version":"{NCP_VERSION}"}}"#).as_bytes()
            ),
            Err(AcceptError::Invalid)
        ));
        assert!(matches!(
            accept_observation(b"not json"),
            Err(AcceptError::Invalid)
        ));
    }

    #[test]
    fn bounded_handoff_counts_saturation_drop() {
        let (sender, _receiver) = mpsc::channel(1);
        sender.try_send(Ingress::Sensor(vec![1])).unwrap();
        let drops = AtomicU64::new(0);

        enqueue(&sender, Ingress::Sensor(vec![2]), &drops);

        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn connection_mode_must_be_explicit() {
        let error = parse_args_from(argv(&[])).err().unwrap();

        assert!(error.to_string().contains("explicit connection mode"));
    }

    #[test]
    fn open_and_secure_modes_are_mutually_exclusive() {
        let error = parse_args_from(argv(&["--open", "--secure"]))
            .err()
            .unwrap();

        assert!(error.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn unknown_arguments_fail_closed() {
        let error = parse_args_from(argv(&["--open", "--unknown"]))
            .err()
            .unwrap();

        assert!(error.to_string().contains("unknown argument"));
    }

    #[test]
    fn explicit_open_mode_is_accepted() {
        let args = parse_args_from(argv(&[
            "--open",
            "--session",
            "s1",
            "--runlog",
            "run.jsonl",
        ]))
        .unwrap();

        assert_eq!(args.mode, Some(ConnectionMode::Open));
        assert_eq!(args.session, "s1");
    }

    #[test]
    fn canonical_runlog_is_required() {
        let error = parse_args_from(argv(&["--open"])).err().unwrap();

        assert!(error.to_string().contains("--runlog is required"));
    }

    #[tokio::test]
    async fn capture_worker_rejects_observation_payload_for_another_session() {
        let (sender, receiver) = mpsc::channel(1);
        sender
            .send(Ingress::Observation(obs_json(NCP_VERSION, Some(1))))
            .await
            .unwrap();
        drop(sender);

        let result = capture_worker(
            Observer::new("run", "nest", "task", Mapping::default()),
            receiver,
            "expected".to_string(),
        )
        .await;

        assert_eq!(result.observer.sample_count(), 0);
        assert_eq!(result.counters.observation_decode_failures, 1);
        assert!(result.error.is_none());
    }
}
