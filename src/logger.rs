/// wrapper for logging implementation
/// use the standard log crate interface and wrap the log4rs for the implementation

use log::{LevelFilter, SetLoggerError};
use log4rs::{self, Handle};
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::filter::threshold::ThresholdFilter;
use std::path::{Path, PathBuf};
use std::process;

/// max size of log file before rolling to next one
const TRIGGER_FILE_SIZE: u64 = 20 * 1024 * 1024; // 2 MB

/// max number of archive log files to keep
const LOG_FILE_COUNT: u32 = 1;

/// log entry pattern
const LOG_ENTRY_PATTERN: &str = "{d(%Y-%m-%dT%H:%M:%S)} [{l}] {f}:{L} {m}\n";

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
}

pub fn init_logger(    
    log_directory: &str,    
    log_basename: &str,    
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
    })
}
