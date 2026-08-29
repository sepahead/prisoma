use std::io::{self, Write};

fn main() {
    std::panic::set_hook(Box::new(|_| {
        let _ = writeln!(
            io::stderr().lock(),
            "managed-observer: internal-panic-contained"
        );
    }));
    if let Err(error) = prisoma_engram_managed_observer::serve_managed_runtime(
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
    ) {
        let _ = writeln!(io::stderr().lock(), "managed-observer: {}", error.reason());
        std::process::exit(2);
    }
}
