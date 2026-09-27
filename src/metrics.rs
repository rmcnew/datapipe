//! Metrics tracking and live display for datapipe streaming pipelines.
//!
//! This module provides:
//! - [`ByteMetrics`]: Thread-safe byte counters for tracking input reads and output writes.
//! - [`ElaspedTime`]: Start timestamp and duration tracking for pipeline instances.
//! - [`DatapipeMetrics`]: Aggregated pipeline metrics combining reader and writer counters with timing.
//! - [`LiveDisplayGuard`]: Background task manager for displaying real-time metrics to standard error.

use std::fmt::{self, Display, Formatter};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Thread-safe byte metrics tracker for recording the number of bytes read or written.
///
/// `ByteMetrics` utilizes an atomic counter wrapped in an [`Arc`], allowing cheap cloning
/// across asynchronous tasks while referencing the same shared metric.
///
/// # Examples
///
/// ```
/// use datapipe::metrics::ByteMetrics;
///
/// let metrics = ByteMetrics::new();
/// metrics.add_bytes(1024);
/// assert_eq!(metrics.bytes(), 1024);
///
/// let clone = metrics.clone();
/// clone.add_bytes(2048);
/// assert_eq!(metrics.bytes(), 3072);
/// ```
#[derive(Debug, Clone)]
pub struct ByteMetrics {
    bytes: Arc<AtomicU64>,
}

impl ByteMetrics {
    /// Creates a new `ByteMetrics` instance with counter initialized to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ByteMetrics;
    ///
    /// let metrics = ByteMetrics::new();
    /// assert_eq!(metrics.bytes(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Creates a new `ByteMetrics` initialized with the given byte count.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ByteMetrics;
    ///
    /// let metrics = ByteMetrics::with_bytes(512);
    /// assert_eq!(metrics.bytes(), 512);
    /// ```
    #[must_use]
    pub fn with_bytes(initial_bytes: u64) -> Self {
        Self {
            bytes: Arc::new(AtomicU64::new(initial_bytes)),
        }
    }

    /// Atomically adds `count` bytes to the tracked total.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ByteMetrics;
    ///
    /// let metrics = ByteMetrics::new();
    /// metrics.add_bytes(4096);
    /// assert_eq!(metrics.bytes(), 4096);
    /// ```
    pub fn add_bytes(&self, count: u64) {
        self.bytes.fetch_add(count, Ordering::Relaxed);
    }

    /// Returns the current total number of bytes tracked.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ByteMetrics;
    ///
    /// let metrics = ByteMetrics::new();
    /// assert_eq!(metrics.bytes(), 0);
    /// ```
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }

    /// Resets the tracked byte counter back to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ByteMetrics;
    ///
    /// let metrics = ByteMetrics::with_bytes(100);
    /// metrics.reset();
    /// assert_eq!(metrics.bytes(), 0);
    /// ```
    pub fn reset(&self) {
        self.bytes.store(0, Ordering::Relaxed);
    }

    /// Returns a human-readable representation of the tracked bytes (e.g., "1.50 MB").
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ByteMetrics;
    ///
    /// let metrics = ByteMetrics::with_bytes(1_048_576);
    /// assert_eq!(metrics.human_readable(), "1.00 MB");
    /// ```
    #[must_use]
    pub fn human_readable(&self) -> String {
        format_bytes(self.bytes())
    }
}

impl Default for ByteMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for ByteMetrics {
    fn eq(&self, other: &Self) -> bool {
        self.bytes() == other.bytes()
    }
}

impl Eq for ByteMetrics {}

impl Display for ByteMetrics {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.human_readable())
    }
}

impl From<u64> for ByteMetrics {
    fn from(bytes: u64) -> Self {
        Self::with_bytes(bytes)
    }
}

/// Tracks the datapipe instance start timestamp and elapsed execution duration.
///
/// Records both a [`SystemTime`] wall-clock timestamp when started and a monotonic
/// [`Instant`] for precise duration measurements.
///
/// # Examples
///
/// ```
/// use datapipe::metrics::ElaspedTime;
///
/// let timer = ElaspedTime::new();
/// assert!(timer.elapsed_secs() >= 0.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElaspedTime {
    start_time: SystemTime,
    start_instant: Instant,
}

/// Type alias for [`ElaspedTime`] using standard English spelling.
pub type ElapsedTime = ElaspedTime;

impl ElaspedTime {
    /// Creates a new `ElaspedTime` initialized to the current time.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ElaspedTime;
    ///
    /// let timer = ElaspedTime::new();
    /// assert!(timer.elapsed().as_secs() < 10);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            start_time: SystemTime::now(),
            start_instant: Instant::now(),
        }
    }

    /// Creates an `ElaspedTime` with specified start timestamps.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ElaspedTime;
    /// use std::time::{Instant, SystemTime};
    ///
    /// let timer = ElaspedTime::with_start(SystemTime::now(), Instant::now());
    /// assert!(timer.elapsed_secs() >= 0.0);
    /// ```
    #[must_use]
    pub fn with_start(start_time: SystemTime, start_instant: Instant) -> Self {
        Self {
            start_time,
            start_instant,
        }
    }

    /// Returns the wall-clock start timestamp of the datapipe instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ElaspedTime;
    ///
    /// let timer = ElaspedTime::new();
    /// let _start = timer.start_time();
    /// ```
    #[must_use]
    pub fn start_time(&self) -> SystemTime {
        self.start_time
    }

    /// Returns the monotonic start instant of the datapipe instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ElaspedTime;
    ///
    /// let timer = ElaspedTime::new();
    /// let _instant = timer.start_instant();
    /// ```
    #[must_use]
    pub fn start_instant(&self) -> Instant {
        self.start_instant
    }

    /// Returns the elapsed [`Duration`] since this timer started.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ElaspedTime;
    ///
    /// let timer = ElaspedTime::new();
    /// assert!(timer.elapsed().as_nanos() > 0);
    /// ```
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.start_instant.elapsed()
    }

    /// Returns the elapsed duration in seconds as a floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ElaspedTime;
    ///
    /// let timer = ElaspedTime::new();
    /// assert!(timer.elapsed_secs() >= 0.0);
    /// ```
    #[must_use]
    pub fn elapsed_secs(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }

    /// Returns the formatted elapsed duration (e.g., "00:01:23").
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::ElaspedTime;
    ///
    /// let timer = ElaspedTime::new();
    /// let formatted = timer.format_elapsed();
    /// assert!(formatted.contains(':'));
    /// ```
    #[must_use]
    pub fn format_elapsed(&self) -> String {
        format_duration(self.elapsed())
    }
}

impl Default for ElaspedTime {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ElaspedTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_elapsed())
    }
}

/// Aggregated metrics for a datapipe streaming pipeline instance.
///
/// Combines reader [`ByteMetrics`], writer [`ByteMetrics`], and an [`ElaspedTime`] timer.
///
/// # Examples
///
/// ```
/// use datapipe::metrics::DatapipeMetrics;
///
/// let metrics = DatapipeMetrics::new();
/// metrics.reader_bytes.add_bytes(2048);
/// metrics.writer_bytes.add_bytes(2048);
/// assert_eq!(metrics.read_bytes(), 2048);
/// assert_eq!(metrics.written_bytes(), 2048);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DatapipeMetrics {
    /// Metrics for bytes read by the input reader.
    pub reader_bytes: ByteMetrics,
    /// Metrics for bytes written by output writer(s).
    pub writer_bytes: ByteMetrics,
    /// Start timestamp and elapsed execution duration.
    pub elapsed_time: ElaspedTime,
}

impl DatapipeMetrics {
    /// Creates a new `DatapipeMetrics` instance with zeroed byte counters and the current start time.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::DatapipeMetrics;
    ///
    /// let metrics = DatapipeMetrics::new();
    /// assert_eq!(metrics.read_bytes(), 0);
    /// assert_eq!(metrics.written_bytes(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            reader_bytes: ByteMetrics::new(),
            writer_bytes: ByteMetrics::new(),
            elapsed_time: ElaspedTime::new(),
        }
    }

    /// Returns the total number of bytes read by the input reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::DatapipeMetrics;
    ///
    /// let metrics = DatapipeMetrics::new();
    /// metrics.reader_bytes.add_bytes(100);
    /// assert_eq!(metrics.read_bytes(), 100);
    /// ```
    #[must_use]
    pub fn read_bytes(&self) -> u64 {
        self.reader_bytes.bytes()
    }

    /// Returns the total number of bytes written by output writers.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::DatapipeMetrics;
    ///
    /// let metrics = DatapipeMetrics::new();
    /// metrics.writer_bytes.add_bytes(250);
    /// assert_eq!(metrics.written_bytes(), 250);
    /// ```
    #[must_use]
    pub fn written_bytes(&self) -> u64 {
        self.writer_bytes.bytes()
    }

    /// Returns the elapsed duration since the pipeline instance started.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::DatapipeMetrics;
    ///
    /// let metrics = DatapipeMetrics::new();
    /// assert!(metrics.elapsed().as_nanos() > 0);
    /// ```
    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.elapsed_time.elapsed()
    }

    /// Calculates the instantaneous read rate in bytes per second.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::DatapipeMetrics;
    ///
    /// let metrics = DatapipeMetrics::new();
    /// metrics.reader_bytes.add_bytes(1000);
    /// assert!(metrics.read_rate_bytes_per_sec() >= 0.0);
    /// ```
    #[must_use]
    pub fn read_rate_bytes_per_sec(&self) -> f64 {
        let secs = self.elapsed_time.elapsed_secs();
        if secs > 0.0 {
            self.read_bytes() as f64 / secs
        } else {
            0.0
        }
    }

    /// Calculates the instantaneous write rate in bytes per second.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::DatapipeMetrics;
    ///
    /// let metrics = DatapipeMetrics::new();
    /// metrics.writer_bytes.add_bytes(2000);
    /// assert!(metrics.write_rate_bytes_per_sec() >= 0.0);
    /// ```
    #[must_use]
    pub fn write_rate_bytes_per_sec(&self) -> f64 {
        let secs = self.elapsed_time.elapsed_secs();
        if secs > 0.0 {
            self.written_bytes() as f64 / secs
        } else {
            0.0
        }
    }

    /// Formats live metrics for terminal or status displays.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::DatapipeMetrics;
    ///
    /// let metrics = DatapipeMetrics::new();
    /// let live = metrics.format_live_display();
    /// assert!(live.contains("[datapipe]"));
    /// assert!(live.contains("Elapsed:"));
    /// ```
    #[must_use]
    pub fn format_live_display(&self) -> String {
        format!(
            "[datapipe] Elapsed: {} | Read: {} ({}) | Written: {} ({})",
            self.elapsed_time.format_elapsed(),
            self.reader_bytes.human_readable(),
            format_rate(self.read_rate_bytes_per_sec()),
            self.writer_bytes.human_readable(),
            format_rate(self.write_rate_bytes_per_sec()),
        )
    }

    /// Formats final summary metrics after pipeline completion.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::metrics::DatapipeMetrics;
    ///
    /// let metrics = DatapipeMetrics::new();
    /// let summary = metrics.format_summary();
    /// assert!(summary.contains("[datapipe]"));
    /// assert!(summary.contains("Completed in"));
    /// ```
    #[must_use]
    pub fn format_summary(&self) -> String {
        format!(
            "[datapipe] Completed in {:.2}s | Read: {} ({}) | Written: {} ({})",
            self.elapsed_time.elapsed_secs(),
            self.reader_bytes.human_readable(),
            format_rate(self.read_rate_bytes_per_sec()),
            self.writer_bytes.human_readable(),
            format_rate(self.write_rate_bytes_per_sec()),
        )
    }
}

/// Formats a byte quantity into a human-readable string (B, KB, MB, GB).
///
/// # Examples
///
/// ```
/// use datapipe::metrics::format_bytes;
///
/// assert_eq!(format_bytes(500), "500 B");
/// assert_eq!(format_bytes(2048), "2.00 KB");
/// assert_eq!(format_bytes(1048576), "1.00 MB");
/// ```
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;

    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.2} KB", b / KB)
    } else if b < GB {
        format!("{:.2} MB", b / MB)
    } else {
        format!("{:.2} GB", b / GB)
    }
}

/// Formats a transfer rate (bytes per second) into a human-readable string.
///
/// # Examples
///
/// ```
/// use datapipe::metrics::format_rate;
///
/// assert_eq!(format_rate(500.0), "500.00 B/s");
/// assert_eq!(format_rate(2048.0), "2.00 KB/s");
/// ```
#[must_use]
pub fn format_rate(bytes_per_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * KB;
    const GB: f64 = 1024.0 * MB;

    if bytes_per_sec < KB {
        format!("{bytes_per_sec:.2} B/s")
    } else if bytes_per_sec < MB {
        format!("{:.2} KB/s", bytes_per_sec / KB)
    } else if bytes_per_sec < GB {
        format!("{:.2} MB/s", bytes_per_sec / MB)
    } else {
        format!("{:.2} GB/s", bytes_per_sec / GB)
    }
}

/// Formats a [`Duration`] as `HH:MM:SS` (or `MM:SS` if under an hour).
///
/// # Examples
///
/// ```
/// use datapipe::metrics::format_duration;
/// use std::time::Duration;
///
/// assert_eq!(format_duration(Duration::from_secs(65)), "00:01:05");
/// assert_eq!(format_duration(Duration::from_secs(3665)), "01:01:05");
/// ```
#[must_use]
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{hours:02}:{mins:02}:{secs:02}")
}

/// Controls and manages the background live metrics display task for CLI execution.
///
/// Spawns a background task that periodically writes formatted live metrics to standard error
/// with carriage returns to update in place on terminal displays.
#[derive(Debug)]
pub struct LiveDisplayGuard {
    stop_sender: Option<oneshot::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
    metrics: DatapipeMetrics,
}

impl LiveDisplayGuard {
    /// Starts live metrics display on standard error at the specified polling interval.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use datapipe::metrics::{DatapipeMetrics, LiveDisplayGuard};
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let metrics = DatapipeMetrics::new();
    ///     let guard = LiveDisplayGuard::start(metrics, Duration::from_millis(100));
    ///     guard.stop().await;
    /// }
    /// ```
    pub fn start(metrics: DatapipeMetrics, interval: Duration) -> Self {
        let (stop_sender, mut stop_receiver) = oneshot::channel();
        let display_metrics = metrics.clone();

        let join_handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                tokio::select! {
                    _ = &mut stop_receiver => {
                        break;
                    }
                    _ = ticker.tick() => {
                        eprint!("\r{}", display_metrics.format_live_display());
                        let _ = std::io::stderr().flush();
                    }
                }
            }
        });

        Self {
            stop_sender: Some(stop_sender),
            join_handle: Some(join_handle),
            metrics,
        }
    }

    /// Stops the live display task, waits for it to terminate, and prints the final summary.
    pub async fn stop(mut self) {
        if let Some(sender) = self.stop_sender.take() {
            let _ = sender.send(());
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.await;
        }
        eprintln!("\r{}", self.metrics.format_summary());
        let _ = std::io::stderr().flush();
    }
}

impl Drop for LiveDisplayGuard {
    fn drop(&mut self) {
        if let Some(sender) = self.stop_sender.take() {
            let _ = sender.send(());
        }
        if let Some(handle) = self.join_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_metrics_basic() {
        let metrics = ByteMetrics::new();
        assert_eq!(metrics.bytes(), 0);
        assert_eq!(metrics.to_string(), "0 B");

        metrics.add_bytes(512);
        assert_eq!(metrics.bytes(), 512);
        assert_eq!(metrics.human_readable(), "512 B");

        metrics.add_bytes(1024);
        assert_eq!(metrics.bytes(), 1536);
        assert_eq!(metrics.human_readable(), "1.50 KB");

        metrics.reset();
        assert_eq!(metrics.bytes(), 0);
    }

    #[test]
    fn test_byte_metrics_cloning_shares_state() {
        let m1 = ByteMetrics::new();
        let m2 = m1.clone();

        m1.add_bytes(100);
        assert_eq!(m2.bytes(), 100);
        assert_eq!(m1, m2);

        m2.add_bytes(200);
        assert_eq!(m1.bytes(), 300);
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_byte_metrics_from_u64() {
        let metrics = ByteMetrics::from(4096);
        assert_eq!(metrics.bytes(), 4096);
        assert_eq!(metrics.human_readable(), "4.00 KB");
    }

    #[test]
    fn test_elapsed_time_basic() {
        let timer = ElaspedTime::new();
        assert!(timer.elapsed_secs() >= 0.0);
        assert_eq!(timer.start_time(), timer.start_time);
        assert_eq!(timer.start_instant(), timer.start_instant);

        let formatted = timer.format_elapsed();
        assert!(formatted.contains(':'));
        assert_eq!(timer.to_string(), formatted);
    }

    #[test]
    fn test_elapsed_time_alias() {
        let timer: ElapsedTime = ElapsedTime::new();
        assert!(timer.elapsed().as_nanos() > 0);
    }

    #[test]
    fn test_datapipe_metrics_aggregation() {
        let metrics = DatapipeMetrics::new();
        metrics.reader_bytes.add_bytes(1024);
        metrics.writer_bytes.add_bytes(2048);

        assert_eq!(metrics.read_bytes(), 1024);
        assert_eq!(metrics.written_bytes(), 2048);
        assert!(metrics.read_rate_bytes_per_sec() >= 0.0);
        assert!(metrics.write_rate_bytes_per_sec() >= 0.0);

        let live = metrics.format_live_display();
        assert!(live.contains("[datapipe]"));
        assert!(live.contains("Elapsed:"));
        assert!(live.contains("Read: 1.00 KB"));
        assert!(live.contains("Written: 2.00 KB"));

        let summary = metrics.format_summary();
        assert!(summary.contains("[datapipe] Completed in"));
        assert!(summary.contains("Read: 1.00 KB"));
        assert!(summary.contains("Written: 2.00 KB"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1_572_864), "1.50 MB");
        assert_eq!(format_bytes(2_147_483_648), "2.00 GB");
    }

    #[test]
    fn test_format_rate() {
        assert_eq!(format_rate(50.0), "50.00 B/s");
        assert_eq!(format_rate(2048.0), "2.00 KB/s");
        assert_eq!(format_rate(2_097_152.0), "2.00 MB/s");
        assert_eq!(format_rate(3_221_225_472.0), "3.00 GB/s");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "00:00:00");
        assert_eq!(format_duration(Duration::from_secs(59)), "00:00:59");
        assert_eq!(format_duration(Duration::from_secs(60)), "00:01:00");
        assert_eq!(format_duration(Duration::from_secs(3599)), "00:59:59");
        assert_eq!(format_duration(Duration::from_secs(3600)), "01:00:00");
        assert_eq!(format_duration(Duration::from_secs(86400)), "24:00:00");
    }

    #[tokio::test]
    async fn test_live_display_guard() {
        let metrics = DatapipeMetrics::new();
        metrics.reader_bytes.add_bytes(500);
        metrics.writer_bytes.add_bytes(500);

        let guard = LiveDisplayGuard::start(metrics.clone(), Duration::from_millis(50));
        tokio::time::sleep(Duration::from_millis(120)).await;
        metrics.reader_bytes.add_bytes(500);
        metrics.writer_bytes.add_bytes(500);
        guard.stop().await;

        assert_eq!(metrics.read_bytes(), 1000);
        assert_eq!(metrics.written_bytes(), 1000);
    }
}
