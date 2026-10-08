//! Run tab UI view, animated pipeline diagrams, metrics monitoring, and start/stop controls.

use crate::config::DatapipeConfig;
use crate::gui::pipeline::PipelineController;
use crate::gui::style::{
    BLUE_BUTTON_BG, BORDER_COLOR, PANEL_BG, blue_button, nav_arrow, start_button, stop_button,
};
use crate::gui::types::{
    DecryptState, EncryptState, InputState, OutputState, PipelineStatus, Tab,
    load_config_into_states, states_to_config,
};
use crate::metrics::{format_bytes, format_duration, format_rate};
use egui::{
    Align2, Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText, Stroke, Ui, Vec2,
};

/// Renders the Run tab.
pub fn show_run_tab(ui: &mut Ui, app: &mut crate::gui::DatapipeApp) {
    ui.add_space(8.0);

    // Top buttons row: "Load config", "Save config", "Clear config"
    ui.horizontal(|ui| {
        ui.add_space(8.0);

        if blue_button(ui, "Load config", !app.controller.is_running()).clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("TOML configuration", &["toml"])
                .pick_file()
        {
            match DatapipeConfig::load_from_file(&path) {
                Ok(loaded_cfg) => match loaded_cfg.validate() {
                    Ok(()) => {
                        load_config_into_states(
                            &loaded_cfg,
                            &mut app.input_state,
                            &mut app.decrypt_state,
                            &mut app.encrypt_state,
                            &mut app.output_states,
                        );
                        app.last_message = Some((
                            format!(
                                "Configuration loaded successfully from {:?}",
                                path.file_name().unwrap_or_default()
                            ),
                            false,
                        ));
                    }
                    Err(err) => {
                        app.last_message =
                            Some((format!("Configuration file validation failed: {err}"), true));
                    }
                },
                Err(err) => {
                    app.last_message =
                        Some((format!("Error loading configuration file: {err}"), true));
                }
            }
        }

        ui.add_space(12.0);

        if blue_button(ui, "Save config", !app.controller.is_running()).clicked() {
            match states_to_config(
                &app.input_state,
                &app.decrypt_state,
                &app.encrypt_state,
                &app.output_states,
            ) {
                Ok(cfg) => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("TOML configuration", &["toml"])
                        .set_file_name("datapipe.toml")
                        .save_file()
                    {
                        match cfg.save_to_file(&path) {
                            Ok(()) => {
                                app.last_message = Some((
                                    format!(
                                        "Configuration saved to {:?}",
                                        path.file_name().unwrap_or_default()
                                    ),
                                    false,
                                ));
                            }
                            Err(err) => {
                                app.last_message =
                                    Some((format!("Failed to save configuration: {err}"), true));
                            }
                        }
                    }
                }
                Err(errs) => {
                    app.last_message = Some((
                        format!("Cannot save invalid configuration: {}", errs.join("; ")),
                        true,
                    ));
                }
            }
        }

        ui.add_space(12.0);

        if blue_button(ui, "Clear config", !app.controller.is_running()).clicked() {
            app.input_state = InputState::default();
            app.decrypt_state = DecryptState::default();
            app.encrypt_state = EncryptState::default();
            app.output_states.clear();
            app.output_states.push(OutputState::default());
            app.last_message = Some(("Configuration cleared.".to_string(), false));
        }
    });

    ui.add_space(10.0);

    // Center status container (light gray box)
    let panel_frame = Frame::canvas(ui.style())
        .fill(PANEL_BG)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(Margin::same(12));

    let available_panel_height = ui.available_height() - 56.0;

    panel_frame.show(ui, |ui| {
        ui.set_min_size(Vec2::new(ui.available_width(), available_panel_height));
        render_status_and_metrics(
            ui,
            &app.input_state,
            &app.decrypt_state,
            &app.encrypt_state,
            &app.output_states,
            &mut app.controller,
            &app.last_message,
        );
    });

    // Check if configuration is currently valid for starting
    let config_res = states_to_config(
        &app.input_state,
        &app.decrypt_state,
        &app.encrypt_state,
        &app.output_states,
    );
    let can_start = config_res.is_ok() && !app.controller.is_running();
    let can_stop = app.controller.is_running();

    // Bottom navigation and control buttons
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);

            // Left arrow to Output tab
            if nav_arrow(ui, "⬅").clicked() {
                app.active_tab = Tab::Output;
            }

            // Stop Datapipe button in center
            let center_offset = (ui.available_width() / 2.0) - 160.0;
            if center_offset > 0.0 {
                ui.add_space(center_offset);
            }

            if stop_button(ui, can_stop).clicked() {
                app.controller.stop();
                app.last_message = Some(("Datapipe stopped by user.".to_string(), false));
            }

            // Start Datapipe button on bottom right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if start_button(ui, can_start).clicked()
                    && let Ok(cfg) = config_res
                {
                    let rt_handle = app.runtime.handle().clone();
                    app.controller.start(&rt_handle, cfg);
                    app.last_message = Some(("Datapipe started.".to_string(), false));
                }
            });
        });
    });
}

fn render_status_and_metrics(
    ui: &mut Ui,
    input: &InputState,
    decrypt: &DecryptState,
    encrypt: &EncryptState,
    outputs: &[OutputState],
    controller: &mut PipelineController,
    last_message: &Option<(String, bool)>,
) {
    // 1. Status header and badge
    ui.horizontal(|ui| {
        ui.label(RichText::new("Status:").size(16.0).strong());

        let (status_text, status_color) = match &controller.status {
            PipelineStatus::Idle => ("IDLE", Color32::from_rgb(80, 80, 80)),
            PipelineStatus::Ready => ("READY", Color32::from_rgb(0, 120, 40)),
            PipelineStatus::Running => ("RUNNING", Color32::from_rgb(0, 160, 50)),
            PipelineStatus::Stopped => ("STOPPED", Color32::from_rgb(200, 120, 0)),
            PipelineStatus::Completed => ("COMPLETED", Color32::from_rgb(0, 100, 180)),
            PipelineStatus::Error(_) => ("ERROR", Color32::from_rgb(210, 20, 20)),
        };

        let badge = RichText::new(format!(" {status_text} "))
            .size(15.0)
            .strong()
            .color(Color32::WHITE);

        let badge_bg = status_color;
        egui::Frame::canvas(ui.style())
            .fill(badge_bg)
            .corner_radius(CornerRadius::same(3))
            .inner_margin(Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.label(badge);
            });

        if let PipelineStatus::Error(ref err) = controller.status {
            ui.label(RichText::new(err).color(Color32::from_rgb(200, 20, 20)));
        }
    });

    if let Some((msg, is_err)) = last_message {
        ui.add_space(4.0);
        let color = if *is_err {
            Color32::from_rgb(180, 30, 30)
        } else {
            Color32::from_rgb(30, 120, 30)
        };
        ui.label(RichText::new(msg).size(13.0).color(color));
    }

    ui.add_space(8.0);

    // 2. Validation errors display if not running
    if !controller.is_running()
        && let Err(errs) = states_to_config(input, decrypt, encrypt, outputs)
    {
        Frame::canvas(ui.style())
            .fill(Color32::from_rgb(255, 235, 235))
            .stroke(Stroke::new(1.0, Color32::from_rgb(220, 100, 100)))
            .corner_radius(CornerRadius::same(3))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Missing or invalid required parameters:")
                        .strong()
                        .color(Color32::from_rgb(180, 20, 20)),
                );
                for err in errs {
                    ui.label(
                        RichText::new(format!("• {err}"))
                            .color(Color32::from_rgb(180, 20, 20))
                            .size(12.0),
                    );
                }
            });
    }

    // 3. Animated Pipeline Diagram
    render_animated_diagram(
        ui,
        input,
        decrypt,
        encrypt,
        outputs,
        controller.is_running(),
    );

    ui.add_space(10.0);

    // 4. Live metrics and data transfer statistics
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.heading("Data Transfer Metrics");
        ui.add_space(4.0);

        if let Some(ref m) = controller.metrics {
            egui::Grid::new("metrics_display_grid")
                .num_columns(4)
                .spacing([24.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Elapsed Time:").strong());
                    ui.label(
                        RichText::new(format_duration(m.elapsed()))
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(20, 80, 160)),
                    );

                    ui.label("");
                    ui.label("");
                    ui.end_row();

                    ui.label(RichText::new("Bytes Read:").strong());
                    ui.label(RichText::new(format_bytes(m.read_bytes())).size(14.0));

                    ui.label(RichText::new("Read Rate:").strong());
                    ui.label(RichText::new(format_rate(m.read_rate_bytes_per_sec())).size(14.0));
                    ui.end_row();

                    ui.label(RichText::new("Bytes Written:").strong());
                    ui.label(RichText::new(format_bytes(m.written_bytes())).size(14.0));

                    ui.label(RichText::new("Write Rate:").strong());
                    ui.label(RichText::new(format_rate(m.write_rate_bytes_per_sec())).size(14.0));
                    ui.end_row();
                });
        } else {
            ui.label("No metrics available. Start the pipeline to begin tracking.");
        }
    });
}

fn render_animated_diagram(
    ui: &mut Ui,
    input: &InputState,
    decrypt: &DecryptState,
    encrypt: &EncryptState,
    outputs: &[OutputState],
    is_running: bool,
) {
    let diagram_size = Vec2::new(ui.available_width(), 80.0);
    let (rect, _) = ui.allocate_exact_size(diagram_size, egui::Sense::hover());
    let painter = ui.painter();

    // Draw background for diagram
    painter.rect_filled(
        rect,
        CornerRadius::same(3),
        Color32::from_rgb(215, 215, 215),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0, Color32::from_rgb(180, 180, 180)),
        egui::StrokeKind::Inside,
    );

    // Collect stages to render: Input -> [Decrypt] -> [Encrypt] -> Outputs
    let mut stages = Vec::new();
    stages.push(format!("Input\n({})", input.input_type.name()));

    if decrypt.enabled {
        stages.push("Decrypt\n(ChaCha20)".to_string());
    }

    if encrypt.enabled {
        stages.push("Encrypt\n(ChaCha20)".to_string());
    }

    let out_count = outputs.len();
    if out_count == 1 {
        stages.push(format!("Output\n({})", outputs[0].output_type.name()));
    } else {
        stages.push(format!("Outputs\n({out_count} sinks)"));
    }

    let num_stages = stages.len();
    let node_width = 110.0;
    let node_height = 44.0;
    let total_width = rect.width();
    let spacing = (total_width - (node_width * num_stages as f32)) / (num_stages as f32 + 1.0);

    let y_center = rect.center().y;

    let time = ui.input(|i| i.time);
    if is_running {
        ui.ctx().request_repaint();
    }

    let mut centers = Vec::new();

    for (i, stage_label) in stages.iter().enumerate() {
        let x = rect.min.x + spacing + (i as f32 * (node_width + spacing));
        let node_rect = Rect::from_min_size(
            Pos2::new(x, y_center - (node_height / 2.0)),
            Vec2::new(node_width, node_height),
        );
        centers.push(node_rect.center());

        // Node fill and outline
        let fill = if is_running {
            Color32::from_rgb(240, 248, 255)
        } else {
            Color32::from_rgb(245, 245, 245)
        };

        painter.rect_filled(node_rect, CornerRadius::same(4), fill);
        painter.rect_stroke(
            node_rect,
            CornerRadius::same(4),
            Stroke::new(1.5, BLUE_BUTTON_BG),
            egui::StrokeKind::Inside,
        );

        painter.text(
            node_rect.center(),
            Align2::CENTER_CENTER,
            stage_label,
            FontId::proportional(11.0),
            Color32::from_rgb(20, 20, 20),
        );
    }

    // Draw connecting arrows and animated particles between centers
    for i in 0..centers.len() - 1 {
        let start = Pos2::new(centers[i].x + (node_width / 2.0), y_center);
        let end = Pos2::new(centers[i + 1].x - (node_width / 2.0), y_center);

        // Arrow base line
        painter.line_segment(
            [start, end],
            Stroke::new(2.0, Color32::from_rgb(140, 140, 140)),
        );

        // Arrow tip
        let tip_size = 5.0;
        painter.line_segment(
            [end, Pos2::new(end.x - tip_size, end.y - tip_size)],
            Stroke::new(2.0, Color32::from_rgb(140, 140, 140)),
        );
        painter.line_segment(
            [end, Pos2::new(end.x - tip_size, end.y + tip_size)],
            Stroke::new(2.0, Color32::from_rgb(140, 140, 140)),
        );

        // Animated particles when running
        if is_running {
            let cycle = (time * 1.5 + (i as f64 * 0.3)) % 1.0;
            let particle_x = start.x + ((end.x - start.x) * cycle as f32);
            let particle_pos = Pos2::new(particle_x, y_center);
            painter.circle_filled(particle_pos, 4.0, Color32::from_rgb(0, 180, 60));
        }
    }
}
