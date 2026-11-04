//! Graphical User Interface implementation using `egui` / `eframe` with Material Design 3.

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
use style::{
    ThemeMode, apply_theme, build_m3_style, configure_visuals, paint_brand_icon, palette,
    primary_tab, status_chip, status_chip_with_dot, theme_toggle_button,
};
use types::{DecryptState, EncryptState, InputState, OutputState, PipelineStatus, Tab};

use eframe::{App, Frame, NativeOptions};
use egui::{
    CentralPanel, CornerRadius, Frame as EguiFrame, Layout, Margin, RichText, Stroke, Ui, pos2,
    vec2,
};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// The primary `egui` application state for `datapipe`.
pub struct DatapipeApp {
    /// Currently active navigation tab.
    pub active_tab: Tab,
    /// Active Material Design 3 color scheme (`Light` or `Dark`).
    pub theme_mode: ThemeMode,
    /// State for the Input stage tab.
    pub input_state: InputState,
    /// State for the Decrypt stage tab.
    pub decrypt_state: DecryptState,
    /// State for the Encrypt stage tab.
    pub encrypt_state: EncryptState,
    /// State for the Output stage tab (1 to 64 destinations).
    pub output_states: Vec<OutputState>,
    /// Background streaming pipeline controller.
    pub controller: PipelineController,
    /// Tokio runtime instance for asynchronous pipeline execution.
    pub runtime: Arc<Runtime>,
    /// Last user-facing status message and error indicator flag.
    pub last_message: Option<(String, bool)>,
    /// Tracks which [`ThemeMode`] has already been applied to the [`egui::Context`].
    applied_theme: Option<ThemeMode>,
}

impl std::fmt::Debug for DatapipeApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatapipeApp")
            .field("active_tab", &self.active_tab)
            .field("theme_mode", &self.theme_mode)
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
    /// Creates a new [`DatapipeApp`] instance with default Material Design 3 Light state.
    ///
    /// # Errors
    /// Returns [`std::io::Error`] if the Tokio runtime fails to initialize.
    pub fn new() -> Result<Self, std::io::Error> {
        let runtime = Arc::new(Runtime::new()?);
        Ok(Self {
            active_tab: Tab::Input,
            theme_mode: ThemeMode::Light,
            input_state: InputState::default(),
            decrypt_state: DecryptState::default(),
            encrypt_state: EncryptState::default(),
            output_states: vec![OutputState::default()],
            controller: PipelineController::new(),
            runtime,
            last_message: None,
            applied_theme: None,
        })
    }

    /// Synchronizes the [`egui::Context`] and root [`Ui`] style whenever [`Self::theme_mode`] changes.
    fn sync_theme(&mut self, ui: &mut Ui) {
        if self.applied_theme != Some(self.theme_mode) {
            apply_theme(ui.ctx(), self.theme_mode);
            *ui.style_mut() = build_m3_style(self.theme_mode);
            self.applied_theme = Some(self.theme_mode);
        }
    }

    /// Renders the Material Design 3 Top App Bar and Primary Tab Navigation Bar.
    fn render_top_app_bar(&mut self, ui: &mut Ui) {
        let p = palette(ui);
        let header_frame = EguiFrame::NONE.fill(p.surface_container_low);

        egui::Panel::top("m3_top_app_bar")
            .resizable(false)
            .show_separator_line(false)
            .frame(header_frame)
            .show(ui, |ui| {
                // 1. M3 Top App Bar header row
                EguiFrame::NONE
                    .inner_margin(Margin {
                        left: 24,
                        right: 24,
                        top: 12,
                        bottom: 8,
                    })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Brand icon container (M3 Primary tonal badge)
                            let (icon_rect, _) =
                                ui.allocate_exact_size(vec2(34.0, 34.0), egui::Sense::hover());
                            ui.painter()
                                .rect_filled(icon_rect, CornerRadius::same(10), p.primary);
                            paint_brand_icon(ui.painter(), icon_rect, p.on_primary);

                            ui.add_space(4.0);

                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(
                                    RichText::new("datapipe")
                                        .size(19.0)
                                        .strong()
                                        .color(p.on_surface),
                                );
                                ui.label(
                                    RichText::new("Asynchronous Stream Pipeline")
                                        .size(11.5)
                                        .color(p.on_surface_variant),
                                );
                            });

                            // Right-aligned M3 Theme Toggle & live pipeline status chip
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if theme_toggle_button(ui, self.theme_mode)
                                    .on_hover_text(
                                        "Toggle between Material Design 3 Light and Dark color schemes",
                                    )
                                    .clicked()
                                {
                                    self.theme_mode.toggle();
                                }

                                match &self.controller.status {
                                    PipelineStatus::Idle => {}
                                    PipelineStatus::Ready => {
                                        status_chip(
                                            ui,
                                            "READY",
                                            p.secondary_container,
                                            p.on_secondary_container,
                                        );
                                    }
                                    PipelineStatus::Running => {
                                        status_chip_with_dot(
                                            ui,
                                            "RUNNING",
                                            p.tertiary_container,
                                            p.on_tertiary_container,
                                            true,
                                        );
                                    }
                                    PipelineStatus::Stopped => {
                                        status_chip(
                                            ui,
                                            "STOPPED",
                                            p.warning_container,
                                            p.on_warning_container,
                                        );
                                    }
                                    PipelineStatus::Completed => {
                                        status_chip(
                                            ui,
                                            "COMPLETED",
                                            p.primary_container,
                                            p.on_primary_container,
                                        );
                                    }
                                    PipelineStatus::Error(_) => {
                                        status_chip(
                                            ui,
                                            "ERROR",
                                            p.error_container,
                                            p.on_error_container,
                                        );
                                    }
                                }
                            });
                        });
                    });

                // 2. M3 Primary Tabs row
                let tabs = [
                    Tab::Input,
                    Tab::Decrypt,
                    Tab::Encrypt,
                    Tab::Output,
                    Tab::Run,
                ];
                let output_badge = if self.output_states.len() > 1 {
                    Some(self.output_states.len().to_string())
                } else {
                    None
                };

                let total_width = ui.available_width();
                let tab_width = total_width / tabs.len() as f32;

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for &t in &tabs {
                        let is_selected = self.active_tab == t;
                        let badge = match t {
                            Tab::Input => None,
                            Tab::Decrypt => self.decrypt_state.enabled.then_some("ON"),
                            Tab::Encrypt => self.encrypt_state.enabled.then_some("ON"),
                            Tab::Output => output_badge.as_deref(),
                            Tab::Run => self.controller.is_running().then_some("LIVE"),
                        };
                        if primary_tab(ui, t.title(), badge, is_selected, tab_width).clicked() {
                            self.active_tab = t;
                        }
                    }
                });

                // Full-width 1dp M3 surface divider under the Primary Tab bar
                let rect = ui.min_rect();
                let y = rect.max.y - 0.5;
                ui.painter().line_segment(
                    [pos2(rect.min.x, y), pos2(rect.max.x, y)],
                    Stroke::new(1.0, p.outline_variant),
                );
            });
    }

    /// Renders the complete Material Design 3 application shell (Top App Bar, Primary Tabs,
    /// and active tab view) into `ui`.
    pub fn render(&mut self, ui: &mut Ui) {
        self.sync_theme(ui);

        // Poll events from the background streaming pipeline
        self.controller.poll_events();

        if self.controller.is_running() {
            ui.ctx().request_repaint();
        }

        self.render_top_app_bar(ui);

        let p = palette(ui);
        CentralPanel::default()
            .frame(EguiFrame::NONE.fill(p.surface))
            .show(ui, |ui| match self.active_tab {
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
            });
    }
}

impl App for DatapipeApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        self.render(ui);
    }
}

/// Runs the `datapipe` native GUI application.
///
/// # Errors
/// Returns [`eframe::Error`] if native application window creation or execution fails.
pub fn run_gui() -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("datapipe")
            .with_inner_size([840.0, 600.0])
            .with_min_inner_size([680.0, 480.0]),
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
