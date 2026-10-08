//! Graphical User Interface implementation using egui / eframe.

pub mod decrypt_tab;
pub mod encrypt_tab;
pub mod input_tab;
pub mod output_tab;
pub mod pipeline;
pub mod run_tab;
pub mod style;
pub mod types;

use decrypt_tab::show_decrypt_tab;
use encrypt_tab::show_encrypt_tab;
use input_tab::show_input_tab;
use output_tab::show_output_tab;
use pipeline::PipelineController;
use run_tab::show_run_tab;
use style::{configure_visuals, tab_button};
use types::{DecryptState, EncryptState, InputState, OutputState, Tab};

use eframe::{App, Frame, NativeOptions};
use egui::{CentralPanel, Ui};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// The primary egui application state for datapipe.
pub struct DatapipeApp {
    /// Currently active tab
    pub active_tab: Tab,
    /// State for the input tab
    pub input_state: InputState,
    /// State for the decrypt tab
    pub decrypt_state: DecryptState,
    /// State for the encrypt tab
    pub encrypt_state: EncryptState,
    /// State for the output tab (1 to 64 outputs)
    pub output_states: Vec<OutputState>,
    /// Background streaming pipeline controller
    pub controller: PipelineController,
    /// Tokio runtime instance for async operations
    pub runtime: Arc<Runtime>,
    /// Last status message and error flag
    pub last_message: Option<(String, bool)>,
}

impl std::fmt::Debug for DatapipeApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatapipeApp")
            .field("active_tab", &self.active_tab)
            .field("input_state", &self.input_state)
            .field("decrypt_state", &self.decrypt_state)
            .field("encrypt_state", &self.encrypt_state)
            .field("output_states", &self.output_states)
            .field("controller", &self.controller)
            .field("last_message", &self.last_message)
            .finish()
    }
}

impl DatapipeApp {
    /// Creates a new `DatapipeApp` instance with default state.
    ///
    /// # Errors
    /// Returns [`std::io::Error`] if the Tokio runtime fails to initialize.
    pub fn new() -> Result<Self, std::io::Error> {
        let runtime = Arc::new(Runtime::new()?);
        Ok(Self {
            active_tab: Tab::Input,
            input_state: InputState::default(),
            decrypt_state: DecryptState::default(),
            encrypt_state: EncryptState::default(),
            output_states: vec![OutputState::default()],
            controller: PipelineController::new(),
            runtime,
            last_message: None,
        })
    }

    /// Renders the top tab bar matching the mockup layout.
    fn render_tab_bar(&mut self, ui: &mut Ui) {
        let tabs = [
            Tab::Input,
            Tab::Decrypt,
            Tab::Encrypt,
            Tab::Output,
            Tab::Run,
        ];

        let total_width = ui.available_width();
        let tab_width = total_width / tabs.len() as f32;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for &t in &tabs {
                let is_selected = self.active_tab == t;
                if tab_button(ui, t.title(), is_selected, tab_width).clicked() {
                    self.active_tab = t;
                }
            }
        });
    }
}

impl App for DatapipeApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        // Poll events from the background streaming pipeline
        self.controller.poll_events();

        if self.controller.is_running() {
            ui.ctx().request_repaint();
        }

        CentralPanel::default().show(ui, |ui| {
            self.render_tab_bar(ui);

            ui.add_space(8.0);

            match self.active_tab {
                Tab::Input => {
                    show_input_tab(ui, &mut self.input_state, &mut self.active_tab);
                }
                Tab::Decrypt => {
                    show_decrypt_tab(ui, &mut self.decrypt_state, &mut self.active_tab);
                }
                Tab::Encrypt => {
                    show_encrypt_tab(ui, &mut self.encrypt_state, &mut self.active_tab);
                }
                Tab::Output => {
                    show_output_tab(ui, &mut self.output_states, &mut self.active_tab);
                }
                Tab::Run => {
                    show_run_tab(ui, self);
                }
            }
        });
    }
}

/// Runs the datapipe native GUI application.
///
/// # Errors
/// Returns [`eframe::Error`] if native application window creation or execution fails.
pub fn run_gui() -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("datapipe")
            .with_inner_size([760.0, 520.0])
            .with_min_inner_size([640.0, 460.0]),
        ..Default::default()
    };

    eframe::run_native(
        "datapipe",
        options,
        Box::new(|cc| {
            configure_visuals(&cc.egui_ctx);
            let app = match DatapipeApp::new() {
                Ok(app) => app,
                Err(err) => {
                    eprintln!("Failed to initialize DatapipeApp runtime: {err}");
                    std::process::exit(1);
                }
            };
            Ok(Box::new(app))
        }),
    )
}
