use ncp_local::local::{
    local_profile_digest, serve_local, LocalBinding, LocalCode, LocalError, LocalOwner, LocalRole,
};
use prisoma_ncp_local_capture::{verify_journal, CaptureBackend};
use std::io;
use std::path::Path;

fn run() -> Result<(), LocalError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "--verify" {
        let report = verify_journal(Path::new(&args[1]))?;
        println!(
            "{}",
            serde_json::to_string(&report).map_err(|_| LocalError(LocalCode::Wire))?
        );
        return Ok(());
    }
    if args.len() != 6
        || args[0] != "--run-id"
        || args[2] != "--generation"
        || args[4] != "--output-file"
        || args.iter().any(|value| value.len() > 4096)
    {
        return Err(LocalError(LocalCode::Binding));
    }
    let binding = LocalBinding {
        profile_digest: local_profile_digest()?,
        run_id: args[1].clone(),
        generation: args[3].clone(),
        role: LocalRole::Capture,
    };
    let backend = CaptureBackend::new(binding.clone(), Path::new(&args[5]))?;
    let mut owner = LocalOwner::new(binding, backend)?;
    serve_local(
        &mut owner,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
    )
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
