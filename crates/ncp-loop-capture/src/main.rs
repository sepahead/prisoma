//! `prisoma-ncp-loop-capture`: capture a CREBAIN NCP 2 loop run, or check a journal.
//!
//! prisoma-ncp-loop-capture import RUN_DIR JOURNAL [--max-bytes N]
//! prisoma-ncp-loop-capture verify JOURNAL

use prisoma_ncp_loop_capture::{import, verify_journal, MAX_CAPTURE_BYTES};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let outcome = match refs.as_slice() {
        ["import", run, journal] => import(Path::new(run), Path::new(journal), MAX_CAPTURE_BYTES),
        ["import", run, journal, "--max-bytes", bytes] => match bytes.parse() {
            Ok(limit) => import(Path::new(run), Path::new(journal), limit),
            Err(_) => Err(prisoma_ncp_loop_capture::CaptureError(
                "--max-bytes takes an integer".into(),
            )),
        },
        ["verify", journal] => verify_journal(Path::new(journal)),
        _ => {
            eprintln!("usage: prisoma-ncp-loop-capture import RUN_DIR JOURNAL [--max-bytes N] | verify JOURNAL");
            return ExitCode::from(2);
        }
    };
    match outcome.and_then(|report| {
        serde_json::to_string(&report)
            .map_err(|e| prisoma_ncp_loop_capture::CaptureError(e.to_string()))
    }) {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("prisoma-ncp-loop-capture: {error}");
            ExitCode::FAILURE
        }
    }
}
