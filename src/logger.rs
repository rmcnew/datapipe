/// wrapper for logging implementation
/// use the standard log crate interface and wrap the log4rs for the implementation
use log::{LevelFilter, SetLoggerError};
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

/// max size of log file before rolling to next one
const TRIGGER_FILE_SIZE: u64 = 20 * 1024 * 1024; // 2 MB

/// max number of archive log files to keep
const LOG_FILE_COUNT: u32 = 1;

/// log entry pattern
const LOG_ENTRY_PATTERN: &str = "{d} [{l}] {f}:{L} {m}\n";

/// log level to use
const LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;

/// this logger struct is returned to the application
pub struct Logger {
    /// where the log files will be written
    pub log_directory: PathBuf,
    /// the prefix of the log files
    pub log_path: PathBuf,
    /// the prefix of the archive log files
    pub archive_prefix: String,
    /// the logger handle can be used to change the logger settings on the fly if wanted
    pub handle: Handle,
    /// should logs be kept when the program exits?
    pub keep_logs: bool,
}

// deleting log files on exit is handled by a custom drop function
impl Drop for Logger {
    fn drop(&mut self) {
        if !self.keep_logs {
            silence(&mut self.handle);
            // delete log files
            for maybe_entry in fs::read_dir(&self.log_directory).unwrap() {
                let entry = maybe_entry.unwrap();
                let entry_path = entry.path();
                // note that Path.starts_with treats filenames with extensions differently from str.starts_with
                // Path.starts_with returns false unless the Path's filename matches, so we use the str.starts_with
                // which gives the desired prefix matching behavior
                if entry_path.is_file()
                    && (entry_path == self.log_path
                        || entry_path
                            .to_str()
                            .unwrap()
                            .starts_with(&self.archive_prefix))
                {
                    fs::remove_file(entry_path).unwrap();
                }
            }
        }
    }
}

// Silence the logger by setting logger config to console logging and maximum filtering (turn logging off)
pub fn silence(handle: &mut Handle) {
    let silent_config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(log::LevelFilter::Off)))
                .build("off", Box::new(ConsoleAppender::builder().build())),
        )
        .build(Root::builder().appender("off").build(LevelFilter::Off))
        .unwrap();
    handle.set_config(silent_config);
}

pub fn init_logger(
    log_directory: &str,
    log_basename: &str,
    keep_logs: bool,
) -> Result<Logger, SetLoggerError> {
    // Use the process ID as part of the log filename to avoid filename conflicts when running multiple instances
    let pid = process::id();
    let log_filename = format!("{}_PID{}.log", log_basename, pid);
    let archive_filename = format!("{}_PID{}_prev{{}}.log", log_basename, pid);

    let log_path = Path::new(log_directory).join(log_filename);
    let archive_path = Path::new(log_directory).join(archive_filename);
    let archive_prefix = archive_path
        .to_str()
        .unwrap()
        .strip_suffix("{}.log")
        .unwrap()
        .to_owned();

    // create rolling file size policy for logger
    let trigger = SizeTrigger::new(TRIGGER_FILE_SIZE);
    let roller = FixedWindowRoller::builder()
        .build(archive_path.to_str().unwrap(), LOG_FILE_COUNT)
        .unwrap();
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    // file appender
    let logfile = log4rs::append::rolling_file::RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_ENTRY_PATTERN)))
        .build(&log_path, Box::new(policy))
        .unwrap();

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
        .unwrap();

    let handle = log4rs::init_config(config)?;
    Ok(Logger {
        log_directory: Path::new(log_directory).to_path_buf(),
        log_path,
        archive_prefix,
        handle,
        keep_logs,
    })
}

// useful for unit tests
pub fn init_console_logger() -> Result<Handle, SetLoggerError> {
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
        .unwrap();
    let handle = log4rs::init_config(console_config)?;
    Ok(handle)
}
