use anyhow::{bail, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() > 3 || args.len() == 2 || args.get(1).is_some_and(|arg| arg != "--out") {
        bail!("usage: {} [--out contract.json]", args[0]);
    }

    let contract = pid_bridge::bridge_runlog_contract();
    if let Some(path) = args.get(2).map(PathBuf::from) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        pid_runlog::write_json_file(&path, &contract)?;
        println!("wrote {}", path.display());
    } else {
        serde_json::to_writer_pretty(std::io::stdout(), &contract)?;
        println!();
    }
    Ok(())
}
