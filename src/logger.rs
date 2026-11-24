//! Logging wrapper around the [`log`] facade using [`jiff`] for timestamps.

use crate::datapipe_types::{DatapipeError, error_root_cause};
use jiff::ZonedDateTime;
use jiff::fmt::strtime;
use log::{LevelFilter, Log, Metadata, Record};
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Max size of log file before rolling to next one (20 MB).
const TRIGGER_FILE_SIZE: u64 = 20 * 1024 * 1024; // 20 MB

/// Default log level threshold.
const LOG_LEVEL: LevelFilter = LevelFilter::Info;

/// Timestamp format matching ISO 8601 / RFC 3339 with nanosecond precision and local offset.
const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.9f%:z";

/// Fallback timestamp used only if system time formatting fails.
const FALLBACK_TIMESTAMP: &str = "1970-01-01T00:00:00.000000000+00:00";

fn level_to_u8(level: LevelFilter) -> u8 {
    match level {
        LevelFilter::Off => 0,
        LevelFilter::Error => 1,
        LevelFilter::Warn => 2,
        LevelFilter::Info => 3,
        LevelFilter::Debug => 4,
        LevelFilter::Trace => 5,
    }
}

fn u8_to_level(raw: u8) -> LevelFilter {
    match raw {
        0 => LevelFilter::Off,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

fn validate_log_basename(log_basename: &str) -> Result<(), DatapipeError> {
    if log_basename.is_empty()
        || log_basename == "."
        || log_basename == ".."
        || log_basename.contains(['/', '\\', '\0'])
    {
        return Err(DatapipeError::ValidationError(format!(
            "Invalid log basename {log_basename:?}: must be a non-empty filename without path separators or null bytes"
        )));
    }
    Ok(())
}

fn format_timestamp(system_time: SystemTime) -> String {
    let Ok(zdt) = ZonedDateTime::try_from(system_time) else {
        return FALLBACK_TIMESTAMP.to_string();
    };
    strtime::format(TIMESTAMP_FORMAT, &zdt).unwrap_or_else(|_| FALLBACK_TIMESTAMP.to_string())
}

fn format_log_entry(timestamp: &str, record: &Record<'_>) -> String {
    let file = record.file().unwrap_or("???");
    let mut entry = String::new();
    match record.line() {
        Some(line) => {
            let _ = writeln!(
                entry,
                "{timestamp} [{}] {file}:{line} {}",
                record.level(),
                record.args()
            );
        }
        None => {
            let _ = writeln!(
                entry,
                "{timestamp} [{}] {file}:??? {}",
                record.level(),
                record.args()
            );
        }
    }
    entry
}

/// Opens a regular log file safely without following symlinks and enforces owner-only permissions
/// on Unix. Returns the opened handle together with a flag indicating whether the file was newly
/// created.
fn open_log_file_secure(path: &Path, truncate: bool) -> io::Result<(File, bool)> {
    let mut create_opts = OpenOptions::new();
    create_opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        create_opts.mode(0o600);
    }
    match create_opts.open(path) {
        Ok(file) => return Ok((file, true)),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
        Err(err) => return Err(err),
    }

    let lstat_meta = fs::symlink_metadata(path)?;
    if lstat_meta.file_type().is_symlink() || !lstat_meta.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Refusing to open symlink or non-regular log path {path:?}"),
        ));
    }

    let mut open_opts = OpenOptions::new();
    open_opts.write(true);
    if !truncate {
        open_opts.append(true);
    }
    let file = open_opts.open(path)?;

    let opened_meta = file.metadata()?;
    if !opened_meta.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Refusing to use non-regular log file {path:?}"),
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if (opened_meta.dev(), opened_meta.ino()) != (lstat_meta.dev(), lstat_meta.ino()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Log path {path:?} changed during open"),
            ));
        }
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }

    if truncate {
        file.set_len(0)?;
    }

    Ok((file, false))
}

fn remove_regular_file_if_exists(path: &Path) {
    if let Ok(meta) = fs::symlink_metadata(path)
        && meta.file_type().is_file()
    {
        let _ = fs::remove_file(path);
    }
}

struct RollingFileSink {
    file: Option<File>,
    current_size: u64,
    log_path: PathBuf,
    archive_path: PathBuf,
    trigger_size: u64,
    write_error_reported: bool,
}

impl RollingFileSink {
    fn new(
        log_path: PathBuf,
        archive_path: PathBuf,
        trigger_size: u64,
    ) -> Result<(Self, bool), DatapipeError> {
        if trigger_size == 0 {
            return Err(DatapipeError::ConfigurationError(
                "Log trigger file size must be greater than zero".to_string(),
            ));
        }
        let (file, created_new) = open_log_file_secure(&log_path, false).map_err(|err| {
            DatapipeError::InputOutputError(format!(
                "Failed to open log file {:?}: {}",
                log_path,
                error_root_cause(&err)
            ))
        })?;
        let current_size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        Ok((
            Self {
                file: Some(file),
                current_size,
                log_path,
                archive_path,
                trigger_size,
                write_error_reported: false,
            },
            created_new,
        ))
    }

    fn ensure_open(&mut self) -> io::Result<&mut File> {
        if self.file.is_none() {
            let (file, _) = open_log_file_secure(&self.log_path, false)?;
            self.current_size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
            self.file = Some(file);
        }
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("Log file unavailable"))
    }

    fn write_entry(&mut self, entry: &str) -> io::Result<()> {
        let file = self.ensure_open()?;
        file.write_all(entry.as_bytes())?;
        self.current_size = self.current_size.saturating_add(entry.len() as u64);
        if self.current_size >= self.trigger_size
            && let Err(err) = self.rotate()
        {
            // Best-effort reopen so future writes can recover after transient rotation errors.
            if self.file.is_none()
                && let Ok((file, _)) = open_log_file_secure(&self.log_path, false)
            {
                self.current_size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
                self.file = Some(file);
            }
            return Err(err);
        }
        self.write_error_reported = false;
        Ok(())
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.close();

        let log_meta = fs::symlink_metadata(&self.log_path)?;
        if log_meta.file_type().is_symlink() || !log_meta.file_type().is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Refusing to rotate symlink or non-regular log path {:?}",
                    self.log_path
                ),
            ));
        }

        match fs::symlink_metadata(&self.archive_path) {
            Ok(meta) => {
                if meta.file_type().is_symlink() || !meta.file_type().is_file() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "Refusing to overwrite symlink or non-regular archive path {:?}",
                            self.archive_path
                        ),
                    ));
                }
                fs::remove_file(&self.archive_path)?;
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }

        fs::rename(&self.log_path, &self.archive_path)?;
        let (fresh_file, _) = open_log_file_secure(&self.log_path, true)?;
        self.file = Some(fresh_file);
        self.current_size = 0;
        Ok(())
    }

    fn flush(&mut self) {
        if let Some(ref mut file) = self.file {
            let _ = file.flush();
        }
    }

    fn close(&mut self) {
        if let Some(mut file) = self.file.take() {
            let _ = file.flush();
        }
    }
}

enum LogSink {
    RollingFile(RollingFileSink),
    Console,
}

struct LoggerCore {
    level: AtomicU8,
    update_global_max_level: bool,
    sink: Mutex<LogSink>,
}

struct DatapipeLogger {
    core: Arc<LoggerCore>,
}

impl Log for DatapipeLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        let active = u8_to_level(self.core.level.load(Ordering::Acquire));
        active != LevelFilter::Off && metadata.level() <= active
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let timestamp = format_timestamp(SystemTime::now());
        let entry = format_log_entry(&timestamp, record);

        let mut sink = self.core.sink.lock().unwrap_or_else(|err| err.into_inner());
        if !self.enabled(record.metadata()) {
            return;
        }
        match *sink {
            LogSink::RollingFile(ref mut rolling_sink) => {
                if let Err(err) = rolling_sink.write_entry(&entry)
                    && !rolling_sink.write_error_reported
                {
                    rolling_sink.write_error_reported = true;
                    eprintln!("datapipe logger write error: {err}");
                }
            }
            LogSink::Console => {
                let _ = io::stdout().write_all(entry.as_bytes());
            }
        }
    }

    fn flush(&self) {
        let mut sink = self.core.sink.lock().unwrap_or_else(|err| err.into_inner());
        match *sink {
            LogSink::RollingFile(ref mut rolling_sink) => rolling_sink.flush(),
            LogSink::Console => {
                let _ = io::stdout().flush();
            }
        }
    }
}

/// Dynamic handle for adjusting the active logger level or disabling log output.
///
/// # Examples
///
/// ```no_run
/// use datapipe::datapipe_types::DatapipeError;
/// use datapipe::logger::init_console_logger;
/// use log::LevelFilter;
///
/// fn main() -> Result<(), DatapipeError> {
///     let handle = init_console_logger()?;
///     handle.set_level(LevelFilter::Warn);
///     assert_eq!(handle.level(), LevelFilter::Warn);
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Handle {
    core: Arc<LoggerCore>,
}

impl Handle {
    /// Sets the active log filter threshold.
    ///
    /// Passing [`LevelFilter::Off`] disables further log writes and closes any active rolling log
    /// file descriptor; raising the level later reopens the log file automatically on the next
    /// logged message.
    pub fn set_level(&self, level: LevelFilter) {
        self.core.level.store(level_to_u8(level), Ordering::Release);
        if self.core.update_global_max_level {
            log::set_max_level(level);
        }
        if level == LevelFilter::Off {
            let mut sink = self.core.sink.lock().unwrap_or_else(|err| err.into_inner());
            if let LogSink::RollingFile(ref mut rolling_sink) = *sink {
                rolling_sink.close();
            }
        }
    }

    /// Returns the currently configured log filter threshold.
    #[must_use]
    pub fn level(&self) -> LevelFilter {
        u8_to_level(self.core.level.load(Ordering::Acquire))
    }
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle")
            .field("level", &self.level())
            .finish_non_exhaustive()
    }
}

/// Logger handle returned to the application, cleaning up temporary logs on drop unless `keep_logs` is true.
pub struct Logger {
    /// Directory where log files are written.
    pub log_directory: PathBuf,
    /// Active log file path.
    pub log_path: PathBuf,
    /// Rolled archive log file path.
    pub archive_path: PathBuf,
    /// Prefix of rolled archive log files.
    pub archive_prefix: String,
    /// Handle used to update logger settings dynamically.
    pub handle: Handle,
    /// Whether log files should be retained when this `Logger` is dropped.
    pub keep_logs: bool,
}

impl std::fmt::Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Logger")
            .field("log_directory", &self.log_directory)
            .field("log_path", &self.log_path)
            .field("archive_path", &self.archive_path)
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
            remove_regular_file_if_exists(&self.log_path);
            remove_regular_file_if_exists(&self.archive_path);
        }
    }
}

/// Silence the logger by switching the configuration to [`LevelFilter::Off`].
///
/// # Examples
///
/// ```no_run
/// use datapipe::datapipe_types::DatapipeError;
/// use datapipe::logger::{init_logger, silence};
/// use log::LevelFilter;
///
/// fn main() -> Result<(), DatapipeError> {
///     let mut logger = init_logger("/var/tmp", "datapipe", false)?;
///     silence(&mut logger.handle);
///     assert_eq!(logger.handle.level(), LevelFilter::Off);
///     Ok(())
/// }
/// ```
pub fn silence(handle: &mut Handle) {
    handle.set_level(LevelFilter::Off);
}

/// Initialize the rolling file logger inside `log_directory`.
///
/// # Examples
///
/// ```no_run
/// use datapipe::datapipe_types::DatapipeError;
/// use datapipe::logger::init_logger;
///
/// fn main() -> Result<(), DatapipeError> {
///     let _logger = init_logger("/var/tmp", "datapipe", false)?;
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns [`DatapipeError::ValidationError`] if `log_basename` is empty or contains path
/// separators, or [`DatapipeError::ConfigurationError`] / [`DatapipeError::InputOutputError`] if
/// the paths are invalid UTF-8, the log file cannot be opened safely, or logger installation fails.
pub fn init_logger(
    log_directory: &str,
    log_basename: &str,
    keep_logs: bool,
) -> Result<Logger, DatapipeError> {
    validate_log_basename(log_basename)?;

    // Use the process ID in the log filename to avoid conflicts across concurrent instances.
    let pid = process::id();
    let log_filename = format!("{log_basename}_PID{pid}.log");
    let archive_filename = format!("{log_basename}_PID{pid}_prev0.log");

    let log_path = Path::new(log_directory).join(log_filename);
    let archive_path = Path::new(log_directory).join(archive_filename);
    let archive_path_str = archive_path.to_str().ok_or_else(|| {
        DatapipeError::ConfigurationError("Log archive path is not valid UTF-8".to_string())
    })?;
    let archive_prefix = archive_path_str
        .strip_suffix("0.log")
        .ok_or_else(|| {
            DatapipeError::ConfigurationError("Invalid log archive pattern".to_string())
        })?
        .to_owned();

    let (sink, created_new) =
        RollingFileSink::new(log_path.clone(), archive_path.clone(), TRIGGER_FILE_SIZE)?;
    let core = Arc::new(LoggerCore {
        level: AtomicU8::new(level_to_u8(LOG_LEVEL)),
        update_global_max_level: true,
        sink: Mutex::new(LogSink::RollingFile(sink)),
    });

    if let Err(err) = log::set_boxed_logger(Box::new(DatapipeLogger {
        core: Arc::clone(&core),
    })) {
        let mut cleanup_handle = Handle {
            core: Arc::clone(&core),
        };
        silence(&mut cleanup_handle);
        if created_new && !keep_logs {
            remove_regular_file_if_exists(&log_path);
        }
        return Err(DatapipeError::ConfigurationError(format!(
            "Failed to install logger: {err}"
        )));
    }
    log::set_max_level(LOG_LEVEL);

    Ok(Logger {
        log_directory: Path::new(log_directory).to_path_buf(),
        log_path,
        archive_path,
        archive_prefix,
        handle: Handle { core },
        keep_logs,
    })
}

/// Initialize a console-only logger (useful for unit tests).
///
/// # Examples
///
/// ```no_run
/// use datapipe::datapipe_types::DatapipeError;
/// use datapipe::logger::init_console_logger;
///
/// fn main() -> Result<(), DatapipeError> {
///     let _handle = init_console_logger()?;
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns [`DatapipeError::ConfigurationError`] if global logger installation fails.
pub fn init_console_logger() -> Result<Handle, DatapipeError> {
    let core = Arc::new(LoggerCore {
        level: AtomicU8::new(level_to_u8(LOG_LEVEL)),
        update_global_max_level: true,
        sink: Mutex::new(LogSink::Console),
    });

    log::set_boxed_logger(Box::new(DatapipeLogger {
        core: Arc::clone(&core),
    }))
    .map_err(|err| {
        DatapipeError::ConfigurationError(format!("Failed to install console logger: {err}"))
    })?;
    log::set_max_level(LOG_LEVEL);

    Ok(Handle { core })
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::Level;
    use std::time::Duration;

    fn temp_test_dir(suffix: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "datapipe_logger_{suffix}_{}_{unique}",
            process::id()
        ));
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn test_format_timestamp_and_log_entry() -> Result<(), Box<dyn std::error::Error>> {
        let ts =
            format_timestamp(SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789));
        let parsed: jiff::Timestamp = ts.parse()?;
        assert_eq!(parsed.as_second(), 1_700_000_000);
        assert_eq!(parsed.subsec_nanosecond(), 123_456_789);

        let record_with_loc = Record::builder()
            .args(format_args!("hello world"))
            .level(Level::Info)
            .file(Some("src/main.rs"))
            .line(Some(42))
            .build();
        assert_eq!(
            format_log_entry(&ts, &record_with_loc),
            format!("{ts} [INFO] src/main.rs:42 hello world\n")
        );

        let record_without_loc = Record::builder()
            .args(format_args!("missing loc"))
            .level(Level::Warn)
            .build();
        assert_eq!(
            format_log_entry(&ts, &record_without_loc),
            format!("{ts} [WARN] ???:??? missing loc\n")
        );
        Ok(())
    }

    #[test]
    fn test_validate_log_basename_rejects_invalid_names() {
        for invalid in ["", ".", "..", "../escape", "dir/file", "dir\\file", "bad\0"] {
            assert!(
                matches!(
                    validate_log_basename(invalid),
                    Err(DatapipeError::ValidationError(_))
                ),
                "expected ValidationError for {invalid:?}"
            );
        }
        assert!(validate_log_basename("datapipe").is_ok());
        assert!(validate_log_basename("datapipe-gui_1").is_ok());

        assert!(matches!(
            init_logger("/var/tmp", "../escape", false),
            Err(DatapipeError::ValidationError(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn test_open_log_file_secure_permissions_and_rejects_symlinks()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let dir = temp_test_dir("symlink")?;
        let target_file = dir.join("victim.txt");
        fs::write(&target_file, "do-not-clobber")?;

        // Creating a new log file sets mode 0o600
        let secure_log = dir.join("secure.log");
        let (file, created) = open_log_file_secure(&secure_log, false)?;
        assert!(created);
        drop(file);
        assert_eq!(
            fs::metadata(&secure_log)?.permissions().mode() & 0o777,
            0o600
        );

        // Symlink as active log path must be rejected without modifying target
        let symlink_log = dir.join("symlink_active.log");
        symlink(&target_file, &symlink_log)?;
        let err = open_log_file_secure(&symlink_log, false).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        let trunc_err = open_log_file_secure(&symlink_log, true).unwrap_err();
        assert_eq!(trunc_err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(fs::read_to_string(&target_file)?, "do-not-clobber");

        // Dangling symlink must also be rejected cleanly
        let dangling_link = dir.join("dangling.log");
        symlink(dir.join("missing_target.txt"), &dangling_link)?;
        let dangling_err = open_log_file_secure(&dangling_link, false).unwrap_err();
        assert_eq!(dangling_err.kind(), io::ErrorKind::InvalidInput);

        // Symlink as archive path must be rejected on rotation without removing target
        let rot_log = dir.join("rot.log");
        let rot_archive = dir.join("rot_prev0.log");
        symlink(&target_file, &rot_archive)?;
        let (mut sink, _) = RollingFileSink::new(rot_log.clone(), rot_archive.clone(), 8)?;
        let rot_err = sink.write_entry("123456789\n").unwrap_err();
        assert_eq!(rot_err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(fs::read_to_string(&target_file)?, "do-not-clobber");

        sink.close();
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_rolling_file_sink_rotation() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_test_dir("rotation")?;
        let log_path = dir.join("app.log");
        let archive_path = dir.join("app_prev0.log");

        let (mut sink, created_new) =
            RollingFileSink::new(log_path.clone(), archive_path.clone(), 20)?;
        assert!(created_new);
        sink.write_entry("1234567890\n")?; // 11 bytes (< 20)
        assert!(!archive_path.exists());

        sink.write_entry("abcdefghij\n")?; // +11 = 22 bytes (>= 20 -> rotates)
        assert!(archive_path.exists());
        assert_eq!(
            fs::read_to_string(&archive_path)?,
            "1234567890\nabcdefghij\n"
        );
        assert_eq!(fs::read_to_string(&log_path)?, "");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&log_path)?.permissions().mode() & 0o777, 0o600);
            assert_eq!(
                fs::metadata(&archive_path)?.permissions().mode() & 0o777,
                0o600
            );
        }

        sink.write_entry("second-batch-payload!\n")?; // 22 bytes (>= 20 -> rotates again)
        assert_eq!(
            fs::read_to_string(&archive_path)?,
            "second-batch-payload!\n"
        );
        assert_eq!(fs::read_to_string(&log_path)?, "");

        sink.close();
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_rolling_file_sink_new_errors() -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_test_dir("errors")?;
        let missing_dir_log = dir.join("missing_subdir").join("app.log");
        let archive = dir.join("app_prev0.log");
        assert!(matches!(
            RollingFileSink::new(missing_dir_log, archive.clone(), 100),
            Err(DatapipeError::InputOutputError(_))
        ));

        let valid_log = dir.join("zero_trigger.log");
        assert!(matches!(
            RollingFileSink::new(valid_log, archive, 0),
            Err(DatapipeError::ConfigurationError(_))
        ));

        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_rolling_file_sink_recovers_after_failed_rotation()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_test_dir("rotate_recovery")?;
        let log_path = dir.join("app.log");
        let missing_archive_dir = dir.join("missing_archive_dir");
        let archive_path = missing_archive_dir.join("app_prev0.log");

        let (mut sink, _) = RollingFileSink::new(log_path.clone(), archive_path.clone(), 10)?;
        // First write exceeds trigger_size; rename into missing_archive_dir fails,
        // yet active log file is reopened rather than permanently lost.
        assert!(sink.write_entry("first-123456\n").is_err());

        // Fix the archive directory and confirm subsequent writes succeed & rotate cleanly.
        fs::create_dir_all(&missing_archive_dir)?;
        sink.write_entry("second-123\n")?;
        assert!(archive_path.exists());
        let archived = fs::read_to_string(&archive_path)?;
        assert!(archived.contains("first-123456\n"));
        assert!(archived.contains("second-123\n"));
        assert_eq!(fs::read_to_string(&log_path)?, "");

        sink.close();
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_datapipe_logger_level_filtering_and_reversible_off()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = temp_test_dir("filtering")?;
        let log_path = dir.join("filter.log");
        let archive_path = dir.join("filter_prev0.log");
        let (sink, _) = RollingFileSink::new(log_path.clone(), archive_path, 4096)?;

        let core = Arc::new(LoggerCore {
            level: AtomicU8::new(level_to_u8(LevelFilter::Info)),
            update_global_max_level: false,
            sink: Mutex::new(LogSink::RollingFile(sink)),
        });
        let logger_impl = DatapipeLogger {
            core: Arc::clone(&core),
        };
        let handle = Handle { core };

        let debug_meta = log::Metadata::builder().level(Level::Debug).build();
        let info_meta = log::Metadata::builder().level(Level::Info).build();
        assert!(!logger_impl.enabled(&debug_meta));
        assert!(logger_impl.enabled(&info_meta));

        let debug_record = Record::builder()
            .metadata(debug_meta.clone())
            .args(format_args!("debug-hidden"))
            .file(Some("lib.rs"))
            .line(Some(10))
            .build();
        let info_record = Record::builder()
            .metadata(info_meta.clone())
            .args(format_args!("info-visible"))
            .file(Some("lib.rs"))
            .line(Some(20))
            .build();

        logger_impl.log(&debug_record);
        logger_impl.log(&info_record);
        logger_impl.flush();

        let contents = fs::read_to_string(&log_path)?;
        assert!(!contents.contains("debug-hidden"));
        assert!(contents.contains("info-visible"));

        // Disable via Off (closes file handle), then re-enable at Debug (lazily reopens)
        handle.set_level(LevelFilter::Off);
        assert!(!logger_impl.enabled(&info_meta));
        logger_impl.log(&info_record);

        handle.set_level(LevelFilter::Debug);
        assert!(logger_impl.enabled(&debug_meta));
        let debug_after = Record::builder()
            .metadata(debug_meta)
            .args(format_args!("debug-restored"))
            .file(Some("lib.rs"))
            .line(Some(30))
            .build();
        logger_impl.log(&debug_after);
        logger_impl.flush();

        let restored = fs::read_to_string(&log_path)?;
        assert!(restored.contains("debug-restored"));

        handle.set_level(LevelFilter::Off);
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn test_silence_and_logger_drop_cleanup() -> Result<(), Box<dyn std::error::Error>> {
        // Verify cleanup when keep_logs == false leaves unrelated sibling files intact
        let cleanup_dir = temp_test_dir("cleanup")?;
        let log_path = cleanup_dir.join("test_PID1.log");
        let archive_path = cleanup_dir.join("test_PID1_prev0.log");
        let archive_prefix = archive_path
            .to_str()
            .and_then(|s| s.strip_suffix("0.log"))
            .ok_or("non-UTF-8 test path")?
            .to_owned();
        let unrelated_sibling = cleanup_dir.join("test_PID1_prev_unrelated.txt");
        fs::write(&unrelated_sibling, "keep-me")?;

        let (mut sink, _) = RollingFileSink::new(log_path.clone(), archive_path.clone(), 16)?;
        sink.write_entry("line-one-sixteen!\n")?; // triggers rotation into _prev0.log
        sink.write_entry("active-line\n")?;

        let core = Arc::new(LoggerCore {
            level: AtomicU8::new(level_to_u8(LOG_LEVEL)),
            update_global_max_level: false,
            sink: Mutex::new(LogSink::RollingFile(sink)),
        });
        let mut handle = Handle { core };
        silence(&mut handle);
        assert_eq!(handle.level(), LevelFilter::Off);

        let logger = Logger {
            log_directory: cleanup_dir.clone(),
            log_path: log_path.clone(),
            archive_path: archive_path.clone(),
            archive_prefix,
            handle,
            keep_logs: false,
        };
        drop(logger);
        assert!(!log_path.exists());
        assert!(!archive_path.exists());
        assert!(unrelated_sibling.exists());
        fs::remove_dir_all(&cleanup_dir)?;

        // Verify retention when keep_logs == true
        let keep_dir = temp_test_dir("keep")?;
        let keep_log = keep_dir.join("keep_PID1.log");
        let keep_archive = keep_dir.join("keep_PID1_prev0.log");
        let keep_prefix = keep_archive
            .to_str()
            .and_then(|s| s.strip_suffix("0.log"))
            .ok_or("non-UTF-8 test path")?
            .to_owned();
        let (mut keep_sink, _) = RollingFileSink::new(keep_log.clone(), keep_archive.clone(), 16)?;
        keep_sink.write_entry("line-one-sixteen!\n")?;
        keep_sink.write_entry("active-line\n")?;
        let keep_core = Arc::new(LoggerCore {
            level: AtomicU8::new(level_to_u8(LOG_LEVEL)),
            update_global_max_level: false,
            sink: Mutex::new(LogSink::RollingFile(keep_sink)),
        });
        let keep_logger = Logger {
            log_directory: keep_dir.clone(),
            log_path: keep_log.clone(),
            archive_path: keep_archive.clone(),
            archive_prefix: keep_prefix,
            handle: Handle { core: keep_core },
            keep_logs: true,
        };
        drop(keep_logger);
        assert!(keep_log.exists());
        assert!(keep_archive.exists());
        fs::remove_dir_all(&keep_dir)?;
        Ok(())
    }
}
