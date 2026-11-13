//! Logging wrapper around `log` and `log4rs`.
use crate::datapipe_types::{DatapipeError, error_root_cause};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use log4rs::{self, Handle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

/// Max size of log file before rolling to next one (20 MB).
const TRIGGER_FILE_SIZE: u64 = 20 * 1024 * 1024; // 20 MB

/// Max number of archive log files to keep.
const LOG_FILE_COUNT: u32 = 1;

/// Log entry pattern.
const LOG_ENTRY_PATTERN: &str = "{d} [{l}] {f}:{L} {m}\n";

/// Log level to use.
const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

/// Logger handle returned to the application, cleaning up temporary logs on drop unless `keep_logs` is true.
pub struct Logger {
    /// Directory where log files are written.
    pub log_directory: PathBuf,
    /// Active log file path.
    pub log_path: PathBuf,
    /// Prefix of rolled archive log files.
    pub archive_prefix: String,
    /// Underlying `log4rs` handle used to update logger settings dynamically.
    pub handle: Handle,
    /// Whether log files should be retained when this `Logger` is dropped.
    pub keep_logs: bool,
}

impl std::fmt::Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Logger")
            .field("log_directory", &self.log_directory)
            .field("log_path", &self.log_path)
            .field("archive_prefix", &self.archive_prefix)
            .field("keep_logs", &self.keep_logs)
            .finish_non_exhaustive()
    }
}

// Deleting log files on exit is handled by a custom drop implementation.
impl Drop for Logger {
    fn drop(&mut self) {
        if !self.keep_logs {
            silence(&mut self.handle);
            if let Ok(entries) = fs::read_dir(&self.log_directory) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    // Path::starts_with compares path components rather than filename prefixes,
                    // so compare via to_str() when present.
                    if entry_path.is_file()
                        && (entry_path == self.log_path
                            || entry_path
                                .to_str()
                                .is_some_and(|s| s.starts_with(&self.archive_prefix)))
                    {
                        let _ = fs::remove_file(entry_path);
                    }
                }
            }
        }
    }
}

/// Silence the logger by switching the configuration to an `Off` threshold filter.
pub fn silence(handle: &mut Handle) {
    if let Ok(silent_config) = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(log::LevelFilter::Off)))
                .build("off", Box::new(ConsoleAppender::builder().build())),
        )
        .build(Root::builder().appender("off").build(LevelFilter::Off))
    {
        handle.set_config(silent_config);
    }
}

/// Initialize the rolling file logger inside `log_directory`.
///
/// # Errors
///
/// Returns [`DatapipeError::ConfigurationError`] or [`DatapipeError::InputOutputError`]
/// if the paths are invalid UTF-8, the log appender cannot be created, or logger initialization fails.
pub fn init_logger(
    log_directory: &str,
    log_basename: &str,
    keep_logs: bool,
) -> Result<Logger, DatapipeError> {
    // Use the process ID in the log filename to avoid conflicts across concurrent instances.
    let pid = process::id();
    let log_filename = format!("{}_PID{}.log", log_basename, pid);
    let archive_filename = format!("{}_PID{}_prev{{}}.log", log_basename, pid);

    let log_path = Path::new(log_directory).join(log_filename);
    let archive_path = Path::new(log_directory).join(archive_filename);
    let archive_path_str = archive_path.to_str().ok_or_else(|| {
        DatapipeError::ConfigurationError("Log archive path is not valid UTF-8".to_string())
    })?;
    let archive_prefix = archive_path_str
        .strip_suffix("{}.log")
        .ok_or_else(|| {
            DatapipeError::ConfigurationError("Invalid log archive pattern".to_string())
        })?
        .to_owned();

    // Create rolling file size policy for logger.
    let trigger = SizeTrigger::new(TRIGGER_FILE_SIZE);
    let roller = FixedWindowRoller::builder()
        .build(archive_path_str, LOG_FILE_COUNT)
        .map_err(|err| {
            DatapipeError::ConfigurationError(format!("Failed to configure log file roller: {err}"))
        })?;
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    // File appender.
    let logfile = log4rs::append::rolling_file::RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_ENTRY_PATTERN)))
        .build(&log_path, Box::new(policy))
        .map_err(|err| {
            DatapipeError::InputOutputError(format!(
                "Failed to open log file {:?}: {}",
                log_path,
                error_root_cause(&err)
            ))
        })?;

    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LOG_LEVEL)))
                .build("logfile", Box::new(logfile)),
        )
        .build(
            Root::builder()
                .appender("logfile")
                .build(LevelFilter::Trace),
        )
        .map_err(|err| {
            DatapipeError::ConfigurationError(format!("Failed to build log4rs config: {err}"))
        })?;

    let handle = log4rs::init_config(config).map_err(|err| {
        DatapipeError::ConfigurationError(format!("Failed to install logger: {err}"))
    })?;
    Ok(Logger {
        log_directory: Path::new(log_directory).to_path_buf(),
        log_path,
        archive_prefix,
        handle,
        keep_logs,
    })
}

/// Initialize a console-only logger (useful for unit tests).
///
/// # Errors
///
/// Returns [`DatapipeError::ConfigurationError`] if the configuration or global logger setup fails.
pub fn init_console_logger() -> Result<Handle, DatapipeError> {
    let console_config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(log::LevelFilter::Info)))
                .build(
                    "console",
                    Box::new(
                        ConsoleAppender::builder()
                            .encoder(Box::new(PatternEncoder::new(LOG_ENTRY_PATTERN)))
                            .build(),
                    ),
                ),
        )
        .build(
            Root::builder()
                .appender("console")
                .build(LevelFilter::Trace),
        )
        .map_err(|err| {
            DatapipeError::ConfigurationError(format!(
                "Failed to build console logger config: {err}"
            ))
        })?;
    log4rs::init_config(console_config).map_err(|err| {
        DatapipeError::ConfigurationError(format!("Failed to install console logger: {err}"))
    })
}
