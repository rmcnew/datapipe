//! Output tab UI view, dynamic multi-output list (1 to 64 outputs), and form fields.

use crate::gui::style::{PANEL_BG, minus_button, nav_arrow, plus_button};
use crate::gui::types::{OutputState, OutputType, Tab};
use egui::{CornerRadius, Frame, Margin, Stroke, Ui};

/// Maximum number of output destinations supported.
pub const MAX_OUTPUTS: usize = 64;

/// Renders the Output tab.
pub fn show_output_tab(ui: &mut Ui, outputs: &mut Vec<OutputState>, active_tab: &mut Tab) {
    if outputs.is_empty() {
        outputs.push(OutputState::default());
    }

    let mut remove_index: Option<usize> = None;

    // Use a vertical layout with top scroll area and bottom fixed control bar
    let available_height = ui.available_height() - 56.0;

    egui::ScrollArea::vertical()
        .max_height(available_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let total_outputs = outputs.len();
            for (index, state) in outputs.iter_mut().enumerate() {
                ui.add_space(8.0);

                // Selector row matching mockup
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Output #{}:", index + 1))
                            .strong()
                            .size(14.0),
                    );
                    egui::ComboBox::from_id_salt(format!("output_type_combo_{index}"))
                        .width(200.0)
                        .selected_text(egui::RichText::new(state.output_type.name()).strong())
                        .show_ui(ui, |ui| {
                            for ot in OutputType::all() {
                                ui.selectable_value(&mut state.output_type, *ot, ot.name());
                            }
                        });

                    // For outputs after the first output, '-' button is directly to the right
                    if index > 0 {
                        ui.add_space(8.0);
                        if minus_button(ui).clicked() {
                            remove_index = Some(index);
                        }
                    }
                });

                ui.add_space(6.0);

                // Form fields container (light gray box)
                let panel_frame = Frame::canvas(ui.style())
                    .fill(PANEL_BG)
                    .stroke(Stroke::new(1.0, crate::gui::style::BORDER_COLOR))
                    .corner_radius(CornerRadius::same(3))
                    .inner_margin(Margin::same(10));

                panel_frame.show(ui, |ui| {
                    render_single_output_fields(ui, state, index);
                });

                // Horizontal rule between output sections
                if index < total_outputs - 1 {
                    ui.add_space(10.0);
                    ui.separator();
                }
            }
        });

    if let Some(idx) = remove_index
        && outputs.len() > 1
    {
        outputs.remove(idx);
    }

    // Bottom navigation and control buttons
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);

            // Left arrow to Encrypt
            if nav_arrow(ui, "⬅").clicked() {
                *active_tab = Tab::Encrypt;
            }

            // Bottom center '+' button (disabled at 64 outputs)
            let center_offset = (ui.available_width() / 2.0) - 60.0;
            if center_offset > 0.0 {
                ui.add_space(center_offset);
            }

            let can_add = outputs.len() < MAX_OUTPUTS;
            if plus_button(ui, can_add).clicked() {
                outputs.push(OutputState::default());
            }

            // Bottom right arrow to Run
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if nav_arrow(ui, "➡").clicked() {
                    *active_tab = Tab::Run;
                }
            });
        });
    });
}

fn render_single_output_fields(ui: &mut Ui, state: &mut OutputState, index: usize) {
    egui::Grid::new(format!("output_grid_{index}"))
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| match state.output_type {
            OutputType::File => {
                ui.label("File Path:");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 90.0, 24.0],
                        egui::TextEdit::singleline(&mut state.file_path),
                    );
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new().save_file()
                    {
                        state.file_path = path.display().to_string();
                    }
                });
                ui.end_row();
            }
            OutputType::Stdout => {
                ui.label("Standard Output:");
                ui.label("Streams raw output directly to process standard output (STDOUT).");
                ui.end_row();
            }
            OutputType::Tcp => {
                ui.label("Remote TCP Address:");
                ui.add_sized(
                    [240.0, 24.0],
                    egui::TextEdit::singleline(&mut state.tcp_address),
                );
                ui.end_row();
            }
            OutputType::Udp => {
                ui.label("Remote UDP Address:");
                ui.add_sized(
                    [240.0, 24.0],
                    egui::TextEdit::singleline(&mut state.udp_address),
                );
                ui.end_row();
            }
            OutputType::Http => {
                ui.label("HTTP URL:");
                ui.add_sized(
                    [ui.available_width(), 24.0],
                    egui::TextEdit::singleline(&mut state.http_url),
                );
                ui.end_row();
                ui.label("Send Rate (ms):");
                ui.add_sized(
                    [120.0, 24.0],
                    egui::TextEdit::singleline(&mut state.http_rate),
                );
                ui.end_row();

                ui.label("Delimiter:");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [80.0, 24.0],
                        egui::TextEdit::singleline(&mut state.http_delimiter),
                    );
                    ui.checkbox(
                        &mut state.http_include_delimiter,
                        "Include Delimiter in Request",
                    );
                });
                ui.end_row();
            }
            OutputType::Https => {
                ui.label("HTTPS URL:");
                ui.add_sized(
                    [ui.available_width(), 24.0],
                    egui::TextEdit::singleline(&mut state.https_url),
                );
                ui.end_row();

                ui.label("Send Rate (ms):");
                ui.add_sized(
                    [120.0, 24.0],
                    egui::TextEdit::singleline(&mut state.https_rate),
                );
                ui.end_row();

                ui.label("Delimiter:");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [80.0, 24.0],
                        egui::TextEdit::singleline(&mut state.https_delimiter),
                    );
                    ui.checkbox(
                        &mut state.https_include_delimiter,
                        "Include Delimiter in Request",
                    );
                });
                ui.end_row();

                ui.label("Root Certificates (PEM):");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 90.0, 24.0],
                        egui::TextEdit::singleline(&mut state.https_root_certificates),
                    );
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        state.https_root_certificates = path.display().to_string();
                    }
                });
                ui.end_row();

                ui.label("Cert Revocation List (PEM):");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 90.0, 24.0],
                        egui::TextEdit::singleline(&mut state.https_crl),
                    );
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        state.https_crl = path.display().to_string();
                    }
                });
                ui.end_row();

                ui.label("Client Identity (PEM):");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 90.0, 24.0],
                        egui::TextEdit::singleline(&mut state.https_client_identity),
                    );
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        state.https_client_identity = path.display().to_string();
                    }
                });
                ui.end_row();

                ui.label("TLS Security Options:");
                ui.vertical(|ui| {
                    ui.checkbox(
                        &mut state.https_allow_invalid_hostnames,
                        "Allow Invalid Hostnames (Insecure)",
                    );
                    ui.checkbox(
                        &mut state.https_allow_invalid_certificates,
                        "Allow Invalid Certificates (Insecure)",
                    );
                });
                ui.end_row();
            }
            OutputType::Tls => {
                ui.label("Remote TLS Address:");
                ui.add_sized(
                    [240.0, 24.0],
                    egui::TextEdit::singleline(&mut state.tls_address),
                );
                ui.end_row();

                ui.label("Client Cert Chain (DER):");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 90.0, 24.0],
                        egui::TextEdit::singleline(&mut state.tls_cert_chain),
                    );
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        state.tls_cert_chain = path.display().to_string();
                    }
                });
                ui.end_row();

                ui.label("Client Private Key (DER):");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 90.0, 24.0],
                        egui::TextEdit::singleline(&mut state.tls_client_key),
                    );
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        state.tls_client_key = path.display().to_string();
                    }
                });
                ui.end_row();

                ui.label("Custom Root CA (PEM):");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 90.0, 24.0],
                        egui::TextEdit::singleline(&mut state.tls_root_ca),
                    );
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        state.tls_root_ca = path.display().to_string();
                    }
                });
                ui.end_row();

                ui.label("TLS Security Options:");
                ui.checkbox(
                    &mut state.tls_skip_server_verify,
                    "Skip Server Identity Verification (Insecure)",
                );
                ui.end_row();
            }
        });
}
