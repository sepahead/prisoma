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
//!     --session uav3 --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
//! # then:
//! cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json ...
//! ```
//!
//! Zenoh connectivity: `ncp-zenoh` reads the standard Zenoh configuration (e.g.
//! the `ZENOH_CONFIG` file/env conventions of the pinned Zenoh release) — if the
//! tap prints its banner but captures nothing, check that the observer can reach
//! the session's Zenoh routers/peers before suspecting the mapping.

use ncp_core::keys::Keys;
use ncp_core::{decode_validated, CommandFrame, ObservationFrame, SensorFrame};
use ncp_observer::{Mapping, Observer};
use ncp_zenoh::ZenohBus;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Accept an observation frame off the plane under the NCP wire-0.6 contract.
///
/// The observer is a passive, read-only tap, so it must never panic and never
/// drive anything — a frame that fails the wire contract is DROPPED and COUNTED,
/// exactly like a serde decode failure. `decode_validated` enforces the right
/// `kind`, a compatible `ncp_version` (an absent/incompatible version is rejected,
/// never coerced — the wire-0.6 gate), and the kind's `seq` bound
/// (`observation_frame` allows `seq >= 0`: `0` is the pull/RPC-reply form, `>= 1`
/// is the plane form that echoes the driving `SensorFrame.seq`).
///
/// Returns the accepted frame, or an `AcceptError` telling the caller which
/// counter to bump: `Invalid` (version-less / incompatible / wrong kind /
/// unparseable — a real drop) vs `Unstamped` (a valid frame whose `seq == 0`, so
/// its D-sample can only be joined by recency, not exact seq — this quantifies
/// the residual gap until the producer stamps `obs.seq`).
enum AcceptError {
    Invalid,
    // Boxed: `ObservationFrame` is large, and keeping the whole error enum small
    // keeps `Result<ObservationFrame, AcceptError>` cheap to move on the hot path.
    Unstamped(Box<ObservationFrame>),
}

fn accept_observation(bytes: &[u8]) -> Result<ObservationFrame, AcceptError> {
    match decode_validated::<ObservationFrame>(bytes) {
        Ok(f) if f.seq >= 1 => Ok(f),
        // Valid 0.6 frame but unstamped on the plane (seq 0 = pull-path form): the
        // observer still uses it (recency-fallback D join), but it is counted so a
        // capture that never got exact D alignment is diagnosable, not silent.
        Ok(f) => Err(AcceptError::Unstamped(Box::new(f))),
        Err(_) => Err(AcceptError::Invalid),
    }
}

struct Args {
    session: String,
    realm: String,
    out: String,
    runlog: Option<String>,
    model: String,
    task: String,
    language_channel: String,
    episode: Option<String>,
}

fn parse_args() -> Args {
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
    };
    let argv: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < argv.len() {
        let flag = argv[i].clone();
        // Every recognized flag takes exactly one value argument. Unknown args
        // advance by ONE (so they never swallow the following real flag), and a
        // value-less trailing flag is reported rather than silently set to "".
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
            eprintln!("[ncp-observe] ignoring unknown arg {flag:?}");
            i += 1;
            continue;
        }
        let value = match argv.get(i + 1) {
            Some(v) => v.clone(),
            None => {
                eprintln!(
                    "[ncp-observe] flag {flag:?} expects a value but none was given; ignoring"
                );
                break;
            }
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
    a
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
    let args = parse_args();
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
    );
    if let Some(path) = &args.runlog {
        observer = observer.with_runlog(path)?;
    }
    let observer = Arc::new(Mutex::new(observer));
    // A frame that fails the wire contract (undecodable, OR — since wire 0.6 —
    // version-less / incompatible / unstamped) must not vanish silently: count
    // per plane and report at finalize, so an empty or degraded capture is
    // diagnosable instead of mysterious.
    let sensor_decode_failures = Arc::new(AtomicU64::new(0));
    let command_decode_failures = Arc::new(AtomicU64::new(0));
    let observation_decode_failures = Arc::new(AtomicU64::new(0));
    // Valid observations published on the plane WITHOUT a stamped seq (seq 0):
    // usable (recency-fallback D join) but not exactly seq-aligned. Counting them
    // quantifies the residual producer-side gap (until Engram stamps obs.seq).
    let observation_unstamped = Arc::new(AtomicU64::new(0));

    let bus = ZenohBus::open_realm(Keys::new(args.realm.clone())).await?;

    let o = observer.clone();
    let fails = sensor_decode_failures.clone();
    // Wire 0.6: `decode_validated` enforces kind + compatible ncp_version + a
    // stamped seq >= 1 in one call, so an unstamped / version-less / incompatible
    // sensor frame is dropped-and-counted, never fed into the (V,L,D,A) join.
    bus.subscribe_sensors(&args.session, move |_k, bytes| {
        match decode_validated::<SensorFrame>(&bytes) {
            Ok(f) => o
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .on_sensor(&f),
            Err(_) => {
                fails.fetch_add(1, Ordering::Relaxed);
            }
        }
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe sensors: {e}"))?;

    let o = observer.clone();
    let fails = command_decode_failures.clone();
    bus.subscribe_commands(&args.session, move |_k, bytes| {
        match decode_validated::<CommandFrame>(&bytes) {
            Ok(f) => o
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .on_command(&f),
            Err(_) => {
                fails.fetch_add(1, Ordering::Relaxed);
            }
        }
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe commands: {e}"))?;

    let o = observer.clone();
    let fails = observation_decode_failures.clone();
    let unstamped = observation_unstamped.clone();
    bus.subscribe_observations(&args.session, move |_k, bytes| {
        match accept_observation(&bytes) {
            Ok(f) => o
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .on_observation(&f),
            // A valid-but-unstamped plane frame is still used (recency-fallback D),
            // but counted so the finalize summary can report how much D fell back.
            Err(AcceptError::Unstamped(f)) => {
                unstamped.fetch_add(1, Ordering::Relaxed);
                o.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .on_observation(&f);
            }
            Err(AcceptError::Invalid) => {
                fails.fetch_add(1, Ordering::Relaxed);
            }
        }
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe observations: {e}"))?;

    println!(
        "[ncp-observe] tapping '{}/session/{}/{{sensor,command,observation}}' (read-only). \
         Ctrl-C / SIGTERM to finalize → {}",
        args.realm, args.session, args.out
    );
    shutdown_signal().await?;

    let mut guard = observer
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let stats = guard.finalize(&args.out)?;
    println!(
        "[ncp-observe] wrote {} (V,L,D,A) samples → {}",
        stats.kept_samples, args.out
    );
    println!("[ncp-observe] capture quality: {stats:?}");
    let (sf, cf, of) = (
        sensor_decode_failures.load(Ordering::Relaxed),
        command_decode_failures.load(Ordering::Relaxed),
        observation_decode_failures.load(Ordering::Relaxed),
    );
    if sf + cf + of > 0 {
        eprintln!(
            "[ncp-observe] WARNING: dropped {} frame(s) that failed the wire-0.6 contract \
             (sensor={sf} command={cf} observation={of}) — version-less, incompatible, or \
             unstamped (seq<1); check the publisher's NCP wire version against the pinned \
             ncp-core (v0.6.0)",
            sf + cf + of
        );
    }
    let unstamped = observation_unstamped.load(Ordering::Relaxed);
    if unstamped > 0 {
        eprintln!(
            "[ncp-observe] NOTE: {unstamped} observation(s) arrived on the plane WITHOUT a \
             stamped seq (seq 0) and were joined by recency, not exact seq — D-axis alignment \
             for those samples is degraded. Wire 0.6 makes plane-published observation seq \
             stamping normative; this counts the producer-side gap until it stamps obs.seq."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ncp_core::NCP_VERSION;

    fn obs_json(ver: &str, seq: i64) -> Vec<u8> {
        format!(
            r#"{{"kind":"observation_frame","ncp_version":"{ver}","session_id":"s","seq":{seq}}}"#
        )
        .into_bytes()
    }

    #[test]
    fn accept_observation_enforces_wire_0_6() {
        // A stamped, current-wire plane frame is accepted for exact-seq join.
        assert!(accept_observation(&obs_json(NCP_VERSION, 1)).is_ok());
        // A valid but UNSTAMPED (seq 0) plane frame is usable (recency) but flagged.
        assert!(matches!(
            accept_observation(&obs_json(NCP_VERSION, 0)),
            Err(AcceptError::Unstamped(_))
        ));
        // A previous-wire (0.5) frame is INCOMPATIBLE and dropped, never coerced.
        assert!(matches!(
            accept_observation(&obs_json("0.5", 1)),
            Err(AcceptError::Invalid)
        ));
        // A version-less frame is dropped (wire 0.6 makes ncp_version mandatory).
        assert!(matches!(
            accept_observation(br#"{"kind":"observation_frame","session_id":"s","seq":1}"#),
            Err(AcceptError::Invalid)
        ));
        // A wrong-kind or unparseable payload is dropped.
        assert!(matches!(
            accept_observation(br#"{"kind":"sensor_frame","ncp_version":"0.6","seq":1}"#),
            Err(AcceptError::Invalid)
        ));
        assert!(matches!(
            accept_observation(b"not json"),
            Err(AcceptError::Invalid)
        ));
    }
}
