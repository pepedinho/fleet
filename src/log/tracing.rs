//! Optional structured logging (`tracing`) support.
//!
//! Enabled with the `debug-logs` feature. The subscriber writes to stderr on
//! purpose: the dashboard (`fleet stats`) owns stdout, and stderr keeps logs
//! interleaved safely with it.

#[cfg(feature = "debug-logs")]
pub fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::from_default_env();
    // Failure to install a subscriber (e.g. already initialized) is not fatal.
    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

/// No-op build: `tracing` events are dropped unless the `debug-logs` feature
/// brings in a subscriber.
#[cfg(not(feature = "debug-logs"))]
pub fn init_tracing() {}
