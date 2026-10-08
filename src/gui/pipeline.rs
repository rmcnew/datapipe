//! Pipeline background execution controller and task management.

use crate::config::DatapipeConfig;
use crate::engine::run_datapipe;
use crate::gui::types::{PipelineEvent, PipelineStatus};
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

    /// Starts the datapipe streaming pipeline in the background using the provided configuration.
    pub fn start(&mut self, rt_handle: &Handle, config: DatapipeConfig) {
        let (tx, rx): (Sender<PipelineEvent>, Receiver<PipelineEvent>) = channel();
        self.event_receiver = Some(rx);

        let metrics = DatapipeMetrics::new();
        self.metrics = Some(metrics.clone());
        self.status = PipelineStatus::Running;

        let mut config_with_metrics = config;
        config_with_metrics.no_metrics = false;

        let metrics_clone = metrics.clone();
        let handle = rt_handle.spawn(async move {
            let params_res = config_with_metrics.to_parameters().await;
            match params_res {
                Ok(mut params) => {
                    params.metrics = Some(metrics_clone);
                    let _ = tx.send(PipelineEvent::Started);
                    let result = run_datapipe(params).await;
                    let _ = tx.send(PipelineEvent::Finished(result.map_err(|e| e.to_string())));
                }
                Err(err) => {
                    let _ = tx.send(PipelineEvent::Finished(Err(err.to_string())));
                }
            }
        });

        self.task_handle = Some(handle);
    }

    /// Stops the currently running datapipe pipeline.
    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
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
                    PipelineEvent::Finished(result) => {
                        finished_event = Some(result);
                    }
                }
            }
        }

        if let Some(result) = finished_event {
            self.task_handle = None;
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
