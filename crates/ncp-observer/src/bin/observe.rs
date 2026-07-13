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

use ncp_core::keys::valid_id_segment;
use ncp_core::keys::Keys;
use ncp_core::NCP_VERSION;
use ncp_observer::{
    ingest_wire_frame, IngressPlane, IngressRoutes, Mapping, Observer, RawIngressCounters,
};
use ncp_zenoh::ZenohBus;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

const HANDOFF_CAPACITY: usize = 64;

enum Ingress {
    Sensor(Vec<u8>),
    Command(Vec<u8>),
    Observation(Vec<u8>),
}

struct CaptureResult {
    observer: Observer,
    counters: RawIngressCounters,
    error: Option<anyhow::Error>,
}

async fn capture_worker(
    mut observer: Observer,
    mut ingress: mpsc::Receiver<Ingress>,
) -> CaptureResult {
    let mut counters = RawIngressCounters::default();
    let mut error = None;
    while let Some(frame) = ingress.recv().await {
        let (plane, bytes) = match frame {
            Ingress::Sensor(bytes) => (IngressPlane::Sensor, bytes),
            Ingress::Command(bytes) => (IngressPlane::Command, bytes),
            Ingress::Observation(bytes) => (IngressPlane::Observation, bytes),
        };
        let result = ingest_wire_frame(&mut observer, plane, &bytes, &mut counters).map(drop);
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

fn enqueue(
    sender: &mpsc::Sender<Ingress>,
    frame: Ingress,
    handoff_drops: &AtomicU64,
    oversized_drops: &AtomicU64,
    max_wire_frame_bytes: usize,
) {
    let bytes = match &frame {
        Ingress::Sensor(bytes) | Ingress::Command(bytes) | Ingress::Observation(bytes) => bytes,
    };
    if bytes.len() > max_wire_frame_bytes {
        let _ = oversized_drops.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
            Some(count.saturating_add(1))
        });
        return;
    }
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
    if a.session.is_empty() || a.session.len() > 64 || !valid_id_segment(&a.session) {
        anyhow::bail!("--session must be a valid NCP key segment of 1..=64 bytes");
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
    let mode_label = match mode {
        ConnectionMode::Open => "open/unauthenticated",
        ConnectionMode::Secure => "secure/fail-closed client config",
    };
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
    // session; the first validated authorizing sensor locks the live generation.
    .with_expected_session(args.session.clone())?
    .with_capture_transport(args.realm.clone(), mode_label, HANDOFF_CAPACITY)?;
    let runlog_path = args
        .runlog
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("run-log requirement was not validated"))?;
    observer = observer.with_runlog(runlog_path)?;
    let max_wire_frame_bytes = observer.limits().max_wire_frame_bytes;
    let keys = Keys::try_new(args.realm.clone())
        .map_err(|error| anyhow::anyhow!("invalid NCP realm {:?}: {error}", args.realm))?;
    let sensor_key = keys
        .try_sensor(&args.session)
        .map_err(|error| anyhow::anyhow!("invalid sensor key: {error}"))?;
    let command_key = keys
        .try_command(&args.session)
        .map_err(|error| anyhow::anyhow!("invalid command key: {error}"))?;
    let observation_key = keys
        .try_observation(&args.session)
        .map_err(|error| anyhow::anyhow!("invalid observation key: {error}"))?;
    let ingress_routes = IngressRoutes::new(sensor_key, command_key, observation_key)?;
    let bus = match mode {
        ConnectionMode::Open => ZenohBus::open_realm(keys).await?,
        ConnectionMode::Secure => ZenohBus::open_secure(keys).await?,
    };

    // Zenoh callbacks only enqueue owned bytes into this bounded handoff. One
    // Tokio worker owns Observer and performs decode/join/log bookkeeping in
    // arrival order, so callback threads never block on a mutex or do disk/JSON
    // work. Saturation drops are explicit per-plane counters.
    let (ingress_tx, ingress_rx) = mpsc::channel(HANDOFF_CAPACITY);
    let worker = tokio::spawn(capture_worker(observer, ingress_rx));
    let sensor_handoff_drops = Arc::new(AtomicU64::new(0));
    let command_handoff_drops = Arc::new(AtomicU64::new(0));
    let observation_handoff_drops = Arc::new(AtomicU64::new(0));
    let oversized_drops = Arc::new(AtomicU64::new(0));
    let routing_key_mismatches = Arc::new(AtomicU64::new(0));

    // Subscribe once at the raw session boundary. The typed per-plane helpers
    // prefilter invalid payloads inside ncp-zenoh, which would hide the very
    // version/kind/schema faults this observer must count. Exact key matching
    // below preserves the plane boundary before the shared production decoder.
    let tx = ingress_tx.clone();
    let sensor_drops = sensor_handoff_drops.clone();
    let command_drops = command_handoff_drops.clone();
    let observation_drops = observation_handoff_drops.clone();
    let oversized = oversized_drops.clone();
    let routing_mismatches = routing_key_mismatches.clone();
    bus.subscribe_session(&args.session, move |key, bytes| {
        let (frame, drops) = match ingress_routes.classify(&key) {
            Some(IngressPlane::Sensor) => (Ingress::Sensor(bytes), sensor_drops.as_ref()),
            Some(IngressPlane::Command) => (Ingress::Command(bytes), command_drops.as_ref()),
            Some(IngressPlane::Observation) => {
                (Ingress::Observation(bytes), observation_drops.as_ref())
            }
            None => {
                let _ = routing_mismatches.fetch_update(
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                    |count| Some(count.saturating_add(1)),
                );
                return;
            }
        };
        enqueue(&tx, frame, drops, &oversized, max_wire_frame_bytes);
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe raw session data planes: {e}"))?;

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
        error: capture_error,
    } = worker
        .await
        .map_err(|join_error| anyhow::anyhow!("capture worker failed: {join_error}"))?;
    if capture_error.is_some() {
        observer.record_capture_worker_failure()?;
    }
    if close_error.is_some() || shutdown_error.is_some() {
        observer.record_capture_teardown_failure()?;
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
    let callback_oversized = oversized_drops.load(Ordering::Relaxed);
    let callback_route_mismatches = routing_key_mismatches.load(Ordering::Relaxed);
    let oversized = counters.oversized_frames.saturating_add(callback_oversized);
    let route_mismatches = counters
        .routing_key_mismatches
        .saturating_add(callback_route_mismatches);
    observer.record_callback_drops(
        callback_oversized,
        callback_route_mismatches,
        handoff_dropped,
    )?;
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
             incompatible, wrong-kind, duplicate-key, or invalid; check the publisher \
             and subscribed session against this NCP build",
            decode_dropped
        );
    }
    if oversized > 0 {
        eprintln!(
            "[ncp-observe] WARNING: rejected {oversized} raw frame(s) above the \
             {max_wire_frame_bytes}-byte ingress limit before decode"
        );
    }
    if route_mismatches > 0 {
        eprintln!(
            "[ncp-observe] WARNING: rejected {route_mismatches} receipt(s) outside the \
             exact sensor/command/observation keys for this session"
        );
    }
    if unstamped > 0 {
        eprintln!(
            "[ncp-observe] WARNING: dropped {unstamped} observation-plane frame(s) with \
             no source correlation. Pull/RPC-form observations have no exact driving tick \
             and are never promoted into D by recency."
        );
    }
    if handoff_dropped > 0 {
        eprintln!(
            "[ncp-observe] WARNING: bounded ingress handoff dropped {} frame(s) under load \
             (sensor={} command={} observation={}); capture is incomplete",
            handoff_dropped, handoff.0, handoff.1, handoff.2
        );
    }
    if let Some(error) = capture_error {
        anyhow::bail!("capture finalized as failed after worker error: {error:#}");
    }
    if let Some(error) = close_error {
        anyhow::bail!("capture finalized, but Zenoh close failed: {error}");
    }
    if let Some(error) = shutdown_error {
        anyhow::bail!("capture finalized, but shutdown signal handling failed: {error}");
    }
    if stats.kept_samples == 0 {
        anyhow::bail!(
            "capture finalized with zero analyzable samples; artifact is diagnostic-only"
        );
    }
    if !matches!(
        stats.capture_integrity(),
        "complete" | "complete_with_warning"
    ) {
        anyhow::bail!(
            "capture finalized with capture_integrity={}; artifact is diagnostic-only",
            stats.capture_integrity()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ncp_core::{ObservationFrame, SessionRef, StreamPosition, NCP_VERSION};

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
    fn shared_ingress_enforces_wire_0_8_observation_plane_source() {
        let mut observer = Observer::new("run", "nest", "task", Mapping::default());
        let mut counters = RawIngressCounters::default();
        // A stamped, current-wire plane frame (carrying a source) is accepted.
        assert_eq!(
            ingest_wire_frame(
                &mut observer,
                IngressPlane::Observation,
                &obs_json(NCP_VERSION, Some(1)),
                &mut counters,
            )
            .unwrap(),
            ncp_observer::RawIngressDisposition::Applied
        );
        // A valid pull/RPC-form frame (no source) is rejected on the plane.
        assert_eq!(
            ingest_wire_frame(
                &mut observer,
                IngressPlane::Observation,
                &obs_json(NCP_VERSION, None),
                &mut counters,
            )
            .unwrap(),
            ncp_observer::RawIngressDisposition::UnstampedObservationDropped
        );
        // A previous-wire frame is incompatible and dropped, never coerced.
        assert_eq!(
            ingest_wire_frame(
                &mut observer,
                IngressPlane::Observation,
                &obs_json("0.6", Some(1)),
                &mut counters,
            )
            .unwrap(),
            ncp_observer::RawIngressDisposition::DecodeDropped
        );
        // A version-less frame is dropped.
        assert_eq!(
            ingest_wire_frame(
                &mut observer,
                IngressPlane::Observation,
                br#"{"kind":"observation_frame","session_id":"s"}"#,
                &mut counters,
            )
            .unwrap(),
            ncp_observer::RawIngressDisposition::DecodeDropped
        );
        // A wrong-kind or unparseable payload is dropped.
        for bytes in [
            format!(r#"{{"kind":"sensor_frame","ncp_version":"{NCP_VERSION}"}}"#).into_bytes(),
            b"not json".to_vec(),
        ] {
            assert_eq!(
                ingest_wire_frame(
                    &mut observer,
                    IngressPlane::Observation,
                    &bytes,
                    &mut counters,
                )
                .unwrap(),
                ncp_observer::RawIngressDisposition::DecodeDropped
            );
        }
    }

    #[test]
    fn bounded_handoff_counts_saturation_drop() {
        let (sender, _receiver) = mpsc::channel(1);
        sender.try_send(Ingress::Sensor(vec![1])).unwrap();
        let drops = AtomicU64::new(0);

        let oversized = AtomicU64::new(0);
        enqueue(&sender, Ingress::Sensor(vec![2]), &drops, &oversized, 1024);

        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn oversized_frame_is_rejected_before_handoff() {
        let (sender, mut receiver) = mpsc::channel(1);
        let drops = AtomicU64::new(0);
        let oversized = AtomicU64::new(0);

        enqueue(
            &sender,
            Ingress::Sensor(vec![0; 5]),
            &drops,
            &oversized,
            4,
        );

        assert_eq!(oversized.load(Ordering::Relaxed), 1);
        assert!(receiver.try_recv().is_err());
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
    fn invalid_session_key_segments_fail_without_panicking() {
        for session in ["bad/*", "bad/session", "bad session"] {
            let error = parse_args_from(argv(&[
                "--open",
                "--session",
                session,
                "--runlog",
                "run.jsonl",
            ]))
            .unwrap_err();
            assert!(error.to_string().contains("valid NCP key segment"));
        }
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
            Observer::new("run", "nest", "task", Mapping::default())
                .with_expected_session("expected")
                .unwrap(),
            receiver,
        )
        .await;

        assert_eq!(result.observer.sample_count(), 0);
        assert_eq!(result.counters.observation_decode_failures, 0);
        assert_eq!(result.observer.stats().session_mismatch_dropped, 1);
        assert!(result.error.is_none());
    }
}
