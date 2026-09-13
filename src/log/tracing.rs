//! Structured logging (`tracing`) support.
//!
//! A subscriber is always installed at boot. Events carrying a `log_path`
//! field are appended to the matching per-watch log file by
//! [`super::file_layer::FileLayer`], while the same events are mirrored to
//! stderr when `RUST_LOG` allows (default: warnings and errors only).
//!
//! stderr is used on purpose: the dashboard (`fleet stats`) owns stdout, and
//! stderr keeps logs interleaved safely with it.

use std::sync::OnceLock;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, Layer, layer::SubscriberExt, registry, util::SubscriberInitExt,
};

use super::file_layer::FileLayer;

static INIT: OnceLock<()> = OnceLock::new();

/// Install the global subscriber exactly once. Safe to call repeatedly; a
/// second call is a no-op (whether or not another subscriber was registered).
pub fn init_tracing() {
    INIT.get_or_init(|| {
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::WARN.into())
            .from_env_lossy();

        // `FLEET_NO_COLOR` applies to the per-watch log files (honored by
        // `FileLayer`) but must also keep the stderr mirror free of ANSI
        // codes, in particular when stderr is piped.
        let ansi = std::env::var("FLEET_NO_COLOR").is_err();

        let _ = registry()
            .with(FileLayer::new())
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stderr)
                    .with_ansi(ansi)
                    .with_filter(filter),
            )
            .try_init()
            // A subscriber installed before us (embedding harness) silently
            // drops the file layer: say so instead of losing per-watch logs.
            .map_err(|e| eprintln!("fleet.tracing: failed to install subscriber: {e}"));
    });
}
