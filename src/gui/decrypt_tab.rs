//! Decrypt tab Material Design 3 view and ChaCha20-Poly1305 key configuration.

use crate::gui::style::{
    bottom_action_bar, m3_card, m3_encryption_key_status, m3_insecure_checkbox, m3_secret_field,
    m3_switch, nav_back_button, nav_next_button, palette, section_header, status_chip,
};
use crate::gui::types::{DecryptState, Tab};
use egui::{Frame, Layout, Margin, RichText, Ui};

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
                                        "ChaCha20-Poly1305 AEAD stream cipher (full-entropy dpk1: / hex: / 51-byte legacy)",
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
                            RichText::new("Decryption Key")
                                .size(13.5)
                                .strong()
                                .color(if state.enabled {
                                    p.on_surface
                                } else {
                                    p.disabled_content()
                                }),
                        );
                        ui.add_space(6.0);

                        m3_secret_field(
                            ui,
                            "decrypt_key",
                            &mut state.key,
                            "Enter decryption key (dpk1:... or legacy 51-byte)...",
                            ui.available_width(),
                            state.enabled,
                        );

                        ui.add_space(10.0);

                        if state.enabled {
                            m3_encryption_key_status(
                                ui,
                                &state.key,
                                "Ready to decrypt incoming stream frames.",
                                |_| {},
                            );
                            ui.add_space(10.0);
                            m3_insecure_checkbox(
                                ui,
                                &mut state.allow_legacy,
                                "Allow legacy unauthenticated v1 streams (UNSAFE against truncation)",
                            );
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
