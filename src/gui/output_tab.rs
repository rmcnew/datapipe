//! Output tab Material Design 3 view, dynamic multi-output fan-out cards (1 to 64 sinks), and form fields.

use crate::gui::style::{
    bottom_action_bar, compact_tonal_button, danger_outlined_button, m3_card, m3_monospace_field,
    m3_text_field, m3_tonal_card, nav_back_button, nav_next_button, palette, plus_button,
    section_header, status_chip,
};
use crate::gui::types::{OutputState, OutputType, Tab};
use egui::{Frame, Layout, Margin, RichText, Ui};

/// Maximum number of concurrent output destinations supported.
pub const MAX_OUTPUTS: usize = 64;

/// Renders the Output stage configuration tab using Material Design 3 cards and controls.
pub fn show_output_tab(ui: &mut Ui, outputs: &mut Vec<OutputState>, active_tab: &mut Tab) {
    if outputs.is_empty() {
        outputs.push(OutputState::default());
    }

    let can_add = outputs.len() < MAX_OUTPUTS;
    let mut add_requested = false;

    bottom_action_bar(ui, "m3_output_footer", |ui| {
        ui.horizontal(|ui| {
            if nav_back_button(ui, "Encrypt").clicked() {
                *active_tab = Tab::Encrypt;
            }

            ui.add_space(8.0);

            if plus_button(ui, can_add).clicked() {
                add_requested = true;
            }

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if nav_next_button(ui, "Next: Run").clicked() {
                    *active_tab = Tab::Run;
                }
            });
        });
    });

    if add_requested && outputs.len() < MAX_OUTPUTS {
        outputs.push(OutputState::default());
    }

    let mut remove_index: Option<usize> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            Frame::NONE
                .inner_margin(Margin::symmetric(24, 18))
                .show(ui, |ui| {
                    let p = palette(ui);

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            section_header(
                                ui,
                                "Output Destinations",
                                "Configure up to 64 concurrent output sinks for fan-out data streaming.",
                            );
                        });

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            status_chip(
                                ui,
                                &format!("{} / {MAX_OUTPUTS} SINKS", outputs.len()),
                                p.secondary_container,
                                p.on_secondary_container,
                            );
                        });
                    });

                    ui.add_space(14.0);

                    for (index, state) in outputs.iter_mut().enumerate() {
                        if index > 0 {
                            ui.add_space(12.0);
                        }

                        m3_card(ui).show(ui, |ui| {
                            ui.set_width(ui.available_width());

                            // Card header row
                            ui.horizontal(|ui| {
                                status_chip(
                                    ui,
                                    &format!("SINK #{}", index + 1),
                                    p.primary_container,
                                    p.on_primary_container,
                                );

                                ui.add_space(4.0);

                                egui::ComboBox::from_id_salt(format!("output_type_combo_{index}"))
                                    .width(200.0)
                                    .selected_text(
                                        RichText::new(state.output_type.name())
                                            .size(14.0)
                                            .strong()
                                            .color(p.on_surface),
                                    )
                                    .show_ui(ui, |ui| {
                                        for ot in OutputType::all() {
                                            ui.selectable_value(
                                                &mut state.output_type,
                                                *ot,
                                                ot.name(),
                                            );
                                        }
                                    });

                                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                    if index > 0
                                        && danger_outlined_button(ui, "Remove", true).clicked()
                                    {
                                        remove_index = Some(index);
                                    }
                                });
                            });

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(10.0);

                            render_single_output_fields(ui, state, index);
                        });
                    }
                });
        });

    if let Some(idx) = remove_index
        && outputs.len() > 1
    {
        outputs.remove(idx);
    }
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

fn pick_file_row(ui: &mut Ui, value: &mut String, hint: &str, save_dialog: bool) {
    ui.horizontal(|ui| {
        let text_width = (ui.available_width() - 96.0).max(120.0);
        m3_text_field(ui, value, hint, text_width);
        if compact_tonal_button(ui, "Browse...", true).clicked() {
            let selected = if save_dialog {
                rfd::FileDialog::new().save_file()
            } else {
                rfd::FileDialog::new().pick_file()
            };
            if let Some(path) = selected {
                *value = path.display().to_string();
            }
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

fn render_single_output_fields(ui: &mut Ui, state: &mut OutputState, index: usize) {
    let p = palette(ui);

    if state.output_type == OutputType::Stdout {
        m3_tonal_card(ui).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Standard Output (STDOUT)")
                    .size(14.0)
                    .strong()
                    .color(p.on_surface),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Writes outbound bytes directly to the process standard output stream.",
                )
                .size(13.0)
                .color(p.on_surface_variant),
            );
        });
        return;
    }

    egui::Grid::new(format!("output_grid_{index}"))
        .num_columns(2)
        .spacing([16.0, 10.0])
        .show(ui, |ui| match state.output_type {
            OutputType::File => {
                field_label(ui, "Output File Path");
                pick_file_row(ui, &mut state.file_path, "/path/to/output.bin", true);
                ui.end_row();
            }
            OutputType::Stdout => {}
            OutputType::Tcp => {
                field_label(ui, "Remote TCP Address");
                m3_text_field(ui, &mut state.tcp_address, "127.0.0.1:9001", 260.0);
                ui.end_row();
            }
            OutputType::Udp => {
                field_label(ui, "Remote UDP Address");
                m3_text_field(ui, &mut state.udp_address, "127.0.0.1:9101", 260.0);
                ui.end_row();
            }
            OutputType::Http => {
                field_label(ui, "HTTP Endpoint URL");
                m3_text_field(
                    ui,
                    &mut state.http_url,
                    "http://example.com:8080/ingest",
                    ui.available_width(),
                );
                ui.end_row();

                field_label(ui, "Send Interval (ms)");
                m3_text_field(ui, &mut state.http_rate, "1000", 140.0);
                ui.end_row();

                field_label(ui, "Record Delimiter");
                ui.horizontal(|ui| {
                    m3_text_field(ui, &mut state.http_delimiter, "\\n", 96.0);
                    ui.checkbox(
                        &mut state.http_include_delimiter,
                        "Include Delimiter in Request",
                    );
                });
                ui.end_row();
            }
            OutputType::Https => {
                field_label(ui, "HTTPS Endpoint URL");
                m3_text_field(
                    ui,
                    &mut state.https_url,
                    "https://example.com:8443/ingest",
                    ui.available_width(),
                );
                ui.end_row();

                field_label(ui, "Send Interval (ms)");
                m3_text_field(ui, &mut state.https_rate, "1000", 140.0);
                ui.end_row();

                field_label(ui, "Record Delimiter");
                ui.horizontal(|ui| {
                    m3_text_field(ui, &mut state.https_delimiter, "\\n", 96.0);
                    ui.checkbox(
                        &mut state.https_include_delimiter,
                        "Include Delimiter in Request",
                    );
                });
                ui.end_row();

                field_label(ui, "Root Certificates (PEM)");
                pick_file_row(
                    ui,
                    &mut state.https_root_certificates,
                    "Optional custom CA bundle (.pem)",
                    false,
                );
                ui.end_row();

                field_label(ui, "Cert Revocation List (PEM)");
                pick_file_row(
                    ui,
                    &mut state.https_crl,
                    "Optional certificate revocation list (.pem)",
                    false,
                );
                ui.end_row();

                field_label(ui, "Client Identity (PEM)");
                pick_file_row(
                    ui,
                    &mut state.https_client_identity,
                    "Optional mTLS client cert + key (.pem)",
                    false,
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
            OutputType::Tls => {
                field_label(ui, "Remote TLS Address");
                m3_text_field(ui, &mut state.tls_address, "127.0.0.1:9444", 260.0);
                ui.end_row();

                field_label(ui, "Client Cert Chain (DER)");
                pick_file_row(
                    ui,
                    &mut state.tls_cert_chain,
                    "Client certificate (.der)",
                    false,
                );
                ui.end_row();

                field_label(ui, "Client Private Key (DER)");
                pick_file_row(
                    ui,
                    &mut state.tls_client_key,
                    "Client private key (.der)",
                    false,
                );
                ui.end_row();

                field_label(ui, "Custom Root CA (PEM)");
                pick_file_row(
                    ui,
                    &mut state.tls_root_ca,
                    "Optional root CA certificate (.pem)",
                    false,
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
            OutputType::Iroh => {
                field_label(ui, "Remote EndpointId");
                m3_monospace_field(
                    ui,
                    &mut state.iroh_peer_id,
                    "64-character hex public key (EndpointId)",
                    ui.available_width(),
                    true,
                );
                ui.end_row();

                field_label(ui, "Direct Addresses (Optional)");
                m3_text_field(
                    ui,
                    &mut state.iroh_direct_addrs,
                    "127.0.0.1:11204, 192.168.1.10:11204",
                    ui.available_width(),
                );
                ui.end_row();

                field_label(ui, "Custom Relay URL (Optional)");
                m3_text_field(
                    ui,
                    &mut state.iroh_relay_url,
                    "https://use1-1.relay.iroh.network.",
                    ui.available_width(),
                );
                ui.end_row();

                field_label(ui, "ALPN Protocol");
                m3_text_field(ui, &mut state.iroh_alpn, "datapipe/iroh/1", 260.0);
                ui.end_row();

                field_label(ui, "Local Secret Key (Optional)");
                ui.horizontal(|ui| {
                    let text_width = (ui.available_width() - 120.0).max(140.0);
                    m3_monospace_field(
                        ui,
                        &mut state.iroh_secret_key,
                        "Optional 64-character hex Ed25519 secret key",
                        text_width,
                        true,
                    );
                    if compact_tonal_button(ui, "Generate Key", true).clicked() {
                        state.generate_iroh_key();
                    }
                });
                ui.end_row();

                field_label(ui, "Relay Transport");
                ui.checkbox(
                    &mut state.iroh_no_relay,
                    "Disable Default Relay Servers (Direct IP Only)",
                );
                ui.end_row();
            }
        });
}
