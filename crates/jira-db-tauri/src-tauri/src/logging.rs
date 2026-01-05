//! Logging wrapper module
//!
//! This module provides a centralized logging interface that can be easily
//! modified to change the output destination (console, GUI, file, etc.).

use std::sync::OnceLock;

/// Log level enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Log output destination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOutput {
    /// Output to console via tracing
    Console,
    /// Output to both console and GUI (future)
    #[allow(dead_code)]
    ConsoleAndGui,
}

static LOG_OUTPUT: OnceLock<LogOutput> = OnceLock::new();

/// Initialize logging with the specified output destination
pub fn init(output: LogOutput) {
    let _ = LOG_OUTPUT.set(output);
}

/// Get the current log output destination
fn get_output() -> LogOutput {
    *LOG_OUTPUT.get().unwrap_or(&LogOutput::Console)
}

/// Internal log function
fn log_internal(level: LogLevel, target: &str, message: &str) {
    let output = get_output();

    match output {
        LogOutput::Console | LogOutput::ConsoleAndGui => {
            // Output to console via tracing
            match level {
                LogLevel::Debug => tracing::debug!(target: target, "{}", message),
                LogLevel::Info => tracing::info!(target: target, "{}", message),
                LogLevel::Warn => tracing::warn!(target: target, "{}", message),
                LogLevel::Error => tracing::error!(target: target, "{}", message),
            }
        }
    }

    // Future: Add GUI event emission here
    // if matches!(output, LogOutput::ConsoleAndGui) {
    //     emit_to_gui(level, target, message);
    // }
}

/// Logger for a specific target/context
pub struct Logger {
    target: String,
}

impl Logger {
    /// Create a new logger for the specified target
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
        }
    }

    /// Log a debug message
    pub fn debug(&self, message: &str) {
        log_internal(LogLevel::Debug, &self.target, message);
    }

    /// Log an info message
    pub fn info(&self, message: &str) {
        log_internal(LogLevel::Info, &self.target, message);
    }

    /// Log a warning message
    pub fn warn(&self, message: &str) {
        log_internal(LogLevel::Warn, &self.target, message);
    }

    /// Log an error message
    pub fn error(&self, message: &str) {
        log_internal(LogLevel::Error, &self.target, message);
    }

    /// Log a debug message with formatting
    pub fn debug_fmt(&self, args: std::fmt::Arguments<'_>) {
        log_internal(LogLevel::Debug, &self.target, &args.to_string());
    }

    /// Log an info message with formatting
    pub fn info_fmt(&self, args: std::fmt::Arguments<'_>) {
        log_internal(LogLevel::Info, &self.target, &args.to_string());
    }

    /// Log a warning message with formatting
    pub fn warn_fmt(&self, args: std::fmt::Arguments<'_>) {
        log_internal(LogLevel::Warn, &self.target, &args.to_string());
    }

    /// Log an error message with formatting
    pub fn error_fmt(&self, args: std::fmt::Arguments<'_>) {
        log_internal(LogLevel::Error, &self.target, &args.to_string());
    }
}

/// Convenience macros for logging

#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $($arg:tt)*) => {
        $logger.debug_fmt(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $($arg:tt)*) => {
        $logger.info_fmt(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($logger:expr, $($arg:tt)*) => {
        $logger.warn_fmt(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($logger:expr, $($arg:tt)*) => {
        $logger.error_fmt(format_args!($($arg)*))
    };
}

/// Create a logger for a specific target
#[macro_export]
macro_rules! logger {
    ($target:expr) => {
        $crate::logging::Logger::new($target)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger() {
        init(LogOutput::Console);
        let logger = Logger::new("test");
        logger.info("Test message");
    }
}
