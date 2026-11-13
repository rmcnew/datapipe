//! Pipeline background execution controller and task management.

use crate::config::DatapipeConfig;
use crate::engine::run_datapipe;
use crate::gui::types::{MetricsSnapshot, PipelineEvent, PipelineStatus};
use crate::metrics::DatapipeMetrics;
use std::sync::mpsc::{Receiver, Sender, channel};
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

/// Controller managing the background datapipe streaming task.
#[derive(Debug)]
pub struct PipelineController {
    /// Current execution status
    pub status: PipelineStatus,
    /// Shared metrics tracker for live statistics
    pub metrics: Option<DatapipeMetrics>,
    /// Metrics frozen at the moment the pipeline finished or was stopped.
    ///
    /// When present, this takes precedence over the live `metrics` so the
    /// elapsed time and transfer rates stop updating once the pipeline ends.
    final_snapshot: Option<MetricsSnapshot>,
    /// Background task join handle
    task_handle: Option<JoinHandle<()>>,
    /// Receiver for async pipeline events
    event_receiver: Option<Receiver<PipelineEvent>>,
}

impl Default for PipelineController {
    fn default() -> Self {
        Self {
            status: PipelineStatus::Idle,
            metrics: None,
            final_snapshot: None,
            task_handle: None,
            event_receiver: None,
        }
    }
}

impl PipelineController {
    /// Creates a new `PipelineController`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the pipeline is currently running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self.status, PipelineStatus::Running)
    }

    /// Returns the metrics to display.
    ///
    /// - While running: a fresh snapshot of the live metrics.
    /// - After finishing or stopping: the snapshot frozen at that moment.
    /// - Before any run, or after [`reset_metrics`](Self::reset_metrics): all zeros.
    #[must_use]
    pub fn metrics_snapshot(&self) -> MetricsSnapshot {
        if let Some(snapshot) = self.final_snapshot {
            snapshot
        } else if let Some(ref metrics) = self.metrics {
            MetricsSnapshot::from_metrics(metrics)
        } else {
            MetricsSnapshot::default()
        }
    }

    /// Discards all collected metrics so the display shows zeros, and returns
    /// the controller to the idle state. Has no effect while running.
    pub fn reset_metrics(&mut self) {
        if self.is_running() {
            return;
        }
        self.metrics = None;
        self.final_snapshot = None;
        self.event_receiver = None;
        self.status = PipelineStatus::Idle;
    }

    /// Starts the datapipe streaming pipeline in the background using the provided configuration.
    pub fn start(&mut self, rt_handle: &Handle, config: DatapipeConfig) {
        if self.is_running() {
            return;
        }
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        let (tx, rx): (Sender<PipelineEvent>, Receiver<PipelineEvent>) = channel();
        self.event_receiver = Some(rx);

        let metrics = DatapipeMetrics::new();
        self.metrics = Some(metrics.clone());
        self.final_snapshot = None;
        self.status = PipelineStatus::Running;

        let mut config_with_metrics = config;
        config_with_metrics.no_metrics = false;

        let metrics_clone = metrics;
        let handle = rt_handle.spawn(async move {
            let params_res = config_with_metrics.to_parameters().await;
            match params_res {
                Ok(mut params) => {
                    params.metrics = Some(metrics_clone.clone());
                    let _ = tx.send(PipelineEvent::Started);
                    let result = run_datapipe(params).await;
                    // Freeze metrics immediately on completion so the GUI does not
                    // keep accumulating elapsed time until its next frame.
                    let snapshot = MetricsSnapshot::from_metrics(&metrics_clone);
                    let _ = tx.send(PipelineEvent::Finished(
                        result.map_err(|e| e.to_string()),
                        snapshot,
                    ));
                }
                Err(err) => {
                    let snapshot = MetricsSnapshot::from_metrics(&metrics_clone);
                    let _ = tx.send(PipelineEvent::Finished(Err(err.to_string()), snapshot));
                }
            }
        });

        self.task_handle = Some(handle);
    }

    /// Stops the currently running datapipe pipeline and freezes its metrics.
    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        if self.final_snapshot.is_none()
            && let Some(ref metrics) = self.metrics
        {
            self.final_snapshot = Some(MetricsSnapshot::from_metrics(metrics));
        }
        // Ignore any late events from the aborted task.
        self.event_receiver = None;
        self.status = PipelineStatus::Stopped;
    }

    /// Polls for incoming events from the background pipeline task.
    pub fn poll_events(&mut self) {
        let mut finished_event = None;
        if let Some(ref rx) = self.event_receiver {
            while let Ok(event) = rx.try_recv() {
                match event {
                    PipelineEvent::Started => {
                        self.status = PipelineStatus::Running;
                    }
                    PipelineEvent::Finished(result, snapshot) => {
                        finished_event = Some((result, snapshot));
                    }
                }
            }
        }

        if let Some((result, snapshot)) = finished_event {
            self.task_handle = None;
            self.event_receiver = None;
            self.final_snapshot = Some(snapshot);
            match result {
                Ok(()) => {
                    self.status = PipelineStatus::Completed;
                }
                Err(err) => {
                    self.status = PipelineStatus::Error(err);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_zero_by_default() {
        let controller = PipelineController::new();
        assert_eq!(controller.metrics_snapshot(), MetricsSnapshot::default());
    }

    #[test]
    fn test_stop_freezes_metrics() {
        let mut controller = PipelineController::new();
        let metrics = DatapipeMetrics::new();
        metrics.reader_bytes.add_bytes(100);
        controller.metrics = Some(metrics.clone());
        controller.status = PipelineStatus::Running;

        controller.stop();
        let frozen = controller.metrics_snapshot();

        // Further activity on the live metrics must not change the display.
        metrics.reader_bytes.add_bytes(900);
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert_eq!(controller.metrics_snapshot(), frozen);
        assert_eq!(frozen.read_bytes, 100);
    }

    #[test]
    fn test_finished_event_freezes_metrics() {
        let mut controller = PipelineController::new();
        let (tx, rx) = channel();
        controller.event_receiver = Some(rx);
        controller.status = PipelineStatus::Running;
        let metrics = DatapipeMetrics::new();
        controller.metrics = Some(metrics.clone());

        let snapshot = MetricsSnapshot {
            read_bytes: 42,
            ..Default::default()
        };
        assert!(tx.send(PipelineEvent::Finished(Ok(()), snapshot)).is_ok());
        controller.poll_events();

        metrics.reader_bytes.add_bytes(1000);
        assert_eq!(controller.status, PipelineStatus::Completed);
        assert_eq!(controller.metrics_snapshot(), snapshot);
    }

    #[test]
    fn test_reset_metrics_zeroes_display() {
        let mut controller = PipelineController::new();
        let metrics = DatapipeMetrics::new();
        metrics.writer_bytes.add_bytes(512);
        controller.metrics = Some(metrics);
        controller.status = PipelineStatus::Completed;
        controller.final_snapshot = Some(MetricsSnapshot {
            written_bytes: 512,
            ..Default::default()
        });

        controller.reset_metrics();
        assert_eq!(controller.metrics_snapshot(), MetricsSnapshot::default());
        assert!(controller.metrics.is_none());
        assert_eq!(controller.status, PipelineStatus::Idle);
    }
}
