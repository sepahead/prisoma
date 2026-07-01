//! `ncp-observe` — run the passive NCP → (V,L,D,A) observer.
//!
//! Subscribes read-only to a session's NCP data planes over Zenoh and, on Ctrl-C,
//! writes an `OfflineVldaDataset` artifact (run it through `pid-offline-harness`)
//! plus a provenance run log. It drives nothing — the Agent Bridge stays the only
//! control plane.
//!
//! ```bash
//! cargo run -p ncp-observer --bin ncp-observe -- \
//!     --session uav3 --out outputs/ncp_vlda.json --runlog outputs/ncp_runlog.jsonl
//! # then:
//! cargo run -p pid-sim --bin pid-offline-harness -- --input outputs/ncp_vlda.json ...
//! ```

use ncp_core::keys::Keys;
use ncp_core::{CommandFrame, ObservationFrame, SensorFrame};
use ncp_observer::{Mapping, Observer};
use ncp_zenoh::ZenohBus;
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

    let bus = ZenohBus::open_realm(Keys::new(args.realm.clone())).await?;

    let o = observer.clone();
    bus.subscribe_sensors(&args.session, move |_k, bytes| {
        if let Ok(f) = serde_json::from_slice::<SensorFrame>(&bytes) {
            o.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .on_sensor(&f);
        }
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe sensors: {e}"))?;

    let o = observer.clone();
    bus.subscribe_commands(&args.session, move |_k, bytes| {
        if let Ok(f) = serde_json::from_slice::<CommandFrame>(&bytes) {
            o.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .on_command(&f);
        }
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe commands: {e}"))?;

    let o = observer.clone();
    bus.subscribe_observations(&args.session, move |_k, bytes| {
        if let Ok(f) = serde_json::from_slice::<ObservationFrame>(&bytes) {
            o.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .on_observation(&f);
        }
    })
    .await
    .map_err(|e| anyhow::anyhow!("subscribe observations: {e}"))?;

    println!(
        "[ncp-observe] tapping '{}/session/{}/{{sensor,command,observation}}' (read-only). Ctrl-C to finalize → {}",
        args.realm, args.session, args.out
    );
    tokio::signal::ctrl_c().await?;

    let mut guard = observer
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.finalize(&args.out)?;
    println!(
        "[ncp-observe] wrote {} (V,L,D,A) samples → {}",
        guard.sample_count(),
        args.out
    );
    Ok(())
}
