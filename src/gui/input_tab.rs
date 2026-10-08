//! Input tab UI view and form fields.

use crate::gui::style::{PANEL_BG, nav_arrow};
use crate::gui::types::{InputState, InputType, Tab};
use egui::{CornerRadius, Frame, Margin, Stroke, Ui, Vec2};

/// Renders the Input tab.
pub fn show_input_tab(ui: &mut Ui, state: &mut InputState, active_tab: &mut Tab) {
    ui.add_space(8.0);

    // Dropdown selector matching mockup
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Select Input:").strong().size(15.0));
        egui::ComboBox::from_id_salt("input_type_dropdown")
            .width(220.0)
            .selected_text(egui::RichText::new(state.input_type.name()).strong())
            .show_ui(ui, |ui| {
                for it in InputType::all() {
                    ui.selectable_value(&mut state.input_type, *it, it.name());
                }
            });
    });

    ui.add_space(8.0);

    // Form fields container (light gray box)
    let panel_frame = Frame::canvas(ui.style())
        .fill(PANEL_BG)
        .stroke(Stroke::new(1.0, crate::gui::style::BORDER_COLOR))
        .corner_radius(CornerRadius::same(3))
        .inner_margin(Margin::same(12));

    panel_frame.show(ui, |ui| {
        ui.set_min_size(Vec2::new(ui.available_width(), 260.0));
        render_input_fields(ui, state);
    });

    // Spacer to push navigation to bottom
    ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if nav_arrow(ui, "➡").clicked() {
                *active_tab = Tab::Decrypt;
            }
        });
    });
}

fn render_input_fields(ui: &mut Ui, state: &mut InputState) {
    egui::Grid::new("input_form_grid")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| match state.input_type {
            InputType::File => {
                ui.label("File Path:");
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [ui.available_width() - 90.0, 24.0],
                        egui::TextEdit::singleline(&mut state.file_path),
                    );
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        state.file_path = path.display().to_string();
                    }
                });
                ui.end_row();
            }
            InputType::Http => {
                ui.label("HTTP URL:");
                ui.add_sized(
                    [ui.available_width(), 24.0],
                    egui::TextEdit::singleline(&mut state.http_url),
                );
                ui.end_row();

                ui.label("Poll Rate (ms):");
                ui.add_sized(
                    [120.0, 24.0],
                    egui::TextEdit::singleline(&mut state.http_rate),
                );
                ui.end_row();
            }
            InputType::Https => {
                ui.label("HTTPS URL:");
                ui.add_sized(
                    [ui.available_width(), 24.0],
                    egui::TextEdit::singleline(&mut state.https_url),
                );
                ui.end_row();

                ui.label("Poll Rate (ms):");
                ui.add_sized(
                    [120.0, 24.0],
                    egui::TextEdit::singleline(&mut state.https_rate),
                );
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
            InputType::Stdin => {
                ui.label("Standard Input:");
                ui.label("Streams raw data directly from process standard input (STDIN).");
                ui.end_row();
            }
            InputType::Tcp => {
                ui.label("Remote TCP Address:");
                ui.add_sized(
                    [240.0, 24.0],
                    egui::TextEdit::singleline(&mut state.tcp_address),
                );
                ui.end_row();
            }
            InputType::TcpListen => {
                ui.label("Local Listen Address:");
                ui.add_sized(
                    [240.0, 24.0],
                    egui::TextEdit::singleline(&mut state.tcp_listen_address),
                );
                ui.end_row();
            }
            InputType::Tls => {
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
            InputType::TlsListen => {
                ui.label("Local Listen Address:");
                ui.add_sized(
                    [240.0, 24.0],
                    egui::TextEdit::singleline(&mut state.tls_listen_address),
                );
                ui.end_row();

                ui.label("Server Cert Chain (DER):");
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        !state.tls_listen_generate_self_signed,
                        egui::TextEdit::singleline(&mut state.tls_listen_cert_chain)
                            .desired_width(ui.available_width() - 90.0),
                    );
                    if ui
                        .add_enabled(
                            !state.tls_listen_generate_self_signed,
                            egui::Button::new("Browse..."),
                        )
                        .clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        state.tls_listen_cert_chain = path.display().to_string();
                    }
                });
                ui.end_row();

                ui.label("Server Private Key (DER):");
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        !state.tls_listen_generate_self_signed,
                        egui::TextEdit::singleline(&mut state.tls_listen_server_key)
                            .desired_width(ui.available_width() - 90.0),
                    );
                    if ui
                        .add_enabled(
                            !state.tls_listen_generate_self_signed,
                            egui::Button::new("Browse..."),
                        )
                        .clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        state.tls_listen_server_key = path.display().to_string();
                    }
                });
                ui.end_row();

                ui.label("TLS Listen Options:");
                ui.vertical(|ui| {
                    ui.checkbox(
                        &mut state.tls_listen_generate_self_signed,
                        "Generate Self-Signed Certificate and Key Automatically",
                    );
                    ui.checkbox(
                        &mut state.tls_listen_skip_client_verify,
                        "Skip Client Identity Verification (Insecure)",
                    );
                });
                ui.end_row();
            }
            InputType::Udp => {
                ui.label("UDP Local Address:");
                ui.add_sized(
                    [240.0, 24.0],
                    egui::TextEdit::singleline(&mut state.udp_address),
                );
                ui.end_row();
            }
            InputType::UdpMulticast => {
                ui.label("Multicast Address:");
                ui.add_sized(
                    [240.0, 24.0],
                    egui::TextEdit::singleline(&mut state.udp_multicast_address),
                );
                ui.end_row();
            }
        });
}
