//! Visual styling and themed widgets matching the GUI mockups.

use egui::{Color32, CornerRadius, Response, RichText, Stroke, Ui, Vec2, Visuals};

/// Window background color (medium gray).
pub const WINDOW_BG: Color32 = Color32::from_rgb(175, 175, 175);
/// Form container / panel background color (light gray).
pub const PANEL_BG: Color32 = Color32::from_rgb(228, 228, 228);
/// Active tab background color (blue).
pub const TAB_ACTIVE_BG: Color32 = Color32::from_rgb(108, 158, 214);
/// Inactive tab background color (neutral gray).
pub const TAB_INACTIVE_BG: Color32 = Color32::from_rgb(180, 180, 180);
/// Dark border stroke color.
pub const BORDER_COLOR: Color32 = Color32::from_rgb(30, 30, 30);
/// Standard text color for light backgrounds.
pub const TEXT_DARK: Color32 = Color32::from_rgb(20, 20, 20);
/// Blue action button color.
pub const BLUE_BUTTON_BG: Color32 = Color32::from_rgb(108, 158, 214);
/// Stop button color (red).
pub const STOP_BUTTON_BG: Color32 = Color32::from_rgb(220, 20, 20);
/// Start button color (green).
pub const START_BUTTON_BG: Color32 = Color32::from_rgb(0, 153, 51);
/// Disabled button background color.
pub const DISABLED_BG: Color32 = Color32::from_rgb(155, 155, 155);

/// Configures the egui context visuals to match the mockups.
pub fn configure_visuals(ctx: &egui::Context) {
    let mut visuals = Visuals::light();
    visuals.panel_fill = WINDOW_BG;
    visuals.window_fill = WINDOW_BG;
    visuals.override_text_color = Some(TEXT_DARK);
    visuals.widgets.noninteractive.bg_fill = PANEL_BG;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DARK);
    visuals.widgets.inactive.bg_fill = BLUE_BUTTON_BG;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_DARK);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(130, 175, 230);
    visuals.widgets.active.bg_fill = Color32::from_rgb(90, 140, 200);
    ctx.set_visuals(visuals);
}

/// Renders a top-level tab button.
pub fn tab_button(ui: &mut Ui, title: &str, is_selected: bool, width: f32) -> Response {
    let bg_color = if is_selected {
        TAB_ACTIVE_BG
    } else {
        TAB_INACTIVE_BG
    };

    let text = RichText::new(title).size(15.0).color(TEXT_DARK).strong();

    let button = egui::Button::new(text)
        .fill(bg_color)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .corner_radius(CornerRadius::ZERO);

    ui.add_sized(Vec2::new(width, 36.0), button)
}

/// Renders a navigation arrow button (`⬅` or `➡`).
pub fn nav_arrow(ui: &mut Ui, text: &str) -> Response {
    let rich = RichText::new(text).size(22.0).color(TEXT_DARK).strong();

    let button = egui::Button::new(rich)
        .fill(BLUE_BUTTON_BG)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .corner_radius(CornerRadius::same(2));

    ui.add_sized(Vec2::new(80.0, 38.0), button)
}

/// Renders a standard blue action button (e.g., "Load config", "Generate Key").
pub fn blue_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let bg = if enabled { BLUE_BUTTON_BG } else { DISABLED_BG };

    let rich = RichText::new(text)
        .size(14.0)
        .color(if enabled {
            TEXT_DARK
        } else {
            Color32::from_rgb(80, 80, 80)
        })
        .strong();

    let button = egui::Button::new(rich)
        .fill(bg)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .corner_radius(CornerRadius::same(2));

    ui.add_enabled(enabled, button)
}

/// Renders the plus `+` button on the Output tab.
pub fn plus_button(ui: &mut Ui, enabled: bool) -> Response {
    let bg = if enabled { BLUE_BUTTON_BG } else { DISABLED_BG };

    let rich = RichText::new("+")
        .size(22.0)
        .color(if enabled {
            TEXT_DARK
        } else {
            Color32::from_rgb(80, 80, 80)
        })
        .strong();

    let button = egui::Button::new(rich)
        .fill(bg)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .corner_radius(CornerRadius::same(2))
        .min_size(Vec2::new(56.0, 36.0));

    ui.add_enabled(enabled, button)
}

/// Renders the minus `-` button next to output items 2+.
pub fn minus_button(ui: &mut Ui) -> Response {
    let rich = RichText::new("-").size(20.0).color(TEXT_DARK).strong();

    let button = egui::Button::new(rich)
        .fill(BLUE_BUTTON_BG)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .corner_radius(CornerRadius::same(2));

    ui.add_sized(Vec2::new(36.0, 32.0), button)
}

/// Renders the "Stop Datapipe" button (red with white text).
pub fn stop_button(ui: &mut Ui, enabled: bool) -> Response {
    let bg = if enabled { STOP_BUTTON_BG } else { DISABLED_BG };

    let text_color = if enabled {
        Color32::WHITE
    } else {
        Color32::from_rgb(90, 90, 90)
    };

    let rich = RichText::new("Stop Datapipe")
        .size(15.0)
        .color(text_color)
        .strong();

    let button = egui::Button::new(rich)
        .fill(bg)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .corner_radius(CornerRadius::same(2))
        .min_size(Vec2::new(150.0, 38.0));

    ui.add_enabled(enabled, button)
}

/// Renders the "Start Datapipe" button (green with white text).
pub fn start_button(ui: &mut Ui, enabled: bool) -> Response {
    let bg = if enabled {
        START_BUTTON_BG
    } else {
        DISABLED_BG
    };

    let text_color = if enabled {
        Color32::WHITE
    } else {
        Color32::from_rgb(90, 90, 90)
    };

    let rich = RichText::new("Start Datapipe")
        .size(15.0)
        .color(text_color)
        .strong();

    let button = egui::Button::new(rich)
        .fill(bg)
        .stroke(Stroke::new(1.0, BORDER_COLOR))
        .corner_radius(CornerRadius::same(2))
        .min_size(Vec2::new(150.0, 38.0));

    ui.add_enabled(enabled, button)
}
