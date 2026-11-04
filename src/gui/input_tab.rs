//! Input tab Material Design 3 view and protocol configuration form fields.

use crate::gui::style::{
    bottom_action_bar, compact_tonal_button, m3_card, m3_tonal_card, nav_next_button, palette,
    section_header, status_chip,
};
use crate::gui::types::{InputState, InputType, Tab};
use egui::{Frame, Layout, Margin, RichText, Ui};

/// Renders the Input stage configuration tab using Material Design 3 cards and controls.
pub fn show_input_tab(ui: &mut Ui, state: &mut InputState, active_tab: &mut Tab) {
    bottom_action_bar(ui, "m3_input_footer", |ui| {
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if nav_next_button(ui, "Next: Decrypt").clicked() {
                *active_tab = Tab::Decrypt;
            }
        });
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::NONE
                .inner_margin(Margin::symmetric(24, 18))
                .show(ui, |ui| {
                    section_header(
                        ui,
                        "Input Source",
                        "Select the inbound data source protocol and configure its connection or file parameters.",
                    );

                    ui.add_space(14.0);

                    m3_card(ui).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let p = palette(ui);

                        // Protocol selector header inside the M3 card
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Source Protocol")
                                    .size(14.0)
                                    .strong()
                                    .color(p.on_surface),
                            );
                            ui.add_space(8.0);
                            egui::ComboBox::from_id_salt("input_type_dropdown")
                                .width(220.0)
                                .selected_text(
                                    RichText::new(state.input_type.name())
                                        .size(14.0)
                                        .strong()
                                        .color(p.on_surface),
                                )
                                .show_ui(ui, |ui| {
                                    for it in InputType::all() {
                                        ui.selectable_value(&mut state.input_type, *it, it.name());
                                    }
                                });

                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                status_chip(
                                    ui,
                                    state.input_type.name(),
                                    p.primary_container,
                                    p.on_primary_container,
                                );
                            });
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        render_input_fields(ui, state);
                    });
                });
        });
}

fn field_label(ui: &mut Ui, label: &str) {
    let p = palette(ui);
    ui.label(
        RichText::new(label)
            .size(13.5)
            .strong()
            .color(p.on_surface_variant),
    );
}

fn file_picker_row(ui: &mut Ui, value: &mut String, hint: &str, enabled: bool) {
    ui.horizontal(|ui| {
        let text_width = (ui.available_width() - 96.0).max(120.0);
        let edit = egui::TextEdit::singleline(value)
            .hint_text(hint)
            .desired_width(text_width);
        ui.add_enabled(enabled, edit);

        if compact_tonal_button(ui, "Browse...", enabled).clicked()
            && let Some(path) = rfd::FileDialog::new().pick_file()
        {
            *value = path.display().to_string();
        }
    });
}

fn insecure_checkbox(ui: &mut Ui, checked: &mut bool, label: &str) {
    let p = palette(ui);
    ui.horizontal(|ui| {
        ui.checkbox(checked, label);
        if *checked {
            status_chip(ui, "INSECURE", p.warning_container, p.on_warning_container);
        }
    });
}

fn render_input_fields(ui: &mut Ui, state: &mut InputState) {
    let p = palette(ui);

    if state.input_type == InputType::Stdin {
        m3_tonal_card(ui).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Standard Input (STDIN)")
                    .size(14.0)
                    .strong()
                    .color(p.on_surface),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Streams raw bytes directly from the process standard input descriptor. No additional parameters are required.",
                )
                .size(13.0)
                .color(p.on_surface_variant),
            );
        });
        return;
    }

    egui::Grid::new("input_form_grid")
        .num_columns(2)
        .spacing([16.0, 12.0])
        .show(ui, |ui| match state.input_type {
            InputType::File => {
                field_label(ui, "File Path");
                file_picker_row(ui, &mut state.file_path, "/path/to/input.bin", true);
                ui.end_row();
            }
            InputType::Http => {
                field_label(ui, "HTTP Endpoint URL");
                ui.add(
                    egui::TextEdit::singleline(&mut state.http_url)
                        .hint_text("http://example.com:8080/stream")
                        .desired_width(ui.available_width()),
                );
                ui.end_row();

                field_label(ui, "Poll Interval (ms)");
                ui.add(
                    egui::TextEdit::singleline(&mut state.http_rate)
                        .hint_text("1000")
                        .desired_width(140.0),
                );
                ui.end_row();
            }
            InputType::Https => {
                field_label(ui, "HTTPS Endpoint URL");
                ui.add(
                    egui::TextEdit::singleline(&mut state.https_url)
                        .hint_text("https://example.com:8443/stream")
                        .desired_width(ui.available_width()),
                );
                ui.end_row();

                field_label(ui, "Poll Interval (ms)");
                ui.add(
                    egui::TextEdit::singleline(&mut state.https_rate)
                        .hint_text("1000")
                        .desired_width(140.0),
                );
                ui.end_row();

                field_label(ui, "Root Certificates (PEM)");
                file_picker_row(
                    ui,
                    &mut state.https_root_certificates,
                    "Optional custom CA bundle (.pem)",
                    true,
                );
                ui.end_row();

                field_label(ui, "Cert Revocation List (PEM)");
                file_picker_row(
                    ui,
                    &mut state.https_crl,
                    "Optional certificate revocation list (.pem)",
                    true,
                );
                ui.end_row();

                field_label(ui, "Client Identity (PEM)");
                file_picker_row(
                    ui,
                    &mut state.https_client_identity,
                    "Optional mTLS client cert + key (.pem)",
                    true,
                );
                ui.end_row();

                field_label(ui, "TLS Verification");
                ui.vertical(|ui| {
                    insecure_checkbox(
                        ui,
                        &mut state.https_allow_invalid_hostnames,
                        "Allow Invalid Hostnames",
                    );
                    insecure_checkbox(
                        ui,
                        &mut state.https_allow_invalid_certificates,
                        "Allow Invalid Certificates",
                    );
                });
                ui.end_row();
            }
            InputType::Stdin => {}
            InputType::Tcp => {
                field_label(ui, "Remote TCP Address");
                ui.add(
                    egui::TextEdit::singleline(&mut state.tcp_address)
                        .hint_text("127.0.0.1:9000")
                        .desired_width(260.0),
                );
                ui.end_row();
            }
            InputType::TcpListen => {
                field_label(ui, "Local Listen Address");
                ui.add(
                    egui::TextEdit::singleline(&mut state.tcp_listen_address)
                        .hint_text("0.0.0.0:9000")
                        .desired_width(260.0),
                );
                ui.end_row();
            }
            InputType::Tls => {
                field_label(ui, "Remote TLS Address");
                ui.add(
                    egui::TextEdit::singleline(&mut state.tls_address)
                        .hint_text("127.0.0.1:9443")
                        .desired_width(260.0),
                );
                ui.end_row();

                field_label(ui, "Client Cert Chain (DER)");
                file_picker_row(
                    ui,
                    &mut state.tls_cert_chain,
                    "Client certificate (.der)",
                    true,
                );
                ui.end_row();

                field_label(ui, "Client Private Key (DER)");
                file_picker_row(
                    ui,
                    &mut state.tls_client_key,
                    "Client private key (.der)",
                    true,
                );
                ui.end_row();

                field_label(ui, "Custom Root CA (PEM)");
                file_picker_row(
                    ui,
                    &mut state.tls_root_ca,
                    "Optional root CA certificate (.pem)",
                    true,
                );
                ui.end_row();

                field_label(ui, "TLS Verification");
                insecure_checkbox(
                    ui,
                    &mut state.tls_skip_server_verify,
                    "Skip Server Identity Verification",
                );
                ui.end_row();
            }
            InputType::TlsListen => {
                field_label(ui, "Local Listen Address");
                ui.add(
                    egui::TextEdit::singleline(&mut state.tls_listen_address)
                        .hint_text("0.0.0.0:9443")
                        .desired_width(260.0),
                );
                ui.end_row();

                let manual_certs = !state.tls_listen_generate_self_signed;

                field_label(ui, "Server Cert Chain (DER)");
                file_picker_row(
                    ui,
                    &mut state.tls_listen_cert_chain,
                    "Server certificate (.der)",
                    manual_certs,
                );
                ui.end_row();

                field_label(ui, "Server Private Key (DER)");
                file_picker_row(
                    ui,
                    &mut state.tls_listen_server_key,
                    "Server private key (.der)",
                    manual_certs,
                );
                ui.end_row();

                field_label(ui, "TLS Listener Options");
                ui.vertical(|ui| {
                    ui.checkbox(
                        &mut state.tls_listen_generate_self_signed,
                        "Generate Self-Signed Certificate & Key Automatically",
                    );
                    insecure_checkbox(
                        ui,
                        &mut state.tls_listen_skip_client_verify,
                        "Skip Client Identity Verification",
                    );
                });
                ui.end_row();
            }
            InputType::Udp => {
                field_label(ui, "UDP Bind Address");
                ui.add(
                    egui::TextEdit::singleline(&mut state.udp_address)
                        .hint_text("0.0.0.0:9100")
                        .desired_width(260.0),
                );
                ui.end_row();
            }
            InputType::UdpMulticast => {
                field_label(ui, "Multicast Group Address");
                ui.add(
                    egui::TextEdit::singleline(&mut state.udp_multicast_address)
                        .hint_text("239.255.0.1:9200")
                        .desired_width(260.0),
                );
                ui.end_row();
            }
        });
}
