//! Decrypt tab Material Design 3 view and ChaCha20-Poly1305 key configuration.

use crate::gui::style::{
    bottom_action_bar, m3_card, m3_switch, nav_back_button, nav_next_button, palette,
    section_header, status_chip,
};
use crate::gui::types::{DecryptState, Tab};
use egui::{CornerRadius, Frame, Layout, Margin, ProgressBar, RichText, TextStyle, Ui};

/// Renders the Decrypt stage configuration tab using Material Design 3 cards and controls.
pub fn show_decrypt_tab(ui: &mut Ui, state: &mut DecryptState, active_tab: &mut Tab) {
    bottom_action_bar(ui, "m3_decrypt_footer", |ui| {
        ui.horizontal(|ui| {
            if nav_back_button(ui, "Input").clicked() {
                *active_tab = Tab::Input;
            }

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if nav_next_button(ui, "Next: Encrypt").clicked() {
                    *active_tab = Tab::Encrypt;
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
                        "Stream Decryption",
                        "Optionally authenticate and decrypt incoming ChaCha20-Poly1305 framed stream chunks.",
                    );

                    ui.add_space(14.0);

                    m3_card(ui).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let p = palette(ui);

                        // M3 Switch row
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.label(
                                    RichText::new("Enable Stream Decryption")
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
                            });
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);

                        ui.label(
                            RichText::new("51-Byte UTF-8 Decryption Key")
                                .size(13.5)
                                .strong()
                                .color(if state.enabled {
                                    p.on_surface
                                } else {
                                    p.disabled_content()
                                }),
                        );
                        ui.add_space(6.0);

                        let text_edit = egui::TextEdit::singleline(&mut state.key)
                            .font(TextStyle::Monospace)
                            .hint_text("Enter 51-character UTF-8 decryption key...")
                            .desired_width(ui.available_width());

                        ui.add_enabled(state.enabled, text_edit);

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

                            ui.horizontal(|ui| {
                                if byte_count == 51 {
                                    status_chip(
                                        ui,
                                        "VALID (51 / 51 BYTES)",
                                        p.tertiary_container,
                                        p.on_tertiary_container,
                                    );
                                    ui.label(
                                        RichText::new("Ready to decrypt incoming stream frames.")
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
                                        "Decryption stage bypassed. Enable the switch above to configure a key.",
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
