//! PID-VLA Viewer: Simple Rerun visualization launcher.
//!
//! Usage:
//!   cargo run -p pid-rerun --bin pid-viewer
//!   cargo run -p pid-rerun --bin pid-viewer -- --load recording.rrd

use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 2 && args[1] == "--load" {
        // Load an existing recording
        let path = &args[2];
        println!("Loading recording from: {}", path);
        // Rerun CLI would handle this - just spawn viewer
        std::process::Command::new("rerun")
            .arg(path)
            .spawn()?
            .wait()?;
    } else {
        // Just spawn the viewer
        println!("Spawning Rerun viewer...");
        println!("Use the vla-demo binary to send data to the viewer.");
        std::process::Command::new("rerun").spawn()?.wait()?;
    }

    Ok(())
}
