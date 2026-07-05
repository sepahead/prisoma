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
use ncp_core::{CommandFrame, ObservationFrame, SensorFrame};
use ncp_observer::{Mapping, Observer};
use ncp_zenoh::ZenohBus;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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
    // A frame that fails to deserialize (e.g. a wire-version-mismatched Engram)
    // must not vanish silently: count per plane and report at finalize, so an
    // empty capture is diagnosable instead of mysterious.
    let sensor_decode_failures = Arc::new(AtomicU64::new(0));
    let command_decode_failures = Arc::new(AtomicU64::new(0));
    let observation_decode_failures = Arc::new(AtomicU64::new(0));

    let bus = ZenohBus::open_realm(Keys::new(args.realm.clone())).await?;

    let o = observer.clone();
    let fails = sensor_decode_failures.clone();
    bus.subscribe_sensors(
        &args.session,
        move |_k, bytes| match serde_json::from_slice::<SensorFrame>(&bytes) {
            Ok(f) => o
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .on_sensor(&f),
            Err(_) => {
                fails.fetch_add(1, Ordering::Relaxed);
            }
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("subscribe sensors: {e}"))?;

    let o = observer.clone();
    let fails = command_decode_failures.clone();
    bus.subscribe_commands(
        &args.session,
        move |_k, bytes| match serde_json::from_slice::<CommandFrame>(&bytes) {
            Ok(f) => o
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .on_command(&f),
            Err(_) => {
                fails.fetch_add(1, Ordering::Relaxed);
            }
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("subscribe commands: {e}"))?;

    let o = observer.clone();
    let fails = observation_decode_failures.clone();
    bus.subscribe_observations(
        &args.session,
        move |_k, bytes| match serde_json::from_slice::<ObservationFrame>(&bytes) {
            Ok(f) => o
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .on_observation(&f),
            Err(_) => {
                fails.fetch_add(1, Ordering::Relaxed);
            }
        },
    )
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
            "[ncp-observe] WARNING: dropped undecodable frames (sensor={sf} command={cf} \
             observation={of}) — check the NCP wire version of the publisher against the \
             pinned ncp-core"
        );
    }
    Ok(())
}
