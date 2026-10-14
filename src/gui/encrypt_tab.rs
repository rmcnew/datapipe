//! Encrypt tab UI view, key generation, and form fields.

use crate::gui::style::{blue_button, nav_arrow};
use crate::gui::types::{EncryptState, Tab};
use egui::{Color32, RichText, Ui};

/// Renders the Encrypt tab.
pub fn show_encrypt_tab(ui: &mut Ui, state: &mut EncryptState, active_tab: &mut Tab) {
    ui.add_space(20.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Encrypt?").size(16.0).strong());
        ui.checkbox(&mut state.enabled, "");

        ui.add_space(80.0);

        if blue_button(ui, "Generate Key", state.enabled).clicked() {
            state.generate_key();
        }
    });

    ui.add_space(24.0);

    ui.label(
        RichText::new("51-byte UTF-8 Encryption Key:")
            .size(15.0)
            .strong(),
    );
    ui.add_space(8.0);

    let copied_id = egui::Id::new("encrypt_key_copied_at");

    ui.horizontal(|ui| {
        // Reserve room on the right for the copy button.
        let text_edit = egui::TextEdit::singleline(&mut state.key)
            .hint_text("Enter or generate exactly 51 UTF-8 bytes...")
            .desired_width((ui.available_width() - 90.0).max(100.0));

        ui.add_enabled(state.enabled, text_edit);

        let can_copy = state.enabled && !state.key.is_empty();
        if ui
            .add_enabled(can_copy, egui::Button::new("Copy"))
            .on_hover_text("Copy the encryption key to the system clipboard")
            .clicked()
        {
            ui.ctx().copy_text(state.key.clone());
            let now = ui.input(|i| i.time);
            ui.data_mut(|d| d.insert_temp(copied_id, now));
        }
    });

    ui.add_space(8.0);

    // Briefly confirm a successful copy.
    if let Some(copied_at) = ui.data(|d| d.get_temp::<f64>(copied_id)) {
        if ui.input(|i| i.time) - copied_at < 2.0 {
            ui.label(
                RichText::new("Encryption key copied to clipboard.")
                    .color(Color32::from_rgb(0, 100, 180)),
            );
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        } else {
            ui.data_mut(|d| d.remove::<f64>(copied_id));
        }
    }

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
            RichText::new("Encryption disabled. Check 'Encrypt?' above to enable.")
                .color(Color32::from_rgb(100, 100, 100)),
        );
    }

    // Bottom navigation buttons: left arrow to Decrypt, right arrow to Output
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if nav_arrow(ui, "⬅").clicked() {
                *active_tab = Tab::Decrypt;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if nav_arrow(ui, "➡").clicked() {
                    *active_tab = Tab::Output;
                }
            });
        });
    });
}
