//! Decrypt tab UI view and form fields.

use crate::gui::style::nav_arrow;
use crate::gui::types::{DecryptState, Tab};
use egui::{Color32, RichText, Ui};

/// Renders the Decrypt tab.
pub fn show_decrypt_tab(ui: &mut Ui, state: &mut DecryptState, active_tab: &mut Tab) {
    ui.add_space(20.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Decrypt?").size(16.0).strong());
        ui.checkbox(&mut state.enabled, "");
    });

    ui.add_space(24.0);

    ui.label(
        RichText::new("51-byte UTF-8 Decryption Key:")
            .size(15.0)
            .strong(),
    );
    ui.add_space(8.0);

    let text_edit = egui::TextEdit::singleline(&mut state.key)
        .hint_text("Enter exactly 51 UTF-8 bytes...")
        .desired_width(ui.available_width() - 20.0);

    ui.add_enabled(state.enabled, text_edit);

    ui.add_space(8.0);

    if state.enabled {
        let byte_count = state.key.len();
        if byte_count == 51 {
            ui.label(
                RichText::new(format!("Key length: {byte_count} / 51 bytes (Valid)"))
                    .color(Color32::from_rgb(0, 140, 40)),
            );
        } else {
            ui.label(
                RichText::new(format!(
                    "Key length: {byte_count} / 51 bytes (Must be exactly 51 bytes)"
                ))
                .color(Color32::from_rgb(200, 30, 30)),
            );
        }
    } else {
        ui.label(
            RichText::new("Decryption disabled. Check 'Decrypt?' above to enable.")
                .color(Color32::from_rgb(100, 100, 100)),
        );
    }

    // Bottom navigation buttons: left arrow to Input, right arrow to Encrypt
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if nav_arrow(ui, "⬅").clicked() {
                *active_tab = Tab::Input;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if nav_arrow(ui, "➡").clicked() {
                    *active_tab = Tab::Encrypt;
                }
            });
        });
    });
}
