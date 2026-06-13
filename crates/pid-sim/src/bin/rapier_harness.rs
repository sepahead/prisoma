//! Physics-backed push-to-goal manipulation harness (milestone M3).
//!
//! Runs a scripted "push the cube toward a goal" episode on a real Rapier3D
//! backend (or, with `--backend null`, on the kinematic Null backend for a
//! cross-backend robustness check), writes the canonical run log, and prints the
//! externally meaningful success label.
//!
//! Example:
//! ```text
//! cargo run -p pid-sim --features rapier --bin pid-rapier-harness -- \
//!     --runlog outputs/rapier_push_runlog.jsonl --push-impulse 0.18
//! ```

use anyhow::{bail, Context, Result};
use pid_runlog::RunLogWriter;
use pid_sim::manipulation::{run_push_episode, PushEpisode, PushTaskParams};
use pid_sim::physics::{NullPhysicsBackend, PhysicsWorldConfig};
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct Args {
    runlog: PathBuf,
    summary_json: Option<PathBuf>,
    backend: Backend,
    params: PushTaskParams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Rapier,
    Null,
}

fn main() -> Result<()> {
    let args = match parse_args()? {
        Some(args) => args,
        None => return Ok(()),
    };

    if let Some(parent) = args.runlog.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }

    let world = PhysicsWorldConfig::default();
    let episode = match args.backend {
        Backend::Rapier => run_rapier(&world, &args.params)?,
        Backend::Null => {
            let mut backend = NullPhysicsBackend::new();
            run_push_episode(&mut backend, &world, &args.params)?
        }
    };

    let mut writer = RunLogWriter::create(&args.runlog)?;
    for event in &episode.events {
        writer.append(event)?;
    }
    writer.flush()?;

    if let Some(summary_path) = &args.summary_json {
        write_summary(summary_path, &args, &episode)?;
    }

    println!(
        "backend={} success={} final_x={:.4} goal_x={:.4} dist={:.4} contacts<= {} steps={}",
        match args.backend {
            Backend::Rapier => "rapier3d",
            Backend::Null => "null",
        },
        episode.success,
        episode.final_position[0],
        args.params.goal_x,
        episode.distance_to_goal,
        episode.max_contact_count,
        episode.total_steps,
    );
    println!("wrote {}", args.runlog.display());
    Ok(())
}

#[cfg(feature = "rapier")]
fn run_rapier(world: &PhysicsWorldConfig, params: &PushTaskParams) -> Result<PushEpisode> {
    use pid_sim::physics::rapier_adapter::RapierBackend;
    let mut backend = RapierBackend::new(world.clone());
    // Ground slab whose top sits at z=0, large enough to contain the slide.
    backend.add_ground_slab(5.0, 0.1, 0.5);
    run_push_episode(&mut backend, world, params)
}

#[cfg(not(feature = "rapier"))]
fn run_rapier(_world: &PhysicsWorldConfig, _params: &PushTaskParams) -> Result<PushEpisode> {
    bail!("this binary was built without the `rapier` feature; rebuild with --features rapier")
}

fn write_summary(path: &PathBuf, args: &Args, episode: &PushEpisode) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let value = serde_json::json!({
        "backend": match args.backend { Backend::Rapier => "rapier3d", Backend::Null => "null" },
        "success": episode.success,
        "final_position": episode.final_position,
        "distance_to_goal": episode.distance_to_goal,
        "max_contact_count": episode.max_contact_count,
        "total_steps": episode.total_steps,
        "params": args.params,
    });
    std::fs::write(path, serde_json::to_string_pretty(&value)?)?;
    Ok(())
}

fn parse_args() -> Result<Option<Args>> {
    let mut runlog = PathBuf::from("outputs/rapier_push_runlog.jsonl");
    let mut summary_json = None;
    let mut backend = Backend::Rapier;
    let mut params = PushTaskParams::default();

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--runlog" => runlog = PathBuf::from(next(&mut it, "--runlog")?),
            "--summary-json" => {
                summary_json = Some(PathBuf::from(next(&mut it, "--summary-json")?))
            }
            "--backend" => {
                backend = match next(&mut it, "--backend")?.as_str() {
                    "rapier" | "rapier3d" => Backend::Rapier,
                    "null" => Backend::Null,
                    other => bail!("unknown --backend: {other} (expected rapier|null)"),
                };
            }
            "--push-impulse" => params.push_impulse = parse_f64(&mut it, "--push-impulse")?,
            "--goal-x" => params.goal_x = parse_f64(&mut it, "--goal-x")?,
            "--tolerance" => params.tolerance = parse_f64(&mut it, "--tolerance")?,
            "--dt" => params.dt = parse_f64(&mut it, "--dt")?,
            "--settle-steps" => params.settle_steps = parse_usize(&mut it, "--settle-steps")?,
            "--coast-steps" => params.coast_steps = parse_usize(&mut it, "--coast-steps")?,
            "--run-id" => params.run_id = next(&mut it, "--run-id")?,
            "--help" | "-h" => {
                print_usage();
                return Ok(None);
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    Ok(Some(Args {
        runlog,
        summary_json,
        backend,
        params,
    }))
}

fn next(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    it.next()
        .ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))
}

fn parse_f64(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<f64> {
    next(it, flag)?
        .parse::<f64>()
        .with_context(|| format!("{flag} requires a number"))
}

fn parse_usize(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<usize> {
    next(it, flag)?
        .parse::<usize>()
        .with_context(|| format!("{flag} requires a non-negative integer"))
}

fn print_usage() {
    println!(
        "Usage: pid-rapier-harness [--runlog PATH] [--summary-json PATH] [--backend rapier|null]"
    );
    println!("       [--push-impulse F] [--goal-x F] [--tolerance F] [--dt F]");
    println!("       [--settle-steps N] [--coast-steps N] [--run-id ID]");
}
