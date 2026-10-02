//! Not the game's: the port's own messages, through `tracing`. They go to
//! stderr, filtered by `PINEY_LOG` (an `EnvFilter`: `debug`,
//! `piney_world=trace,warn`, ...); by default the port's crates at `info` and
//! everything else (wgpu, naga) at `warn`. Warnings and errors are also kept
//! for the console, which shows them as it would an answer.

use std::fmt::Write as _;
use std::sync::Mutex;

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// The filter when `PINEY_LOG` is unset or unreadable.
const DEFAULT: &str = "warn,piney=info";

/// Warnings and errors not yet shown in the console.
static FOR_CONSOLE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Installs the subscriber (once; later calls do nothing). `log` records
/// (wgpu's) come through it as well.
pub fn init() {
    let filter = EnvFilter::try_from_env("PINEY_LOG").unwrap_or_else(|_| EnvFilter::new(DEFAULT));
    let ansi = std::io::IsTerminal::is_terminal(&std::io::stderr());
    let stderr = tracing_subscriber::fmt::layer().with_writer(std::io::stderr).with_ansi(ansi).compact();
    let _ = tracing_subscriber::registry().with(filter).with(stderr).with(ToConsole).try_init();
}

/// The warnings and errors since the last call, as one line each.
pub fn take_for_console() -> Vec<String> {
    FOR_CONSOLE.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
}

/// Keeps each warning and error for [`take_for_console`].
struct ToConsole;

impl<S: Subscriber> Layer<S> for ToConsole {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let level = *event.metadata().level();
        if level > Level::WARN {
            return;
        }
        let mut line = Line(format!("{level}: "));
        event.record(&mut line);
        if let Ok(mut v) = FOR_CONSOLE.lock() {
            v.push(line.0);
        }
    }
}

/// An event's message, then its other fields as `name=value`.
struct Line(String);

impl Visit for Line {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.0, "{value:?}");
        } else {
            let _ = write!(self.0, " {}={value:?}", field.name());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Warnings and errors reach the console with their level and fields;
    /// info and below do not.
    #[test]
    fn warnings_reach_the_console() {
        let sub = tracing_subscriber::registry().with(ToConsole);
        tracing::subscriber::with_default(sub, || {
            take_for_console();
            tracing::info!("not this");
            tracing::warn!(area = 31, "the area's map: {}", "gone");
            tracing::error!("no GPU");
        });
        assert_eq!(take_for_console(), ["WARN: the area's map: gone area=31", "ERROR: no GPU"]);
    }
}
