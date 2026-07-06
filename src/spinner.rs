//! Thin wrapper around an [`indicatif::ProgressBar`] spinner.
//!
//! Usage:
//! ```ignore
//! let spin = Spinner::new("Loading history…");
//! spin.update("Calling AI…");
//! spin.success("Done (3.2s)");
//! ```

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Braille dot-cycle spinner with a purple tick.
const TICK_CHARS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Spinner {
    bar: ProgressBar,
}

impl Spinner {
    /// Creates a new spinner with `msg` as the initial status message.
    /// The spinner starts ticking immediately at 80 ms intervals.
    pub fn new(msg: impl Into<String>) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template("{spinner:.magenta} {msg}")
                .unwrap()
                .tick_strings(TICK_CHARS),
        );
        bar.set_message(msg.into());
        bar.enable_steady_tick(Duration::from_millis(80));
        Self { bar }
    }

    /// Updates the spinner message without stopping it.
    pub fn update(&self, msg: impl Into<String>) {
        self.bar.set_message(msg.into());
    }

    /// Stops the spinner and prints a ✅ success message.
    pub fn success(self, msg: impl Into<String>) {
        self.bar
            .finish_with_message(format!("\x1b[32m✓\x1b[0m  {}", msg.into()));
    }

    /// Stops the spinner and prints a ⚠ warning message.
    #[allow(dead_code)]
    pub fn warn(self, msg: impl Into<String>) {
        self.bar
            .finish_with_message(format!("\x1b[33m⚠\x1b[0m  {}", msg.into()));
    }

    /// Stops the spinner and prints a ✗ failure message.
    pub fn fail(self, msg: impl Into<String>) {
        self.bar
            .finish_with_message(format!("\x1b[31m✗\x1b[0m  {}", msg.into()));
    }
}
