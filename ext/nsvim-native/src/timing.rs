//! Startup timing for performance work: `EDITOR_TIMING=1 nsvim .` prints how long each step
//! of startup took, measured from the start of `main`.

use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static ON: OnceLock<bool> = OnceLock::new();

/// Starts the clock. Call first thing in `main`.
pub fn start() {
    START.get_or_init(Instant::now);
    ON.get_or_init(|| std::env::var_os("EDITOR_TIMING").is_some());
}

/// Prints `label` with the milliseconds since `start`, when `EDITOR_TIMING` is set.
pub fn mark(label: &str) {
    if *ON.get().unwrap_or(&false) {
        let ms = START.get().map_or(0.0, |s| s.elapsed().as_secs_f64() * 1000.0);
        eprintln!("timing: {ms:8.2} ms  {label}");
    }
}
