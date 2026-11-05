//! Encrypt tab Material Design 3 view, key generation, clipboard copy, and validation.

use crate::gui::style::{
    bottom_action_bar, compact_tonal_button, m3_card, m3_monospace_field, m3_switch,
    nav_back_button, nav_next_button, palette, section_header, status_chip, status_chip_with_dot,
    tonal_button,
};
use crate::gui::types::{EncryptState, Tab};
use egui::{CornerRadius, Frame, Layout, Margin, ProgressBar, RichText, Ui};

/// Renders the Encrypt stage configuration tab using Material Design 3 cards and controls.
pub fn show_encrypt_tab(ui: &mut Ui, state: &mut EncryptState, active_tab: &mut Tab) {
    bottom_action_bar(ui, "m3_encrypt_footer", |ui| {
        ui.horizontal(|ui| {
            if nav_back_button(ui, "Decrypt").clicked() {
                *active_tab = Tab::Decrypt;
            }

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if nav_next_button(ui, "Next: Output").clicked() {
                    *active_tab = Tab::Output;
                }
            });
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
                        "Stream Encryption",
                        "Optionally encrypt outbound data into length-prefixed ChaCha20-Poly1305 AEAD cipher frames.",
                    );

                    ui.add_space(14.0);

                    m3_card(ui).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let p = palette(ui);

                        // M3 Switch + Generate Key action row
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.label(
                                    RichText::new("Enable Stream Encryption")
                                        .size(15.0)
                                        .strong()
                                        .color(p.on_surface),
                                );
                                ui.label(
                                    RichText::new(
                                        "ChaCha20-Poly1305 AEAD stream cipher (32-byte key + 19-byte nonce)",
                                    )
                                    .size(12.0)
                                    .color(p.on_surface_variant),
                                );
                            });

                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                m3_switch(ui, &mut state.enabled);
                                ui.add_space(6.0);
                                if tonal_button(ui, "Generate Key", state.enabled).clicked() {
                                    state.generate_key();
                                }
                            });
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        ui.label(
                            RichText::new("51-Byte UTF-8 Encryption Key")
                                .size(13.5)
                                .strong()
                                .color(if state.enabled {
                                    p.on_surface
                                } else {
                                    p.disabled_content()
                                }),
                        );
                        ui.add_space(6.0);

                        let copied_id = egui::Id::unique("encrypt_key_copied_at");

                        ui.horizontal(|ui| {
                            let text_width = (ui.available_width() - 94.0).max(120.0);
                            m3_monospace_field(
                                ui,
                                &mut state.key,
                                "Enter or generate a 51-character UTF-8 key...",
                                text_width,
                                state.enabled,
                            );

                            let can_copy = state.enabled && !state.key.is_empty();
                            if compact_tonal_button(ui, "Copy", can_copy)
                                .on_hover_text("Copy the encryption key to the system clipboard")
                                .clicked()
                            {
                                ui.ctx().copy_text(state.key.clone());
                                let now = ui.input(|i| i.time);
                                ui.data_mut(|d| d.insert_temp(copied_id, now));
                            }
                        });

                        ui.add_space(10.0);

                        if state.enabled {
                            let byte_count = state.key.len();
                            let progress = (byte_count as f32 / 51.0).clamp(0.0, 1.0);
                            let bar_color = if byte_count == 51 {
                                p.tertiary
                            } else {
                                p.error
                            };

                            ui.add(
                                ProgressBar::new(progress)
                                    .desired_height(6.0)
                                    .corner_radius(CornerRadius::same(3))
                                    .fill(bar_color),
                            );

                            ui.add_space(8.0);

                            ui.horizontal_wrapped(|ui| {
                                if byte_count == 51 {
                                    status_chip(
                                        ui,
                                        "VALID (51 / 51 BYTES)",
                                        p.tertiary_container,
                                        p.on_tertiary_container,
                                    );
                                    ui.label(
                                        RichText::new("Ready to encrypt outbound stream chunks.")
                                            .size(12.5)
                                            .color(p.on_surface_variant),
                                    );
                                } else {
                                    status_chip(
                                        ui,
                                        &format!("{byte_count} / 51 BYTES"),
                                        p.error_container,
                                        p.on_error_container,
                                    );
                                    ui.label(
                                        RichText::new("Key must be exactly 51 UTF-8 bytes.")
                                            .size(12.5)
                                            .color(p.error),
                                    );
                                }

                                if let Some(copied_at) = ui.data(|d| d.get_temp::<f64>(copied_id)) {
                                    if ui.input(|i| i.time) - copied_at < 2.0 {
                                        status_chip_with_dot(
                                            ui,
                                            "COPIED TO CLIPBOARD",
                                            p.primary_container,
                                            p.on_primary_container,
                                            true,
                                        );
                                        ui.ctx().request_repaint_after(
                                            std::time::Duration::from_millis(250),
                                        );
                                    } else {
                                        ui.data_mut(|d| d.remove::<f64>(copied_id));
                                    }
                                }
                            });
                        } else {
                            ui.horizontal(|ui| {
                                status_chip(
                                    ui,
                                    "BYPASSED",
                                    p.surface_container_highest,
                                    p.on_surface_variant,
                                );
                                ui.label(
                                    RichText::new(
                                        "Encryption stage bypassed. Enable the switch above to generate or enter a key.",
                                    )
                                    .size(12.5)
                                    .color(p.on_surface_variant),
                                );
                            });
                        }
                    });
                });
        });
}
