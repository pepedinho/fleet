//! A [`Layer`] that appends every event carrying a `log_path` field to the
//! matching per-watch log file in `~/.fleet/logs/`.
//!
//! It reproduces the exact line format produced by the historical `Logger`
//! (`[YYYY-MM-DD HH:MM:SS] LEVEL: message`) so `fleet logs`, the `fleet stats`
//! dashboard and the file-based tests are unaffected.

use std::{fs::OpenOptions, io::Write, path::Path};

use chrono::Local;
use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{Layer, registry::LookupSpan};

const RESET: &str = "\x1b[0m";
const BG_BLUE: &str = "\x1b[44m"; // info
const BG_ORANGE: &str = "\x1b[48;5;208m"; // warning
const BG_RED: &str = "\x1b[41m";
const BG_GREEN: &str = "\x1b[42m"; // job start
const BG_MAGENTA: &str = "\x1b[45m"; // job end
const FG_BOLD_WHITE: &str = "\x1b[97;1m";

/// Fields a logging event carries for the per-watch line to be reconstructed.
#[derive(Debug, Default)]
struct LogFields {
    log_path: Option<String>,
    message: Option<String>,
    kind: Option<String>,
}

impl Visit for LogFields {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "log_path" => self.log_path = Some(value.to_string()),
            "message" => self.message = Some(value.to_string()),
            "kind" => self.kind = Some(value.to_string()),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let rendered = format!("{value:?}");
        match field.name() {
            "log_path" => self.log_path = Some(rendered),
            "message" => self.message = Some(rendered),
            "kind" => self.kind = Some(rendered),
            _ => {}
        }
    }
}

/// A [`Layer`] writing each `tracing` event to a per-watch log file.
#[derive(Debug)]
pub struct FileLayer {
    color_enable: bool,
}

impl FileLayer {
    pub fn new() -> Self {
        let no_color = std::env::var("FLEET_NO_COLOR").ok().as_deref() == Some("1");
        Self {
            color_enable: !no_color,
        }
    }

    /// Level badge for the line, honoring the custom `JOB START`/`JOB END`
    /// labels and the `FLEET_NO_COLOR` escape hatch.
    fn label(&self, event: &Event<'_>, fields: &LogFields) -> String {
        if let Some(kind @ ("JOB START" | "JOB END")) = fields.kind.as_deref() {
            return if self.color_enable {
                let bg = if kind == "JOB START" {
                    BG_GREEN
                } else {
                    BG_MAGENTA
                };
                format!("{bg}{FG_BOLD_WHITE} {kind} {RESET}")
            } else {
                kind.to_string()
            };
        }

        let level = match *event.metadata().level() {
            Level::ERROR => "ERROR",
            Level::WARN => "WARNING",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };

        if self.color_enable {
            let bg = match level {
                "INFO" => BG_BLUE,
                "WARNING" => BG_ORANGE,
                "ERROR" => BG_RED,
                _ => "",
            };
            if bg.is_empty() {
                level.to_string()
            } else {
                format!("{bg}{FG_BOLD_WHITE} {level} {RESET}")
            }
        } else {
            level.to_string()
        }
    }

    /// Appends a single line to the log file, creating it when missing.
    /// A fresh handle per write keeps the write loop immune to `logs rm`.
    fn append(path: &Path, line: &str) {
        let Ok(mut file) = OpenOptions::new().append(true).create(true).open(path) else {
            return;
        };
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();
    }
}

impl Default for FileLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for FileLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut fields = LogFields::default();
        event.record(&mut fields);

        let Some(log_path) = fields.log_path.as_deref().filter(|p| !p.is_empty()) else {
            return;
        };

        let now = Local::now();
        let line = format!(
            "[{}] {}: {}\n",
            now.format("%Y-%m-%d %H:%M:%S"),
            self.label(event, &fields),
            fields.message.unwrap_or_default()
        );
        Self::append(Path::new(&log_path), &line);
    }
}
