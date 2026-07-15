//! prisoma Viewer: Simple Rerun visualization launcher.
//!
//! Usage:
//!   cargo run -p pid-rerun --bin pid-viewer
//!   cargo run -p pid-rerun --bin pid-viewer -- --load recording.rrd

use anyhow::{bail, Result};
use pid_rerun::require_matching_viewer_version;
use std::env;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
enum ViewerOptions {
    Launch,
    Load(String),
}

fn parse_options(args: &[String]) -> Result<ViewerOptions> {
    match args {
        [] => Ok(ViewerOptions::Launch),
        [flag, path] if flag == "--load" && !path.is_empty() && !path.starts_with('-') => {
            Ok(ViewerOptions::Load(path.clone()))
        }
        [flag] if flag == "--load" => bail!("--load requires a recording path"),
        _ => bail!("usage: pid-viewer [--load <recording.rrd>]"),
    }
}

fn run_viewer(path: Option<&str>) -> Result<()> {
    let mut command = Command::new("rerun");
    if let Some(path) = path {
        command.arg(path);
    }
    let status = command.status()?;
    if !status.success() {
        bail!("Rerun viewer exited with status {status}");
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let options = parse_options(&args)?;
    require_matching_viewer_version()?;

    match options {
        ViewerOptions::Load(path) => {
            println!("Loading recording from: {path}");
            run_viewer(Some(&path))?;
        }
        ViewerOptions::Launch => {
            println!("Spawning Rerun viewer...");
            println!("Use the vla-demo binary to send data to the viewer.");
            run_viewer(None)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_options, ViewerOptions};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn viewer_modes_are_explicit() {
        assert_eq!(parse_options(&[]).unwrap(), ViewerOptions::Launch);
        assert_eq!(
            parse_options(&args(&["--load", "recording.rrd"])).unwrap(),
            ViewerOptions::Load("recording.rrd".to_owned())
        );
    }

    #[test]
    fn viewer_rejects_malformed_arguments() {
        assert!(parse_options(&args(&["--load"])).is_err());
        assert!(parse_options(&args(&["--load", ""])).is_err());
        assert!(parse_options(&args(&["--load", "--unknown"])).is_err());
        assert!(parse_options(&args(&["--unknown"])).is_err());
        assert!(parse_options(&args(&["recording.rrd"])).is_err());
        assert!(parse_options(&args(&["--load", "a.rrd", "extra"])).is_err());
    }
}
