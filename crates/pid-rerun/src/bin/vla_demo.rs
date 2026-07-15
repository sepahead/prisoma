//! VLA Demo: Demonstrate prisoma visualization with synthetic data.
//!
//! This binary:
//! 1. Generates synthetic VLA episode data
//! 2. Computes PID metrics using pid-core
//! 3. Logs everything to Rerun for visualization
//!
//! Usage:
//!   cargo run -p pid-rerun --bin vla-demo -- --serve
//!   cargo run -p pid-rerun --bin vla-demo -- --save demo.rrd

use anyhow::{bail, ensure, Context, Result};
use ndarray::{s, Array2};
use pid_core::diagnostics::{
    distance_concentration_stats, intrinsic_dimension_levina_bickel, DistanceConcentrationConfig,
    IntrinsicDimConfig,
};
use pid_core::experimental::continuous::{pid2_isx, IsxConfig, Pid2Config};
use pid_core::stable::continuous::{KsgConfig, NegativeHandling};
use pid_core::MatRef;
use pid_rerun::adapters::{PidLogger, VlaLogger};
use pid_rerun::data::generate_synthetic_episode;
use pid_rerun::{init_recording, save_recording};
use std::env;

const DEMO_PID_SOURCE_LABELS: [&str; 2] =
    ["vision_embedding_dims_0_1", "vision_embedding_dims_2_3"];

#[derive(Debug, PartialEq, Eq)]
struct DemoOptions {
    save_path: Option<String>,
    serve: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PidAtoms {
    redundancy: f64,
    synergy: f64,
    unique_v1: f64,
    unique_v2: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct GeometryDiagnostics {
    intrinsic_dim: f64,
    distance_concentration_cv: f64,
}

trait DemoWindowSink {
    fn atom_metrics(
        &mut self,
        timestamp: f64,
        atoms: PidAtoms,
        source_labels: [&str; 2],
    ) -> Result<()>;
    fn geometry_metrics(&mut self, timestamp: f64, diagnostics: GeometryDiagnostics) -> Result<()>;
    fn diagnostic(&mut self, timestamp: f64, message: &str) -> Result<()>;
}

impl DemoWindowSink for PidLogger<'_> {
    fn atom_metrics(
        &mut self,
        timestamp: f64,
        atoms: PidAtoms,
        source_labels: [&str; 2],
    ) -> Result<()> {
        self.log_source_labeled_pid_atoms(
            timestamp,
            atoms.redundancy,
            atoms.synergy,
            [atoms.unique_v1, atoms.unique_v2],
            source_labels,
        )
    }

    fn geometry_metrics(&mut self, timestamp: f64, diagnostics: GeometryDiagnostics) -> Result<()> {
        self.log_geometry(
            timestamp,
            diagnostics.intrinsic_dim,
            diagnostics.distance_concentration_cv,
            None,
        )
    }

    fn diagnostic(&mut self, timestamp: f64, message: &str) -> Result<()> {
        self.log_event(timestamp, "WARN", message)
    }
}

fn emit_pid_window(
    sink: &mut impl DemoWindowSink,
    timestamp: f64,
    estimate: Result<PidAtoms>,
) -> Result<bool> {
    match estimate {
        Ok(atoms) => {
            sink.atom_metrics(timestamp, atoms, DEMO_PID_SOURCE_LABELS)?;
            Ok(true)
        }
        Err(error) => {
            sink.diagnostic(
                timestamp,
                &format!("PID2 window abstained; no atom metrics emitted: {error:#}"),
            )?;
            Ok(false)
        }
    }
}

fn emit_geometry_window(
    sink: &mut impl DemoWindowSink,
    timestamp: f64,
    estimate: Result<GeometryDiagnostics>,
) -> Result<Option<GeometryDiagnostics>> {
    match estimate {
        Ok(diagnostics) => {
            sink.geometry_metrics(timestamp, diagnostics)?;
            Ok(Some(diagnostics))
        }
        Err(error) => {
            sink.diagnostic(
                timestamp,
                &format!("geometry diagnostics unavailable; no numeric metrics emitted: {error:#}"),
            )?;
            Ok(None)
        }
    }
}

fn estimate_geometry(
    values: &[f64],
    n_rows: usize,
    n_columns: usize,
) -> Result<GeometryDiagnostics> {
    let matrix = MatRef::new(values, n_rows, n_columns)
        .context("vision window has an invalid matrix shape")?;
    let intrinsic_dim = intrinsic_dimension_levina_bickel(matrix, &IntrinsicDimConfig::default())
        .context("intrinsic-dimension diagnostic abstained")?;
    let distance_concentration_cv =
        distance_concentration_stats(matrix, &DistanceConcentrationConfig::default())
            .context("distance-concentration diagnostic abstained")?
            .pairwise_cv;
    ensure!(
        intrinsic_dim.is_finite()
            && intrinsic_dim >= 0.0
            && distance_concentration_cv.is_finite()
            && distance_concentration_cv >= 0.0,
        "geometry diagnostics returned a non-finite or negative value"
    );
    Ok(GeometryDiagnostics {
        intrinsic_dim,
        distance_concentration_cv,
    })
}

fn estimate_pid_atoms(
    source_1: &[f64],
    source_2: &[f64],
    target: &[f64],
    n_rows: usize,
    source_columns: usize,
    target_columns: usize,
    config: &Pid2Config,
) -> Result<PidAtoms> {
    let source_1 = MatRef::new(source_1, n_rows, source_columns)
        .context("first vision source has an invalid matrix shape")?;
    let source_2 = MatRef::new(source_2, n_rows, source_columns)
        .context("second vision source has an invalid matrix shape")?;
    let target = MatRef::new(target, n_rows, target_columns)
        .context("action target has an invalid matrix shape")?;
    let estimate = pid2_isx(source_1, source_2, target, config)
        .context("continuous PID2 estimator abstained")?;
    let atoms = PidAtoms {
        redundancy: estimate.redundancy,
        synergy: estimate.synergy,
        unique_v1: estimate.unique_s1,
        unique_v2: estimate.unique_s2,
    };
    ensure!(
        [
            atoms.redundancy,
            atoms.synergy,
            atoms.unique_v1,
            atoms.unique_v2,
        ]
        .into_iter()
        .all(f64::is_finite),
        "continuous PID2 estimator returned a non-finite atom"
    );
    ensure!(
        (atoms.redundancy + atoms.unique_v1 + atoms.unique_v2 + atoms.synergy).is_finite(),
        "continuous PID2 atom sum overflowed the finite MI range"
    );
    Ok(atoms)
}

fn parse_options(args: &[String]) -> Result<DemoOptions> {
    let mut save_path = None;
    let mut serve = false;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--save" => {
                if save_path.is_some() {
                    bail!("--save may be specified only once");
                }
                let Some(path) = args.get(i + 1) else {
                    bail!("--save requires a path");
                };
                if path.is_empty() || path.starts_with('-') {
                    bail!("--save requires a nonempty path that does not start with '-'");
                }
                save_path = Some(path.clone());
                i += 2;
            }
            "--serve" => {
                if serve {
                    bail!("--serve may be specified only once");
                }
                serve = true;
                i += 1;
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    if save_path.is_some() && serve {
        bail!("--save and --serve are mutually exclusive");
    }
    Ok(DemoOptions { save_path, serve })
}

fn main() -> Result<()> {
    println!("=== prisoma Demo with Rerun Visualization ===\n");

    let args = env::args().skip(1).collect::<Vec<_>>();
    let DemoOptions { save_path, serve } = parse_options(&args)?;

    // Initialize Rerun
    println!("Initializing Rerun...");
    let rec = init_recording("prisoma_demo", serve)?;

    // Create loggers
    let mut pid_logger = PidLogger::new(&rec);
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

    let vision_embeddings = episode.vision_embeddings()?;
    let actions = episode.actions()?;
    let timestamps = episode.timestamps();

    let mut window_start = 0;
    let mut pid_windows_produced = 0usize;
    let mut pid_windows_abstained = 0usize;
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

        let geometry = emit_geometry_window(
            &mut pid_logger,
            timestamp,
            estimate_geometry(&v_flat, n, vision_dim),
        )?;

        // Compute PID2 using a subset of dimensions (for speed)
        let v_source_1: Vec<f64> = v_window.slice(s![.., 0..2]).iter().cloned().collect();
        let v_source_2: Vec<f64> = v_window.slice(s![.., 2..4]).iter().cloned().collect();
        let a_subset: Vec<f64> = a_window.slice(s![.., 0..1]).iter().cloned().collect();

        // pid-core 1.0 fails closed on an unspecified support contract; this demo runs on
        // synthetic continuous data, so the full-dimensional assertion holds by construction.
        let ksg_config = KsgConfig::assume_regular_full_dimensional()
            .with_negative_handling(NegativeHandling::Allow);
        let pid_config = Pid2Config {
            ksg: ksg_config,
            isx: IsxConfig::assume_regular_full_dimensional(),
        };
        // NB: this synthetic demo decomposes two VISION subspaces (dims 0..2
        // vs dims 2..4), so `unique_s2` is a second-vision-subspace term, not a
        // language term. The source-agnostic logger records both vision labels
        // as provenance and uses fixed unique_source_1/unique_source_2 entities.
        let produced = emit_pid_window(
            &mut pid_logger,
            timestamp,
            estimate_pid_atoms(&v_source_1, &v_source_2, &a_subset, n, 2, 1, &pid_config),
        )?;
        if produced {
            pid_windows_produced += 1;
        } else {
            pid_windows_abstained += 1;
        }

        // Log events at interesting points
        if let Some(diagnostics) = geometry.filter(|diagnostics| diagnostics.intrinsic_dim > 20.0) {
            pid_logger.log_event(
                timestamp,
                "WARN",
                &format!(
                    "High intrinsic dimension detected: {:.1} (threshold: 20)",
                    diagnostics.intrinsic_dim
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
    println!("The recording includes:");
    println!("  - PID windows: {pid_windows_produced} produced, {pid_windows_abstained} abstained");
    println!("  - Geometry diagnostics (pid/geometry/*)");
    println!("  - VLA action trajectories (vla/action/*)");
    println!("  - Object positions (world/objects)");
    println!("  - Flow predictions (flow/*)");
    println!("  - Ghost splat overlay (world/ghost)");

    // Save if requested
    if let Some(path) = &save_path {
        println!("\nSaving recording to: {}", path);
        save_recording(&rec, path)?;
        println!("Recording saved successfully!");
    } else if serve {
        // Keep running so viewer stays connected (only in interactive mode)
        println!("\nPress Ctrl+C to exit...");
        std::thread::sleep(std::time::Duration::from_secs(3600));
    } else {
        println!("\nNo --save or --serve selected; recording was buffered and discarded.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        emit_geometry_window, emit_pid_window, parse_options, DemoOptions, DemoWindowSink,
        GeometryDiagnostics, PidAtoms,
    };
    use anyhow::{anyhow, Result};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn demo_modes_are_explicit() {
        assert_eq!(
            parse_options(&[]).unwrap(),
            DemoOptions {
                save_path: None,
                serve: false,
            }
        );
        assert_eq!(
            parse_options(&args(&["--serve"])).unwrap(),
            DemoOptions {
                save_path: None,
                serve: true,
            }
        );
        assert_eq!(
            parse_options(&args(&["--save", "demo.rrd"])).unwrap(),
            DemoOptions {
                save_path: Some("demo.rrd".to_owned()),
                serve: false,
            }
        );
    }

    #[test]
    fn demo_rejects_ambiguous_or_unknown_modes() {
        assert!(parse_options(&args(&["--save"])).is_err());
        assert!(parse_options(&args(&["--save", "demo.rrd", "--serve"])).is_err());
        assert!(parse_options(&args(&["--unknown"])).is_err());
    }

    #[test]
    fn demo_rejects_option_as_save_path() {
        assert!(parse_options(&args(&["--save", "--serve"])).is_err());
    }

    #[test]
    fn demo_rejects_duplicate_save_mode() {
        assert!(parse_options(&args(&["--save", "one.rrd", "--save", "two.rrd"])).is_err());
    }

    #[test]
    fn demo_rejects_duplicate_serve_mode() {
        assert!(parse_options(&args(&["--serve", "--serve"])).is_err());
    }

    #[derive(Debug, Default, PartialEq, Eq)]
    struct CountingSink {
        atom_metric_events: usize,
        geometry_metric_events: usize,
        diagnostic_events: usize,
        source_labels: Option<[String; 2]>,
    }

    impl DemoWindowSink for CountingSink {
        fn atom_metrics(
            &mut self,
            _timestamp: f64,
            _atoms: PidAtoms,
            source_labels: [&str; 2],
        ) -> Result<()> {
            self.atom_metric_events += 1;
            self.source_labels = Some(source_labels.map(str::to_owned));
            Ok(())
        }

        fn geometry_metrics(
            &mut self,
            _timestamp: f64,
            _diagnostics: GeometryDiagnostics,
        ) -> Result<()> {
            self.geometry_metric_events += 1;
            Ok(())
        }

        fn diagnostic(&mut self, _timestamp: f64, _message: &str) -> Result<()> {
            self.diagnostic_events += 1;
            Ok(())
        }
    }

    #[test]
    fn abstained_pid_window_emits_diagnostic_without_atom_metrics() -> Result<()> {
        let mut sink = CountingSink::default();

        emit_pid_window(&mut sink, 1.0, Err(anyhow!("forced abstention")))?;

        assert_eq!(
            sink,
            CountingSink {
                atom_metric_events: 0,
                geometry_metric_events: 0,
                diagnostic_events: 1,
                source_labels: None,
            }
        );
        Ok(())
    }

    #[test]
    fn unavailable_geometry_emits_diagnostic_without_numeric_metrics() -> Result<()> {
        let mut sink = CountingSink::default();

        emit_geometry_window(&mut sink, 1.0, Err(anyhow!("forced abstention")))?;

        assert_eq!(
            sink,
            CountingSink {
                atom_metric_events: 0,
                geometry_metric_events: 0,
                diagnostic_events: 1,
                source_labels: None,
            }
        );
        Ok(())
    }

    #[test]
    fn produced_pid_window_uses_two_vision_source_labels() -> Result<()> {
        let mut sink = CountingSink::default();
        let atoms = PidAtoms {
            redundancy: 0.1,
            synergy: 0.2,
            unique_v1: 0.3,
            unique_v2: 0.4,
        };

        emit_pid_window(&mut sink, 1.0, Ok(atoms))?;

        assert_eq!(
            sink.source_labels,
            Some([
                "vision_embedding_dims_0_1".to_owned(),
                "vision_embedding_dims_2_3".to_owned(),
            ])
        );
        Ok(())
    }
}
