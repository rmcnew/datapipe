//! Run tab Material Design 3 view, animated pipeline topology, telemetry tiles, and execution controls.

use crate::config::DatapipeConfig;
use crate::gui::pipeline::PipelineController;
use crate::gui::style::{
    M3Palette, bottom_action_bar, m3_card, m3_tonal_card, nav_back_button, outlined_button,
    palette, section_header, start_button, status_chip_with_dot, stop_button, tonal_button,
};
use crate::gui::types::{
    DecryptState, EncryptState, InputState, OutputState, PipelineStatus, Tab,
    load_config_into_states, states_to_config,
};
use crate::metrics::{format_bytes, format_duration, format_rate};
use egui::{
    Align2, CornerRadius, FontFamily, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, Stroke,
    StrokeKind, Ui, Vec2,
};

/// Renders the Run stage execution and monitoring tab using Material Design 3 cards and controls.
pub fn show_run_tab(ui: &mut Ui, app: &mut crate::gui::DatapipeApp) {
    let config_res = states_to_config(
        &app.input_state,
        &app.decrypt_state,
        &app.encrypt_state,
        &app.output_states,
    );
    let is_running = app.controller.is_running();
    let can_start = config_res.is_ok() && !is_running;
    let can_stop = is_running;

    let mut navigate_back = false;
    let mut stop_clicked = false;
    let mut start_clicked = false;

    bottom_action_bar(ui, "m3_run_footer", |ui| {
        ui.horizontal(|ui| {
            if nav_back_button(ui, "Output").clicked() {
                navigate_back = true;
            }

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if start_button(ui, can_start).clicked() {
                    start_clicked = true;
                }
                ui.add_space(6.0);
                if stop_button(ui, can_stop).clicked() {
                    stop_clicked = true;
                }
            });
        });
    });

    if navigate_back {
        app.active_tab = Tab::Output;
    }
    if stop_clicked {
        app.controller.stop();
        app.last_message = Some(("Datapipe stopped by user.".to_string(), false));
    }
    if start_clicked && let Ok(cfg) = config_res {
        let rt_handle = app.runtime.handle().clone();
        app.controller.start(&rt_handle, cfg);
        app.last_message = Some(("Datapipe started.".to_string(), false));
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::NONE
                .inner_margin(Margin::symmetric(24, 18))
                .show(ui, |ui| {
                    // Header + Configuration File Actions
                    ui.horizontal_wrapped(|ui| {
                        ui.vertical(|ui| {
                            section_header(
                                ui,
                                "Pipeline Execution",
                                "Manage TOML profiles, verify topology, and monitor live stream throughput.",
                            );
                        });

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            let editable = !app.controller.is_running();

                            if outlined_button(ui, "Clear", editable).clicked() {
                                app.input_state = InputState::default();
                                app.decrypt_state = DecryptState::default();
                                app.encrypt_state = EncryptState::default();
                                app.output_states.clear();
                                app.output_states.push(OutputState::default());
                                app.controller.reset_metrics();
                                app.last_message =
                                    Some(("Configuration cleared.".to_string(), false));
                            }

                            if tonal_button(ui, "Save TOML", editable).clicked() {
                                handle_save_config(app);
                            }

                            if tonal_button(ui, "Load TOML", editable).clicked() {
                                handle_load_config(app);
                            }
                        });
                    });

                    ui.add_space(14.0);

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
        });
}

fn handle_load_config(app: &mut crate::gui::DatapipeApp) {
    if let Some(path) = rfd::FileDialog::new()
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
                app.last_message = Some((format!("Error loading configuration file: {err}"), true));
            }
        }
    }
}

fn handle_save_config(app: &mut crate::gui::DatapipeApp) {
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

fn render_status_and_metrics(
    ui: &mut Ui,
    input: &InputState,
    decrypt: &DecryptState,
    encrypt: &EncryptState,
    outputs: &[OutputState],
    controller: &mut PipelineController,
    last_message: &Option<(String, bool)>,
) {
    let p = palette(ui);

    // 1. Optional feedback banner
    if let Some((msg, is_err)) = last_message {
        let (bg, fg, border) = if *is_err {
            (p.error_container, p.on_error_container, p.error)
        } else {
            (
                p.secondary_container,
                p.on_secondary_container,
                p.outline_variant,
            )
        };
        Frame::NONE
            .fill(bg)
            .stroke(Stroke::new(1.0, border))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(msg).size(13.0).strong().color(fg));
            });
        ui.add_space(12.0);
    }

    // 2. Validation alert card when configuration is incomplete
    if !controller.is_running()
        && let Err(errs) = states_to_config(input, decrypt, encrypt, outputs)
    {
        Frame::NONE
            .fill(p.error_container)
            .stroke(Stroke::new(1.0, p.error))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new("Configuration Incomplete")
                        .size(14.0)
                        .strong()
                        .color(p.on_error_container),
                );
                ui.add_space(4.0);
                for err in errs {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::new(6.0, 14.0), egui::Sense::hover());
                        ui.painter()
                            .circle_filled(dot_rect.center(), 2.5, p.on_error_container);
                        ui.label(RichText::new(err).size(12.5).color(p.on_error_container));
                    });
                }
            });
        ui.add_space(12.0);
    }

    // 3. M3 Pipeline Topology & Status Card
    m3_card(ui).show(ui, |ui| {
        ui.set_width(ui.available_width());

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Pipeline Topology")
                    .size(15.0)
                    .strong()
                    .color(p.on_surface),
            );

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                let (label, chip_bg, chip_fg, show_dot) = match &controller.status {
                    PipelineStatus::Idle => (
                        "IDLE",
                        p.surface_container_highest,
                        p.on_surface_variant,
                        false,
                    ),
                    PipelineStatus::Ready => (
                        "READY",
                        p.secondary_container,
                        p.on_secondary_container,
                        false,
                    ),
                    PipelineStatus::Running => (
                        "RUNNING",
                        p.tertiary_container,
                        p.on_tertiary_container,
                        true,
                    ),
                    PipelineStatus::Stopped => (
                        "STOPPED",
                        p.warning_container,
                        p.on_warning_container,
                        false,
                    ),
                    PipelineStatus::Completed => (
                        "COMPLETED",
                        p.primary_container,
                        p.on_primary_container,
                        false,
                    ),
                    PipelineStatus::Error(_) => {
                        ("ERROR", p.error_container, p.on_error_container, false)
                    }
                };
                status_chip_with_dot(ui, label, chip_bg, chip_fg, show_dot);
            });
        });

        if let PipelineStatus::Error(ref err) = controller.status {
            ui.add_space(6.0);
            ui.label(RichText::new(err).size(12.5).color(p.error));
        }

        ui.add_space(10.0);

        render_animated_diagram(
            ui,
            &p,
            input,
            decrypt,
            encrypt,
            outputs,
            controller.is_running(),
        );
    });

    ui.add_space(12.0);

    // 4. M3 Live Telemetry Metric Tiles Card
    m3_card(ui).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("Data Transfer Telemetry")
                .size(15.0)
                .strong()
                .color(p.on_surface),
        );
        ui.add_space(10.0);

        let m = controller.metrics_snapshot();

        ui.columns(3, |cols| {
            render_metric_tile(
                &mut cols[0],
                "ELAPSED TIME",
                &format_duration(m.elapsed),
                p.primary,
            );
            render_metric_tile(
                &mut cols[1],
                "BYTES READ",
                &format_bytes(m.read_bytes),
                p.on_surface,
            );
            render_metric_tile(
                &mut cols[2],
                "BYTES WRITTEN",
                &format_bytes(m.written_bytes),
                p.on_surface,
            );
        });

        ui.add_space(8.0);

        let rate_color = if controller.is_running() {
            p.tertiary
        } else {
            p.on_surface
        };

        ui.columns(2, |cols| {
            render_metric_tile(
                &mut cols[0],
                "READ THROUGHPUT",
                &format_rate(m.read_rate),
                rate_color,
            );
            render_metric_tile(
                &mut cols[1],
                "WRITE THROUGHPUT",
                &format_rate(m.write_rate),
                rate_color,
            );
        });
    });
}

fn render_metric_tile(ui: &mut Ui, label: &str, value: &str, value_color: egui::Color32) {
    let p = palette(ui);
    m3_tonal_card(ui).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.label(
            RichText::new(label)
                .size(11.0)
                .strong()
                .color(p.on_surface_variant),
        );
        ui.label(RichText::new(value).size(17.0).strong().color(value_color));
    });
}

fn render_animated_diagram(
    ui: &mut Ui,
    p: &M3Palette,
    input: &InputState,
    decrypt: &DecryptState,
    encrypt: &EncryptState,
    outputs: &[OutputState],
    is_running: bool,
) {
    let diagram_size = Vec2::new(ui.available_width(), 88.0);
    let (rect, _) = ui.allocate_exact_size(diagram_size, egui::Sense::hover());
    let painter = ui.painter();

    painter.rect_filled(rect, CornerRadius::same(12), p.surface_container);
    painter.rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(1.0, p.outline_variant),
        StrokeKind::Inside,
    );

    // Collect active stages: Input -> [Decrypt] -> [Encrypt] -> Outputs
    let mut stages: Vec<(String, String)> = Vec::with_capacity(4);
    stages.push(("Input".to_string(), input.input_type.name().to_string()));

    if decrypt.enabled {
        stages.push(("Decrypt".to_string(), "ChaCha20-P1305".to_string()));
    }

    if encrypt.enabled {
        stages.push(("Encrypt".to_string(), "ChaCha20-P1305".to_string()));
    }

    let out_count = outputs.len();
    if out_count == 1 {
        stages.push((
            "Output".to_string(),
            outputs[0].output_type.name().to_string(),
        ));
    } else {
        stages.push(("Outputs".to_string(), format!("{out_count} Sinks")));
    }

    let num_stages = stages.len();
    let total_width = rect.width();
    let max_node_w =
        ((total_width - (num_stages as f32 + 1.0) * 18.0) / num_stages as f32).clamp(92.0, 142.0);
    let node_width = max_node_w;
    let node_height = 52.0;
    let spacing =
        ((total_width - (node_width * num_stages as f32)) / (num_stages as f32 + 1.0)).max(8.0);

    let y_center = rect.center().y;
    let time = ui.input(|i| i.time);
    if is_running {
        ui.ctx().request_repaint();
    }

    let mut centers = Vec::with_capacity(num_stages);

    for (i, (title, subtitle)) in stages.iter().enumerate() {
        let x = rect.min.x + spacing + (i as f32 * (node_width + spacing));
        let node_rect = Rect::from_min_size(
            Pos2::new(x, y_center - (node_height * 0.5)),
            Vec2::new(node_width, node_height),
        );
        centers.push(node_rect.center());

        let (fill, border, title_fg, sub_fg) = if is_running {
            (
                p.primary_container,
                Stroke::new(1.5, p.primary),
                p.on_primary_container,
                p.on_primary_container,
            )
        } else {
            (
                p.surface_container_lowest,
                Stroke::new(1.0, p.outline),
                p.on_surface,
                p.on_surface_variant,
            )
        };

        painter.rect_filled(node_rect, CornerRadius::same(10), fill);
        painter.rect_stroke(
            node_rect,
            CornerRadius::same(10),
            border,
            StrokeKind::Inside,
        );

        painter.text(
            Pos2::new(node_rect.center().x, node_rect.center().y - 7.5),
            Align2::CENTER_CENTER,
            title,
            FontId::new(13.0, FontFamily::Proportional),
            title_fg,
        );
        painter.text(
            Pos2::new(node_rect.center().x, node_rect.center().y + 9.0),
            Align2::CENTER_CENTER,
            subtitle,
            FontId::new(11.0, FontFamily::Proportional),
            sub_fg,
        );
    }

    let connector_color = if is_running { p.primary } else { p.outline };

    for i in 0..centers.len().saturating_sub(1) {
        let start = Pos2::new(centers[i].x + (node_width * 0.5) + 2.0, y_center);
        let end = Pos2::new(centers[i + 1].x - (node_width * 0.5) - 2.0, y_center);
        if end.x <= start.x {
            continue;
        }

        painter.line_segment([start, end], Stroke::new(2.0, connector_color));

        let tip_size = 5.0;
        painter.line_segment(
            [end, Pos2::new(end.x - tip_size, end.y - tip_size)],
            Stroke::new(2.0, connector_color),
        );
        painter.line_segment(
            [end, Pos2::new(end.x - tip_size, end.y + tip_size)],
            Stroke::new(2.0, connector_color),
        );

        if is_running {
            for offset in [0.0_f64, 0.5_f64] {
                let cycle = (time * 1.4 + (i as f64 * 0.25) + offset) % 1.0;
                let particle_x = start.x + ((end.x - start.x) * cycle as f32);
                let particle_pos = Pos2::new(particle_x, y_center);
                painter.circle_filled(particle_pos, 6.5, p.tertiary.gamma_multiply(0.28));
                painter.circle_filled(particle_pos, 3.8, p.tertiary);
            }
        }
    }
}
