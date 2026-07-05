//! VLA Demo: Demonstrate prisoma visualization with synthetic data.
//!
//! This binary:
//! 1. Generates synthetic VLA episode data
//! 2. Computes PID metrics using pid-core
//! 3. Logs everything to Rerun for visualization
//!
//! Usage:
//!   cargo run -p pid-rerun --bin vla-demo
//!   cargo run -p pid-rerun --bin vla-demo -- --save demo.rrd

use anyhow::Result;
use ndarray::{s, Array2};
use pid_core::{
    distance_concentration_stats, intrinsic_dimension_levina_bickel, pid2_isx,
    DistanceConcentrationConfig, IntrinsicDimConfig, IsxConfig, KsgConfig, MatRef,
    NegativeHandling, Pid2Config,
};
use pid_rerun::adapters::{PidLogger, VlaLogger};
use pid_rerun::data::generate_synthetic_episode;
use rerun::RecordingStreamBuilder;
use std::env;

fn main() -> Result<()> {
    println!("=== prisoma Demo with Rerun Visualization ===\n");

    // Parse args
    let args: Vec<String> = env::args().collect();
    let mut save_path = None;
    let mut serve = false;
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--save" if i + 1 < args.len() => {
                save_path = Some(args[i + 1].clone());
                i += 2;
            }
            "--serve" => {
                serve = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    // Initialize Rerun
    println!("Initializing Rerun...");
    let rec = if save_path.is_some() {
        // Use buffered mode when saving to file
        RecordingStreamBuilder::new("prisoma_demo").buffered()?
    } else {
        // Spawn viewer for interactive use
        RecordingStreamBuilder::new("prisoma_demo").spawn()?
    };

    // Create loggers
    let pid_logger = PidLogger::new(&rec);
    let vla_logger = VlaLogger::new(&rec);

    // Generate synthetic VLA episode
    println!("Generating synthetic VLA episode...");
    let n_frames = 100;
    let vision_dim = 64; // Use smaller dim for demo (real: 768-4096)
    let action_dim = 7;
    let episode = generate_synthetic_episode(n_frames, vision_dim, action_dim, 42);
    episode.validate_shapes()?;

    println!(
        "  Episode: {} ({} frames, {:.1}s)",
        episode.episode_id,
        episode.frames.len(),
        episode.duration()
    );
    println!("  Instruction: {}", episode.instruction);

    // Log the episode
    println!("\nLogging episode to Rerun...");
    vla_logger.log_episode(&episode)?;

    // Compute PID metrics over sliding windows
    println!("\nComputing PID metrics (sliding window)...");
    let window_size = 20;
    let step_size = 5;

    let vision_embeddings = episode.vision_embeddings();
    let actions = episode.actions();
    let timestamps = episode.timestamps();

    let mut window_start = 0;
    while window_start + window_size <= n_frames {
        let window_end = window_start + window_size;

        // Extract window data
        let v_window = vision_embeddings.slice(s![window_start..window_end, ..]);
        let a_window = actions.slice(s![window_start..window_end, ..]);

        let n = window_size;

        // Compute PID if we have enough samples
        let timestamp = timestamps[window_start + window_size / 2];

        // Convert to contiguous slice for pid-core
        let v_flat: Vec<f64> = v_window.iter().cloned().collect();

        // Check geometry using pid-core MatRef API
        let id_config = IntrinsicDimConfig::default();
        let v_mat = MatRef::new(&v_flat, n, vision_dim).ok();
        let intrinsic_dim = v_mat
            .and_then(|m| intrinsic_dimension_levina_bickel(m, &id_config).ok())
            .unwrap_or(f64::NAN);

        let dc_config = DistanceConcentrationConfig::default();
        let dc_cv = v_mat
            .and_then(|m| distance_concentration_stats(m, &dc_config).ok())
            .map(|s| s.pairwise_cv)
            .unwrap_or(f64::NAN);

        // Log geometry
        pid_logger.log_geometry(timestamp, intrinsic_dim, dc_cv, None)?;

        // Compute PID2 using a subset of dimensions (for speed)
        let v_source_1: Vec<f64> = v_window.slice(s![.., 0..2]).iter().cloned().collect();
        let v_source_2: Vec<f64> = v_window.slice(s![.., 2..3]).iter().cloned().collect();
        let a_subset: Vec<f64> = a_window.slice(s![.., 0..1]).iter().cloned().collect();

        let ksg_config = KsgConfig {
            negative_handling: NegativeHandling::Allow,
            ..Default::default()
        };
        let pid_config = Pid2Config {
            ksg: ksg_config,
            isx: IsxConfig::default(),
        };
        let v1_mat = MatRef::new(&v_source_1, n, 2).ok();
        let v2_mat = MatRef::new(&v_source_2, n, 1).ok();
        let a_mat = MatRef::new(&a_subset, n, 1).ok();

        // NB: this synthetic demo decomposes two VISION subspaces (dims 0..2
        // vs dim 2), so `unique_s2` is a second-vision-subspace term, not a
        // language term. It is fed into the `unique_l` slot below only to
        // exercise the plotting path — a real (V,L)→A screen would put an
        // actual language source here.
        let (redundancy, unique_v, unique_v2, synergy) = match (v1_mat, v2_mat, a_mat) {
            (Some(v1), Some(v2), Some(a)) => match pid2_isx(v1, v2, a, &pid_config) {
                Ok(pid) => (pid.redundancy, pid.unique_s1, pid.unique_s2, pid.synergy),
                Err(err) => {
                    pid_logger.log_event(
                        timestamp,
                        "WARN",
                        &format!("PID2 estimate failed for demo window: {err}"),
                    )?;
                    (0.0, 0.0, 0.0, 0.0)
                }
            },
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        // Log PID metrics (unique_v2 occupies the unique_l slot — see note above).
        pid_logger.log_pid_atoms(timestamp, redundancy, synergy, unique_v, unique_v2)?;

        // Log events at interesting points
        if intrinsic_dim > 20.0 {
            pid_logger.log_event(
                timestamp,
                "WARN",
                &format!(
                    "High intrinsic dimension detected: {:.1} (threshold: 20)",
                    intrinsic_dim
                ),
            )?;
        }

        window_start += step_size;
    }

    // Log ghost splat demo
    println!("\nLogging ghost splat overlay...");
    for i in 0..10 {
        let t = i as f64;

        // Simulated predicted positions
        let predicted = Array2::from_shape_vec(
            (5, 3),
            vec![
                0.5 - 0.05 * t,
                0.0,
                0.1,
                0.2,
                0.3,
                0.05,
                0.3,
                0.1,
                0.08,
                0.4,
                -0.1,
                0.12,
                0.35,
                0.2,
                0.09,
            ],
        )?;

        // Simulated actual positions (slightly different)
        let actual = Array2::from_shape_vec(
            (5, 3),
            vec![
                0.48 - 0.05 * t,
                0.02,
                0.11,
                0.21,
                0.29,
                0.06,
                0.31,
                0.09,
                0.07,
                0.39,
                -0.08,
                0.13,
                0.36,
                0.19,
                0.1,
            ],
        )?;

        // PID values for coloring
        let pid_values = vec![0.3, -0.1, 0.5, 0.2, 0.0];

        vla_logger.log_flow(t, &predicted, &actual)?;
        vla_logger.log_ghost_splat(t, &predicted, &pid_values)?;
    }

    println!("\n=== Demo complete! ===");
    println!("The Rerun viewer should now show:");
    println!("  - PID metrics over time (pid/metrics/*)");
    println!("  - Geometry diagnostics (pid/geometry/*)");
    println!("  - VLA action trajectories (vla/action/*)");
    println!("  - Object positions (world/objects)");
    println!("  - Flow predictions (flow/*)");
    println!("  - Ghost splat overlay (world/ghost)");

    // Save if requested
    if let Some(path) = &save_path {
        println!("\nSaving recording to: {}", path);
        rec.save(path)?;
        println!("Recording saved successfully!");
    } else if serve {
        // Keep running so viewer stays connected (only in interactive mode)
        println!("\nPress Ctrl+C to exit...");
        std::thread::sleep(std::time::Duration::from_secs(3600));
    } else {
        println!("\nRun with --serve to keep the interactive viewer connected.");
    }

    Ok(())
}
