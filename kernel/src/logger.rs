/*
 * A simple logger implementation that writes log messages to the serial port (COM1).
 * The log messages include a timestamp, log level, source file name, and line number.
 *
 * Author: Fabian Ruhland, Heinrich Heine University Duesseldorf, 2026-01-07
 * License: GPLv3
 */

use crate::print;
use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};
use log::{Metadata, Record};
use crate::device::serial;
use crate::println;

//use crate::println;
//use crate::println;
/// A simple logger implementing the `log::Log` trait, writing to the serial port (COM1) and optionally the terminal.
pub struct Logger {
    log_to_terminal: AtomicBool,
}

impl Logger {
    /// Create a new logger.
    pub const fn new() -> Logger {
        Logger {
            log_to_terminal: AtomicBool::new(false),
        }
    }
    pub fn enable_terminal_logging(&self, enabled: bool) {
        self.log_to_terminal.store(enabled, Ordering::Relaxed);
    }
}

impl log::Log for Logger {
    /// Check if the logger is enabled for the given metadata.
    /// This simple implementation always returns true.
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_str = level_abbreviation(record.level());
            let file = record.file().unwrap_or("unknown");
            let file_name = file.rsplit('/').next().unwrap_or(file);
            let line = record.line().unwrap_or(0);
            let _ = writeln!(
                serial::COM1.lock(),
                "[0.000][{}][{}@{}] {}",
                level_str,
                file_name,
                line,
                record.args()
            );

            if self.log_to_terminal.load(Ordering::Relaxed) {
                println!("[0.000][{}][{}@{}] {}", level_str, file_name, line, record.args());
            }
        }
    }

    /// Flush the logger.
    /// Since all messages are written immediately, this is a no-op.
    fn flush(&self) {}
}

/// Convert a log level abbreviation to a `log::Level`.
/// Supported abbreviations are:
/// - "TRC" -> Trace
/// - "DBG" -> Debug
/// - "INF" -> Info
/// - "WRN" -> Warn
/// - "ERR" -> Error
/// Returns `None` for unrecognized abbreviations.
pub fn level_from_abbreviation(abbr: &str) -> Option<log::Level> {
    match abbr {
        "TRC" | "trc" => Some(log::Level::Trace),
        "DBG" | "dbg" => Some(log::Level::Debug),
        "INF" | "inf" => Some(log::Level::Info),
        "WRN" | "wrn" => Some(log::Level::Warn),
        "ERR" | "err" => Some(log::Level::Error),
        _ => None,
    }
}

/// Get the three-letter abbreviation for a given log level.
fn level_abbreviation(level: log::Level) -> &'static str {
    match level {
        log::Level::Trace => "TRC",
        log::Level::Debug => "DBG",
        log::Level::Info => "INF",
        log::Level::Warn => "WRN",
        log::Level::Error => "ERR",
    }
}